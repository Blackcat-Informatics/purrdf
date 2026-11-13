// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact native statistics over immutable subject-first physical rows.

use std::hash::{Hash, Hasher as _};
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

use super::{QuadIds, RdfDataset, TermId, TermRef};
use crate::ir::pack::bits::PackBitsError;
use crate::ir::statistics::{LocalStatistics, Relation, exact, relation};
use crate::{
    GraphMatch, PredicateStatistics, RdfTermKind, SnapshotStatistics, StatisticCount,
    StatisticsOwnerToken, StatisticsPosition, StatisticsStream,
};

#[derive(Debug)]
pub(super) struct NativeStatistics {
    scalar: Mutex<Vec<u64>>,
    visitor: Mutex<Option<Vec<u64>>>,
    fingerprint: OnceLock<u64>,
    cache_identity: OnceLock<Option<StatisticsOwnerToken>>,
    rdf_type: Option<TermId>,
    /// Rows read by statistic scans so far: the length of the run or relation each
    /// scan reads.
    scanned: AtomicU64,
}

impl NativeStatistics {
    pub(super) fn new<S: Admission + ?Sized>(
        dictionary: &super::super::frozen_terms::FrozenTerms,
        deferred: bool,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        let words = if deferred {
            dictionary.terms.len().div_ceil(64)
        } else {
            0
        };
        let scalar = memory.collect(std::iter::repeat_n(0, words))?;
        let visitor = memory.collect(std::iter::repeat_n(0, words))?;
        Ok(Self {
            scalar: Mutex::new(scalar),
            visitor: Mutex::new(Some(visitor)),
            fingerprint: OnceLock::new(),
            cache_identity: OnceLock::new(),
            rdf_type: dictionary
                .term_id_by_iri_with_memory(purrdf_iri::vocab::rdf::TYPE, memory)?,
            scanned: AtomicU64::new(0),
        })
    }

    pub(super) fn retained_bytes(&self) -> Option<usize> {
        // A checked-out visitor retains the same original reserved allocation.
        self.scalar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .capacity()
            .checked_mul(2 * size_of::<u64>())
    }
}

impl Clone for NativeStatistics {
    fn clone(&self) -> Self {
        let scalar = self
            .scalar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Self {
            scalar: Mutex::new(vec![0; scalar.len()]),
            visitor: Mutex::new(Some(vec![0; scalar.len()])),
            fingerprint: self.fingerprint.clone(),
            cache_identity: OnceLock::new(),
            rdf_type: self.rdf_type,
            scanned: AtomicU64::new(0),
        }
    }
}

struct VisitorWorkspace<'a> {
    words: Vec<u64>,
    home: Option<&'a Mutex<Option<Vec<u64>>>>,
    refund: Option<&'a mut dyn FnMut(Vec<u64>) -> Result<(), StorageError>>,
}

impl VisitorWorkspace<'_> {
    fn refund(&mut self) -> Result<(), StorageError> {
        self.refund
            .take()
            .map_or(Ok(()), |refund| refund(std::mem::take(&mut self.words)))
    }
}

impl Drop for VisitorWorkspace<'_> {
    fn drop(&mut self) {
        if let Some(home) = self.home {
            let mut available = home
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *available = Some(std::mem::take(&mut self.words));
        } else {
            let _ = self.refund();
        }
    }
}

struct PreparedStatistics<'m, 'a, S: Admission + ?Sized> {
    statistics: Option<LocalStatistics>,
    memory: &'m mut Memory<'a, S>,
}

impl<S: Admission + ?Sized> PreparedStatistics<'_, '_, S> {
    fn release(&mut self) -> Result<(), StorageError> {
        self.statistics.take().map_or(Ok(()), |statistics| {
            statistics.release_admitted(self.memory)
        })
    }
}

impl<S: Admission + ?Sized> Drop for PreparedStatistics<'_, '_, S> {
    fn drop(&mut self) {
        // A visitor's panic remains authoritative if its original account refuses
        // shrink. Normal returns report cleanup failures through release instead.
        let _ = self.release();
    }
}

