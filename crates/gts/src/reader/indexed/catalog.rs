// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Structural frame discovery and independently checked index locators.

use super::{BoundRange, DecodeCapabilities, IndexedError, IndexedInput, LookupPolicy};
use crate::dictionary::DictionaryError;
use crate::model::TermKind;
use crate::reader::term_decode::{self, TermDiagnostic, TermReferences};
use crate::reader::{FoldError, Payload, as_i128, catalog_from, payload};
use crate::reader_rows::TermIdBase;
use crate::wire::{MAGIC, VERSION, header_id, map_get};
use purrdf_lex::cbor::{EncodedItem, Value, ValueRef};
use std::fmt;
use std::io::{self, Read, Seek};
use std::sync::Arc;

/// An unavailable segment/index or a precise contradiction in an index claim.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum CatalogError {
    /// The requested segment does not exist in the pinned source.
    SegmentMissing(u64),
    /// The segment has no usable locator footer; no unindexed full-reader
    /// fallback ran. A footer with only head/MMR claims is not a locator.
    IndexUnavailable(u64),
    /// A locator claim contradicts observed source structure.
    Contradiction {
        /// Zero-based segment-local frame position, excluding its header.
        frame: usize,
        /// Stable identity of the contradicted invariant.
        rule: &'static str,
    },
}
impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SegmentMissing(segment) => write!(f, "indexed segment {segment} is absent"),
            Self::IndexUnavailable(segment) => {
                write!(f, "segment {segment} has no dictionary index")
            }
            Self::Contradiction { frame, rule } => {
                write!(f, "indexed frame {frame} contradicts {rule}")
            }
        }
    }
}
impl std::error::Error for CatalogError {}
impl purrdf_hash::error::StructuredError for CatalogError {
    purrdf_hash::error_visit! {
        SegmentMissing(segment) => "gts-indexed-segment-missing" {
            "segment" => Unsigned(u128::from(*segment))
        },
        IndexUnavailable(segment) => "gts-indexed-unavailable" {
            "segment" => Unsigned(u128::from(*segment))
        },
        Contradiction { frame, rule } => "gts-indexed-contradiction" {
            "frame" => Unsigned(*frame as u128), "rule" => Code(rule)
        },
    }
}
fn contradiction(frame: usize, rule: &'static str) -> IndexedError {
    IndexedError::Catalog(CatalogError::Contradiction { frame, rule })
}

#[derive(Debug)]
struct FrameLocation {
    range: BoundRange,
    kind: Option<String>,
    id: Option<Vec<u8>>,
    data: Option<BoundRange>,
    transforms: Option<BoundRange>,
}
struct Discovered {
    header: bool,
    kind: Option<BoundRange>,
    id: Option<BoundRange>,
    data: Option<BoundRange>,
    transforms: Option<BoundRange>,
}

/// One source-bound append-order term interval, found by actual map-row counts.
/// Non-map rows consume no term id. Snapshots rebase their local references at
/// the interval's first id, just as the sequential reader does.
#[derive(Clone, Debug)]
pub struct TermBlock {
    pub(super) frame: usize,
    pub(super) first: usize,
    pub(super) end: usize,
    pub(super) snapshot: bool,
    range: BoundRange,
}
impl TermBlock {
    /// First segment-local term id contributed by this block.
    pub const fn first(&self) -> usize {
        self.first
    }
    /// Segment-local term id immediately after this block's last term.
    pub const fn end(&self) -> usize {
        self.end
    }
    /// Whether this interval rebases snapshot-local term references.
    pub const fn is_snapshot(&self) -> bool {
        self.snapshot
    }
}

