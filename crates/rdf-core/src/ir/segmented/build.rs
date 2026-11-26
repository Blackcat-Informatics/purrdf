// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded resident fixture construction and complete untrusted-image validation.

use super::super::statistics::{LocalStatistics, StatisticsMergeError, StatisticsSection};
use core::ops::Range;
use purrdf_lex::allocation::{CapacitySpace, Memory, Resident, StorageError};
use std::sync::Arc;

use crate::{GlobalTermId, QuadIds, RdfDataset, RdfDatasetBuilder, TermId, TermRef, TermValue};

use super::format::{self, Header, Reader, Stream};
use super::{
    SegmentedBytes, SegmentedError, SegmentedReadLimits, SegmentedReceipt, SegmentedSession,
};

// The v1 cost law is a wire-profile constant, independent of the builder's
// pointer width: 80 bytes for an owned node plus 64 bytes of allocation slack.
const OWNED_TERM_NODE_BYTES: u64 = 144;
const _: () = assert!(size_of::<TermValue>() <= 80);

/// Inclusive bounds for the resident constructor. This is not external ingestion.
#[derive(Debug, Clone, Copy)]
pub struct SegmentedBuildLimits {
    terms: u32,
    term_bytes: u32,
    rows: u32,
    block_bytes: u32,
    records_per_block: u32,
    first_term_index: u64,
}
impl SegmentedBuildLimits {
    /// Choose resident construction limits and the persisted block layout explicitly.
    /// `terms` and `term_bytes` bound distinct dictionary entries and their UTF-8
    /// payload, including datatype and reification IRIs. Spare allocation capacity
    /// and temporary workspace are separate. `rows` counts accepted row and named
    /// graph declaration operations, including duplicates.
    ///
    /// # Errors
    /// Refuses invalid block sizes or zero/oversized dictionary block counts.
    pub fn new(
        terms: u32,
        term_bytes: u32,
        rows: u32,
        block_bytes: u32,
        records_per_block: u32,
    ) -> Result<Self, SegmentedError> {
        if terms == u32::MAX
            || !(512..=1_048_576).contains(&block_bytes)
            || records_per_block == 0
            || records_per_block > format::MAX_RECORDS_PER_BLOCK
        {
            return Err(SegmentedError::ConstructionLimit);
        }
        Ok(Self {
            terms,
            term_bytes,
            rows,
            block_bytes,
            records_per_block,
            first_term_index: 0,
        })
    }
    /// Select a logical dictionary interval, including intervals above `2^32`.
    /// Numeric IDs remain stable through sealing and reopening.
    ///
    /// # Errors
    /// Refuses an interval reaching the reserved terminal ID representation.
    pub fn with_first_term_index(mut self, first: u64) -> Result<Self, SegmentedError> {
        first
            .checked_add(u64::from(self.terms))
            .ok_or(SegmentedError::AddressExhausted)?;
        self.first_term_index = first;
        Ok(self)
    }
}

/// Resident construction with explicit limits and storage grown as input arrives.
/// The ordinary validated IR builder owns RDF normalization and positional checks.
/// This constructor's bounded IDs are translated arithmetically, without a global
/// per-page translation vector.
#[derive(Debug)]
pub struct SegmentedBuilder {
    builder: RdfDatasetBuilder,
    limits: SegmentedBuildLimits,
    admitted_rows: u64,
}
impl SegmentedBuilder {
    /// Start an empty resident builder without allocating its declared ceilings.
    pub fn new(limits: SegmentedBuildLimits) -> Result<Self, SegmentedError> {
        Ok(Self {
            builder: RdfDatasetBuilder::with_dictionary_limits(limits.terms, limits.term_bytes),
            limits,
            admitted_rows: 0,
        })
    }

    /// Number of distinct retained statements across the ordinary, reifier and
    /// annotation streams. Repeated pushes count once per distinct stored row;
    /// named-graph declarations add no statements. This reads the shared IR
    /// builder's existing counts without allocating or scanning the rows.
    /// Structural RDF validation still occurs at [`Self::seal`].
    #[must_use]
    pub fn statement_count(&self) -> usize {
        self.builder.statement_count()
    }

    fn global(&self, id: TermId) -> GlobalTermId {
        GlobalTermId::from_index(
            self.limits.first_term_index
                + u64::try_from(id.index()).expect("resident term index fits u64"),
        )
        .expect("stored term index is representable")
    }
    fn local(&self, id: GlobalTermId) -> Result<TermId, SegmentedError> {
        let index = id
            .index()
            .checked_sub(self.limits.first_term_index)
            .ok_or(SegmentedError::Corrupt("foreign constructor term ID"))?;
        let stored = u64::try_from(self.builder.term_count())
            .map_err(|_| SegmentedError::AddressExhausted)?;
        if index >= stored {
            return Err(SegmentedError::Corrupt("foreign constructor term ID"));
        }
        Ok(
            TermId::from_index(u32::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?)
                .expect("stored term index is representable"),
        )
    }

    /// Intern a batch through one normalization authority, without allocating a batch
    /// output. Every returned ID remains valid in this generation after sealing.
    ///
    /// # Errors
    /// Refuses a value atomically when its new terms or string bytes exceed the
    /// declared dictionary limits. Earlier batch values remain admitted.
    pub fn intern_batch(
        &mut self,
        values: &[TermValue],
        mut visit: impl FnMut(&TermValue, GlobalTermId),
    ) -> Result<(), SegmentedError> {
        for value in values {
            validate_constructor_value(value)?;
            let id = self
                .builder
                .intern_value_atomically(value)
                .map_err(construction_storage_error)?;
            visit(value, self.global(id));
        }
        Ok(())
    }

    fn next_row_count(&self) -> Result<u64, SegmentedError> {
        let next = self
            .admitted_rows
            .checked_add(1)
            .ok_or(SegmentedError::AddressExhausted)?;
        if next > u64::from(self.limits.rows) {
            return Err(SegmentedError::ConstructionLimit);
        }
        Ok(next)
    }
    /// Add a primary quad. Duplicate rows reuse the shared IR builder's stored row.
    ///
    /// # Errors
    /// Refuses foreign IDs or the inclusive resident row ceiling.
    pub fn push_quad(&mut self, quad: QuadIds<GlobalTermId>) -> Result<(), SegmentedError> {
        let (s, p, o) = (
            self.local(quad.s)?,
            self.local(quad.p)?,
            self.local(quad.o)?,
        );
        let g = quad.g.map(|id| self.local(id)).transpose()?;
        let rows = self.next_row_count()?;
        self.builder.push_quad(s, p, o, g)?;
        self.admitted_rows = rows;
        Ok(())
    }
    /// Add a scoped RDF 1.2 reifier binding.
    ///
    /// # Errors
    /// Refuses foreign IDs or the resident row ceiling. Seal checks RDF roles.
    pub fn push_reifier(
        &mut self,
        reifier: GlobalTermId,
        triple: GlobalTermId,
        graph: Option<GlobalTermId>,
    ) -> Result<(), SegmentedError> {
        let (r, t) = (self.local(reifier)?, self.local(triple)?);
        let g = graph.map(|id| self.local(id)).transpose()?;
        let rows = self.next_row_count()?;
        self.builder
            .push_reifier_in_graph(r, t, g)
            .map_err(construction_storage_error)?;
        self.admitted_rows = rows;
        Ok(())
    }
    /// Add a scoped RDF 1.2 statement annotation.
    ///
    /// # Errors
    /// Refuses foreign IDs or the resident row ceiling. Seal checks RDF roles.
    pub fn push_annotation(&mut self, quad: QuadIds<GlobalTermId>) -> Result<(), SegmentedError> {
        let (s, p, o) = (
            self.local(quad.s)?,
            self.local(quad.p)?,
            self.local(quad.o)?,
        );
        let g = quad.g.map(|id| self.local(id)).transpose()?;
        let rows = self.next_row_count()?;
        self.builder.push_annotation_in_graph(s, p, o, g)?;
        self.admitted_rows = rows;
        Ok(())
    }
    /// Preserve an explicit named graph, including a graph with no rows.
    ///
    /// # Errors
    /// Refuses foreign IDs and the same resident row/declaration ceiling.
    pub fn declare_named_graph(&mut self, graph: GlobalTermId) -> Result<(), SegmentedError> {
        let graph = self.local(graph)?;
        let rows = self.next_row_count()?;
        self.builder.declare_named_graph(graph)?;
        self.admitted_rows = rows;
        Ok(())
    }
    /// Validate RDF, encode the versioned blocks and certify their complete index law.
    ///
    /// # Errors
    /// Returns structural RDF validation, format-capacity or complete certification
    /// errors. No usable receipt is minted for a partial or invalid image.
    pub fn seal(self) -> Result<SegmentedImage, SegmentedError> {
        let dataset = self.builder.freeze().map_err(|error| {
            SegmentedError::from_rdf(error, "resident RDF freeze rejected the fixture")
        })?;
        let bytes = encode_dataset(dataset.as_ref(), self.limits)?;
        drop(dataset);
        SegmentedImage::certify(bytes)
    }
}

fn construction_storage_error(cause: StorageError) -> SegmentedError {
    match cause {
        StorageError::Capacity(error)
            if matches!(
                error.space,
                CapacitySpace::LocalTerms | CapacitySpace::LocalTermBytes
            ) =>
        {
            SegmentedError::ConstructionLimit
        }
        cause => SegmentedError::Storage(cause),
    }
}

fn validate_constructor_value(value: &TermValue) -> Result<(), SegmentedError> {
    use core::ops::ControlFlow;
    use purrdf_lex::allocation::{Memory, Resident};
    match value.visit_terms_with_memory(
        |term, _| {
            let check = match term {
                TermValue::Iri(iri) => Some((iri.as_str(), "constructor IRI is not absolute")),
                TermValue::Literal { datatype, .. } => {
                    Some((datatype.as_str(), "constructor datatype is not absolute"))
                }
                TermValue::Blank { .. } | TermValue::Triple { .. } => None,
            };
            Ok(match check {
                Some((iri, reason)) if super::super::absolute::check_absolute(iri).is_err() => {
                    ControlFlow::Break(SegmentedError::Corrupt(reason))
                }
                _ => ControlFlow::Continue(()),
            })
        },
        &mut Memory::new(&mut Resident),
    )? {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(cause) => Err(cause),
    }
}

/// Complete persisted image plus the authority produced by full certification.
#[derive(Debug, Clone)]
pub struct SegmentedImage {
    bytes: super::Shared<Vec<u8>>,
    receipt: SegmentedReceipt,
}
impl SegmentedImage {
    #[cfg(test)]
    pub(in crate::ir) fn from_native_fixture(
        dataset: &RdfDataset,
        limits: SegmentedBuildLimits,
    ) -> Result<Self, SegmentedError> {
        Self::certify(encode_dataset(dataset, limits)?)
    }
    /// Encoded bytes, suitable for a host-owned file/object/browser-storage writer.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Complete certification authority retained across selective reopening.
    #[must_use]
    pub fn receipt(&self) -> &SegmentedReceipt {
        &self.receipt
    }
    /// A fixture/browser provider sharing the immutable bytes without copying them.
    #[must_use]
    pub fn provider(&self) -> SegmentedBytes {
        SegmentedBytes::from_certified(self.bytes.clone(), self.receipt.snapshot)
    }

    /// Open a selective session over this image's immutable provider and receipt.
    /// The caller supplies every read and residency limit.
    ///
    /// # Errors
    /// Propagates the original session's authentication, capacity and request refusal.
    pub fn open_session(
        &self,
        limits: SegmentedReadLimits,
    ) -> Result<SegmentedSession, SegmentedError> {
        SegmentedSession::open(Arc::new(self.provider()), self.receipt(), limits)
    }

    /// Fully validate an untrusted image, including every dictionary/index block,
    /// forward/reverse agreement, row ordering and roles, summaries and empty graphs.
    /// This resident certification operation is separate from bounded selective reads.
    /// On success, the image retains the supplied vector's allocation, including
    /// any spare capacity, without copying its bytes.
    ///
    /// # Errors
    /// No receipt is returned if any unvisited-range/index claim could hide a fact.
    pub fn certify(bytes: Vec<u8>) -> Result<Self, SegmentedError> {
        let header = Header::decode(&bytes)?;
        if header.terms >= u64::from(u32::MAX)
            || header
                .streams
                .iter()
                .any(|stream| stream.rows >= u64::from(u32::MAX))
        {
            return Err(SegmentedError::ConstructionLimit);
        }
        let block_bytes =
            usize::try_from(header.block_bytes).map_err(|_| SegmentedError::AddressExhausted)?;
        let count = [
            header.dictionary,
            header.reverse,
            header.streams[0],
            header.streams[1],
            header.streams[2],
            header.graphs,
            header.summaries,
            header.value_ranges,
            header.statistics[0],
            header.statistics[1],
            header.statistics[2],
            header.statistics[3],
            header.effective_directory,
            header.source_checkpoints,
            header.overflow,
        ]
        .iter()
        .try_fold(1_u64, |n, stream| {
            n.checked_add(stream.blocks)
                .ok_or(SegmentedError::AddressExhausted)
        })?;
        header.validate_addresses(count)?;
        let leaves = count
            .checked_next_power_of_two()
            .ok_or(SegmentedError::AddressExhausted)?;
        let data_bytes = count
            .checked_mul(u64::from(header.block_bytes))
            .ok_or(SegmentedError::AddressExhausted)?;
        let tree_bytes = leaves
            .checked_mul(2)
            .and_then(|n| n.checked_mul(32))
            .ok_or(SegmentedError::AddressExhausted)?;
        let total = data_bytes
            .checked_add(tree_bytes)
            .ok_or(SegmentedError::AddressExhausted)?;
        if total != u64::try_from(bytes.len()).map_err(|_| SegmentedError::AddressExhausted)? {
            return Err(SegmentedError::Corrupt("encoded image length mismatch"));
        }
        let tree = make_tree(
            &bytes[..usize::try_from(data_bytes).map_err(|_| SegmentedError::AddressExhausted)?],
            block_bytes,
            count,
            leaves,
        )?;
        let tree_start =
            usize::try_from(data_bytes).map_err(|_| SegmentedError::AddressExhausted)?;
        if bytes[tree_start..] != tree {
            return Err(SegmentedError::Corrupt("authenticated-index tree mismatch"));
        }
        let root: [u8; 32] = tree[32..64]
            .try_into()
            .map_err(|_| SegmentedError::Corrupt("missing authenticated root"))?;
        drop(tree);
        validate_content(&bytes, &header)?;
        let snapshot = format::snapshot_digest(&root, header.block_bytes, count);
        Ok(Self {
            bytes: super::Shared::try_new(bytes).map_err(|_| StorageError::AllocationFailed)?,
            receipt: SegmentedReceipt {
                snapshot,
                root,
                block_bytes: header.block_bytes,
                block_count: count,
                leaf_count: leaves,
                byte_len: total,
            },
        })
    }

