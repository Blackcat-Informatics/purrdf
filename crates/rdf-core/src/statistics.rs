// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Snapshot-bound statistics shared by query and rule planning.
//!
//! These describe effective RDF rows, never a resource charge or a guaranteed
//! output bound. Unknown counts remain absent. Providers retain the same local
//! identity space as their DatasetView and do not translate IDs numerically.

use crate::{GraphMatch, ProbePattern, RdfTermKind, ViewTermId};

/// The evidence behind one statistical count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatisticsAccuracy {
    /// Counted exactly over this snapshot's effective rows or distinct domain.
    Exact,
    /// A conservative upper bound; it must not be presented as exact.
    UpperBound,
}

/// One count together with the strength of its evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatisticCount {
    /// Number of rows or distinct values, according to the containing field.
    pub value: u64,
    /// Whether this value is exact or only an upper bound.
    pub accuracy: StatisticsAccuracy,
}

/// A physical RDF stream. Its counts are kept separate until a consumer asks
/// for the complete virtual RDF surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatisticsStream {
    /// Asserted triples or quads.
    Asserted,
    /// Virtual reifier declarations, with rdf:reifies as their predicate.
    Reifiers,
    /// Statement annotations.
    Annotations,
}

/// The distinct identity domain requested from immutable statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticsPosition {
    /// Subject identities.
    Subject,
    /// Object identities.
    Object,
}

/// The domain of rows under one predicate and graph restriction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PredicateStatistics {
    /// Matching rows, retaining RDF graph multiplicity.
    pub rows: StatisticCount,
    /// Distinct subject IDs, not subject occurrences. None means unknown.
    pub subjects: Option<StatisticCount>,
    /// Distinct object IDs, not object occurrences. None means unknown.
    pub objects: Option<StatisticCount>,
}

/// An immutable provider in the local identity space of its owning read view.
///
/// A returned zero is evidence of absence; None is not. Providers must describe
/// effective rows after snapshot layering, including retractions. Add row counts
/// only over disjoint effective streams. Distinct counts require sorted-ID union
/// in the same dictionary generation; adding their sizes is not an exact merge.
/// An operational storage failure remains on the owning view's first-cause path.
pub trait SnapshotStatistics<I: ViewTermId>: Sync {
    /// Caller-supplied or native provider label, recorded with consulted evidence.
    fn source(&self) -> &str;

    /// Deterministic revision discriminator for these immutable statistics.
    /// A host must change it whenever any returned count or exactness changes.
    fn fingerprint(&self) -> u64;

    /// Visit sorted, unique IDs in the provider's dictionary generation. This
    /// permits exact domain union across segments without adding distinct counts.
    /// Returns false when the domain is unknown. A visitor may stop early.
    ///
    /// # Errors
    /// Returns an integrity error for a malformed sealed domain. Storage faults
    /// remain latched on the owning view, as on its other metadata paths.
    fn visit_distinct(
        &self,
        _predicate: I,
        _graph: GraphMatch<I>,
        _stream: StatisticsStream,
        _position: StatisticsPosition,
        _visit: &mut dyn FnMut(I) -> std::ops::ControlFlow<()>,
    ) -> Result<bool, crate::ir::pack::bits::PackBitsError> {
        Ok(false)
    }

    /// Count a predicate-bound pattern across all three streams. This default
    /// requires every contributing count, and leaves subject restrictions to a
    /// provider with joint statistics rather than guessing independence.
    fn pattern(&self, pattern: ProbePattern<I>) -> Option<StatisticCount> {
        if pattern.s.is_some() {
            return None;
        }
        let predicate = pattern.p?;
        let mut total = StatisticCount {
            value: 0,
            accuracy: StatisticsAccuracy::Exact,
        };
        for stream in [
            StatisticsStream::Asserted,
            StatisticsStream::Reifiers,
            StatisticsStream::Annotations,
        ] {
            let count = match pattern.o {
                Some(object) => self.predicate_object(predicate, object, pattern.g, stream)?,
                None => self.predicate(predicate, pattern.g, stream)?.rows,
            };
            total.value = total.value.checked_add(count.value)?;
            if count.accuracy == StatisticsAccuracy::UpperBound {
                total.accuracy = StatisticsAccuracy::UpperBound;
            }
        }
        Some(total)
    }

    /// Predicate row and distinct-domain counts in the requested stream/scope.
    fn predicate(
        &self,
        predicate: I,
        graph: GraphMatch<I>,
        stream: StatisticsStream,
    ) -> Option<PredicateStatistics>;

    /// Frequency of an exact predicate/object pair, including rdf:type classes.
    fn predicate_object(
        &self,
        predicate: I,
        object: I,
        graph: GraphMatch<I>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount>;

    /// Joint predicate/object-category frequency. A datatype restriction is
    /// meaningful only for Literal; None covers the complete category.
    fn predicate_object_kind(
        &self,
        predicate: I,
        kind: RdfTermKind,
        datatype: Option<I>,
        graph: GraphMatch<I>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount>;

    /// Reifier declarations whose target triple has this predicate. These are
    /// target-component statistics, not the virtual rdf:reifies predicate count.
    fn reifier_target_predicate(
        &self,
        predicate: I,
        graph: GraphMatch<I>,
    ) -> Option<PredicateStatistics>;
}
