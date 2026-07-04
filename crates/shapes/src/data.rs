// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SHACL engine's data-access abstraction (C4).
//!
//! [`ShaclDataGraph`] is the single seam through which the SHACL Core engine reads
//! the data graph. The engine, constraint evaluator, and path evaluator are all
//! generic over it. It is now IR-native: pattern lookups are answered directly from
//! a frozen [`::purrdf::RdfDataset`]'s iteration surface
//! ([`RdfDataset::quad_refs`]), converting matched IR terms to the engine's native
//! [`Term`] value model at the boundary. There is NO oxigraph store on this path.
//!
//! SHACL-AF SPARQL paths (`sh:SPARQLTarget` / `sh:sparql`) need a SPARQL engine; the
//! trait exposes the borrowed `Arc<RdfDataset>` so the native
//! [`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine) can run over it.

use std::sync::Arc;

use ::purrdf::{DatasetView, GraphMatch};
use ::purrdf::{RdfDataset, TermId};

use crate::term::{term_id_to_native, NamedNode, Term};

/// Resolve a pattern term to its interned id, trying each candidate lookup key
/// ([`Term::lookup_term_values`]) until one resolves. Returns `None` if the term is
/// not interned in this dataset (the pattern then matches nothing).
fn resolve_id(dataset: &RdfDataset, term: &Term) -> Option<TermId> {
    term.lookup_term_values()
        .iter()
        .find_map(|value| dataset.term_id_by_value(value))
}

/// Which graph(s) a pattern lookup ranges over.
///
/// - `AnyGraph` — every graph (named and default);
/// - `DefaultGraph` — the default graph only.
///
/// The IR datasets produced by [`crate::engine::validate_dataset`] flatten all
/// quads into the default graph, so the two filters coincide there; the distinction
/// is honored structurally for any named-graph IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFilter {
    /// Match quads in any graph (named or default).
    AnyGraph,
    /// Match quads in the default graph only.
    DefaultGraph,
}

/// A native quad: the engine's value model in subject/predicate/object positions.
///
/// `predicate` is always an IRI; `subject` is an IRI or blank node (the engine
/// never queries with a literal subject pattern).
#[derive(Clone, Debug)]
pub struct Quad {
    /// The subject term (IRI or blank node).
    pub subject: Term,
    /// The predicate IRI.
    pub predicate: NamedNode,
    /// The object term.
    pub object: Term,
}

/// The data-access surface the SHACL Core engine reads through.
///
/// Implementations answer triple-pattern lookups, returning native [`Quad`] values,
/// and expose the borrowed dataset for the native SPARQL paths.
///
/// The `Send + Sync` bound readies the seam for parallel focus-node validation; it
/// is non-breaking — the IR backends hold only `Sync` frozen data.
pub trait ShaclDataGraph: Send + Sync {
    /// All quads matching `(subject?, predicate?, object?)` under `graph`.
    ///
    /// A `None` position is a wildcard. A subject/predicate pattern that cannot
    /// legally occupy its position (a literal subject, a non-IRI predicate) matches
    /// nothing.
    fn quads_for_pattern(
        &self,
        subject: Option<&Term>,
        predicate: Option<&Term>,
        object: Option<&Term>,
        graph: GraphFilter,
    ) -> Vec<Quad>;

    /// The borrowed frozen dataset, for native SHACL-SPARQL evaluation.
    fn sparql_dataset(&self) -> Arc<RdfDataset>;

    /// The IRI of the named graph under which the shapes graph is exposed to
    /// SHACL-SPARQL queries, if any.
    fn shapes_graph_iri(&self) -> Option<&str> {
        None
    }
}

// ── &RdfDataset backend (the IR-native path) ───────────────────────────────────

/// An IR-native [`ShaclDataGraph`] that owns an `Arc` of the frozen dataset.
///
/// Pattern lookups iterate the frozen quad table; the SPARQL paths hand the engine
/// the same `Arc<RdfDataset>` (no materialization — the native engine reads the IR
/// directly).
#[derive(Debug)]
pub struct IrDataGraph {
    dataset: Arc<RdfDataset>,
}

impl IrDataGraph {
    /// Wrap a borrowed frozen dataset.
    pub fn new(dataset: Arc<RdfDataset>) -> Self {
        Self { dataset }
    }

    /// The underlying frozen dataset.
    pub fn dataset(&self) -> &RdfDataset {
        &self.dataset
    }
}

impl ShaclDataGraph for IrDataGraph {
    fn quads_for_pattern(
        &self,
        subject: Option<&Term>,
        predicate: Option<&Term>,
        object: Option<&Term>,
        graph: GraphFilter,
    ) -> Vec<Quad> {
        let dataset = self.dataset.as_ref();

        // Resolve each bound pattern term to its interned id ONCE. A bound term that
        // is not interned in this dataset matches nothing, so we short-circuit to an
        // empty result without scanning. `None` (wildcard) stays `None`. A blank node
        // may have several candidate keys (default-scope qualified label vs original
        // `(label, scope)`); the first that resolves wins.
        let s_id = match subject {
            Some(t) => match resolve_id(dataset, t) {
                Some(id) => Some(id),
                None => return Vec::new(),
            },
            None => None,
        };
        let p_id = match predicate {
            Some(t) => match resolve_id(dataset, t) {
                Some(id) => Some(id),
                None => return Vec::new(),
            },
            None => None,
        };
        let o_id = match object {
            Some(t) => match resolve_id(dataset, t) {
                Some(id) => Some(id),
                None => return Vec::new(),
            },
            None => None,
        };

        // A flattened IR has `g == None` (default graph) for every quad, so
        // `DefaultGraph` and `AnyGraph` coincide; honor the distinction structurally
        // for any named-graph IR via the index's graph filter.
        let graph_match = match graph {
            GraphFilter::AnyGraph => GraphMatch::Any,
            GraphFilter::DefaultGraph => GraphMatch::Default,
        };

        // Indexed pattern lookup (P4b): only matched quads are materialized into
        // native terms — no whole-table scan per call.
        let mut out = Vec::new();
        for q in dataset.quads_for_pattern(s_id, p_id, o_id, graph_match) {
            let s = term_id_to_native(dataset, q.s);
            // Subject must be IRI/blank and predicate an IRI; a frozen quad always
            // satisfies this, but guard rather than panic.
            if !s.is_subject() {
                continue;
            }
            let Term::NamedNode(predicate) = term_id_to_native(dataset, q.p) else {
                continue;
            };
            let object = term_id_to_native(dataset, q.o);
            out.push(Quad {
                subject: s,
                predicate,
                object,
            });
        }
        out
    }

    fn sparql_dataset(&self) -> Arc<RdfDataset> {
        Arc::clone(&self.dataset)
    }
}
