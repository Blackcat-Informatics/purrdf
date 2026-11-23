// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![expect(
    clippy::result_large_err,
    reason = "preserve native failure data without allocating on the refusal path"
)]

use core::fmt;
use std::borrow::Cow;

use purrdf_lex::cbor::StructuralError;

use super::fields::Fields;
use super::{
    ContentError, DefinitionFindings, FoldError, FormedBudget, FrameEnvelope, FrameField,
    FrameType, Generation, Header, Item, ItemKind, LinkState, MalformedReason, PayloadBudget,
    PayloadCapacity, PayloadDamage, PayloadRefusal, PayloadState, RdfFold, RowFindings,
    SegmentCheckError, SegmentDictionary, SegmentVerifier, StoredState, ValueName, ValueStore,
    WireError,
};
use crate::{FastMap, model::Graph};

/// Limits for materializing an RDF graph from a GTS v2 file.
#[derive(Clone, Copy, Debug)]
pub struct ReadCapacity {
    /// Maximum stored and decoded payload lengths.
    pub payload: PayloadCapacity,
    /// Maximum total bytes copied by front coding.
    pub formed_bytes: u64,
    /// Maximum total bytes admitted for recipe and value rebuilds.
    pub rebuilt_bytes: u64,
    /// Maximum number of CBOR items delimited.
    pub items: u64,
}

impl Default for ReadCapacity {
    fn default() -> Self {
        Self {
            payload: PayloadCapacity {
                stored_bytes_per_frame: 1 << 26,
                decoded_bytes_per_frame: 1 << 26,
                decoded_bytes_per_read: 1 << 36,
            },
            formed_bytes: 1 << 37,
            rebuilt_bytes: 1 << 37,
            items: 1 << 26,
        }
    }
}

/// A GTS file decoded into the graph model of its generation.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadGraph<'a> {
    /// The legacy reader's original result, including its diagnostics.
    First(Graph),
    /// The resolved GTS v2 RDF lines and verified blobs.
    Second(RdfGraph<'a>),
}

/// The RDF and embedded-blob projection of a verified generation 2 file.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct RdfGraph<'a> {
    /// Readable RDF terms and resolved RDF lines.
    pub rdf: RdfFold,
    /// Verified blob declarations in file order.
    pub blobs: Vec<VerifiedBlob<'a>>,
    /// File offsets of row frames whose declared order differs from their rows.
    pub row_order_violations: Vec<usize>,
    /// File offsets whose stored check differs from the compressed bytes.
    pub stored_digest_mismatches: Vec<usize>,
}

/// The declared media type and verified bytes of an embedded blob.
#[derive(Debug)]
#[non_exhaustive]
pub struct VerifiedBlob<'a> {
    /// Blob identity and length.
    pub name: ValueName,
    /// Media type from the declaration.
    pub media_type: Option<&'a str>,
    /// Verified payload, borrowed for plain storage.
    pub bytes: Cow<'a, [u8]>,
}

/// A graph read could not verify or interpret the item at this byte offset.
#[derive(Debug)]
pub struct ReadError {
    /// File-relative start of the affected item.
    pub offset: usize,
    /// The framing, verification or interpretation failure.
    pub failure: ReadFailure,
}