    /// Explicitly migrate the eager v1 pack through its retained validated reader.
    /// This materializing convenience is bounded by the supplied construction limits.
    /// v1 does not carry declaration-only graphs; use [`Self::from_pack_v1_with_graphs`]
    /// to restate declarations from an independently trusted sidecar.
    ///
    /// # Errors
    /// Returns old-reader validation failures, source read failures or capacity refusal.
    pub fn from_pack_v1(
        bytes: &[u8],
        limits: SegmentedBuildLimits,
    ) -> Result<Self, SegmentedError> {
        let source = super::super::pack::restore_pack(bytes)
            .map_err(|_| SegmentedError::Corrupt("invalid eager v1 pack"))?;
        if source.as_ref().term_count()
            > usize::try_from(limits.terms).map_err(|_| SegmentedError::AddressExhausted)?
            || source.as_ref().rdf_row_count()
                > usize::try_from(limits.rows).map_err(|_| SegmentedError::AddressExhausted)?
            || source.as_ref().rdf_text_bytes()
                > usize::try_from(limits.term_bytes)
                    .map_err(|_| SegmentedError::AddressExhausted)?
        {
            return Err(SegmentedError::ConstructionLimit);
        }
        let bytes = encode_dataset(source.as_ref(), limits)?;
        drop(source);
        Self::certify(bytes)
    }
    /// Migrate v1 RDF and explicitly restate named-graph declarations. This never
    /// guesses a graph role from an otherwise unused dictionary IRI.
    ///
    /// # Errors
    /// Refuses invalid v1 RDF, non-graph values, or construction capacities before
    /// admitting sidecar terms/declarations. The new identity binds the sidecar too.
    pub fn from_pack_v1_with_graphs(
        bytes: &[u8],
        graphs: &[TermValue],
        limits: SegmentedBuildLimits,
    ) -> Result<Self, SegmentedError> {
        let source = super::super::pack::restore_pack(bytes)
            .map_err(|_| SegmentedError::Corrupt("invalid eager v1 pack"))?;
        let terms = u64::try_from(source.as_ref().term_count())
            .map_err(|_| SegmentedError::AddressExhausted)?;
        let term_bytes = u64::try_from(source.as_ref().rdf_text_bytes())
            .map_err(|_| SegmentedError::AddressExhausted)?;
        let rows =
            u64::try_from(source.as_ref().rdf_row_count() + source.as_ref().named_graphs().count())
                .map_err(|_| SegmentedError::AddressExhausted)?;
        if terms > u64::from(limits.terms)
            || term_bytes > u64::from(limits.term_bytes)
            || rows > u64::from(limits.rows)
        {
            return Err(SegmentedError::ConstructionLimit);
        }
        let mut builder = SegmentedBuilder::new(limits)?;
        super::super::import::DatasetImporter::new(&mut builder.builder, source.as_ref())
            .map_err(StorageError::from)
            .map_err(construction_storage_error)?
            .append()
            .map_err(StorageError::from)
            .map_err(construction_storage_error)?;
        drop(source);
        builder.admitted_rows = rows;
        for graph in graphs {
            if !matches!(graph, TermValue::Iri(_) | TermValue::Blank { .. }) {
                return Err(SegmentedError::Corrupt(
                    "sidecar graph declaration is not an IRI or blank",
                ));
            }
            let mut id = None;
            builder.intern_batch(core::slice::from_ref(graph), |_, interned| {
                id = Some(interned);
            })?;
            builder
                .declare_named_graph(id.expect("one admitted batch term invokes its visitor"))?;
        }
        builder.seal()
    }
}

fn map_id(id: TermId, first: u64) -> GlobalTermId {
    GlobalTermId::from_index(first + u64::try_from(id.index()).expect("local term index fits u64"))
        .expect("stored term index is representable")
}
fn map_quad(q: QuadIds, first: u64) -> QuadIds<GlobalTermId> {
    QuadIds {
        s: map_id(q.s, first),
        p: map_id(q.p, first),
        o: map_id(q.o, first),
        g: q.g.map(|id| map_id(id, first)),
    }
}

fn encode_dataset(
    dataset: &RdfDataset,
    limits: SegmentedBuildLimits,
) -> Result<Vec<u8>, SegmentedError> {
    let mut resident = Resident;
    dataset.with_prepared_statistics(&mut Memory::new(&mut resident), |statistics, _| {
        encode_prepared_dataset(dataset, statistics, limits)
    })
}

/// Sizing claims logical pages. Emission admits sections before writing.
struct ImagePages<'a> {
    data: Option<&'a mut [u8]>,
    block_bytes: usize,
    next: usize,
    sections: Option<[Stream; 15]>,
    claimed: usize,
    auxiliary: Option<(Stream, Stream)>,
}
impl ImagePages<'_> {
    fn block(&mut self, slot: usize) -> Result<&mut [u8], SegmentedError> {
        let offset = slot
            .checked_mul(self.block_bytes)
            .ok_or(SegmentedError::AddressExhausted)?;
        let data = self
            .data
            .as_deref_mut()
            .ok_or(SegmentedError::Corrupt("missing output image"))?;
        data.get_mut(offset..)
            .and_then(|tail| tail.get_mut(..self.block_bytes))
            .ok_or(SegmentedError::Corrupt("image emission exceeds layout"))
    }

    fn claim(&mut self, rows: usize, count: usize) -> Result<Stream, SegmentedError> {
        let stream = Stream {
            first: u64::try_from(self.next).map_err(|_| SegmentedError::AddressExhausted)?,
            blocks: u64::try_from(count).map_err(|_| SegmentedError::AddressExhausted)?,
            rows: u64::try_from(rows).map_err(|_| SegmentedError::AddressExhausted)?,
        };
        if let Some(sections) = &self.sections {
            let expected = sections
                .get(self.claimed)
                .ok_or(SegmentedError::Corrupt("extra image section"))?;
            if (stream.first, stream.blocks, stream.rows)
                != (expected.first, expected.blocks, expected.rows)
            {
                return Err(SegmentedError::Corrupt(
                    "image section differs from its layout",
                ));
            }
        }
        self.claimed += 1;
        self.next = self
            .next
            .checked_add(count)
            .ok_or(SegmentedError::AddressExhausted)?;
        Ok(stream)
    }

    fn section_page(&mut self, stream: Stream, page: u64) -> Result<&mut [u8], SegmentedError> {
        if page >= stream.blocks {
            return Err(SegmentedError::Corrupt("page exceeds image section"));
        }
        let slot = stream
            .first
            .checked_add(page)
            .and_then(|slot| usize::try_from(slot).ok())
            .ok_or(SegmentedError::AddressExhausted)?;
        self.block(slot)
    }

    fn put_record(
        &mut self,
        stream: Stream,
        ordinal: u64,
        kind: u8,
        record: &[u8],
    ) -> Result<(), SegmentedError> {
        let width = record.len();
        let per_block = u64::try_from((self.block_bytes - 9) / width)
            .map_err(|_| SegmentedError::AddressExhausted)?;
        if ordinal >= stream.rows {
            return Err(SegmentedError::Corrupt("auxiliary stream exceeds plan"));
        }
        let page = ordinal / per_block;
        let block = self.section_page(stream, page)?;
        let in_page =
            usize::try_from(ordinal % per_block).map_err(|_| SegmentedError::AddressExhausted)?;
        if in_page == 0 {
            let count = usize::try_from((stream.rows - ordinal).min(per_block))
                .map_err(|_| SegmentedError::AddressExhausted)?;
            fixed_header(block, kind, count, width)?;
        }
        let at = 9 + in_page * width;
        block[at..at + width].copy_from_slice(record);
        Ok(())
    }

    fn overflow(
        &mut self,
        span: format::OverflowSpan,
        parts: [&[u8]; 5],
    ) -> Result<(), SegmentedError> {
        let stream = self
            .sections
            .as_ref()
            .and_then(|sections| sections.last())
            .copied()
            .ok_or(SegmentedError::Corrupt("missing planned overflow section"))?;
        let end = span
            .first
            .checked_add(span.blocks(self.block_bytes)?)
            .ok_or(SegmentedError::AddressExhausted)?;
        if end > stream.blocks {
            return Err(SegmentedError::Corrupt(
                "overflow span exceeds image section",
            ));
        }
        let mut remaining =
            usize::try_from(span.bytes).map_err(|_| SegmentedError::AddressExhausted)?;
        let mut parts = parts.into_iter().filter(|part| !part.is_empty());
        let mut part = parts.next().unwrap_or_default();
        for ordinal in span.first..end {
            let width = remaining.min(self.block_bytes - format::OVERFLOW_HEADER_BYTES);
            let block = self.section_page(stream, ordinal)?;
            block[0] = format::OVERFLOW;
            crate::bytes::put_u32_le(
                block,
                1,
                u32::try_from(width).map_err(|_| SegmentedError::AddressExhausted)?,
            )
            .expect("overflow page size field");
            let start = format::OVERFLOW_HEADER_BYTES;
            let mut page = &mut block[start..start + width];
            while !page.is_empty() {
                if part.is_empty() {
                    part = parts
                        .next()
                        .ok_or(SegmentedError::Corrupt("short overflow encoding"))?;
                }
                let copied = page.len().min(part.len());
                page[..copied].copy_from_slice(&part[..copied]);
                page = &mut page[copied..];
                part = &part[copied..];
            }
            remaining -= width;
        }
        if remaining != 0 || !part.is_empty() || parts.next().is_some() {
            return Err(SegmentedError::Corrupt(
                "overflow encoding differs from its layout",
            ));
        }
        Ok(())
    }
}

fn encode_prepared_dataset(
    dataset: &RdfDataset,
    source_statistics: &LocalStatistics,
    limits: SegmentedBuildLimits,
) -> Result<Vec<u8>, SegmentedError> {
    let block_bytes =
        usize::try_from(limits.block_bytes).map_err(|_| SegmentedError::AddressExhausted)?;
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut sizing = ImagePages {
        data: None,
        block_bytes,
        next: 1,
        sections: None,
        claimed: 0,
        auxiliary: None,
    };
    let mut term_data = prepare_dictionary(dataset, limits)?;
    let statistics =
        translated_statistics(source_statistics, limits.first_term_index, &mut memory)?;
    let planned = emit_image(
        dataset,
        source_statistics,
        limits,
        &mut sizing,
        &mut term_data,
        &statistics,
    )?;
    let count = planned
        .overflow
        .first
        .checked_add(planned.overflow.blocks)
        .ok_or(SegmentedError::AddressExhausted)?;
    let leaves = count
        .checked_next_power_of_two()
        .ok_or(SegmentedError::AddressExhausted)?;
    let data_size = usize::try_from(count)
        .ok()
        .and_then(|count| count.checked_mul(block_bytes))
        .ok_or(SegmentedError::AddressExhausted)?;
    let tree_size = leaves
        .checked_mul(64)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or(SegmentedError::AddressExhausted)?;
    let capacity = data_size
        .checked_add(tree_size)
        .ok_or(SegmentedError::AddressExhausted)?;
    let mut bytes = Vec::new();
    memory.reserve(&mut bytes, capacity)?;
    bytes.resize(capacity, 0);
    let (data, tree) = bytes.split_at_mut(data_size);
    let sections = [
        planned.dictionary,
        planned.reverse,
        planned.streams[0],
        planned.streams[1],
        planned.streams[2],
        planned.graphs,
        planned.summaries,
        planned.value_ranges,
        planned.statistics[0],
        planned.statistics[1],
        planned.statistics[2],
        planned.statistics[3],
        planned.effective_directory,
        planned.source_checkpoints,
        planned.overflow,
    ];
    let mut output = ImagePages {
        data: Some(data),
        block_bytes,
        next: 1,
        sections: Some(sections),
        claimed: 0,
        auxiliary: Some((planned.summaries, planned.value_ranges)),
    };
    let actual = emit_image(
        dataset,
        source_statistics,
        limits,
        &mut output,
        &mut term_data,
        &statistics,
    )?;
    if output.claimed != sections.len()
        || output.next != usize::try_from(count).map_err(|_| SegmentedError::AddressExhausted)?
    {
        return Err(SegmentedError::Corrupt(
            "image emission did not consume its layout",
        ));
    }
    drop(term_data);
    drop(statistics);
    let header = actual.encode();
    if header != planned.encode() {
        return Err(SegmentedError::Corrupt(
            "image emission disagrees with its layout",
        ));
    }
    data[..header.len()].copy_from_slice(&header);
    fill_tree(data, block_bytes, count, leaves, tree)?;
    Ok(bytes)
}

