// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Content-derived source rows, independently of dictionary ordinals or sizing
//! statistics. Row hashes preserve blank scopes and every literal coordinate.

use purrdf_hash::multiset::Multiset256;
use purrdf_hash::{Domain, blake3::Hasher, frame::frame_le_with};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

use crate::ir::term::CanonicalNode;
use crate::{RdfTextDirection, TermValue};

const ROW: Domain = Domain::new(b"purrdf/source-row/v1");
const TRIPLE_ROW: Domain = Domain::new(b"purrdf/triples/source-row/v1");
const RECORD: Domain = Domain::new(b"purrdf/source-record/v1");
const SOURCE_VERSION: Domain = Domain::new(b"purrdf/source-version/v1");

pub(crate) fn resident_snapshot(
    records: Multiset256,
    graphs: Multiset256,
    dictionary: [u8; 32],
) -> purrdf_hash::hex::Digest32 {
    const ROWS: Domain = Domain::new(b"purrdf-core/snapshot/rows/v1");
    const GRAPHS: Domain = Domain::new(b"purrdf-core/snapshot/graphs/v1");
    let mut hash = purrdf_hash::blake3::RecordHasher::new();
    hash.update(&records.fingerprint(ROWS));
    hash.update(&graphs.fingerprint(GRAPHS));
    hash.update(&dictionary);
    purrdf_hash::hex::Digest32::new(*hash.finalize().as_bytes())
}

pub(crate) fn record_hash(relation: crate::ir::statistics::Relation, row: [u8; 32]) -> [u8; 32] {
    use crate::ir::statistics::Relation;
    let mut record = Hasher::new();
    frame_le_with(RECORD.as_bytes(), &mut |bytes| {
        record.update(bytes);
    });
    record.update(&[match relation {
        Relation::Asserted => 0,
        Relation::Reifiers => 1,
        Relation::Annotations => 2,
        Relation::Targets => 3,
    }]);
    record.update(&row);
    *record.finalize().as_bytes()
}

/// Borrowed statement values supplied by a source owner. No field outlives its
/// callback; the owner retains the original lexical storage and read admission.
#[derive(Clone, Copy, Debug)]
pub struct SourceRowRef<'a> {
    /// Subject value.
    pub subject: &'a TermValue,
    /// Predicate value.
    pub predicate: &'a TermValue,
    /// Object value, including nonliteral objects and quoted triples.
    pub object: &'a TermValue,
    /// Named graph, or the default graph.
    pub graph: Option<&'a TermValue>,
}

impl SourceRowRef<'_> {
    pub(crate) fn hashes<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<[[u8; 32]; 2], StorageError> {
        let mut row = SourceRowHasher::with_graph(self.graph, memory)?;
        for term in [self.subject, self.predicate, self.object] {
            term.visit_canonical_bytes_with_memory(|bytes| row.term(bytes), memory)?;
        }
        Ok(row.finish())
    }
}

/// Which original immutable carrier owns a changed row's local IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceSide {
    /// A physical record absent from the successor.
    Previous,
    /// A physical record absent from the predecessor.
    Next,
}

/// The two independently visible families needed for source accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFamily {
    /// The union of asserted and annotation rows, regardless of classification.
    Statements,
    /// Virtual reifier declarations, independently of the statement union.
    Reifiers,
}

/// A source-owned immutable observation. Cloning retains its original admitted
/// owner rather than copying historical heads or allocating a second directory.
#[derive(Clone)]
pub struct SourceVersion(
    pub(crate) purrdf_lex::allocation::shared::Shared<crate::ir::segmented::SegmentedSourceVersion>,
);
purrdf_hash::debug_non_exhaustive!(SourceVersion {});

impl SourceVersion {
    /// Exact effective statement-union plus reifier population.
    #[must_use]
    pub fn statement_count(&self) -> u128 {
        self.0.records.population()
    }

    /// Content identity of the complete statement union and reifier families,
    /// independent of dictionary IDs, physical segmentation or classification.
    /// This is a fingerprint, not a signature or a membership proof.
    #[must_use]
    pub fn content_fingerprint(&self) -> [u8; 32] {
        self.0.records.fingerprint(SOURCE_VERSION)
    }
}

/// Deterministic work in an independently observed source transition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceChangeWork {
    /// Native resident-directory work, excluding callback and point-read work.
    pub directory: SourceDifferenceWork,
    /// Distinct value-quads considered by the original visibility engine.
    pub candidates: u64,
    /// Effective family records emitted to the consumer.
    pub emitted: u64,
    /// Physical records offered before value-quad deduplication.
    pub physical: u64,
    /// Immutable descriptor comparisons used for source alignment.
    pub descriptors: u64,
}