fn insert(words: &mut [u64], id: TermId) -> bool {
    let word = &mut words[id.index() / 64];
    let bit = 1 << (id.index() % 64);
    let fresh = *word & bit == 0;
    *word |= bit;
    fresh
}

impl RdfDataset {
    pub(crate) fn prepare_statistics_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> Result<std::borrow::Cow<'_, LocalStatistics>, StorageError> {
        if let Some(statistics) = &self.statistics {
            return Ok(std::borrow::Cow::Borrowed(statistics));
        }
        LocalStatistics::build(
            &self.dictionary,
            (self.quads.len(), |index| QuadIds::from(self.quads[index])),
            &(self.reifiers.len(), |index| self.reifiers[index]),
            (self.annotations.len(), |index| {
                let (s, p, o, g) = self.annotations[index];
                QuadIds { s, p, o, g }
            }),
            memory,
        )
        .map(std::borrow::Cow::Owned)
    }

    pub(crate) fn with_prepared_statistics<S: Admission + ?Sized, T, E: From<StorageError>>(
        &self,
        memory: &mut Memory<'_, S>,
        use_statistics: impl FnOnce(&LocalStatistics, &mut Memory<'_, S>) -> Result<T, E>,
    ) -> Result<T, E> {
        let statistics = match self.prepare_statistics_with_memory(memory)? {
            std::borrow::Cow::Borrowed(statistics) => return use_statistics(statistics, memory),
            std::borrow::Cow::Owned(statistics) => statistics,
        };
        let mut prepared = PreparedStatistics {
            statistics: Some(statistics),
            memory,
        };
        // The summary is installed above and only release can take it, after the
        // callback finishes or while the guard unwinds.
        let result = use_statistics(
            prepared
                .statistics
                .as_ref()
                .expect("owned prepared summary"),
            prepared.memory,
        );
        let cleanup = prepared.release();
        match result {
            Err(cause) => Err(cause),
            Ok(value) => cleanup.map(|()| value).map_err(E::from),
        }
    }

    fn statistic_rows(&self, relation: Relation) -> impl Iterator<Item = QuadIds> + '_ {
        let asserted =
            self.quads
                .iter()
                .copied()
                .map(QuadIds::from)
                .take(if relation == Relation::Asserted {
                    usize::MAX
                } else {
                    0
                });
        let annotations = self
            .annotations
            .iter()
            .copied()
            .map(|(s, p, o, g)| QuadIds { s, p, o, g })
            .take(if relation == Relation::Annotations {
                usize::MAX
            } else {
                0
            });
        let reifiers = self
            .reifiers
            .iter()
            .copied()
            .map(move |(s, o, g)| QuadIds {
                s,
                o,
                g,
                p: if relation == Relation::Targets {
                    self.dictionary.triple_components(o)[1]
                } else {
                    self.rdf_reifies_id()
                        .expect("nonempty validated reifiers intern their predicate")
                },
            })
            .take(
                if matches!(relation, Relation::Reifiers | Relation::Targets) {
                    usize::MAX
                } else {
                    0
                },
            );
        asserted.chain(annotations).chain(reifiers)
    }

    fn native_predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        relation: Relation,
    ) -> PredicateStatistics {
        if relation == Relation::Asserted
            && let Some(answer) = self.indexed_asserted_predicate(predicate, graph)
        {
            return answer;
        }
        self.count_scan(relation);
        let mut words = self
            .native_statistics
            .scalar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        words.fill(0);
        let mut rows = 0;
        let mut subjects = 0;
        let mut objects = 0;
        let mut previous = None;
        for row in self
            .statistic_rows(relation)
            .filter(|row| row.p == predicate && graph.matches(row.g))
        {
            rows += 1;
            subjects += u64::from(previous != Some(row.s));
            previous = Some(row.s);
            objects += u64::from(insert(&mut words, row.o));
        }
        drop(words);
        PredicateStatistics {
            rows: exact(rows),
            subjects: Some(exact(subjects)),
            objects: Some(exact(objects)),
        }
    }

    /// The permutation run of the asserted rows with the given bound positions,
    /// or `None` while the permutation that serves them is not built: a read of
    /// statistics allocates nothing, so it uses an index only once a query path
    /// has materialized it, and otherwise scans.
    fn built_run(
        &self,
        predicate: TermId,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> Option<(&[u32], usize, usize)> {
        let plan = Self::probe_plan(false, true, object.is_some(), graph);
        if matches!(plan.perm, super::QuadPermutation::Spog) {
            return None;
        }
        let built = self.permutation_cell(plan.perm).get()?;
        let (_, lo, hi) = self.candidate_run(&plan, None, Some(predicate), object, graph);
        Some((&**built, lo, hi))
    }

    /// The asserted rows of one predicate in one graph restriction, read from the
    /// built permutation run whose sort prefix is the bound predicate (and graph)
    /// rather than from the whole table. The run is exactly the matching rows and
    /// is ordered by object within it, so distinct objects are the changes of
    /// object and distinct subjects are marked in the preallocated bit set.
    fn indexed_asserted_predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
    ) -> Option<PredicateStatistics> {
        let (built, lo, hi) = self.built_run(predicate, None, graph)?;
        let run = &built[lo..hi];
        self.native_statistics
            .scanned
            .fetch_add(run.len() as u64, Ordering::Relaxed);
        let mut words = self
            .native_statistics
            .scalar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        words.fill(0);
        let mut subjects = 0;
        let mut objects = 0;
        let mut previous = None;
        for &ordinal in run {
            let row = QuadIds::from(self.quads[ordinal as usize]);
            objects += u64::from(previous != Some(row.o));
            previous = Some(row.o);
            subjects += u64::from(insert(&mut words, row.s));
        }
        drop(words);
        Some(PredicateStatistics {
            rows: exact(run.len() as u64),
            subjects: Some(exact(subjects)),
            objects: Some(exact(objects)),
        })
    }

    /// Rows of one predicate and object in one graph restriction, counted from
    /// the bounds of the built permutation run, with no row read.
    fn indexed_asserted_frequency(
        &self,
        predicate: TermId,
        object: TermId,
        graph: GraphMatch,
    ) -> Option<u64> {
        let (_, lo, hi) = self.built_run(predicate, Some(object), graph)?;
        Some((hi - lo) as u64)
    }

    /// Record one scan of `relation` in the work counter.
    fn count_scan(&self, relation: Relation) {
        let rows = match relation {
            Relation::Asserted => self.quads.len(),
            Relation::Annotations => self.annotations.len(),
            Relation::Reifiers | Relation::Targets => self.reifiers.len(),
        };
        self.native_statistics
            .scanned
            .fetch_add(rows as u64, Ordering::Relaxed);
    }

    /// Rows read by the native statistic scans of this dataset so far: the
    /// length of the scanned relation, added once per scan. A statistic already
    /// answered is not scanned again, so repeated planning leaves this unchanged.
    #[must_use]
    pub fn statistics_rows_scanned(&self) -> u64 {
        self.native_statistics.scanned.load(Ordering::Relaxed)
    }

    pub(crate) fn native_source_summary(
        &self,
        predicate: Option<TermId>,
        graph: GraphMatch,
        relation: Relation,
    ) -> crate::source::SourceSummary {
        let mut summary = crate::source::SourceSummary::default();
        let mut pending = Vec::new();
        let mut hasher = crate::source::SourceRowHasher::default();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for row in self
            .statistic_rows(relation)
            .filter(|row| predicate.is_none_or(|p| row.p == p) && graph.matches(row.g))
        {
            // Canonical traversal has no workspace allocation. Native row counts
            // are bounded by the frozen u32 tables; Resident cannot refuse work.
            summary
                .add_row(
                    relation,
                    row,
                    &self.dictionary,
                    &mut pending,
                    &mut hasher,
                    &mut memory,
                )
                .expect("validated native source hashing has no fallible storage or count growth");
        }
        summary
    }
}

