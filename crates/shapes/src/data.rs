// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SHACL engine's data-access surface (C4).
//!
//! The SHACL Core engine reads the data graph DIRECTLY from a frozen
//! [`::purrdf::RdfDataset`]'s id-native iteration surface — there is no trait
//! object and no owned per-lookup quad materialization on the hot path. Pattern
//! lookups answer in interned [`TermId`]s ([`quads_for_pattern_ids`], returning
//! `Copy` [`QuadIds`]); the hot traversal (path evaluation, focus resolution)
//! stays in id space and resolves to the engine's native [`Term`] value model
//! only at the boundary. There is NO oxigraph store on this path.
//!
//! [`ShaclData`] is the concrete holder threaded through the engine: it borrows
//! the projected data graph for Core lookups and carries an `Arc<RdfDataset>`
//! (plus the shapes-graph IRI) so the native
//! [`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine) can run
//! SHACL-SPARQL paths over the combined data(+shapes) dataset.

use std::sync::Arc;

use ::purrdf::{DatasetView, GraphMatch, QuadIds};
use ::purrdf::{RdfDataset, TermId};

use crate::term::{term_id_to_native, NamedNode, Term};

/// Resolve a pattern term to its interned id, trying each candidate lookup key
/// ([`Term::lookup_term_values`]) until one resolves. Returns `None` if the term is
/// not interned in this dataset (the pattern then matches nothing).
pub(crate) fn resolve_id(dataset: &RdfDataset, term: &Term) -> Option<TermId> {
    term.lookup_term_values()
        .iter()
        .find_map(|value| dataset.term_id_by_value(value))
}

/// Which graph(s) a pattern lookup ranges over.
///
/// - `AnyGraph` — every graph (named and default);
/// - `DefaultGraph` — the default graph only.
///
/// The IR datasets produced by [`crate::engine::project_dataset`] flatten all
/// quads into the default graph, so the two filters coincide there; the distinction
/// is honored structurally for any named-graph IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFilter {
    /// Match quads in any graph (named or default).
    AnyGraph,
    /// Match quads in the default graph only.
    DefaultGraph,
}

impl GraphFilter {
    /// Map the SHACL graph filter onto the IR's id-native [`GraphMatch`].
    #[inline]
    fn as_graph_match(self) -> GraphMatch {
        match self {
            Self::AnyGraph => GraphMatch::Any,
            Self::DefaultGraph => GraphMatch::Default,
        }
    }
}

/// The concrete data-access holder threaded through the SHACL Core engine.
///
/// Core pattern lookups read `core` (the projected data graph); SHACL-SPARQL paths
/// hand the native SPARQL engine the combined `sparql` dataset (data in the default
/// graph, shapes optionally exposed under `shapes_graph_iri`). Both are held as
/// `Arc`s (usually the SAME frozen graph), so the holder carries no borrow.
#[derive(Debug)]
pub struct ShaclData {
    /// The projected data graph, read for Core pattern lookups.
    core: Arc<RdfDataset>,
    /// The combined data(+shapes) dataset handed to the native SPARQL engine.
    sparql: Arc<RdfDataset>,
    /// The named-graph IRI under which the shapes dataset is exposed, when known.
    shapes_graph_iri: Option<String>,
}

impl ShaclData {
    /// Build a holder from the Core dataset, the SPARQL dataset, and the optional
    /// shapes-graph IRI.
    pub fn new(
        core: Arc<RdfDataset>,
        sparql: Arc<RdfDataset>,
        shapes_graph_iri: Option<String>,
    ) -> Self {
        Self {
            core,
            sparql,
            shapes_graph_iri,
        }
    }

    /// The projected data graph used for Core pattern lookups.
    #[inline]
    pub fn core(&self) -> &RdfDataset {
        &self.core
    }

    /// The combined dataset for native SHACL-SPARQL evaluation.
    #[inline]
    pub fn sparql_dataset(&self) -> Arc<RdfDataset> {
        Arc::clone(&self.sparql)
    }

    /// The IRI of the named graph under which the shapes graph is exposed to
    /// SHACL-SPARQL queries, if any.
    #[inline]
    pub fn shapes_graph_iri(&self) -> Option<&str> {
        self.shapes_graph_iri.as_deref()
    }
}

/// Id-native pattern lookup: all quads matching `(s?, p?, o?)` under `graph`, in
/// interned [`TermId`]s. A `None` position is a wildcard.
///
/// The caller is responsible for resolving any BOUND pattern term to its id via
/// [`resolve_id`] first, and for short-circuiting to an empty match when a bound
/// position does not resolve (a term not interned in `ds` matches nothing).
#[inline]
pub fn quads_for_pattern_ids(
    ds: &RdfDataset,
    s: Option<TermId>,
    p: Option<TermId>,
    o: Option<TermId>,
    graph: GraphFilter,
) -> impl Iterator<Item = QuadIds> + '_ {
    ds.quads_for_pattern(s, p, o, graph.as_graph_match())
}

/// A cold-path (parser / report / JSON-projection) pattern lookup that
/// materializes matched quads into the native [`Term`] value model.
///
/// Bound pattern terms are resolved once; a bound term not interned in `ds`
/// short-circuits to an empty result. A matched quad whose subject is not a legal
/// subject (never for a frozen quad) or whose predicate is not an IRI is skipped.
///
/// This owned materialization is for the cold paths only — the hot Core traversal
/// ([`crate::path`], [`crate::engine`]) stays in id space via
/// [`quads_for_pattern_ids`].
pub fn native_quads(
    ds: &RdfDataset,
    subject: Option<&Term>,
    predicate: Option<&Term>,
    object: Option<&Term>,
    graph: GraphFilter,
) -> Vec<(Term, NamedNode, Term)> {
    let s_id = match subject {
        Some(t) => match resolve_id(ds, t) {
            Some(id) => Some(id),
            None => return Vec::new(),
        },
        None => None,
    };
    let p_id = match predicate {
        Some(t) => match resolve_id(ds, t) {
            Some(id) => Some(id),
            None => return Vec::new(),
        },
        None => None,
    };
    let o_id = match object {
        Some(t) => match resolve_id(ds, t) {
            Some(id) => Some(id),
            None => return Vec::new(),
        },
        None => None,
    };

    let mut out = Vec::new();
    for q in quads_for_pattern_ids(ds, s_id, p_id, o_id, graph) {
        let s = term_id_to_native(ds, q.s);
        if !s.is_subject() {
            continue;
        }
        let Term::NamedNode(predicate) = term_id_to_native(ds, q.p) else {
            continue;
        };
        let object = term_id_to_native(ds, q.o);
        out.push((s, predicate, object));
    }
    out
}
