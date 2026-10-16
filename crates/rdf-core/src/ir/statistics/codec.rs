// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed statistical records: counts are independently readable from domains.

use super::{Domain, LocalStatistics, Predicate, Relation, Scope, StatisticsId, exact, kind_key};
use crate::ir::pack::bits::PackBitsError;
use crate::ir::pack::bits::PackBitsError::Malformed as invalid;
use crate::{GraphMatch, PredicateStatistics, StatisticsPosition};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ir) enum StatisticsSection {
    Predicates,
    Domains,
    Frequencies,
    Kinds,
}
impl StatisticsSection {
    pub(in crate::ir) const ALL: [Self; 4] = [
        Self::Predicates,
        Self::Domains,
        Self::Frequencies,
        Self::Kinds,
    ];
    pub(in crate::ir) const fn index(self) -> usize {
        self as usize
    }
    pub(in crate::ir) const fn width(self) -> usize {
        match self {
            Self::Predicates => 136,
            Self::Domains => 1,
            Self::Frequencies => 16,
            Self::Kinds => 32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::ir) struct PredicateRecord<I> {
    relation: Relation,
    scope: Scope<I>,
    predicate: I,
    rows: u64,
    subjects: [u64; 3],
    objects: [u64; 3],
    frequencies: Option<(u64, u64)>,
    unbounded: bool,
    kinds: (u64, u64),
}

fn read_words<const N: usize>(bytes: &[u8]) -> Result<[u64; N], PackBitsError> {
    if bytes.len() != N * 8 {
        return Err(invalid("wrong statistics record width"));
    }
    let mut values = [0; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = crate::bytes::read_u64_le(bytes, index * 8)
            .ok_or(invalid("truncated statistics word"))?;
    }
    Ok(values)
}

fn flag(value: u64) -> Result<bool, PackBitsError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid("invalid statistics flag")),
    }
}

impl<I: StatisticsId> From<&Predicate<I>> for PredicateRecord<I> {
    fn from(entry: &Predicate<I>) -> Self {
        let domain = |value: Domain| [value.offset as u64, value.bytes as u64, value.count as u64];
        Self {
            relation: entry.key.0,
            scope: entry.key.1,
            predicate: entry.key.2,
            rows: entry.rows,
            subjects: domain(entry.subjects),
            objects: domain(entry.objects),
            frequencies: entry
                .frequencies
                .map(|(offset, count)| (offset as u64, count as u64)),
            unbounded: entry.unbounded_frequencies,
            kinds: (entry.kinds.0 as u64, entry.kinds.1 as u64),
        }
    }
}

impl<I: StatisticsId> PredicateRecord<I> {
    fn words(self) -> [u64; 17] {
        let (scope, graph) = match self.scope {
            Scope::Any => (0, 0),
            Scope::Graph(None) => (1, 0),
            Scope::Graph(Some(id)) => (2, id.statistics_index()),
        };
        let (frequency_offset, frequency_count) = self.frequencies.unwrap_or((0, 0));
        [
            self.relation as u64,
            scope,
            graph,
            self.predicate.statistics_index(),
            self.rows,
            self.subjects[0],
            self.subjects[1],
            self.subjects[2],
            self.objects[0],
            self.objects[1],
            self.objects[2],
            u64::from(self.frequencies.is_some()),
            u64::from(self.unbounded),
            frequency_offset,
            frequency_count,
            self.kinds.0,
            self.kinds.1,
        ]
    }