/// The reason an item could not be used to materialize a graph.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadFailure {
    /// No opener was found.
    Empty,
    /// The first item's generation cannot be read.
    Generation(Generation),
    /// The CBOR item could not be delimited.
    Framing(StructuralError),
    /// The item is outside the item count capacity.
    ItemCapacity(u64),
    /// The item failed the structural classification rules.
    Classification(MalformedReason),
    /// A header or envelope field failed validation.
    Wire(WireError),
    /// The registry code is newer than this graph reader.
    Registry(u64),
    /// Lineage does not match the previous segment.
    Lineage,
    /// An item kind requires additional interpretation.
    Item(ItemKind),
    /// A frame type requires additional interpretation.
    Frame(FrameType),
    /// The link to the preceding frame is broken.
    BrokenLink,
    /// The decoded payload failed verification.
    PayloadDamage(PayloadDamage),
    /// The payload could not be checked.
    PayloadRefusal(PayloadRefusal),
    /// The segment verifier could not complete a check.
    Segment(SegmentCheckError),
    /// The commit disagrees with the computed prefix.
    Commit,
    /// The content of a verified frame is malformed or unavailable.
    Content(ContentError),
    /// The verification result has no content for this frame.
    MissingContent,
    /// Some definitions could not be read.
    Definitions(DefinitionFindings),
    /// Some rows could not be resolved.
    Rows(RowFindings),
    /// The annotation multiplicity is wider than the wire format.
    AnnotationWidth,
    /// The declared blob identity disagrees with its decoded bytes.
    BlobDigest,
    /// No verified unit was available to a recipe entry.
    MissingUnit(ValueName),
    /// The recipe entry's length disagrees with the unit it names.
    UnitLength(ValueName),
    /// The recipe or value exceeds the remaining rebuild capacity.
    RebuildCapacity(ValueName),
    /// The graph could not retain the verified content.
    Fold(FoldError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GTS graph read at byte {}: ", self.offset)?;
        match &self.failure {
            ReadFailure::Framing(error) => error.fmt(f),
            ReadFailure::Wire(error) => error.fmt(f),
            ReadFailure::Segment(error) => error.fmt(f),
            ReadFailure::Content(error) => error.fmt(f),
            ReadFailure::Fold(error) => error.fmt(f),
            failure => write!(f, "{failure:?}"),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.failure {
            ReadFailure::Framing(error) => Some(error),
            ReadFailure::Wire(error) => Some(error),
            ReadFailure::Segment(error) => Some(error),
            ReadFailure::Content(error) => Some(error),
            ReadFailure::Fold(error) => Some(error),
            ReadFailure::PayloadDamage(PayloadDamage::DecodeFailed(error)) => Some(error),
            _ => None,
        }
    }
}

/// Decode a GTS file into the graph model of its opening generation.
///
/// The first generation uses the existing reader over the original bytes.
/// The generation 2 graph projection checks framing, links, payloads, commits
/// and RDF positions. It returns an error when a row or term is unreadable or
/// when an item requires content interpretation outside the graph projection.
/// The graph carries no assertion about file authenticity or canonical form.
///
/// # Errors
/// Returns the byte position and reason for the first refusal.
pub fn read(bytes: &[u8], capacity: ReadCapacity) -> Result<ReadGraph<'_>, ReadError> {
    if capacity.items == 0 && !bytes.is_empty() {
        return Err(ReadError {
            offset: 0,
            failure: ReadFailure::ItemCapacity(0),
        });
    }
    let opener = delimit(bytes, 0)?.ok_or(ReadError {
        offset: 0,
        failure: ReadFailure::Empty,
    })?;
    let generation = opener.generation().map_err(|error| ReadError {
        offset: 0,
        failure: ReadFailure::Framing(error),
    })?;
    match generation {
        Generation::First => Ok(ReadGraph::First(crate::reader::read(bytes, true, None))),
        Generation::Second => GraphReader::new(bytes, capacity)
            .read(opener)
            .map(ReadGraph::Second),
        generation => Err(ReadError {
            offset: 0,
            failure: ReadFailure::Generation(generation),
        }),
    }
}

fn delimit(bytes: &[u8], offset: usize) -> Result<Option<Item<'_>>, ReadError> {
    Item::first(&bytes[offset..]).map_err(|error| ReadError {
        offset,
        failure: ReadFailure::Framing(error),
    })
}

struct Segment<'a> {
    verifier: SegmentVerifier,
    dictionary: SegmentDictionary,
    ordinal: Option<u64>,
    file_ordinal: u64,
    units: FastMap<[u8; 32], Unit<'a>>,
}

enum Unit<'a> {
    Blob(usize),
    Chunk(Cow<'a, [u8]>),
}

struct GraphReader<'a> {
    bytes: &'a [u8],
    graph: RdfGraph<'a>,
    values: ValueStore,
    payload: PayloadBudget,
    formed: FormedBudget,
    items: u64,
    item_limit: u64,
    rebuilt_remaining: u64,
}

impl<'a> GraphReader<'a> {
    fn new(bytes: &'a [u8], capacity: ReadCapacity) -> Self {
        Self {
            bytes,
            graph: RdfGraph::default(),
            values: ValueStore::default(),
            payload: PayloadBudget::new(capacity.payload),
            formed: FormedBudget::new(capacity.formed_bytes),
            items: 0,
            item_limit: capacity.items,
            rebuilt_remaining: capacity.rebuilt_bytes,
        }
    }

