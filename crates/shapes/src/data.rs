// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SHACL engine's data-access surface (C4).
//!
//! SHACL Core reads shared immutable carriers through id-native iteration.
//! Native datasets preserve their local handles; composite and delta sources use
//! compact validation-local handle mappings while borrowing their dictionaries.
//! Pattern lookups answer in [`TermId`]s ([`quads_for_pattern_ids`], returning
//! `Copy` [`QuadIds`]); traversal stays in id space and resolves to the engine's
//! [`Term`] values only at a consumer boundary.
//!
//! [`ShaclData`] is the concrete holder threaded through the engine: it carries
//! the projected and SPARQL datasets, their shared asserted-subclass membership
//! views, and the shapes-graph IRI so the native
//! [`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine) can run
//! SHACL-SPARQL paths over the combined data(+shapes) dataset.

use crate::data_view::{ShaclDatasetView, ShaclRead};
use std::sync::Arc;

use ::purrdf::{GraphMatch, QuadIds};
use ::purrdf::{RdfDataset, TermId};

use crate::class_membership::ClassMembershipView;
use crate::term::{NamedNode, Term, term_id_to_native};

/// Resolve a pattern term to its interned id using variant-specific dataset
/// lookups, recursively resolving the components of a quoted triple. Returns
/// `None` if the term (including any quoted-triple component) is not interned
/// in this dataset, in which case the pattern matches nothing.
pub(crate) fn resolve_id(dataset: &impl ShaclRead, term: &Term) -> Option<TermId> {
    match term {
        Term::NamedNode(node) => dataset.term_id_by_iri(node.as_str()),
        // The native term carries the SCOPE-QUALIFIED label `term_id_to_native`
        // rendered; decode it back to the `(label, scope)` pair the dataset holds.
        // A label a caller MINTED rather than read out of a dataset is raw, not
        // qualified, so the verbatim default-scope lookup is kept as the fallback.
        // (The two spellings differ only for a scoped or marker-prefixed label;
        // every other label is its own qualification.)
        Term::BlankNode(label) => {
            let (decoded, scope) = ::purrdf::BlankScope::unqualify_label(label);
            dataset
                .term_id_by_blank(&decoded, scope)
                .or_else(|| dataset.term_id_by_blank(label, ::purrdf::BlankScope::DEFAULT))
        }
        Term::Literal(literal) => dataset.term_id_by_literal(
            literal.value(),
            literal.datatype_str(),
            literal.language(),
            literal.direction(),
        ),
        Term::Triple(triple) => {
            let s = resolve_id(dataset, &triple.subject)?;
            let p = dataset.term_id_by_iri(triple.predicate.as_str())?;
            let o = resolve_id(dataset, &triple.object)?;
            dataset.term_id_by_triple(s, p, o)
        }
    }
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

/// Which retained Core view a [`TermId`] is addressed against.
///
/// A `TermId` is dataset-local (C0.8) and carries no provenance, so an id from
/// one binding used against another is in range, resolves, and denotes the wrong
/// term. This is the token that says which binding an id space belongs to.
///
/// It is the ADDRESS of the retained [`ShaclDatasetView`], which is exactly the
/// question being asked — "is this the same view?" — and costs nothing to mint,
/// nothing to compare, needs no atomic (so it is identical on `wasm32`, where a
/// 64-bit counter is not free) and cannot wrap around into a false match the way
/// a counter can. The view is retained behind an `Arc` for the whole life of the
/// holder, so the address is stable, and a token is only ever compared against
/// tokens from LIVE holders, so a freed address can never alias a live one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct DatasetIdentity(usize);

/// The concrete data-access holder threaded through the SHACL Core engine.
///
/// Core pattern lookups read `core` (the projected data graph); SHACL-SPARQL paths
/// hand the native SPARQL engine the combined `sparql` dataset (data in the default
/// graph, shapes optionally exposed under `shapes_graph_iri`). Both are held as
/// `Arc`s (usually the SAME frozen graph), so the holder carries no borrow. Their
/// class-membership views share one immutable index when the Arcs are identical.
#[derive(Debug)]
pub struct ShaclData {
    /// The projected data graph, read for Core pattern lookups.
    core: Arc<ShaclDatasetView>,
    /// The combined data(+shapes) dataset handed to the native SPARQL engine.
    sparql: Arc<ShaclDatasetView>,
    /// The effective SHACL instance relation over the Core data graph.
    class_membership: ClassMembershipView,
    /// The same relation over the dataset visible to SHACL-SPARQL.
    sparql_view: ClassMembershipView,
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
        let same = Arc::ptr_eq(&core, &sparql);
        let core = Arc::new(ShaclDatasetView::native(core));
        let sparql = if same {
            Arc::clone(&core)
        } else {
            Arc::new(ShaclDatasetView::native(sparql))
        };
        Self::from_views(core, sparql, shapes_graph_iri)
    }

    /// Retain complete immutable carriers for native and SPARQL validation.
    /// Both views must use compatible typed RDF identities; local term handles
    /// remain private to each view. No owned dataset is materialized here.
    pub fn from_views(
        core: Arc<ShaclDatasetView>,
        sparql: Arc<ShaclDatasetView>,
        shapes_graph_iri: Option<String>,
    ) -> Self {
        let class_membership = ClassMembershipView::from_view(Arc::clone(&core));
        let sparql_view = if Arc::ptr_eq(&core, &sparql) {
            class_membership.clone()
        } else {
            ClassMembershipView::from_view(Arc::clone(&sparql))
        };
        Self {
            core,
            sparql,
            class_membership,
            sparql_view,
            shapes_graph_iri,
        }
    }

    /// Materialize the projected data graph at an explicit compatibility boundary.
    /// Native validation uses [`Self::core_view`] and does not require this copy.
    #[inline]
    pub fn core(&self) -> &RdfDataset {
        self.core.materialized()
    }

    /// Borrow the native validation carrier without materializing its dataset.
    #[inline]
    pub fn core_view(&self) -> &ShaclDatasetView {
        &self.core
    }

    /// Which Core view this holder reads — the token an id-native focus set is
    /// checked against.
    ///
    /// See [`DatasetIdentity`] for what it is and why it is this and not a
    /// counter.
    #[inline]
    pub(crate) fn identity(&self) -> DatasetIdentity {
        DatasetIdentity(Arc::as_ptr(&self.core).cast::<()>() as usize)
    }

    /// Whether the view SHACL-SPARQL runs against addresses the same id space
    /// [`Self::core_view`] does.
    ///
    /// Every id-native surface in this crate resolves against the Core view, and
    /// every SPARQL surface EXECUTES against [`Self::sparql_view`]. Those are the
    /// same view whenever the two datasets were the same `Arc` — the common case,
    /// and what [`Self::from_views`] checks when it decides whether to share one
    /// class-membership index. They are DIFFERENT views when a shapes graph is
    /// exposed under a named graph IRI, and then a Core id handed to the SPARQL view
    /// is in range, resolves, and denotes another term entirely: the composite view
    /// installs a dense handle remapping, so the id is not merely unfound, it is
    /// wrong.
    ///
    /// So this is the predicate that decides whether an id may cross from one
    /// surface to the other. A caller that cannot answer `true` here keeps the
    /// owned-term door, which carries no dataset-local identity and is correct in
    /// both configurations.
    #[inline]
    pub(crate) fn sparql_view_shares_core_ids(&self) -> bool {
        Arc::ptr_eq(&self.core, &self.sparql)
    }

    /// Retain the native validation carrier for repeated prepared bindings.
    pub fn core_view_arc(&self) -> Arc<ShaclDatasetView> {
        Arc::clone(&self.core)
    }

    /// A cloned handle to the projected Core dataset `Arc`.
    ///
    /// Used by the SHACL-AF rules engine, which iterates a fixpoint over ever-larger
    /// projections of the base graph and needs an owned `Arc` to seed each round's
    /// rebuilt dataset (the round driver freezes a fresh graph per round).
    #[inline]
    pub fn core_arc(&self) -> Arc<RdfDataset> {
        Arc::clone(self.core.materialized())
    }

    /// The combined dataset for native SHACL-SPARQL evaluation.
    ///
    /// This compatibility accessor lazily materializes a view once. Native
    /// SHACL-SPARQL validation reads the retained carrier directly.
    #[inline]
    pub fn sparql(&self) -> &Arc<RdfDataset> {
        self.sparql.materialized()
    }

    /// Operational carrier measurements for Core and SPARQL, respectively.
    /// Counters never enter shape, graph or validation-result identity.
    #[must_use]
    pub fn view_stats(&self) -> [crate::data_view::ShaclViewStats; 2] {
        [self.core.stats(), self.sparql.stats()]
    }

    /// The effective asserted-subclass instance relation used by native SHACL
    /// class checks and class targets.
    #[inline]
    pub(crate) fn class_view(&self) -> &ClassMembershipView {
        &self.class_membership
    }

    /// The effective asserted-subclass instance relation visible to every
    /// SHACL-SPARQL query surface.
    #[inline]
    pub(crate) fn sparql_view(&self) -> &ClassMembershipView {
        &self.sparql_view
    }

    /// Force both immutable indexes at a preparation boundary. When Core and
    /// SPARQL share one dataset, the cloned views share one initialization cell.
    pub(crate) fn prepare_class_membership(&self) {
        self.class_membership.prepare();
        self.sparql_view.prepare();
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
/// `resolve_id` first, and for short-circuiting to an empty match when a bound
/// position does not resolve (a term not interned in `ds` matches nothing).
#[inline]
pub fn quads_for_pattern_ids(
    ds: &impl ShaclRead,
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
    ds: &impl ShaclRead,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **`sparql_view_shares_core_ids` answers the question it is asked.**
    ///
    /// It gates the id-native `$this` binding: every SPARQL-bearing validator
    /// resolves its focus nodes against [`ShaclData::core_view`] and EXECUTES
    /// against [`ShaclData::sparql_view`], and a term id may cross between them only
    /// when those are one view. When they are two — which is what exposing a shapes
    /// graph under a named-graph IRI produces — the SPARQL side is a composite view
    /// with its own handle mapping, so a Core id handed to it is in range and
    /// denotes another term. That is the silently-wrong-answer shape, and it is the
    /// whole reason the predicate exists.
    ///
    /// Both directions, because a predicate that always said `false` would be
    /// equally safe and would silently give the saving back, and a predicate that
    /// always said `true` would be the defect itself.
    #[test]
    fn the_id_crossing_predicate_distinguishes_one_view_from_two() {
        let mut b = ::purrdf::RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("freeze");

        // One dataset for both roles: the views are the same `Arc`, so a Core id
        // means the same term on the SPARQL side and may cross.
        let shared = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
        assert!(
            shared.sparql_view_shares_core_ids(),
            "one retained view addressed from both roles is one id space"
        );

        // Two views over the SAME BYTES are still two views. This is the case worth
        // pinning rather than an obviously-unrelated pair: the term tables here are
        // identical, so a predicate that compared CONTENT would say `true` and let an
        // id cross a boundary whose whole hazard is a handle remapping the content
        // cannot see.
        let core = Arc::new(ShaclDatasetView::native(Arc::clone(&dataset)));
        let sparql = Arc::new(ShaclDatasetView::native(dataset));
        let split = ShaclData::from_views(core, sparql, Some("http://example.org/g".to_owned()));
        assert!(
            !split.sparql_view_shares_core_ids(),
            "two retained views are two id spaces, however alike their contents"
        );
    }
}