    pub(in crate::ir) fn decode(bytes: &[u8]) -> Result<Self, PackBitsError> {
        let v = read_words::<17>(bytes)?;
        let relation = match v[0] {
            0 => Relation::Asserted,
            1 => Relation::Reifiers,
            2 => Relation::Annotations,
            3 => Relation::Targets,
            _ => return Err(invalid("invalid statistics relation")),
        };
        let scope = match (v[1], v[2]) {
            (0, 0) => Scope::Any,
            (1, 0) => Scope::Graph(None),
            (2, id) => Scope::Graph(Some(I::statistics_id(id)?)),
            _ => return Err(invalid("invalid statistics graph scope")),
        };
        let frequencies = if flag(v[11])? {
            Some((v[13], v[14]))
        } else if v[13] == 0 && v[14] == 0 {
            None
        } else {
            return Err(invalid("nonzero absent statistics frequency range"));
        };
        let value = Self {
            relation,
            scope,
            predicate: I::statistics_id(v[3])?,
            rows: v[4],
            subjects: [v[5], v[6], v[7]],
            objects: [v[8], v[9], v[10]],
            frequencies,
            unbounded: flag(v[12])?,
            kinds: (v[15], v[16]),
        };
        if value.rows == 0
            || [value.subjects[2], value.objects[2]]
                .iter()
                .any(|&count| count == 0 || count > value.rows)
            || value.kinds.1 == 0
            || value.kinds.1 > value.rows.saturating_mul(2)
            || value
                .frequencies
                .is_some_and(|(_, count)| count != value.objects[2])
            || (value.unbounded && value.frequencies.is_none())
        {
            return Err(invalid("inconsistent statistics counts"));
        }
        Ok(value)
    }

    pub(in crate::ir) fn validate_ranges(self, sections: [u64; 4]) -> Result<(), PackBitsError> {
        for (offset, length, limit) in [
            (self.subjects[0], self.subjects[1], sections[1]),
            (self.objects[0], self.objects[1], sections[1]),
            (self.kinds.0, self.kinds.1, sections[3]),
        ]
        .into_iter()
        .chain(
            self.frequencies
                .map(|(offset, count)| (offset, count, sections[2])),
        ) {
            if offset.checked_add(length).is_none_or(|end| end > limit) {
                return Err(invalid("statistics range outside its section"));
            }
        }
        Ok(())
    }

    pub(in crate::ir) fn compare_scope(
        self,
        relation: Relation,
        graph: GraphMatch<I>,
    ) -> std::cmp::Ordering {
        (self.relation, self.scope).cmp(&(relation, scope_of(graph)))
    }
    pub(in crate::ir) fn compare_key(
        self,
        relation: Relation,
        graph: GraphMatch<I>,
        predicate: I,
    ) -> std::cmp::Ordering {
        (self.relation, self.scope, self.predicate).cmp(&(relation, scope_of(graph), predicate))
    }
    pub(in crate::ir) fn summary(self) -> PredicateStatistics {
        PredicateStatistics {
            rows: exact(self.rows),
            subjects: Some(exact(self.subjects[2])),
            objects: Some(exact(self.objects[2])),
        }
    }
    pub(in crate::ir) const fn frequency_range(self) -> Option<(u64, u64)> {
        self.frequencies
    }
    pub(in crate::ir) const fn kind_range(self) -> (u64, u64) {
        self.kinds
    }
    pub(in crate::ir) fn domain(self, position: StatisticsPosition) -> [u64; 3] {
        match position {
            StatisticsPosition::Subject => self.subjects,
            StatisticsPosition::Object => self.objects,
        }
    }
}

pub(super) fn scope_of<I: crate::ViewTermId>(graph: GraphMatch<I>) -> Scope<I> {
    match graph {
        GraphMatch::Any => Scope::Any,
        GraphMatch::Default => Scope::Graph(None),
        GraphMatch::Named(id) => Scope::Graph(Some(id)),
    }
}

pub(in crate::ir) fn decode_frequency<I: StatisticsId>(
    bytes: &[u8],
) -> Result<(I, u64), PackBitsError> {
    let [id, rows] = read_words::<2>(bytes)?;
    if rows == 0 {
        return Err(invalid("empty statistics frequency"));
    }
    Ok((I::statistics_id(id)?, rows))
}

