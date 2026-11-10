// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Snapshot-bound statistics shared by query and rule planning.
//!
//! These describe effective RDF rows, never a resource charge or a guaranteed
//! output bound. Unknown counts remain absent. Providers retain the same local
//! identity space as their DatasetView and do not translate IDs numerically.

use crate::{GraphMatch, ProbePattern, RdfTermKind, ViewTermId};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) mod layered;

/// Opaque process-local identity of one immutable statistics owner or revision.
///
/// This token is not a content fingerprint, a persisted identity or a proof of
/// dictionary compatibility. An owner retains its token while its statistical
/// answers remain immutable; a different owner or revision obtains a fresh one.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatisticsOwnerToken(NonZeroUsize);

purrdf_hash::debug_non_exhaustive!(StatisticsOwnerToken {});

impl StatisticsOwnerToken {
    /// Issue a token that has never named another owner in this process.
    ///
    /// Returns `None` permanently once the checked issuer is exhausted. Providers
    /// then report no cache identity and planning continues without retention.
    #[must_use]
    pub fn try_new() -> Option<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(1);
        Self::issue(&NEXT)
    }

    fn issue(next: &AtomicUsize) -> Option<Self> {
        let value = next
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .ok()?;
        // The private production issuer starts at one and never wraps or resets.
        Some(Self(
            NonZeroUsize::new(value).expect("nonzero statistics owner issuer"),
        ))
    }
}

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

    /// Process-local token for retaining forecasts of this immutable owner.
    /// The same token may be shared only while the owner's statistical answers,
    /// availability, exactness and ordered distinct-ID sequences are unchanged.
    /// Consumers still key every numeric request and all other planning inputs.
    /// Tokens do not certify equal content across owners or compatible dictionaries.
    /// The deterministic fingerprint and physical access model remain separate.
    /// Unknown providers and an exhausted token issuer return `None`, keeping the
    /// original planning path without numeric forecast retention.
    fn cache_identity(&self) -> Option<StatisticsOwnerToken> {
        None
    }

    /// Total rows in one effective stream and graph scope. Unknown providers
    /// retain None; a stored zero certifies absence in that stream.
    fn stream_rows(
        &self,
        _graph: GraphMatch<I>,
        _stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        None
    }

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

    /// Count a pattern across all three streams, using joint object frequencies
    /// when available and stream totals for an unbound predicate. Residual subject
    /// and unsupported object restrictions produce an upper bound; no independence
    /// assumption or fabricated zero substitutes for a missing count.
    fn pattern(&self, pattern: ProbePattern<I>) -> Option<StatisticCount> {
        pattern_count(self, pattern)
    }

    /// Conservative candidate count for a structural triple in object position.
    /// Object-category counts cover ordinary and annotation rows; target-predicate
    /// counts narrow reifiers. Remaining component/subject restrictions are
    /// residuals, so the result is explicitly an upper bound, never a joint exact
    /// count or an allocation certificate. Missing metadata stays unknown.
    fn structural_pattern(
        &self,
        pattern: ProbePattern<I>,
        components: [Option<I>; 3],
    ) -> Option<StatisticCount> {
        structural_count(self, pattern, components)
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

pub(crate) fn pattern_count<I: ViewTermId, S: SnapshotStatistics<I> + ?Sized>(
    provider: &S,
    pattern: ProbePattern<I>,
) -> Option<StatisticCount> {
    let predicate = pattern.p;
    let mut total = StatisticCount {
        value: 0,
        accuracy: if pattern.s.is_some() || (predicate.is_none() && pattern.o.is_some()) {
            StatisticsAccuracy::UpperBound
        } else {
            StatisticsAccuracy::Exact
        },
    };
    for stream in [
        StatisticsStream::Asserted,
        StatisticsStream::Reifiers,
        StatisticsStream::Annotations,
    ] {
        let count = match (predicate, pattern.o) {
            (Some(predicate), Some(object)) => {
                provider.predicate_object(predicate, object, pattern.g, stream)?
            }
            (Some(predicate), None) => provider.predicate(predicate, pattern.g, stream)?.rows,
            (None, _) => provider.stream_rows(pattern.g, stream)?,
        };
        total.value = total.value.checked_add(count.value)?;
        if count.accuracy == StatisticsAccuracy::UpperBound {
            total.accuracy = StatisticsAccuracy::UpperBound;
        }
    }
    Some(total)
}

pub(crate) fn structural_count<I: ViewTermId, S: SnapshotStatistics<I> + ?Sized>(
    provider: &S,
    pattern: ProbePattern<I>,
    components: [Option<I>; 3],
) -> Option<StatisticCount> {
    let mut total = 0_u64;
    for stream in [
        StatisticsStream::Asserted,
        StatisticsStream::Reifiers,
        StatisticsStream::Annotations,
    ] {
        let mut count = match pattern.p {
            Some(predicate) => {
                provider
                    .predicate_object_kind(predicate, RdfTermKind::Triple, None, pattern.g, stream)?
                    .value
            }
            None => provider.stream_rows(pattern.g, stream)?.value,
        };
        if stream == StatisticsStream::Reifiers
            && let Some(predicate) = components[1]
        {
            count = count.min(
                provider
                    .reifier_target_predicate(predicate, pattern.g)?
                    .rows
                    .value,
            );
        }
        total = total.checked_add(count)?;
    }
    Some(StatisticCount {
        value: total,
        accuracy: StatisticsAccuracy::UpperBound,
    })
}

#[cfg(test)]
mod owner_token_tests {
    use super::{AtomicUsize, Ordering, StatisticsOwnerToken};

    #[test]
    fn exhausted_issuer_never_wraps_or_reissues_owner_tokens() {
        let next = AtomicUsize::new(usize::MAX - 2);
        let previous = StatisticsOwnerToken::issue(&next).expect("penultimate owner");
        let last = StatisticsOwnerToken::issue(&next).expect("last owner");
        assert_ne!(previous, last);
        assert!(StatisticsOwnerToken::issue(&next).is_none());
        assert!(StatisticsOwnerToken::issue(&next).is_none());
        assert_eq!(next.load(Ordering::Relaxed), usize::MAX);
    }
}
