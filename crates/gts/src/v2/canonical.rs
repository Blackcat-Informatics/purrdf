// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![expect(
    clippy::result_large_err,
    reason = "preserve native replay causes without allocating on the refusal path"
)]

use core::fmt;
use std::collections::TryReserveError;
use std::io::{self, Write};

use purrdf_core::canonical_labelling::{
    Direction, FoldLabelling, FoldLine, FoldRecord, FoldTerm, Kind, LabellingCapacity, Limit,
    LineSet, LoweringRefusal, Placement, Refusal, StatisticsRecord, ViewError, label_fold,
};
use purrdf_iri::vocab::rdf;
use purrdf_lex::cbor::head::{self, write_bytes, write_text};

use crate::FastMap;

mod check_labels;
mod check_structure;
mod report;
pub use check_labels::{
    CanonicalLabellingCheck, CanonicalLabellingDifference, CanonicalLabellingUnchecked,
};
pub(super) use check_structure::PackRegion;
pub use check_structure::{CanonicalStructureCheck, CanonicalStructureDifference};
pub use report::{
    CanonicalDifference, CanonicalDifferenceReason, CanonicalStorageCheck, SegmentCanonicalCheck,
};

use super::fields::Fields;
use super::read::OwnSegment;
use super::references::{LeafNode, write_blob_name, write_referent};
use super::{
    Binding, BlobEntry, Line, LiteralDirection, LiteralValue, RdfFold, RdfGraph, ReadError,
    ResolvedLeaf, Suppression, Term, TermId,
};

/// Canonical lines and positions for a GTS file fold.
#[derive(Debug)]
#[non_exhaustive]
pub struct ContentLabelling {
    /// The admitted set and its scoped blank and nested-term source identifiers.
    pub lines: LineSet,
    canonical_lines: Vec<Box<[u8]>>,
    placements: Vec<Placement>,
    statistics: StatisticsRecord,
    /// The content digest; absent when the fold contains a held row or entry.
    pub content_digest: Option<[u8; 32]>,
}

/// A fold exceeded a format capacity or the native labeller refused it.
#[derive(Debug)]
#[non_exhaustive]
pub enum ContentLabellingRefusal {
    /// Size, weight, or a nested-term identity collision.
    Lowering(LoweringRefusal),
    /// Width or engine work limit.
    Engine(Refusal),
}

impl fmt::Display for ContentLabellingRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lowering(cause) => cause.fmt(f),
            Self::Engine(refusal) => {
                write!(
                    f,
                    "the GTS labelling refused the fold (code {})",
                    refusal.code()
                )
            }
        }
    }
}

purrdf_hash::error_sources!(ContentLabellingRefusal, Lowering);

impl purrdf_hash::error::StructuredError for ContentLabellingRefusal {
    purrdf_hash::error_visit! {
        Lowering(cause) => "gts-v2-content-lowering-refusal" { "cause" => Cause(cause) },
        Engine(cause) => "gts-v2-content-engine-refusal" { "cause" => Cause(cause) },
    }
}

/// Digest, index comparisons, and canonical checks of a segment.
#[derive(Debug)]
#[non_exhaustive]
pub struct SegmentContentLabelling {
    /// The segment's ordinal in the input file.
    pub segment: u64,
    /// The digest or its labelling refusal. The digest is absent when the
    /// segment has no fold or its fold has a held item.
    pub content_digest: Result<Option<[u8; 32]>, ContentLabellingRefusal>,
    /// Digest and statistics comparison for the eligible closing index.
    pub closing_index: Option<ClosingIndexLabelling>,
    /// The canonical form check on this own fold.
    pub canonical: SegmentCanonicalCheck,
}

/// The closing index's comparisons against its segment's own fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClosingIndexLabelling {
    /// File-relative byte offset of the index.
    pub offset: usize,
    /// Counts and predicate row totals match the own fold and its written rows.
    pub statistics_match: bool,
    /// The claimed optional digest matches the own fold.
    pub content_digest_matches: bool,
}