    fn read(mut self, opener: Item<'a>) -> Result<RdfGraph<'a>, ReadError> {
        self.count_item(0)?;
        let mut segment =
            start_segment(opener, None).map_err(|failure| ReadError { offset: 0, failure })?;
        let mut offset = opener.bytes().len();
        let mut next = self.next_item(offset);
        while let Some(item) = next? {
            self.count_item(offset)?;
            let end = offset + item.bytes().len();
            next = self.next_item(end);
            let observed = self.observe(
                item,
                next.as_ref().ok().and_then(|item| *item),
                offset,
                &mut segment,
            );
            observed.map_err(|failure| ReadError { offset, failure })?;
            offset = end;
        }
        Ok(self.graph)
    }

    fn count_item(&mut self, offset: usize) -> Result<(), ReadError> {
        if self.items == self.item_limit {
            return Err(ReadError {
                offset,
                failure: ReadFailure::ItemCapacity(self.item_limit),
            });
        }
        self.items += 1;
        Ok(())
    }

    fn next_item(&self, offset: usize) -> Result<Option<Item<'a>>, ReadError> {
        if self.items == self.item_limit && offset < self.bytes.len() {
            return Err(ReadError {
                offset,
                failure: ReadFailure::ItemCapacity(self.item_limit),
            });
        }
        delimit(self.bytes, offset)
    }

    fn observe(
        &mut self,
        item: Item<'a>,
        next: Option<Item<'a>>,
        offset: usize,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        let kind = item
            .classify(core::num::NonZeroU64::MIN)
            .map_err(ReadFailure::Classification)?;
        match kind {
            ItemKind::Header => *segment = start_segment(item, Some(segment))?,
            ItemKind::Frame => {
                let frame = item
                    .frame()
                    .map_err(ReadFailure::Framing)?
                    .ok_or(ReadFailure::MissingContent)?;
                let witness = next
                    .map(Item::frame)
                    .transpose()
                    .map_err(ReadFailure::Framing)?
                    .flatten();
                self.observe_frame(frame, witness.as_ref(), offset, segment)?;
            }
            _ => return Err(ReadFailure::Item(kind)),
        }
        Ok(())
    }

    fn observe_frame(
        &mut self,
        frame: FrameEnvelope<'a>,
        witness: Option<&FrameEnvelope<'a>>,
        offset: usize,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        let mut checked = segment
            .verifier
            .check(&frame, witness, &mut self.payload)
            .map_err(ReadFailure::Segment)?;
        if let Some(error) = checked.frame.malformed {
            return Err(ReadFailure::Wire(error));
        }
        if checked.frame.link.state == LinkState::Broken {
            return Err(ReadFailure::BrokenLink);
        }
        if checked.frame.payload.stored == Some(StoredState::Mismatch) {
            super::fields::push(&mut self.graph.stored_digest_mismatches, offset)?;
        }
        match &checked.frame.payload.state {
            PayloadState::Damaged(_) | PayloadState::Unchecked(_) => {
                return Err(payload_failure(checked.frame.payload.state));
            }
            PayloadState::Verified(_) => {}
        }
        let kind = checked
            .frame
            .frame_type
            .and_then(FrameType::from_code)
            .ok_or(ReadFailure::MissingContent)?;
        match kind {
            FrameType::Terms => {
                let reservation = checked.terms.as_mut().ok_or(ReadFailure::MissingContent)?;
                reservation.content = reservation
                    .content
                    .take()
                    .transpose()
                    .map_err(ReadFailure::Content)?
                    .map(Ok);
                let decoded = segment
                    .verifier
                    .decode_terms(&checked, &mut self.formed)
                    .ok_or(ReadFailure::MissingContent)?
                    .map_err(ReadFailure::Content)?;
                let reservation = checked.terms.as_ref().ok_or(ReadFailure::MissingContent)?;
                let findings = segment
                    .dictionary
                    .observe_terms(
                        &mut self.graph.rdf,
                        reservation,
                        Some(decoded),
                        &self.values,
                    )
                    .map_err(ReadFailure::Fold)?;
                if findings.ill_formed_term
                    || findings.forward_reference
                    || findings.position_constraint
                    || !findings.missing_values.is_empty()
                {
                    return Err(ReadFailure::Definitions(findings));
                }
            }
            FrameType::Quads
            | FrameType::ReifierBindings
            | FrameType::Annotations
            | FrameType::EmptyGraphs => {
                self.observe_rows(checked, offset, segment)?;
            }
            FrameType::Blob | FrameType::Chunk | FrameType::Recipe | FrameType::Value => {
                let bytes = verified_payload(checked.frame.payload.state)?;
                let mut fields = frame.read_field(FrameField::Content);
                match kind {
                    FrameType::Blob | FrameType::Chunk => {
                        self.store_unit(kind, &mut fields, bytes, segment)?;
                    }
                    _ => self.rebuild(kind, &mut fields, &bytes, segment)?,
                }
            }
            FrameType::Commit if checked.commit.is_some_and(|commit| commit.valid) => {}
            FrameType::Commit => return Err(ReadFailure::Commit),
            kind => return Err(ReadFailure::Frame(kind)),
        }
        Ok(())
    }

