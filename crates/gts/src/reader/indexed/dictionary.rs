// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-bound raw dictionary admission with retained frame diagnostics.

use super::{AccessBinding, IndexedCatalog, IndexedError, SegmentCatalog, TermBlock};
use crate::FastSet;
use crate::dictionary::{DictionaryError, ReaderMetadata, TermKey, TermLease, TermStorage};
use crate::model::Diagnostic;
use crate::model::{Graph, TermKind, Triple3};
use crate::reader::term_decode;
use crate::reader::term_decode::TermReferences;
use crate::reader::{FoldError, ReifierIndex};
use crate::reader_rows::TermIdBase;
use crate::reader_rows::{RowDecode, decode_reifier_row, numeric_row, take_storage_failure};
use crate::term_table::TermMetadata;
use purrdf_lex::cbor::ValueRef;
use std::cell::RefCell;
use std::io::{self, Read, Seek};

/// One original admitted raw term and the selected frame's diagnostics.
/// Diagnostics survive warm lease reloads; they are never converted into a
/// clean success merely because the decoded payload is already stored.
#[derive(Debug)]
pub struct SelectedTerm {
    /// Original raw descriptor and its caller-owned admission account.
    pub term: TermLease,
    /// Original selected-frame diagnostics, in reader order.
    pub diagnostics: Vec<Diagnostic>,
}
struct AdmittedBlock {
    segment: u64,
    block: TermBlock,
    diagnostics: Vec<Diagnostic>,
}
struct AdmittedBinding {
    key: TermKey,
    triple: Option<Triple3>,
    diagnostics: Vec<Diagnostic>,
}

/// Source-ordered first binding and the diagnostics encountered admitting it.
#[derive(Clone, Debug)]
pub struct SelectedBinding {
    /// First accepted tuple, or absence after the complete relevant scan.
    pub triple: Option<Triple3>,
    /// Original projection/binding diagnostics retained for warm replay.
    pub diagnostics: Vec<Diagnostic>,
}

