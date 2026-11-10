// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sealed predicate statistics in one validated local dictionary generation.
//! Transient ordinal sorts and retained metadata use the original freeze memory.
//! Distinct domains use the pack codec's delta lists, not another integer codec.

use core::hash::{Hash, Hasher as _};
use purrdf_lex::allocation::{Admission, Memory, StorageError, growth_capacity};
use purrdf_lex::walk::{try_dedup_by, try_equal_range_by, try_sort_unstable_by};

use super::dataset::{QuadIds, ReifierRow, TermRef};
use super::frozen_terms::FrozenTerms;
use super::pack::bits::{DeltaListRef, write_varint};
use super::source_directory::{SourceDirectory, SourceRecord};
use super::term::TermId;
use crate::{
    GraphMatch, PredicateStatistics, RdfTermKind, StatisticCount, StatisticsAccuracy, ViewTermId,
};

mod codec;
mod source;
use crate::source::SourceSummary;
pub(super) use codec::{
    PREDICATE_RECORD_BYTES, PredicateRecord, StatisticsSection, decode_frequency, decode_kind,
};

// Keep complete frequencies for low-cardinality objects and every rdf:type
// class. Higher-cardinality predicates retain exact domains and kind counts;
// a missing selected frequency remains unknown, never a fabricated zero.
const SELECTED_OBJECT_FREQUENCIES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Relation {
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
    source: SourceSummary,
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
    // Native addresses are exposed only by LocalStatistics<TermId>. A merged
    // global statistics table has no single physical row address space.
    directory: SourceDirectory,
    // Without annotations, normalized keys are exactly the physical keys.
    // Reuse their original directory instead of retaining a second copy.
    effective_directory: Option<SourceDirectory>,
    source_union: purrdf_hash::multiset::Multiset256,
    statement_count: u128,
}
purrdf_hash::debug_non_exhaustive!([I] LocalStatistics<I> { fingerprint });

pub(super) struct StatisticsBuilder<I = TermId> {
    predicates: Vec<Predicate<I>>,
    domains: Vec<u8>,
    frequencies: Vec<Frequency<I>>,
    kinds: Vec<KindCount<I>>,
    records: Vec<SourceRecord>,
    union: Vec<SourceRecord>,
    normalize_annotations: bool,
}

