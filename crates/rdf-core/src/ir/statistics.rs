// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sealed predicate statistics in one validated local dictionary generation.
//! Transient ordinal sorts and retained metadata use the original freeze memory.
//! Distinct domains use the pack codec's delta lists, not another integer codec.

use purrdf_lex::allocation::{Admission, Memory, StorageError, growth_capacity};

use super::dataset::{QuadIds, ReifierRow, TermRef};
use super::frozen_terms::FrozenTerms;
use super::pack::bits::{DeltaListRef, write_varint};
use super::term::TermId;
use crate::{
    GraphMatch, PredicateStatistics, RdfTermKind, StatisticCount, StatisticsAccuracy, ViewTermId,
};

// Keep complete frequencies for low-cardinality objects and every rdf:type
// class. Higher-cardinality predicates retain exact domains and kind counts;
// a missing selected frequency remains unknown, never a fabricated zero.
const SELECTED_OBJECT_FREQUENCIES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Relation {
    Asserted,
    Reifiers,
    Annotations,
    Targets,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Scope<I = TermId> {
    Any,
    Graph(Option<I>),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Domain {
    offset: usize,
    bytes: usize,
    count: usize,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Predicate<I = TermId> {
    key: (Relation, Scope<I>, I),
    rows: u64,
    subjects: Domain,
    objects: Domain,
    frequencies: Option<(usize, usize)>,
    /// Class frequencies remain complete beyond the ordinary selection cap.
    unbounded_frequencies: bool,
    kinds: (usize, usize),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Frequency<I = TermId> {
    object: I,
    rows: u64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct KindCount<I = TermId> {
    kind: u8,
    datatype: Option<I>,
    rows: u64,
}

/// No query-time growth: every slice is frozen through the original admission.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LocalStatistics<I = TermId> {
    predicates: Box<[Predicate<I>]>,
    domains: Box<[u8]>,
    frequencies: Box<[Frequency<I>]>,
    kinds: Box<[KindCount<I>]>,
    fingerprint: u64,
}
purrdf_hash::debug_non_exhaustive!([I] LocalStatistics<I> { fingerprint });

pub(super) struct StatisticsBuilder<I = TermId> {
    predicates: Vec<Predicate<I>>,
    domains: Vec<u8>,
    frequencies: Vec<Frequency<I>>,
    kinds: Vec<KindCount<I>>,
}

// Deriving Default would unnecessarily require an empty identity value.
impl<I> Default for StatisticsBuilder<I> {
    fn default() -> Self {
        Self {
            predicates: Vec::new(),
            domains: Vec::new(),
            frequencies: Vec::new(),
            kinds: Vec::new(),
        }
    }
}

/// The two native sealed dictionary widths. This is not the caller's identity
/// interface: external providers continue to implement SnapshotStatistics.
pub(crate) trait StatisticsId: ViewTermId {
    const VARINT_BYTES: usize;
    const SOURCE: &'static str;
    fn statistics_index(self) -> u64;
    fn statistics_id(index: u64) -> Result<Self, super::pack::bits::PackBitsError>;
}

impl StatisticsId for TermId {
    const VARINT_BYTES: usize = 5;
    const SOURCE: &'static str = "native-frozen-predicate-domains-v1";

    fn statistics_index(self) -> u64 {
        self.index() as u64
    }

    fn statistics_id(index: u64) -> Result<Self, super::pack::bits::PackBitsError> {
        use super::pack::bits::PackBitsError;
        let index = u32::try_from(index)
            .map_err(|_| PackBitsError::Malformed("statistics identity exceeds local width"))?;
        Self::from_index(index)
            .map_err(|_| PackBitsError::Malformed("statistics identity is not representable"))
    }
}

impl StatisticsId for super::GlobalTermId {
    const VARINT_BYTES: usize = 10;
    const SOURCE: &'static str = "merged-dictionary-predicate-domains-v1";

    fn statistics_index(self) -> u64 {
        self.index()
    }

    fn statistics_id(index: u64) -> Result<Self, super::pack::bits::PackBitsError> {
        Self::from_index(index).map_err(|_| {
            super::pack::bits::PackBitsError::Malformed(
                "statistics global identity is not representable",
            )
        })
    }
}

/// Admission failure and malformed encoded metadata remain distinct.
#[derive(Debug)]
pub(super) enum StatisticsMergeError {
    Storage(StorageError),
    Encoding(super::pack::bits::PackBitsError),
}
purrdf_lex::variant_from!(StatisticsMergeError {
    Storage(StorageError), Encoding(super::pack::bits::PackBitsError)
});

impl std::fmt::Display for StatisticsMergeError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(output),
            Self::Encoding(error) => error.fmt(output),
        }
    }
}

fn exact(value: u64) -> StatisticCount {
    StatisticCount {
        value,
        accuracy: StatisticsAccuracy::Exact,
    }
}

fn kind_key(kind: RdfTermKind) -> u8 {
    match kind {
        RdfTermKind::Iri => 0,
        RdfTermKind::BlankNode => 1,
        RdfTermKind::Literal => 2,
        RdfTermKind::Triple => 3,
    }
}

impl StatisticsBuilder {
    /// Aggregate one immutable physical relation without cloning its rows.
    fn add<S: Admission + ?Sized>(
        &mut self,
        relation: Relation,
        count: usize,
        row: impl Fn(usize) -> QuadIds,
        dictionary: &FrozenTerms,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        if count == 0 {
            return Ok(());
        }
        let class_predicate = dictionary.term_id_by_iri(purrdf_iri::vocab::rdf::TYPE);
        let mut ordinals = memory.collect(0..count)?;
        let one_graph = (1..count).all(|index| row(index).g == row(0).g);
        for graph_scoped in [false, true] {
            if graph_scoped && one_graph {
                let end = self
                    .predicates
                    .partition_point(|entry| entry.key.0 <= relation);
                let first = self
                    .predicates
                    .partition_point(|entry| entry.key.0 < relation);
                for index in first..end {
                    let mut alias = self.predicates[index];
                    alias.key.1 = Scope::Graph(row(0).g);
                    memory.push(&mut self.predicates, alias)?;
                }
                break;
            }
            let key = |index| {
                let quad = row(index);
                (
                    relation,
                    if graph_scoped {
                        Scope::Graph(quad.g)
                    } else {
                        Scope::Any
                    },
                    quad.p,
                )
            };
            let start = self.predicates.len();
            ordinals.sort_unstable_by_key(|&index| (key(index), row(index).s));
            let mut begin = 0;
            while begin < count {
                let group = key(ordinals[begin]);
                let end = begin + ordinals[begin..].partition_point(|&index| key(index) == group);
                let subjects = self.encode_domain(
                    ordinals[begin..end].iter().map(|&index| row(index).s),
                    memory,
                )?;
                memory.push(
                    &mut self.predicates,
                    Predicate {
                        key: group,
                        rows: (end - begin) as u64,
                        subjects,
                        objects: Domain {
                            offset: 0,
                            bytes: 0,
                            count: 0,
                        },
                        frequencies: None,
                        unbounded_frequencies: Some(group.2) == class_predicate,
                        kinds: (0, 0),
                    },
                )?;
                begin = end;
            }
            ordinals.sort_unstable_by_key(|&index| (key(index), row(index).o));
            begin = 0;
            let mut entry = start;
            while begin < count {
                let group = key(ordinals[begin]);
                let end = begin + ordinals[begin..].partition_point(|&index| key(index) == group);
                let objects = self.encode_domain(
                    ordinals[begin..end].iter().map(|&index| row(index).o),
                    memory,
                )?;
                self.predicates[entry].objects = objects;
                let frequencies = self.frequencies.len();
                let kinds = self.kinds.len();
                let retain_frequencies = objects.count <= SELECTED_OBJECT_FREQUENCIES
                    || Some(group.2) == class_predicate;
                let mut next = begin;
                while next < end {
                    let object = row(ordinals[next]).o;
                    let stop =
                        next + ordinals[next..end].partition_point(|&index| row(index).o == object);
                    let rows = (stop - next) as u64;
                    if retain_frequencies {
                        memory.push(&mut self.frequencies, Frequency { object, rows })?;
                    }
                    let (kind, datatype) = match dictionary.resolve(object) {
                        TermRef::Iri(_) => (0, None),
                        TermRef::Blank { .. } => (1, None),
                        TermRef::Literal { datatype, .. } => (2, Some(datatype)),
                        TermRef::Triple { .. } => (3, None),
                    };
                    for datatype in
                        [None, datatype]
                            .into_iter()
                            .take(if datatype.is_some() { 2 } else { 1 })
                    {
                        if let Some(value) = self.kinds[kinds..]
                            .iter_mut()
                            .find(|value| value.kind == kind && value.datatype == datatype)
                        {
                            value.rows = value
                                .rows
                                .checked_add(rows)
                                .ok_or(StorageError::SizeOverflow)?;
                        } else {
                            memory.push(
                                &mut self.kinds,
                                KindCount {
                                    kind,
                                    datatype,
                                    rows,
                                },
                            )?;
                        }
                    }
                    next = stop;
                }
                self.predicates[entry].frequencies = retain_frequencies
                    .then_some((frequencies, self.frequencies.len() - frequencies));
                self.predicates[entry].kinds = (kinds, self.kinds.len() - kinds);
                entry += 1;
                begin = end;
            }
            self.predicates.sort_unstable_by_key(|entry| entry.key);
        }
        memory.release_vec(ordinals)
    }
}

impl<I: StatisticsId> StatisticsBuilder<I> {
    fn encode_domain<S: Admission + ?Sized>(
        &mut self,
        ids: impl IntoIterator<Item = I>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Domain, StorageError> {
        let offset = self.domains.len();
        let mut previous = None;
        let mut count = 0;
        for id in ids {
            let value = id.statistics_index();
            if previous == Some(value) {
                continue;
            }
            let required = self
                .domains
                .len()
                .checked_add(I::VARINT_BYTES)
                .ok_or(StorageError::SizeOverflow)?;
            let capacity = growth_capacity::<u8>(self.domains.capacity(), required)?;
            memory.reserve(&mut self.domains, capacity)?;
            write_varint(&mut self.domains, value - previous.unwrap_or(0));
            previous = Some(value);
            count += 1;
        }
        Ok(Domain {
            offset,
            bytes: self.domains.len() - offset,
            count,
        })
    }

    pub(super) fn freeze<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<LocalStatistics<I>, StorageError> {
        self.predicates.sort_unstable_by_key(|entry| entry.key);
        let fingerprint = purrdf_hash::fixed::hash_one(&(
            &self.predicates,
            &self.domains,
            &self.frequencies,
            &self.kinds,
        ));
        Ok(LocalStatistics {
            predicates: memory.boxed_slice(self.predicates)?,
            domains: memory.boxed_slice(self.domains)?,
            frequencies: memory.boxed_slice(self.frequencies)?,
            kinds: memory.boxed_slice(self.kinds)?,
            fingerprint,
        })
    }
}

impl LocalStatistics {
    pub(super) fn build<S: Admission + ?Sized>(
        dictionary: &FrozenTerms,
        asserted: (usize, impl Fn(usize) -> QuadIds),
        reifiers: &(usize, impl Fn(usize) -> ReifierRow),
        annotations: (usize, impl Fn(usize) -> QuadIds),
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        memory.try_scope(|memory| {
            let mut builder = StatisticsBuilder::default();
            builder.add(
                Relation::Asserted,
                asserted.0,
                asserted.1,
                dictionary,
                memory,
            )?;
            builder.add(
                Relation::Annotations,
                annotations.0,
                annotations.1,
                dictionary,
                memory,
            )?;
            if reifiers.0 != 0 {
                let reifies = dictionary
                    .term_id_by_iri(purrdf_iri::vocab::rdf::REIFIES)
                    .expect("validated nonempty reifier table interns rdf:reifies");
                for relation in [Relation::Reifiers, Relation::Targets] {
                    builder.add(
                        relation,
                        reifiers.0,
                        |index| {
                            let (s, o, g) = reifiers.1(index);
                            QuadIds {
                                s,
                                p: if relation == Relation::Targets {
                                    dictionary.triple_components(o)[1]
                                } else {
                                    reifies
                                },
                                o,
                                g,
                            }
                        },
                        dictionary,
                        memory,
                    )?;
                }
            }
            builder.freeze(memory)
        })
    }
}

impl<I: StatisticsId> LocalStatistics<I> {
    /// Merge disjoint effective row partitions into the destination dictionary.
    /// Translation precedes sorting and union: source-local numeric order never
    /// substitutes for destination identity. A missing selected frequency stays
    /// unknown, while domains and category counts remain complete and exact.
    pub(super) fn merge_translated<'a, J: StatisticsId, T: Copy, S: Admission + ?Sized>(
        sources: impl IntoIterator<Item = T>,
        summary: impl Fn(T) -> &'a LocalStatistics<J>,
        translate: impl Fn(T, J) -> I,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StatisticsMergeError> {
        memory.try_scope(|memory| {
            let mut keys = Vec::new();
            for source in sources {
                for (index, entry) in summary(source).predicates.iter().enumerate() {
                    let scope = match entry.key.1 {
                        Scope::Any => Scope::Any,
                        Scope::Graph(graph) => Scope::Graph(graph.map(|id| translate(source, id))),
                    };
                    memory.push(
                        &mut keys,
                        (
                            (entry.key.0, scope, translate(source, entry.key.2)),
                            source,
                            index,
                        ),
                    )?;
                }
            }
            keys.sort_unstable_by_key(|entry| entry.0);
            let mut builder = StatisticsBuilder::default();
            let mut ids = Vec::new();
            let mut frequencies: Vec<Frequency<I>> = Vec::new();
            let mut kinds: Vec<KindCount<I>> = Vec::new();
            let mut begin = 0;
            while begin < keys.len() {
                let key = keys[begin].0;
                let end = begin + keys[begin..].partition_point(|entry| entry.0 == key);
                let mut rows = 0_u64;
                let mut complete_frequencies = true;
                let mut unbounded_frequencies = false;
                for &(_, source, index) in &keys[begin..end] {
                    let entry = &summary(source).predicates[index];
                    rows = rows
                        .checked_add(entry.rows)
                        .ok_or(StorageError::SizeOverflow)?;
                    complete_frequencies &= entry.frequencies.is_some();
                    unbounded_frequencies |= entry.unbounded_frequencies;
                }
                // A graph subset with every row is identical to the any-graph
                // relation. Share all sealed ranges rather than doubling them.
                if matches!(key.1, Scope::Graph(_))
                    && let Ok(index) = builder.predicates.binary_search_by_key(
                        &(key.0, Scope::Any, key.2),
                        |entry: &Predicate<I>| entry.key,
                    )
                    && builder.predicates[index].rows == rows
                {
                    let alias = Predicate {
                        key,
                        ..builder.predicates[index]
                    };
                    memory.push(&mut builder.predicates, alias)?;
                    begin = end;
                    continue;
                }
                let mut domains = [Domain {
                    offset: 0,
                    bytes: 0,
                    count: 0,
                }; 2];
                for (position, domain) in domains.iter_mut().enumerate() {
                    ids.clear();
                    for &(_, source, index) in &keys[begin..end] {
                        let statistics = summary(source);
                        let entry = &statistics.predicates[index];
                        let local_domain = if position == 0 {
                            entry.subjects
                        } else {
                            entry.objects
                        };
                        for value in statistics.read_domain(local_domain) {
                            let id = J::statistics_id(value?)?;
                            memory.push(&mut ids, translate(source, id))?;
                        }
                    }
                    ids.sort_unstable();
                    ids.dedup();
                    *domain = builder.encode_domain(ids.iter().copied(), memory)?;
                }
                frequencies.clear();
                kinds.clear();
                complete_frequencies &=
                    domains[1].count <= SELECTED_OBJECT_FREQUENCIES || unbounded_frequencies;
                for &(_, source, index) in &keys[begin..end] {
                    let statistics = summary(source);
                    let entry = &statistics.predicates[index];
                    if complete_frequencies {
                        let (offset, count) = entry.frequencies.expect("complete frequencies");
                        for value in &statistics.frequencies[offset..offset + count] {
                            memory.push(
                                &mut frequencies,
                                Frequency {
                                    object: translate(source, value.object),
                                    rows: value.rows,
                                },
                            )?;
                        }
                    }
                    let (offset, count) = entry.kinds;
                    for value in &statistics.kinds[offset..offset + count] {
                        memory.push(
                            &mut kinds,
                            KindCount {
                                kind: value.kind,
                                datatype: value.datatype.map(|id| translate(source, id)),
                                rows: value.rows,
                            },
                        )?;
                    }
                }
                frequencies.sort_unstable_by_key(|value| value.object);
                kinds.sort_unstable_by_key(|value| (value.kind, value.datatype));
                let frequency_begin = builder.frequencies.len();
                for &value in &frequencies {
                    if let Some(previous) = builder.frequencies[frequency_begin..].last_mut()
                        && previous.object == value.object
                    {
                        previous.rows = previous
                            .rows
                            .checked_add(value.rows)
                            .ok_or(StorageError::SizeOverflow)?;
                    } else {
                        memory.push(&mut builder.frequencies, value)?;
                    }
                }
                let kind_begin = builder.kinds.len();
                for &value in &kinds {
                    if let Some(previous) = builder.kinds[kind_begin..].last_mut()
                        && (previous.kind, previous.datatype) == (value.kind, value.datatype)
                    {
                        previous.rows = previous
                            .rows
                            .checked_add(value.rows)
                            .ok_or(StorageError::SizeOverflow)?;
                    } else {
                        memory.push(&mut builder.kinds, value)?;
                    }
                }
                memory.push(
                    &mut builder.predicates,
                    Predicate {
                        key,
                        rows,
                        subjects: domains[0],
                        objects: domains[1],
                        frequencies: complete_frequencies.then_some((
                            frequency_begin,
                            builder.frequencies.len() - frequency_begin,
                        )),
                        kinds: (kind_begin, builder.kinds.len() - kind_begin),
                        unbounded_frequencies,
                    },
                )?;
                begin = end;
            }
            memory.release_vec(keys)?;
            memory.release_vec(ids)?;
            memory.release_vec(frequencies)?;
            memory.release_vec(kinds)?;
            Ok(builder.freeze(memory)?)
        })
    }

    pub(super) fn clone_admitted<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        memory.try_scope(|memory| {
            let predicates = memory.collect(self.predicates.iter().copied())?;
            let predicates = memory.boxed_slice(predicates)?;
            let domains = memory.collect(self.domains.iter().copied())?;
            let domains = memory.boxed_slice(domains)?;
            let frequencies = memory.collect(self.frequencies.iter().copied())?;
            let frequencies = memory.boxed_slice(frequencies)?;
            let kinds = memory.collect(self.kinds.iter().copied())?;
            Ok(Self {
                predicates,
                domains,
                frequencies,
                kinds: memory.boxed_slice(kinds)?,
                fingerprint: self.fingerprint,
            })
        })
    }

    /// Canonical words for the enclosing summary's existing digest. Pointer
    /// width, native struct padding and the table hasher never enter this stream.
    pub(super) fn visit_words(&self, mut emit: impl FnMut(u64)) {
        emit(self.predicates.len() as u64);
        for entry in &self.predicates {
            emit(entry.key.0 as u64);
            match entry.key.1 {
                Scope::Any => emit(0),
                Scope::Graph(None) => emit(1),
                Scope::Graph(Some(graph)) => {
                    emit(2);
                    emit(graph.statistics_index());
                }
            }
            emit(entry.key.2.statistics_index());
            emit(entry.rows);
            for domain in [entry.subjects, entry.objects] {
                emit(domain.offset as u64);
                emit(domain.bytes as u64);
                emit(domain.count as u64);
            }
            emit(u64::from(entry.frequencies.is_some()));
            emit(u64::from(entry.unbounded_frequencies));
            if let Some((offset, count)) = entry.frequencies {
                emit(offset as u64);
                emit(count as u64);
            }
            emit(entry.kinds.0 as u64);
            emit(entry.kinds.1 as u64);
        }
        emit(self.domains.len() as u64);
        for chunk in self.domains.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            emit(u64::from_le_bytes(word));
        }
        emit(self.frequencies.len() as u64);
        for value in &self.frequencies {
            emit(value.object.statistics_index());
            emit(value.rows);
        }
        emit(self.kinds.len() as u64);
        for value in &self.kinds {
            emit(u64::from(value.kind));
            emit(u64::from(value.datatype.is_some()));
            if let Some(datatype) = value.datatype {
                emit(datatype.statistics_index());
            }
            emit(value.rows);
        }
    }

    fn entry(
        &self,
        relation: Relation,
        predicate: I,
        graph: GraphMatch<I>,
    ) -> Option<&Predicate<I>> {
        let scope = match graph {
            GraphMatch::Any => Scope::Any,
            GraphMatch::Default => Scope::Graph(None),
            GraphMatch::Named(graph) => Scope::Graph(Some(graph)),
        };
        self.predicates
            .binary_search_by_key(&(relation, scope, predicate), |entry| entry.key)
            .ok()
            .map(|index| &self.predicates[index])
    }

    pub(super) fn predicate(
        &self,
        relation: Relation,
        predicate: I,
        graph: GraphMatch<I>,
    ) -> PredicateStatistics {
        let entry = self.entry(relation, predicate, graph);
        PredicateStatistics {
            rows: exact(entry.map_or(0, |entry| entry.rows)),
            subjects: Some(exact(entry.map_or(0, |entry| entry.subjects.count as u64))),
            objects: Some(exact(entry.map_or(0, |entry| entry.objects.count as u64))),
        }
    }

    pub(super) fn frequency(
        &self,
        relation: Relation,
        predicate: I,
        object: I,
        graph: GraphMatch<I>,
    ) -> Option<StatisticCount> {
        let Some(entry) = self.entry(relation, predicate, graph) else {
            return Some(exact(0));
        };
        let (offset, count) = entry.frequencies?;
        let values = &self.frequencies[offset..offset + count];
        Some(exact(
            values
                .binary_search_by_key(&object, |value| value.object)
                .ok()
                .map_or(0, |index| values[index].rows),
        ))
    }

    pub(super) fn kind(
        &self,
        relation: Relation,
        predicate: I,
        kind: RdfTermKind,
        datatype: Option<I>,
        graph: GraphMatch<I>,
    ) -> StatisticCount {
        let Some(entry) = self.entry(relation, predicate, graph) else {
            return exact(0);
        };
        let (offset, count) = entry.kinds;
        exact(
            self.kinds[offset..offset + count]
                .iter()
                .find(|value| value.kind == kind_key(kind) && value.datatype == datatype)
                .map_or(0, |value| value.rows),
        )
    }

    pub(super) fn domain(
        &self,
        relation: Relation,
        predicate: I,
        graph: GraphMatch<I>,
        subject: bool,
    ) -> DeltaListRef<'_> {
        let domain = self.entry(relation, predicate, graph).map(|entry| {
            if subject {
                entry.subjects
            } else {
                entry.objects
            }
        });
        match domain {
            Some(domain) => self.read_domain(domain),
            None => DeltaListRef::new(&[], 0),
        }
    }

    fn read_domain(&self, domain: Domain) -> DeltaListRef<'_> {
        DeltaListRef::new(
            &self.domains[domain.offset..domain.offset + domain.bytes],
            domain.count,
        )
    }

    pub(super) fn retained_bytes(&self) -> Option<usize> {
        size_of_val(self.predicates.as_ref())
            .checked_add(self.domains.len())?
            .checked_add(size_of_val(self.frequencies.as_ref()))?
            .checked_add(size_of_val(self.kinds.as_ref()))
    }
}