enum DictionaryPage {
    Inline { used: usize, compressed: bool },
    Overflow(format::OverflowSpan),
}

struct PreparedDictionary {
    pages: Vec<DictionaryPage>,
    records: Vec<(TermId, u32)>,
    max_footprint: u64,
}

fn term_encoding(
    dataset: &RdfDataset,
    local: TermId,
    first: u64,
) -> Result<format::TermEncoding<'_>, SegmentedError> {
    format::TermEncoding::new(dataset.resolve(local).map_ids(|id| map_id(id, first)))
}

fn prepare_dictionary(
    dataset: &RdfDataset,
    limits: SegmentedBuildLimits,
) -> Result<PreparedDictionary, SegmentedError> {
    let count =
        u32::try_from(dataset.term_count()).map_err(|_| SegmentedError::AddressExhausted)?;
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut records = Vec::new();
    let mut footprints = Vec::new();
    memory.reserve(&mut records, dataset.term_count())?;
    memory.reserve(&mut footprints, dataset.term_count())?;
    let mut max_footprint = 0;
    for raw in 0..count {
        let local = TermId::from_index(raw).map_err(StorageError::from)?;
        let term = dataset
            .resolve(local)
            .map_ids(|id| map_id(id, limits.first_term_index));
        let footprint = expanded_footprint(
            term.map_ids(GlobalTermId::index),
            limits.first_term_index,
            &footprints,
        )?;
        max_footprint = max_footprint.max(footprint);
        footprints.push(footprint);
        let width = u32::try_from(format::encoded_term_size(&term)?)
            .map_err(|_| SegmentedError::AddressExhausted)?;
        records.push((local, width));
    }
    Ok(PreparedDictionary {
        pages: Vec::new(),
        records,
        max_footprint,
    })
}

fn emit_image(
    dataset: &RdfDataset,
    source_statistics: &LocalStatistics,
    limits: SegmentedBuildLimits,
    blocks: &mut ImagePages<'_>,
    term_data: &mut PreparedDictionary,
    prepared_statistics: &LocalStatistics<GlobalTermId>,
) -> Result<Header, SegmentedError> {
    let block_bytes = blocks.block_bytes;
    let records_per_block =
        usize::try_from(limits.records_per_block).map_err(|_| SegmentedError::AddressExhausted)?;
    let mut overflow_blocks = 0;
    let (dictionary, overflow_rows) = append_dictionary(
        blocks,
        term_data,
        dataset,
        limits.first_term_index,
        records_per_block,
        block_bytes,
        &mut overflow_blocks,
    )?;
    if blocks.data.is_some() {
        purrdf_lex::walk::try_sort_unstable_by(&mut term_data.records, |left, right| {
            let left = term_encoding(dataset, left.0, limits.first_term_index)?;
            let right = term_encoding(dataset, right.0, limits.first_term_index)?;
            let order =
                format::compare_key_parts(left.parts().into_iter(), right.parts().into_iter()).1;
            Ok::<_, SegmentedError>(order)
        })?;
    }
    let reverse = append_fixed(
        blocks,
        term_data.records.len(),
        format::REVERSE,
        8,
        block_bytes,
        |range, _, payload| {
            for ((id, _), bytes) in term_data.records[range]
                .iter()
                .zip(payload.as_chunks_mut::<8>().0)
            {
                *bytes = format::id_bytes(map_id(*id, limits.first_term_index));
            }
            Ok(())
        },
    )?;
    if blocks.data.is_some() {
        term_data.pages = Vec::new();
        term_data.records = Vec::new();
    }
    let mut summaries = 0;
    let mut value_ranges = 0;
    let mut streams = [Stream::default(); 3];
    let mut rows = Vec::new();
    let mut resident = Resident;
    let mut scratch = Memory::new(&mut resident);
    for (table, stream) in streams.iter_mut().enumerate() {
        match table {
            0 => prepare_ordered_quads(&mut rows, dataset.quads(), limits, &mut scratch)?,
            1 => prepare_ordered_quads(&mut rows, dataset.reifier_quads(), limits, &mut scratch)?,
            _ => {
                prepare_ordered_quads(&mut rows, dataset.annotation_quads(), limits, &mut scratch)?;
            }
        }
        *stream = append_quads(
            blocks,
            &rows,
            table as u8,
            block_bytes,
            &mut summaries,
            &mut value_ranges,
            |id| {
                let index = id
                    .index()
                    .checked_sub(limits.first_term_index)
                    .ok_or(SegmentedError::AddressExhausted)?;
                let local = TermId::from_index(
                    u32::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?,
                )
                .map_err(StorageError::from)?;
                Ok(dataset
                    .resolve(local)
                    .map_ids(|id| map_id(id, limits.first_term_index)))
            },
        )?;
    }
    drop(rows);
    // The frozen dictionary already owns the sorted, distinct graph IDs.
    let mut graph_ids = dataset.named_graphs();
    let graphs = append_fixed(
        blocks,
        dataset.named_graphs().count(),
        format::GRAPHS,
        8,
        block_bytes,
        |_, _, payload| {
            for bytes in payload.as_chunks_mut::<8>().0 {
                let local = graph_ids.next().ok_or(SegmentedError::Corrupt(
                    "named graph count changed during emission",
                ))?;
                *bytes = format::id_bytes(map_id(local, limits.first_term_index));
            }
            Ok(())
        },
    )?;
    let summaries = fixed_layout(blocks, summaries, format::SUMMARY_BYTES)?;
    let value_ranges = fixed_layout(blocks, value_ranges, format::VALUE_RANGE_BYTES)?;
    let statistics = append_statistics(prepared_statistics, blocks, block_bytes)?;
    let directory = source_statistics.effective_source_directory();
    let effective_directory = append_fixed(
        blocks,
        directory.len(),
        format::SOURCE_DIRECTORY,
        super::super::source_directory::RECORD_BYTES,
        block_bytes,
        |range, _, payload| {
            directory.encode_records(range.start, payload);
            Ok(())
        },
    )?;
    let checkpoints = directory.node_checkpoints()?;
    let source_checkpoints = append_fixed(
        blocks,
        checkpoints.len(),
        format::SOURCE_CHECKPOINTS,
        super::super::source_directory::NODE_CHECKPOINT_BYTES,
        block_bytes,
        |range, _, payload| {
            super::super::source_directory::encode_node_checkpoints(&checkpoints[range], payload);
            Ok(())
        },
    )?;
    let overflow = blocks.claim(
        usize::try_from(overflow_rows).map_err(|_| SegmentedError::AddressExhausted)?,
        usize::try_from(overflow_blocks).map_err(|_| SegmentedError::AddressExhausted)?,
    )?;
    let caps = dataset.capabilities();
    let flags = u8::from(caps.quoted_triples)
        | (u8::from(caps.reifiers) << 1)
        | (u8::from(caps.annotations) << 2);
    Ok(Header {
        version: format::VERSION,
        block_bytes: limits.block_bytes,
        records_per_block: limits.records_per_block,
        first_term_index: limits.first_term_index,
        max_owned_term_bytes: term_data.max_footprint,
        terms: u64::try_from(dataset.term_count()).map_err(|_| SegmentedError::AddressExhausted)?,
        dictionary,
        reverse,
        streams,
        graphs,
        summaries,
        value_ranges,
        statistics,
        effective_directory,
        source_checkpoints,
        overflow,
        flags,
        source_union: source_statistics.source_union(),
    })
}

fn prepare_ordered_quads(
    rows: &mut Vec<QuadIds<GlobalTermId>>,
    source: impl Iterator<Item = QuadIds>,
    limits: SegmentedBuildLimits,
    memory: &mut Memory<'_, Resident>,
) -> Result<(), SegmentedError> {
    rows.clear();
    memory.reserve(rows, source.size_hint().0)?;
    for quad in source {
        memory.push(rows, map_quad(quad, limits.first_term_index))?;
    }
    rows.sort_unstable_by_key(|quad| (quad.s, quad.p, quad.o, quad.g));
    rows.dedup();
    Ok(())
}

fn translated_statistics(
    source_statistics: &LocalStatistics,
    first: u64,
    memory: &mut Memory<'_, Resident>,
) -> Result<LocalStatistics<GlobalTermId>, SegmentedError> {
    LocalStatistics::merge_translated(
        [source_statistics],
        |statistics| statistics,
        |_, id| map_id(id, first),
        memory,
    )
    .map_err(|error| match error {
        StatisticsMergeError::Storage(error) => SegmentedError::Storage(error),
        StatisticsMergeError::Encoding(error) => SegmentedError::Statistics(error),
    })
}

fn append_statistics(
    statistics: &LocalStatistics<GlobalTermId>,
    blocks: &mut ImagePages<'_>,
    block_bytes: usize,
) -> Result<[Stream; 4], SegmentedError> {
    let mut streams = [Stream::default(); 4];
    for section in StatisticsSection::ALL {
        streams[section.index()] = append_fixed(
            blocks,
            statistics.section_rows(section),
            format::statistics_kind(section),
            section.width(),
            block_bytes,
            |range, _, payload| {
                statistics.encode_records(section, range.start, payload)?;
                Ok(())
            },
        )?;
    }
    Ok(streams)
}

fn append_dictionary(
    blocks: &mut ImagePages<'_>,
    dictionary: &mut PreparedDictionary,
    dataset: &RdfDataset,
    first: u64,
    chunk: usize,
    block_bytes: usize,
    overflow: &mut u64,
) -> Result<(Stream, u64), SegmentedError> {
    let mut spanned = 0_u64;
    let inline_limit = block_bytes - (format::RECORD_HEADER + 16);
    let inline_record = |&(id, _): &(TermId, u32)| {
        let key = term_encoding(dataset, id, first)?;
        Ok((map_id(id, first), format::WireKey::Term(key)))
    };
    if blocks.sections.is_none() {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut remaining = dictionary.records.as_slice();
        while !remaining.is_empty() {
            let offered = &remaining[..remaining.len().min(chunk)];
            let inline = offered
                .iter()
                .position(|(_, width)| *width as usize > inline_limit)
                .unwrap_or(offered.len());
            let (page, used) = if inline == 0 {
                let span = format::OverflowSpan {
                    first: *overflow,
                    bytes: offered[0].1,
                };
                *overflow = overflow
                    .checked_add(span.blocks(block_bytes)?)
                    .ok_or(SegmentedError::AddressExhausted)?;
                spanned += 1;
                (DictionaryPage::Overflow(span), 1)
            } else {
                let records = offered[..inline].iter().map(inline_record);
                let layout = format::plan_record_block(&records, block_bytes)?;
                (
                    DictionaryPage::Inline {
                        used: layout.used,
                        compressed: layout.compressed,
                    },
                    layout.used,
                )
            };
            memory.push(&mut dictionary.pages, page)?;
            remaining = &remaining[used..];
        }
    }
    let stream = blocks.claim(dictionary.records.len(), dictionary.pages.len())?;
    if blocks.data.is_some() {
        let mut remaining = dictionary.records.as_slice();
        for (page, plan) in dictionary.pages.iter().enumerate() {
            let page = u64::try_from(page).map_err(|_| SegmentedError::AddressExhausted)?;
            let used = match *plan {
                DictionaryPage::Overflow(span) => {
                    let &(id, width) = remaining
                        .first()
                        .ok_or(SegmentedError::Corrupt("missing planned overflow record"))?;
                    if *overflow != span.first
                        || span.bytes != width
                        || width as usize <= inline_limit
                    {
                        return Err(SegmentedError::Corrupt(
                            "overflow span differs from its layout",
                        ));
                    }
                    let key = term_encoding(dataset, id, first)?;
                    blocks.overflow(span, key.parts())?;
                    format::write_spanned_record(
                        map_id(id, first),
                        span,
                        blocks.section_page(stream, page)?,
                    );
                    *overflow = overflow
                        .checked_add(span.blocks(block_bytes)?)
                        .ok_or(SegmentedError::AddressExhausted)?;
                    spanned += 1;
                    1
                }
                DictionaryPage::Inline { used, compressed } => {
                    let selected = remaining
                        .get(..used)
                        .ok_or(SegmentedError::Corrupt("missing planned dictionary prefix"))?;
                    if selected.iter().any(|(_, key)| *key as usize > inline_limit) {
                        return Err(SegmentedError::Corrupt("inline page names a spanned term"));
                    }
                    let records = selected.iter().map(inline_record);
                    format::replay_record_block(
                        records,
                        compressed,
                        blocks.section_page(stream, page)?,
                    )?;
                    used
                }
            };
            remaining = &remaining[used..];
        }
        if !remaining.is_empty() {
            return Err(SegmentedError::Corrupt(
                "dictionary emission did not consume all records",
            ));
        }
    }
    Ok((stream, spanned))
}

fn fixed_layout(
    blocks: &mut ImagePages<'_>,
    rows: usize,
    width: usize,
) -> Result<Stream, SegmentedError> {
    blocks.claim(rows, rows.div_ceil((blocks.block_bytes - 9) / width))
}

fn fixed_header(
    block: &mut [u8],
    kind: u8,
    count: usize,
    width: usize,
) -> Result<(), SegmentedError> {
    let encoded_count = u32::try_from(count).map_err(|_| SegmentedError::AddressExhausted)?;
    let encoded_size =
        u32::try_from(count * width).map_err(|_| SegmentedError::AddressExhausted)?;
    block[0] = kind;
    crate::bytes::put_u32_le(block, 1, encoded_count).expect("fixed block count field");
    crate::bytes::put_u32_le(block, 5, encoded_size).expect("fixed block size field");
    Ok(())
}

