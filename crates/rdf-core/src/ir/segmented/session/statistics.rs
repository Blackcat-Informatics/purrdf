// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Selective counts and streaming domains through the original block owner.

use super::{GlobalTermId, GraphMatch, SegmentedError, SegmentedSession, fixed_records, format};
use crate::ir::pack::bits::{PackBitsError, read_varint};
use crate::ir::statistics::{
    PredicateRecord, Relation, StatisticsSection, decode_frequency, decode_kind, exact, kind_key,
    relation,
};
use crate::{
    PredicateStatistics, RdfTermKind, SnapshotStatistics, StatisticCount, StatisticsPosition,
    StatisticsStream,
};
use std::ops::ControlFlow;

struct SourceDirectory<'a> {
    session: &'a SegmentedSession,
    stream: super::Stream,
}

impl crate::ir::source_directory::DirectoryRead for SourceDirectory<'_> {
    type Error = SegmentedError;
    type Record = crate::ir::source_directory::SourceRecord;

    fn len(&self) -> Result<usize, Self::Error> {
        self.session.ready()?;
        usize::try_from(self.stream.rows).map_err(|_| SegmentedError::AddressExhausted)
    }

    fn record(
        &self,
        index: usize,
    ) -> Result<crate::ir::source_directory::SourceRecord, Self::Error> {
        let bytes = self
            .session
            .fixed_record::<{ crate::ir::source_directory::RECORD_BYTES }>(
                self.stream,
                format::SOURCE_DIRECTORY,
                index as u64,
            )?;
        let stream = match bytes[36] {
            0 => StatisticsStream::Asserted,
            1 => StatisticsStream::Reifiers,
            2 => StatisticsStream::Annotations,
            _ => return Err(SegmentedError::Corrupt("invalid source directory stream")),
        };
        Ok(crate::ir::source_directory::SourceRecord {
            digest: bytes[..32].try_into().expect("fixed source digest"),
            ordinal: crate::bytes::read_u32_le(&bytes, 32).expect("fixed source ordinal"),
            stream,
        })
    }

    fn prefix(
        &self,
        end: usize,
        work: &mut crate::source::SourceDifferenceWork,
    ) -> Result<purrdf_hash::multiset::Multiset256, Self::Error> {
        use crate::ir::source_directory::CHECKPOINT_ROWS;

        if end > self.len()? {
            return Err(SegmentedError::Corrupt(
                "source directory prefix outside section",
            ));
        }
        let checkpoint = end / CHECKPOINT_ROWS;
        let start = checkpoint * CHECKPOINT_ROWS;
        let mut prefix = if checkpoint == 0 {
            purrdf_hash::multiset::Multiset256::default()
        } else {
            let sum = self.session.fixed_record::<32>(
                self.session.header.source_checkpoints,
                format::SOURCE_CHECKPOINTS,
                (checkpoint - 1) as u64,
            )?;
            purrdf_hash::multiset::Multiset256::from_parts(sum, start as u128)
        };
        for index in start..end {
            let record = self.record(index)?;
            prefix
                .insert(record.digest)
                .map_err(|_| SegmentedError::AddressExhausted)?;
            work.members += 1;
        }
        Ok(prefix)
    }
}