impl SnapshotStatistics<TermId> for RdfDataset {
    fn source(&self) -> &'static str {
        "native-frozen-predicate-domains-v1"
    }

    fn cache_identity(&self) -> Option<StatisticsOwnerToken> {
        *self
            .native_statistics
            .cache_identity
            .get_or_init(StatisticsOwnerToken::try_new)
    }

    fn fingerprint(&self) -> u64 {
        *self.native_statistics.fingerprint.get_or_init(|| {
            let mut hash = purrdf_hash::fixed::FixedHasher::default();
            self.native_statistics.rdf_type.hash(&mut hash);
            for term in &self.dictionary.terms {
                // Local dictionary descriptors contain every kind/datatype ID;
                // lexical content is irrelevant to these sizing statistics.
                use super::super::term::InternedTerm;
                let tag = match term {
                    InternedTerm::Iri(_) => 0_u8,
                    InternedTerm::Blank { .. } => 1,
                    InternedTerm::Literal(literal) => {
                        literal.datatype.hash(&mut hash);
                        2
                    }
                    InternedTerm::Triple { .. } => 3,
                };
                tag.hash(&mut hash);
            }
            for relation in [
                Relation::Asserted,
                Relation::Reifiers,
                Relation::Annotations,
                Relation::Targets,
            ] {
                relation.hash(&mut hash);
                for row in self.statistic_rows(relation) {
                    row.hash(&mut hash);
                }
            }
            hash.finish()
        })
    }

    fn stream_rows(&self, graph: GraphMatch, stream: StatisticsStream) -> Option<StatisticCount> {
        if let Some(statistics) = &self.statistics {
            return SnapshotStatistics::stream_rows(statistics, graph, stream);
        }
        Some(exact(
            self.statistic_rows(relation(stream))
                .filter(|row| graph.matches(row.g))
                .count() as u64,
        ))
    }

    fn predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: StatisticsStream,
    ) -> Option<PredicateStatistics> {
        if let Some(statistics) = &self.statistics {
            return SnapshotStatistics::predicate(statistics, predicate, graph, stream);
        }
        Some(self.native_predicate(predicate, graph, relation(stream)))
    }

    fn predicate_object(
        &self,
        predicate: TermId,
        object: TermId,
        graph: GraphMatch,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        if let Some(statistics) = &self.statistics {
            return SnapshotStatistics::predicate_object(
                statistics, predicate, object, graph, stream,
            );
        }
        let relation = relation(stream);
        if relation == Relation::Asserted
            && let Some(rows) = self.indexed_asserted_frequency(predicate, object, graph)
        {
            return (self
                .native_predicate(predicate, graph, relation)
                .objects
                .is_some_and(|objects| objects.value <= 64)
                || Some(predicate) == self.native_statistics.rdf_type)
                .then(|| exact(rows));
        }
        self.count_scan(relation);
        let mut words = self
            .native_statistics
            .scalar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        words.fill(0);
        let mut distinct = 0;
        let mut rows = 0;
        for row in self
            .statistic_rows(relation)
            .filter(|row| row.p == predicate && graph.matches(row.g))
        {
            distinct += usize::from(insert(&mut words, row.o));
            rows += u64::from(row.o == object);
        }
        drop(words);
        (distinct <= 64 || Some(predicate) == self.native_statistics.rdf_type)
            .then_some(exact(rows))
    }

    fn predicate_object_kind(
        &self,
        predicate: TermId,
        kind: RdfTermKind,
        datatype: Option<TermId>,
        graph: GraphMatch,
        stream: StatisticsStream,
    ) -> Option<StatisticCount> {
        if let Some(statistics) = &self.statistics {
            return SnapshotStatistics::predicate_object_kind(
                statistics, predicate, kind, datatype, graph, stream,
            );
        }
        Some(exact(
            self.statistic_rows(relation(stream))
                .filter(|row| {
                    if row.p != predicate || !graph.matches(row.g) {
                        return false;
                    }
                    match (self.resolve(row.o), kind, datatype) {
                        (TermRef::Iri(_), RdfTermKind::Iri, None)
                        | (TermRef::Blank { .. }, RdfTermKind::BlankNode, None)
                        | (TermRef::Triple { .. }, RdfTermKind::Triple, None) => true,
                        (
                            TermRef::Literal {
                                datatype: actual, ..
                            },
                            RdfTermKind::Literal,
                            selected,
                        ) => selected.is_none_or(|id| id == actual),
                        _ => false,
                    }
                })
                .count() as u64,
        ))
    }

    fn reifier_target_predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
    ) -> Option<PredicateStatistics> {
        if let Some(statistics) = &self.statistics {
            return SnapshotStatistics::reifier_target_predicate(statistics, predicate, graph);
        }
        Some(self.native_predicate(predicate, graph, Relation::Targets))
    }

    fn visit_distinct(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: StatisticsStream,
        position: StatisticsPosition,
        visit: &mut dyn FnMut(TermId) -> ControlFlow<()>,
    ) -> Result<bool, PackBitsError> {
        let mut resident = Resident;
        self.visit_distinct_with_memory(
            predicate,
            graph,
            stream,
            position,
            &mut Memory::new(&mut resident),
            visit,
        )
    }
}

