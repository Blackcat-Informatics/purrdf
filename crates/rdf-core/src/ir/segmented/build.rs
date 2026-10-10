// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded resident fixture construction and complete untrusted-image validation.

use std::sync::Arc;

use crate::backend::TermFactory;
use crate::{GlobalTermId, QuadIds, RdfDataset, RdfDatasetBuilder, TermId, TermRef, TermValue};

use super::format::{self, DecodedRecords, Header, Reader, Stream};
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
    /// Choose all resident capacities and the persisted block layout explicitly.
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
            || records_per_block > 256
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

/// Small resident construction path with pre-admitted term/string/row capacities.
/// The ordinary validated IR builder owns RDF normalization and positional checks.
/// This constructor's bounded IDs are translated arithmetically, without a global
/// per-page translation vector.
#[derive(Debug)]
pub struct SegmentedBuilder {
    builder: RdfDatasetBuilder,
    limits: SegmentedBuildLimits,
    admitted_terms: u64,
    admitted_bytes: u64,
    admitted_rows: u64,
}
impl SegmentedBuilder {
    /// Reserve the resident fixture capacities before accepting input.
    #[must_use]
    pub fn new(limits: SegmentedBuildLimits) -> Self {
        let mut builder = RdfDatasetBuilder::new();
        builder.reserve_for_replay(
            usize::try_from(limits.terms).expect("u32 capacity fits supported pointer widths"),
            usize::try_from(limits.term_bytes).expect("u32 capacity fits supported pointer widths"),
            usize::try_from(limits.rows).expect("u32 capacity fits supported pointer widths"),
        );
        Self {
            builder,
            limits,
            admitted_terms: 0,
            admitted_bytes: 0,
            admitted_rows: 0,
        }
    }

    fn global(&self, id: TermId) -> GlobalTermId {
        GlobalTermId::from_index(
            self.limits.first_term_index
                + u64::try_from(id.index()).expect("resident term index fits u64"),
        )
    }
    fn local(&self, id: GlobalTermId) -> Result<TermId, SegmentedError> {
        let index = id
            .index()
            .checked_sub(self.limits.first_term_index)
            .ok_or(SegmentedError::Corrupt("foreign constructor term ID"))?;
        if index >= self.admitted_terms {
            return Err(SegmentedError::Corrupt("foreign constructor term ID"));
        }
        Ok(TermId::from_index(
            u32::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?,
        ))
    }

    /// Intern a batch through one normalization authority, without allocating a batch
    /// output. Every returned ID remains valid in this generation after sealing.
    ///
    /// # Errors
    /// Refuses before interning when conservative referenced-term/string bounds
    /// would exceed the pre-reserved resident capacities.
    pub fn intern_batch(
        &mut self,
        values: &[TermValue],
        mut visit: impl FnMut(&TermValue, GlobalTermId),
    ) -> Result<(), SegmentedError> {
        for value in values {
            let (terms, bytes) = value_bound(value)?;
            let next_terms = self
                .admitted_terms
                .checked_add(terms)
                .ok_or(SegmentedError::AddressExhausted)?;
            let next_bytes = self
                .admitted_bytes
                .checked_add(bytes)
                .ok_or(SegmentedError::AddressExhausted)?;
            if next_terms > u64::from(self.limits.terms)
                || next_bytes > u64::from(self.limits.term_bytes)
            {
                return Err(SegmentedError::ConstructionLimit);
            }
            // The conservative bound includes dependency IRIs and composite blank
            // labels. It is intentionally independent of whether interning hits.
            self.admitted_terms = next_terms;
            self.admitted_bytes = next_bytes;
            let id = self.builder.intern_value(value);
            visit(value, self.global(id));
        }
        Ok(())
    }