impl SegmentedSession {
    /// Independently certified normalized statement/reifier keys and addresses.
    pub(in crate::ir::segmented) fn source_directory(
        &self,
    ) -> impl crate::ir::source_directory::DirectoryRead<
        Error = SegmentedError,
        Record = crate::ir::source_directory::SourceRecord,
    > + '_ {
        SourceDirectory {
            session: self,
            stream: self.header.effective_directory,
        }
    }

    pub(in crate::ir::segmented) fn source_record_row(
        &self,
        record: crate::ir::source_directory::SourceRecord,
    ) -> Result<crate::QuadIds<GlobalTermId>, SegmentedError> {
        let table = match record.stream {
            StatisticsStream::Asserted => 0,
            StatisticsStream::Reifiers => 1,
            StatisticsStream::Annotations => 2,
        };
        format::decode_quad(&self.fixed_record::<32>(
            self.header.streams[table],
            format::QUADS,
            u64::from(record.ordinal),
        )?)
    }

    pub(super) fn source_summary(
        &self,
        predicate: GlobalTermId,
        graph: GraphMatch<GlobalTermId>,
        stream: StatisticsStream,
    ) -> Result<Option<crate::source::SourceSummary>, SegmentedError> {
        self.ready()?;
        self.find_statistic(relation(stream), graph, predicate)
            .map(|record| {
                Some(record.map_or_else(
                    crate::source::SourceSummary::default,
                    PredicateRecord::source_summary,
                ))
            })
            .or_else(|error| self.fail(error))
    }

    fn narrow_subject_statistic(
        &self,
        pattern: crate::ProbePattern<GlobalTermId>,
        count: Option<StatisticCount>,
    ) -> Option<StatisticCount> {
        let mut count = count?;
        self.statistic(|| {
            if pattern.s.is_some() {
                // This source already counts the selected subject range across
                // all three streams. Keep that tighter bound in replay too.
                count.value = count.value.min(crate::DatasetView::cardinality_estimate(
                    self, pattern.s, pattern.p, pattern.o, pattern.g,
                ));
            }
            self.ready()?;
            Ok(count)
        })
    }
    fn statistic<T>(&self, read: impl FnOnce() -> Result<T, SegmentedError>) -> Option<T> {
        match self.ready().and_then(|()| read()) {
            Ok(value) => Some(value),
            Err(error) => {
                // The optional metadata interface keeps operational failures on
                // the original fallible view; guarded completion returns it.
                let _ = self.fail::<()>(error);
                None
            }
        }
    }

    fn statistic_record<const N: usize>(
        &self,
        section: StatisticsSection,
        ordinal: u64,
    ) -> Result<[u8; N], SegmentedError> {
        let stream = self.header.statistics[section.index()];
        if N != section.width() {
            return Err(SegmentedError::Corrupt("wrong statistics record width"));
        }
        self.fixed_record(stream, format::statistics_kind(section), ordinal)
    }

    fn fixed_record<const N: usize>(
        &self,
        stream: super::Stream,
        kind: u8,
        ordinal: u64,
    ) -> Result<[u8; N], SegmentedError> {
        self.ready()?;
        if ordinal >= stream.rows {
            return Err(SegmentedError::Corrupt(
                "fixed record ordinal outside section",
            ));
        }
        let per = (u64::from(self.header.block_bytes) - 9) / N as u64;
        let block = self.block(stream.first + ordinal / per)?;
        let data = fixed_records(&block.encoded, kind, N)?;
        if data.len() as u64 / N as u64 != (stream.rows - ordinal / per * per).min(per) {
            return Err(SegmentedError::Corrupt("false statistics record packing"));
        }
        let begin =
            usize::try_from(ordinal % per).map_err(|_| SegmentedError::AddressExhausted)? * N;
        data.get(begin..begin + N)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(SegmentedError::Corrupt("missing statistics record"))
    }

    fn statistic_predicate(
        &self,
        ordinal: u64,
    ) -> Result<PredicateRecord<GlobalTermId>, SegmentedError> {
        let record =
            PredicateRecord::decode(
                &self.statistic_record::<{ crate::ir::statistics::PREDICATE_RECORD_BYTES }>(
                    StatisticsSection::Predicates,
                    ordinal,
                )?,
            )?;
        record.validate_ranges(self.header.statistics.map(|stream| stream.rows))?;
        Ok(record)
    }

    fn find_statistic(
        &self,
        relation: Relation,
        graph: GraphMatch<GlobalTermId>,
        predicate: GlobalTermId,
    ) -> Result<Option<PredicateRecord<GlobalTermId>>, SegmentedError> {
        let table = match relation {
            Relation::Asserted => 0,
            Relation::Reifiers | Relation::Targets => 1,
            Relation::Annotations => 2,
        };
        if self.header.streams[table].rows == 0 {
            return Ok(None);
        }
        let rows = self.header.statistics[0].rows;
        let index = format::partition_point(rows, |index| {
            Ok::<_, SegmentedError>(
                self.statistic_predicate(index)?
                    .compare_key(relation, graph, predicate)
                    .is_lt(),
            )
        })?;
        if index == rows {
            return Ok(None);
        }
        let record = self.statistic_predicate(index)?;
        Ok(record
            .compare_key(relation, graph, predicate)
            .is_eq()
            .then_some(record))
    }

    fn frequency_record(&self, ordinal: u64) -> Result<(GlobalTermId, u64), SegmentedError> {
        Ok(decode_frequency(&self.statistic_record::<16>(
            StatisticsSection::Frequencies,
            ordinal,
        )?)?)
    }
    fn kind_record(&self, ordinal: u64) -> Result<(u8, Option<GlobalTermId>, u64), SegmentedError> {
        Ok(decode_kind(&self.statistic_record::<32>(
            StatisticsSection::Kinds,
            ordinal,
        )?)?)
    }

    fn copy_statistic_domain(&self, offset: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        let stream = self.header.statistics[StatisticsSection::Domains.index()];
        if offset
            .checked_add(output.len() as u64)
            .is_none_or(|end| end > stream.rows)
        {
            return Err(SegmentedError::Corrupt("statistics domain outside section"));
        }
        let per = u64::from(self.header.block_bytes) - 9;
        let mut copied = 0;
        while copied < output.len() {
            let position = offset + copied as u64;
            let block = self.block(stream.first + position / per)?;
            let data = fixed_records(
                &block.encoded,
                format::statistics_kind(StatisticsSection::Domains),
                1,
            )?;
            if data.len() as u64 != (stream.rows - position / per * per).min(per) {
                return Err(SegmentedError::Corrupt("false statistics domain packing"));
            }
            let begin =
                usize::try_from(position % per).map_err(|_| SegmentedError::AddressExhausted)?;
            let count = (data.len() - begin).min(output.len() - copied);
            output[copied..copied + count].copy_from_slice(&data[begin..begin + count]);
            copied += count;
        }
        Ok(())
    }
}

