// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Selected GTS terms through the authoritative RDF builder and shared resolver.

use super::{
    RdfDiagnostic, SinkImporter, TermId, dictionary_import_failure, fold_diagnostic_failure,
};
use purrdf_gts::dictionary::{TermKey, TermStorage};
use purrdf_gts::model::Triple3;
use purrdf_gts::reader::indexed::{AccessBinding, IndexedDictionary};
use purrdf_gts::segment_decode::{
    PendingResolution, ResidentResolutionStorage, ResolutionCounts, ResolutionStorage,
    ResolutionTerm, SegmentResolver,
};
use std::io::{self, Read, Seek};

/// Resolve one source term and its dependency closure into a fresh native RDF
/// owner. Original IRI/literal/triple validation and segment-scoped blank-node
/// interning run through the authoritative importer callbacks. The returned id
/// belongs only to the returned handle; no destination id cache is shared with
/// another builder. This does not import unrelated rows, materialize the complete
/// dictionary or certify global signatures/expected-head policy. Full-file
/// verification remains a separate explicit reader operation.
/// Indirect quoted terms require the caller's separate
/// [`IndexedDictionary::prepare_reference_access`] step; this operation never
/// initiates that preparation implicitly.
///
/// # Errors
/// Returns original indexed/storage/budget/capability refusals, reader
/// diagnostics or native RDF validation errors without a partial handle.
pub fn import_gts_indexed_term<R, C, S>(
    dictionary: &mut IndexedDictionary<R, C, S>,
    key: TermKey,
) -> Result<(purrdf_core::backend::DatasetHandle, TermId), RdfDiagnostic>
where
    R: Read + Seek,
    C: FnMut(&R) -> io::Result<AccessBinding>,
    S: TermStorage,
{
    let storage = IndexedResolution {
        dictionary,
        state: ResidentResolutionStorage::default(),
    };
    let mut resolver =
        SegmentResolver::with_storage(SinkImporter::new(), storage, std::convert::identity);
    let id = resolver.resolve_selected(key)?;
    let (sink, mut storage) = resolver.into_parts();
    storage.state = ResidentResolutionStorage::default();
    storage.checkpoint(0)?;
    drop(storage);
    Ok((sink.builder.freeze()?.into(), id))
}

// Fresh private state ensures every cached id belongs to this one destination.
// The dictionary owns raw trust and first-binding evidence across calls.
struct IndexedResolution<
    'a,
    R: Read + Seek,
    C: FnMut(&R) -> io::Result<AccessBinding>,
    S: TermStorage,
> {
    dictionary: &'a mut IndexedDictionary<R, C, S>,
    state: ResidentResolutionStorage<TermId>,
}
impl<R: Read + Seek, C: FnMut(&R) -> io::Result<AccessBinding>, S: TermStorage>
    IndexedResolution<'_, R, C, S>
{
    fn checkpoint(&mut self, additional: usize) -> Result<(), RdfDiagnostic> {
        let counts = self.state.counts();
        let entries = counts
            .pending
            .saturating_add(counts.resolved)
            .saturating_add(counts.reifiers)
            .saturating_add(additional);
        self.dictionary
            .resolution_checkpoint(entries)
            .map_err(dictionary_import_failure)
    }
}
impl<R: Read + Seek, C: FnMut(&R) -> io::Result<AccessBinding>, S: TermStorage> Drop
    for IndexedResolution<'_, R, C, S>
{
    fn drop(&mut self) {
        self.state = ResidentResolutionStorage::default();
        self.dictionary.release_resolution_metadata();
    }
}
impl<R: Read + Seek, C: FnMut(&R) -> io::Result<AccessBinding>, S: TermStorage>
    ResolutionStorage<TermId> for IndexedResolution<'_, R, C, S>
{
    type Error = RdfDiagnostic;
    fn pending(&mut self, key: TermKey) -> Result<Option<PendingResolution>, Self::Error> {
        self.checkpoint(0)?;
        if let Some(pending) = self.state.pending(key).map_err(dictionary_import_failure)? {
            return Ok(Some(pending));
        }
        let selected = self
            .dictionary
            .get(key)
            .map_err(dictionary_import_failure)?;
        for diagnostic in selected.diagnostics {
            fold_diagnostic_failure(&diagnostic)?;
        }
        Ok(Some(PendingResolution {
            term: Some(ResolutionTerm::Stored(selected.term)),
            parent: None,
            next: 0,
            active: false,
        }))
    }
    fn set_pending(&mut self, key: TermKey, value: PendingResolution) -> Result<(), Self::Error> {
        let additional = usize::from(
            self.state
                .pending(key)
                .map_err(dictionary_import_failure)?
                .is_none(),
        );
        self.checkpoint(additional)?;
        self.state
            .set_pending(key, value)
            .map_err(dictionary_import_failure)
    }
    fn remove_pending(&mut self, key: TermKey) -> Result<(), Self::Error> {
        self.checkpoint(0)?;
        self.state
            .remove_pending(key)
            .map_err(dictionary_import_failure)
    }
    fn next_term(
        &mut self,
        _segment: u64,
        _after: Option<usize>,
    ) -> Result<Option<usize>, Self::Error> {
        Err(RdfDiagnostic::error(
            "rdf-ir-gts-indexed-enumeration",
            "selected indexed resolution cannot enumerate unrelated terms",
        ))
    }
    fn resolved(&mut self, key: TermKey) -> Result<Option<TermId>, Self::Error> {
        self.checkpoint(0)?;
        self.state.resolved(key).map_err(dictionary_import_failure)
    }
    fn set_resolved(&mut self, key: TermKey, id: TermId) -> Result<(), Self::Error> {
        let additional = usize::from(
            self.state
                .resolved(key)
                .map_err(dictionary_import_failure)?
                .is_none(),
        );
        self.checkpoint(additional)?;
        self.state
            .set_resolved(key, id)
            .map_err(dictionary_import_failure)
    }
    fn first_binding(&mut self, key: TermKey) -> Result<Option<Triple3>, Self::Error> {
        self.checkpoint(0)?;
        let selected = self
            .dictionary
            .first_binding(key)
            .map_err(dictionary_import_failure)?;
        for diagnostic in selected.diagnostics {
            fold_diagnostic_failure(&diagnostic)?;
        }
        Ok(selected.triple)
    }
    fn bind_first(&mut self, key: TermKey, triple: Triple3) -> Result<(), Self::Error> {
        let additional = usize::from(
            self.state
                .first_binding(key)
                .map_err(dictionary_import_failure)?
                .is_none(),
        );
        self.checkpoint(additional)?;
        self.state
            .bind_first(key, triple)
            .map_err(dictionary_import_failure)
    }
    fn complete_segment(&mut self, segment: u64) -> Result<(), Self::Error> {
        self.checkpoint(0)?;
        self.state
            .complete_segment(segment)
            .map_err(dictionary_import_failure)
    }
    fn counts(&self) -> ResolutionCounts {
        self.state.counts()
    }
}