/// The canonical results of the segment-own folds and the file fold.
#[derive(Debug)]
#[non_exhaustive]
pub struct ReadContentLabelling {
    /// Observed segments in file order.
    pub segments: Vec<SegmentContentLabelling>,
    /// Labelling of the file fold, absent if the read produced no fold.
    pub file: Result<Option<ContentLabelling>, ContentLabellingRefusal>,
}

impl SegmentContentLabelling {
    const fn without_fold(segment: u64, claimed: bool) -> Self {
        Self {
            segment,
            content_digest: Ok(None),
            closing_index: None,
            canonical: SegmentCanonicalCheck::without_fold(claimed),
        }
    }
}

/// A labelling operation could not return a result for the read.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadLabellingError {
    /// Isolating a readable segment failed.
    Segment(ReadError),
    /// The labeller could not read or store the folded input.
    Labeller(ViewError<io::Error>),
    /// Comparing canonical labels could not complete.
    CanonicalLabels(io::Error),
    /// Comparing the canonical structure could not complete.
    CanonicalStructure(io::Error),
    /// The report could not reserve storage for the segment results.
    Resource(TryReserveError),
    /// The input had no fold to label.
    MissingFold,
}

impl fmt::Display for ReadLabellingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Segment(error) => error.fmt(f),
            Self::Labeller(error) => error.fmt(f),
            Self::CanonicalLabels(error) => error.fmt(f),
            Self::CanonicalStructure(error) => error.fmt(f),
            Self::Resource(error) => {
                write!(f, "could not reserve segment labelling results ({error})")
            }
            Self::MissingFold => f.write_str("the GTS read has no fold to label"),
        }
    }
}