fn empty_summary() -> PredicateStatistics {
    PredicateStatistics {
        rows: exact(0),
        subjects: Some(exact(0)),
        objects: Some(exact(0)),
    }
}

impl SnapshotStatistics<GlobalTermId> for SegmentedSession {
    fn pattern(&self, pattern: crate::ProbePattern<GlobalTermId>) -> Option<StatisticCount> {
        self.narrow_subject_statistic(pattern, crate::statistics::pattern_count(self, pattern))
    }

    fn structural_pattern(
        &self,
        pattern: crate::ProbePattern<GlobalTermId>,
        components: [Option<GlobalTermId>; 3],
    ) -> Option<StatisticCount> {
        self.narrow_subject_statistic(
            pattern,
            crate::statistics::structural_count(self, pattern, components),
        )
    }
    fn source(&self) -> &'static str {
        "segmented-sealed-predicate-domains-v1"
    }
    fn fingerprint(&self) -> u64 {
        crate::bytes::read_u64_le(self.receipt.snapshot().as_bytes(), 0)
            .expect("fixed snapshot identity width")
    }
    fn stream_rows(
        &self,
        graph: GraphMatch<GlobalTermId>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        self.statistic(|| {
            let table = match stream {
                StatisticsStream::Asserted => 0,
                StatisticsStream::Reifiers => 1,
                StatisticsStream::Annotations => 2,
            };
            if graph == GraphMatch::Any || self.header.streams[table].rows == 0 {
                return Ok(exact(self.header.streams[table].rows));
            }
            let rows = self.header.statistics[0].rows;
            let first = format::partition_point(rows, |index| {
                Ok::<_, SegmentedError>(
                    self.statistic_predicate(index)?
                        .compare_scope(relation(stream), graph)
                        .is_lt(),
                )
            })?;
            let mut total = 0_u64;
            for index in first..rows {
                let entry = self.statistic_predicate(index)?;
                if !entry.compare_scope(relation(stream), graph).is_eq() {
                    break;
                }
                total = total
                    .checked_add(entry.summary().rows.value)
                    .ok_or(SegmentedError::Corrupt("statistics row sum overflow"))?;
            }
            Ok(exact(total))
        })
    }
    fn predicate(
        &self,
        predicate: GlobalTermId,
        graph: GraphMatch<GlobalTermId>,
        stream: StatisticsStream,
    ) -> Option<PredicateStatistics> {
        self.statistic(|| {
            Ok(self
                .find_statistic(relation(stream), graph, predicate)?
                .map_or_else(empty_summary, PredicateRecord::summary))
        })
    }
    fn predicate_object(
        &self,
        predicate: GlobalTermId,
        object: GlobalTermId,
        graph: GraphMatch<GlobalTermId>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        self.statistic(|| {
            let Some(entry) = self.find_statistic(relation(stream), graph, predicate)? else {
                return Ok(Some(exact(0)));
            };
            let Some((first, count)) = entry.frequency_range() else {
                return Ok(None);
            };
            let index = format::partition_point(count, |index| {
                Ok::<_, SegmentedError>(self.frequency_record(first + index)?.0 < object)
            })?;
            let rows = if index < count {
                let (id, rows) = self.frequency_record(first + index)?;
                if id == object { rows } else { 0 }
            } else {
                0
            };
            Ok(Some(exact(rows)))
        })
        .flatten()
    }
    fn predicate_object_kind(
        &self,
        predicate: GlobalTermId,
        kind: RdfTermKind,
        datatype: Option<GlobalTermId>,
        graph: GraphMatch<GlobalTermId>,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        self.statistic(|| {
            let Some(entry) = self.find_statistic(relation(stream), graph, predicate)? else {
                return Ok(exact(0));
            };
            let (first, count) = entry.kind_range();
            let key = (kind_key(kind), datatype);
            let index = format::partition_point(count, |index| {
                let (kind, datatype, _) = self.kind_record(first + index)?;
                Ok::<_, SegmentedError>((kind, datatype) < key)
            })?;
            let rows = if index < count {
                let (kind, datatype, rows) = self.kind_record(first + index)?;
                if (kind, datatype) == key { rows } else { 0 }
            } else {
                0
            };
            Ok(exact(rows))
        })
    }
    fn reifier_target_predicate(
        &self,
        predicate: GlobalTermId,
        graph: GraphMatch<GlobalTermId>,
    ) -> Option<PredicateStatistics> {
        self.statistic(|| {
            Ok(self
                .find_statistic(Relation::Targets, graph, predicate)?
                .map_or_else(empty_summary, PredicateRecord::summary))
        })
    }
    fn visit_distinct(
        &self,
        predicate: GlobalTermId,
        graph: GraphMatch<GlobalTermId>,
        stream: StatisticsStream,
        position: StatisticsPosition,
        visit: &mut dyn FnMut(GlobalTermId) -> ControlFlow<()>,
    ) -> Result<bool, PackBitsError> {
        let result = (|| {
            self.ready()?;
            let Some(entry) = self.find_statistic(relation(stream), graph, predicate)? else {
                return Ok::<_, SegmentedError>(());
            };
            let [offset, length, count] = entry.domain(position);
            let mut consumed = 0_u64;
            let mut previous = 0_u64;
            for ordinal in 0..count {
                let remaining = usize::try_from((length - consumed).min(10))
                    .map_err(|_| SegmentedError::AddressExhausted)?;
                let mut encoded = [0; 10];
                self.copy_statistic_domain(offset + consumed, &mut encoded[..remaining])?;
                let mut cursor = 0;
                let delta = read_varint(&encoded[..remaining], &mut cursor)?;
                let value = previous
                    .checked_add(delta)
                    .ok_or(PackBitsError::Malformed("statistics domain value overflow"))?;
                if ordinal != 0 && value <= previous {
                    return Err(PackBitsError::Malformed(
                        "statistics domain is not strictly ordered",
                    )
                    .into());
                }
                let id = GlobalTermId::checked_from_index(value).ok_or(
                    PackBitsError::Malformed("invalid statistics domain identifier"),
                )?;
                previous = value;
                consumed += cursor as u64;
                if visit(id).is_break() {
                    return Ok(());
                }
            }
            if consumed != length {
                return Err(
                    PackBitsError::Malformed("statistics domain has trailing bytes").into(),
                );
            }
            Ok(())
        })();
        match result {
            Ok(()) => Ok(true),
            Err(error) => {
                let _ = self.fail::<()>(error.clone());
                if let SegmentedError::Statistics(error) = error {
                    Err(error)
                } else {
                    Ok(false)
                }
            }
        }
    }
}
