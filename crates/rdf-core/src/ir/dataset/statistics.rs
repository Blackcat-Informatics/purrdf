// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact native statistics over immutable subject-first physical rows.

use std::hash::{Hash, Hasher as _};
use std::ops::ControlFlow;
use std::sync::{Mutex, OnceLock};

use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

use super::{QuadIds, RdfDataset, TermId, TermRef};
use crate::ir::pack::bits::PackBitsError;
use crate::ir::statistics::{LocalStatistics, Relation, exact, relation};
use crate::{
    GraphMatch, PredicateStatistics, RdfTermKind, SnapshotStatistics, StatisticCount,
    StatisticsPosition, StatisticsStream,
};

#[derive(Debug)]
pub(super) struct NativeStatistics {
    scalar: Mutex<Vec<u64>>,
    visitor: Mutex<Option<Vec<u64>>>,
    fingerprint: OnceLock<u64>,
    rdf_type: Option<TermId>,
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
            rdf_type: dictionary
                .term_id_by_iri_with_memory(purrdf_iri::vocab::rdf::TYPE, memory)?,
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
            rdf_type: self.rdf_type,
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
        let statistics = self.prepare_statistics_with_memory(memory)?;
        let result = use_statistics(&statistics, memory);
        let cleanup = match statistics {
            std::borrow::Cow::Borrowed(_) => Ok(()),
            std::borrow::Cow::Owned(statistics) => statistics.release_admitted(memory),
        };
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
        let mut words = self
            .native_statistics
            .scalar
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        words.fill(0);
        let mut distinct = 0;
        let mut rows = 0;
        for row in self
            .statistic_rows(relation(stream))
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