impl std::error::Error for ReadLabellingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Segment(error) => Some(error),
            Self::Labeller(error) => Some(error),
            Self::CanonicalLabels(error) => Some(error),
            Self::CanonicalStructure(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::MissingFold => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ReadLabellingError {
    purrdf_hash::error_visit! {
        Segment(cause) => "gts-v2-read-labelling-segment" { "cause" => Cause(cause) },
        Labeller(cause) => "gts-v2-read-labelling-engine" { "cause" => Cause(cause) },
        CanonicalLabels(cause) => "gts-v2-canonical-labels-check" { "cause" => Cause(cause) },
        CanonicalStructure(cause) => "gts-v2-canonical-structure-check" { "cause" => Cause(cause) },
        Resource(cause) => "gts-v2-read-labelling-resource" { "cause" => Cause(cause) },
        MissingFold => "gts-v2-read-labelling-missing-fold" {},
    }
}

impl ContentLabelling {
    /// Encoded canonical lines in delivery order.
    #[must_use]
    pub fn canonical_lines(&self) -> &[Box<[u8]>] {
        &self.canonical_lines
    }

    /// Placements by node index in the admitted set.
    #[must_use]
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// The structural statistics of the labelling.
    #[must_use]
    pub const fn statistics(&self) -> &StatisticsRecord {
        &self.statistics
    }

    pub(super) fn index_counts(&self) -> [u64; 3] {
        let rdf_lines = self
            .canonical_lines
            .iter()
            .filter(|line| line.first() != Some(&(Kind::Record as u8)))
            .count();
        let nodes = self.lines.blank_nodes();
        let blank_nodes = (0..nodes)
            .filter(|&index| self.lines.source(index).first() == Some(&0x82))
            .count();
        // Every lowered term node has exactly one nested-term record.
        let records = self.canonical_lines.len() - rdf_lines - (nodes - blank_nodes);
        [rdf_lines as u64, records as u64, blank_nodes as u64]
    }

    pub(super) fn blank_positions(&self) -> io::Result<FastMap<(u64, &[u8]), u64>> {
        let mut result = FastMap::default();
        for (node, placement) in self.placements.iter().enumerate() {
            let source = self.lines.source(node);
            // Blank sources are CBOR pairs. Term-node sources start with an array of one.
            if source.first() == Some(&0x82) {
                let mut fields = Fields::new(source);
                let (ordinal, label) = (|| {
                    fields.record(2)?;
                    let ordinal = fields.uint()?;
                    let label = fields.bytes()?;
                    fields.finish()?;
                    Ok::<_, super::WireError>((ordinal, label))
                })()
                .map_err(io::Error::other)?;
                result.try_reserve(1).map_err(io::Error::other)?;
                result.insert((ordinal, label), placement.position);
            }
        }
        Ok(result)
    }
}

impl<'a> RdfGraph<'a> {
    /// Label the segments' own folds in file order, then label the file fold.
    ///
    /// Each own fold resolves terms and references within its original
    /// segment only. It uses ordinal zero for blank sources. All folds share
    /// the weight account. If the item budget stopped the read, reports the
    /// observed segments with no digests and leaves the account untouched.
    /// Use the original read result.
    ///
    /// # Errors
    /// Returns a replay, storage, or non-refusal labelling failure.
    pub fn label_read(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
    ) -> Result<ReadContentLabelling, ReadLabellingError> {
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(self.segment_durability.len() + self.skipped_segments.len())
            .map_err(ReadLabellingError::Resource)?;
        let file = if self.labelling_size().is_some() {
            self.label_read_folds(limit, capacity, &mut segments)?
                .map(Some)
        } else {
            let mut claims = self.claimed_segments.iter().copied().peekable();
            segments.extend(self.segment_durability.iter().map(|segment| {
                let claimed = claims.next_if_eq(&segment.segment).is_some();
                SegmentContentLabelling::without_fold(segment.segment, claimed)
            }));
            Ok(None)
        };
        if !self.skipped_segments.is_empty() {
            segments.extend(
                self.skipped_segments
                    .iter()
                    .map(|segment| SegmentContentLabelling::without_fold(segment.ordinal, false)),
            );
            segments.sort_unstable_by_key(|segment| segment.segment);
        }
        Ok(ReadContentLabelling { segments, file })
    }

    fn label_read_folds(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
        segments: &mut Vec<SegmentContentLabelling>,
    ) -> Result<Result<ContentLabelling, ContentLabellingRefusal>, ReadLabellingError> {
        self.for_labelled_segments(limit, capacity, None, |segment, fold, labelled| {
            let closing_index = labelled.as_ref().ok().and_then(|result| {
                let (offset, check) = segment.closing_index(self)?;
                let (statistics_match, content_digest_matches) =
                    check.compare(result.index_counts(), result.content_digest);
                Some(ClosingIndexLabelling {
                    offset,
                    statistics_match,
                    content_digest_matches,
                })
            });
            segments.push(SegmentContentLabelling {
                segment: segment.ordinal,
                canonical: SegmentCanonicalCheck::from_levels(
                    fold,
                    check_structure::check_structure(
                        fold,
                        labelled.as_ref().ok(),
                        segment.input(self).ok_or(ReadLabellingError::MissingFold)?,
                    )
                    .map_err(ReadLabellingError::CanonicalStructure)?,
                    check_labels::check(fold, labelled.as_ref().ok())
                        .map_err(ReadLabellingError::CanonicalLabels)?,
                ),
                content_digest: labelled.map(|result| result.content_digest),
                closing_index,
            });
            Ok(())
        })?;
        read_labelling(self.label_content(limit, capacity))
    }

    pub(super) fn label_for_compaction(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
    ) -> Result<Result<ContentLabelling, ContentLabellingRefusal>, ReadLabellingError> {
        self.for_labelled_segments(limit, capacity, None, |_, _, _| Ok(()))?;
        read_labelling(self.label_content(limit, capacity))
    }

    pub(super) fn label_segment_for_compaction(
        &self,
        ordinal: u64,
        fold: &Self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
    ) -> Result<Result<ContentLabelling, ContentLabellingRefusal>, ReadLabellingError> {
        self.for_labelled_segments(limit, capacity, Some(ordinal), |_, _, _| Ok(()))?;
        read_labelling(fold.label_scoped(limit, capacity, Some(0)))
    }

    fn for_labelled_segments(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
        before: Option<u64>,
        mut visit: impl FnMut(
            &OwnSegment<'a>,
            &Self,
            Result<ContentLabelling, ContentLabellingRefusal>,
        ) -> Result<(), ReadLabellingError>,
    ) -> Result<(), ReadLabellingError> {
        for segment in self
            .own_segments()
            .take_while(|segment| before.is_none_or(|end| segment.ordinal < end))
        {
            let fold = segment.fold(self).map_err(ReadLabellingError::Segment)?;
            let labelled = read_labelling(fold.label_scoped(limit, capacity, Some(0)))?;
            visit(segment, &fold, labelled)?;
        }
        Ok(())
    }

    /// Label the file fold, including the content hidden by suppression.
    /// An item-capacity refusal withholds the fold and returns `None`.
    /// A held row, binding or entry withholds the digest while the readable
    /// lines and records still participate in labelling.
    /// The size and held-item findings describe the read. Use this method
    /// with the original fold, before changing its public contents.
    ///
    /// # Errors
    /// Returns the first labelling refusal or a storage or encoding failure.
    pub fn label_content(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
    ) -> Result<Option<ContentLabelling>, ViewError<io::Error>> {
        self.label_scoped(limit, capacity, None)
    }

    fn label_scoped(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
        ordinal: Option<u64>,
    ) -> Result<Option<ContentLabelling>, ViewError<io::Error>> {
        let Some(size) = self.labelling_size() else {
            return Ok(None);
        };
        let records = self
            .references
            .bindings
            .iter()
            .map(Record::Binding)
            .chain(
                self.references
                    .suppressions
                    .iter()
                    .map(|claim| Record::Suppression(&claim.entry)),
            )
            .chain(self.blob_set.iter().map(Record::Blob))
            .chain(
                self.rdf
                    .values()
                    .iter()
                    .map(|(literal, value)| Record::Value(*literal, value)),
            );
        let limits = FoldLabelling {
            size,
            annotation_width: self.rdf.annotation_width_exceeded(),
            limit,
        };
        let mut result = label_rdf(&self.rdf, records, limits, capacity, |_, segment, label| {
            (ordinal.unwrap_or(segment), label)
        })?;
        let held = self.rdf.has_held_rows()
            || self
                .reference_findings
                .iter()
                .any(|finding| finding.opaque_term);
        if held {
            result.content_digest = None;
        }
        Ok(Some(result))
    }
}

pub(super) fn label_written_rdf(
    fold: &RdfFold,
    limit: Limit,
    capacity: &mut LabellingCapacity,
) -> Result<ContentLabelling, ViewError<io::Error>> {
    let mut labels = Vec::new();
    let mut size = fold.lines().count() as u128 + fold.values().len() as u128;
    for (index, term) in fold.terms().enumerate() {
        match term {
            Term::Blank { .. } => {
                labels
                    .try_reserve(1)
                    .map_err(io::Error::other)
                    .map_err(ViewError::Read)?;
                labels.push((index, (index as u64).to_be_bytes()));
            }
            Term::Triple([_, _, object]) if matches!(fold.term(*object), Some(Term::Triple(_))) => {
                size += 1;
            }
            _ => {}
        }
    }
    label_rdf(
        fold,
        fold.values()
            .iter()
            .map(|(id, value)| Record::Value(*id, value)),
        FoldLabelling {
            size,
            annotation_width: fold.annotation_width_exceeded(),
            limit,
        },
        capacity,
        |index, _, _| {
            let position = labels
                .binary_search_by_key(&index, |(id, _)| *id)
                .expect("every written blank term was assigned a label");
            (0, &labels[position].1)
        },
    )
}

fn label_rdf<'a>(
    fold: &'a RdfFold,
    records: impl Iterator<Item = Record<'a>> + Clone,
    limits: FoldLabelling,
    capacity: &mut LabellingCapacity,
    blank: impl Fn(usize, u64, &'a [u8]) -> (u64, &'a [u8]),
) -> Result<ContentLabelling, ViewError<io::Error>> {
    let (lines, labelled) = label_fold(
        fold.lines().map(fold_line),
        records,
        |index| fold_term(fold, index, &blank),
        limits,
        capacity,
    )?;
    Ok(ContentLabelling {
        lines,
        canonical_lines: labelled.lines,
        placements: labelled.placements,
        statistics: labelled.record,
        content_digest: Some(labelled.digest),
    })
}