fn append_fixed(
    blocks: &mut ImagePages<'_>,
    rows: usize,
    kind: u8,
    width: usize,
    block_bytes: usize,
    mut encode: impl FnMut(Range<usize>, u64, &mut [u8]) -> Result<(), SegmentedError>,
) -> Result<Stream, SegmentedError> {
    let stream = fixed_layout(blocks, rows, width)?;
    if blocks.data.is_some() {
        let per_block = (block_bytes - 9) / width;
        for (page, start) in (0..rows).step_by(per_block).enumerate() {
            let count = (rows - start).min(per_block);
            let page = u64::try_from(page).map_err(|_| SegmentedError::AddressExhausted)?;
            let slot = stream
                .first
                .checked_add(page)
                .ok_or(SegmentedError::AddressExhausted)?;
            let block = blocks.section_page(stream, page)?;
            fixed_header(block, kind, count, width)?;
            encode(start..start + count, slot, &mut block[9..9 + count * width])?;
        }
    }
    Ok(stream)
}

fn append_quads<'a>(
    blocks: &mut ImagePages<'_>,
    rows: &[QuadIds<GlobalTermId>],
    table: u8,
    block_bytes: usize,
    summaries: &mut usize,
    value_ranges: &mut usize,
    resolve: impl Fn(GlobalTermId) -> Result<TermRef<'a, GlobalTermId>, SegmentedError>,
) -> Result<Stream, SegmentedError> {
    let stream = append_fixed(
        blocks,
        rows.len(),
        format::QUADS,
        32,
        block_bytes,
        |range, _, payload| {
            for (&q, bytes) in rows[range].iter().zip(payload.as_chunks_mut::<32>().0) {
                format::encode_quad(q, bytes);
            }
            Ok(())
        },
    )?;
    for (page, chunk) in rows.chunks((block_bytes - 9) / 32).enumerate() {
        let intervals =
            crate::annotation_range::seal_intervals_with(chunk.iter().copied(), &resolve)?;
        let first = u64::try_from(*value_ranges).map_err(|_| SegmentedError::AddressExhausted)?;
        let count = u64::try_from(intervals.len()).map_err(|_| SegmentedError::AddressExhausted)?;
        if let Some((summary_stream, ranges_stream)) = blocks.auxiliary {
            let slot = stream
                .first
                .checked_add(u64::try_from(page).map_err(|_| SegmentedError::AddressExhausted)?)
                .ok_or(SegmentedError::AddressExhausted)?;
            let row = summary(chunk, slot, table, first, count)?;
            blocks.put_record(
                summary_stream,
                u64::try_from(*summaries).map_err(|_| SegmentedError::AddressExhausted)?,
                format::SUMMARIES,
                &row,
            )?;
            for (offset, interval) in intervals.iter().enumerate() {
                let ordinal = first
                    .checked_add(
                        u64::try_from(offset).map_err(|_| SegmentedError::AddressExhausted)?,
                    )
                    .ok_or(SegmentedError::AddressExhausted)?;
                blocks.put_record(
                    ranges_stream,
                    ordinal,
                    format::VALUE_RANGES,
                    &value_range_bytes(interval),
                )?;
            }
        }
        *summaries = summaries
            .checked_add(1)
            .ok_or(SegmentedError::AddressExhausted)?;
        *value_ranges = value_ranges
            .checked_add(intervals.len())
            .ok_or(SegmentedError::AddressExhausted)?;
    }
    Ok(stream)
}
pub(super) fn summary(
    rows: &[QuadIds<GlobalTermId>],
    slot: u64,
    table: u8,
    first_range: u64,
    range_count: u64,
) -> Result<[u8; format::SUMMARY_BYTES], SegmentedError> {
    let mut bytes = [0; format::SUMMARY_BYTES];
    crate::bytes::put_u64_le(&mut bytes, 0, slot).expect("fixed summary slot field");
    bytes[8] = table;
    let count = u64::try_from(rows.len()).map_err(|_| SegmentedError::AddressExhausted)?;
    crate::bytes::put_u64_le(&mut bytes, 16, count).expect("fixed summary count field");
    for (axis, bounds) in bytes[24..88].as_chunks_mut::<16>().0.iter_mut().enumerate() {
        let values = rows.iter().map(|q| match axis {
            0 => q.s.index() + 1,
            1 => q.p.index() + 1,
            2 => q.o.index() + 1,
            _ => q.g.map_or(0, |id| id.index() + 1),
        });
        crate::bytes::put_u64_le(bounds, 0, values.clone().min().unwrap_or(0))
            .expect("fixed lower bound field");
        crate::bytes::put_u64_le(bounds, 8, values.max().unwrap_or(0))
            .expect("fixed upper bound field");
    }
    crate::bytes::put_u64_le(&mut bytes, 88, first_range).expect("fixed first range field");
    crate::bytes::put_u64_le(&mut bytes, 96, range_count).expect("fixed range count field");
    Ok(bytes)
}

fn value_range_bytes(
    interval: &crate::annotation_range::ValueInterval<GlobalTermId>,
) -> [u8; format::VALUE_RANGE_BYTES] {
    let mut bytes = [0; format::VALUE_RANGE_BYTES];
    bytes[..8].copy_from_slice(&format::id_bytes(interval.predicate));
    for (index, endpoint) in [
        interval.endpoints.map(|(low, _)| low),
        interval.endpoints.map(|(_, high)| high),
    ]
    .into_iter()
    .enumerate()
    {
        let start = 8 + index * 8;
        crate::bytes::put_u64_le(&mut bytes, start, endpoint.map_or(0, |id| id.index() + 1))
            .expect("fixed interval endpoint field");
    }
    bytes
}

/// Compares generated fixed records with canonical blocks, keeping just a page view.
struct FixedStreamProof<'a, const WIDTH: usize> {
    bytes: &'a [u8],
    header: &'a Header,
    stream: Stream,
    kind: u8,
    errors: [&'static str; 3],
    slot: u64,
    seen: u64,
    unread: &'a [u8],
}

impl<'a, const WIDTH: usize> FixedStreamProof<'a, WIDTH> {
    const fn new(
        bytes: &'a [u8],
        header: &'a Header,
        stream: Stream,
        kind: u8,
        errors: [&'static str; 3],
    ) -> Self {
        Self {
            bytes,
            header,
            stream,
            kind,
            errors,
            slot: stream.first,
            seen: 0,
            unread: &[],
        }
    }

    fn check(&mut self, expected: [u8; WIDTH]) -> Result<(), SegmentedError> {
        if self.seen == self.stream.rows {
            return Err(SegmentedError::Corrupt(self.errors[2]));
        }
        if self.unread.is_empty() {
            if self.slot == self.stream.first + self.stream.blocks {
                return Err(SegmentedError::Corrupt(self.errors[2]));
            }
            let block = block(self.bytes, self.header, self.slot)?;
            self.unread = fixed_records(block, self.kind, WIDTH)?;
            let page_rows = u64::try_from((block.len() - 9) / WIDTH)
                .map_err(|_| SegmentedError::AddressExhausted)?;
            let expected_rows = (self.stream.rows - self.seen).min(page_rows);
            if self.unread.len() / WIDTH
                != usize::try_from(expected_rows).map_err(|_| SegmentedError::AddressExhausted)?
            {
                return Err(SegmentedError::Corrupt(self.errors[0]));
            }
            self.slot += 1;
        }
        let (record, rest) = self.unread.split_at(WIDTH);
        if record != expected {
            return Err(SegmentedError::Corrupt(self.errors[1]));
        }
        self.unread = rest;
        self.seen += 1;
        Ok(())
    }

    fn finish(self) -> Result<(), SegmentedError> {
        if self.seen != self.stream.rows
            || !self.unread.is_empty()
            || self.slot != self.stream.first + self.stream.blocks
        {
            return Err(SegmentedError::Corrupt(self.errors[2]));
        }
        Ok(())
    }
}

fn make_tree(
    bytes: &[u8],
    block_bytes: usize,
    count: u64,
    leaves: u64,
) -> Result<Vec<u8>, SegmentedError> {
    let nodes = usize::try_from(
        leaves
            .checked_mul(2)
            .ok_or(SegmentedError::AddressExhausted)?,
    )
    .map_err(|_| SegmentedError::AddressExhausted)?;
    let tree_bytes = nodes
        .checked_mul(32)
        .ok_or(SegmentedError::AddressExhausted)?;
    let mut tree = Vec::new();
    Memory::new(&mut Resident).reserve(&mut tree, tree_bytes)?;
    tree.resize(tree_bytes, 0);
    fill_tree(bytes, block_bytes, count, leaves, &mut tree)?;
    Ok(tree)
}

fn fill_tree(
    bytes: &[u8],
    block_bytes: usize,
    count: u64,
    leaves: u64,
    tree: &mut [u8],
) -> Result<(), SegmentedError> {
    let leaf = usize::try_from(leaves).map_err(|_| SegmentedError::AddressExhausted)?;
    for index in 0..leaf {
        let digest = if u64::try_from(index).map_err(|_| SegmentedError::AddressExhausted)? < count
        {
            format::leaf_digest(
                u64::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?,
                &bytes[index * block_bytes..(index + 1) * block_bytes],
            )
        } else {
            format::leaf_digest(
                u64::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?,
                &[],
            )
        };
        tree[(leaf + index) * 32..(leaf + index + 1) * 32].copy_from_slice(&digest);
    }
    for node in (1..leaf).rev() {
        let left = tree[node * 64..node * 64 + 32]
            .try_into()
            .map_err(|_| SegmentedError::Corrupt("bad tree child"))?;
        let right = tree[node * 64 + 32..node * 64 + 64]
            .try_into()
            .map_err(|_| SegmentedError::Corrupt("bad tree child"))?;
        tree[node * 32..node * 32 + 32].copy_from_slice(&format::node_digest(&left, &right));
    }
    Ok(())
}

fn block<'a>(bytes: &'a [u8], header: &Header, slot: u64) -> Result<&'a [u8], SegmentedError> {
    let start = slot
        .checked_mul(u64::from(header.block_bytes))
        .and_then(|v| usize::try_from(v).ok())
        .ok_or(SegmentedError::AddressExhausted)?;
    bytes
        .get(
            start
                ..start
                    .checked_add(
                        usize::try_from(header.block_bytes)
                            .map_err(|_| SegmentedError::AddressExhausted)?,
                    )
                    .ok_or(SegmentedError::AddressExhausted)?,
        )
        .ok_or(SegmentedError::Corrupt("missing data block"))
}
#[derive(Clone, Copy)]
struct ResidentOverflow<'a> {
    pages: &'a [u8],
    block_bytes: usize,
    key_bytes: usize,
}

impl<'a> ResidentOverflow<'a> {
    fn new(
        bytes: &'a [u8],
        header: &Header,
        span: format::OverflowSpan,
    ) -> Result<Self, SegmentedError> {
        let block_bytes = header.block_bytes as usize;
        let first = header
            .overflow
            .first
            .checked_add(span.first)
            .ok_or(SegmentedError::AddressExhausted)?;
        let end = first
            .checked_add(span.blocks(block_bytes)?)
            .ok_or(SegmentedError::AddressExhausted)?;
        let offset = |slot| {
            u64::checked_mul(slot, u64::from(header.block_bytes))
                .and_then(|offset| usize::try_from(offset).ok())
                .ok_or(SegmentedError::AddressExhausted)
        };
        let pages = bytes
            .get(offset(first)?..offset(end)?)
            .ok_or(SegmentedError::Corrupt("missing overflow blocks"))?;
        let key_bytes =
            usize::try_from(span.bytes).map_err(|_| SegmentedError::AddressExhausted)?;
        let mut remaining = key_bytes;
        for page in pages.chunks_exact(block_bytes) {
            let width = remaining.min(block_bytes - format::OVERFLOW_HEADER_BYTES);
            format::overflow_payload(page, width)?;
            remaining -= width;
        }
        if remaining != 0 {
            return Err(SegmentedError::Corrupt("false expanded dictionary size"));
        }
        Ok(Self {
            pages,
            block_bytes,
            key_bytes,
        })
    }

    fn parts(self) -> impl Iterator<Item = &'a [u8]> {
        // Construction already validated all page widths, framing and padding.
        self.pages
            .chunks_exact(self.block_bytes)
            .scan(self.key_bytes, move |remaining, page| {
                let start = format::OVERFLOW_HEADER_BYTES;
                let width = (*remaining).min(self.block_bytes - start);
                *remaining -= width;
                Some(&page[start..start + width])
            })
    }
}

fn append_resident_overflow(
    bytes: &[u8],
    header: &Header,
    span: format::OverflowSpan,
    key: &mut Vec<u8>,
    memory: &mut Memory<'_, Resident>,
) -> Result<(), SegmentedError> {
    let overflow = ResidentOverflow::new(bytes, header, span)?;
    key.clear();
    memory.reserve(key, overflow.key_bytes)?;
    for part in overflow.parts() {
        key.extend_from_slice(part);
    }
    Ok(())
}

struct OriginalLiteral {
    index: usize,
    datatype: GlobalTermId,
    language: Option<Range<usize>>,
}

struct DictionaryReplay<'m, 'r> {
    first: u64,
    builder: RdfDatasetBuilder,
    local_ids: Vec<TermId>,
    footprints: Vec<u64>,
    original_literals: Vec<OriginalLiteral>,
    original_languages: String,
    datatype_scratch: String,
    memory: &'m mut Memory<'r, Resident>,
    quoted: bool,
    max_footprint: u64,
}

