// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SPARQL **active dataset** (§13) for a query or UPDATE `WHERE`.
//!
//! A query's `FROM` / `FROM NAMED` clauses (and an UPDATE's `USING` / `USING NAMED`)
//! replace the store's default dataset with a custom one:
//!
//! - **default graph** — with no clause, the store's own default graph; with `FROM`
//!   graphs, the *RDF-merge* of those named graphs (the store default is then
//!   excluded). The merge is a **read-time union** — each merged graph is still read
//!   through the ordinary indexed [`DatasetView::quads_for_pattern`] path, and the
//!   results are unioned (BGP de-dupes triples; path evaluation de-dupes endpoints via
//!   its `BTreeSet`s). No graph is ever materialised or mutated.
//! - **named graphs** — with no clause, every store named graph is addressable by
//!   `GRAPH`; with `FROM NAMED` / `USING NAMED`, exactly the listed graphs are.
//!
//! An IRI that names no graph (a graph with zero quads, per the implicit-existence
//! doctrine) contributes nothing — it is silently dropped, never an error (§13.2).
//!
//! [`GraphMatch`] (the storage read-view's graph filter) is deliberately left as a
//! closed three-way `Copy` enum; the dataset-clause logic lives entirely here so the
//! indexed read path and the C-ABI are untouched.

use std::collections::BTreeSet;

use purrdf_core::{DatasetView, GraphMatch, QuadIds, RdfDataset, TermId, TermValue};
use purrdf_sparql_algebra::{QueryDataset, UsingClause};

use crate::convert::named_node_to_value;

/// How the active **default graph** is sourced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DefaultSpec {
    /// No `FROM`/`USING`: the store's own default graph.
    StoreDefault,
    /// `FROM`/`USING` graphs: the RDF-merge of these named graphs (sorted, deduped).
    /// An empty vector is legal — every named IRI was absent, or only `FROM NAMED`
    /// was given — and matches nothing.
    Merged(Vec<TermId>),
}

/// The SPARQL active dataset (§13), resolved to dataset-local ids once per query/op.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveDataset {
    /// The active default-graph specification.
    default: DefaultSpec,
    /// The graphs addressable by `GRAPH`. `None` = every store named graph (the
    /// no-clause default); `Some(set)` = exactly these (an explicit `FROM NAMED` /
    /// `USING NAMED`; possibly empty).
    named: Option<BTreeSet<TermId>>,
}

/// The resolved graph filter(s) for the *current* evaluation scope — almost always a
/// single [`GraphMatch`]; a `FROM`/`USING`-merged default graph expands to one filter
/// per merged graph (the union of which IS the RDF-merge).
pub(crate) enum GraphScope {
    /// A single filter (store default, a named graph, or `Any`). No de-dup needed.
    One(GraphMatch),
    /// The merge of these named graphs. A consumer producing *rows* (BGP) must de-dupe
    /// triples; a consumer collecting *endpoints* into a set (paths) gets it for free.
    Merge(Vec<TermId>),
}

/// Resolve a list of graph IRIs to their dataset-local ids, dropping any that name no
/// term (an absent graph contributes nothing — never an error), then sort+dedupe for
/// a deterministic, duplicate-free merge set.
fn resolve_graphs(
    graphs: &[purrdf_sparql_algebra::NamedNode],
    dataset: &RdfDataset,
) -> Vec<TermId> {
    let mut ids: Vec<TermId> = graphs
        .iter()
        .filter_map(|n| dataset.term_id_by_value(&named_node_to_value(n)))
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

impl ActiveDataset {
    /// The store's own default dataset: the store default graph plus every named graph.
    pub(crate) fn store_default() -> Self {
        Self {
            default: DefaultSpec::StoreDefault,
            named: None,
        }
    }

    /// Build the active dataset from a query's `FROM` / `FROM NAMED` clause. An empty
    /// clause means "use the store's default dataset" (§13.2).
    pub(crate) fn from_query_dataset(qd: &QueryDataset, dataset: &RdfDataset) -> Self {
        if qd.default.is_empty() && qd.named.is_empty() {
            return Self::store_default();
        }
        Self {
            default: DefaultSpec::Merged(resolve_graphs(&qd.default, dataset)),
            named: Some(resolve_graphs(&qd.named, dataset).into_iter().collect()),
        }
    }

    /// Build the WHERE active dataset from an UPDATE op's `USING` / `USING NAMED`
    /// clauses (the write-path twin of [`from_query_dataset`]). The caller only invokes
    /// this when `using` is non-empty (§3.1.3: `USING` replaces `WITH`'s effect on the
    /// WHERE dataset).
    pub(crate) fn from_using(using: &[UsingClause], dataset: &RdfDataset) -> Self {
        let mut default = Vec::new();
        let mut named = Vec::new();
        for u in using {
            match u {
                UsingClause::Default(n) => {
                    if let Some(id) = dataset.term_id_by_value(&named_node_to_value(n)) {
                        default.push(id);
                    }
                }
                UsingClause::Named(n) => {
                    if let Some(id) = dataset.term_id_by_value(&named_node_to_value(n)) {
                        named.push(id);
                    }
                }
            }
        }
        default.sort();
        default.dedup();
        Self {
            default: DefaultSpec::Merged(default),
            named: Some(named.into_iter().collect()),
        }
    }

    /// Scope the WHERE default graph to a single `WITH <g>` graph (named graphs stay
    /// unrestricted). An absent `g` yields an empty default graph (the WHERE matches
    /// nothing), matching the implicit-existence doctrine.
    pub(crate) fn with_default_graph(dataset: &RdfDataset, g: &TermValue) -> Self {
        let ids = dataset
            .term_id_by_value(g)
            .map(|id| vec![id])
            .unwrap_or_default();
        Self {
            default: DefaultSpec::Merged(ids),
            named: None,
        }
    }

    /// The graph scope for the current `active_graph`. A root default-graph scope
    /// (`GraphMatch::Default`) expands through the default spec (store default or a
    /// merge); an in-`GRAPH` scope (`Named`) is always a single filter — the default
    /// merge never applies inside a `GRAPH` block.
    pub(crate) fn scope_for(&self, active_graph: GraphMatch) -> GraphScope {
        match active_graph {
            GraphMatch::Default => match &self.default {
                DefaultSpec::StoreDefault => GraphScope::One(GraphMatch::Default),
                DefaultSpec::Merged(gs) => GraphScope::Merge(gs.clone()),
            },
            other => GraphScope::One(other),
        }
    }

    /// Whether the named graph `id` is addressable by `GRAPH` under this dataset.
    pub(crate) fn named_allows(&self, id: TermId) -> bool {
        match &self.named {
            None => true,
            Some(set) => set.contains(&id),
        }
    }
}

impl GraphScope {
    /// Visit every quad matching `(s, p, o)` in this scope. For a merged scope the
    /// per-graph indexed reads are unioned in order; **no triple de-dup is applied
    /// here** (the BGP caller de-dupes by `(s,p,o)`; endpoint-collecting callers do
    /// not need it).
    pub(crate) fn for_each_quad(
        &self,
        dataset: &RdfDataset,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        mut f: impl FnMut(QuadIds),
    ) {
        match self {
            Self::One(gm) => {
                for q in dataset.quads_for_pattern(s, p, o, *gm) {
                    f(q);
                }
            }
            Self::Merge(gs) => {
                for &g in gs {
                    for q in dataset.quads_for_pattern(s, p, o, GraphMatch::Named(g)) {
                        f(q);
                    }
                }
            }
        }
    }
}