fn read_labelling(
    labelled: Result<Option<ContentLabelling>, ViewError<io::Error>>,
) -> Result<Result<ContentLabelling, ContentLabellingRefusal>, ReadLabellingError> {
    match labelled {
        Ok(Some(result)) => Ok(Ok(result)),
        Ok(None) => Err(ReadLabellingError::MissingFold),
        Err(ViewError::Lowering(refusal)) => Ok(Err(ContentLabellingRefusal::Lowering(refusal))),
        Err(ViewError::Refused(refusal @ (Refusal::Width { .. } | Refusal::Limit { .. }))) => {
            Ok(Err(ContentLabellingRefusal::Engine(refusal)))
        }
        Err(error) => Err(ReadLabellingError::Labeller(error)),
    }
}

fn fold_line(line: Line) -> FoldLine {
    match line {
        Line::Quad(terms, graph) => {
            FoldLine::Quad(terms.map(TermId::index), graph.map(TermId::index))
        }
        Line::ReifierBinding(terms, graph) => {
            FoldLine::ReifierBinding(terms.map(TermId::index), graph.map(TermId::index))
        }
        Line::Annotation(terms, count, graph) => FoldLine::Annotation(
            terms.map(TermId::index),
            count.get(),
            graph.map(TermId::index),
        ),
        Line::EmptyGraph(graph) => FoldLine::EmptyGraph(graph.index()),
    }
}

