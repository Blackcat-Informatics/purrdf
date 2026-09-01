// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//!   ([`DatasetView::reifier_quads_of`] / [`DatasetView::annotations_of_with_graph`]). Only
//!   an unbound subject scans the whole table. This matters because the BGP matcher probes
//!   once per row: a full scan here would make a join quadratic in the reifier count.
//!
//! A dataset with no statement layer costs nothing: both side-tables are empty, and the
//! trait's capability-gated defaults yield nothing for a backend that has no such layer.

use purrdf_core::{DatasetView, GraphMatch, QuadIds, ViewTermId};

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

/// Call `emit` for every statement-layer virtual quad matching `probe`.
///
/// Reifier rows come first, then annotation rows, each in its side-table's frozen sorted
/// order — a deterministic, dataset-derived order, as every result-observable order in
/// this crate must be.
pub(crate) fn visit_quads<D: DatasetView>(
    dataset: &D,
    probe: StatementProbe<D::Id>,
    mut emit: impl FnMut(QuadIds<D::Id>),
) {
    let StatementProbe {
        s,
        p,
        o,
        graph,
        scan_reifier_rows,
    } = probe;

    // The residual both walks share verbatim: the graph scope plus the predicate/object
    // id-equality `quads_for_pattern` applies. The side-table walks are not pre-narrowed
    // by the probe the way an indexed quad read is, so whatever a walk was not narrowed
    // by is filtered here.
    let residual = move |quad: &QuadIds<D::Id>| {
        graph.matches(quad.g) && p.is_none_or(|id| quad.p == id) && o.is_none_or(|id| quad.o == id)
    };

    // ── reifier rows: (reifier, rdf:reifies, <<triple>>) ────────────────────
    if scan_reifier_rows {
        match s {
            Some(reifier) => {
                for quad in dataset.reifier_quads_of(reifier).filter(residual) {
                    emit(quad);
                }
            }
            None => {
                for quad in dataset.reifier_quads().filter(residual) {
                    emit(quad);
                }
            }
        }
    }

    // ── annotation rows: (reifier, predicate, object) ───────────────────────
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
                .filter(residual)
            {
                emit(quad);
            }
        }
        None => {
            for quad in dataset.annotation_quads().filter(residual) {
                emit(quad);
            }
        }
    }
}