/// Owns raw storage together with the exact source/capability binding and
/// selected-frame evidence that permits warm reuse. Wrapping prepopulated
/// storage confers no trust: each newly selected block must be verified and
/// introduced through the original source-prefix contract first.
pub struct IndexedDictionary<R: Read + Seek, C, S> {
    source: IndexedCatalog<R, C>,
    storage: S,
    catalogs: Vec<SegmentCatalog>,
    admitted: Vec<AdmittedBlock>,
    bindings: Vec<AdmittedBinding>,
    prepared_segments: Vec<u64>,
    resolution_entries: usize,
    binding_entries: usize,
    failure: Option<IndexedError>,
}
purrdf_hash::debug_non_exhaustive!([R: Read + Seek, C, S] IndexedDictionary<R, C, S> {});
impl<R: Read + Seek, C: FnMut(&R) -> io::Result<AccessBinding>, S: TermStorage>
    IndexedDictionary<R, C, S>
{
    /// Retain the bound source and caller storage without scanning any file.
    /// # Errors
    /// Returns original identity/storage failure or incompatible resource bounds.
    pub fn new(mut source: IndexedCatalog<R, C>, mut storage: S) -> Result<Self, IndexedError> {
        source.check_current()?;
        let actual = storage.limits()?;
        let selected = source.limits();
        for (resource, required, limit) in [
            (
                "encoded-frame-bytes",
                selected.encoded_frame_bytes,
                actual.encoded_frame_bytes,
            ),
            (
                "decoded-frame-bytes",
                selected.decoded_frame_bytes,
                actual.decoded_frame_bytes,
            ),
            (
                "metadata-entries",
                selected.metadata_entries,
                actual.metadata_entries,
            ),
        ] {
            if required > limit {
                return Err(DictionaryError::Limit {
                    resource,
                    required,
                    limit,
                }
                .into());
            }
        }
        Ok(Self {
            source,
            storage,
            catalogs: Vec::new(),
            admitted: Vec::new(),
            bindings: Vec::new(),
            prepared_segments: Vec::new(),
            resolution_entries: 0,
            binding_entries: 0,
            failure: None,
        })
    }
    /// Actual transport observations, including cold catalog/range discovery.
    pub fn work(&self) -> super::IndexedWork {
        self.source.work()
    }
    /// Actual settled reader-side metadata. Raw term allocations and escaped
    /// pins remain under caller storage; each returned lease reports its actual
    /// retained bytes. This owner retains no raw term lease between calls.
    pub fn metadata(&self) -> ReaderMetadata {
        ReaderMetadata {
            entries: self.entries(),
            frame_ids: self.catalogs.iter().fold(0usize, |count, catalog| {
                count.saturating_add(catalog.frame_count())
            }),
            frame_capacity: self.catalogs.iter().fold(0usize, |count, catalog| {
                count.saturating_add(catalog.frame_capacity())
            }),
            ..ReaderMetadata::default()
        }
    }
    /// Admit the actual active pending/resolved/binding metadata of one
    /// exclusively borrowed consumer. This shares the catalog/proof ceiling;
    /// it is not an independent second allowance. Successful checks also
    /// preserve original host cancellation and metadata refusal callbacks.
    /// # Errors
    /// Returns the original source/storage refusal or combined metadata limit.
    pub fn resolution_checkpoint(&mut self, entries: usize) -> Result<(), IndexedError> {
        self.workspace_checkpoint(
            self.resolution_entries,
            entries,
            "indexed-resolution-metadata",
        )?;
        self.resolution_entries = entries;
        Ok(())
    }
    fn binding_checkpoint(&mut self, entries: usize) -> Result<(), IndexedError> {
        self.workspace_checkpoint(self.binding_entries, entries, "indexed-reifier-metadata")?;
        self.binding_entries = entries;
        Ok(())
    }
    fn workspace_checkpoint(
        &mut self,
        previous: usize,
        entries: usize,
        resource: &'static str,
    ) -> Result<(), IndexedError> {
        if let Some(cause) = &self.failure {
            return Err(cause.clone());
        }
        self.source.check_current()?;
        self.storage.checkpoint(1)?;
        let required = self
            .entries()
            .saturating_sub(previous)
            .saturating_add(entries);
        let limit = self.source.limits().metadata_entries;
        if required > limit {
            return Err(DictionaryError::Limit {
                resource,
                required,
                limit,
            }
            .into());
        }
        let mut usage = self.metadata();
        usage.entries = required;
        self.storage.reader_metadata(usage)?;
        self.source.check_current()?;
        Ok(())
    }
    /// Release only the consumer metadata charge after destroying its actual
    /// pending/resolved state. This performs no source I/O during destruction.
    pub fn release_resolution_metadata(&mut self) {
        self.resolution_entries = 0;
    }
    fn observe_metadata(&mut self, additional: usize) -> Result<(), IndexedError> {
        let mut usage = self.metadata();
        usage.entries = usage.entries.saturating_add(additional);
        self.storage.reader_metadata(usage)?;
        Ok(())
    }

    fn entries(&self) -> usize {
        self.catalogs
            .iter()
            .fold(
                self.admitted
                    .len()
                    .saturating_add(self.bindings.len())
                    .saturating_add(self.prepared_segments.len())
                    .saturating_add(self.resolution_entries)
                    .saturating_add(self.binding_entries),
                |count, catalog| count.saturating_add(catalog.metadata_entries()),
            )
            .saturating_add(self.admitted.iter().fold(0usize, |count, block| {
                count.saturating_add(block.diagnostics.len())
            }))
            .saturating_add(self.bindings.iter().fold(0usize, |count, binding| {
                count.saturating_add(binding.diagnostics.len())
            }))
    }
    fn admit(&self, additional: usize) -> Result<(), IndexedError> {
        let required = self.entries().saturating_add(additional);
        let limit = self.source.limits().metadata_entries;
        if required > limit {
            return Err(DictionaryError::Limit {
                resource: "indexed-metadata-entries",
                required,
                limit,
            }
            .into());
        }
        Ok(())
    }

    fn retain_catalog(&mut self, segment: u64) -> Result<usize, IndexedError> {
        if let Some(index) = self
            .catalogs
            .iter()
            .position(|catalog| catalog.segment() == segment)
        {
            return Ok(index);
        }
        let catalog = self.source.segment(segment)?;
        self.admit(catalog.metadata_entries())?;
        self.catalogs.push(catalog);
        self.observe_metadata(0)?;
        Ok(self.catalogs.len() - 1)
    }

    /// Explicitly authenticate the complete observed dictionary/snapshot frame
    /// set for indirect reference access. Encoded admission verifies each frame
    /// hash and predecessor with one bounded frame owner, without folding RDF
    /// rows or admitting raw terms. This can read every dictionary frame's bytes;
    /// it is separate from ordinary open/fetch and from global trust verification.
    /// A successful proof is reused only by this same bound session.
    /// # Errors
    /// Returns original source/integrity/resource refusals without publishing
    /// incomplete coverage. A failed preparation poisons the session.
    pub fn prepare_reference_access(&mut self, segment: u64) -> Result<(), IndexedError> {
        if let Some(cause) = &self.failure {
            return Err(cause.clone());
        }
        let result: Result<(), IndexedError> = (|| {
            self.source.check_current()?;
            self.storage.checkpoint(1)?;
            if self.prepared_segments.contains(&segment) {
                self.source.check_current()?;
                return Ok(());
            }
            let index = self.retain_catalog(segment)?;
            self.source
                .verify_reference_contributors(&self.catalogs[index])?;
            self.admit(1)?;
            self.observe_metadata(1)?;
            self.source.check_current()?;
            self.prepared_segments.push(segment);
            Ok(())
        })();
        if let Err(cause) = &result {
            self.failure = Some(cause.clone());
        }
        result
    }

    /// Fetch one original descriptor from a verified selected dictionary block.
    /// Cold lookup structurally locates preceding blocks; a warm hit checks the
    /// live binding and reloads its original lease without reintroducing terms.
    /// No forward-only or resident full-reader fallback is selected implicitly.
    /// # Errors
    /// Returns original storage/capability/corruption/resource failure. The first
    /// refusal poisons this session, including provisional storage operations.
    pub fn get(&mut self, key: TermKey) -> Result<SelectedTerm, IndexedError> {
        if let Some(cause) = &self.failure {
            return Err(cause.clone());
        }
        let result = self.get_inner(key);
        if let Err(cause) = &result {
            self.failure = Some(cause.clone());
        }
        result
    }
    fn get_inner(&mut self, key: TermKey) -> Result<SelectedTerm, IndexedError> {
        self.source.check_current()?;
        self.storage.checkpoint(1)?;
        if let Some(proof) = self.admitted.iter().find(|proof| {
            proof.segment == key.segment && (proof.block.first..proof.block.end).contains(&key.term)
        }) {
            let term = self
                .storage
                .get(key)?
                .ok_or(DictionaryError::Missing(key))?;
            self.source.check_current()?;
            return Ok(SelectedTerm {
                term,
                diagnostics: proof.diagnostics.clone(),
            });
        }
        let catalog = self.retain_catalog(key.segment)?;
        let catalog = &self.catalogs[catalog];
        let block = self.source.locate_term(catalog, key.term)?;
        let payload = self.source.block_payload(catalog, &block)?;
        let rows = if block.snapshot {
            ValueRef::Owned(&payload)
                .get("terms")
                .map_err(IndexedError::native)?
        } else {
            Some(ValueRef::Owned(&payload))
        };
        let rows = rows
            .map(ValueRef::array)
            .transpose()
            .map_err(IndexedError::native)?
            .flatten()
            .ok_or(DictionaryError::Corrupt(
                "located term block has no term array",
            ))?;
        let frame_item = catalog.frame_item(block.frame)?;
        let base = TermIdBase(if block.snapshot { block.first } else { 0 });
        let mut next = block.first;
        let mut diagnostics = Vec::new();
        for raw in rows {
            self.storage.checkpoint(1)?;
            let raw = raw
                .map_err(IndexedError::native)?
                .to_value()
                .map_err(IndexedError::native)?;
            let term = term_decode::decode(raw.as_ref(), next, base, |cause| {
                diagnostics.push(cause.into_diagnostic(frame_item));
            });
            let Some(term) = term else {
                continue;
            };
            let introduced = TermKey {
                segment: key.segment,
                term: next,
            };
            let lease = self.storage.introduce(introduced, &term)?;
            drop(lease);
            next = next
                .checked_add(1)
                .ok_or(IndexedError::Overflow("admitted term ordinal"))?;
        }
        if next != block.end {
            return Err(DictionaryError::Corrupt(
                "decoded term count contradicts structural interval",
            )
            .into());
        }
        self.admit(1usize.saturating_add(diagnostics.len()))?;
        self.observe_metadata(1usize.saturating_add(diagnostics.len()))?;
        self.storage.commit()?;
        self.source.check_current()?;
        let term = self
            .storage
            .get(key)?
            .ok_or(DictionaryError::Missing(key))?;
        self.source.check_current()?;
        let result = SelectedTerm {
            term,
            diagnostics: diagnostics.clone(),
        };
        self.admitted.push(AdmittedBlock {
            segment: key.segment,
            block,
            diagnostics,
        });
        Ok(result)
    }

    /// Discover an indirect term's first binding in original source order.
    /// Requires explicit reference preparation. Plain lexical bodies are
    /// skipped during numeric anchor/position/recursion queries; these queries
    /// admit no unrelated raw terms. Actual RDF dependencies acquire their
    /// original leases through the separate selected-term operation.
    /// The complete relevant scan detects later legacy ambiguity. Ordinary and
    /// self-describing triple lookups do not call this operation.
    /// # Errors
    /// Returns original source/storage/capability/resource refusal, preserving
    /// the first failed session cause without publishing provisional bindings.
    /// Missing preparation is an actionable precondition, leaves the session
    /// usable, and can be followed by explicit preparation and retry.
    pub fn first_binding(&mut self, key: TermKey) -> Result<SelectedBinding, IndexedError> {
        if let Some(cause) = &self.failure {
            return Err(cause.clone());
        }
        let result = self.source.check_current().and_then(|()| {
            if !self.prepared_segments.contains(&key.segment) {
                return Err(IndexedError::ReferencePreparationRequired(key.segment));
            }
            self.first_binding_inner(key)
        });
        self.binding_entries = 0;
        let result = result.and_then(|selected| {
            self.observe_metadata(0)?;
            self.source.check_current()?;
            Ok(selected)
        });
        if let Err(cause) = &result
            && !matches!(cause, IndexedError::ReferencePreparationRequired(_))
        {
            self.failure = Some(cause.clone());
        }
        result
    }

    fn first_binding_inner(&mut self, key: TermKey) -> Result<SelectedBinding, IndexedError> {
        let selected = self.get_inner(key)?;
        drop(selected.term);
        self.source.check_current()?;
        if let Some(proof) = self.bindings.iter().find(|proof| proof.key == key) {
            let actual = self.storage.first_binding(key)?;
            self.source.check_current()?;
            if actual != proof.triple {
                return Err(DictionaryError::Corrupt(
                    "stored binding differs from source-ordered evidence",
                )
                .into());
            }
            return Ok(SelectedBinding {
                triple: proof.triple,
                diagnostics: proof.diagnostics.clone(),
            });
        }
        if self.storage.first_binding(key)?.is_some() {
            return Err(DictionaryError::Corrupt("unproven stored first binding").into());
        }
        self.source.check_current()?;
        let catalog = self
            .catalogs
            .iter()
            .position(|catalog| catalog.segment() == key.segment)
            .expect("selected term retains its segment catalog");
        let mut reifiers = ReifierIndex::default();
        let mut indirect = FastSet::default();
        let mut rebound = Vec::new();
        let mut diagnostics = selected.diagnostics;
        let mut graph = Graph::default();
        let mut terms = 0usize;
        let limit = self.source.limits().metadata_entries;
        for position in 0..self.catalogs[catalog].frame_count() {
            let base = TermIdBase(terms);
            let item = self.catalogs[catalog].frame_item(position)?;
            let required = self
                .entries()
                .saturating_sub(self.binding_entries)
                .saturating_add(rebound.len());
            terms = self.source.reference_rows(
                &self.catalogs[catalog],
                position,
                terms,
                |term_id, references, _diagnostics| {
                    let anchor_entries = references.anchor(term_id).map_or(0usize, |rid| {
                        1usize.saturating_add(usize::from(reifiers.anchors(rid).is_empty()))
                    });
                    let legacy = (references.kind == TermKind::Triple
                        && references.triple.is_none())
                    .then_some(references.reifier)
                    .flatten()
                    .filter(|rid| *rid == key.term);
                    let indirect_entries =
                        usize::from(legacy.is_some_and(|rid| !indirect.contains(&rid)));
                    let required = required
                        .saturating_add(reifiers.retained_entries())
                        .saturating_add(indirect.len())
                        .saturating_add(diagnostics.len())
                        .saturating_add(anchor_entries)
                        .saturating_add(indirect_entries);
                    if required > limit {
                        return Err(DictionaryError::Limit {
                            resource: "indexed-reifier-metadata",
                            required,
                            limit,
                        }
                        .into());
                    }
                    if let Some(rid) = legacy {
                        indirect.insert(rid);
                    }
                    reifiers.observe_references(term_id, references);
                    // These are authenticated navigation projections, not raw
                    // term admission or global semantic diagnostics. Required
                    // RDF terms still enter the original raw decoder below.
                    Ok(())
                },
            )?;
            self.binding_checkpoint(
                reifiers
                    .retained_entries()
                    .saturating_add(indirect.len())
                    .saturating_add(rebound.len())
                    .saturating_add(diagnostics.len()),
            )?;
            let kind = self.catalogs[catalog].frame_kind(position);
            if !matches!(kind, Some("reifies" | "snapshot")) {
                continue;
            }
            let snapshot = kind == Some("snapshot");
            let payload = self.source.payload(&self.catalogs[catalog], position)?;
            let rows = if snapshot {
                ValueRef::Owned(&payload)
                    .get("reifies")
                    .map_err(IndexedError::native)?
            } else {
                Some(ValueRef::Owned(&payload))
            };
            let Some(rows) = rows else {
                continue;
            };
            let array = rows.array().map_err(IndexedError::native)?;
            let Some(rows_array) = array else {
                if !snapshot || rows.is_map() {
                    diagnostics.push(
                        term_decode::TermDiagnostic::Fold(FoldError::ReifiesArray { snapshot })
                            .into_diagnostic(item),
                    );
                }
                continue;
            };
            for row in rows_array {
                let raw = numeric_row::<5>(row.map_err(IndexedError::native)?)
                    .map_err(IndexedError::native)?;
                let row_base = if snapshot { base } else { TermIdBase(0) };
                let relevant = match raw
                    .as_ref()
                    .map(|row| row_base.integer_value(row.values[0]))
                {
                    Some(Ok(Some(rid))) => usize::try_from(rid).ok() == Some(key.term),
                    // Unrepresentable or absent reifier ids cannot introduce
                    // this requested key. The shared decoder still classifies
                    // their rows; this selected operation makes no global claim.
                    Some(Ok(None) | Err(_)) | None => false,
                };
                let limits = self.source.limits();
                self.binding_checkpoint(
                    reifiers
                        .retained_entries()
                        .saturating_add(indirect.len())
                        .saturating_add(rebound.len())
                        .saturating_add(diagnostics.len()),
                )?;
                let mut failure = None;
                let (decoded, recursive) = {
                    let remaining = limits.metadata_entries.saturating_sub(self.entries());
                    let view = MetadataTerms {
                        dictionary: RefCell::new(&mut *self),
                        segment: key.segment,
                        count: terms,
                        traversal_limit: remaining,
                    };
                    let decoded = decode_reifier_row(raw, &view, row_base, &mut failure);
                    let recursive = match &decoded {
                        RowDecode::Row((rid, triple, _)) => {
                            reifiers.binding_is_recursive(&view, *rid, *triple)?
                        }
                        _ => false,
                    };
                    (decoded, recursive)
                };
                let (rid, triple, graph_slot) = match decoded {
                    RowDecode::Skip => continue,
                    RowDecode::Storage => return Err(take_storage_failure(&mut failure).into()),
                    RowDecode::Damaged(cause) => {
                        if relevant {
                            diagnostics.push(
                                term_decode::TermDiagnostic::Row(*cause).into_diagnostic(item),
                            );
                        }
                        continue;
                    }
                    RowDecode::Position(cause) => {
                        if relevant {
                            diagnostics.push(
                                Diagnostic {
                                    code: "PositionConstraint".into(),
                                    detail: cause.to_string(),
                                    frame_index: Some(item),
                                    cause: None,
                                }
                                .with_native_cause(*cause),
                            );
                        }
                        continue;
                    }
                    RowDecode::Row(row) => row,
                };
                let previous = reifiers.first_binding(rid);
                let is_rebound = relevant && previous.is_some_and(|previous| previous != triple);
                self.binding_checkpoint(
                    reifiers
                        .retained_entries()
                        .saturating_add(indirect.len())
                        .saturating_add(rebound.len())
                        .saturating_add(usize::from(is_rebound))
                        .saturating_add(diagnostics.len())
                        .saturating_add(usize::from(
                            (recursive && relevant) || (!recursive && previous.is_none()),
                        )),
                )?;
                if is_rebound {
                    rebound.push((rid, item));
                }
                if recursive {
                    if relevant {
                        diagnostics.push(
                            term_decode::TermDiagnostic::Fold(FoldError::RecursiveReifier {
                                reifier: rid,
                                triple,
                            })
                            .into_diagnostic(item),
                        );
                    }
                    continue;
                }
                reifiers.record(&mut graph, (rid, triple, graph_slot), false);
            }
            self.binding_checkpoint(
                reifiers
                    .retained_entries()
                    .saturating_add(indirect.len())
                    .saturating_add(rebound.len())
                    .saturating_add(diagnostics.len()),
            )?;
        }
        for (rid, item) in rebound {
            if indirect.contains(&rid) {
                diagnostics.push(
                    term_decode::TermDiagnostic::Fold(FoldError::ConflictingReifier {
                        reifier: rid,
                    })
                    .into_diagnostic(item),
                );
            }
        }
        let triple = reifiers.first_binding(key.term);
        self.admit(1usize.saturating_add(diagnostics.len()))?;
        self.observe_metadata(1usize.saturating_add(diagnostics.len()))?;
        if let Some(triple) = triple {
            self.storage.bind_first(key, triple)?;
        }
        self.storage.commit()?;
        self.source.check_current()?;
        if self.storage.first_binding(key)? != triple {
            return Err(DictionaryError::Corrupt(
                "committed first binding differs from accepted source row",
            )
            .into());
        }
        self.source.check_current()?;
        let result = SelectedBinding {
            triple,
            diagnostics: diagnostics.clone(),
        };
        self.bindings.push(AdmittedBinding {
            key,
            triple,
            diagnostics,
        });
        Ok(result)
    }
}