impl DictionaryReplay<'_, '_> {
    fn push(&mut self, id: GlobalTermId, key: &[u8]) -> Result<(), SegmentedError> {
        let index = self.local_ids.len();
        let ordinal = u64::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?;
        if id.index().checked_sub(self.first) != Some(ordinal) {
            return Err(SegmentedError::Corrupt(
                "dictionary IDs are not stable dense ordinals",
            ));
        }
        let term = format::decode_term(key)?;
        let footprint = expanded_footprint(
            term.map_ids(GlobalTermId::index),
            self.first,
            &self.footprints,
        )?;
        let reference = |id: GlobalTermId| -> Result<TermId, SegmentedError> {
            id.index()
                .checked_sub(self.first)
                .and_then(|local| usize::try_from(local).ok())
                .and_then(|local| self.local_ids.get(local))
                .copied()
                .ok_or(SegmentedError::Corrupt(
                    "cyclic, forward or foreign RDF term reference",
                ))
        };
        let local = match term {
            TermRef::Iri(iri) => {
                super::super::absolute::check_absolute(iri)
                    .map_err(|_| SegmentedError::Corrupt("non-absolute dictionary IRI"))?;
                self.builder.intern_iri(iri)
            }
            TermRef::Blank { label, scope } => self.builder.intern_blank(label, scope),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let TermRef::Iri(iri) = self.builder.resolve(reference(datatype)?) else {
                    return Err(SegmentedError::Corrupt("literal datatype is not an IRI"));
                };
                self.datatype_scratch.clear();
                self.memory.push_str(&mut self.datatype_scratch, iri)?;
                self.builder.intern_literal_parts(
                    lexical,
                    Some(&self.datatype_scratch),
                    language,
                    direction,
                )
            }
            TermRef::Triple { s, p, o } => {
                self.quoted = true;
                self.builder
                    .intern_triple(reference(s)?, reference(p)?, reference(o)?)
            }
        }?;
        if local.index() != index {
            return Err(SegmentedError::Corrupt(
                "dictionary duplicates or non-normalized RDF values",
            ));
        }
        // The builder preserves the wire fields except for literal datatype
        // expansion and the language case fold. Keep just those differences,
        // so reverse proof does not reread the compressed forward pages.
        if let (
            TermRef::Literal {
                datatype, language, ..
            },
            TermRef::Literal {
                datatype: expanded,
                language: folded,
                ..
            },
        ) = (
            term,
            self.builder
                .resolve(local)
                .map_ids(|id| map_id(id, self.first)),
        ) && (datatype != expanded || language != folded)
        {
            let original_language = if let Some(tag) = language.filter(|_| language != folded) {
                let start = self.original_languages.len();
                self.memory.push_str(&mut self.original_languages, tag)?;
                Some(start..self.original_languages.len())
            } else {
                None
            };
            self.memory.push(
                &mut self.original_literals,
                OriginalLiteral {
                    index,
                    datatype,
                    language: original_language,
                },
            )?;
        }
        self.memory.push(&mut self.footprints, footprint)?;
        self.memory.push(&mut self.local_ids, local)?;
        self.max_footprint = self.max_footprint.max(footprint);
        Ok(())
    }

    fn wire_encoding(
        &self,
        index: usize,
        local: TermId,
    ) -> Result<format::TermEncoding<'_>, SegmentedError> {
        let mut term = self
            .builder
            .resolve(local)
            .map_ids(|id| map_id(id, self.first));
        if let Ok(entry) = self
            .original_literals
            .binary_search_by_key(&index, |entry| entry.index)
        {
            let original = &self.original_literals[entry];
            let TermRef::Literal {
                datatype, language, ..
            } = &mut term
            else {
                return Err(SegmentedError::Corrupt(
                    "stored wire override is not a literal",
                ));
            };
            *datatype = original.datatype;
            if let Some(range) = &original.language {
                *language = Some(&self.original_languages[range.clone()]);
            }
        }
        format::TermEncoding::new(term)
    }
}

fn replay_dictionary(
    bytes: &[u8],
    header: &Header,
    replay: &mut DictionaryReplay<'_, '_>,
) -> Result<(), SegmentedError> {
    let mut scratch = Vec::new();
    let mut inflater = None;
    let (mut overflow_blocks, mut overflow_rows) = (0, 0);
    for slot in header.dictionary.first..header.dictionary.first + header.dictionary.blocks {
        let encoded = block(bytes, header, slot)?;
        if replay.local_ids.len() as u64 >= header.terms {
            return Err(SegmentedError::Corrupt(
                "forward dictionary block packing is false",
            ));
        }
        let decoded = header.dictionary_decode_bound(encoded[0]);
        if encoded[0] == format::SPANNED_DICTIONARY {
            let (id, span) = format::overflow_descriptor(encoded, header.overflow)?;
            if span.first != overflow_blocks {
                return Err(SegmentedError::Corrupt("noncontiguous overflow coverage"));
            }
            if u64::from(span.bytes) > decoded {
                return Err(SegmentedError::Corrupt("record decode bound exceeded"));
            }
            append_resident_overflow(bytes, header, span, &mut scratch, replay.memory)?;
            overflow_blocks += span.blocks(header.block_bytes as usize)?;
            overflow_rows += 1;
            replay.push(id, &scratch)?;
        } else {
            let mut page = format::InlineFrames::open(
                encoded,
                header.records_per_block,
                decoded,
                &mut inflater,
            )?;
            while let Some(id) = page.read_next(&mut scratch, replay.memory)? {
                if replay.local_ids.len() as u64 >= header.terms {
                    return Err(SegmentedError::Corrupt(
                        "forward dictionary block packing is false",
                    ));
                }
                replay.push(id, &scratch)?;
            }
            page.finish()?;
        }
    }
    if overflow_blocks != header.overflow.blocks || overflow_rows != header.overflow.rows {
        return Err(SegmentedError::Corrupt("incomplete overflow coverage"));
    }
    if replay.local_ids.len() as u64 != header.terms {
        return Err(SegmentedError::Corrupt("dictionary cardinality is false"));
    }
    replay.memory.release_vec(scratch)?;
    Ok(())
}

fn validate_reverse(
    bytes: &[u8],
    header: &Header,
    replay: &DictionaryReplay<'_, '_>,
) -> Result<(), SegmentedError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut seen = super::super::pack::bits::BitVec::new();
    if let Some(last) = replay.local_ids.len().checked_sub(1) {
        seen.set_with_memory(last, false, &mut memory)?;
    }
    let mut previous: Option<format::TermEncoding<'_>> = None;
    let mut count = 0_u64;
    for slot in header.reverse.first..header.reverse.first + header.reverse.blocks {
        let records = fixed_records(block(bytes, header, slot)?, format::REVERSE, 8)?;
        if records.is_empty() {
            return Err(SegmentedError::Corrupt(
                "reverse index block packing is false",
            ));
        }
        for record in records.as_chunks::<8>().0 {
            let id = Reader::at(record, 0).id()?;
            let index = id
                .index()
                .checked_sub(header.first_term_index)
                .and_then(|local| usize::try_from(local).ok())
                .ok_or(SegmentedError::Corrupt("reverse index names foreign ID"))?;
            let local = *replay
                .local_ids
                .get(index)
                .ok_or(SegmentedError::Corrupt("reverse index names foreign ID"))?;
            if seen.get(index) != Some(false) {
                return Err(SegmentedError::Corrupt(
                    "forward/reverse correspondence is false",
                ));
            }
            let current = replay.wire_encoding(index, local)?;
            if let Some(previous) = previous.as_ref() {
                let a = previous.parts();
                let b = current.parts();
                if !format::compare_key_parts(a.into_iter(), b.into_iter())
                    .1
                    .is_lt()
                {
                    return Err(SegmentedError::Corrupt(
                        "reverse index is not globally strictly ordered",
                    ));
                }
            }
            seen.set_with_memory(index, true, &mut memory)?;
            previous = Some(current);
            count += 1;
        }
    }
    if count != header.terms {
        return Err(SegmentedError::Corrupt(
            "reverse index omits dictionary values",
        ));
    }
    Ok(())
}

fn validate_content(bytes: &[u8], header: &Header) -> Result<(), SegmentedError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut replay = DictionaryReplay {
        first: header.first_term_index,
        builder: RdfDatasetBuilder::new(),
        local_ids: Vec::new(),
        footprints: Vec::new(),
        original_literals: Vec::new(),
        original_languages: String::new(),
        datatype_scratch: String::new(),
        memory: &mut memory,
        quoted: false,
        max_footprint: 0,
    };
    replay_dictionary(bytes, header, &mut replay)?;
    validate_reverse(bytes, header, &replay)?;
    let DictionaryReplay {
        mut builder,
        local_ids,
        quoted,
        max_footprint,
        footprints,
        datatype_scratch,
        original_literals,
        original_languages,
        memory,
        ..
    } = replay;
    memory.release_vec(footprints)?;
    memory.release_string(datatype_scratch)?;
    memory.release_vec(original_literals)?;
    memory.release_string(original_languages)?;
    let native_id = |id: GlobalTermId| -> Result<TermId, SegmentedError> {
        id.index()
            .checked_sub(header.first_term_index)
            .and_then(|v| usize::try_from(v).ok())
            .and_then(|v| local_ids.get(v).copied())
            .ok_or(SegmentedError::Corrupt(
                "quad names unknown dictionary term",
            ))
    };
    let mut summaries = FixedStreamProof::new(
        bytes,
        header,
        header.summaries,
        format::SUMMARIES,
        [
            "pruning summary block packing is false",
            "pruning summary hides or invents rows",
            "pruning index omits quad partitions",
        ],
    );
    let mut value_ranges = FixedStreamProof::new(
        bytes,
        header,
        header.value_ranges,
        format::VALUE_RANGES,
        [
            "annotation value interval packing is false",
            "annotation value intervals hide or invent candidates",
            "annotation value interval directory is incomplete",
        ],
    );
    let mut previous = None;
    let mut named_graphs = super::super::pack::bits::BitVec::new();
    let mut named_count = 0_u64;
    // Allocate from proved dictionary cardinality, never from an unverified
    // graph count. A default-only dataset does not need this scratch.
    if header.graphs.rows != 0
        && let Some(last) = local_ids.len().checked_sub(1)
    {
        named_graphs.set_with_memory(last, false, memory)?;
    }
    for slot in header.graphs.first..header.graphs.first + header.graphs.blocks {
        let graph_records = fixed_records(block(bytes, header, slot)?, format::GRAPHS, 8)?;
        let remaining = header
            .graphs
            .rows
            .checked_sub(named_count)
            .ok_or(SegmentedError::Corrupt("extra named graph declarations"))?;
        if u64::try_from(graph_records.len() / 8).map_err(|_| SegmentedError::AddressExhausted)?
            != remaining.min((u64::from(header.block_bytes) - 9) / 8)
        {
            return Err(SegmentedError::Corrupt(
                "named graph block packing is false",
            ));
        }
        for encoded in graph_records.as_chunks::<8>().0 {
            let id = Reader::at(encoded, 0).id()?;
            if previous.is_some_and(|previous| previous >= id) {
                return Err(SegmentedError::Corrupt(
                    "named graphs are not strictly ordered",
                ));
            }
            previous = Some(id);
            let local = native_id(id)?;
            named_graphs.set_with_memory(local.index(), true, memory)?;
            builder.declare_named_graph(local)?;
            named_count += 1;
        }
    }
    if named_count != header.graphs.rows {
        return Err(SegmentedError::Corrupt(
            "named-graph declarations are incomplete",
        ));
    }
    let mut rows = Vec::new();
    for (table, stream) in header.streams.iter().enumerate() {
        let mut previous = None;
        let mut count = 0_u64;
        for slot in stream.first..stream.first + stream.blocks {
            let data = fixed_records(block(bytes, header, slot)?, format::QUADS, 32)?;
            rows.clear();
            memory.reserve(&mut rows, data.len() / 32)?;
            for encoded in data.as_chunks::<32>().0 {
                let quad = format::decode_quad(encoded)?;
                let order = (quad.s, quad.p, quad.o, quad.g);
                if previous.is_some_and(|previous| previous >= order) {
                    return Err(SegmentedError::Corrupt(
                        "quad partitions overlap or are unordered",
                    ));
                }
                previous = Some(order);
                count += 1;
                rows.push(quad);
                let (s, p, o) = (native_id(quad.s)?, native_id(quad.p)?, native_id(quad.o)?);
                let g = quad.g.map(native_id).transpose()?;
                if g.is_some_and(|id| named_graphs.get(id.index()) != Some(true)) {
                    return Err(SegmentedError::Corrupt(
                        "named-graph declarations are incomplete",
                    ));
                }
                match table {
                    0 => builder.push_quad(s, p, o, g),
                    1 => {
                        if !matches!(builder.resolve(p), TermRef::Iri(iri) if iri == purrdf_iri::vocab::rdf::REIFIES)
                        {
                            return Err(SegmentedError::Corrupt(
                                "reifier stream predicate is not rdf:reifies",
                            ));
                        }
                        builder.push_reifier_in_graph(s, o, g)
                    }
                    _ => builder.push_annotation_in_graph(s, p, o, g),
                }?;
            }
            let first_range = value_ranges.seen;
            for interval in
                crate::annotation_range::seal_intervals_with(rows.iter().copied(), |id| {
                    // The interner keeps lexical bytes, direction and language
                    // presence. It only overrides an explicit datatype in the
                    // language-tagged case, where intervals never use it.
                    Ok::<_, SegmentedError>(
                        builder
                            .resolve(native_id(id)?)
                            .map_ids(|id| map_id(id, header.first_term_index)),
                    )
                })?
            {
                value_ranges.check(value_range_bytes(&interval))?;
            }
            summaries.check(summary(
                &rows,
                slot,
                table as u8,
                first_range,
                value_ranges.seen - first_range,
            )?)?;
        }
        if count != stream.rows {
            return Err(SegmentedError::Corrupt("quad stream count is false"));
        }
    }
    memory.release_vec(rows)?;
    drop(named_graphs);
    memory.release_vec(local_ids)?;
    summaries.finish()?;
    value_ranges.finish()?;
    if header.flags
        != u8::from(quoted)
            | (u8::from(header.streams[1].rows != 0) << 1)
            | (u8::from(header.streams[2].rows != 0) << 2)
    {
        return Err(SegmentedError::Corrupt(
            "capability claims disagree with RDF streams",
        ));
    }
    if header.max_owned_term_bytes != max_footprint {
        return Err(SegmentedError::Corrupt(
            "expanded term footprint claim is false",
        ));
    }
    let dataset = builder
        .freeze()
        .map_err(|error| SegmentedError::from_rdf(error, "RDF reference/role validation failed"))?;
    validate_statistics(bytes, header, &dataset)?;
    Ok(())
}