/// Source-bound catalog of one segment, with its last index checked against
/// observed frame extents and kinds. This is not full payload verification.
#[derive(Debug)]
pub struct SegmentCatalog {
    segment: u64,
    header_item: u64,
    header_range: BoundRange,
    header: Vec<(Value, Value)>,
    frames: Vec<FrameLocation>,
    index: usize,
}
impl SegmentCatalog {
    /// Original segment ordinal, used by dictionary keys and blank scopes.
    pub const fn segment(&self) -> u64 {
        self.segment
    }
    /// Number of observed frames, including the index and its legal tail.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    /// Number of frames covered by the last checked index. Its own position is
    /// this number; any later frames are an accretive tail.
    pub const fn covered_frames(&self) -> usize {
        self.index
    }
    pub(super) fn metadata_entries(&self) -> usize {
        self.header.len().saturating_add(self.frames.len())
    }
    pub(super) fn frame_capacity(&self) -> usize {
        self.frames.capacity()
    }
    pub(super) fn frame_kind(&self, position: usize) -> Option<&str> {
        self.frames.get(position)?.kind.as_deref()
    }
    pub(super) fn frame_item(&self, frame: usize) -> Result<u64, IndexedError> {
        self.header_item
            .checked_add(1)
            .and_then(|item| item.checked_add(frame as u64))
            .ok_or(IndexedError::Overflow("frame ordinal"))
    }
}

/// Bounded structural discovery over a caller-owned bound seek input.
/// Opening captures no dictionary payload and performs no full-file validation.
pub struct IndexedCatalog<R: Read + Seek, C> {
    input: IndexedInput<R, C>,
    capabilities: DecodeCapabilities,
    policy: Arc<LookupPolicy>,
    verified_frames: u128,
    decoded_payloads: u128,
}
purrdf_hash::debug_non_exhaustive!([R: Read + Seek, C] IndexedCatalog<R, C> {});

impl<R: Read + Seek, C: FnMut(&R) -> io::Result<super::AccessBinding>> IndexedCatalog<R, C> {
    /// Bind the actual decoder to an already captured input without scanning.
    /// # Errors
    /// Returns the original source/context failure before testing whether this
    /// resolver owns the input's captured capabilities.
    pub fn new(
        mut input: IndexedInput<R, C>,
        capabilities: DecodeCapabilities,
        policy: Arc<LookupPolicy>,
    ) -> Result<Self, IndexedError> {
        input.check_current()?;
        if input.binding().capabilities != *capabilities.owner() {
            return Err(IndexedError::CapabilityOwner);
        }
        if input.binding().policy != policy.owner() {
            return Err(IndexedError::PolicyMismatch("owner"));
        }
        if input.limits() != policy.limits {
            return Err(IndexedError::PolicyMismatch("reader-limits"));
        }
        if policy.expected_head.is_some() {
            return Err(IndexedError::FullVerificationRequired("expected-head"));
        }
        Ok(Self {
            input,
            capabilities,
            policy,
            verified_frames: 0,
            decoded_payloads: 0,
        })
    }
    /// Actual underlying reads, seeks and identity checks, including discovery.
    pub fn work(&self) -> super::IndexedWork {
        super::IndexedWork {
            verified_frames: self.verified_frames,
            decoded_payloads: self.decoded_payloads,
            ..self.input.work()
        }
    }
    /// Immutable byte and metadata limits captured when the input was opened.
    pub fn limits(&self) -> crate::dictionary::DictionaryLimits {
        self.input.limits()
    }

    /// Verify source and capabilities before any retained catalog/cache hit.
    /// # Errors
    /// Returns the original host refusal or precise binding change.
    pub fn check_current(&mut self) -> Result<(), IndexedError> {
        self.input.check_current()
    }

    // One metadata walk retains first-occurrence semantics without repeatedly
    // refilling replay windows on either side of a large unvisited payload.
    fn inspect(&mut self, range: &BoundRange) -> Result<Discovered, IndexedError> {
        let inner = self
            .input
            .tag(range)?
            .map_or_else(|| range.clone(), |(_, child)| child);
        let mut discovered = Discovered {
            header: false,
            kind: None,
            id: None,
            data: None,
            transforms: None,
        };
        let Some(mut entries) = self.input.map(&inner)? else {
            return Ok(discovered);
        };
        let mut magic = false;
        while let Some(key) = self.input.next(&mut entries)? {
            let Some(value) = self.input.next(&mut entries)? else {
                return Err(contradiction(0, "map-key-without-value"));
            };
            if self.input.major(&key)? == 3 {
                match self.input.decode(&key)?.as_text() {
                    Some("gts") => magic = true,
                    Some("t") if discovered.kind.is_none() => discovered.kind = Some(value),
                    Some("id") if discovered.id.is_none() => discovered.id = Some(value),
                    Some("d") if discovered.data.is_none() => discovered.data = Some(value),
                    Some("x") if discovered.transforms.is_none() => {
                        discovered.transforms = Some(value);
                    }
                    _ => {}
                }
            }
        }
        discovered.header = magic && discovered.kind.is_none();
        Ok(discovered)
    }

