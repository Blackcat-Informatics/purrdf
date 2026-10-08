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
use std::sync::{Arc, OnceLock};

use ::purrdf_rdf::{GraphMatch, QuadIds};
use ::purrdf_rdf::{RdfDataset, TermId};

use crate::class_membership::ClassMembershipView;
use crate::term::{NamedNode, Term, term_id_to_native};

/// Resolve a pattern term to its interned id using variant-specific dataset
/// lookups, through the components of a quoted triple. Returns `None` if the term
/// (including any quoted-triple component) is not interned in this dataset, in which
/// case the pattern matches nothing.
///
/// A quoted triple is resolved over [`Term::fold_nested`]'s work list: its subject,
/// predicate and object, each fully before the next, and the first component the
/// dataset lacks ends the lookup.
pub(crate) fn resolve_id(dataset: &impl ShaclRead, term: &Term) -> Option<TermId> {
    term.fold_nested(
        &mut (),
        |(), term| {
            match term {
                Term::NamedNode(node) => dataset.term_id_by_iri(node.as_str()),
                // The native term carries the SCOPE-QUALIFIED label `term_id_to_native`
                // rendered; decode it back to the `(label, scope)` pair the dataset
                // holds. A label a caller MINTED rather than read out of a dataset is
                // raw, not qualified, so the verbatim default-scope lookup is kept as
                // the fallback. (The two spellings differ only for a scoped or
                // marker-prefixed label; every other label is its own qualification.)
                Term::BlankNode(label) => {
                    let (decoded, scope) = ::purrdf_rdf::BlankScope::unqualify_label(label);
                    dataset.term_id_by_blank(&decoded, scope).or_else(|| {
                        dataset.term_id_by_blank(label, ::purrdf_rdf::BlankScope::DEFAULT)
                    })
                }
                Term::Literal(literal) => dataset.term_id_by_literal(
                    literal.value(),
                    literal.datatype_str(),
                    literal.language(),
                    literal.direction(),
                ),
                Term::Triple(_) => unreachable!("a quoted triple is folded from its parts"),
            }
            .ok_or(())
        },
        |(), predicate| dataset.term_id_by_iri(predicate.as_str()).ok_or(()),
        |(), s, p, o| dataset.term_id_by_triple(s, p, o).ok_or(()),
    )
    .ok()
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
    /// The NODES of the Core data graph, derived on first use and shared by every
    /// `sh:targetWhere` the bound shapes declare (see [`Self::graph_nodes`]).
    graph_nodes: OnceLock<Box<[TermId]>>,
}