struct MetadataTerms<'a, R: Read + Seek, C, S> {
    dictionary: RefCell<&'a mut IndexedDictionary<R, C, S>>,
    segment: u64,
    count: usize,
    traversal_limit: usize,
}
fn dictionary_failure(cause: IndexedError) -> DictionaryError {
    match cause {
        IndexedError::Dictionary(cause) => cause,
        cause => purrdf_hash::error::io_error(io::ErrorKind::Other, cause).into(),
    }
}
impl<R: Read + Seek, C: FnMut(&R) -> io::Result<AccessBinding>, S: TermStorage> TermMetadata
    for &MetadataTerms<'_, R, C, S>
{
    fn len(self) -> usize {
        self.count
    }
    fn kind(self, id: usize) -> Result<TermKind, DictionaryError> {
        self.references(id)?
            .map(|references| references.kind)
            .ok_or(DictionaryError::Missing(TermKey {
                segment: self.segment,
                term: id,
            }))
    }
    fn references(self, id: usize) -> Result<Option<TermReferences>, DictionaryError> {
        if id >= self.count {
            return Ok(None);
        }
        let mut dictionary = self
            .dictionary
            .try_borrow_mut()
            .map_err(|_| DictionaryError::Corrupt("indexed dependency lookup reentered"))?;
        let dictionary = &mut **dictionary;
        dictionary
            .source
            .check_current()
            .map_err(dictionary_failure)?;
        dictionary.storage.checkpoint(1)?;
        let catalog = dictionary
            .catalogs
            .iter()
            .position(|catalog| catalog.segment() == self.segment)
            .ok_or(DictionaryError::Corrupt(
                "prepared reference catalog is absent",
            ))?;
        dictionary
            .source
            .references_at(&dictionary.catalogs[catalog], id)
            .map(Some)
            .map_err(dictionary_failure)
    }
    fn traversal_limit(self) -> Option<usize> {
        Some(self.traversal_limit)
    }
}