fn fold_term<'a>(
    fold: &'a RdfFold,
    index: usize,
    blank: impl Fn(usize, u64, &'a [u8]) -> (u64, &'a [u8]),
) -> Option<FoldTerm<'a>> {
    Some(match fold.term(TermId::from_index(index))? {
        Term::Iri(iri) => FoldTerm::Iri(iri),
        Term::Blank { segment, label } => {
            let (ordinal, label) = blank(index, *segment, label);
            FoldTerm::Blank { ordinal, label }
        }
        Term::Typed { lexical, datatype } => {
            let Term::Iri(datatype) = fold.term(*datatype)? else {
                return None;
            };
            FoldTerm::Literal {
                lexical,
                datatype,
                language: None,
                direction: None,
            }
        }
        Term::Language {
            lexical,
            language,
            direction,
        } => {
            let direction = direction.map(|direction| match direction {
                LiteralDirection::LeftToRight => Direction::Ltr,
                LiteralDirection::RightToLeft => Direction::Rtl,
            });
            let datatype = if direction.is_some() {
                rdf::DIR_LANG_STRING
            } else {
                rdf::LANG_STRING
            };
            FoldTerm::Literal {
                lexical,
                datatype,
                language: Some(language),
                direction,
            }
        }
        Term::Triple(terms) => FoldTerm::Triple(terms.map(TermId::index)),
    })
}