impl ShaclData {
    /// Retain the exact two read views and their shared class analysis for a
    /// cold request binding. The optional whole-node memo stays cold rather
    /// than copying a warmed graph-sized buffer.
    pub(crate) fn retained(&self) -> Self {
        Self {
            core: Arc::clone(&self.core),
            sparql: Arc::clone(&self.sparql),
            class_membership: self.class_membership.clone(),
            sparql_view: self.sparql_view.clone(),
            shapes_graph_iri: self.shapes_graph_iri.clone(),
            graph_nodes: OnceLock::new(),
        }
    }

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
        Self::from_views_with_supplement(core, sparql, shapes_graph_iri, &[])
    }

    /// [`Self::from_views`], with SHACL type also following `supplement`: the
    /// `(subclass, superclass)` IRI pairs of the shapes graph's `rdfs:subClassOf`
    /// triples, when the caller asked for them (SHACL 1.2 Core §6.3,
    /// `subClassOfInShapesGraph`). Both views must intern every IRI of the pairs.
    pub(crate) fn from_views_with_supplement(
        core: Arc<ShaclDatasetView>,
        sparql: Arc<ShaclDatasetView>,
        shapes_graph_iri: Option<String>,
        supplement: &[(String, String)],
    ) -> Self {
        let class_membership =
            ClassMembershipView::from_view_with_supplement(Arc::clone(&core), supplement);
        let sparql_view = if Arc::ptr_eq(&core, &sparql) {
            class_membership.clone()
        } else {
            ClassMembershipView::from_view_with_supplement(Arc::clone(&sparql), supplement)
        };
        Self {
            core,
            sparql,
            class_membership,
            sparql_view,
            shapes_graph_iri,
            graph_nodes: OnceLock::new(),
        }
    }

    /// The NODES of the Core data graph, as RDF 1.2 Concepts defines them: "The
    /// set of nodes of an RDF graph is the set of subjects and objects of the
    /// asserted triples of the graph." Across every graph of the dataset, as every
    /// Core lookup reads, deduplicated and in first-seen order.
    ///
    /// A triple term that is the OBJECT of an asserted triple is a node; a term
    /// that occurs only INSIDE a triple term is not, because the triple term's
    /// constituents are not subjects or objects of an asserted triple.
    ///
    /// Built once per holder, on the first `sh:targetWhere` that has to scan the
    /// whole graph, and reused by every later one; a shapes graph with none never
    /// pays for it.
    pub(crate) fn graph_nodes(&self) -> &[TermId] {
        self.graph_nodes.get_or_init(|| {
            let mut seen = ::purrdf_rdf::IdSet::default();
            let mut nodes: Vec<TermId> = Vec::new();
            for quad in quads_for_pattern_ids(&*self.core, None, None, None, GraphFilter::AnyGraph)
            {
                for node in [quad.s, quad.o] {
                    if seen.insert(node) {
                        nodes.push(node);
                    }
                }
            }
            nodes.into_boxed_slice()
        })
    }

    /// Whether `node` is a NODE of the Core data graph (see
    /// [`Self::graph_nodes`]): the subject or the object of an asserted triple.
    pub(crate) fn is_graph_node(&self, node: TermId) -> bool {
        quads_for_pattern_ids(&*self.core, Some(node), None, None, GraphFilter::AnyGraph)
            .next()
            .is_some()
            || quads_for_pattern_ids(&*self.core, None, None, Some(node), GraphFilter::AnyGraph)
                .next()
                .is_some()
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

    /// This holder with SHACL type also following `supplement`, the `(subclass,
    /// superclass)` IRI pairs of the shapes graph's `rdfs:subClassOf` triples (SHACL 1.2
    /// Core §6.3, `subClassOfInShapesGraph`).
    ///
    /// The rows both views read are unchanged. An IRI of the pairs the Core view does
    /// not intern — a class only the shapes graph names — is added to both views' term
    /// tables ([`ShaclDatasetView::with_extra_terms`]), so a class target or
    /// `sh:class` naming it resolves and derives its members through the supplement.
    ///
    /// # Errors
    /// A composite or handle mapping exceeding the default view limits.
    pub(crate) fn with_class_supplement(
        &self,
        supplement: &[(String, String)],
    ) -> Result<Self, String> {
        let mut missing: Vec<&str> = supplement
            .iter()
            .flat_map(|(child, parent)| [child.as_str(), parent.as_str()])
            .filter(|iri| self.core.term_id_by_iri(iri).is_none())
            .collect();
        missing.sort_unstable();
        missing.dedup();
        let (core, sparql) = if missing.is_empty() {
            (Arc::clone(&self.core), Arc::clone(&self.sparql))
        } else {
            let mut terms = ::purrdf_rdf::RdfDatasetBuilder::new();
            for iri in missing {
                terms.intern_iri(iri);
            }
            let terms = terms
                .freeze()
                .map_err(|error| format!("the subClassOfInShapesGraph terms: {error}"))?;
            let limits = ::purrdf_rdf::ir::ViewLimits::default();
            let core = Arc::new(self.core.with_extra_terms(Arc::clone(&terms), limits)?);
            let sparql = if self.sparql_view_shares_core_ids() {
                Arc::clone(&core)
            } else {
                Arc::new(self.sparql.with_extra_terms(terms, limits)?)
            };
            (core, sparql)
        };
        Ok(Self::from_views_with_supplement(
            core,
            sparql,
            self.shapes_graph_iri.clone(),
            supplement,
        ))
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

/// The distinct objects of `(subject, predicate, ?)` in any graph, as native
/// terms: [`DatasetView::objects`](::purrdf_rdf::DatasetView::objects), lifted from
/// the native term and IRI a cold-path reader holds. Empty when `subject` or
/// `predicate` is not interned, so neither can have a statement.
pub(crate) fn objects_of(ds: &impl ShaclRead, subject: &Term, predicate: &str) -> Vec<Term> {
    let (Some(subject), Some(predicate)) = (resolve_id(ds, subject), ds.term_id_by_iri(predicate))
    else {
        return Vec::new();
    };
    ds.objects(subject, predicate, GraphMatch::Any)
        .into_iter()
        .map(|object| term_id_to_native(ds, object))
        .collect()
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
        let mut b = ::purrdf_rdf::RdfDatasetBuilder::new();
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

#[cfg(test)]
mod term_walk_tests {
    //! The pattern-term lookup against its recursive reference.

    use purrdf_core::backend::TermFactory as _;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermId};

    use super::resolve_id;
    use crate::term::Term;
    use crate::term::term_walk_tests::generated;

    fn reference(dataset: &RdfDataset, term: &Term) -> Option<TermId> {
        match term {
            Term::Triple(triple) => {
                let s = reference(dataset, &triple.subject)?;
                let p = dataset.term_id_by_iri(triple.predicate.as_str())?;
                let o = reference(dataset, &triple.object)?;
                dataset.term_id_by_triple(s, p, o)
            }
            leaf => resolve_id(dataset, leaf),
        }
    }

    /// Every generated term — stored in the dataset, and not — resolves exactly as the
    /// recursive reference resolves it.
    #[test]
    fn the_lookup_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut found = 0;
        for seed in 0..300_u64 {
            let stored = generated(seed);
            let mut builder = RdfDatasetBuilder::new();
            let object = builder.intern_value(&stored.to_term_value());
            let holder = builder.intern_iri("http://example.org/holder");
            builder.push_quad(holder, holder, object, None);
            let dataset = builder.freeze().expect("a generated term freezes");
            for term in [stored, generated(seed + 5_000)] {
                let id = resolve_id(&*dataset, &term);
                assert_eq!(id, reference(&dataset, &term), "seed {seed}");
                found += usize::from(id.is_some());
            }
        }
        assert!(found > 0, "some generated term is found");
    }
}