    fn observe_rows(
        &mut self,
        checked: super::SegmentFrame<'_>,
        offset: usize,
        segment: &mut Segment<'_>,
    ) -> Result<(), ReadFailure> {
        if let Some(Err(error)) = checked.rows {
            return Err(ReadFailure::Content(error));
        }
        let decoded = segment
            .verifier
            .decode_rows(&checked)
            .ok_or(ReadFailure::MissingContent)?
            .map_err(ReadFailure::Content)?;
        if decoded.declared_order_violated {
            super::fields::push(&mut self.graph.row_order_violations, offset)?;
        }
        let findings = segment
            .dictionary
            .fold_rows(&mut self.graph.rdf, &decoded)
            .map_err(|error| match error {
                FoldError::RepeatedAnnotation => ReadFailure::Content(
                    WireError::new(MalformedReason::UnorderedOrRepeated, 0).into(),
                ),
                error => ReadFailure::Fold(error),
            })?;
        if findings.forward_reference || findings.opaque_term || findings.position_constraint {
            return Err(ReadFailure::Rows(findings));
        }
        if self.graph.rdf.annotation_width_exceeded() {
            return Err(ReadFailure::AnnotationWidth);
        }
        Ok(())
    }

    fn store_unit(
        &mut self,
        kind: FrameType,
        fields: &mut Fields<'a>,
        bytes: Cow<'a, [u8]>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        let blob = kind == FrameType::Blob;
        fields.record(if blob { 2 } else { 1 })?;
        let digest = fields.fixed_bytes::<32>()?;
        let media_type = if blob {
            fields.option(Fields::text)?
        } else {
            None
        };
        let name = ValueName::of(&bytes);
        if name.digest != *digest {
            return Err(ReadFailure::BlobDigest);
        }
        segment.units.try_reserve(1)?;
        let unit = if blob {
            let index = self.graph.blobs.len();
            self.store_blob(VerifiedBlob {
                name,
                media_type,
                bytes,
            })?;
            Unit::Blob(index)
        } else {
            Unit::Chunk(bytes)
        };
        segment.units.entry(*digest).or_insert(unit);
        Ok(())
    }

    fn rebuild(
        &mut self,
        kind: FrameType,
        content: &mut Fields<'a>,
        bytes: &[u8],
        segment: &Segment<'a>,
    ) -> Result<(), ReadFailure> {
        let recipe = kind == FrameType::Recipe;
        content.record(if recipe { 4 } else { 3 })?;
        let digest = *content.fixed_bytes()?;
        let length = content.uint()?;
        let name = ValueName { digest, length };
        let media_type = if recipe {
            content.option(Fields::text)?
        } else {
            None
        };
        let offset = content.offset();
        let entry_count = content.uint()?;
        if entry_count < 1 + u64::from(recipe) {
            return Err(WireError::new(MalformedReason::OutOfRange, offset).into());
        }
        let entries = recipe_entries(bytes, entry_count, length)?;
        self.rebuilt_remaining = self
            .rebuilt_remaining
            .checked_sub(length)
            .ok_or(ReadFailure::RebuildCapacity(name))?;
        let mut output = Vec::new();
        for entry in entries {
            let unit = segment
                .units
                .get(&entry.digest)
                .ok_or(ReadFailure::MissingUnit(entry))?;
            let bytes = match unit {
                Unit::Blob(index) => self.graph.blobs[*index].bytes.as_ref(),
                Unit::Chunk(bytes) => bytes.as_ref(),
            };
            if bytes.len() as u64 != entry.length {
                return Err(ReadFailure::UnitLength(entry));
            }
            output.try_reserve(bytes.len())?;
            output.extend_from_slice(bytes);
        }
        if recipe {
            if ValueName::of(&output) != name {
                return Err(ReadFailure::BlobDigest);
            }
            self.store_blob(VerifiedBlob {
                name,
                media_type,
                bytes: Cow::Owned(output),
            })?;
        } else {
            self.values
                .record(name, Some(output))
                .map_err(ReadFailure::Fold)?;
        }
        Ok(())
    }