fn validate_statistics(
    bytes: &[u8],
    header: &Header,
    dataset: &RdfDataset,
) -> Result<(), SegmentedError> {
    let mut resident = Resident;
    dataset.with_prepared_statistics(&mut Memory::new(&mut resident), |statistics, _| {
        validate_prepared_statistics(bytes, header, statistics)
    })
}

fn validate_prepared_statistics(
    bytes: &[u8],
    header: &Header,
    source_statistics: &LocalStatistics,
) -> Result<(), SegmentedError> {
    if header.source_union != source_statistics.source_union() {
        return Err(SegmentedError::Corrupt(
            "source union disagrees with RDF rows",
        ));
    }
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let statistics =
        translated_statistics(source_statistics, header.first_term_index, &mut memory)?;
    for section in StatisticsSection::ALL {
        let stream = header.statistics[section.index()];
        if statistics.section_rows(section) as u64 != stream.rows {
            return Err(SegmentedError::Corrupt(
                "statistical section count disagrees with RDF rows",
            ));
        }
        let width = section.width();
        let per = (header.block_bytes as usize - 9) / width;
        if stream.rows == 0 {
            continue;
        }
        let size = statistics.section_rows(section).min(per) * width;
        let mut expected = memory.collect(std::iter::repeat_n(0_u8, size))?;
        let mut ordinal = 0_usize;
        for slot in stream.first..stream.first + stream.blocks {
            let records = fixed_records(
                block(bytes, header, slot)?,
                format::statistics_kind(section),
                width,
            )?;
            let remaining = statistics.section_rows(section) - ordinal;
            if records.len() / width != remaining.min(per) {
                return Err(SegmentedError::Corrupt(
                    "statistical block packing disagrees with RDF rows",
                ));
            }
            statistics.encode_records(section, ordinal, &mut expected[..records.len()])?;
            if records != &expected[..records.len()] {
                return Err(SegmentedError::Corrupt(
                    "statistical contents disagree with RDF rows",
                ));
            }
            ordinal += records.len() / width;
        }
        memory.release_vec(expected)?;
    }
    validate_source_directory(
        bytes,
        header,
        source_statistics.effective_source_directory(),
        header.effective_directory,
        format::SOURCE_DIRECTORY,
    )
}

fn validate_source_directory(
    bytes: &[u8],
    header: &Header,
    directory: &super::super::source_directory::SourceDirectory,
    stream: Stream,
    kind: u8,
) -> Result<(), SegmentedError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    if stream.rows != directory.len() as u64 {
        return Err(SegmentedError::Corrupt(
            "source directory count disagrees with RDF rows",
        ));
    }
    let width = super::super::source_directory::RECORD_BYTES;
    let per = (header.block_bytes as usize - 9) / width;
    let mut expected =
        memory.collect(std::iter::repeat_n(0_u8, directory.len().min(per) * width))?;
    let mut ordinal = 0;
    for slot in stream.first..stream.first + stream.blocks {
        let actual = fixed_records(block(bytes, header, slot)?, kind, width)?;
        let count = (directory.len() - ordinal).min(per);
        if actual.len() != count * width {
            return Err(SegmentedError::Corrupt(
                "source directory packing disagrees with RDF rows",
            ));
        }
        directory.encode_records(ordinal, &mut expected[..actual.len()]);
        if actual != &expected[..actual.len()] {
            return Err(SegmentedError::Corrupt(
                "source directory contents disagree with RDF rows",
            ));
        }
        ordinal += count;
    }
    memory.release_vec(expected)?;
    if header.version == format::ADDITIVE_VERSION {
        // A version 7 image's additive prefix sums are a local consistency
        // check only; its readers compare ranges from the records.
        validate_additive_checkpoints(bytes, header, directory)
    } else {
        validate_node_checkpoints(bytes, header, directory)
    }
}

fn validate_node_checkpoints(
    bytes: &[u8],
    header: &Header,
    directory: &super::super::source_directory::SourceDirectory,
) -> Result<(), SegmentedError> {
    let width = super::super::source_directory::NODE_CHECKPOINT_BYTES;
    let checkpoints = directory.node_checkpoints()?;
    let count = checkpoints.len();
    if header.source_checkpoints.rows != count as u64 {
        return Err(SegmentedError::Corrupt(
            "source checkpoint packing disagrees with RDF rows",
        ));
    }
    let per = (header.block_bytes as usize - 9) / width;
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut expected = memory.collect(std::iter::repeat_n(0_u8, count.min(per) * width))?;
    let mut ordinal = 0;
    for slot in header.source_checkpoints.first
        ..header.source_checkpoints.first + header.source_checkpoints.blocks
    {
        let actual = fixed_records(
            block(bytes, header, slot)?,
            format::SOURCE_CHECKPOINTS,
            width,
        )?;
        if actual.len() != (count - ordinal).min(per) * width {
            return Err(SegmentedError::Corrupt(
                "source checkpoint packing disagrees with RDF rows",
            ));
        }
        super::super::source_directory::encode_node_checkpoints(
            &checkpoints[ordinal..ordinal + actual.len() / width],
            &mut expected[..actual.len()],
        );
        if actual != &expected[..actual.len()] {
            return Err(SegmentedError::Corrupt(
                "source checkpoint contents disagree with RDF rows",
            ));
        }
        ordinal += actual.len() / width;
    }
    memory.release_vec(expected)?;
    Ok(())
}

