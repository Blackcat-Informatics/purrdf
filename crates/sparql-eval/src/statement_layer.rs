// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The RDF 1.2 **statement layer** as virtual quads: one traversal, every consumer.
//!
//! RDF 1.2 reifiers and statement annotations do NOT live in the frozen dataset's quad
//! table — they live in two side-tables. Anything that reads a dataset only through
//! [`DatasetView::quads_for_pattern`] is therefore BLIND to them, and blindness here does
//! not announce itself: the probe simply returns nothing, and a consumer reports a
//! complete, diagnostic-free, empty answer about data the dataset is holding.
//!
//! [`visit_quads`] is the quad-table walk's twin. It yields the two side-tables AS the
//! virtual triples they denote,
//!
//! * `(reifier, rdf:reifies, <<triple>>)` — one per reifier binding;
//! * `(reifier, annotationPredicate, annotationObject)` — one per annotation;
//!
//! each carrying its declaration's own graph slot, filtered by the same `(s, p, o)`
//! id-equality and the same [`GraphMatch`] scope the quad-table scan applies. The rows are
//! strictly ADDITIVE — the side-tables are disjoint from `quads`, so folding them in
//! double-counts nothing.
//!
//! # Why this is a module and not two copies
//!
//! Two consumers in this crate need exactly this walk: the basic-graph-pattern matcher,
//! which probes it once per candidate row, and [`PathGraph`](crate::path_relation::PathGraph)'s
//! snapshot, which probes it once per declared step alternative. They differ only in how
//! they decide whether the reifier table is worth touching at all, and that decision is
//! the caller's — hence [`StatementProbe::scan_reifier_rows`]. Everything downstream of
//! it is one implementation, because a second copy is a second place for the statement
//! layer to be forgotten. (The ShEx validator holds the third instance of this walk, over
//! the concrete `RdfDataset` rather than over a [`DatasetView`]; it is the same shape and
//! the same doctrine.)
//!
//! # The walks are gated and narrowed, never blind
//!
//! * The reifier table is walked only when the caller says a reifier row could match at
//!   all — a predicate bound to anything but `rdf:reifies`, or an object bound to an
//!   IRI/blank/literal, can match no reifier row, and scanning for it would be pure cost.
//! * BOTH side-tables index straight to a bound subject's run, because the subject IS the
//!   reifier key and hence each table's primary sort key
//!   ([`DatasetView::reifier_quads_of`] / [`DatasetView::annotations_of_with_graph`]).
//! * An UNBOUND subject with an exact triple object uses
//!   [`DatasetView::reifier_quads_for_target`]. Other unbound probes narrow by GRAPH
//!   ([`DatasetView::reifier_quads_in_graph`] / [`DatasetView::annotation_quads_in_graph`])
//!   rather than scanning the whole table and filtering the graph out afterwards. A
//!   backend that can name, without materializing anything, which of its own storage
//!   units could possibly hold a row in that graph (a paged dataset's per-page graph
//!   postings, for instance) visits only those; a backend with no such index falls back
//!   to the same whole-table-then-filter behavior this walk always had. Either way the
//!   graph scope is applied ONCE — inside the narrowed walk, not a second time in the
//!   residual below — so an unbound-subject probe never forces every storage unit to be
//!   read just to throw most of it away.
//!
//! A dataset with no statement layer costs nothing: both side-tables are empty, and the
//! trait's capability-gated defaults yield nothing for a backend that has no such layer.

use purrdf_core::{DatasetView, GraphMatch, QuadIds, ViewTermId};
use std::ops::ControlFlow;

/// The `(s, p, o)` probe, graph scope, and reifier-table gate one statement-layer walk
/// runs under.
///
/// A struct rather than a parameter list because the two consumers pass five correlated
/// values that are meaningless apart, and because a walk whose gate is one `bool` among
/// six positional arguments is a walk whose gate gets passed wrong.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StatementProbe<I: ViewTermId> {
    /// The bound subject, or `None` when the subject is free. A bound subject narrows
    /// BOTH side-tables to one contiguous run.
    pub s: Option<I>,
    /// The bound predicate, or `None` when the predicate is free.
    pub p: Option<I>,
    /// The bound object, or `None` when the object is free.
    pub o: Option<I>,
    /// Bound first-level components of a structural triple object. Deeper
    /// structure remains the caller's exact unification responsibility.
    pub components: [Option<I>; 3],
    /// The active graph scope, applied to each row's own graph slot — so a `GRAPH ?g`
    /// probe binds `?g` to the graph the reifier or annotation was declared in.
    pub graph: GraphMatch<I>,
    /// Whether the reifier table can contribute a matching row AT ALL.
    ///
    /// The caller owns this because the two consumers decide it differently: the BGP
    /// matcher reads it off a compiled pattern's predicate and object positions, while a
    /// path step reads it off the one predicate IRI the alternative declares. It is a
    /// pure cost gate — the residual filter below enforces the same `(p, o)` equality
    /// either way, so a caller that sets it conservatively true gets the same rows, more
    /// slowly.
    pub scan_reifier_rows: bool,
}