    fn admit_row(&mut self) -> Result<(), SegmentedError> {
        let next = self
            .admitted_rows
            .checked_add(1)
            .ok_or(SegmentedError::AddressExhausted)?;
        if next > u64::from(self.limits.rows) {
            return Err(SegmentedError::ConstructionLimit);
        }
        self.admitted_rows = next;
        Ok(())
    }
    /// Add a primary quad. Duplicate rows are normalized by the shared IR freeze.
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
        self.admit_row()?;
        self.builder.push_quad(s, p, o, g);
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
        let terms = self
            .admitted_terms
            .checked_add(1)
            .ok_or(SegmentedError::AddressExhausted)?;
        let bytes = self
            .admitted_bytes
            .checked_add(
                u64::try_from(purrdf_iri::vocab::rdf::REIFIES.len())
                    .expect("vocabulary length fits u64"),
            )
            .ok_or(SegmentedError::AddressExhausted)?;
        if terms > u64::from(self.limits.terms) || bytes > u64::from(self.limits.term_bytes) {
            return Err(SegmentedError::ConstructionLimit);
        }
        self.admit_row()?;
        self.admitted_terms = terms;
        self.admitted_bytes = bytes;
        self.builder.push_reifier_in_graph(r, t, g);
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
        self.admit_row()?;
        self.builder.push_annotation_in_graph(s, p, o, g);
        Ok(())
    }
    /// Preserve an explicit named graph, including a graph with no rows.
    ///
    /// # Errors
    /// Refuses foreign IDs and the same resident row/declaration ceiling.
    pub fn declare_named_graph(&mut self, graph: GlobalTermId) -> Result<(), SegmentedError> {
        let graph = self.local(graph)?;
        self.admit_row()?;
        self.builder.declare_named_graph(graph);
        Ok(())
    }
    /// Validate RDF, encode the versioned blocks and certify their complete index law.
    ///
    /// # Errors
    /// Returns structural RDF validation, format-capacity or complete certification
    /// errors. No usable receipt is minted for a partial or invalid image.
    pub fn seal(self) -> Result<SegmentedImage, SegmentedError> {
        let dataset = self
            .builder
            .freeze()
            .map_err(|_| SegmentedError::Corrupt("resident RDF freeze rejected the fixture"))?;
        encode_dataset(dataset.as_ref(), self.limits)
    }
}

fn value_bound(value: &TermValue) -> Result<(u64, u64), SegmentedError> {
    let mut terms = 0_u64;
    let mut bytes = 0_u64;
    let mut stack: crate::SmallVec<[(&TermValue, u32); 64]> = crate::SmallVec::default();
    stack.push((value, 0_u32));
    while let Some((value, depth)) = stack.pop() {
        if depth > 16 {
            return Err(SegmentedError::ConstructionLimit);
        }
        terms = terms
            .checked_add(1)
            .ok_or(SegmentedError::AddressExhausted)?;
        let size = match value {
            TermValue::Iri(iri) => {
                super::super::absolute::check_absolute(iri)
                    .map_err(|_| SegmentedError::Corrupt("constructor IRI is not absolute"))?;
                iri.len()
            }
            TermValue::Blank { label, .. } => label.len(),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                ..
            } => {
                super::super::absolute::check_absolute(datatype)
                    .map_err(|_| SegmentedError::Corrupt("constructor datatype is not absolute"))?;
                terms = terms
                    .checked_add(1)
                    .ok_or(SegmentedError::AddressExhausted)?;
                // Composite lexical blank labels may be interned by the shared IR;
                // one byte is a conservative lower bound on each label's spelling.
                if crate::cdt_blank::is_cdt_datatype(datatype) {
                    terms = terms
                        .checked_add(
                            u64::try_from(lexical_form.len())
                                .map_err(|_| SegmentedError::AddressExhausted)?,
                        )
                        .ok_or(SegmentedError::AddressExhausted)?;
                }
                lexical_form
                    .len()
                    .checked_mul(2)
                    .and_then(|v| v.checked_add(datatype.len()))
                    .and_then(|v| v.checked_add(language.as_ref().map_or(0, String::len)))
                    .ok_or(SegmentedError::AddressExhausted)?
            }
            TermValue::Triple { s, p, o } => {
                stack.extend([(&**o, depth + 1), (&**p, depth + 1), (&**s, depth + 1)]);
                0
            }
        };
        bytes = bytes
            .checked_add(u64::try_from(size).map_err(|_| SegmentedError::AddressExhausted)?)
            .ok_or(SegmentedError::AddressExhausted)?;
    }
    Ok((terms, bytes))
}

