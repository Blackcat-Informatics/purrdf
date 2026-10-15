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
use crate::{GraphMatch, PredicateStatistics, RdfTermKind, StatisticCount, StatisticsAccuracy};

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
enum Scope {
    Any,
    Graph(Option<TermId>),
}

#[derive(Clone, Copy, Debug, Hash)]
struct Domain {
    offset: usize,
    bytes: usize,
    count: usize,
}

#[derive(Clone, Copy, Debug, Hash)]
struct Predicate {
    key: (Relation, Scope, TermId),
    rows: u64,
    subjects: Domain,
    objects: Domain,
    frequencies: Option<(usize, usize)>,
    kinds: (usize, usize),
}

#[derive(Clone, Copy, Debug, Hash)]
struct Frequency {
    object: TermId,
    rows: u64,
}

#[derive(Clone, Copy, Debug, Hash)]
struct KindCount {
    kind: u8,
    datatype: Option<TermId>,
    rows: u64,
}

/// No query-time growth: every slice is frozen through the original admission.
#[derive(Clone)]
pub(super) struct LocalStatistics {
    predicates: Box<[Predicate]>,
    domains: Box<[u8]>,
    frequencies: Box<[Frequency]>,
    kinds: Box<[KindCount]>,
    fingerprint: u64,
}
purrdf_hash::debug_non_exhaustive!(LocalStatistics { fingerprint });

#[derive(Default)]
pub(super) struct StatisticsBuilder {
    predicates: Vec<Predicate>,
    domains: Vec<u8>,
    frequencies: Vec<Frequency>,
    kinds: Vec<KindCount>,
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
                let subjects =
                    self.encode_domain(&ordinals[begin..end], |index| row(index).s, memory)?;
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
                let objects =
                    self.encode_domain(&ordinals[begin..end], |index| row(index).o, memory)?;
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

    fn encode_domain<S: Admission + ?Sized>(
        &mut self,
        ordinals: &[usize],
        id: impl Fn(usize) -> TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<Domain, StorageError> {
        let offset = self.domains.len();
        let mut previous = None;
        let mut count = 0;
        for &ordinal in ordinals {
            let value = id(ordinal).index() as u64;
            if previous == Some(value) {
                continue;
            }
            let required = self
                .domains
                .len()
                .checked_add(5)
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
    ) -> Result<LocalStatistics, StorageError> {
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

    fn entry(
        &self,
        relation: Relation,
        predicate: TermId,
        graph: GraphMatch,
    ) -> Option<&Predicate> {
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
        predicate: TermId,
        graph: GraphMatch,
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
        predicate: TermId,
        object: TermId,
        graph: GraphMatch,
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
        predicate: TermId,
        kind: RdfTermKind,
        datatype: Option<TermId>,
        graph: GraphMatch,
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
        predicate: TermId,
        graph: GraphMatch,
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
            Some(domain) => DeltaListRef::new(
                &self.domains[domain.offset..domain.offset + domain.bytes],
                domain.count,
            ),
            None => DeltaListRef::new(&[], 0),
        }
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

impl crate::SnapshotStatistics<TermId> for LocalStatistics {
    fn source(&self) -> &'static str {
        "native-frozen-predicate-domains-v1"
    }

    fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    fn predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<PredicateStatistics> {
        Some(Self::predicate(self, relation(stream), predicate, graph))
    }

    fn predicate_object(
        &self,
        predicate: TermId,
        object: TermId,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<StatisticCount> {
        self.frequency(relation(stream), predicate, object, graph)
    }

    fn predicate_object_kind(
        &self,
        predicate: TermId,
        kind: RdfTermKind,
        datatype: Option<TermId>,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<StatisticCount> {
        Some(self.kind(relation(stream), predicate, kind, datatype, graph))
    }

    fn reifier_target_predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
    ) -> Option<PredicateStatistics> {
        Some(Self::predicate(self, Relation::Targets, predicate, graph))
    }

    fn visit_distinct(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
        position: crate::StatisticsPosition,
        visit: &mut dyn FnMut(TermId) -> std::ops::ControlFlow<()>,
    ) -> Result<bool, super::pack::bits::PackBitsError> {
        use super::pack::bits::PackBitsError;
        for value in self.domain(
            relation(stream),
            predicate,
            graph,
            position == crate::StatisticsPosition::Subject,
        ) {
            let value = u32::try_from(value?)
                .map_err(|_| PackBitsError::Malformed("statistics identity exceeds local width"))?;
            let id = TermId::from_index(value).map_err(|_| {
                PackBitsError::Malformed("statistics identity is not representable")
            })?;
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
        let mut builder = RdfDatasetBuilder::new();
        let ids: Vec<_> = [
            "a",
            "b",
            "p",
            "annotation",
            "rare",
            "common",
            "g",
            "r",
            "r2",
        ]
        .into_iter()
        .map(|name| {
            builder
                .intern_iri(&format!("http://example.org/{name}"))
                .unwrap()
        })
        .collect();
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

    fn assert_rows<D: DatasetView<Id = TermId>>(
        view: &D,
        predicates: &[TermId],
        scopes: &[GraphMatch],
    ) {
        let statistics = view.statistics().unwrap();
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
                        assert_eq!(
                            statistics.predicate_object(predicate, object, graph, stream),
                            Some(exact(
                                selected.iter().filter(|row| row.o == object).count() as u64
                            ))
                        );
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
