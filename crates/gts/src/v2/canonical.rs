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
    Direction, FoldLabelling, FoldLine, FoldRecord, FoldTerm, Kind, Labelled, LabellingCapacity,
    Limit, LineSet, LoweringRefusal, Placement, Refusal, StatisticsRecord, ViewError, label_fold,
};
use purrdf_iri::vocab::rdf;
use purrdf_lex::cbor::head::{self, write_bytes, write_text};

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

/// Digest and labelling refusal for a readable segment's own fold.
#[derive(Debug)]
#[non_exhaustive]
pub struct SegmentContentLabelling {
    /// The segment's ordinal in the input file.
    pub segment: u64,
    /// The digest, absent for a fold with a held item, or its refusal.
    pub content_digest: Result<Option<[u8; 32]>, ContentLabellingRefusal>,
    /// Digest and statistics comparison for the eligible closing index.
    pub closing_index: Option<ClosingIndexLabelling>,
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
    /// Readable segments in file order. Unreadable segments have no digest.
    pub segments: Vec<SegmentContentLabelling>,
    /// Canonical results of the file fold, or its labelling refusal.
    pub file: Result<ContentLabelling, ContentLabellingRefusal>,
}

/// A labelling operation could not return a result for the read.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadLabellingError {
    /// Isolating a readable segment failed.
    Segment(ReadError),
    /// The labeller could not read or store the folded input.
    Labeller(ViewError<io::Error>),
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
            Self::Resource(error) => Some(error),
            Self::MissingFold => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ReadLabellingError {
    purrdf_hash::error_visit! {
        Segment(cause) => "gts-v2-read-labelling-segment" { "cause" => Cause(cause) },
        Labeller(cause) => "gts-v2-read-labelling-engine" { "cause" => Cause(cause) },
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

    fn index_counts(&self) -> [u64; 3] {
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
}

impl RdfGraph<'_> {
    /// Label the segments' own folds in file order, then label the file fold.
    ///
    /// Each own fold resolves terms and references within its original
    /// segment only. It uses ordinal zero for blank sources. All folds share
    /// the weight account. If the item budget stopped the read, returns
    /// `None` and leaves the account untouched. Use the original read result.
    ///
    /// # Errors
    /// Returns a replay, storage, or non-refusal labelling failure.
    pub fn label_read(
        &self,
        limit: Limit,
        capacity: &mut LabellingCapacity,
    ) -> Result<Option<ReadContentLabelling>, ReadLabellingError> {
        if self.labelling_size().is_none() {
            return Ok(None);
        }
        let own = self.own_segments();
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(own.len())
            .map_err(ReadLabellingError::Resource)?;
        for segment in own {
            let fold = segment.fold(self).map_err(ReadLabellingError::Segment)?;
            let labelled = read_labelling(fold.label_scoped(limit, capacity, Some(0)))?;
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
                content_digest: labelled.map(|result| result.content_digest),
                closing_index,
            });
        }
        let file = read_labelling(self.label_content(limit, capacity))?;
        Ok(Some(ReadContentLabelling { segments, file }))
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
        let (lines, labelled) = label_fold(
            self.rdf.lines().map(fold_line),
            records,
            |index| fold_term(&self.rdf, index, ordinal),
            limits,
            capacity,
        )?;
        let held = self.rdf.has_held_rows()
            || self
                .reference_findings
                .iter()
                .any(|finding| finding.opaque_term);
        let Labelled {
            lines: canonical_lines,
            placements,
            record: statistics,
            digest,
            ..
        } = labelled;
        Ok(Some(ContentLabelling {
            lines,
            canonical_lines,
            placements,
            statistics,
            content_digest: (!held).then_some(digest),
        }))
    }
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

fn fold_term(fold: &RdfFold, index: usize, ordinal: Option<u64>) -> Option<FoldTerm<'_>> {
    Some(match fold.term(TermId::from_index(index))? {
        Term::Iri(iri) => FoldTerm::Iri(iri),
        Term::Blank { segment, label } => FoldTerm::Blank {
            ordinal: ordinal.unwrap_or(*segment),
            label,
        },
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