/// Complete persisted image plus the authority produced by full certification.
#[derive(Debug, Clone)]
pub struct SegmentedImage {
    bytes: Arc<[u8]>,
    receipt: SegmentedReceipt,
}
impl SegmentedImage {
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
        SegmentedBytes::new(Arc::clone(&self.bytes), self.receipt.snapshot)
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
        validate_content(&bytes, header)?;
        let snapshot = format::snapshot_digest(&root, header.block_bytes, count);
        Ok(Self {
            bytes: bytes.into(),
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
        encode_dataset(source.as_ref(), limits)
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
        let mut builder = SegmentedBuilder::new(limits);
        super::super::import::DatasetImporter::new(&mut builder.builder, source.as_ref()).append();
        builder.admitted_terms = terms;
        builder.admitted_bytes = term_bytes;
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
) -> Result<SegmentedImage, SegmentedError> {
    let block_bytes =
        usize::try_from(limits.block_bytes).map_err(|_| SegmentedError::AddressExhausted)?;
    let records_per_block =
        usize::try_from(limits.records_per_block).map_err(|_| SegmentedError::AddressExhausted)?;
    let mut blocks = vec![vec![0_u8; block_bytes]];
    let mut records = Vec::with_capacity(dataset.term_count());
    let mut footprints = Vec::with_capacity(dataset.term_count());
    for index in 0..dataset.term_count() {
        let local =
            TermId::from_index(u32::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?);
        let mut key = Vec::new();
        format::encode_term(
            dataset
                .resolve(local)
                .map_ids(|id| map_id(id, limits.first_term_index)),
            &mut key,
        )?;
        footprints.push(expanded_footprint(
            format::decode_term(&key)?,
            limits.first_term_index,
            &footprints,
        )?);
        records.push((map_id(local, limits.first_term_index), key));
    }
    let dictionary = append_records(
        &mut blocks,
        &records,
        format::DICTIONARY,
        records_per_block,
        block_bytes,
    )?;
    records.sort_by(|a, b| a.1.cmp(&b.1));
    let reverse = append_records(
        &mut blocks,
        &records,
        format::REVERSE,
        records_per_block,
        block_bytes,
    )?;
    let mut summaries = Vec::new();
    let mut streams = [Stream::default(); 3];
    for (table, rows) in [
        dataset.quads().collect::<Vec<_>>(),
        dataset.reifier_quads().collect(),
        dataset.annotation_quads().collect(),
    ]
    .into_iter()
    .enumerate()
    {
        let mut rows: Vec<_> = rows
            .into_iter()
            .map(|q| map_quad(q, limits.first_term_index))
            .collect();
        rows.sort_by_key(|q| (q.s, q.p, q.o, q.g));
        rows.dedup();
        streams[table] =
            append_quads(&mut blocks, &rows, table as u8, block_bytes, &mut summaries)?;
    }
    let mut graphs: Vec<_> = dataset
        .named_graphs()
        .map(|id| map_id(id, limits.first_term_index))
        .collect();
    graphs.sort_unstable();
    graphs.dedup();
    let graphs = append_fixed(
        &mut blocks,
        graphs.chunks((block_bytes - 9) / 8).map(|chunk| {
            let mut bytes = Vec::with_capacity(chunk.len() * 8);
            for &id in chunk {
                format::push_id(&mut bytes, id);
            }
            bytes
        }),
        u64::try_from(graphs.len()).map_err(|_| SegmentedError::AddressExhausted)?,
        format::GRAPHS,
        8,
        block_bytes,
    )?;
    let summary_rows =
        u64::try_from(summaries.len()).map_err(|_| SegmentedError::AddressExhausted)?;
    let summaries = append_fixed(
        &mut blocks,
        summaries
            .chunks((block_bytes - 9) / format::SUMMARY_BYTES)
            .map(|chunk| {
                let mut bytes = Vec::with_capacity(chunk.len() * format::SUMMARY_BYTES);
                for summary in chunk {
                    bytes.extend_from_slice(summary);
                }
                bytes
            }),
        summary_rows,
        format::SUMMARIES,
        format::SUMMARY_BYTES,
        block_bytes,
    )?;
    let caps = dataset.capabilities();
    let flags = u8::from(caps.quoted_triples)
        | (u8::from(caps.reifiers) << 1)
        | (u8::from(caps.annotations) << 2);
    let header = Header {
        block_bytes: limits.block_bytes,
        records_per_block: limits.records_per_block,
        first_term_index: limits.first_term_index,
        max_owned_term_bytes: footprints
            .iter()
            .map(|(bytes, _)| *bytes)
            .max()
            .unwrap_or(0),
        terms: u64::try_from(dataset.term_count()).map_err(|_| SegmentedError::AddressExhausted)?,
        dictionary,
        reverse,
        streams,
        graphs,
        summaries,
        flags,
    };
    let encoded = header.encode();
    blocks[0][..encoded.len()].copy_from_slice(&encoded);
    let count = u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)?;
    let leaves = count
        .checked_next_power_of_two()
        .ok_or(SegmentedError::AddressExhausted)?;
    let mut bytes = blocks.concat();
    bytes.extend_from_slice(&make_tree(&bytes, block_bytes, count, leaves)?);
    SegmentedImage::certify(bytes)
}

fn append_records(
    blocks: &mut Vec<Vec<u8>>,
    records: &[(GlobalTermId, Vec<u8>)],
    kind: u8,
    chunk: usize,
    block_bytes: usize,
) -> Result<Stream, SegmentedError> {
    let first = u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)?;
    for records in records.chunks(chunk) {
        blocks.push(format::record_block(kind, records, block_bytes)?);
    }
    Ok(Stream {
        first,
        blocks: u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)? - first,
        rows: u64::try_from(records.len()).map_err(|_| SegmentedError::AddressExhausted)?,
    })
}
fn append_fixed(
    blocks: &mut Vec<Vec<u8>>,
    chunks: impl Iterator<Item = Vec<u8>>,
    rows: u64,
    kind: u8,
    width: usize,
    block_bytes: usize,
) -> Result<Stream, SegmentedError> {
    let first = u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)?;
    for chunk in chunks {
        let mut block = Vec::with_capacity(block_bytes);
        block.push(kind);
        block.extend_from_slice(
            &u32::try_from(chunk.len() / width)
                .map_err(|_| SegmentedError::AddressExhausted)?
                .to_le_bytes(),
        );
        block.extend_from_slice(
            &u32::try_from(chunk.len())
                .map_err(|_| SegmentedError::AddressExhausted)?
                .to_le_bytes(),
        );
        block.extend_from_slice(&chunk);
        block.resize(block_bytes, 0);
        blocks.push(block);
    }
    Ok(Stream {
        first,
        blocks: u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)? - first,
        rows,
    })
}
fn append_quads(
    blocks: &mut Vec<Vec<u8>>,
    rows: &[QuadIds<GlobalTermId>],
    table: u8,
    block_bytes: usize,
    summaries: &mut Vec<Vec<u8>>,
) -> Result<Stream, SegmentedError> {
    let first = u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)?;
    for chunk in rows.chunks((block_bytes - 9) / 32) {
        let mut bytes = Vec::with_capacity(chunk.len() * 32);
        for &q in chunk {
            format::encode_quad(q, &mut bytes);
        }
        let slot = u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)?;
        summaries.push(summary(chunk, slot, table)?);
        append_fixed(
            blocks,
            std::iter::once(bytes),
            u64::try_from(chunk.len()).map_err(|_| SegmentedError::AddressExhausted)?,
            format::QUADS,
            32,
            block_bytes,
        )?;
    }
    Ok(Stream {
        first,
        blocks: u64::try_from(blocks.len()).map_err(|_| SegmentedError::AddressExhausted)? - first,
        rows: u64::try_from(rows.len()).map_err(|_| SegmentedError::AddressExhausted)?,
    })
}
pub(super) fn summary(
    rows: &[QuadIds<GlobalTermId>],
    slot: u64,
    table: u8,
) -> Result<Vec<u8>, SegmentedError> {
    let mut bytes = Vec::with_capacity(format::SUMMARY_BYTES);
    bytes.extend_from_slice(&slot.to_le_bytes());
    bytes.push(table);
    bytes.extend_from_slice(&[0; 7]);
    bytes.extend_from_slice(
        &u64::try_from(rows.len())
            .map_err(|_| SegmentedError::AddressExhausted)?
            .to_le_bytes(),
    );
    for axis in 0..4 {
        let values = rows.iter().map(|q| match axis {
            0 => q.s.index() + 1,
            1 => q.p.index() + 1,
            2 => q.o.index() + 1,
            _ => q.g.map_or(0, |id| id.index() + 1),
        });
        bytes.extend_from_slice(&values.clone().min().unwrap_or(0).to_le_bytes());
        bytes.extend_from_slice(&values.max().unwrap_or(0).to_le_bytes());
    }
    Ok(bytes)
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
    let mut tree = vec![
        0_u8;
        nodes
            .checked_mul(32)
            .ok_or(SegmentedError::AddressExhausted)?
    ];
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
    Ok(tree)
}