/// Call `emit` for each matching quad until it requests a break. A break stops
/// both streams before pulling or translating another source row.
///
/// Reifier rows come first, then annotation rows, each in its side-table's frozen sorted
/// order — a deterministic, dataset-derived order, as every result-observable order in
/// this crate must be.
pub(crate) fn visit_quads<D: DatasetView>(
    dataset: &D,
    probe: StatementProbe<D::Id>,
    emit: impl FnMut(QuadIds<D::Id>) -> ControlFlow<()>,
) {
    visit_quads_range(dataset, probe, None, emit);
}

pub(crate) fn visit_quads_range<D: DatasetView>(
    dataset: &D,
    probe: StatementProbe<D::Id>,
    range: Option<purrdf_core::AnnotationValueRange<'_>>,
    mut emit: impl FnMut(QuadIds<D::Id>) -> ControlFlow<()>,
) {
    let StatementProbe {
        s,
        p,
        o,
        components,
        graph,
        scan_reifier_rows,
    } = probe;

    // The KEYED walks (`reifier_quads_of`/`annotations_of_with_graph`) are narrowed by
    // REIFIER (Part A: skipped per-page unless the page's sealed summary proves it owns
    // a row for that exact reifier), never by graph, so the graph scope is still applied
    // here alongside the predicate/object id-equality `quads_for_pattern` applies.
    let keyed_residual = move |quad: &QuadIds<D::Id>| {
        graph.matches(quad.g) && p.is_none_or(|id| quad.p == id) && o.is_none_or(|id| quad.o == id)
    };

    // The UNKEYED walks (`reifier_quads_in_graph`/`annotation_quads_in_graph`) are
    // narrowed by the SAME `graph` scope before the residual runs — a `GraphMatch`-aware
    // backend (e.g. a paged dataset) visits only the storage units its own graph-postings
    // index names for that stream, then still applies the per-row `graph.matches` check
    // inside each visited unit (a unit named by the index may also hold rows in OTHER
    // graphs). So the walk itself already enforces the graph scope; only the
    // predicate/object id-equality remains here.
    let unkeyed_residual = move |quad: &QuadIds<D::Id>| {
        p.is_none_or(|id| quad.p == id) && o.is_none_or(|id| quad.o == id)
    };

    // ── reifier rows: (reifier, rdf:reifies, <<triple>>) ────────────────────
    if scan_reifier_rows {
        match (s, o) {
            (Some(reifier), _) => {
                for quad in dataset.reifier_quads_of(reifier).filter(keyed_residual) {
                    if emit(quad).is_break() {
                        return;
                    }
                }
            }
            (None, Some(target)) => {
                for quad in dataset
                    .reifier_quads_for_target(target, graph)
                    .filter(unkeyed_residual)
                {
                    if emit(quad).is_break() {
                        return;
                    }
                }
            }
            (None, None) if components.iter().any(Option::is_some) => {
                for quad in dataset
                    .reifier_quads_for_components(components, graph)
                    .filter(unkeyed_residual)
                {
                    if emit(quad).is_break() {
                        return;
                    }
                }
            }
            (None, None) => {
                for quad in dataset
                    .reifier_quads_in_graph(graph)
                    .filter(unkeyed_residual)
                {
                    if emit(quad).is_break() {
                        return;
                    }
                }
            }
        }
    }

    // ── annotation rows: (reifier, predicate, object) ───────────────────────
    if let Some(range) = range.as_ref() {
        for quad in dataset.annotation_quads_for_range(p, o, graph, range) {
            if s.is_none_or(|subject| quad.s == subject) && emit(quad).is_break() {
                return;
            }
        }
        return;
    }
    match s {
        Some(reifier) => {
            for quad in dataset
                .annotations_of_with_graph(reifier)
                .map(|(pred, obj, g)| QuadIds {
                    s: reifier,
                    p: pred,
                    o: obj,
                    g,
                })
                .filter(keyed_residual)
            {
                if emit(quad).is_break() {
                    return;
                }
            }
        }
        None => {
            for quad in dataset.annotation_quads_for_pattern(p, o, graph) {
                if emit(quad).is_break() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_core::RdfDatasetBuilder;

    #[test]
    fn component_and_annotation_walks_materialize_only_selected_orders() {
        let mut builder = RdfDatasetBuilder::new();
        let terms: Vec<_> = (0..9)
            .map(|index| {
                builder
                    .intern_iri(&format!("http://example.org/key/{index}"))
                    .unwrap()
            })
            .collect();
        for index in 0..257 {
            let reifier = builder
                .intern_iri(&format!("http://example.org/claim/{index}"))
                .unwrap();
            let target = builder
                .intern_triple(
                    if index % 17 == 0 { terms[0] } else { terms[3] },
                    if index % 2 == 0 { terms[1] } else { terms[4] },
                    terms[2],
                )
                .unwrap();
            builder.push_reifier(reifier, target).unwrap();
            builder
                .push_annotation(
                    reifier,
                    if index % 23 == 0 { terms[5] } else { terms[6] },
                    if index % 2 == 0 { terms[7] } else { terms[8] },
                )
                .unwrap();
        }
        let dataset = builder.freeze().unwrap();
        let reifies = dataset
            .term_id_by_iri(purrdf_iri::vocab::rdf::REIFIES)
            .unwrap();
        for round in 0..2 {
            for annotation in [false, true] {
                let probe = StatementProbe {
                    s: None,
                    p: Some(if annotation { terms[5] } else { reifies }),
                    o: annotation.then_some(terms[7]),
                    components: if annotation {
                        [None; 3]
                    } else {
                        [Some(terms[0]), Some(terms[1]), None]
                    },
                    graph: GraphMatch::Any,
                    scan_reifier_rows: !annotation,
                };
                let mut count = 0;
                let window = CurrentThreadWindow::open();
                visit_quads(&dataset, probe, |quad| {
                    assert_eq!(Some(quad.p), probe.p);
                    if annotation {
                        assert_eq!(quad.o, terms[7]);
                    } else {
                        let purrdf_core::TermRef::Triple { s, p, .. } =
                            dataset.as_ref().resolve(quad.o)
                        else {
                            panic!("triple target")
                        };
                        assert_eq!([s, p], [terms[0], terms[1]]);
                    }
                    count += 1;
                    ControlFlow::Continue(())
                });
                let measured = window.close();
                assert_eq!(count, if annotation { 6 } else { 8 });
                let arrays = if round == 0 {
                    if annotation { 1 } else { 2 }
                } else {
                    0
                };
                assert_eq!(measured.allocations, arrays);
                assert_eq!(
                    measured.retained_bytes,
                    i64::try_from(arrays * 257 * 4).unwrap()
                );
            }
        }
    }

    #[test]
    fn exact_target_walk_materializes_the_target_order_then_reuses_it() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/s").unwrap();
        let predicate = builder.intern_iri("http://example.org/p").unwrap();
        let graph = builder.intern_iri("http://example.org/g").unwrap();
        let target = builder.intern_triple(subject, predicate, subject).unwrap();
        let other = builder.intern_triple(graph, predicate, subject).unwrap();
        for index in 0..257 {
            let reifier = builder
                .intern_iri(&format!("http://example.org/r{index}"))
                .unwrap();
            builder
                .push_reifier_in_graph(
                    reifier,
                    if index % 17 == 0 { target } else { other },
                    (index % 2 == 0).then_some(graph),
                )
                .unwrap();
        }
        let dataset = builder.freeze().unwrap();
        // The actual shared traversal must reach the target index through Arc's
        // DatasetView forwarding. Merely filtering a whole-table walk allocates
        // no index and therefore cannot satisfy this physical-path assertion.
        for (iteration, scope) in [
            GraphMatch::Named(graph),
            GraphMatch::Default,
            GraphMatch::Any,
        ]
        .into_iter()
        .enumerate()
        {
            let expected = dataset
                .reifier_quads()
                .filter(|quad| quad.o == target && scope.matches(quad.g))
                .count();
            let mut count = 0;
            let window = CurrentThreadWindow::open();
            visit_quads(
                &dataset,
                StatementProbe {
                    s: None,
                    p: None,
                    o: Some(target),
                    components: [None; 3],
                    graph: scope,
                    scan_reifier_rows: true,
                },
                |quad| {
                    assert_eq!(quad.o, target);
                    assert!(scope.matches(quad.g));
                    count += 1;
                    ControlFlow::Continue(())
                },
            );
            let measured = window.close();
            assert_eq!(count, expected);
            assert_eq!(
                measured.retained_bytes,
                if iteration == 0 { 257 * 4 } else { 0 }
            );
            assert_eq!(measured.allocations, u64::from(iteration == 0));
        }
    }
}