impl RdfDataset {
    pub(crate) fn visit_distinct_with_memory<S: Admission + ?Sized>(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: StatisticsStream,
        position: StatisticsPosition,
        memory: &mut Memory<'_, S>,
        visit: &mut dyn FnMut(TermId) -> ControlFlow<()>,
    ) -> Result<bool, PackBitsError> {
        if let Some(statistics) = &self.statistics {
            return SnapshotStatistics::visit_distinct(
                statistics, predicate, graph, stream, position, visit,
            );
        }
        let reserved = self
            .native_statistics
            .visitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let (words, home) = if let Some(words) = reserved {
            (words, Some(&self.native_statistics.visitor))
        } else {
            let words = memory
                .collect(std::iter::repeat_n(
                    0,
                    self.dictionary.terms.len().div_ceil(64),
                ))
                .map_err(PackBitsError::Storage)?;
            (words, None)
        };
        let mut refund = |words| memory.release_vec(words);
        let mut scratch = VisitorWorkspace {
            words,
            home,
            refund: if home.is_none() {
                Some(&mut refund)
            } else {
                None
            },
        };
        scratch.words.fill(0);
        for row in self
            .statistic_rows(relation(stream))
            .filter(|row| row.p == predicate && graph.matches(row.g))
        {
            insert(
                &mut scratch.words,
                if position == StatisticsPosition::Subject {
                    row.s
                } else {
                    row.o
                },
            );
        }
        'domain: for (index, &word) in scratch.words.iter().enumerate() {
            let mut remaining = word;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                let index = u32::try_from(index * 64 + bit)
                    .map_err(|_| PackBitsError::Storage(StorageError::SizeOverflow))?;
                let id = TermId::from_index(index)
                    .map_err(|cause| PackBitsError::Storage(cause.into()))?;
                if visit(id).is_break() {
                    break 'domain;
                }
                remaining &= remaining - 1;
            }
        }
        scratch.refund().map_err(PackBitsError::Storage)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use purrdf_lex::allocation::{Memory, Resident};

    use super::*;
    use crate::ir::RdfDatasetBuilder;

    fn builder() -> (RdfDatasetBuilder, Vec<TermId>, Vec<TermId>) {
        let mut b = RdfDatasetBuilder::new();
        let rdf_type = b.intern_iri(purrdf_iri::vocab::rdf::TYPE).unwrap();
        let link = b.intern_iri("http://example.org/link").unwrap();
        let wide = b.intern_iri("http://example.org/wide").unwrap();
        let classes: Vec<TermId> = (0..3)
            .map(|i| b.intern_iri(&format!("http://example.org/C{i}")).unwrap())
            .collect();
        let graphs: Vec<TermId> = (0..2)
            .map(|i| b.intern_iri(&format!("http://example.org/g{i}")).unwrap())
            .collect();
        let nodes: Vec<TermId> = (0..80)
            .map(|i| b.intern_iri(&format!("http://example.org/n{i}")).unwrap())
            .collect();
        for (i, &s) in nodes.iter().enumerate() {
            let graph = [None, Some(graphs[0]), Some(graphs[1])][i % 3];
            b.push_quad(s, rdf_type, classes[i % 3], graph).unwrap();
            b.push_quad(s, link, nodes[(i * 7 + 1) % 80], graph)
                .unwrap();
            b.push_quad(s, link, nodes[(i * 5 + 2) % 80], None).unwrap();
            // More than 64 distinct objects: a frequency is not kept for it.
            b.push_quad(s, wide, nodes[(i + 1) % 80], graph).unwrap();
        }
        let objects = nodes.into_iter().chain(classes).collect();
        (b, objects, vec![rdf_type, link, wide])
    }

    /// The statistics an ordinary freeze answers on demand equal the ones a
    /// `freeze_with_memory` freeze builds eagerly, for every predicate, graph
    /// restriction and object, both by scanning (no permutation built yet) and by
    /// reading index runs (permutations built by a query's cardinality estimates):
    /// the planner must be told the same either way.
    #[test]
    fn on_demand_statistics_equal_eager_statistics() {
        let (ordinary, objects, predicates) = builder();
        let ordinary = ordinary.freeze().unwrap();
        let (eager, ..) = builder();
        let mut resident = Resident;
        let eager = eager
            .freeze_with_memory(&mut Memory::resume(&mut resident, 1 << 30))
            .unwrap();
        let graphs = [
            GraphMatch::Any,
            GraphMatch::Default,
            GraphMatch::Named(ordinary.term_id_by_iri("http://example.org/g0").unwrap()),
            GraphMatch::Named(ordinary.term_id_by_iri("http://example.org/g1").unwrap()),
        ];
        let compare = || {
            let mut saw_wide = false;
            for &predicate in &predicates {
                for graph in graphs {
                    let stream = StatisticsStream::Asserted;
                    let domain =
                        SnapshotStatistics::predicate(&*ordinary, predicate, graph, stream);
                    assert_eq!(
                        domain,
                        SnapshotStatistics::predicate(&eager, predicate, graph, stream),
                    );
                    let wide = domain.unwrap().objects.unwrap().value > 64;
                    saw_wide |= wide;
                    for &object in &objects {
                        let on_demand = SnapshotStatistics::predicate_object(
                            &*ordinary, predicate, object, graph, stream,
                        );
                        if wide {
                            assert_eq!(on_demand, None, "a wide predicate keeps no frequency");
                        } else {
                            assert_eq!(
                                on_demand,
                                SnapshotStatistics::predicate_object(
                                    &eager, predicate, object, graph, stream
                                ),
                            );
                        }
                    }
                }
            }
            assert!(
                saw_wide,
                "the fixture has a predicate with more than 64 objects"
            );
        };
        compare();
        let scanned = ordinary.statistics_rows_scanned();
        for &predicate in &predicates {
            for graph in graphs {
                let _ = ordinary.cardinality_estimate(None, Some(predicate), None, graph);
                for &object in &objects {
                    let _ =
                        ordinary.cardinality_estimate(None, Some(predicate), Some(object), graph);
                }
            }
        }
        compare();
        assert!(
            ordinary.statistics_rows_scanned() - scanned < scanned,
            "the second pass read index runs, not the table"
        );
    }

    /// With its permutation built, a predicate's statistic reads that predicate's
    /// rows, not the table.
    #[test]
    fn a_predicate_statistic_reads_only_its_own_rows() {
        let (b, _, predicates) = builder();
        let dataset = b.freeze().unwrap();
        let link = predicates[1];
        let _ = dataset.cardinality_estimate(None, Some(link), None, GraphMatch::Any);
        let domain = SnapshotStatistics::predicate(
            &*dataset,
            link,
            GraphMatch::Any,
            StatisticsStream::Asserted,
        )
        .unwrap();
        assert_eq!(dataset.statistics_rows_scanned(), domain.rows.value);
        assert!(domain.rows.value < dataset.quad_count() as u64);
    }
}