fn relation(stream: crate::StatisticsStream) -> Relation {
    match stream {
        crate::StatisticsStream::Asserted => Relation::Asserted,
        crate::StatisticsStream::Reifiers => Relation::Reifiers,
        crate::StatisticsStream::Annotations => Relation::Annotations,
    }
}

impl<I: StatisticsId> crate::SnapshotStatistics<I> for LocalStatistics<I> {
    fn source(&self) -> &'static str {
        I::SOURCE
    }

    fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    fn stream_rows(
        &self,
        graph: GraphMatch<I>,
        stream: crate::StatisticsStream,
    ) -> Option<StatisticCount> {
        let scope = match graph {
            GraphMatch::Any => Scope::Any,
            GraphMatch::Default => Scope::Graph(None),
            GraphMatch::Named(graph) => Scope::Graph(Some(graph)),
        };
        self.predicates
            .iter()
            .filter(|entry| entry.key.0 == relation(stream) && entry.key.1 == scope)
            .try_fold(0_u64, |rows, entry| rows.checked_add(entry.rows))
            .map(exact)
    }

    fn predicate(
        &self,
        predicate: I,
        graph: GraphMatch<I>,
        stream: crate::StatisticsStream,
    ) -> Option<PredicateStatistics> {
        Some(Self::predicate(self, relation(stream), predicate, graph))
    }

    fn predicate_object(
        &self,
        predicate: I,
        object: I,
        graph: GraphMatch<I>,
        stream: crate::StatisticsStream,
    ) -> Option<StatisticCount> {
        self.frequency(relation(stream), predicate, object, graph)
    }

    fn predicate_object_kind(
        &self,
        predicate: I,
        kind: RdfTermKind,
        datatype: Option<I>,
        graph: GraphMatch<I>,
        stream: crate::StatisticsStream,
    ) -> Option<StatisticCount> {
        Some(self.kind(relation(stream), predicate, kind, datatype, graph))
    }

    fn reifier_target_predicate(
        &self,
        predicate: I,
        graph: GraphMatch<I>,
    ) -> Option<PredicateStatistics> {
        Some(Self::predicate(self, Relation::Targets, predicate, graph))
    }

    fn visit_distinct(
        &self,
        predicate: I,
        graph: GraphMatch<I>,
        stream: crate::StatisticsStream,
        position: crate::StatisticsPosition,
        visit: &mut dyn FnMut(I) -> std::ops::ControlFlow<()>,
    ) -> Result<bool, super::pack::bits::PackBitsError> {
        for value in self.domain(
            relation(stream),
            predicate,
            graph,
            position == crate::StatisticsPosition::Subject,
        ) {
            let id = I::statistics_id(value?)?;
            if visit(id).is_break() {
                break;
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::ops::ControlFlow;
    use std::sync::Arc;

    use super::*;
    use crate::{
        DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral, StatisticsPosition,
        StatisticsStream, TripleDataset,
    };

    fn fixture(named: bool) -> (Arc<RdfDataset>, Vec<TermId>, Vec<GraphMatch>) {
        fixture_partition(named, 0, 0)
    }

    fn fixture_partition(
        named: bool,
        partition: u8,
        extra_objects: u8,
    ) -> (Arc<RdfDataset>, Vec<TermId>, Vec<GraphMatch>) {
        let mut builder = RdfDatasetBuilder::new();
        let names = [
            "a",
            "b",
            "p",
            "annotation",
            "rare",
            "common",
            "g",
            "r",
            "r2",
        ];
        let mut intern = |name: &str| {
            let suffix = if partition != 0 && matches!(name, "a" | "b" | "r" | "r2") {
                format!("/{partition}")
            } else {
                String::new()
            };
            builder
                .intern_iri(&format!("http://example.org/{name}{suffix}"))
                .unwrap()
        };
        if !partition.is_multiple_of(2) {
            for name in names.iter().rev() {
                intern(name);
            }
        }
        let ids: Vec<_> = names.into_iter().map(intern).collect();
        let [a, b, p, annotation, rare, common, g, r, r2] = ids[..] else {
            unreachable!()
        };
        let rdf_type = builder.intern_iri(purrdf_iri::vocab::rdf::TYPE).unwrap();
        let number = builder
            .intern_literal(RdfLiteral::typed("7", purrdf_xsd::datatype::XSD_INTEGER))
            .unwrap();
        let string = builder.intern_literal(RdfLiteral::simple("seven")).unwrap();
        let triple = builder.intern_triple(a, p, b).unwrap();
        for (s, predicate, o, graph) in [
            (a, p, number, None),
            (a, p, string, None),
            (b, p, number, named.then_some(g)),
            (a, rdf_type, rare, None),
            (b, rdf_type, common, named.then_some(g)),
        ] {
            builder.push_quad(s, predicate, o, graph).unwrap();
        }
        for index in 0..extra_objects {
            let object = builder
                .intern_iri(&format!("http://example.org/extra/{partition}/{index}"))
                .unwrap();
            builder.push_quad(a, p, object, None).unwrap();
            builder.push_quad(a, rdf_type, object, None).unwrap();
        }
        builder.push_reifier(r, triple).unwrap();
        builder
            .push_reifier_in_graph(r2, triple, named.then_some(g))
            .unwrap();
        builder.push_annotation(r, annotation, number).unwrap();
        builder
            .push_annotation_in_graph(r2, annotation, number, named.then_some(g))
            .unwrap();
        let reifies = builder.intern_iri(purrdf_iri::vocab::rdf::REIFIES).unwrap();
        let dataset = builder.freeze().unwrap();
        (
            dataset,
            vec![p, annotation, rdf_type, reifies, rare],
            vec![GraphMatch::Any, GraphMatch::Default, GraphMatch::Named(g)],
        )
    }

    fn assert_rows<D: DatasetView>(view: &D, predicates: &[D::Id], scopes: &[GraphMatch<D::Id>]) {
        let statistics = view.statistics().unwrap();
        for &graph in scopes {
            let total = view
                .quads()
                .chain(view.reifier_quads())
                .chain(view.annotation_quads())
                .filter(|row| graph.matches(row.g))
                .count();
            assert_eq!(
                statistics.pattern(crate::ProbePattern {
                    s: None,
                    p: None,
                    o: None,
                    g: graph
                }),
                Some(exact(total as u64))
            );
        }
        let class = view
            .term_id_by_value(&crate::TermValue::iri(purrdf_iri::vocab::rdf::TYPE))
            .unwrap();
        for stream in [
            StatisticsStream::Asserted,
            StatisticsStream::Reifiers,
            StatisticsStream::Annotations,
        ] {
            let rows: Vec<_> = match stream {
                StatisticsStream::Asserted => view.quads().collect(),
                StatisticsStream::Reifiers => view.reifier_quads().collect(),
                StatisticsStream::Annotations => view.annotation_quads().collect(),
            };
            for &graph in scopes {
                assert_eq!(
                    statistics.stream_rows(graph, stream),
                    Some(exact(
                        rows.iter().filter(|row| graph.matches(row.g)).count() as u64
                    ))
                );
            }
            for &predicate in predicates {
                for &graph in scopes {
                    let selected: Vec<_> = rows
                        .iter()
                        .filter(|row| row.p == predicate && graph.matches(row.g))
                        .collect();
                    let summary = statistics.predicate(predicate, graph, stream).unwrap();
                    assert_eq!(summary.rows, exact(selected.len() as u64));
                    for (position, expected, actual) in [
                        (
                            StatisticsPosition::Subject,
                            selected.iter().map(|row| row.s).collect::<BTreeSet<_>>(),
                            summary.subjects,
                        ),
                        (
                            StatisticsPosition::Object,
                            selected.iter().map(|row| row.o).collect::<BTreeSet<_>>(),
                            summary.objects,
                        ),
                    ] {
                        assert_eq!(actual, Some(exact(expected.len() as u64)));
                        let mut domain = Vec::new();
                        assert!(
                            statistics
                                .visit_distinct(predicate, graph, stream, position, &mut |id| {
                                    domain.push(id);
                                    ControlFlow::Continue(())
                                })
                                .unwrap()
                        );
                        assert_eq!(domain, expected.into_iter().collect::<Vec<_>>());
                    }
                    for object in rows
                        .iter()
                        .map(|row| row.o)
                        .chain(predicates.iter().copied())
                    {
                        let actual = statistics.predicate_object(predicate, object, graph, stream);
                        if actual.is_none() {
                            assert_ne!(Some(predicate), class, "class counts are never capped");
                            assert!(
                                summary.objects.unwrap().value > SELECTED_OBJECT_FREQUENCIES as u64
                            );
                        } else {
                            assert_eq!(
                                actual,
                                Some(exact(
                                    selected.iter().filter(|row| row.o == object).count() as u64
                                ))
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn native_summaries_match_actual_graph_streams_and_sorted_domains() {
        let (dataset, predicates, scopes) = fixture(true);
        assert_rows(&*dataset, &predicates, &scopes);
        let stats = dataset.statistics().unwrap();
        let target = stats
            .reifier_target_predicate(predicates[0], GraphMatch::Any)
            .unwrap();
        assert_eq!(
            (target.rows, target.subjects, target.objects),
            (exact(2), Some(exact(2)), Some(exact(1)))
        );
        let integer = dataset
            .term_id_by_iri(purrdf_xsd::datatype::XSD_INTEGER)
            .unwrap();
        for (kind, datatype, expected) in [
            (RdfTermKind::Literal, None, 3),
            (RdfTermKind::Literal, Some(integer), 2),
            (RdfTermKind::Iri, None, 0),
        ] {
            assert_eq!(
                stats.predicate_object_kind(
                    predicates[0],
                    kind,
                    datatype,
                    GraphMatch::Any,
                    StatisticsStream::Asserted
                ),
                Some(exact(expected))
            );
        }
    }

    #[test]
    fn paged_metadata_is_exact_shared_on_reopen_and_requires_no_query_io() {
        use crate::{
            FallibleDatasetView, InMemoryPageProvider, PagedDataset, PagedQueryLimits,
            ViewOperationStatus,
        };
        let (native, predicates, scopes) = fixture(true);
        let provider = Arc::new(InMemoryPageProvider::new(vec![native.clone()]));
        let paged = PagedDataset::from_provider(provider.clone()).unwrap();
        let global = |id| {
            paged
                .term_id_by_value(&native.as_ref().term_value(id))
                .unwrap()
                .unwrap()
        };
        let predicates: Vec<_> = predicates.into_iter().map(global).collect();
        let scopes: Vec<_> = scopes
            .into_iter()
            .map(|scope| match scope {
                GraphMatch::Any => GraphMatch::Any,
                GraphMatch::Default => GraphMatch::Default,
                GraphMatch::Named(id) => GraphMatch::Named(global(id)),
            })
            .collect();
        let (dictionary, generation, parts) = paged.to_parts();
        let warm = PagedDataset::from_parts(dictionary, provider, generation, parts).unwrap();
        assert!(std::ptr::addr_eq(
            paged.statistics().unwrap(),
            warm.statistics().unwrap()
        ));
        let operational = warm.query_view(PagedQueryLimits::new(0, 0));
        let statistics = operational.statistics().unwrap();
        for predicate in &predicates {
            for scope in &scopes {
                assert_eq!(
                    statistics.predicate(*predicate, *scope, StatisticsStream::Annotations),
                    paged.statistics().unwrap().predicate(
                        *predicate,
                        *scope,
                        StatisticsStream::Annotations
                    ),
                );
            }
        }
        let ViewOperationStatus::Ready { evidence } = operational.operation_status() else {
            panic!("metadata requires no page admission");
        };
        assert_eq!(evidence.consumed_pages, 0);
        assert_eq!(evidence.requested_pages, []);
        assert_rows(&paged, &predicates, &scopes);
        assert_rows(&warm, &predicates, &scopes);
    }

    #[test]
    fn translated_domains_keep_full_width_ids_and_sort_after_translation() {
        use crate::{GlobalTermId, SnapshotStatistics};
        let (native, predicates, _) = fixture(false);
        let base = u64::from(u32::MAX) + 100;
        let translate = |id: TermId| GlobalTermId::from_index(base - id.index() as u64).unwrap();
        let mut admission = purrdf_lex::allocation::Resident;
        let merged = LocalStatistics::<GlobalTermId>::merge_translated(
            [&native],
            |source| source.predicate_statistics(),
            |_, id| translate(id),
            &mut Memory::new(&mut admission),
        )
        .unwrap();
        for predicate in predicates {
            for position in [StatisticsPosition::Subject, StatisticsPosition::Object] {
                let mut expected = Vec::new();
                native
                    .statistics()
                    .unwrap()
                    .visit_distinct(
                        predicate,
                        GraphMatch::Any,
                        StatisticsStream::Asserted,
                        position,
                        &mut |id| {
                            expected.push(translate(id));
                            ControlFlow::Continue(())
                        },
                    )
                    .unwrap();
                expected.sort_unstable();
                let mut actual = Vec::new();
                merged
                    .visit_distinct(
                        translate(predicate),
                        GraphMatch::Any,
                        StatisticsStream::Asserted,
                        position,
                        &mut |id| {
                            actual.push(id);
                            ControlFlow::Continue(())
                        },
                    )
                    .unwrap();
                assert_eq!(actual, expected);
                assert!(actual.iter().all(|id| id.index() > u64::from(u32::MAX)));
            }
        }
    }

    use purrdf_testkit::prop::prelude::*;

    prop_test! {
        #[test]
        fn property_translated_two_page_summaries_match_one_combined_snapshot(
            first in 0_u8..16, delta in 1_u8..16, named in any::<bool>(), extra in 0_u8..70,
        ) {
            use crate::{InMemoryPageProvider, PagedDataset};
            let (left, predicates, scopes) = fixture_partition(named, first, extra);
            let (right, _, _) = fixture_partition(!named, first + delta, extra);
            let mut builder = RdfDatasetBuilder::new();
            builder.push_dataset(&left).unwrap();
            builder.push_dataset(&right).unwrap();
            let combined = builder.freeze().unwrap();
            let paged = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(
                vec![left.clone(), right],
            ))).unwrap();
            let native_ids: Vec<_> = predicates.iter().map(|&id|
                combined.as_ref().term_id_by_value(&left.as_ref().term_value(id)).unwrap()).collect();
            let global_ids: Vec<_> = predicates.iter().map(|&id|
                paged.term_id_by_value(&left.as_ref().term_value(id)).unwrap().unwrap()).collect();
            let mut native_scopes = Vec::new();
            let mut global_scopes = Vec::new();
            for scope in scopes {
                match scope {
                    GraphMatch::Any => { native_scopes.push(GraphMatch::Any); global_scopes.push(GraphMatch::Any); }
                    GraphMatch::Default => { native_scopes.push(GraphMatch::Default); global_scopes.push(GraphMatch::Default); }
                    GraphMatch::Named(id) => {
                        let value = left.as_ref().term_value(id);
                        native_scopes.push(GraphMatch::Named(combined.as_ref().term_id_by_value(&value).unwrap()));
                        global_scopes.push(GraphMatch::Named(paged.term_id_by_value(&value).unwrap().unwrap()));
                    }
                }
            }
            assert_rows(&*combined, &native_ids, &native_scopes);
            assert_rows(&paged, &global_ids, &global_scopes);
            for (&native_id, &global_id) in native_ids.iter().zip(&global_ids) {
                for (&native_scope, &global_scope) in native_scopes.iter().zip(&global_scopes) {
                    prop_assert_eq!(
                        combined.statistics().unwrap().reifier_target_predicate(native_id, native_scope),
                        paged.statistics().unwrap().reifier_target_predicate(global_id, global_scope),
                    );
                    for stream in [StatisticsStream::Asserted, StatisticsStream::Reifiers, StatisticsStream::Annotations] {
                        prop_assert_eq!(
                            combined.statistics().unwrap().predicate(native_id, native_scope, stream),
                            paged.statistics().unwrap().predicate(global_id, global_scope, stream),
                        );
                        for kind in [RdfTermKind::Iri, RdfTermKind::Literal, RdfTermKind::BlankNode, RdfTermKind::Triple] {
                            prop_assert_eq!(
                                combined.statistics().unwrap().predicate_object_kind(native_id, kind, None, native_scope, stream),
                                paged.statistics().unwrap().predicate_object_kind(global_id, kind, None, global_scope, stream),
                            );
                        }
                        let objects: BTreeSet<_> = match stream {
                            StatisticsStream::Asserted => combined.quads().map(|row| row.o).collect(),
                            StatisticsStream::Reifiers => combined.reifier_quads().map(|row| row.o).collect(),
                            StatisticsStream::Annotations => combined.annotation_quads().map(|row| row.o).collect(),
                        };
                        for object in objects {
                            let global_object = paged.term_id_by_value(&combined.as_ref().term_value(object)).unwrap().unwrap();
                            prop_assert_eq!(
                                combined.statistics().unwrap().predicate_object(native_id, object, native_scope, stream),
                                paged.statistics().unwrap().predicate_object(global_id, global_object, global_scope, stream),
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn compact_statistics_preserve_ids_and_refuse_named_graph_metadata() {
        let (dataset, predicates, scopes) = fixture(false);
        assert_rows(&*dataset, &predicates, &scopes);
        let compact = TripleDataset::try_from(Arc::try_unwrap(dataset).unwrap()).unwrap();
        let read = compact.read().unwrap();
        assert_rows(&read, &predicates, &scopes[..2]);
        assert!(
            read.statistics()
                .unwrap()
                .predicate(predicates[0], scopes[2], StatisticsStream::Asserted)
                .is_none()
        );
        assert_eq!(
            read.read_error(),
            Some(super::super::triple::TripleReadError::NamedGraphsUnsupported)
        );
        for subject in [None, Some(predicates[0])] {
            for predicate in [None, Some(predicates[0])] {
                let fresh = compact.read().unwrap();
                assert!(
                    fresh
                        .statistics()
                        .unwrap()
                        .pattern(crate::ProbePattern {
                            s: subject,
                            p: predicate,
                            o: None,
                            g: scopes[2],
                        })
                        .is_none()
                );
                assert_eq!(
                    fresh.read_error(),
                    Some(super::super::triple::TripleReadError::NamedGraphsUnsupported)
                );
            }
        }
    }
}