fn validate_additive_checkpoints(
    bytes: &[u8],
    header: &Header,
    directory: &super::super::source_directory::SourceDirectory,
) -> Result<(), SegmentedError> {
    let mut sums = directory.additive_checkpoints();
    let per = (header.block_bytes as usize - 9) / 32;
    for slot in header.source_checkpoints.first
        ..header.source_checkpoints.first + header.source_checkpoints.blocks
    {
        let actual = fixed_records(block(bytes, header, slot)?, format::SOURCE_CHECKPOINTS, 32)?;
        if actual.len() / 32 != sums.len().min(per) {
            return Err(SegmentedError::Corrupt(
                "source checkpoint packing disagrees with RDF rows",
            ));
        }
        for sum in actual.as_chunks::<32>().0 {
            if Some(*sum) != sums.next() {
                return Err(SegmentedError::Corrupt(
                    "source checkpoint contents disagree with RDF rows",
                ));
            }
        }
    }
    Ok(())
}
pub(super) fn fixed_records(bytes: &[u8], kind: u8, width: usize) -> Result<&[u8], SegmentedError> {
    let mut input = Reader::at(bytes, 0);
    if input.byte()? != kind {
        return Err(SegmentedError::Corrupt("wrong fixed-record block kind"));
    }
    let count = usize::try_from(input.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
    let size = usize::try_from(input.u32()?).map_err(|_| SegmentedError::AddressExhausted)?;
    if count == 0 || count.checked_mul(width) != Some(size) {
        return Err(SegmentedError::Corrupt("fixed-record count/size mismatch"));
    }
    let data = input.take(size)?;
    if input.take(input.remaining())?.iter().any(|&byte| byte != 0) {
        return Err(SegmentedError::Corrupt("nonzero fixed-record padding"));
    }
    Ok(data)
}

fn expanded_footprint(
    term: TermRef<'_, u64>,
    first: u64,
    previous: &[u64],
) -> Result<u64, SegmentedError> {
    let base = OWNED_TERM_NODE_BYTES;
    let reference = |id: u64| -> Result<u64, SegmentedError> {
        id.checked_sub(first)
            .and_then(|v| usize::try_from(v).ok())
            .and_then(|v| previous.get(v).copied())
            .ok_or(SegmentedError::Corrupt(
                "non-topological expanded-term reference",
            ))
    };
    let payload = match term {
        TermRef::Iri(iri) => {
            u64::try_from(iri.len()).map_err(|_| SegmentedError::AddressExhausted)?
        }
        TermRef::Blank { label, .. } => {
            u64::try_from(label.len()).map_err(|_| SegmentedError::AddressExhausted)?
        }
        TermRef::Literal {
            lexical,
            datatype,
            language,
            ..
        } => {
            let datatype = reference(datatype)?;
            u64::try_from(lexical.len())
                .map_err(|_| SegmentedError::AddressExhausted)?
                .checked_add(
                    u64::try_from(language.map_or(0, str::len))
                        .map_err(|_| SegmentedError::AddressExhausted)?,
                )
                .and_then(|n| n.checked_add(datatype))
                .ok_or(SegmentedError::AddressExhausted)?
        }
        TermRef::Triple { s, p, o } => {
            let (s, p, o) = (reference(s)?, reference(p)?, reference(o)?);
            s.checked_add(p)
                .and_then(|n| n.checked_add(o))
                .ok_or(SegmentedError::AddressExhausted)?
        }
    };
    base.checked_add(payload)
        .ok_or(SegmentedError::AddressExhausted)
}

/// Compute the same certified expansion law once when a resident carrier is
/// frozen into a publication, never when a query opens or a snapshot is cloned.
pub(super) fn resident_owned_term_bound(dataset: &RdfDataset) -> Result<u64, SegmentedError> {
    use purrdf_lex::allocation::{Memory, Resident, StorageError};
    let mut footprints = Vec::new();
    Memory::new(&mut Resident).reserve(&mut footprints, dataset.term_count())?;
    let mut widest = 0;
    for index in 0..dataset.term_count() {
        let id =
            TermId::from_index(u32::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?)
                .map_err(StorageError::from)?;
        let footprint = expanded_footprint(
            dataset.resolve(id).map_ids(|id| u64::from(id.raw() - 1)),
            0,
            &footprints,
        )?;
        widest = widest.max(footprint);
        footprints.push(footprint);
    }
    Ok(widest)
}

#[cfg(test)]
mod certification_tests {
    use super::*;

    #[test]
    fn freeze_storage_refusal_stays_distinct_from_malformed_rdf() {
        use purrdf_lex::allocation::{Admission, Memory, StorageError};

        struct Ceiling<'a>(&'a core::cell::Cell<usize>);
        impl Admission for Ceiling<'_> {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.0.get() {
                    Err(StorageError::AdmissionFailed)
                } else {
                    Ok(())
                }
            }
        }
        let ceiling = core::cell::Cell::new(usize::MAX);
        let mut grant = Ceiling(&ceiling);
        let mut memory = Memory::new(&mut grant);
        let mut native = RdfDatasetBuilder::new();
        native
            .intern_iri_with_memory("http://example.org/graph", &mut memory)
            .unwrap();
        ceiling.set(memory.admitted_bytes());
        let refusal = native.freeze_with_memory(&mut memory).unwrap_err();
        let diagnostic = crate::RdfDiagnostic::from(refusal);
        assert_eq!(
            diagnostic.storage_error(),
            Some(StorageError::AdmissionFailed)
        );
        assert_eq!(memory.admitted_bytes(), 0);
        for boundary in [
            "resident RDF freeze rejected the fixture",
            "RDF reference/role validation failed",
        ] {
            assert_eq!(
                SegmentedError::from_rdf(diagnostic.clone(), boundary),
                SegmentedError::Storage(StorageError::AdmissionFailed)
            );
        }

        let limits = SegmentedBuildLimits::new(2, 1024, 1, 512, 2).unwrap();
        let mut invalid = SegmentedBuilder::new(limits).unwrap();
        let graph = invalid
            .builder
            .intern_literal(crate::RdfLiteral::simple("literal"))
            .unwrap();
        invalid.builder.declare_named_graph(graph).unwrap();
        assert!(matches!(
            invalid.seal(),
            Err(SegmentedError::Corrupt(
                "resident RDF freeze rejected the fixture"
            ))
        ));
    }

    #[test]
    fn constructor_duplicate_hits_do_not_authorize_unminted_ids() {
        let first = 1_u64 << 40;
        let limits = SegmentedBuildLimits::new(1, 1024, 1, 512, 2)
            .unwrap()
            .with_first_term_index(first)
            .unwrap();
        let mut builder = SegmentedBuilder::new(limits).unwrap();
        let value = TermValue::iri("http://example.org/graph");
        let mut id = None;
        builder
            .intern_batch(&[value.clone(), value], |_, term| id = Some(term))
            .unwrap();
        assert_eq!(builder.builder.term_count(), 1);
        let foreign = GlobalTermId::from_index(first + 1).unwrap();
        for _ in 0..2 {
            assert!(matches!(
                builder.declare_named_graph(foreign),
                Err(SegmentedError::Corrupt("foreign constructor term ID"))
            ));
            assert_eq!(builder.admitted_rows, 0);
        }
        builder.declare_named_graph(id.unwrap()).unwrap();
        assert_eq!(builder.admitted_rows, 1);
    }

    /// Duplicate values consume neither new dictionary slots nor stored text.
    #[test]
    fn constructor_counts_distinct_terms_and_stored_text() {
        let datatype = purrdf_xsd::datatype::XSD_STRING;
        let literal = |lexical_form| TermValue::Literal {
            lexical_form,
            datatype: datatype.to_owned(),
            language: None,
            direction: None,
        };
        let values = [
            TermValue::iri("urn:s"),
            TermValue::iri("urn:p"),
            literal("é".repeat(50)),
            literal("ø".repeat(50)),
        ];
        // Two IRIs, one shared datatype and two lexical forms of 100 bytes each.
        let text = u32::try_from(10 + datatype.len() + 200).unwrap();
        let mut images = Vec::new();
        for repeats in [1, 32] {
            let limits = SegmentedBuildLimits::new(5, text, 2, 512, 4).unwrap();
            let mut builder = SegmentedBuilder::new(limits).unwrap();
            let mut ids = Vec::new();
            builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
            for _ in 0..repeats {
                let mut repeated = Vec::new();
                builder
                    .intern_batch(&values[2..], |_, id| repeated.push(id))
                    .unwrap();
                assert_eq!(repeated, ids[2..]);
            }
            for &object in &ids[2..] {
                builder
                    .push_quad(QuadIds {
                        s: ids[0],
                        p: ids[1],
                        o: object,
                        g: None,
                    })
                    .unwrap();
            }
            images.push(builder.seal().unwrap());
        }
        assert_eq!(images[0].bytes(), images[1].bytes());
    }

    /// A later child's refusal preserves the accepted prefix and reusable IDs.
    #[test]
    fn constructor_late_quota_refusal_rolls_back_the_whole_value() {
        let value = TermValue::Triple {
            s: crate::TermBox::new(TermValue::iri("urn:s")),
            p: crate::TermBox::new(TermValue::iri("urn:p")),
            o: crate::TermBox::new(TermValue::iri("urn:o")),
        };
        let first = 1_u64 << 40;
        for (terms, text) in [(3, 1000), (10, 18)] {
            let limits = SegmentedBuildLimits::new(terms, text, 0, 512, 4)
                .unwrap()
                .with_first_term_index(first)
                .unwrap();
            let mut builder = SegmentedBuilder::new(limits).unwrap();
            let mut visited = Vec::new();
            assert_eq!(
                builder.intern_batch(&[TermValue::iri("urn:base"), value.clone()], |_, id| {
                    visited.push(id);
                },),
                Err(SegmentedError::ConstructionLimit)
            );
            assert_eq!(visited, [GlobalTermId::from_index(first).unwrap()]);
            let mut resumed = Vec::new();
            builder
                .intern_batch(
                    &[TermValue::iri("urn:base"), TermValue::iri("urn:next")],
                    |_, id| resumed.push(id),
                )
                .unwrap();
            assert_eq!(
                resumed,
                [
                    GlobalTermId::from_index(first).unwrap(),
                    GlobalTermId::from_index(first + 1).unwrap(),
                ]
            );
            builder.seal().unwrap();
        }
    }

    /// Distinct reifier rows share the one implicit predicate dictionary entry.
    #[test]
    fn constructor_reifier_bindings_charge_the_predicate_once() {
        let values = [
            TermValue::iri("urn:r0"),
            TermValue::iri("urn:r1"),
            TermValue::iri("urn:r2"),
            TermValue::Triple {
                s: crate::TermBox::new(TermValue::iri("urn:s")),
                p: crate::TermBox::new(TermValue::iri("urn:p")),
                o: crate::TermBox::new(TermValue::iri("urn:o")),
            },
        ];
        let text = 33 + u32::try_from(purrdf_iri::vocab::rdf::REIFIES.len()).unwrap();
        let limits = SegmentedBuildLimits::new(8, text, 4, 512, 4).unwrap();
        let mut builder = SegmentedBuilder::new(limits).unwrap();
        let mut ids = Vec::new();
        builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
        for &reifier in &ids[..3] {
            builder.push_reifier(reifier, ids[3], None).unwrap();
        }
        builder.push_reifier(ids[0], ids[3], None).unwrap();
        assert_eq!(builder.statement_count(), 3);
        builder.seal().unwrap();
    }

    /// A reifier's statement wraps its nested object in one more triple term.
    /// Both that boundary and deeper stored claims retain their IDs on reopen.
    #[test]
    fn constructor_deep_reified_claims_reopen_and_lookup_on_a_small_stack() {
        use crate::DatasetView;
        purrdf_stack::on_stack(128 * 1024, || {
            for levels in [16, 256] {
                let mut object = TermValue::iri("urn:o");
                for _ in 0..=levels {
                    object = TermValue::Triple {
                        s: crate::TermBox::new(TermValue::iri("urn:s")),
                        p: crate::TermBox::new(TermValue::iri("urn:p")),
                        o: crate::TermBox::new(object),
                    };
                }
                let values = [TermValue::iri("urn:r"), object];
                let limits = SegmentedBuildLimits::new(1000, 1000, 2, 4096, 32).unwrap();
                let mut builder = SegmentedBuilder::new(limits).unwrap();
                let mut ids = Vec::new();
                builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
                builder.push_reifier(ids[0], ids[1], None).unwrap();
                let image = builder.seal().unwrap();
                let session = image
                    .open_session(SegmentedReadLimits::new(
                        32_000_000,
                        4,
                        100_000,
                        100_000_000,
                        2,
                    ))
                    .unwrap();
                assert_eq!(session.triple_term_nesting_bound(), None);
                assert_eq!(session.term_id_by_value(&values[1]).unwrap(), Some(ids[1]));
                assert_eq!(session.reifier_quads().next().unwrap().o, ids[1]);
                assert_eq!(session.term_value(ids[1]).unwrap(), values[1]);
                assert!(session.read_error().is_none());
            }
        })
        .expect("the thread starts");
    }

    /// Refusing the implicit predicate publishes no row and consumes no term ID.
    #[test]
    fn constructor_refused_predicate_preserves_rows_and_dictionary_ids() {
        let values = [
            TermValue::iri("urn:r"),
            TermValue::Triple {
                s: crate::TermBox::new(TermValue::iri("urn:s")),
                p: crate::TermBox::new(TermValue::iri("urn:p")),
                o: crate::TermBox::new(TermValue::iri("urn:o")),
            },
        ];
        for (terms, text) in [(5, 1000), (6, 20)] {
            let limits = SegmentedBuildLimits::new(terms, text, 1, 512, 4).unwrap();
            let mut builder = SegmentedBuilder::new(limits).unwrap();
            let mut ids = Vec::new();
            builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
            assert_eq!(
                builder.push_reifier(ids[0], ids[1], None),
                Err(SegmentedError::ConstructionLimit)
            );
            assert_eq!(builder.statement_count(), 0);
            assert!(matches!(
                builder.declare_named_graph(GlobalTermId::from_index(5).unwrap()),
                Err(SegmentedError::Corrupt("foreign constructor term ID"))
            ));
            let mut predicate = None;
            builder
                .intern_batch(&[TermValue::iri("urn:p")], |_, id| predicate = Some(id))
                .unwrap();
            builder
                .push_quad(QuadIds {
                    s: ids[0],
                    p: predicate.unwrap(),
                    o: ids[0],
                    g: None,
                })
                .unwrap();
            assert_eq!(builder.statement_count(), 1);
            builder.seal().unwrap();
        }
    }

    fn image() -> SegmentedImage {
        let limits = SegmentedBuildLimits::new(100, 20_000, 100, 512, 4).unwrap();
        let mut builder = SegmentedBuilder::new(limits).unwrap();
        let values = (0..13)
            .map(|n| {
                TermValue::iri(format!(
                    "http://example.org/{}/{n:02}",
                    if n % 2 == 0 {
                        "a".repeat(180)
                    } else {
                        "z".repeat(180)
                    }
                ))
            })
            .collect::<Vec<_>>();
        let mut ids = Vec::new();
        builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
        for &id in &ids {
            builder
                .push_quad(QuadIds {
                    s: id,
                    p: ids[0],
                    o: id,
                    g: None,
                })
                .unwrap();
        }
        builder.declare_named_graph(ids[12]).unwrap();
        builder.seal().unwrap()
    }

    // This attacker recomputes every leaf, internal node, and the root. Full
    // certification must still reject semantic lies; byte integrity alone cannot
    // authorize index pruning or establish RDF reference closure.
    fn rehash(bytes: &mut [u8], receipt: &SegmentedReceipt) {
        let width = usize::try_from(receipt.block_bytes).unwrap();
        let data_len = usize::try_from(receipt.block_count).unwrap() * width;
        let tree = make_tree(
            &bytes[..data_len],
            width,
            receipt.block_count,
            receipt.leaf_count,
        )
        .unwrap();
        bytes[data_len..].copy_from_slice(&tree);
    }

    #[test]
    fn statistical_counts_are_selective_and_preserve_original_read_refusal() {
        use crate::{DatasetView, GraphMatch, SnapshotStatistics, StatisticsStream};
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let limits = SegmentedReadLimits::new(8_000_000, 2, 10_000, 100_000_000, 8);
        let source = image.open_session(limits).unwrap();
        let opening = source.evidence().with_requests(<[_]>::len);
        let predicate = source
            .term_id_by_value(&TermValue::iri(format!(
                "http://example.org/{}/00",
                "a".repeat(180)
            )))
            .unwrap()
            .unwrap();
        let before = source.evidence().with_requests(<[_]>::len);
        let count = source
            .predicate(predicate, GraphMatch::Any, StatisticsStream::Asserted)
            .unwrap();
        assert_eq!(count.rows.value, 13);
        let content = DatasetView::source_summary(
            &source,
            predicate,
            GraphMatch::Any,
            StatisticsStream::Asserted,
        )
        .unwrap()
        .unwrap();
        assert_eq!(content.statement_count(), 13);
        assert_eq!(
            content.literal_rows(),
            [purrdf_hash::multiset::Multiset256::default(); 2]
        );
        assert_eq!(count.subjects.unwrap().value, 13);
        assert_eq!(
            source
                .predicate_object(
                    predicate,
                    predicate,
                    GraphMatch::Any,
                    StatisticsStream::Asserted
                )
                .unwrap()
                .value,
            1
        );
        assert_eq!(
            source
                .predicate_object_kind(
                    predicate,
                    crate::RdfTermKind::Iri,
                    None,
                    GraphMatch::Any,
                    StatisticsStream::Asserted
                )
                .unwrap()
                .value,
            13
        );
        let data_end = image.receipt().block_count * u64::from(header.block_bytes);
        let first = header.statistics[0].first;
        let domain = header.statistics[1];
        source.evidence().with_requests(|requests| {
            let mut reads = 0;
            for request in &requests[before..] {
                if request.position() >= data_end {
                    continue;
                }
                let slot = request.position() / u64::from(header.block_bytes);
                assert!(
                    slot >= first,
                    "metadata lookup read a dictionary or RDF row block"
                );
                assert!(
                    !(domain.first..domain.first + domain.blocks).contains(&slot),
                    "count lookup read distinct-ID payloads"
                );
                reads += 1;
            }
            assert!(reads > 0);
        });
        for content in [false, true] {
            let refused = image
                .open_session(SegmentedReadLimits::new(
                    8_000_000,
                    2,
                    u32::try_from(opening).unwrap(),
                    100_000_000,
                    8,
                ))
                .unwrap();
            let predicate = refused.attach(source.handle(predicate).unwrap()).unwrap();
            if content {
                assert_eq!(
                    DatasetView::source_summary(
                        &refused,
                        predicate,
                        GraphMatch::Any,
                        StatisticsStream::Asserted,
                    ),
                    Err(SegmentedError::EvidenceExhausted)
                );
            } else {
                assert!(
                    refused
                        .predicate(predicate, GraphMatch::Any, StatisticsStream::Asserted)
                        .is_none()
                );
            }
            assert_eq!(
                refused.read_error(),
                Some(SegmentedError::EvidenceExhausted)
            );
        }
    }

    #[test]
    fn recomputed_hashes_cannot_certify_false_statistics() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let start =
            usize::try_from(header.statistics[0].first).unwrap() * header.block_bytes as usize + 9;
        for word in [4, 17, 18, 22, 26] {
            let mut bytes = image.bytes().to_vec();
            bytes[start + word * 8] ^= 1;
            rehash(&mut bytes, image.receipt());
            assert!(matches!(
                SegmentedImage::certify(bytes),
                Err(SegmentedError::Corrupt(
                    "statistical contents disagree with RDF rows"
                ))
            ));
        }
    }

    #[test]
    fn overflow_pages_and_descriptors_require_complete_semantic_certification() {
        for block_bytes in [512, 4096] {
            let mut builder = SegmentedBuilder::new(
                SegmentedBuildLimits::new(16, 100_000, 0, block_bytes, 4).unwrap(),
            )
            .unwrap();
            let values = [
                TermValue::iri(format!("urn:overflow:{}a", "x".repeat(20_000))),
                TermValue::iri(format!("urn:overflow:{}b", "x".repeat(20_000))),
                TermValue::simple_literal("literal".repeat(2000)),
            ];
            builder.intern_batch(&values, |_, _| {}).unwrap();
            let image = builder.seal().unwrap();
            let header = Header::decode(image.bytes()).unwrap();
            assert_eq!(header.block_bytes, block_bytes);
            assert_eq!(header.overflow.rows, 3);
            let descriptor = (header.dictionary.first
                ..header.dictionary.first + header.dictionary.blocks)
                .find(|&slot| {
                    block(image.bytes(), &header, slot).unwrap()[0] == format::SPANNED_DICTIONARY
                })
                .unwrap();
            // The two IRIs agree across several pages. Only the last byte
            // distinguishes them in the authenticated reverse index.
            let (_, span) = format::overflow_descriptor(
                block(image.bytes(), &header, descriptor).unwrap(),
                header.overflow,
            )
            .unwrap();
            let width = block_bytes as usize;
            let last = header.overflow.first + span.first + span.blocks(width).unwrap() - 1;
            let start = format::OVERFLOW_HEADER_BYTES;
            let tail = (span.bytes as usize - 1) % (width - start);
            let mut bytes = image.bytes().to_vec();
            let byte = &mut bytes[last as usize * width + start + tail];
            assert_eq!(*byte, b'a');
            *byte = b'c';
            rehash(&mut bytes, image.receipt());
            assert_eq!(
                SegmentedImage::certify(bytes).unwrap_err(),
                SegmentedError::Corrupt("reverse index is not globally strictly ordered")
            );
            for (slot, offset) in [
                (descriptor, 5),
                (descriptor, format::RECORD_HEADER + 8),
                (header.overflow.first, 1),
                (header.overflow.first, 5),
            ] {
                let mut bytes = image.bytes().to_vec();
                bytes[slot as usize * block_bytes as usize + offset] ^= 128;
                rehash(&mut bytes, image.receipt());
                assert!(
                    matches!(
                        SegmentedImage::certify(bytes),
                        Err(SegmentedError::Corrupt(_))
                    ),
                    "a rebuilt hash tree cannot authorize false overflow metadata or values"
                );
            }
            let mut false_header = header;
            false_header.overflow.rows += 1;
            let mut bytes = image.bytes().to_vec();
            bytes[..format::HEADER_BYTES].copy_from_slice(&false_header.encode());
            rehash(&mut bytes, image.receipt());
            assert!(matches!(
                SegmentedImage::certify(bytes),
                Err(SegmentedError::Corrupt("incomplete overflow coverage"))
            ));
        }
    }

    #[test]
    fn source_directory_rejects_an_overflowing_physical_population() {
        let image = image();
        let mut header = Header::decode(image.bytes()).unwrap();
        header.streams[0].rows = u64::MAX;
        header.streams[1].rows = 1;
        assert_eq!(
            Header::decode(&header.encode()).unwrap_err(),
            SegmentedError::AddressExhausted
        );
    }

    #[test]
    fn recomputed_hashes_cannot_certify_a_false_source_directory() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        {
            let stream = header.effective_directory;
            assert!(stream.rows > 0);
            let start = stream.first as usize * header.block_bytes as usize + 9;
            for offset in [0, 32, 36] {
                let mut bytes = image.bytes().to_vec();
                bytes[start + offset] ^= 1;
                rehash(&mut bytes, image.receipt());
                assert_eq!(
                    SegmentedImage::certify(bytes).unwrap_err(),
                    SegmentedError::Corrupt("source directory contents disagree with RDF rows")
                );
            }
        }
    }

    #[test]
    fn sparse_source_nodes_match_a_full_scan_across_pages_and_checkpoints() {
        use crate::ir::source_directory::{DirectoryRead, first_differing_bit, node_of};
        use crate::source::SourceDifferenceWork;

        let image = directory_image(1025);
        let header = Header::decode(image.bytes()).unwrap();
        assert_eq!(header.version, format::VERSION);
        let count = header.effective_directory.rows;
        let compact = header.effective_directory.blocks + header.source_checkpoints.blocks;
        let previous = count.div_ceil((u64::from(header.block_bytes) - 9) / 88);
        assert!(
            compact * 2 < previous,
            "directory pages: {compact} vs {previous}"
        );
        assert_eq!(image.bytes(), directory_image(1025).bytes());
        let session = image
            .open_session(SegmentedReadLimits::new(
                8_000_000,
                4,
                100_000,
                100_000_000,
                8,
            ))
            .unwrap();
        let directory = session.source_directory();
        let digests: Vec<_> = (0..directory.len().unwrap())
            .map(|index| directory.record(index).unwrap().digest)
            .collect();
        // Every range a comparison splits into, answered from at most one
        // checkpoint and fewer than one block of records.
        let mut pending = Vec::new();
        pending.push(0..digests.len());
        while let Some(range) = pending.pop() {
            let mut work = SourceDifferenceWork::default();
            assert_eq!(
                directory.node(range.clone(), &mut work).unwrap(),
                node_of(&digests[range.clone()]),
                "range {range:?}"
            );
            assert!(work.members < 32);
            if let Some(bit) = first_differing_bit(&digests[range.start], &digests[range.end - 1]) {
                let middle = range.start
                    + digests[range.clone()].partition_point(|digest| {
                        digest[usize::from(bit / 8)] & (0x80 >> (bit % 8)) == 0
                    });
                pending.extend([range.start..middle, middle..range.end]);
            }
        }
        assert_eq!(
            session
                .source_root(&mut SourceDifferenceWork::default())
                .unwrap(),
            node_of(&digests)
        );
        assert!(matches!(
            directory.node(0..1026, &mut SourceDifferenceWork::default()),
            Err(SegmentedError::Corrupt(_))
        ));
    }

    fn directory_image(count: u32) -> SegmentedImage {
        let mut builder = SegmentedBuilder::new(
            SegmentedBuildLimits::new(count + 2, 100_000, count, 512, 4).unwrap(),
        )
        .unwrap();
        let values = (0..count + 2)
            .map(|index| TermValue::iri(format!("urn:directory:{index}")))
            .collect::<Vec<_>>();
        let mut ids = Vec::new();
        builder.intern_batch(&values, |_, id| ids.push(id)).unwrap();
        for &id in &ids[2..] {
            builder
                .push_quad(QuadIds {
                    s: id,
                    p: ids[0],
                    o: ids[1],
                    g: None,
                })
                .unwrap();
        }
        builder.seal().unwrap()
    }

    #[test]
    fn recomputed_hashes_cannot_certify_false_sparse_source_checkpoints() {
        let image = directory_image(1025);
        let header = Header::decode(image.bytes()).unwrap();
        let stream = header.source_checkpoints;
        assert!(stream.blocks > 1);
        // A boundary byte and a node byte of the first record in each block.
        for (slot, offset) in
            (stream.first..stream.first + stream.blocks).flat_map(|slot| [(slot, 0), (slot, 17)])
        {
            let mut bytes = image.bytes().to_vec();
            bytes[slot as usize * header.block_bytes as usize + 9 + offset] ^= 1;
            rehash(&mut bytes, image.receipt());
            assert_eq!(
                SegmentedImage::certify(bytes).unwrap_err(),
                SegmentedError::Corrupt("source checkpoint contents disagree with RDF rows")
            );
        }
    }

    /// The magic and the version name one checkpoint law together; a
    /// mismatched pair is refused before any checkpoint is read.
    #[test]
    fn header_refuses_a_magic_that_disagrees_with_its_version() {
        let image = image();
        assert_eq!(
            Header::decode(image.bytes()).unwrap().version,
            format::VERSION
        );
        let refusal = SegmentedError::Corrupt(
            "unsupported segmented format/profile; regenerate the image from RDF input",
        );
        let mut older = image.bytes().to_vec();
        crate::bytes::put_u32_le(&mut older, 8, format::ADDITIVE_VERSION).unwrap();
        assert_eq!(Header::decode(&older).unwrap_err(), refusal);
        let mut renamed = image.bytes().to_vec();
        renamed[..8].copy_from_slice(b"PURRSEG7");
        assert_eq!(Header::decode(&renamed).unwrap_err(), refusal);
    }

    #[test]
    fn recomputed_hashes_cannot_certify_a_false_source_union() {
        let image = image();
        assert!(
            Header::decode(image.bytes())
                .unwrap()
                .source_union
                .population()
                > 0
        );
        for population in [false, true] {
            let mut bytes = image.bytes().to_vec();
            if population {
                bytes[format::HEADER_BYTES - 48..format::HEADER_BYTES - 32].fill(0);
            } else {
                bytes[format::HEADER_BYTES - 32] ^= 1;
            }
            rehash(&mut bytes, image.receipt());
            assert_eq!(
                SegmentedImage::certify(bytes).unwrap_err(),
                SegmentedError::Corrupt(if population {
                    "effective source directory packing mismatch"
                } else {
                    "source union disagrees with RDF rows"
                })
            );
        }
    }

    #[test]
    fn recomputed_hashes_cannot_certify_false_reverse_identifiers() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let first = usize::try_from(header.reverse.first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + 9;
        for mutation in 0..4 {
            let mut bytes = image.bytes().to_vec();
            let expected = match mutation {
                0 => {
                    bytes.copy_within(first + 8..first + 16, first);
                    "forward/reverse correspondence is false"
                }
                1 => {
                    bytes[first..first + 8].copy_from_slice(&u64::MAX.to_le_bytes());
                    "reverse index names foreign ID"
                }
                2 => {
                    bytes[first..first + 8].fill(0);
                    "zero dictionary reference"
                }
                _ => {
                    for offset in 0..8 {
                        bytes.swap(first + offset, first + 8 + offset);
                    }
                    "reverse index is not globally strictly ordered"
                }
            };
            rehash(&mut bytes, &image.receipt);
            assert_eq!(
                SegmentedImage::certify(bytes).unwrap_err(),
                SegmentedError::Corrupt(expected)
            );
        }
    }

    #[test]
    fn reverse_proof_orders_original_language_spelling() {
        let limits = SegmentedBuildLimits::new(16, 1_000, 0, 512, 1).unwrap();
        let mut builder = SegmentedBuilder::new(limits).unwrap();
        builder
            .intern_batch(
                &[
                    TermValue::lang_literal("label", "en"),
                    TermValue::lang_literal("label", "fr"),
                ],
                |_, _| {},
            )
            .unwrap();
        let image = builder.seal().unwrap();
        let header = Header::decode(image.bytes()).unwrap();
        let width = header.block_bytes as usize;
        let mut bytes = image.bytes().to_vec();
        let dictionary_start = header.dictionary.first as usize * width;
        let dictionary_end = dictionary_start + header.dictionary.blocks as usize * width;
        let language = bytes[dictionary_start..dictionary_end]
            .windows(7)
            .position(|window| window == [2, 0, 0, 0, b'f', b'r', 0])
            .unwrap()
            + dictionary_start
            + 4;
        bytes[language..language + 2].copy_from_slice(b"FR");
        rehash(&mut bytes, image.receipt());
        assert_eq!(
            SegmentedImage::certify(bytes.clone()).unwrap_err(),
            SegmentedError::Corrupt("reverse index is not globally strictly ordered")
        );
        let reverse = header.reverse.first as usize * width + 9;
        for offset in 0..8 {
            bytes.swap(reverse + 8 + offset, reverse + 16 + offset);
        }
        rehash(&mut bytes, image.receipt());
        SegmentedImage::certify(bytes).unwrap();
    }

    #[test]
    fn recomputed_hashes_cannot_certify_false_pruning_bounds() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let mut bytes = image.bytes().to_vec();
        let record = usize::try_from(header.summaries.first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + 9;
        bytes[record + 24..record + 32].copy_from_slice(&999_u64.to_le_bytes());
        bytes[record + 32..record + 40].copy_from_slice(&999_u64.to_le_bytes());
        rehash(&mut bytes, &image.receipt);
        assert_eq!(
            SegmentedImage::certify(bytes).unwrap_err(),
            SegmentedError::Corrupt("pruning summary hides or invents rows")
        );

        let mut bytes = image.bytes().to_vec();
        let record = usize::try_from(header.value_ranges.first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + 9;
        bytes[record..record + 8].copy_from_slice(&999_u64.to_le_bytes());
        rehash(&mut bytes, &image.receipt);
        assert_eq!(
            SegmentedImage::certify(bytes).unwrap_err(),
            SegmentedError::Corrupt("annotation value intervals hide or invent candidates")
        );
    }

    #[test]
    fn recomputed_hashes_cannot_certify_foreign_quad_references() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let mut bytes = image.bytes().to_vec();
        let record = usize::try_from(header.streams[0].first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + 9;
        bytes[record..record + 8].copy_from_slice(&999_u64.to_le_bytes());
        rehash(&mut bytes, &image.receipt);
        assert!(matches!(
            SegmentedImage::certify(bytes),
            Err(SegmentedError::Corrupt(_))
        ));
        let mut bytes = image.bytes().to_vec();
        // The existing first subject is a valid local term but not a named graph.
        bytes.copy_within(record..record + 8, record + 24);
        rehash(&mut bytes, &image.receipt);
        assert_eq!(
            SegmentedImage::certify(bytes).unwrap_err(),
            SegmentedError::Corrupt("named-graph declarations are incomplete")
        );
    }

    #[test]
    fn recomputed_hashes_cannot_certify_an_underpriced_owned_term() {
        let image = image();
        let mut bytes = image.bytes().to_vec();
        bytes[40..48].copy_from_slice(&1_u64.to_le_bytes());
        rehash(&mut bytes, &image.receipt);
        assert!(matches!(
            SegmentedImage::certify(bytes),
            Err(SegmentedError::Corrupt(_))
        ));
    }
}