    fn admit_metadata(&self, required: usize) -> Result<(), IndexedError> {
        let limit = self.input.limits().metadata_entries;
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

    /// Discover one segment without decoding preceding segment headers or
    /// unrelated frame payloads. The latest index's offsets, type coverage,
    /// dictionary locators, head and optional MMR are checked independently of
    /// unvisited payload hashes. Legal tail contributors remain in the catalog.
    ///
    /// # Errors
    /// Returns original I/O/grammar/capability/budget refusals, invalid header or
    /// index claims, or an explicit unavailable-index error.
    pub fn segment(&mut self, requested: u64) -> Result<SegmentCatalog, IndexedError> {
        self.input.check_current()?;
        let mut offset = 0;
        let mut ordinal = 0u64;
        let mut header_range = None;
        let mut seen_header = false;
        let mut item = 0u64;
        while let Some(range) = self.input.scan(offset)? {
            offset = range.end();
            if !self.inspect(&range)?.header {
                if !seen_header {
                    return Err(IndexedError::native(FoldError::FirstHeader));
                }
                item = item
                    .checked_add(1)
                    .ok_or(IndexedError::Overflow("item ordinal"))?;
                continue;
            }
            if seen_header && !self.policy.allow_segments {
                return Err(IndexedError::native(FoldError::SegmentBoundary { item }));
            }
            if ordinal == requested {
                header_range = Some(range);
                break;
            }
            // An unsupported prefix tag creates no completed segment in the
            // original reader. Refuse its original interpretation error rather
            // than inventing a later segment ordinal. No prior dct body is read.
            if let Some((tag, _)) = self.input.tag(&range)?
                && tag != crate::wire::SELF_DESCRIBE_TAG
            {
                return Err(IndexedError::native(crate::wire::HeaderError::Tag {
                    actual: tag,
                }));
            }
            seen_header = true;
            item = item
                .checked_add(1)
                .ok_or(IndexedError::Overflow("item ordinal"))?;
            ordinal = ordinal
                .checked_add(1)
                .ok_or(IndexedError::Overflow("segment ordinal"))?;
        }
        if header_range.is_none() {
            return Err(IndexedError::Catalog(CatalogError::SegmentMissing(
                requested,
            )));
        }
        let header_range = header_range.expect("checked selected header presence");
        let header = crate::wire::into_header(self.input.decode(&header_range)?)
            .map_err(IndexedError::native)?;
        let stored = bytes(map_get(&header, "id"));
        let computed = header_id(&header);
        if stored.as_deref() != Some(computed.as_slice()) {
            return Err(IndexedError::native(FoldError::HeaderHash {
                stored,
                computed,
            }));
        }
        if map_get(&header, "gts").and_then(Value::as_text) != Some(MAGIC)
            || map_get(&header, "v").and_then(as_i128) != Some(i128::from(VERSION))
        {
            return Err(IndexedError::native(FoldError::HeaderVersion {
                magic: map_get(&header, "gts").cloned(),
                version: map_get(&header, "v").cloned(),
            }));
        }
        let mut catalog = SegmentCatalog {
            segment: requested,
            header_item: item,
            header_range,
            header,
            frames: Vec::new(),
            index: 0,
        };
        self.admit_metadata(catalog.header.len())?;
        let mut last_index = None;
        while let Some(range) = self.input.scan(offset)? {
            let discovered = self.inspect(&range)?;
            if discovered.header {
                if !self.policy.allow_segments {
                    return Err(IndexedError::native(FoldError::SegmentBoundary {
                        item: catalog.frame_item(catalog.frames.len())?,
                    }));
                }
                break;
            }
            offset = range.end();
            self.admit_metadata(
                catalog
                    .header
                    .len()
                    .saturating_add(catalog.frames.len())
                    .saturating_add(1),
            )?;
            let kind = match discovered
                .kind
                .map(|range| self.input.decode(&range))
                .transpose()?
                .as_ref()
            {
                Some(Value::Text(kind)) => Some(kind.clone()),
                _ => None,
            };
            let id = bytes(
                discovered
                    .id
                    .map(|range| self.input.decode(&range))
                    .transpose()?
                    .as_ref(),
            );
            if kind.as_deref() == Some("index") {
                last_index = Some(catalog.frames.len());
            }
            catalog.frames.push(FrameLocation {
                range,
                kind,
                id,
                data: discovered.data,
                transforms: discovered.transforms,
            });
        }
        catalog.index = last_index.ok_or(IndexedError::Catalog(CatalogError::IndexUnavailable(
            requested,
        )))?;
        self.verify_index(&catalog)?;
        self.input.check_current()?;
        Ok(catalog)
    }

    /// Validate one selected frame's self-hash and source-local predecessor.
    /// Other payloads remain unvisited; this does not authenticate their ids.
    /// # Errors
    /// Returns original source/decoding/budget failures or native chain refusal.
    pub fn frame(
        &mut self,
        catalog: &SegmentCatalog,
        position: usize,
    ) -> Result<Value, IndexedError> {
        self.checked_encoded_frame(catalog, position)?
            .decode()
            .map_err(IndexedError::native)
    }
    fn checked_encoded_frame(
        &mut self,
        catalog: &SegmentCatalog,
        position: usize,
    ) -> Result<EncodedItem, IndexedError> {
        self.input.check_current()?;
        let location = catalog
            .frames
            .get(position)
            .ok_or_else(|| contradiction(position, "frame-position"))?;
        let encoded = self.input.encoded(&location.range)?;
        {
            let parsed = crate::reader::encoded_frame::Frame::new(encoded.view())
                .map_err(IndexedError::native)?;
            let Value::Map(entries) = &parsed.metadata else {
                return Err(IndexedError::native(FoldError::FrameMap));
            };
            let stored = bytes(map_get(entries, "id"));
            self.verified_frames = self
                .verified_frames
                .checked_add(1)
                .ok_or(IndexedError::Overflow("verified frame observations"))?;
            let computed = parsed
                .computed
                .expect("the original encoded frame parser hashes maps");
            if stored.as_deref() != Some(computed.as_slice()) {
                return Err(IndexedError::native(FoldError::SelfHash {
                    stored,
                    computed,
                }));
            }
            let expected = if position == 0 {
                bytes(map_get(&catalog.header, "id"))
            } else {
                catalog.frames[position - 1].id.clone()
            }
            .unwrap_or_default();
            let actual = bytes(map_get(entries, "prev"));
            if actual.as_deref() != Some(expected.as_slice()) {
                return Err(IndexedError::native(FoldError::Previous {
                    actual,
                    expected,
                }));
            }
        }
        Ok(encoded)
    }
    /// Authenticate every observed dictionary/snapshot contributor for explicit
    /// reference access, retaining no raw term or complete value tree. This is
    /// dictionary-frame admission, not global RDF/signature verification.
    /// # Errors
    /// Returns original source/admission/budget/integrity refusals.
    pub fn verify_reference_contributors(
        &mut self,
        catalog: &SegmentCatalog,
    ) -> Result<(), IndexedError> {
        self.input.check_current()?;
        for (position, frame) in catalog.frames.iter().enumerate() {
            if matches!(frame.kind.as_deref(), Some("terms" | "snapshot")) {
                self.checked_encoded_frame(catalog, position)?;
            }
        }
        self.input.check_current()
    }

    pub(super) fn payload(
        &mut self,
        catalog: &SegmentCatalog,
        position: usize,
    ) -> Result<Value, IndexedError> {
        let frame = self.frame(catalog, position)?;
        let Value::Map(entries) = &frame else {
            unreachable!("selected frame checked map")
        };
        let codecs = catalog_from(&catalog.header);
        self.admit_metadata(
            catalog
                .header
                .len()
                .saturating_add(catalog.frames.len())
                .saturating_add(codecs.len()),
        )?;
        self.decoded_payloads = self
            .decoded_payloads
            .checked_add(1)
            .ok_or(IndexedError::Overflow("payload decode observations"))?;
        let decoded = payload::PayloadContext {
            catalog: &codecs,
            content_key: Some(self.capabilities.resolver()),
        }
        .decode(
            entries,
            false,
            map_get(entries, "d").map(ValueRef::Owned),
            || Some(self.input.limits().decoded_frame_bytes),
            || None,
        )
        .map_err(IndexedError::native)?;
        match decoded {
            Payload::Value(value) => Ok(value
                .map(ValueRef::to_value)
                .transpose()
                .map_err(IndexedError::native)?
                .map_or(Value::Null, std::borrow::Cow::into_owned)),
            Payload::Encoded(value) => value.view().decode().map_err(IndexedError::native),
            Payload::Blob(_) => unreachable!("dictionary/index decoding is not a blob"),
        }
    }

    fn field(
        &mut self,
        range: &BoundRange,
        name: &str,
    ) -> Result<Option<BoundRange>, IndexedError> {
        let Some(mut cursor) = self.input.map(range)? else {
            return Ok(None);
        };
        while let Some(key) = self.input.next(&mut cursor)? {
            let Some(value) = self.input.next(&mut cursor)? else {
                return Err(contradiction(0, "map-key-without-value"));
            };
            if self.input.major(&key)? == 3 && self.input.decode(&key)?.as_text() == Some(name) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    /// Visit the admitted reference projection without decoding plain lexical
    /// bodies. Transformed contributors use the original bounded decoder.
    pub(super) fn reference_rows(
        &mut self,
        catalog: &SegmentCatalog,
        position: usize,
        first: usize,
        mut visit: impl FnMut(usize, TermReferences, Vec<TermDiagnostic>) -> Result<(), IndexedError>,
    ) -> Result<Option<TermBlock>, IndexedError> {
        self.input.check_current()?;
        let location = catalog
            .frames
            .get(position)
            .ok_or_else(|| contradiction(position, "reference-frame-position"))?;
        let snapshot = location.kind.as_deref() == Some("snapshot");
        if !snapshot && location.kind.as_deref() != Some("terms") {
            return Ok(None);
        }
        let base = TermIdBase(if snapshot { first } else { 0 });
        let transformed = match location.transforms.as_ref() {
            Some(range) => {
                matches!(self.input.decode(range)?, Value::Array(ref ids) if !ids.is_empty())
            }
            None => false,
        };
        let mut next = first;
        let mut project = |entries: Vec<(Value, Value)>| -> Result<(), IndexedError> {
            let kind = TermKind::from_wire(map_get(&entries, "k").and_then(as_i128));
            let mut diagnostics = Vec::new();
            let references = term_decode::decode_references(&entries, next, kind, base, |cause| {
                diagnostics.push(cause);
            });
            visit(next, references, diagnostics)?;
            next = next
                .checked_add(1)
                .ok_or(IndexedError::Overflow("reference term ordinal"))?;
            Ok(())
        };
        if transformed {
            let payload = self.payload(catalog, position)?;
            let rows = if snapshot {
                ValueRef::Owned(&payload)
                    .get("terms")
                    .map_err(IndexedError::native)?
            } else {
                Some(ValueRef::Owned(&payload))
            };
            if let Some(rows) = rows
                .map(ValueRef::array)
                .transpose()
                .map_err(IndexedError::native)?
                .flatten()
            {
                for row in rows {
                    let row = row.map_err(IndexedError::native)?;
                    if !row.is_map() {
                        continue;
                    }
                    let mut entries = Vec::new();
                    for name in ["k", "dt", "rf", "tt"] {
                        if let Some(value) = row.get(name).map_err(IndexedError::native)? {
                            entries.push((
                                name.into(),
                                value.to_value().map_err(IndexedError::native)?.into_owned(),
                            ));
                        }
                    }
                    project(entries)?;
                }
            }
        } else if let Some(data) = &location.data {
            let selected = if snapshot {
                self.field(data, "terms")?
            } else {
                Some(data.clone())
            };
            if let Some(mut rows) = selected
                .as_ref()
                .map(|range| self.input.array(range))
                .transpose()?
                .flatten()
            {
                while let Some(row) = self.input.next(&mut rows)? {
                    if self.input.major(&row)? != 5 {
                        continue;
                    }
                    let mut entries = Vec::new();
                    for name in ["k", "dt", "rf", "tt"] {
                        if let Some(range) = self.field(&row, name)? {
                            entries.push((name.into(), self.input.decode(&range)?));
                        }
                    }
                    project(entries)?;
                }
            }
        }
        Ok(Some(TermBlock {
            frame: position,
            first,
            end: next,
            snapshot,
            range: catalog.frames[position].range.clone(),
        }))
    }

    /// Locate a term by counting source map rows in preceding contributors.
    /// Plain preceding dictionaries remain structural-only. Transformed
    /// predecessors require their bounded original decoder to establish counts.
    /// No later dictionary block is decoded to answer this operation.
    /// # Errors
    /// Returns original source/capability/resource refusals or missing term.
    pub fn locate_term(
        &mut self,
        catalog: &SegmentCatalog,
        term: usize,
    ) -> Result<TermBlock, IndexedError> {
        self.input.check_current()?;
        let mut first = 0usize;
        for (position, location) in catalog.frames.iter().enumerate() {
            let snapshot = location.kind.as_deref() == Some("snapshot");
            if !snapshot && location.kind.as_deref() != Some("terms") {
                continue;
            }
            let transformed = match location.transforms.as_ref() {
                Some(range) => {
                    matches!(self.input.decode(range)?, Value::Array(ref ids) if !ids.is_empty())
                }
                None => false,
            };
            let mut count = 0usize;
            if transformed {
                let payload = self.payload(catalog, position)?;
                let selected = if snapshot {
                    ValueRef::Owned(&payload)
                        .get("terms")
                        .map_err(IndexedError::native)?
                } else {
                    Some(ValueRef::Owned(&payload))
                };
                if let Some(rows) = selected
                    .map(ValueRef::array)
                    .transpose()
                    .map_err(IndexedError::native)?
                    .flatten()
                {
                    for row in rows {
                        if row.map_err(IndexedError::native)?.is_map() {
                            count = count
                                .checked_add(1)
                                .ok_or(IndexedError::Overflow("term count"))?;
                        }
                    }
                }
            } else if let Some(data) = &location.data {
                let selected = if snapshot {
                    self.field(data, "terms")?
                } else {
                    Some(data.clone())
                };
                if let Some(mut rows) = selected
                    .as_ref()
                    .map(|range| self.input.array(range))
                    .transpose()?
                    .flatten()
                {
                    while let Some(row) = self.input.next(&mut rows)? {
                        if self.input.major(&row)? == 5 {
                            count = count
                                .checked_add(1)
                                .ok_or(IndexedError::Overflow("term count"))?;
                        }
                    }
                }
            }
            let end = first
                .checked_add(count)
                .ok_or(IndexedError::Overflow("term interval"))?;
            if (first..end).contains(&term) {
                return Ok(TermBlock {
                    frame: position,
                    first,
                    end,
                    snapshot,
                    range: location.range.clone(),
                });
            }
            first = end;
        }
        Err(DictionaryError::Missing(crate::dictionary::TermKey {
            segment: catalog.segment,
            term,
        })
        .into())
    }

    /// Decode a located block using its original source stamp and the session's
    /// actual capability snapshot. This checks the enclosing frame's content id
    /// and predecessor before publishing the selected payload.
    /// # Errors
    /// Returns original source/codec/resource failures or foreign block refusal.
    pub fn block_payload(
        &mut self,
        catalog: &SegmentCatalog,
        block: &TermBlock,
    ) -> Result<Value, IndexedError> {
        self.input.major(&block.range)?;
        let expected = catalog
            .frames
            .get(block.frame)
            .ok_or_else(|| contradiction(block.frame, "term-block-frame"))?;
        if block.range.start() != expected.range.start()
            || block.range.end() != expected.range.end()
        {
            return Err(contradiction(block.frame, "term-block-extent"));
        }
        self.payload(catalog, block.frame)
    }

    fn verify_index(&mut self, catalog: &SegmentCatalog) -> Result<(), IndexedError> {
        let position = catalog.index;
        let payload = self.payload(catalog, position)?;
        let Value::Map(payload) = &payload else {
            return Err(contradiction(position, "index-payload-map"));
        };
        if integer(map_get(payload, "count")) != Some(position as u64) {
            return Err(contradiction(position, "index-count"));
        }
        let expected = if position == 0 {
            bytes(map_get(&catalog.header, "id"))
        } else {
            catalog.frames[position - 1].id.clone()
        };
        if bytes(map_get(payload, "head")) != expected {
            return Err(contradiction(position, "index-head"));
        }
        if ["off", "ti", "dict"]
            .iter()
            .all(|name| map_get(payload, name).is_none())
        {
            return Err(IndexedError::Catalog(CatalogError::IndexUnavailable(
                catalog.segment,
            )));
        }
        let offsets = match map_get(payload, "off") {
            Some(Value::Array(offsets)) => Some(offsets),
            None => None,
            _ => return Err(contradiction(position, "index-offset-array")),
        };
        if let Some(offsets) = offsets {
            if offsets.len() != position {
                return Err(contradiction(position, "index-offset-count"));
            }
            for (entry, observed) in offsets.iter().zip(&catalog.frames) {
                if integer(Some(entry))
                    != observed
                        .range
                        .start()
                        .checked_sub(catalog.header_range.start())
                {
                    return Err(contradiction(position, "index-offset-boundary"));
                }
            }
        }
        let kinds = match map_get(payload, "ti") {
            Some(Value::Map(kinds)) => Some(kinds),
            None => None,
            _ => return Err(contradiction(position, "index-type-map")),
        };
        // Each type's complete ordered positions must agree with observations.
        // Repeated map keys retain first-occurrence semantics, like map_get.
        if let Some(kinds) = kinds {
            for (key, _) in kinds {
                let Some(kind) = key.as_text() else {
                    return Err(contradiction(position, "index-type-key"));
                };
                let Some(Value::Array(positions)) = map_get(kinds, kind) else {
                    return Err(contradiction(position, "index-type-positions"));
                };
                if positions.is_empty() {
                    return Err(contradiction(position, "index-empty-type-positions"));
                }
                let mut observed =
                    catalog.frames[..position]
                        .iter()
                        .enumerate()
                        .filter_map(|(index, frame)| {
                            (frame.kind.as_deref() == Some(kind)).then_some(index)
                        });
                for claimed in positions {
                    if integer(Some(claimed)) != observed.next().map(|index| index as u64) {
                        return Err(contradiction(position, "index-type-coverage"));
                    }
                }
                if observed.next().is_some() {
                    return Err(contradiction(position, "index-type-coverage"));
                }
            }
            for frame in &catalog.frames[..position] {
                if frame
                    .kind
                    .as_deref()
                    .is_none_or(|kind| map_get(kinds, kind).is_none())
                {
                    return Err(contradiction(position, "index-omitted-type"));
                }
            }
        }
        if let Some(dict) = map_get(payload, "dict") {
            let Value::Array(dict) = dict else {
                return Err(contradiction(position, "index-dictionary-array"));
            };
            if dict.is_empty() {
                return Err(contradiction(position, "index-empty-dictionary-positions"));
            }
            let mut observed =
                catalog.frames[..position]
                    .iter()
                    .enumerate()
                    .filter_map(|(index, frame)| {
                        (frame.kind.as_deref() == Some("terms")).then_some(index)
                    });
            for claimed in dict {
                if integer(Some(claimed)) != observed.next().map(|index| index as u64) {
                    return Err(contradiction(position, "index-dictionary-coverage"));
                }
            }
            if observed.next().is_some() {
                return Err(contradiction(position, "index-dictionary-coverage"));
            }
        }
        if let Some(root) = map_get(payload, "mmr") {
            let ids = catalog.frames[..position]
                .iter()
                .map(|frame| frame.id.clone())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| contradiction(position, "index-mmr-missing-id"))?;
            if bytes(Some(root)) != Some(crate::mmr::root(&ids)) {
                return Err(contradiction(position, "index-mmr-root"));
            }
        }
        Ok(())
    }
}

fn bytes(value: Option<&Value>) -> Option<Vec<u8>> {
    match value {
        Some(Value::Bytes(bytes)) => Some(bytes.clone()),
        _ => None,
    }
}
fn integer(value: Option<&Value>) -> Option<u64> {
    value
        .and_then(as_i128)
        .and_then(|value| u64::try_from(value).ok())
}

#[cfg(test)]
mod tests;
