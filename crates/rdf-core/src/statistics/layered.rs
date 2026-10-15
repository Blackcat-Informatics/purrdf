// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conservative physical evidence for chronological effective sets.

use super::{
    PredicateStatistics, SnapshotStatistics, StatisticCount, StatisticsAccuracy, StatisticsStream,
};
use crate::{DatasetView, GraphMatch, ProbePattern, RdfTermKind, ViewTermId};

#[derive(Clone, Copy)]
pub(crate) enum Request {
    Pattern,
    Stream(StatisticsStream),
    Predicate(StatisticsStream),
    Object(StatisticsStream),
    Kind(StatisticsStream, RdfTermKind),
    Target,
}

impl Request {
    pub(crate) fn read<I: ViewTermId>(
        self,
        provider: &dyn SnapshotStatistics<I>,
        pattern: ProbePattern<I>,
        layered: bool,
    ) -> Option<PredicateStatistics> {
        // Ordinary and annotation rows can change classification when another
        // layer adds or removes a reifier declaration. Either effective stream
        // is bounded by their combined physical population, not its old label.
        let stream = match self {
            Self::Stream(stream)
            | Self::Predicate(stream)
            | Self::Object(stream)
            | Self::Kind(stream, _) => Some(stream),
            _ => None,
        };
        if layered && stream.is_some_and(|stream| stream != StatisticsStream::Reifiers) {
            let with_stream = |stream| match self {
                Self::Stream(_) => Self::Stream(stream),
                Self::Predicate(_) => Self::Predicate(stream),
                Self::Object(_) => Self::Object(stream),
                Self::Kind(_, kind) => Self::Kind(stream, kind),
                _ => unreachable!(),
            };
            return combine(
                [StatisticsStream::Asserted, StatisticsStream::Annotations]
                    .into_iter()
                    .map(|stream| with_stream(stream).read(provider, pattern, false)),
                true,
            );
        }
        let count = match self {
            Self::Pattern => provider.pattern(pattern),
            Self::Stream(stream) => provider.stream_rows(pattern.g, stream),
            Self::Predicate(stream) => return provider.predicate(pattern.p?, pattern.g, stream),
            Self::Object(stream) => {
                provider.predicate_object(pattern.p?, pattern.o?, pattern.g, stream)
            }
            Self::Kind(stream, kind) => {
                provider.predicate_object_kind(pattern.p?, kind, pattern.o, pattern.g, stream)
            }
            Self::Target => return provider.reifier_target_predicate(pattern.p?, pattern.g),
        }?;
        Some(PredicateStatistics {
            rows: count,
            subjects: None,
            objects: None,
        })
    }
}

pub(crate) fn combine(
    sources: impl IntoIterator<Item = Option<PredicateStatistics>>,
    layered: bool,
) -> Option<PredicateStatistics> {
    let mut result = PredicateStatistics {
        rows: StatisticCount {
            value: 0,
            accuracy: StatisticsAccuracy::Exact,
        },
        subjects: None,
        objects: None,
    };
    for source in sources {
        let source = source?;
        result.rows.value = result.rows.value.checked_add(source.rows.value)?;
        if source.rows.accuracy == StatisticsAccuracy::UpperBound {
            result.rows.accuracy = StatisticsAccuracy::UpperBound;
        }
        if !layered {
            result.subjects = source.subjects;
            result.objects = source.objects;
        }
    }
    if layered && result.rows.value != 0 {
        result.rows.accuracy = StatisticsAccuracy::UpperBound;
    }
    // Union sizes require identities in a common generation. Source-local
    // NDVs are never added, even when the row sum is a useful upper bound.
    Some(result)
}

pub(crate) trait LayeredStatistics: DatasetView + Sync {
    fn statistics_source(&self) -> &'static str;
    fn statistics_revision(&self) -> u64;
    fn physical_statistic(
        &self,
        request: Request,
        pattern: ProbePattern<Self::Id>,
    ) -> Option<PredicateStatistics>;
}

impl<D: LayeredStatistics> SnapshotStatistics<D::Id> for D {
    fn source(&self) -> &str {
        self.statistics_source()
    }

    fn fingerprint(&self) -> u64 {
        self.statistics_revision()
    }

    fn pattern(&self, pattern: ProbePattern<D::Id>) -> Option<StatisticCount> {
        Some(self.physical_statistic(Request::Pattern, pattern)?.rows)
    }

    fn stream_rows(
        &self,
        graph: GraphMatch<D::Id>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        Some(
            self.physical_statistic(
                Request::Stream(stream),
                ProbePattern {
                    s: None,
                    p: None,
                    o: None,
                    g: graph,
                },
            )?
            .rows,
        )
    }

    fn predicate(
        &self,
        predicate: D::Id,
        graph: GraphMatch<D::Id>,
        stream: StatisticsStream,
    ) -> Option<PredicateStatistics> {
        self.physical_statistic(
            Request::Predicate(stream),
            ProbePattern {
                s: None,
                p: Some(predicate),
                o: None,
                g: graph,
            },
        )
    }

    fn predicate_object(
        &self,
        predicate: D::Id,
        object: D::Id,
        graph: GraphMatch<D::Id>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        Some(
            self.physical_statistic(
                Request::Object(stream),
                ProbePattern {
                    s: None,
                    p: Some(predicate),
                    o: Some(object),
                    g: graph,
                },
            )?
            .rows,
        )
    }

    fn predicate_object_kind(
        &self,
        predicate: D::Id,
        kind: RdfTermKind,
        datatype: Option<D::Id>,
        graph: GraphMatch<D::Id>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        Some(
            self.physical_statistic(
                Request::Kind(stream, kind),
                ProbePattern {
                    s: None,
                    p: Some(predicate),
                    o: datatype,
                    g: graph,
                },
            )?
            .rows,
        )
    }

    fn reifier_target_predicate(
        &self,
        predicate: D::Id,
        graph: GraphMatch<D::Id>,
    ) -> Option<PredicateStatistics> {
        self.physical_statistic(
            Request::Target,
            ProbePattern {
                s: None,
                p: Some(predicate),
                o: None,
                g: graph,
            },
        )
    }
}