fn block(bytes: &[u8], header: Header, slot: u64) -> Result<&[u8], SegmentedError> {
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
fn validate_content(bytes: &[u8], header: Header) -> Result<(), SegmentedError> {
    let mut keys = Vec::with_capacity(
        usize::try_from(header.terms).map_err(|_| SegmentedError::AddressExhausted)?,
    );
    for slot in header.dictionary.first..header.dictionary.first + header.dictionary.blocks {
        let records = DecodedRecords::decode(
            block(bytes, header, slot)?,
            format::DICTIONARY,
            header.records_per_block,
            u64::from(header.block_bytes) * u64::from(header.records_per_block),
        )?;
        let remaining = header.terms
            - u64::try_from(keys.len()).map_err(|_| SegmentedError::AddressExhausted)?;
        if u64::try_from(records.entries.len()).map_err(|_| SegmentedError::AddressExhausted)?
            != remaining.min(u64::from(header.records_per_block))
        {
            return Err(SegmentedError::Corrupt(
                "forward dictionary block packing is false",
            ));
        }
        for (index, (id, _)) in records.entries.iter().enumerate() {
            if id.index()
                != header.first_term_index
                    + u64::try_from(keys.len()).map_err(|_| SegmentedError::AddressExhausted)?
            {
                return Err(SegmentedError::Corrupt(
                    "dictionary IDs are not stable dense ordinals",
                ));
            }
            keys.push(records.key(index).to_vec());
        }
    }
    if u64::try_from(keys.len()).map_err(|_| SegmentedError::AddressExhausted)? != header.terms {
        return Err(SegmentedError::Corrupt("dictionary cardinality is false"));
    }
    let mut seen = vec![false; keys.len()];
    let mut previous: Option<Vec<u8>> = None;
    let mut reverse_count = 0_u64;
    for slot in header.reverse.first..header.reverse.first + header.reverse.blocks {
        let records = DecodedRecords::decode(
            block(bytes, header, slot)?,
            format::REVERSE,
            header.records_per_block,
            u64::from(header.block_bytes) * u64::from(header.records_per_block),
        )?;
        for (index, (id, _)) in records.entries.iter().enumerate() {
            let key = records.key(index);
            if previous.as_deref().is_some_and(|previous| previous >= key) {
                return Err(SegmentedError::Corrupt(
                    "reverse index is not globally strictly ordered",
                ));
            }
            let local = id
                .index()
                .checked_sub(header.first_term_index)
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(SegmentedError::Corrupt("reverse index names foreign ID"))?;
            if keys
                .get(local)
                .is_none_or(|forward| forward.as_slice() != key)
                || seen.get(local).copied().unwrap_or(true)
            {
                return Err(SegmentedError::Corrupt(
                    "forward/reverse correspondence is false",
                ));
            }
            seen[local] = true;
            previous = Some(key.to_vec());
            reverse_count += 1;
        }
    }
    if reverse_count != header.terms || seen.iter().any(|seen| !seen) {
        return Err(SegmentedError::Corrupt(
            "reverse index omits dictionary values",
        ));
    }
    let mut builder = RdfDatasetBuilder::new();
    let mut footprints = Vec::with_capacity(keys.len());
    let mut local_ids = Vec::with_capacity(keys.len());
    let mut quoted = false;
    for (index, key) in keys.iter().enumerate() {
        let term = format::decode_term(key)?;
        footprints.push(expanded_footprint(
            term,
            header.first_term_index,
            &footprints,
        )?);
        let reference = |id: GlobalTermId| -> Result<TermId, SegmentedError> {
            let local = id
                .index()
                .checked_sub(header.first_term_index)
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(SegmentedError::Corrupt("foreign RDF reference"))?;
            if local >= index {
                return Err(SegmentedError::Corrupt("cyclic/forward RDF term reference"));
            }
            Ok(local_ids[local])
        };
        let id = match term {
            TermRef::Iri(iri) => {
                super::super::absolute::check_absolute(iri)
                    .map_err(|_| SegmentedError::Corrupt("non-absolute dictionary IRI"))?;
                builder.intern_iri(iri)
            }
            TermRef::Blank { label, scope } => builder.intern_blank(label, scope),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let datatype = reference(datatype)?;
                // Reference closure and datatype role are checked before borrowing
                // its value into the interner, without a fabricated fallback IRI.
                let original = format::decode_term(&keys[datatype.index()])?;
                let TermRef::Iri(iri) = original else {
                    return Err(SegmentedError::Corrupt("literal datatype is not an IRI"));
                };
                builder.intern_literal_parts(lexical, Some(iri), language, direction)
            }
            TermRef::Triple { s, p, o } => {
                quoted = true;
                builder.intern_triple(reference(s)?, reference(p)?, reference(o)?)
            }
        };
        if id.index() != index {
            return Err(SegmentedError::Corrupt(
                "dictionary duplicates or non-normalized RDF values",
            ));
        }
        local_ids.push(id);
    }
    let native_id = |id: GlobalTermId| -> Result<TermId, SegmentedError> {
        id.index()
            .checked_sub(header.first_term_index)
            .and_then(|v| usize::try_from(v).ok())
            .and_then(|v| local_ids.get(v).copied())
            .ok_or(SegmentedError::Corrupt(
                "quad names unknown dictionary term",
            ))
    };
    let mut expected_summaries = Vec::new();
    let mut referenced_graphs = std::collections::BTreeSet::new();
    for (table, stream) in header.streams.iter().enumerate() {
        let mut previous = None;
        let mut count = 0_u64;
        for slot in stream.first..stream.first + stream.blocks {
            let data = fixed_records(block(bytes, header, slot)?, format::QUADS, 32)?;
            let mut rows = Vec::with_capacity(data.len() / 32);
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
                referenced_graphs.extend(quad.g);
                match table {
                    0 => builder.push_quad(s, p, o, g),
                    1 => {
                        if !matches!(format::decode_term(&keys[p.index()])?, TermRef::Iri(iri) if iri == purrdf_iri::vocab::rdf::REIFIES)
                        {
                            return Err(SegmentedError::Corrupt(
                                "reifier stream predicate is not rdf:reifies",
                            ));
                        }
                        builder.push_reifier_in_graph(s, o, g);
                    }
                    _ => builder.push_annotation_in_graph(s, p, o, g),
                }
            }
            expected_summaries.push(summary(&rows, slot, table as u8)?);
        }
        if count != stream.rows {
            return Err(SegmentedError::Corrupt("quad stream count is false"));
        }
    }
    let mut previous = None;
    let mut graphs = std::collections::BTreeSet::new();
    for slot in header.graphs.first..header.graphs.first + header.graphs.blocks {
        let graph_records = fixed_records(block(bytes, header, slot)?, format::GRAPHS, 8)?;
        let remaining = header
            .graphs
            .rows
            .checked_sub(u64::try_from(graphs.len()).map_err(|_| SegmentedError::AddressExhausted)?)
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
            graphs.insert(id);
            builder.declare_named_graph(native_id(id)?);
        }
    }
    if u64::try_from(graphs.len()).map_err(|_| SegmentedError::AddressExhausted)?
        != header.graphs.rows
        || !referenced_graphs.is_subset(&graphs)
    {
        return Err(SegmentedError::Corrupt(
            "named-graph declarations are incomplete",
        ));
    }
    let mut summary_index = 0;
    for slot in header.summaries.first..header.summaries.first + header.summaries.blocks {
        let summary_records = fixed_records(
            block(bytes, header, slot)?,
            format::SUMMARIES,
            format::SUMMARY_BYTES,
        )?;
        let remaining = expected_summaries
            .len()
            .checked_sub(summary_index)
            .ok_or(SegmentedError::Corrupt("extra pruning summaries"))?;
        if summary_records.len() / format::SUMMARY_BYTES
            != remaining.min(
                (usize::try_from(header.block_bytes)
                    .map_err(|_| SegmentedError::AddressExhausted)?
                    - 9)
                    / format::SUMMARY_BYTES,
            )
        {
            return Err(SegmentedError::Corrupt(
                "pruning summary block packing is false",
            ));
        }
        for encoded in summary_records.as_chunks::<{ format::SUMMARY_BYTES }>().0 {
            if expected_summaries
                .get(summary_index)
                .is_none_or(|summary| summary.as_slice() != encoded)
            {
                return Err(SegmentedError::Corrupt(
                    "pruning summary hides or invents rows",
                ));
            }
            summary_index += 1;
        }
    }
    if summary_index != expected_summaries.len()
        || u64::try_from(summary_index).map_err(|_| SegmentedError::AddressExhausted)?
            != header.summaries.rows
    {
        return Err(SegmentedError::Corrupt(
            "pruning index omits quad partitions",
        ));
    }
    if header.flags
        != u8::from(quoted)
            | (u8::from(header.streams[1].rows != 0) << 1)
            | (u8::from(header.streams[2].rows != 0) << 2)
    {
        return Err(SegmentedError::Corrupt(
            "capability claims disagree with RDF streams",
        ));
    }
    if header.max_owned_term_bytes
        != footprints
            .iter()
            .map(|(bytes, _)| *bytes)
            .max()
            .unwrap_or(0)
    {
        return Err(SegmentedError::Corrupt(
            "expanded term footprint claim is false",
        ));
    }
    builder
        .freeze()
        .map_err(|_| SegmentedError::Corrupt("RDF reference/role validation failed"))?;
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
    term: TermRef<'_, GlobalTermId>,
    first: u64,
    previous: &[(u64, u32)],
) -> Result<(u64, u32), SegmentedError> {
    let base = OWNED_TERM_NODE_BYTES;
    let reference = |id: GlobalTermId| -> Result<(u64, u32), SegmentedError> {
        id.index()
            .checked_sub(first)
            .and_then(|v| usize::try_from(v).ok())
            .and_then(|v| previous.get(v).copied())
            .ok_or(SegmentedError::Corrupt(
                "non-topological expanded-term reference",
            ))
    };
    let (payload, depth) = match term {
        TermRef::Iri(iri) => (
            u64::try_from(iri.len()).map_err(|_| SegmentedError::AddressExhausted)?,
            0,
        ),
        TermRef::Blank { label, .. } => (
            u64::try_from(label.len()).map_err(|_| SegmentedError::AddressExhausted)?,
            0,
        ),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            ..
        } => (
            u64::try_from(lexical.len())
                .map_err(|_| SegmentedError::AddressExhausted)?
                .checked_add(
                    u64::try_from(language.map_or(0, str::len))
                        .map_err(|_| SegmentedError::AddressExhausted)?,
                )
                .and_then(|n| n.checked_add(reference(datatype).ok()?.0))
                .ok_or(SegmentedError::AddressExhausted)?,
            0,
        ),
        TermRef::Triple { s, p, o } => {
            let (s, p, o) = (reference(s)?, reference(p)?, reference(o)?);
            (
                s.0.checked_add(p.0)
                    .and_then(|n| n.checked_add(o.0))
                    .ok_or(SegmentedError::AddressExhausted)?,
                s.1.max(p.1).max(o.1) + 1,
            )
        }
    };
    if depth > 16 {
        return Err(SegmentedError::Corrupt(
            "expanded term nesting exceeds validated IR limit",
        ));
    }
    Ok((
        base.checked_add(payload)
            .ok_or(SegmentedError::AddressExhausted)?,
        depth,
    ))
}