pub(in crate::ir) fn decode_kind<I: StatisticsId>(
    bytes: &[u8],
) -> Result<(u8, Option<I>, u64), PackBitsError> {
    let [kind, has_datatype, datatype, rows] = read_words::<4>(bytes)?;
    let kind = u8::try_from(kind)
        .ok()
        .filter(|&kind| kind < 4)
        .ok_or(invalid("invalid statistics category"))?;
    let datatype = if flag(has_datatype)? {
        Some(I::statistics_id(datatype)?)
    } else if datatype == 0 {
        None
    } else {
        return Err(invalid("nonzero absent statistics datatype"));
    };
    if rows == 0 || (kind != kind_key(crate::RdfTermKind::Literal) && datatype.is_some()) {
        return Err(invalid("inconsistent statistics category"));
    }
    Ok((kind, datatype, rows))
}

pub(super) struct RecordWords {
    values: [u64; 17],
    length: usize,
}
impl RecordWords {
    pub(super) fn values(&self) -> &[u64] {
        &self.values[..self.length]
    }
}

impl<I: StatisticsId> LocalStatistics<I> {
    pub(in crate::ir) fn section_rows(&self, section: StatisticsSection) -> usize {
        match section {
            StatisticsSection::Predicates => self.predicates.len(),
            StatisticsSection::Domains => self.domains.len(),
            StatisticsSection::Frequencies => self.frequencies.len(),
            StatisticsSection::Kinds => self.kinds.len(),
        }
    }

    pub(super) fn record_words(
        &self,
        section: StatisticsSection,
        index: usize,
    ) -> Option<RecordWords> {
        let mut words = RecordWords {
            values: [0; 17],
            length: section.width() / 8,
        };
        match section {
            StatisticsSection::Predicates => {
                words.values = PredicateRecord::from(self.predicates.get(index)?).words();
            }
            StatisticsSection::Domains => return None,
            StatisticsSection::Frequencies => {
                let value = self.frequencies.get(index)?;
                words.values[..2].copy_from_slice(&[value.object.statistics_index(), value.rows]);
            }
            StatisticsSection::Kinds => {
                let value = self.kinds.get(index)?;
                words.values[..4].copy_from_slice(&[
                    u64::from(value.kind),
                    u64::from(value.datatype.is_some()),
                    value.datatype.map_or(0, StatisticsId::statistics_index),
                    value.rows,
                ]);
            }
        }
        Some(words)
    }

    pub(in crate::ir) fn encode_records(
        &self,
        section: StatisticsSection,
        first: usize,
        output: &mut [u8],
    ) -> Result<(), PackBitsError> {
        if !output.len().is_multiple_of(section.width()) {
            return Err(invalid("partial statistics output record"));
        }
        if section == StatisticsSection::Domains {
            let end = first
                .checked_add(output.len())
                .ok_or(invalid("statistics output range overflow"))?;
            output.copy_from_slice(
                self.domains
                    .get(first..end)
                    .ok_or(invalid("statistics output outside domain section"))?,
            );
        } else {
            for (index, bytes) in output.chunks_exact_mut(section.width()).enumerate() {
                self.encode_record(
                    section,
                    first
                        .checked_add(index)
                        .ok_or(invalid("statistics output range overflow"))?,
                    bytes,
                )?;
            }
        }
        Ok(())
    }

    fn encode_record(
        &self,
        section: StatisticsSection,
        index: usize,
        output: &mut [u8],
    ) -> Result<(), PackBitsError> {
        if output.len() != section.width() {
            return Err(invalid("wrong statistics output width"));
        }
        if section == StatisticsSection::Domains {
            output[0] = *self
                .domains
                .get(index)
                .ok_or(invalid("statistics record outside its section"))?;
            return Ok(());
        }
        let words = self
            .record_words(section, index)
            .ok_or(invalid("statistics record outside its section"))?;
        for (bytes, &word) in output.as_chunks_mut::<8>().0.iter_mut().zip(words.values()) {
            bytes.copy_from_slice(&word.to_le_bytes());
        }
        Ok(())
    }
}