    fn store_blob(&mut self, blob: VerifiedBlob<'a>) -> Result<(), ReadFailure> {
        self.graph.blobs.try_reserve(1)?;
        self.graph.blobs.push(blob);
        Ok(())
    }
}

fn verified_payload(state: PayloadState<'_>) -> Result<Cow<'_, [u8]>, ReadFailure> {
    match state {
        PayloadState::Verified(bytes) => Ok(bytes),
        state => Err(payload_failure(state)),
    }
}

fn payload_failure(state: PayloadState<'_>) -> ReadFailure {
    match state {
        PayloadState::Damaged(damage) => ReadFailure::PayloadDamage(damage),
        PayloadState::Unchecked(reason) => ReadFailure::PayloadRefusal(reason),
        PayloadState::Verified(_) => ReadFailure::MissingContent,
    }
}

fn recipe_entries(bytes: &[u8], declared: u64, length: u64) -> Result<Vec<ValueName>, ReadFailure> {
    let mut fields = Fields::new(bytes);
    let count = fields.array()?;
    let mut entries = Vec::new();
    let mut offsets = Vec::new();
    for _ in 0..count {
        fields.record(2)?;
        let digest = *fields.fixed_bytes()?;
        let offset = fields.offset();
        let length = fields.uint()?;
        super::fields::push(&mut entries, ValueName { digest, length })?;
        super::fields::push(&mut offsets, offset)?;
    }
    fields.finish()?;
    if count != declared {
        return Err(WireError::new(MalformedReason::InconsistentCounts, 0).into());
    }
    if let Some(index) = entries.iter().position(|name| name.length == 0) {
        return Err(WireError::new(MalformedReason::OutOfRange, offsets[index]).into());
    }
    let mut total = 0_u64;
    for (entry, offset) in entries.iter().zip(offsets) {
        total = total
            .checked_add(entry.length)
            .ok_or_else(|| WireError::new(MalformedReason::OutOfRange, offset))?;
    }
    if total != length {
        return Err(WireError::new(MalformedReason::InconsistentCounts, 0).into());
    }
    Ok(entries)
}

impl From<WireError> for ReadFailure {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

impl From<std::collections::TryReserveError> for ReadFailure {
    fn from(error: std::collections::TryReserveError) -> Self {
        Self::Fold(error.into())
    }
}

fn start_segment<'a>(
    opener: Item<'a>,
    previous: Option<&Segment<'a>>,
) -> Result<Segment<'a>, ReadFailure> {
    let kind = opener
        .classify(core::num::NonZeroU64::MIN)
        .map_err(ReadFailure::Classification)?;
    if kind != ItemKind::Header {
        return Err(ReadFailure::Item(kind));
    }
    let header = Header::decode(opener).map_err(ReadFailure::Wire)?;
    let registry = header.registry_version();
    if registry.get() != super::REGISTRY_VERSION {
        return Err(ReadFailure::Registry(registry.get()));
    }
    let lineage = header.lineage();
    if let Some(lineage) = lineage
        && previous.is_none_or(|previous| {
            lineage.previous_head != previous.verifier.head()
                || previous
                    .ordinal
                    .is_some_and(|ordinal| ordinal.checked_add(1) != Some(lineage.ordinal.get()))
        })
    {
        return Err(ReadFailure::Lineage);
    }
    let file_ordinal = previous.map_or(0, |previous| previous.file_ordinal + 1);
    let identifier = header.identifier().ok_or(ReadFailure::MissingContent)?;
    Ok(Segment {
        ordinal: lineage.map(|lineage| lineage.ordinal.get()),
        verifier: SegmentVerifier::new(identifier, registry),
        dictionary: SegmentDictionary::new(file_ordinal),
        file_ordinal,
        units: FastMap::default(),
    })
}