#[cfg(test)]
mod certification_tests {
    use super::*;

    fn image() -> SegmentedImage {
        let limits = SegmentedBuildLimits::new(100, 20_000, 100, 1024, 4).unwrap();
        let mut builder = SegmentedBuilder::new(limits);
        let values = (0..13)
            .map(|n| TermValue::iri(format!("http://example.org/{n:02}")))
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
    fn recomputed_hashes_cannot_certify_a_false_dictionary_fence() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let mut bytes = image.bytes().to_vec();
        let fence = usize::try_from(header.dictionary.first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + format::RECORD_HEADER;
        bytes[fence + 9] ^= 1;
        rehash(&mut bytes, &image.receipt);
        assert!(matches!(
            SegmentedImage::certify(bytes),
            Err(SegmentedError::Corrupt(_))
        ));
    }

    #[test]
    fn recomputed_hashes_cannot_certify_false_pruning_bounds() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let mut bytes = image.bytes().to_vec();
        let record = usize::try_from(header.summaries.first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + format::RECORD_HEADER;
        bytes[record + 24..record + 32].copy_from_slice(&999_u64.to_le_bytes());
        bytes[record + 32..record + 40].copy_from_slice(&999_u64.to_le_bytes());
        rehash(&mut bytes, &image.receipt);
        assert!(matches!(
            SegmentedImage::certify(bytes),
            Err(SegmentedError::Corrupt(_))
        ));
    }

    #[test]
    fn recomputed_hashes_cannot_certify_foreign_quad_references() {
        let image = image();
        let header = Header::decode(image.bytes()).unwrap();
        let mut bytes = image.bytes().to_vec();
        let record = usize::try_from(header.streams[0].first).unwrap()
            * usize::try_from(header.block_bytes).unwrap()
            + format::RECORD_HEADER;
        bytes[record..record + 8].copy_from_slice(&999_u64.to_le_bytes());
        rehash(&mut bytes, &image.receipt);
        assert!(matches!(
            SegmentedImage::certify(bytes),
            Err(SegmentedError::Corrupt(_))
        ));
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