/// Result of visiting independently discovered effective source changes.
#[derive(Debug)]
pub enum SourceVisit<B> {
    /// This provider has no independently maintained source-transition seam.
    Unavailable,
    /// The consumer stopped; no successor observation is published.
    Stopped(B),
    /// The complete transition, with the successor's source-owned observation.
    Complete {
        /// New immutable source observation, after final readiness checks.
        version: SourceVersion,
        /// Exact deterministic discovery and emission work.
        work: SourceChangeWork,
    },
}

/// Work in a native content-directory comparison. Callback work and source
/// reads are accounted separately by their original owners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceDifferenceWork {
    /// Digest-prefix ranges compared or emitted.
    pub ranges: u64,
    /// Stored digests visited while reading sparse prefix-sum tails.
    pub members: u64,
    /// Physical records passed to the consumer.
    pub changed: u64,
}

/// Exact content accounting for one predicate, graph selection and physical
/// statement stream. This is content evidence, distinct from sizing estimates
/// or a statistics cache discriminator. It is not a membership proof.
///
/// Chronological unions must correct overlap and removals before publishing a
/// summary; adding their physical layer summaries alone is insufficient.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SourceSummary {
    pub(crate) literals: [Multiset256; 2],
    pub(crate) records: Multiset256,
}

impl SourceSummary {
    /// Exact statement population, including nonliteral objects.
    #[must_use]
    pub const fn statement_count(self) -> u128 {
        self.records.population()
    }

    /// Literal-source sums in graph-aware, graph-free order. Occurrences in
    /// distinct physical streams contribute separately when combined.
    #[must_use]
    pub const fn literal_rows(self) -> [Multiset256; 2] {
        self.literals
    }

    /// Physical record identity, including the statement stream's kind. Named
    /// graph declarations without statements do not enter this multiset.
    #[must_use]
    pub const fn records(self) -> Multiset256 {
        self.records
    }
}

/// Borrowed coordinates of a literal-bearing statement. The physical asserted
/// or annotation layer does not enter its identity; each occurrence contributes
/// separately to the source multiset.
#[derive(Debug)]
pub struct LiteralSourceRow<'a> {
    /// Named graph, or the default graph.
    pub graph: Option<&'a TermValue>,
    /// Statement subject, including a structured triple term when present.
    pub subject: &'a TermValue,
    /// Statement predicate.
    pub predicate: &'a TermValue,
    /// Verbatim literal lexical form.
    pub lexical: &'a str,
    /// Expanded datatype IRI.
    pub datatype: &'a str,
    /// Canonical language tag, when present.
    pub language: Option<&'a str>,
    /// RDF 1.2 base direction, when present.
    pub direction: Option<RdfTextDirection>,
}

impl LiteralSourceRow<'_> {
    /// Hash both the graph-aware and graph-free projections, in that order.
    /// Triple terms stream their full canonical encoding; literal text is
    /// borrowed directly and is never copied into a temporary owned term.
    ///
    /// # Errors
    /// Returns original traversal-storage refusal. No digest is returned for a
    /// partially encoded row.
    pub fn hashes_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<[[u8; 32]; 2], StorageError> {
        let mut row = SourceRowHasher::with_graph(self.graph, memory)?;
        for term in [self.subject, self.predicate] {
            term.visit_canonical_bytes_with_memory(|bytes| row.term(bytes), memory)?;
        }
        CanonicalNode::Literal {
            lexical: self.lexical,
            datatype: self.datatype,
            language: self.language,
            direction: self.direction,
        }
        .visit_bytes(&mut |bytes| row.term(bytes));
        Ok(row.finish())
    }
}

/// Both projections consume one canonical traversal. Only graph bytes go to
/// the graph-aware state. Owned and native ID-based callers use this same law.
pub(crate) struct SourceRowHasher {
    graph: Hasher,
    triple: Hasher,
}

impl SourceRowHasher {
    fn with_graph<S: Admission + ?Sized>(
        graph: Option<&TermValue>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let mut row = Self::new(graph.is_some());
        if let Some(graph) = graph {
            graph.visit_canonical_bytes_with_memory(|bytes| row.graph(bytes), memory)?;
        }
        Ok(row)
    }

    pub(crate) fn new(named_graph: bool) -> Self {
        let mut graph = Hasher::new();
        frame_le_with(ROW.as_bytes(), &mut |bytes| {
            graph.update(bytes);
        });
        graph.update(&[u8::from(named_graph)]);
        let mut triple = Hasher::new();
        frame_le_with(TRIPLE_ROW.as_bytes(), &mut |bytes| {
            triple.update(bytes);
        });
        Self { graph, triple }
    }

    pub(crate) fn graph(&mut self, bytes: &[u8]) {
        self.graph.update(bytes);
    }

    pub(crate) fn term(&mut self, bytes: &[u8]) {
        self.graph.update(bytes);
        self.triple.update(bytes);
    }

    pub(crate) fn finish(self) -> [[u8; 32]; 2] {
        [
            *self.graph.finalize().as_bytes(),
            *self.triple.finalize().as_bytes(),
        ]
    }
}