enum Record<'a> {
    Binding(&'a Binding<ResolvedLeaf>),
    Suppression(&'a Suppression<ResolvedLeaf>),
    Blob(&'a BlobEntry<'a>),
    Value(TermId, &'a LiteralValue),
}

type RecordError = ViewError<io::Error>;

fn wire<T>(result: io::Result<T>) -> Result<T, RecordError> {
    result.map_err(ViewError::Read)
}

impl FoldRecord for Record<'_> {
    type Error = io::Error;

    fn encode(
        &self,
        payload: &mut dyn Write,
        mut term: impl FnMut(usize) -> Result<(), RecordError>,
    ) -> Result<(), RecordError> {
        match self {
            Self::Binding(binding) => {
                wire(payload.write_all(&[0x82, 1, 0x82]))?;
                binding_slots(payload, binding, &mut term)
            }
            Self::Suppression(entry) => {
                wire(payload.write_all(&[0x82, 2]))?;
                suppression(payload, entry, &mut term)
            }
            Self::Blob(blob) => {
                let presence = if blob.present { 1 } else { 2 };
                wire(payload.write_all(&[0x82, 3, 0x84, presence]))?;
                wire(write_blob_name(payload, &blob.name.digest))?;
                wire(head::write_head(payload, head::UNSIGNED, blob.name.length))?;
                wire(head::write_head(
                    payload,
                    head::ARRAY,
                    blob.media_types.len() as u64,
                ))?;
                for media in &blob.media_types {
                    wire(write_text(payload, media))?;
                }
                Ok(())
            }
            Self::Value(literal, value) => {
                let kind = match value {
                    LiteralValue::Boolean(_) => 1,
                    LiteralValue::Unsigned(_) => 2,
                    LiteralValue::Integer(_) => 3,
                    LiteralValue::Time(..) => 4,
                    LiteralValue::Octets(_) => 5,
                };
                wire(payload.write_all(&[0x82, 4, 0x82, kind, 0x81, 1]))?;
                term(literal.index())
            }
        }
    }
}

fn binding_slots(
    output: &mut dyn Write,
    binding: &Binding<ResolvedLeaf>,
    term: &mut impl FnMut(usize) -> Result<(), RecordError>,
) -> Result<(), RecordError> {
    slot(output, &binding.term, term)?;
    wire(write_referent(output, &binding.referent))
}

fn suppression(
    output: &mut dyn Write,
    entry: &Suppression<ResolvedLeaf>,
    term: &mut impl FnMut(usize) -> Result<(), RecordError>,
) -> Result<(), RecordError> {
    let kind = entry.kind() as u8;
    match entry {
        Suppression::Term(leaf) | Suppression::Reifier(leaf) | Suppression::Graph(leaf) => {
            wire(output.write_all(&[0x82, kind]))?;
            slot(output, leaf, term)
        }
        Suppression::Quad(terms, graph) | Suppression::Annotation(terms, graph) => {
            wire(output.write_all(&[0x85, kind]))?;
            statement_slots(output, terms, graph.as_ref(), term)
        }
        Suppression::ReifierBinding(terms, graph) => {
            wire(output.write_all(&[0x84, kind]))?;
            statement_slots(output, terms, graph.as_ref(), term)
        }
        Suppression::Blob(digest) => {
            wire(output.write_all(&[0x82, kind]))?;
            wire(write_blob_name(output, digest))
        }
        Suppression::Binding(binding) => {
            wire(output.write_all(&[0x83, kind]))?;
            binding_slots(output, binding, term)
        }
    }
}

fn statement_slots(
    output: &mut dyn Write,
    terms: &[ResolvedLeaf],
    graph: Option<&ResolvedLeaf>,
    term: &mut impl FnMut(usize) -> Result<(), RecordError>,
) -> Result<(), RecordError> {
    for leaf in terms {
        slot(output, leaf, term)?;
    }
    if let Some(graph) = graph {
        wire(output.write_all(&[0x82, 2]))?;
        slot(output, graph, term)
    } else {
        wire(output.write_all(&[0x81, 1]))
    }
}

fn slot(
    output: &mut dyn Write,
    leaf: &ResolvedLeaf,
    term: &mut impl FnMut(usize) -> Result<(), RecordError>,
) -> Result<(), RecordError> {
    leaf.visit(
        |node| match node {
            LeafNode::Term(id) => {
                wire(output.write_all(&[0x81, 1]))?;
                term(id.index())
            }
            LeafNode::Blank { frame, label } => {
                wire(output.write_all(&[0x83, 2]))?;
                wire(write_bytes(output, frame))?;
                wire(write_bytes(output, label))
            }
            LeafNode::Triple => wire(output.write_all(&[0x84, 3])),
        },
        |error| ViewError::Read(io::Error::other(error)),
    )
}