// Deriving Default would unnecessarily require an empty identity value.
impl<I> Default for StatisticsBuilder<I> {
    fn default() -> Self {
        Self {
            predicates: Vec::new(),
            domains: Vec::new(),
            frequencies: Vec::new(),
            kinds: Vec::new(),
            records: Vec::new(),
            union: Vec::new(),
            normalize_annotations: false,
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

pub(super) fn exact(value: u64) -> StatisticCount {
    StatisticCount {
        value,
        accuracy: StatisticsAccuracy::Exact,
    }
}

pub(super) fn kind_key(kind: RdfTermKind) -> u8 {
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
            return Ok::<_, StorageError>(());
        }
        let class_predicate =
            dictionary.term_id_by_iri_with_memory(purrdf_iri::vocab::rdf::TYPE, memory)?;
        let mut ordinals = Vec::new();
        let mut pending = Vec::new();
        let mut source_row = crate::source::SourceRowHasher::default();
        let result = (|| {
            memory.admission_mut().checkpoint(0)?;
            memory.reserve(&mut ordinals, count)?;
            for ordinal in 0..count {
                memory.admission_mut().checkpoint(1)?;
                ordinals.push(ordinal);
            }
            let one_graph = if memory.admission().progress_enabled() {
                let mut one_graph = true;
                for index in 1..count {
                    memory.admission_mut().checkpoint(1)?;
                    if row(index).g != row(0).g {
                        one_graph = false;
                        break;
                    }
                }
                one_graph
            } else {
                (1..count).all(|index| row(index).g == row(0).g)
            };
            for graph_scoped in [false, true] {
                if graph_scoped && one_graph {
                    let range = if memory.admission().progress_enabled() {
                        try_equal_range_by(&self.predicates, |entry| {
                            memory.admission_mut().checkpoint(1)?;
                            Ok::<_, StorageError>(entry.key.0.cmp(&relation))
                        })?
                    } else {
                        self.predicates
                            .partition_point(|entry| entry.key.0 < relation)
                            ..self
                                .predicates
                                .partition_point(|entry| entry.key.0 <= relation)
                    };
                    for index in range {
                        memory.admission_mut().checkpoint(1)?;
                        let mut alias = self.predicates[index];
                        alias.key.1 = Scope::Graph(row(0).g);
                        memory.push_copy_with_progress(&mut self.predicates, alias)?;
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
                if memory.admission().progress_enabled() {
                    try_sort_unstable_by(&mut ordinals, |&left, &right| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(
                            (key(left), row(left).s).cmp(&(key(right), row(right).s)),
                        )
                    })?;
                } else {
                    ordinals.sort_unstable_by_key(|&index| (key(index), row(index).s));
                }
                let mut begin = 0;
                while begin < count {
                    memory.admission_mut().checkpoint(1)?;
                    let group = key(ordinals[begin]);
                    let run = if memory.admission().progress_enabled() {
                        try_equal_range_by(&ordinals[begin..], |&index| {
                            memory.admission_mut().checkpoint(1)?;
                            Ok::<_, StorageError>(key(index).cmp(&group))
                        })?
                        .end
                    } else {
                        ordinals[begin..].partition_point(|&index| key(index) == group)
                    };
                    let end = begin.checked_add(run).ok_or(StorageError::SizeOverflow)?;
                    let subjects = self.encode_domain(
                        ordinals[begin..end].iter().map(|&index| row(index).s),
                        memory,
                    )?;
                    let mut source = SourceSummary::default();
                    if graph_scoped || one_graph {
                        for &ordinal in &ordinals[begin..end] {
                            memory.admission_mut().checkpoint(1)?;
                            let (digest, union) = source.add_row(
                                relation,
                                row(ordinal),
                                dictionary,
                                &mut pending,
                                &mut source_row,
                                memory,
                            )?;
                            let stream = match relation {
                                Relation::Asserted => Some(crate::StatisticsStream::Asserted),
                                Relation::Reifiers => Some(crate::StatisticsStream::Reifiers),
                                Relation::Annotations => Some(crate::StatisticsStream::Annotations),
                                Relation::Targets => None,
                            };
                            if let Some(stream) = stream {
                                let record = SourceRecord {
                                    digest,
                                    ordinal: u32::try_from(ordinal)
                                        .map_err(|_| StorageError::SizeOverflow)?,
                                    stream,
                                };
                                memory.push_copy_with_progress(&mut self.records, record)?;
                                if self.normalize_annotations {
                                    memory.push_copy_with_progress(
                                        &mut self.union,
                                        SourceRecord {
                                            digest: union,
                                            ..record
                                        },
                                    )?;
                                }
                            }
                        }
                    }
                    memory.push_copy_with_progress(
                        &mut self.predicates,
                        Predicate {
                            key: group,
                            rows: (end - begin) as u64,
                            source,
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
                if memory.admission().progress_enabled() {
                    try_sort_unstable_by(&mut ordinals, |&left, &right| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(
                            (key(left), row(left).o).cmp(&(key(right), row(right).o)),
                        )
                    })?;
                } else {
                    ordinals.sort_unstable_by_key(|&index| (key(index), row(index).o));
                }
                begin = 0;
                let mut entry = start;
                while begin < count {
                    memory.admission_mut().checkpoint(1)?;
                    let group = key(ordinals[begin]);
                    let run = if memory.admission().progress_enabled() {
                        try_equal_range_by(&ordinals[begin..], |&index| {
                            memory.admission_mut().checkpoint(1)?;
                            Ok::<_, StorageError>(key(index).cmp(&group))
                        })?
                        .end
                    } else {
                        ordinals[begin..].partition_point(|&index| key(index) == group)
                    };
                    let end = begin.checked_add(run).ok_or(StorageError::SizeOverflow)?;
                    let objects = self.encode_domain(
                        ordinals[begin..end].iter().map(|&index| row(index).o),
                        memory,
                    )?;
                    memory.admission_mut().checkpoint(1)?;
                    self.predicates[entry].objects = objects;
                    let frequencies = self.frequencies.len();
                    let kinds = self.kinds.len();
                    let retain_frequencies = objects.count <= SELECTED_OBJECT_FREQUENCIES
                        || Some(group.2) == class_predicate;
                    let mut next = begin;
                    while next < end {
                        memory.admission_mut().checkpoint(1)?;
                        let object = row(ordinals[next]).o;
                        let run = if memory.admission().progress_enabled() {
                            try_equal_range_by(&ordinals[next..end], |&index| {
                                memory.admission_mut().checkpoint(1)?;
                                Ok::<_, StorageError>(row(index).o.cmp(&object))
                            })?
                            .end
                        } else {
                            purrdf_lex::walk::partition_prefix(&ordinals[next..end], |&index| {
                                row(index).o == object
                            })
                        };
                        let stop = next.checked_add(run).ok_or(StorageError::SizeOverflow)?;
                        let rows = (stop - next) as u64;
                        if retain_frequencies {
                            memory.push_copy_with_progress(
                                &mut self.frequencies,
                                Frequency { object, rows },
                            )?;
                        }
                        memory.admission_mut().checkpoint(1)?;
                        let (kind, datatype) = match dictionary.resolve(object) {
                            TermRef::Iri(_) => (0, None),
                            TermRef::Blank { .. } => (1, None),
                            TermRef::Literal { datatype, .. } => (2, Some(datatype)),
                            TermRef::Triple { .. } => (3, None),
                        };
                        for datatype in [None, datatype].into_iter().take(if datatype.is_some() {
                            2
                        } else {
                            1
                        }) {
                            let mut found = None;
                            for (index, value) in self.kinds.iter().enumerate().skip(kinds) {
                                memory.admission_mut().checkpoint(1)?;
                                if value.kind == kind && value.datatype == datatype {
                                    found = Some(index);
                                    break;
                                }
                            }
                            memory.admission_mut().checkpoint(1)?;
                            if let Some(index) = found {
                                let value = &mut self.kinds[index];
                                value.rows = value
                                    .rows
                                    .checked_add(rows)
                                    .ok_or(StorageError::SizeOverflow)?;
                            } else {
                                memory.push_copy_with_progress(
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
                    memory.admission_mut().checkpoint(1)?;
                    self.predicates[entry].frequencies = retain_frequencies
                        .then_some((frequencies, self.frequencies.len() - frequencies));
                    self.predicates[entry].kinds = (kinds, self.kinds.len() - kinds);
                    entry += 1;
                    begin = end;
                }
                if memory.admission().progress_enabled() {
                    try_sort_unstable_by(&mut self.predicates, |left, right| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(left.key.cmp(&right.key))
                    })?;
                } else {
                    self.predicates.sort_unstable_by_key(|entry| entry.key);
                }
            }
            if !one_graph {
                // Any-graph summaries combine exact graph partitions; their rows
                // are not hashed for a second time.
                let range = if memory.admission().progress_enabled() {
                    try_equal_range_by(&self.predicates, |entry| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(entry.key.0.cmp(&relation))
                    })?
                } else {
                    self.predicates
                        .partition_point(|entry| entry.key.0 < relation)
                        ..self
                            .predicates
                            .partition_point(|entry| entry.key.0 <= relation)
                };
                for index in range {
                    memory.admission_mut().checkpoint(1)?;
                    if matches!(self.predicates[index].key.1, Scope::Graph(_)) {
                        let entry = self.predicates[index];
                        let any_key = (relation, Scope::Any, entry.key.2);
                        let any = if memory.admission().progress_enabled() {
                            let range = try_equal_range_by(&self.predicates, |value| {
                                memory.admission_mut().checkpoint(1)?;
                                Ok::<_, StorageError>(value.key.cmp(&any_key))
                            })?;
                            assert!(
                                !range.is_empty(),
                                "the any-graph partition precedes graph partitions"
                            );
                            range.start
                        } else {
                            self.predicates
                                .binary_search_by_key(&any_key, |value| value.key)
                                .expect("the any-graph partition precedes graph partitions")
                        };
                        memory.admission_mut().checkpoint(1)?;
                        self.predicates[any].source.combine(entry.source)?;
                    }
                }
            }
            Ok::<_, StorageError>(())
        })();
        // Only these two local buffers die here. Builder metadata remains owned
        // and admitted even if aggregation failed after publishing a prefix.
        let pending_released = memory.release_vec(pending);
        let ordinals_released = memory.release_vec(ordinals);
        result.and(pending_released).and(ordinals_released)
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
        let mut count = 0_usize;
        let mut ids = ids.into_iter();
        loop {
            // Iterator adapters may read an original row inside next(). Admit
            // before advancing, including entries skipped as duplicate IDs.
            if memory.admission().progress_enabled() {
                memory.admission_mut().checkpoint(1)?;
            }
            let Some(id) = ids.next() else {
                break;
            };
            let value = id.statistics_index();
            if previous == Some(value) {
                continue;
            }
            let required = self
                .domains
                .len()
                .checked_add(I::VARINT_BYTES)
                .ok_or(StorageError::SizeOverflow)?;
            if memory.admission().progress_enabled() {
                let mut encoded = crate::SmallVec::<[u8; 10]>::new();
                write_varint(&mut encoded, value - previous.unwrap_or(0));
                for &byte in &encoded {
                    memory.push_copy_with_progress(&mut self.domains, byte)?;
                }
            } else {
                let capacity = growth_capacity::<u8>(self.domains.capacity(), required)?;
                memory.reserve(&mut self.domains, capacity)?;
                write_varint(&mut self.domains, value - previous.unwrap_or(0));
            }
            previous = Some(value);
            count = count.checked_add(1).ok_or(StorageError::SizeOverflow)?;
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
        // Preparation owns no new scratch. On refusal every original builder
        // buffer dies before its exact grant is refunded; no metadata escapes.
        let prepared = (|| {
            if memory.admission().progress_enabled() {
                try_sort_unstable_by(&mut self.predicates, |left, right| {
                    memory.admission_mut().checkpoint(1)?;
                    Ok::<_, StorageError>(left.key.cmp(&right.key))
                })?;
            } else {
                self.predicates.sort_unstable_by_key(|entry| entry.key);
            }
            let mut statement_count = 0_u128;
            for entry in &self.predicates {
                memory.admission_mut().checkpoint(1)?;
                if matches!(entry.key.1, Scope::Any) && entry.key.0 != Relation::Targets {
                    statement_count = statement_count
                        .checked_add(u128::from(entry.rows))
                        .ok_or(StorageError::SizeOverflow)?;
                }
            }
            if memory.admission().progress_enabled() {
                try_sort_unstable_by(&mut self.union, |left, right| {
                    memory.admission_mut().checkpoint(1)?;
                    Ok::<_, StorageError>((left.digest, left.stream as u8, left.ordinal).cmp(&(
                        right.digest,
                        right.stream as u8,
                        right.ordinal,
                    )))
                })?;
                try_dedup_by(&mut self.union, |left, right| {
                    memory.admission_mut().checkpoint(1)?;
                    Ok::<_, StorageError>(left.digest == right.digest)
                })?;
            } else {
                self.union.sort_unstable_by_key(|record| {
                    (record.digest, record.stream as u8, record.ordinal)
                });
                self.union.dedup_by_key(|record| record.digest);
            }
            let mut source_union = purrdf_hash::multiset::Multiset256::default();
            let effective = if self.normalize_annotations {
                &self.union
            } else {
                &self.records
            };
            for record in effective {
                memory.admission_mut().checkpoint(1)?;
                source_union
                    .insert(record.digest)
                    .map_err(|_| StorageError::SizeOverflow)?;
            }
            let fingerprint = if memory.admission().progress_enabled() {
                let mut hash = purrdf_hash::fixed::FixedHasher::default();
                hash_statistics_entries(&self.predicates, &mut hash, memory)?;
                // Vec<u8>'s Hash law is its length followed by ONE byte-field
                // write. Splitting that write would change the fixed hash.
                memory.admission_mut().checkpoint(1)?;
                self.domains.len().hash(&mut hash);
                hash.try_write(&self.domains, &mut |units| {
                    memory.admission_mut().checkpoint(units)
                })?;
                hash_statistics_entries(&self.frequencies, &mut hash, memory)?;
                hash_statistics_entries(&self.kinds, &mut hash, memory)?;
                memory.admission_mut().checkpoint(1)?;
                hash.finish()
            } else {
                purrdf_hash::fixed::hash_one(&(
                    &self.predicates,
                    &self.domains,
                    &self.frequencies,
                    &self.kinds,
                ))
            };
            Ok::<_, StorageError>((statement_count, source_union, fingerprint))
        })();
        let (statement_count, source_union, fingerprint) = match prepared {
            Ok(values) => values,
            Err(cause) => {
                let _ = self.release_admitted(memory);
                return Err(cause);
            }
        };
        // Boxing and source-directory sealing use the original account.
        let mut frozen = LocalStatistics {
            predicates: Box::default(),
            domains: Box::default(),
            frequencies: Box::default(),
            kinds: Box::default(),
            fingerprint,
            directory: SourceDirectory::default(),
            effective_directory: None,
            source_union,
            statement_count,
        };
        let published = (|| {
            frozen.predicates = memory.boxed_slice(core::mem::take(&mut self.predicates))?;
            frozen.domains = memory.boxed_slice(core::mem::take(&mut self.domains))?;
            frozen.frequencies = memory.boxed_slice(core::mem::take(&mut self.frequencies))?;
            frozen.kinds = memory.boxed_slice(core::mem::take(&mut self.kinds))?;
            frozen.directory = SourceDirectory::freeze(core::mem::take(&mut self.records), memory)?;
            if self.normalize_annotations {
                frozen.effective_directory = Some(SourceDirectory::freeze(
                    core::mem::take(&mut self.union),
                    memory,
                )?);
            }
            Ok::<_, StorageError>(())
        })();
        // Each consumed array is owned by either its publication operation or
        // the staged result. Untouched arrays remain here for exact cleanup.
        let remaining = self.release_admitted(memory);
        if let Err(cause) = published.and(remaining) {
            let _ = frozen.release_admitted(memory);
            return Err(cause);
        }
        Ok(frozen)
    }

    /// Destroy every remaining builder array before its own exact refund.
    fn release_admitted<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let predicates = memory.release_vec(self.predicates);
        let domains = memory.release_vec(self.domains);
        let frequencies = memory.release_vec(self.frequencies);
        let kinds = memory.release_vec(self.kinds);
        let records = memory.release_vec(self.records);
        let union = memory.release_vec(self.union);
        predicates
            .and(domains)
            .and(frequencies)
            .and(kinds)
            .and(records)
            .and(union)
    }
}

/// Preserve Vec's derived Hash framing for fixed-layout metadata entries. Every
/// concrete caller supplies one bounded Copy record, never an owning payload.
fn hash_statistics_entries<T: Copy + Hash, S: Admission + ?Sized>(
    values: &[T],
    hash: &mut purrdf_hash::fixed::FixedHasher,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    memory.admission_mut().checkpoint(1)?;
    values.len().hash(hash);
    for value in values {
        memory.admission_mut().checkpoint(1)?;
        value.hash(hash);
    }
    Ok(())
}

impl LocalStatistics {
    pub(crate) const fn source_union(&self) -> purrdf_hash::multiset::Multiset256 {
        self.source_union
    }
    pub(crate) const fn source_directory(&self) -> &SourceDirectory {
        &self.directory
    }
    pub(crate) fn effective_source_directory(&self) -> &SourceDirectory {
        self.effective_directory.as_ref().unwrap_or(&self.directory)
    }
    pub(super) fn build<S: Admission + ?Sized>(
        dictionary: &FrozenTerms,
        asserted: (usize, impl Fn(usize) -> QuadIds),
        reifiers: &(usize, impl Fn(usize) -> ReifierRow),
        annotations: (usize, impl Fn(usize) -> QuadIds),
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        memory.try_scope(|memory| {
            let count = asserted
                .0
                .checked_add(reifiers.0)
                .and_then(|count| count.checked_add(annotations.0))
                .ok_or(StorageError::SizeOverflow)?;
            let mut builder = StatisticsBuilder {
                normalize_annotations: annotations.0 != 0,
                ..StatisticsBuilder::default()
            };
            memory.reserve(&mut builder.records, count)?;
            if builder.normalize_annotations {
                memory.reserve(&mut builder.union, count)?;
            }
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
    /// Release a staged result after a later publication step refuses.
    pub(crate) fn release_admitted<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let predicates = memory.release_vec(self.predicates.into_vec());
        let domains = memory.release_vec(self.domains.into_vec());
        let frequencies = memory.release_vec(self.frequencies.into_vec());
        let kinds = memory.release_vec(self.kinds.into_vec());
        let directory = self.directory.release_admitted(memory);
        let effective_directory = self
            .effective_directory
            .map_or(Ok(()), |directory| directory.release_admitted(memory));
        predicates
            .and(domains)
            .and(frequencies)
            .and(kinds)
            .and(directory)
            .and(effective_directory)
    }

    pub(crate) const fn statement_count(&self) -> u128 {
        self.statement_count
    }
    /// Merge disjoint effective row partitions into the destination dictionary.
    /// Translation precedes sorting and union: source-local numeric order never
    /// substitutes for destination identity. A missing selected frequency stays
    /// unknown, while domains and category counts remain complete and exact.
    /// Native preparation admits iterator advances, translation calls and each
    /// bounded metadata step through the original account. Borrowed sources keep
    /// their grants; only local scratch and abandoned output buffers are released.
    ///
    /// # Errors
    /// Returns the original storage/progress refusal or domain encoding error.
    /// No partially merged statistics result is published on failure.
    pub(super) fn merge_translated<'a, J: StatisticsId, T: Copy, S: Admission + ?Sized>(
        sources: impl IntoIterator<Item = T>,
        summary: impl Fn(T) -> &'a LocalStatistics<J>,
        translate: impl Fn(T, J) -> I,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StatisticsMergeError> {
        let mut keys = Vec::new();
        let mut builder = StatisticsBuilder::default();
        let mut ids = Vec::new();
        let mut frequencies: Vec<Frequency<I>> = Vec::new();
        let mut kinds: Vec<KindCount<I>> = Vec::new();
        let prepared = (|| {
            memory.admission_mut().checkpoint(1)?;
            let mut sources = sources.into_iter();
            loop {
                memory.admission_mut().checkpoint(1)?;
                let Some(source) = sources.next() else {
                    break;
                };
                memory.admission_mut().checkpoint(1)?;
                let statistics = summary(source);
                for (index, entry) in statistics.predicates.iter().enumerate() {
                    memory.admission_mut().checkpoint(1)?;
                    let scope = match entry.key.1 {
                        Scope::Any => Scope::Any,
                        Scope::Graph(None) => Scope::Graph(None),
                        Scope::Graph(Some(id)) => {
                            memory.admission_mut().checkpoint(1)?;
                            Scope::Graph(Some(translate(source, id)))
                        }
                    };
                    memory.admission_mut().checkpoint(1)?;
                    let predicate = translate(source, entry.key.2);
                    memory.push_copy_with_progress(
                        &mut keys,
                        ((entry.key.0, scope, predicate), source, index),
                    )?;
                }
            }
            if memory.admission().progress_enabled() {
                try_sort_unstable_by(&mut keys, |left, right| {
                    memory.admission_mut().checkpoint(1)?;
                    Ok::<_, StorageError>(left.0.cmp(&right.0))
                })?;
            } else {
                keys.sort_unstable_by_key(|entry| entry.0);
            }
            let mut begin = 0;
            while begin < keys.len() {
                memory.admission_mut().checkpoint(1)?;
                let key = keys[begin].0;
                let run = if memory.admission().progress_enabled() {
                    try_equal_range_by(&keys[begin..], |entry| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(entry.0.cmp(&key))
                    })?
                    .end
                } else {
                    keys[begin..].partition_point(|entry| entry.0 == key)
                };
                let end = begin.checked_add(run).ok_or(StorageError::SizeOverflow)?;
                let mut rows = 0_u64;
                let mut source_summary = SourceSummary::default();
                let mut complete_frequencies = true;
                let mut unbounded_frequencies = false;
                for value in &keys[begin..end] {
                    memory.admission_mut().checkpoint(1)?;
                    let (_, source, index) = *value;
                    let entry = &summary(source).predicates[index];
                    rows = rows
                        .checked_add(entry.rows)
                        .ok_or(StorageError::SizeOverflow)?;
                    source_summary.combine(entry.source)?;
                    complete_frequencies &= entry.frequencies.is_some();
                    unbounded_frequencies |= entry.unbounded_frequencies;
                }
                // A graph subset with every row is identical to the any-graph
                // relation. Share all sealed ranges rather than doubling them.
                let alias = if matches!(key.1, Scope::Graph(_)) {
                    let any_key = (key.0, Scope::Any, key.2);
                    let any = if memory.admission().progress_enabled() {
                        let range =
                            try_equal_range_by(&builder.predicates, |entry: &Predicate<I>| {
                                memory.admission_mut().checkpoint(1)?;
                                Ok::<_, StorageError>(entry.key.cmp(&any_key))
                            })?;
                        (!range.is_empty()).then_some(range.start)
                    } else {
                        builder
                            .predicates
                            .binary_search_by_key(&any_key, |entry: &Predicate<I>| entry.key)
                            .ok()
                    };
                    if let Some(index) = any {
                        memory.admission_mut().checkpoint(1)?;
                        (builder.predicates[index].rows == rows).then_some(index)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(index) = alias {
                    memory.admission_mut().checkpoint(1)?;
                    let alias = Predicate {
                        key,
                        ..builder.predicates[index]
                    };
                    memory.push_copy_with_progress(&mut builder.predicates, alias)?;
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
                    for value in &keys[begin..end] {
                        memory.admission_mut().checkpoint(1)?;
                        let (_, source, index) = *value;
                        let statistics = summary(source);
                        let entry = &statistics.predicates[index];
                        let local_domain = if position == 0 {
                            entry.subjects
                        } else {
                            entry.objects
                        };
                        let mut values = statistics.read_domain(local_domain);
                        loop {
                            // One decode reads at most one bounded u64 varint.
                            memory.admission_mut().checkpoint(1)?;
                            let Some(value) = values.next() else {
                                break;
                            };
                            let id = J::statistics_id(value?)?;
                            memory.admission_mut().checkpoint(1)?;
                            let id = translate(source, id);
                            memory.push_copy_with_progress(&mut ids, id)?;
                        }
                    }
                    if memory.admission().progress_enabled() {
                        try_sort_unstable_by(&mut ids, |left, right| {
                            memory.admission_mut().checkpoint(1)?;
                            Ok::<_, StorageError>(left.cmp(right))
                        })?;
                        try_dedup_by(&mut ids, |left, right| {
                            memory.admission_mut().checkpoint(1)?;
                            Ok::<_, StorageError>(left == right)
                        })?;
                    } else {
                        ids.sort_unstable();
                        ids.dedup();
                    }
                    *domain = builder.encode_domain(ids.iter().copied(), memory)?;
                }
                frequencies.clear();
                kinds.clear();
                complete_frequencies &=
                    domains[1].count <= SELECTED_OBJECT_FREQUENCIES || unbounded_frequencies;
                for value in &keys[begin..end] {
                    memory.admission_mut().checkpoint(1)?;
                    let (_, source, index) = *value;
                    let statistics = summary(source);
                    let entry = &statistics.predicates[index];
                    if complete_frequencies {
                        let (offset, count) = entry.frequencies.expect("complete frequencies");
                        for value in &statistics.frequencies[offset..offset + count] {
                            memory.admission_mut().checkpoint(1)?;
                            let object = translate(source, value.object);
                            memory.push_copy_with_progress(
                                &mut frequencies,
                                Frequency {
                                    object,
                                    rows: value.rows,
                                },
                            )?;
                        }
                    }
                    let (offset, count) = entry.kinds;
                    for value in &statistics.kinds[offset..offset + count] {
                        memory.admission_mut().checkpoint(1)?;
                        let datatype = match value.datatype {
                            None => None,
                            Some(id) => {
                                memory.admission_mut().checkpoint(1)?;
                                Some(translate(source, id))
                            }
                        };
                        memory.push_copy_with_progress(
                            &mut kinds,
                            KindCount {
                                kind: value.kind,
                                datatype,
                                rows: value.rows,
                            },
                        )?;
                    }
                }
                if memory.admission().progress_enabled() {
                    try_sort_unstable_by(&mut frequencies, |left, right| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(left.object.cmp(&right.object))
                    })?;
                    try_sort_unstable_by(&mut kinds, |left, right| {
                        memory.admission_mut().checkpoint(1)?;
                        Ok::<_, StorageError>(
                            (left.kind, left.datatype).cmp(&(right.kind, right.datatype)),
                        )
                    })?;
                } else {
                    frequencies.sort_unstable_by_key(|value| value.object);
                    kinds.sort_unstable_by_key(|value| (value.kind, value.datatype));
                }
                let frequency_begin = builder.frequencies.len();
                for value in &frequencies {
                    memory.admission_mut().checkpoint(1)?;
                    let value = *value;
                    if let Some(previous) = builder.frequencies[frequency_begin..].last_mut()
                        && previous.object == value.object
                    {
                        previous.rows = previous
                            .rows
                            .checked_add(value.rows)
                            .ok_or(StorageError::SizeOverflow)?;
                    } else {
                        memory.push_copy_with_progress(&mut builder.frequencies, value)?;
                    }
                }
                let kind_begin = builder.kinds.len();
                for value in &kinds {
                    memory.admission_mut().checkpoint(1)?;
                    let value = *value;
                    if let Some(previous) = builder.kinds[kind_begin..].last_mut()
                        && (previous.kind, previous.datatype) == (value.kind, value.datatype)
                    {
                        previous.rows = previous
                            .rows
                            .checked_add(value.rows)
                            .ok_or(StorageError::SizeOverflow)?;
                    } else {
                        memory.push_copy_with_progress(&mut builder.kinds, value)?;
                    }
                }
                memory.push_copy_with_progress(
                    &mut builder.predicates,
                    Predicate {
                        key,
                        rows,
                        source: source_summary,
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
            Ok::<_, StatisticsMergeError>(())
        })();
        // Only these scratch owners die here. Existing sources retain their own
        // grants; the builder keeps its admitted payload until freeze or failure.
        let keys_released = memory.release_vec(keys);
        let ids_released = memory.release_vec(ids);
        let frequencies_released = memory.release_vec(frequencies);
        let kinds_released = memory.release_vec(kinds);
        let cleanup = keys_released
            .and(ids_released)
            .and(frequencies_released)
            .and(kinds_released)
            .map_err(StatisticsMergeError::Storage);
        if let Err(cause) = prepared.and(cleanup) {
            let _ = builder.release_admitted(memory);
            return Err(cause);
        }
        builder
            .freeze(memory)
            .map_err(StatisticsMergeError::Storage)
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
                directory: self.directory.clone_admitted(memory)?,
                effective_directory: self
                    .effective_directory
                    .as_ref()
                    .map(|directory| directory.clone_admitted(memory))
                    .transpose()?,
                source_union: self.source_union,
                statement_count: self.statement_count,
            })
        })
    }

    /// Canonical words for the enclosing summary's existing digest. Pointer
    /// width, native struct padding and the table hasher never enter this stream.
    pub(super) fn visit_words(&self, mut emit: impl FnMut(u64)) {
        for section in StatisticsSection::ALL {
            emit(self.section_rows(section) as u64);
            if section == StatisticsSection::Domains {
                for chunk in self.domains.chunks(8) {
                    let mut bytes = [0; 8];
                    bytes[..chunk.len()].copy_from_slice(chunk);
                    emit(u64::from_le_bytes(bytes));
                }
            } else {
                for index in 0..self.section_rows(section) {
                    let words = self
                        .record_words(section, index)
                        .expect("a frozen statistics section owns every enumerated record");
                    for &word in words.values() {
                        emit(word);
                    }
                }
            }
        }
    }

    fn entry(
        &self,
        relation: Relation,
        predicate: I,
        graph: GraphMatch<I>,
    ) -> Option<&Predicate<I>> {
        let scope = codec::scope_of(graph);
        self.predicates
            .binary_search_by_key(&(relation, scope, predicate), |entry| entry.key)
            .ok()
            .map(|index| &self.predicates[index])
    }

    pub(crate) fn source_summary(
        &self,
        predicate: I,
        graph: GraphMatch<I>,
        stream: crate::StatisticsStream,
    ) -> SourceSummary {
        self.entry(relation(stream), predicate, graph)
            .map_or_else(SourceSummary::default, |entry| entry.source)
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
            .checked_add(size_of_val(self.kinds.as_ref()))?
            .checked_add(self.directory.retained_bytes()?)?
            .checked_add(
                self.effective_directory
                    .as_ref()
                    .map_or(Some(0), SourceDirectory::retained_bytes)?,
            )
    }
}

pub(crate) fn relation(stream: crate::StatisticsStream) -> Relation {
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
        let scope = codec::scope_of(graph);
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

    #[test]
    fn sources_without_annotations_share_their_physical_directory() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("urn:source:subject").unwrap();
        let predicate = builder.intern_iri("urn:source:predicate").unwrap();
        let object = builder.intern_literal(RdfLiteral::simple("value")).unwrap();
        builder.push_quad(subject, predicate, object, None).unwrap();
        let reifier = builder.intern_iri("urn:source:reifier").unwrap();
        let triple = builder.intern_triple(subject, predicate, object).unwrap();
        builder.push_reifier(reifier, triple).unwrap();
        let dataset = builder.freeze().unwrap();
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut resident);
        let statistics = dataset.prepare_statistics_with_memory(&mut memory).unwrap();
        assert!(statistics.effective_directory.is_none());
        assert!(std::ptr::eq(
            statistics.source_directory(),
            statistics.effective_source_directory(),
        ));
        assert_eq!(statistics.source_directory().len(), 2);
        assert_eq!(statistics.source_union().population(), 2);

        let (annotated, _, _) = fixture(false);
        assert!(
            annotated
                .prepare_statistics_with_memory(&mut memory)
                .unwrap()
                .effective_directory
                .is_some()
        );
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
                    let source = view
                        .source_summary(predicate, graph, stream)
                        .unwrap()
                        .expect("native content evidence is available independently of sizing");
                    assert_eq!(source.statement_count(), selected.len() as u128);
                    let literals = owned_literal_rows(view, selected.iter().map(|row| **row));
                    assert_eq!(source.literal_rows(), literals);
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

    fn owned_literal_rows<D: DatasetView>(
        view: &D,
        rows: impl IntoIterator<Item = QuadIds<D::Id>>,
    ) -> [purrdf_hash::multiset::Multiset256; 2] {
        let mut literals = [purrdf_hash::multiset::Multiset256::default(); 2];
        for row in rows {
            let crate::TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } = view.term_value(row.o).unwrap()
            else {
                continue;
            };
            let subject = view.term_value(row.s).unwrap();
            let predicate = view.term_value(row.p).unwrap();
            let graph = row.g.map(|id| view.term_value(id).unwrap());
            let mut admission = purrdf_lex::allocation::Resident;
            let hashes = crate::source::LiteralSourceRow {
                graph: graph.as_ref(),
                subject: &subject,
                predicate: &predicate,
                lexical: &lexical_form,
                datatype: &datatype,
                language: language.as_deref(),
                direction,
            }
            .hashes_with_memory(&mut Memory::new(&mut admission))
            .unwrap();
            for (summary, hash) in literals.iter_mut().zip(hashes) {
                summary.insert(hash).unwrap();
            }
        }
        literals
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

        // A live outer visitor owns the reserved buffer. Its nested operation
        // must preserve a refused temporary checkout without corrupting the
        // outer ordered domain, and the reservation must return afterward.
        let expected: BTreeSet<_> = dataset
            .quads()
            .filter(|row| row.p == predicates[0])
            .map(|row| row.o)
            .collect();
        let mut observed = Vec::new();
        stats
            .visit_distinct(
                predicates[0],
                GraphMatch::Any,
                StatisticsStream::Asserted,
                StatisticsPosition::Object,
                &mut |id| {
                    if observed.is_empty() {
                        let mut ceiling = purrdf_lex::allocation::Ceiling(0);
                        let mut memory = Memory::new(&mut ceiling);
                        let refused = dataset.visit_distinct_with_memory(
                            predicates[0],
                            GraphMatch::Any,
                            StatisticsStream::Asserted,
                            StatisticsPosition::Object,
                            &mut memory,
                            &mut |_| panic!("refusal precedes callbacks"),
                        );
                        assert_eq!(
                            refused,
                            Err(super::super::pack::bits::PackBitsError::Storage(
                                StorageError::AdmissionFailed
                            ))
                        );
                        assert_eq!(memory.admitted_bytes(), 0);
                        let mut nested = None;
                        stats
                            .visit_distinct(
                                predicates[0],
                                GraphMatch::Any,
                                StatisticsStream::Asserted,
                                StatisticsPosition::Object,
                                &mut |inner| {
                                    nested = Some(inner);
                                    ControlFlow::Break(())
                                },
                            )
                            .unwrap();
                        assert_eq!(nested, expected.first().copied());
                        assert_eq!(
                            stats
                                .predicate(
                                    predicates[0],
                                    GraphMatch::Any,
                                    StatisticsStream::Asserted
                                )
                                .unwrap()
                                .objects,
                            Some(exact(expected.len() as u64))
                        );
                    }
                    observed.push(id);
                    ControlFlow::Continue(())
                },
            )
            .unwrap();
        assert_eq!(observed, expected.iter().copied().collect::<Vec<_>>());
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            stats.visit_distinct(
                predicates[0],
                GraphMatch::Any,
                StatisticsStream::Asserted,
                StatisticsPosition::Object,
                &mut |_| panic!("visitor unwind"),
            )
        }));
        assert!(unwind.is_err());
        let mut ceiling = purrdf_lex::allocation::Ceiling(0);
        let mut memory = Memory::new(&mut ceiling);
        let mut reused = 0;
        dataset
            .visit_distinct_with_memory(
                predicates[0],
                GraphMatch::Any,
                StatisticsStream::Asserted,
                StatisticsPosition::Object,
                &mut memory,
                &mut |_| {
                    reused += 1;
                    ControlFlow::Continue(())
                },
            )
            .unwrap();
        assert_eq!(reused, expected.len());
        assert_eq!(memory.admitted_bytes(), 0);

        let invalid = TermId::from_index(u32::MAX - 1).unwrap();
        assert_eq!(
            stats
                .predicate(invalid, GraphMatch::Any, StatisticsStream::Asserted)
                .unwrap()
                .rows,
            exact(0)
        );
        assert_eq!(
            stats.predicate_object(
                predicates[0],
                invalid,
                GraphMatch::Any,
                StatisticsStream::Asserted
            ),
            Some(exact(0))
        );
    }

    #[test]
    fn sealed_source_rows_match_independent_owned_projection_and_codec() {
        // Frozen with independent `b3sum` 1.8.7 over the published length
        // frame, domain, relation byte and this exact thirty-two-byte row.
        for (relation, expected) in [
            (
                Relation::Asserted,
                "d7034bfde2887c2cb8f099f512181649878cb0d346c2266d2ed0f0b7bc4c9b7b",
            ),
            (
                Relation::Reifiers,
                "2205757d7a96ded46ae6d705303291ef62bcedba2d7a2986d272246d4b897405",
            ),
            (
                Relation::Annotations,
                "dc415b3ace2210576784975db57755b148f65f655594bfbf96a93a71e7a0746b",
            ),
            (
                Relation::Targets,
                "8a4678deb41a4103fd2bed9af99cbd5bc0d031cd2060727c716c734bfaee3243",
            ),
        ] {
            assert_eq!(
                purrdf_hash::hex::encode(&crate::source::record_hash(
                    relation,
                    *b"0123456789abcdefghijklmnopqrstuv",
                )),
                expected,
            );
        }
        for named in [false, true] {
            let (dataset, _, _) = fixture(named);
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            let statistics = dataset.prepare_statistics_with_memory(&mut memory).unwrap();
            for (index, entry) in statistics.predicates.iter().enumerate() {
                let rows: Vec<_> = match entry.key.0 {
                    Relation::Asserted => dataset.quads().collect(),
                    Relation::Annotations => dataset.annotation_quads().collect(),
                    Relation::Reifiers => dataset.reifier_quads().collect(),
                    Relation::Targets => continue,
                };
                let selected: Vec<_> = rows
                    .into_iter()
                    .filter(|row| {
                        row.p == entry.key.2
                            && match entry.key.1 {
                                Scope::Any => true,
                                Scope::Graph(graph) => graph == row.g,
                            }
                    })
                    .collect();
                let expected = owned_literal_rows(&*dataset, selected.iter().copied());
                assert_eq!(entry.source.literals, expected, "{:?}", entry.key);
                assert_eq!(entry.source.records.population(), selected.len() as u128);
                let mut encoded = [0; PREDICATE_RECORD_BYTES];
                statistics
                    .encode_records(StatisticsSection::Predicates, index, &mut encoded)
                    .unwrap();
                let decoded = PredicateRecord::<TermId>::decode(&encoded).unwrap();
                assert_eq!(decoded.source, entry.source);
                // A hash tree can authenticate false metadata bytes. Their
                // independently reconstructed source summary must still differ.
                encoded[17 * 8 + 8] ^= 1;
                if let Ok(altered) = PredicateRecord::<TermId>::decode(&encoded) {
                    assert_ne!(altered.source, entry.source);
                }
            }
        }

        // Every stopped source-row stage leaves the complete old summary intact,
        // including long lexical extents, nested object emission and finalizers.
        use purrdf_lex::allocation::{Admission, Resident, StorageError};
        struct Progress {
            live: usize,
            remaining: usize,
            stopped: bool,
            calls: usize,
            cause: Option<Box<u8>>,
        }
        impl Admission for Progress {
            fn progress_enabled(&self) -> bool {
                true
            }
            fn checkpoint(&mut self, _: u64) -> Result<(), StorageError> {
                assert!(!self.stopped, "no later work after physical refusal");
                self.calls += 1;
                if self.remaining == 0 {
                    self.stopped = true;
                    return Err(StorageError::AdmissionFailed);
                }
                self.remaining -= 1;
                Ok(())
            }
            fn resize(&mut self, next: usize) -> Result<(), StorageError> {
                assert!(
                    !self.stopped || next <= self.live,
                    "no growth after refusal"
                );
                self.live = next;
                Ok(())
            }
        }
        let lexical = "x".repeat(3 * 16384 + 1);
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("urn:source:subject").unwrap();
        let predicate = builder.intern_iri("urn:source:predicate").unwrap();
        let graph = builder.intern_iri("urn:source:graph").unwrap();
        let short_object = builder.intern_literal(RdfLiteral::simple("short")).unwrap();
        let literal_object = builder
            .intern_literal(RdfLiteral::simple(&lexical))
            .unwrap();
        let mut nested_object = literal_object;
        for _ in 0..12 {
            nested_object = builder
                .intern_triple(subject, predicate, nested_object)
                .unwrap();
        }
        let rows = [
            (literal_object, None),
            (nested_object, Some(graph)),
            (short_object, None),
            (literal_object, Some(graph)),
            (short_object, None),
        ];
        // The triple fixture supplies the dictionary; the row projection below
        // independently selects its named or default graph from those terms.
        for (object, _) in rows {
            builder.push_quad(subject, predicate, object, None).unwrap();
        }
        let dataset = builder.freeze().unwrap();
        let subject_value = dataset.term_value(subject).unwrap();
        let predicate_value = dataset.term_value(predicate).unwrap();
        let graph_value = dataset.term_value(graph).unwrap();
        let object_values: Vec<_> = rows
            .iter()
            .map(|&(object, _)| dataset.term_value(object).unwrap())
            .collect();
        let dataset = Arc::try_unwrap(dataset)
            .unwrap_or_else(|_| panic!("fixture dataset must have one owner"));
        let native = TripleDataset::try_from(dataset).unwrap();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut source_row = crate::source::SourceRowHasher::default();
        for ((object, graph), object_value) in rows.into_iter().zip(&object_values) {
            let quad = QuadIds {
                s: subject,
                p: predicate,
                o: object,
                g: graph,
            };
            let mut pending = Vec::new();
            let mut expected = SourceSummary::default();
            let expected_pair = expected
                .add_row(
                    Relation::Annotations,
                    quad,
                    &native.dictionary,
                    &mut pending,
                    &mut source_row,
                    &mut memory,
                )
                .unwrap();
            memory.release_vec(pending).unwrap();
            let mut accepting = Progress {
                live: 0,
                remaining: usize::MAX,
                stopped: false,
                calls: 0,
                cause: Some(Box::new(17)),
            };
            let mut controlled = Memory::new(&mut accepting);
            let owned = crate::source::SourceRowRef {
                subject: &subject_value,
                predicate: &predicate_value,
                object: object_value,
                graph: graph.map(|_| &graph_value),
            }
            .hashes(&mut controlled)
            .unwrap();
            if object == literal_object {
                let literal = crate::source::LiteralSourceRow {
                    subject: &subject_value,
                    predicate: &predicate_value,
                    graph: graph.map(|_| &graph_value),
                    lexical: &lexical,
                    datatype: purrdf_xsd::datatype::XSD_STRING,
                    language: None,
                    direction: None,
                }
                .hashes_with_memory(&mut controlled)
                .unwrap();
                assert_eq!(owned, literal);
            }
            assert_eq!(
                expected_pair,
                (
                    crate::source::record_hash(Relation::Annotations, owned[0]),
                    crate::source::record_hash(Relation::Asserted, owned[0]),
                )
            );
            assert_eq!(controlled.admitted_bytes(), 0);
            let mut completed = false;
            for remaining in 0..4096 {
                let mut progress = Progress {
                    live: 0,
                    remaining,
                    stopped: false,
                    calls: 0,
                    cause: Some(Box::new(17)),
                };
                let mut memory = Memory::new(&mut progress);
                let mut pending = Vec::new();
                let mut summary = SourceSummary::default();
                let result = summary.add_row(
                    Relation::Annotations,
                    quad,
                    &native.dictionary,
                    &mut pending,
                    &mut source_row,
                    &mut memory,
                );
                let calls = memory.admission().calls;
                match result {
                    Ok(pair) => {
                        assert_eq!(pair, expected_pair);
                        assert_eq!(summary, expected);
                        completed = true;
                    }
                    Err(error) => {
                        assert_eq!(error, StorageError::AdmissionFailed);
                        assert_eq!(summary, SourceSummary::default());
                        assert_eq!(memory.admission().cause.as_deref(), Some(&17));
                        assert_eq!(calls, remaining + 1);
                    }
                }
                memory.release_vec(pending).unwrap();
                assert_eq!(memory.admitted_bytes(), 0);
                assert_eq!(memory.admission().calls, calls);
                if completed {
                    break;
                }
            }
            assert!(completed);
        }
        let quad = QuadIds {
            s: subject,
            p: predicate,
            o: literal_object,
            g: None,
        };
        // A later population overflow cannot publish earlier literal inserts.
        for population in ["literal", "record"] {
            let mut summary = SourceSummary::default();
            let full = purrdf_hash::multiset::Multiset256::from_parts([9; 32], u128::MAX);
            if population == "literal" {
                summary.literals[1] = full;
            } else {
                summary.records = full;
            }
            let before = summary;
            let mut pending = Vec::new();
            assert_eq!(
                summary.add_row(
                    Relation::Annotations,
                    quad,
                    &native.dictionary,
                    &mut pending,
                    &mut source_row,
                    &mut memory
                ),
                Err(StorageError::SizeOverflow)
            );
            assert_eq!(summary, before);
            memory.release_vec(pending).unwrap();
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
    fn segmented_metadata_matches_native_graph_streams_and_sorted_domains() {
        use crate::{SegmentedBuildLimits, SegmentedImage, SegmentedReadLimits};
        let (native, predicates, scopes) = fixture_partition(true, 0, 101);
        let limits = SegmentedBuildLimits::new(4096, 1_000_000, 4096, 512, 4)
            .unwrap()
            .with_first_term_index((1_u64 << 40) + 17)
            .unwrap();
        let image = SegmentedImage::from_native_fixture(&native, limits).unwrap();
        let source = image
            .open_session(SegmentedReadLimits::new(
                8_000_000,
                2,
                100_000,
                100_000_000,
                8,
            ))
            .unwrap();
        let global = |id| {
            source
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
        assert_rows(&source, &predicates, &scopes);
        assert!(source.read_error().is_none());
        assert!(predicates.iter().all(|id| id.index() > u64::from(u32::MAX)));
    }

    #[test]
    fn translated_domains_keep_full_width_ids_and_sort_after_translation() {
        use crate::{GlobalTermId, SnapshotStatistics};
        let (native, predicates, _) = fixture(false);
        let base = u64::from(u32::MAX) + 100;
        let translate = |id: TermId| GlobalTermId::from_index(base - id.index() as u64).unwrap();
        let mut admission = purrdf_lex::allocation::Resident;
        let mut memory = Memory::new(&mut admission);
        let source = native.prepare_statistics_with_memory(&mut memory).unwrap();
        let merged = LocalStatistics::<GlobalTermId>::merge_translated(
            [&*source],
            |source| source,
            |_, id| translate(id),
            &mut memory,
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

    fn assert_persisted_partition_merge(
        left: &RdfDataset,
        right: &RdfDataset,
        combined: &RdfDataset,
        predicates: &[TermId],
        scopes: &[GraphMatch],
    ) {
        use crate::{SegmentedBuildLimits, SegmentedImage, SegmentedReadLimits};
        let images: Vec<_> = [left, right, combined]
            .into_iter()
            .enumerate()
            .map(|(index, dataset)| {
                let first = (1_u64 << (40 + index)) + 17;
                let limits = SegmentedBuildLimits::new(4096, 1_000_000, 4096, 512, 4)
                    .unwrap()
                    .with_first_term_index(first)
                    .unwrap();
                SegmentedImage::from_native_fixture(dataset, limits).unwrap()
            })
            .collect();
        let sources: Vec<_> = images
            .iter()
            .map(|image| {
                image
                    .open_session(SegmentedReadLimits::new(
                        8_000_000,
                        2,
                        100_000,
                        100_000_000,
                        8,
                    ))
                    .unwrap()
            })
            .collect();
        for &predicate in predicates {
            let value = left.term_value(predicate);
            let ids: Vec<_> = sources
                .iter()
                .map(|source| source.term_id_by_value(&value).unwrap().unwrap())
                .collect();
            for &scope in scopes {
                let graphs: Vec<_> = sources
                    .iter()
                    .map(|source| match scope {
                        GraphMatch::Any => GraphMatch::Any,
                        GraphMatch::Default => GraphMatch::Default,
                        GraphMatch::Named(id) => GraphMatch::Named(
                            source
                                .term_id_by_value(&left.term_value(id))
                                .unwrap()
                                .unwrap(),
                        ),
                    })
                    .collect();
                for stream in [
                    StatisticsStream::Asserted,
                    StatisticsStream::Reifiers,
                    StatisticsStream::Annotations,
                ] {
                    let counts: Vec<_> = sources
                        .iter()
                        .enumerate()
                        .map(|(index, source)| {
                            source
                                .statistics()
                                .unwrap()
                                .predicate(ids[index], graphs[index], stream)
                                .unwrap()
                        })
                        .collect();
                    assert_eq!(
                        counts[0].rows.value + counts[1].rows.value,
                        counts[2].rows.value
                    );
                    for position in [StatisticsPosition::Subject, StatisticsPosition::Object] {
                        let mut values = [BTreeSet::new(), BTreeSet::new(), BTreeSet::new()];
                        for (index, source) in sources.iter().enumerate() {
                            let mut previous = None;
                            assert!(
                                source
                                    .statistics()
                                    .unwrap()
                                    .visit_distinct(
                                        ids[index],
                                        graphs[index],
                                        stream,
                                        position,
                                        &mut |id| {
                                            assert!(previous.is_none_or(|previous| previous < id));
                                            previous = Some(id);
                                            values[index].insert(source.term_value(id).unwrap());
                                            ControlFlow::Continue(())
                                        }
                                    )
                                    .unwrap()
                            );
                        }
                        let [mut union, right, expected] = values;
                        union.extend(right);
                        assert_eq!(union, expected);
                        let count = match position {
                            StatisticsPosition::Subject => counts[2].subjects,
                            StatisticsPosition::Object => counts[2].objects,
                        };
                        assert_eq!(union.len() as u64, count.unwrap().value);
                    }
                }
            }
        }
        assert!(sources.iter().all(|source| source.read_error().is_none()));
    }

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
                vec![left.clone(), right.clone()],
            ))).unwrap();
            let native_ids: Vec<_> = predicates.iter().map(|&id|
                combined.as_ref().term_id_by_value(&left.as_ref().term_value(id)).unwrap()).collect();
            let global_ids: Vec<_> = predicates.iter().map(|&id|
                paged.term_id_by_value(&left.as_ref().term_value(id)).unwrap().unwrap()).collect();
            let mut native_scopes = Vec::new();
            let mut global_scopes = Vec::new();
            for &scope in &scopes {
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
            assert_persisted_partition_merge(&left, &right, &combined, &predicates, &scopes);
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
