// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL Core validation engine.
//!
//! `validate` is the top-level entry point.  Resolves focus nodes for every
//! non-deactivated node shape, runs all constraints, and assembles a
//! deterministically-sorted [`ValidationReport`].

use crate::data_view::{ShaclDatasetView, ShaclRead};
use crate::footprint::Endpoint;
use std::sync::{Arc, OnceLock};

use ::purrdf::{DatasetView, FastMap, FastSet, IdSet, RdfDataset, TermId};

use purrdf_sparql_eval::{GovernorEvidence, GovernorState, QueryGovernors, TrippedGovernor};

use crate::data::{DatasetIdentity, GraphFilter, ShaclData, quads_for_pattern_ids, resolve_id};
use crate::plan::{ClassCatalog, DatasetBinding, LoweredShapes, PreparedTargets, ShapePlan};
use crate::provenance::ValidatorProvenance;
use crate::report::ValidationReport;
use crate::shapes::{Shape, Shapes, Target};
use crate::term::{
    NamedNode, Term, canonical_cmp, canonical_cmp_id_term, canonical_cmp_ids, term_id_to_native,
};

// ── Target resolution helpers ─────────────────────────────────────────────────

/// Canonically order focus nodes without allocating one rendered key per node —
/// and now without materializing the nodes themselves.
///
/// The order is the byte order of the rendered terms, which is what
/// [`finish_report`] keys on and what the pinned report golden holds. An interned
/// focus node's key is derived from what its id DENOTES, never from the id's
/// numeric value: ids are handed out in insertion order, so comparing them would
/// produce a deterministic order that is simply not the canonical one — and it
/// would pass an all-interned test and an all-foreign test alike, since both
/// would merely be self-consistent. See [`canonical_cmp_ids`].
///
/// Large target sets use Rayon's deterministic parallel sort; small bounded
/// requests avoid scheduler overhead.
///
/// # Why an UNSTABLE sort, and why that changes nothing
///
/// [`focus_cmp`] is a strict total order over the set it is handed: every focus
/// set reaching here has been deduplicated (by id for interned nodes, by term for
/// foreign ones), two distinct ids in one dataset denote two distinct term values
/// because interning is by value, and where the rendered bytes nonetheless
/// coincide the comparator falls through to the ids themselves. A strict total
/// order admits exactly one sorted permutation, so a stable and an unstable sort
/// return the same array — and the unstable pair allocates NOTHING, where the
/// stable pair allocates a merge scratch buffer whose size, and whose very
/// existence on the parallel branch, is a function of the focus count. That was
/// the last growth term standing between this route and
/// `delta(2N) == delta(N)`, and it was a step at exactly the parallel threshold
/// rather than a slope, which is the kind a per-node figure hides completely.
fn sort_focus_nodes(dataset: &impl ShaclRead, nodes: &mut [FocusNode]) {
    const PARALLEL_SORT_MIN_NODES: usize = 4_096;

    let order = |left: &FocusNode, right: &FocusNode| focus_cmp(dataset, left, right);
    if nodes.len() >= PARALLEL_SORT_MIN_NODES && rayon::current_num_threads() > 1 {
        use rayon::prelude::*;
        nodes.par_sort_unstable_by(order);
    } else {
        nodes.sort_unstable_by(order);
    }
}

/// [`canonical_cmp`] over focus nodes in either representation.
///
/// All four pairings reduce to the same rendered-byte order: two interned nodes
/// through the interner, two foreign nodes through their owned terms, and a mixed
/// pair by streaming one of each. The mixed arm is the one a same-shape test
/// never reaches, and the one that decides whether a focus set holding both kinds
/// is ordered or merely partitioned.
///
/// The interned/interned arm carries a final tiebreak on the ids, and it is the
/// only place in this file where an id's NUMBER is looked at. It is not the sort
/// key and it cannot become one: it is consulted only after the canonical
/// renderings have compared EQUAL, which for two distinct dataset terms takes a
/// literal whose datatype is one of the two the rendering suppresses. That makes
/// the order strict and total, which is what lets the sort be unstable.
fn focus_cmp(dataset: &impl ShaclRead, left: &FocusNode, right: &FocusNode) -> std::cmp::Ordering {
    match (left, right) {
        (FocusNode::Interned(left), FocusNode::Interned(right)) => {
            canonical_cmp_ids(dataset, *left, *right).then_with(|| left.index().cmp(&right.index()))
        }
        (FocusNode::Foreign(left), FocusNode::Foreign(right)) => canonical_cmp(left, right),
        (FocusNode::Interned(left), FocusNode::Foreign(right)) => {
            canonical_cmp_id_term(dataset, *left, right)
        }
        (FocusNode::Foreign(left), FocusNode::Interned(right)) => {
            canonical_cmp_id_term(dataset, *right, left).reverse()
        }
    }
}

/// Resolve a predicate IRI to its interned id in the Core dataset, if present.
///
/// Direct IRI lookup — exactly `resolve_id`'s `Term::NamedNode` arm — without
/// cloning the predicate into a temporary owned `Term` per target resolution.
#[inline]
fn resolve_pred(ds: &impl ShaclRead, pred: &NamedNode) -> Option<TermId> {
    ds.term_id_by_iri(pred.as_str())
}

/// Collect distinct subjects of `(?, pred, ?)` across all graphs. Dedup is on the
/// interned [`TermId`] (`Copy`).
fn subjects_of(ds: &impl ShaclRead, pred: &NamedNode) -> Vec<TermId> {
    let Some(pid) = resolve_pred(ds, pred) else {
        return Vec::new();
    };
    let mut seen: IdSet = IdSet::default();
    let mut result = Vec::new();
    for q in quads_for_pattern_ids(ds, None, Some(pid), None, GraphFilter::AnyGraph) {
        if seen.insert(q.s) {
            result.push(q.s);
        }
    }
    result
}

/// Collect distinct objects of `(?, pred, ?)` across all graphs. Dedup is on the
/// interned [`TermId`] (`Copy`).
fn objects_of(ds: &impl ShaclRead, pred: &NamedNode) -> Vec<TermId> {
    let Some(pid) = resolve_pred(ds, pred) else {
        return Vec::new();
    };
    let mut seen: IdSet = IdSet::default();
    let mut result = Vec::new();
    for q in quads_for_pattern_ids(ds, None, Some(pid), None, GraphFilter::AnyGraph) {
        if seen.insert(q.o) {
            result.push(q.o);
        }
    }
    result
}

/// A focus node identity together with the binding it was minted against.
///
/// # Why a bare [`TermId`] is not enough
///
/// A `TermId` is an index into ONE binding's term table. It carries no
/// provenance, so an id minted against binding A is, handed to binding B, very
/// probably in range — and it then resolves, dispatches, and validates a
/// DIFFERENT node, quietly and with a conforming report to show for it. Two live
/// id spaces is not an exotic misuse either: it is the designed situation the
/// moment a caller binds a mutation snapshot beside the base binding through
/// [`PreparedShapes::bind_delta_with_shapes_graph`].
///
/// So the provenance travels with the id instead of being delegated to the
/// caller. A `FocusId` can only be obtained from a binding — from
/// [`PreparedValidator::term_id`] or from
/// [`PreparedValidator::affected_focus_node_ids`] — and
/// [`PreparedValidator::validate_focus_node_ids`] refuses one that names a
/// different binding. There is no public constructor and no public field,
/// because either would be a way to mint an id with a provenance it does not
/// have.
///
/// # The check is binding-scoped, not dataset-scoped
///
/// A binding's identity is the retained view it was bound over, not the dataset
/// underneath it, so binding the SAME dataset twice produces two identities and
/// an id minted against one is refused by the other — even though both term
/// tables are byte-identical and the id would have resolved correctly. That
/// refusal is deliberate. A token loose enough to admit it would have to reason
/// about how a delta view remaps ids locally, and the failure it would then be
/// unable to catch is the silent one: validating the wrong node and reporting
/// conformance for it. Refusing a portable id costs a caller one visible error
/// telling them which binding to mint from; accepting a non-portable one costs
/// them a wrong answer they never learn about. Mint from the binding you are
/// about to validate against and the question does not arise.
///
/// # Cost
///
/// Two words where a `TermId` was one, and no allocation per focus node: the
/// change-path entry point compares one `usize` per id and is otherwise
/// unchanged. `crates/shapes/tests/change_path_alloc.rs` pins that.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FocusId {
    /// Which binding's term table [`Self::id`] indexes.
    dataset: DatasetIdentity,
    /// The dataset-local identity itself.
    id: TermId,
}

impl FocusId {
    /// Mint an id against the binding that resolved it.
    ///
    /// `pub(crate)` on purpose: minting is the act that asserts provenance, so
    /// it stays with the code that actually performed the resolution.
    #[inline]
    pub(crate) const fn new(dataset: DatasetIdentity, id: TermId) -> Self {
        Self { dataset, id }
    }

    /// The dataset-local identity this names.
    ///
    /// Read-only, and deliberately one-way: a `TermId` can be logged, compared
    /// against another of the same binding, or handed to a lower-level view, but
    /// it cannot be turned back into a `FocusId` without a binding to mint it.
    #[must_use]
    #[inline]
    pub const fn term_id(self) -> TermId {
        self.id
    }
}

/// Dataset-bound invariant state shared by every focus evaluation in one pass.
///
/// The expansion of a graph change into the focus nodes it can move, as answered
/// by [`PreparedValidator::affected_focus_node_ids`].
///
/// Two answers, and the second is the point of the type. A bounded superset is what
/// incremental validation wants; a shapes graph whose reads hide inside query text
/// has no bounded superset anyone can derive from it, and saying so is the only
/// honest alternative to returning a set that LOOKS complete and is not. An
/// under-approximation here does not fail — it reports `conforms` about a node
/// nobody re-checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusExpansion {
    /// Every focus node the change can move, plus whatever the over-approximation
    /// swept in, as identities of the binding that produced them and in ascending
    /// id order. Hand it straight to
    /// [`PreparedValidator::validate_focus_node_ids`].
    ///
    /// Empty means exactly what it says: nothing the shapes graph reads changed.
    Bounded(Vec<FocusId>),
    /// No bounded superset exists for this shapes graph, so the only sound
    /// re-validation is [`PreparedValidator::validate`].
    Everything {
        /// Which construct made the footprint unreadable, for a caller who wants
        /// to know what to change to get incremental validation back.
        reason: &'static str,
    },
}

impl FocusExpansion {
    /// The bounded expansion, or `None` when the footprint is TOP.
    ///
    /// Deliberately NOT a `Default`-flavoured accessor that hands back an empty
    /// slice for the TOP case: an empty slice and "every node in the graph" are
    /// opposite instructions, and collapsing them is the drop this type exists to
    /// prevent.
    #[must_use]
    pub fn ids(&self) -> Option<&[FocusId]> {
        match self {
            Self::Bounded(ids) => Some(ids),
            Self::Everything { .. } => None,
        }
    }

    /// Why the footprint is TOP, or `None` when the expansion is bounded.
    #[must_use]
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Self::Bounded(_) => None,
            Self::Everything { reason } => Some(reason),
        }
    }

    /// Whether this expansion requires a full validation.
    #[must_use]
    pub fn is_everything(&self) -> bool {
        matches!(self, Self::Everything { .. })
    }
}

/// The class analysis (dataset-independent) and the shape lowering it came out of
/// are shared by `Arc`; only the resolved identities are rebuilt per dataset.
///
/// `targets` is positionally parallel to the node-shape list this was bound from.
/// A deactivated shape still occupies its position with an empty index rather than
/// being skipped, because a caller enumerating shapes and a caller enumerating
/// target indexes have to agree on what the n-th entry is.
#[derive(Debug)]
pub(crate) struct BoundShapes {
    lowered: Arc<LoweredShapes>,
    classes: Arc<ClassCatalog>,
    binding: DatasetBinding,
    targets: Vec<PreparedTargets>,
    /// `targets` inverted onto the node, for the bounded (change-path) dispatch.
    /// Derived from `targets` alone, so it is stage-1 work like `targets` itself
    /// and never repeated per focus node.
    dispatch: TargetDispatch,
}

impl BoundShapes {
    /// Resolve every dataset identity `lowered` asked for, then index each shape's
    /// active targets.
    fn bind(
        data: &ShaclData,
        shapes: &[Shape],
        lowered: Arc<LoweredShapes>,
        classes: Arc<ClassCatalog>,
    ) -> Result<Self, String> {
        let binding = lowered.bind(data.core_view(), &classes);
        let targets = shapes
            .iter()
            .map(|shape| {
                if shape.deactivated {
                    Ok(PreparedTargets::default())
                } else {
                    PreparedTargets::for_shape(data, shape, &binding, &classes)
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        let dispatch = TargetDispatch::invert(&targets);
        Ok(Self {
            lowered,
            classes,
            binding,
            targets,
            dispatch,
        })
    }

    /// Bind without indexing any targets, for the entry points that resolve focus
    /// nodes directly from the shape's declarations instead of from an index.
    fn bind_untargeted(
        data: &ShaclData,
        lowered: Arc<LoweredShapes>,
        classes: Arc<ClassCatalog>,
    ) -> Self {
        let binding = lowered.bind(data.core_view(), &classes);
        Self {
            lowered,
            classes,
            binding,
            targets: Vec::new(),
            dispatch: TargetDispatch::default(),
        }
    }

    /// The dependency footprint the shapes-lowering walk derived.
    fn footprint(&self) -> &crate::footprint::Footprint {
        self.lowered.footprint()
    }

    /// The plan for the `position`-th shape, with its prepared target index.
    fn plan<'a>(&'a self, shape: &'a Shape, position: usize) -> Result<ShapePlan<'a>, String> {
        let targets = self
            .targets
            .get(position)
            .unwrap_or_else(|| self.lowered.no_targets());
        self.lowered
            .plan(shape, position, &self.binding, &self.classes, targets)
    }

    /// The inverted target index behind this binding.
    fn dispatch(&self) -> &TargetDispatch {
        &self.dispatch
    }

    /// The class analysis behind this binding.
    fn classes(&self) -> &ClassCatalog {
        &self.classes
    }

    /// The resolved identities behind this binding.
    fn binding(&self) -> &DatasetBinding {
        &self.binding
    }
}

impl PreparedTargets {
    /// Resolve one shape's declared targets against an already-bound dataset.
    ///
    /// The two target families are treated differently on purpose. A Core target
    /// keyed by class or predicate becomes a membership index — the target NODES
    /// are never enumerated, so a bounded request tests only the candidates it
    /// was given instead of paying for the whole extension of `sh:targetClass`.
    /// `sh:targetNode` and a SHACL-SPARQL `sh:target` cannot be answered by a
    /// pattern lookup, so their results are resolved once, here.
    ///
    /// A class or predicate the data graph never interned yields an EMPTY target
    /// set rather than a failure: a shapes graph naming what this data graph does
    /// not is ordinary, and refusing it here would reject valid input.
    ///
    /// # Errors
    ///
    /// Returns an error when a SHACL-SPARQL target fails to evaluate — an
    /// unanswerable target is not an empty one.
    fn for_shape(
        data: &ShaclData,
        shape: &Shape,
        binding: &DatasetBinding,
        classes: &ClassCatalog,
    ) -> Result<Self, String> {
        let mut prepared = Self::default();
        for target in &shape.targets {
            match target {
                Target::Class(class) | Target::ImplicitClass(Term::NamedNode(class)) => {
                    // `None` = the data graph names no such class, so the target
                    // set is empty; that is not a preparation failure.
                    if let Some(class) = binding.class_id(classes, class)? {
                        prepared.target_class_ids.insert(class);
                    }
                }
                Target::SubjectsOf(predicate) => {
                    if let Some(id) = resolve_pred(data.core_view(), predicate) {
                        prepared.subject_predicates.insert(id);
                    }
                }
                Target::ObjectsOf(predicate) => {
                    if let Some(id) = resolve_pred(data.core_view(), predicate) {
                        prepared.object_predicates.insert(id);
                    }
                }
                Target::Node(term) => prepared.insert_explicit(data.core_view(), term.clone()),
                Target::Sparql {
                    select,
                    substitutions,
                } => {
                    let candidates =
                        crate::sparql::eval_target_view(data.sparql_view(), select, substitutions)
                            .map_err(|error| format!("sh:target SPARQLTarget failed: {error}"))?;
                    for candidate in candidates {
                        prepared.insert_explicit(data.core_view(), candidate);
                    }
                }
                Target::ImplicitClass(_) => {}
            }
        }
        Ok(prepared)
    }

    fn insert_explicit(&mut self, dataset: &impl ShaclRead, term: Term) {
        if let Some(id) = resolve_id(dataset, &term) {
            self.explicit_ids.insert(id);
        } else {
            self.explicit_foreign.insert(term);
        }
    }

    /// Whether this shape's targets contain `focus` — **the definition** of
    /// bounded-target membership.
    ///
    /// It is no longer the dispatch mechanism. Answering it once per
    /// (shape, focus node) pair re-derives the focus node's class memberships and
    /// re-scans its outgoing and incoming quads once per shape, so it is
    /// quadratic in exactly the regime the change path exists to serve;
    /// [`TargetDispatch`] inverts the same facts once at bind time and answers
    /// the dual question instead.
    ///
    /// The definition stays executable, and is executed:
    /// `target_dispatch_agrees_with_contains_and_resolve_all` drives it over
    /// every [`Target`] variant and requires the index and
    /// [`Self::resolve_all`] to select the identical focus set. Two
    /// implementations of one predicate with no equivalence test is how they come
    /// to disagree; this one has three, and one test binding all of them.
    #[cfg(test)]
    fn contains(&self, data: &ShaclData, focus: &FocusNode) -> bool {
        let Some(id) = focus.id() else {
            return focus
                .foreign()
                .is_some_and(|term| self.explicit_foreign.contains(term));
        };
        if self.explicit_ids.contains(&id) {
            return true;
        }

        let dataset = data.core_view();
        if self
            .target_class_ids
            .iter()
            .any(|&class| data.class_view().is_instance(id, class))
        {
            return true;
        }
        if !self.subject_predicates.is_empty()
            && quads_for_pattern_ids(dataset, Some(id), None, None, GraphFilter::AnyGraph)
                .any(|quad| self.subject_predicates.contains(&quad.p))
        {
            return true;
        }
        !self.object_predicates.is_empty()
            && quads_for_pattern_ids(dataset, None, None, Some(id), GraphFilter::AnyGraph)
                .any(|quad| self.object_predicates.contains(&quad.p))
    }

    fn resolve_all(&self, data: &ShaclData) -> Vec<FocusNode> {
        let dataset = data.core_view();
        let mut seen_ids = self.explicit_ids.clone();
        let mut nodes: Vec<FocusNode> = self
            .explicit_ids
            .iter()
            .copied()
            .map(FocusNode::Interned)
            .collect();

        for &class in &self.target_class_ids {
            for subject in data.class_view().instances_of(class) {
                if seen_ids.insert(subject) {
                    nodes.push(FocusNode::Interned(subject));
                }
            }
        }
        for &predicate in &self.subject_predicates {
            for quad in
                quads_for_pattern_ids(dataset, None, Some(predicate), None, GraphFilter::AnyGraph)
            {
                if seen_ids.insert(quad.s) {
                    nodes.push(FocusNode::Interned(quad.s));
                }
            }
        }
        // The pattern below is IDENTICAL to the subjects-of loop above it, and
        // that is correct by construction, not a copy-paste defect: the pattern
        // is `(?, predicate, ?)` — open on BOTH ends — so the one quad set
        // carries every subjects-of AND every objects-of target of `predicate`.
        // The loops differ in the only place they can: which END of the matched
        // quad is the focus node, `quad.s` there and `quad.o` here. Binding the
        // object here instead would be the real bug, because it would ask for
        // quads whose object is a predicate. `resolve_all_is_asymmetric_in_the_
        // subject_and_object_directions` pins the difference with a fixture whose
        // subjects-of and objects-of sets are disjoint.
        for &predicate in &self.object_predicates {
            for quad in
                quads_for_pattern_ids(dataset, None, Some(predicate), None, GraphFilter::AnyGraph)
            {
                if seen_ids.insert(quad.o) {
                    nodes.push(FocusNode::Interned(quad.o));
                }
            }
        }
        nodes.extend(
            self.explicit_foreign
                .iter()
                .cloned()
                .map(FocusNode::Foreign),
        );
        sort_focus_nodes(dataset, &mut nodes);
        nodes
    }
}

/// The DUAL of [`PreparedTargets`]: which shapes claim a node, rather than which
/// nodes a shape claims.
///
/// [`PreparedTargets::contains`] answers "shape, do you contain this node?". Used
/// as the change path's DISPATCH it is answered once per (shape, focus node)
/// pair, and each answer re-derives the focus node's class memberships and — for
/// a shape with a `sh:targetSubjectsOf` or `sh:targetObjectsOf` target — re-scans
/// the focus node's outgoing and incoming quads. Fifty node shapes against a
/// thousand-node delta is fifty thousand probes and up to a hundred thousand
/// quad-pattern scans before a single constraint is evaluated, and the whole
/// reason the change path exists is to be cheaper than that.
///
/// The change path asks the dual question — "node, which shapes claim you?" —
/// and this is the index that answers it. Every target key is inverted ONCE, at
/// bind time, so a focus node needs one class-view lookup and one outgoing and
/// one incoming predicate pass TOTAL, shared across every shape, yielding the
/// candidate shape set directly. `O(S·F)` becomes `O(F·(1+deg))`.
///
/// `contains` keeps DEFINING the semantics; this only has to agree with it, and
/// `target_dispatch_agrees_with_contains_and_resolve_all` is where that is
/// executed, over every [`Target`] variant.
///
/// Immutable behind `&` once built, and every field is `Send + Sync`, because the
/// claims derived from it are read from rayon focus-chunk workers.
#[derive(Debug, Default)]
pub(crate) struct TargetDispatch {
    /// `sh:targetNode` identity → the shapes declaring it.
    explicit: FastMap<TermId, Vec<usize>>,
    /// A `sh:targetNode` this dataset never interned → the shapes declaring it.
    foreign: FastMap<Term, Vec<usize>>,
    /// `sh:targetClass` / implicit-class identity → the shapes declaring it.
    classes: FastMap<TermId, Vec<usize>>,
    /// `sh:targetSubjectsOf` predicate identity → the shapes declaring it.
    subject_predicates: FastMap<TermId, Vec<usize>>,
    /// `sh:targetObjectsOf` predicate identity → the shapes declaring it.
    object_predicates: FastMap<TermId, Vec<usize>>,
}

impl TargetDispatch {
    /// Invert `targets`, which is positionally parallel to the node-shape list.
    fn invert(targets: &[PreparedTargets]) -> Self {
        let mut dispatch = Self::default();
        for (position, prepared) in targets.iter().enumerate() {
            for &id in &prepared.explicit_ids {
                dispatch.explicit.entry(id).or_default().push(position);
            }
            for term in &prepared.explicit_foreign {
                dispatch
                    .foreign
                    .entry(term.clone())
                    .or_default()
                    .push(position);
            }
            for &id in &prepared.target_class_ids {
                dispatch.classes.entry(id).or_default().push(position);
            }
            for &id in &prepared.subject_predicates {
                dispatch
                    .subject_predicates
                    .entry(id)
                    .or_default()
                    .push(position);
            }
            for &id in &prepared.object_predicates {
                dispatch
                    .object_predicates
                    .entry(id)
                    .or_default()
                    .push(position);
            }
        }
        dispatch
    }

    /// Every shape position claiming `focus`, ascending and duplicate-free, into
    /// `out` (which is cleared first, so one buffer serves a whole focus set).
    fn claimants(&self, data: &ShaclData, focus: &FocusNode, out: &mut Vec<usize>) {
        out.clear();
        let Some(id) = focus.id() else {
            // A focus node this dataset never interned can only be an EXPLICIT
            // target: every other target form is a fact ABOUT the data graph, and
            // a node absent from it participates in none of them. That is exactly
            // the answer `contains` gives an id-less focus node.
            if let Some(positions) = focus.foreign().and_then(|term| self.foreign.get(term)) {
                out.extend(positions.iter().copied());
            }
            return;
        };
        if let Some(positions) = self.explicit.get(&id) {
            out.extend(positions.iter().copied());
        }
        if !self.classes.is_empty() {
            for class in data.class_view().classes_of(id) {
                if let Some(positions) = self.classes.get(&class) {
                    out.extend(positions.iter().copied());
                }
            }
        }
        let dataset = data.core_view();
        if !self.subject_predicates.is_empty() {
            for quad in quads_for_pattern_ids(dataset, Some(id), None, None, GraphFilter::AnyGraph)
            {
                if let Some(positions) = self.subject_predicates.get(&quad.p) {
                    out.extend(positions.iter().copied());
                }
            }
        }
        if !self.object_predicates.is_empty() {
            for quad in quads_for_pattern_ids(dataset, None, None, Some(id), GraphFilter::AnyGraph)
            {
                if let Some(positions) = self.object_predicates.get(&quad.p) {
                    out.extend(positions.iter().copied());
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    /// Dispatch a whole focus set: for each of `shape_count` shape positions, the
    /// focus nodes that position claims.
    ///
    /// One pass over the focus nodes for ALL shapes, which is the entire point.
    fn claims(
        &self,
        data: &ShaclData,
        focus_nodes: &[FocusNode],
        shape_count: usize,
    ) -> Vec<ClaimedFocus> {
        // Both of these are INPUT-sized and both are sized here, because an
        // unhinted collection that fills to N reallocates about log2(N) times and
        // that is a real growth term in the focus count — small enough to have
        // been invisible under the per-node materialization this dispatch sits
        // behind, and the whole remaining difference once that is gone. The claim
        // rows are one per shape and a shape's claimed set is bounded by the focus
        // set; the claimant buffer is bounded by the shape count.
        let mut claims: Vec<ClaimedFocus> = Vec::with_capacity(shape_count);
        claims.resize_with(shape_count, ClaimedFocus::default);
        let mut positions: Vec<usize> = Vec::with_capacity(shape_count);
        for focus in focus_nodes {
            self.claimants(data, focus, &mut positions);
            for &position in &positions {
                // A prepared-target row exists for every node shape, so a position
                // out of range is impossible; ignoring one rather than indexing is
                // what keeps that impossible case from being a panic across the
                // PyO3 and C ABI boundaries.
                if let Some(claimed) = claims.get_mut(position) {
                    claimed.insert(focus, focus_nodes.len());
                }
            }
        }
        claims
    }
}

/// The focus nodes one shape position claims, as a membership test.
///
/// Identity-keyed for the ordinary interned focus node; a focus node with no
/// dataset identity is keyed by its term, exactly as the explicit-target index
/// that is the only way such a node can be claimed at all.
#[derive(Debug, Default)]
struct ClaimedFocus {
    /// Claimed interned focus nodes.
    ids: IdSet,
    /// Claimed focus nodes this dataset never interned.
    foreign: FastSet<Term>,
}

impl ClaimedFocus {
    /// Record a claim, sizing this row's table on its first entry.
    ///
    /// `focus_count` is the dispatched focus set's length, which is the exact
    /// upper bound on what one row can hold. Taking it here rather than at
    /// construction means only a row that really claims something allocates at
    /// all, while the row that does allocates ONCE instead of growing.
    fn insert(&mut self, focus: &FocusNode, focus_count: usize) {
        match focus {
            FocusNode::Interned(id) => {
                if self.ids.capacity() == 0 {
                    self.ids.reserve(focus_count);
                }
                self.ids.insert(*id);
            }
            FocusNode::Foreign(term) => {
                if self.foreign.capacity() == 0 {
                    self.foreign.reserve(focus_count);
                }
                self.foreign.insert(term.clone());
            }
        }
    }

    /// Whether this position claims `focus`.
    #[inline]
    fn contains(&self, focus: &FocusNode) -> bool {
        match focus {
            FocusNode::Interned(id) => self.ids.contains(id),
            FocusNode::Foreign(term) => self.foreign.contains(term),
        }
    }

    /// Whether this position claims nothing in the dispatched focus set.
    fn is_empty(&self) -> bool {
        self.ids.is_empty() && self.foreign.is_empty()
    }
}

/// Collect subjects that are SHACL instances of `class_iri`: nodes with an
/// `rdf:type` to `class_iri` or to any asserted (transitive) subclass of it.
///
/// `binding` carries the pre-resolved class identity; the shared class view answers
/// membership without a graph-wide closure walk during focus resolution.
fn instances_of_class(
    data: &ShaclData,
    class_iri: &NamedNode,
    binding: &DatasetBinding,
    classes: &ClassCatalog,
) -> Result<Vec<TermId>, String> {
    // A class the data graph never names has no instances — an ordinary empty
    // target, not a failure.
    let Some(class) = binding.class_id(classes, class_iri)? else {
        return Ok(Vec::new());
    };
    Ok(data.class_view().instances_of(class).collect())
}

/// A resolved focus node: an interned identity, or a term the dataset does not
/// hold.
///
/// **Two states, and exactly the two that occur.** The shape this replaced was a
/// struct pairing an owned [`Term`] with an `Option<TermId>`, which spells four
/// states for a value that only ever takes two: a term WITH an id duplicates
/// what the id already denotes, and a value with neither is not a focus node at
/// all. Every construction site builds one of the two arms below.
///
/// The interned arm carries NO term, and that is the point. Resolving one costs
/// between one and three heap `String`s — an IRI, a blank label, or a literal's
/// lexical form plus its datatype plus its tag — and on a conforming graph every
/// one of them is discarded unread, because the only consumers are the canonical
/// sort (which reads the interner instead, see
/// [`canonical_cmp_ids`](crate::term::canonical_cmp_ids)) and the focus-node slot
/// of a [`ValidationResult`](crate::report::ValidationResult) that a conforming
/// node never produces. [`Self::to_term`] is where the cost is paid, at the one
/// boundary that needs it.
///
/// `Send + Sync` and free of a lifetime parameter: rayon focus-chunk workers
/// share the focus set, and this is deferred MATERIALIZATION rather than a borrow
/// of the dataset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum FocusNode {
    /// A focus node the bound dataset interns, held as its identity alone.
    Interned(TermId),
    /// A focus node the bound dataset does not intern — an explicit
    /// `sh:targetNode`, or a SHACL-SPARQL target result, naming a term absent
    /// from the data graph. It has no identity to carry, so its term is.
    Foreign(Term),
}

// `FocusNode` is stored one per focus node in a change-path request, so the
// discriminant must not cost a word beside the term it is discriminating, and
// `Option<FocusNode>` must stay niche-packed into that discriminant. This is the
// same pin `SolutionTerm` carries in `purrdf_sparql_eval::scratch` and `TermId`
// carries in `purrdf_core::ir::term`; it fails the build if either regresses.
const _: () = assert!(size_of::<FocusNode>() == size_of::<Term>());
const _: () = assert!(size_of::<Option<FocusNode>>() == size_of::<FocusNode>());

impl FocusNode {
    /// Resolve `term` against `dataset` into the arm that describes it.
    pub(crate) fn resolve(dataset: &impl ShaclRead, term: &Term) -> Self {
        resolve_id(dataset, term).map_or_else(|| Self::Foreign(term.clone()), Self::Interned)
    }

    /// The interned identity of this focus node, if it has one.
    #[inline]
    pub(crate) fn id(&self) -> Option<TermId> {
        match self {
            Self::Interned(id) => Some(*id),
            Self::Foreign(_) => None,
        }
    }

    /// The owned term this focus node denotes — **the materialization boundary**.
    ///
    /// Every call site is one that is building a
    /// [`ValidationResult`](crate::report::ValidationResult), handing the focus
    /// node to a SPARQL surface that speaks owned terms, or recursing into a
    /// shape at a value node. None of them runs for a conforming focus node on
    /// the Core constraint path.
    pub(crate) fn to_term(&self, dataset: &impl ShaclRead) -> Term {
        match self {
            Self::Interned(id) => term_id_to_native(dataset, *id),
            Self::Foreign(term) => term.clone(),
        }
    }

    /// The foreign term this focus node carries, or `None` when it is interned.
    ///
    /// The explicit-target indexes are keyed by term for exactly the nodes this
    /// answers `Some` for, so a lookup never needs to materialize.
    #[inline]
    pub(crate) fn foreign(&self) -> Option<&Term> {
        match self {
            Self::Interned(_) => None,
            Self::Foreign(term) => Some(term),
        }
    }

    /// Whether this focus node can occupy a subject position (IRI or blank node),
    /// answered from the interner rather than from a materialized term.
    pub(crate) fn is_subject(&self, dataset: &impl ShaclRead) -> bool {
        match self {
            Self::Interned(id) => matches!(
                dataset.resolve(*id),
                ::purrdf::TermRef::Iri(_) | ::purrdf::TermRef::Blank { .. }
            ),
            Self::Foreign(term) => term.is_subject(),
        }
    }
}

/// A focus set together with the identity of the dataset its ids were resolved
/// against.
///
/// [`TermId`]s are DATASET-LOCAL (C0.8): the same integer addresses a different
/// term in every dataset, and an in-range id from the wrong one does not fail —
/// it silently validates a different node and reports a conforming verdict about
/// a node nobody asked about. `PreparedShapes::bind_delta_with_shapes_graph`
/// exists precisely so a caller can hold a base binding and a delta binding at
/// once, so two live id spaces is the DESIGNED situation rather than a mistake
/// nobody would make; and now that a focus node is carried id-natively, a wrong
/// id travels through target dispatch, claim indexing and constraint evaluation
/// before anything renders it, if anything ever does.
///
/// So the set records which dataset it was built from, and
/// [`Self::nodes_of`] refuses to hand it to a different one. The check is one
/// integer comparison per validation, not per focus node.
///
/// # Where this sits relative to [`FocusId`]
///
/// This is the INNER half of the same guard. `FocusId` carries provenance across
/// the public boundary, so an id from another binding is refused before it ever
/// reaches a focus set; this pins the other leg — that a set assembled against
/// one binding's view is never INTERPRETED against another's inside this crate,
/// including on the term-keyed route, which mints no ids a caller can hold.
pub(crate) struct FocusSet {
    /// The dataset this set's ids are addressed against.
    dataset: DatasetIdentity,
    nodes: Vec<FocusNode>,
}

impl FocusSet {
    /// An empty set over `data`, with room for `capacity` nodes.
    fn with_capacity(data: &ShaclData, capacity: usize) -> Self {
        Self {
            dataset: data.identity(),
            nodes: Vec::with_capacity(capacity),
        }
    }

    /// Admit one focus node. Callers admit only nodes resolved against the
    /// dataset this set was opened over.
    #[inline]
    fn push(&mut self, node: FocusNode) {
        self.nodes.push(node);
    }

    /// Order this set canonically.
    fn sort(&mut self, data: &ShaclData) {
        debug_assert_eq!(
            self.dataset,
            data.identity(),
            "a focus set is ordered against the dataset it was resolved from"
        );
        sort_focus_nodes(data.core_view(), &mut self.nodes);
    }

    /// The focus nodes, if `data` is the dataset they were resolved against.
    ///
    /// # Errors
    /// Refuses a set whose ids address a different dataset.
    fn nodes_of(&self, data: &ShaclData) -> Result<&[FocusNode], String> {
        if self.dataset == data.identity() {
            return Ok(&self.nodes);
        }
        Err(
            "focus node TermIds were resolved against a different dataset than the one this \
             validator is bound to; TermIds are dataset-local and an in-range id from another \
             dataset addresses a different term"
                .to_owned(),
        )
    }
}

/// Resolve the focus node set for a single shape from its target declarations.
///
/// Interned targets are deduplicated in id space and resolved to an owned term
/// exactly once. Results retain that id for constraint and path evaluation and
/// are sorted canonically before return.
pub(crate) fn resolve_focus_nodes(
    data: &ShaclData,
    targets: &[Target],
    binding: &DatasetBinding,
    classes: &ClassCatalog,
) -> Result<Vec<FocusNode>, String> {
    let ds = data.core_view();
    let mut seen_ids: IdSet = IdSet::default();
    let mut seen_foreign: FastSet<Term> = FastSet::default();
    let mut nodes: Vec<FocusNode> = Vec::new();

    for target in targets {
        let ids = match target {
            Target::Class(class_iri) => {
                Some(instances_of_class(data, class_iri, binding, classes)?)
            }
            Target::SubjectsOf(pred) => Some(subjects_of(ds, pred)),
            Target::ObjectsOf(pred) => Some(objects_of(ds, pred)),
            Target::ImplicitClass(Term::NamedNode(class)) => {
                Some(instances_of_class(data, class, binding, classes)?)
            }
            Target::ImplicitClass(_) => Some(Vec::new()),
            Target::Node(_) | Target::Sparql { .. } => None,
        };
        if let Some(ids) = ids {
            // Upper bound on the pushes this target adds: one Vec growth step
            // instead of amortized doubling across the loop.
            nodes.reserve(ids.len());
            for id in ids {
                if seen_ids.insert(id) {
                    nodes.push(FocusNode::Interned(id));
                }
            }
            continue;
        }

        let candidates = match target {
            Target::Node(term) => vec![term.clone()],
            // SELECT-form is enforced at shape-load; residual evaluation failures
            // remain hard validation errors.
            Target::Sparql {
                select,
                substitutions,
            } => crate::sparql::eval_target_view(data.sparql_view(), select, substitutions)
                .map_err(|e| format!("sh:target SPARQLTarget failed: {e}"))?,
            Target::Class(_)
            | Target::SubjectsOf(_)
            | Target::ObjectsOf(_)
            | Target::ImplicitClass(_) => unreachable!("id-native target handled above"),
        };
        for term in candidates {
            if let Some(id) = resolve_id(ds, &term) {
                if seen_ids.insert(id) {
                    nodes.push(FocusNode::Interned(id));
                }
            } else if seen_foreign.insert(term.clone()) {
                nodes.push(FocusNode::Foreign(term));
            }
        }
    }

    sort_focus_nodes(ds, &mut nodes);
    Ok(nodes)
}

fn evaluate_shape_focus_nodes(
    data: &ShaclData,
    shapes: &Shapes,
    plan: ShapePlan<'_>,
    focus_nodes: &[FocusNode],
    include_focus: impl Fn(&FocusNode) -> bool + Sync,
) -> Result<Vec<crate::report::ValidationResult>, String> {
    // One validation owns one ordered governor ledger. Letting focus workers charge that
    // shared state directly would make the first trip and its consumed vector depend on
    // rayon scheduling rather than focus order. Governed validation is therefore serial;
    // the ordinary path keeps the established deterministic chunk parallelism.
    if let Some(governors) = crate::sparql::current_governors() {
        let _function_scope = crate::sparql::enter_function_scope(Arc::clone(&shapes.functions));
        let _aggregate_scope = crate::sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
        let _governor_scope = crate::sparql::enter_governor_scope(governors);
        let mut out = Vec::new();
        for focus in focus_nodes {
            if include_focus(focus) {
                out.extend(crate::constraints::validate_shape_with_plan_at(
                    data,
                    focus,
                    shapes.box_role_vocab.as_ref(),
                    plan,
                )?);
            }
        }
        return Ok(out);
    }

    // `functions` and `aggregates` are OWNED by the `Shapes` value (the caller installs
    // them once, on `Shapes::functions` / `Shapes::aggregates`, before validation ever
    // runs), so every focus-chunk worker gets its own scope built directly from `shapes`
    // — no thread-local read, no dependency on which entry point happened to install an
    // ambient scope before reaching here. `relations` (property functions) has no
    // `Shapes`-owned home: it is a genuinely call-scoped, caller-injected table, so it is
    // still snapshotted from the orchestrating thread's ambient scope. A thread-local is
    // invisible from the threads a chunk fork creates, so that snapshot is taken HERE and
    // re-installed per chunk — otherwise a `sh:select` body whose predicate is a property
    // function would resolve on the sequential path and fail to resolve on the parallel
    // one, which is a scheduling-dependent verdict.
    let relations = crate::sparql::current_property_functions();
    crate::parallel::try_map_chunks(
        focus_nodes,
        || {
            (
                crate::sparql::enter_function_scope(Arc::clone(&shapes.functions)),
                relations
                    .clone()
                    .map(crate::sparql::enter_property_function_scope),
                crate::sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates)),
            )
        },
        |_scopes, out, focus| {
            if include_focus(focus) {
                out.extend(crate::constraints::validate_shape_with_plan_at(
                    data,
                    focus,
                    shapes.box_role_vocab.as_ref(),
                    plan,
                )?);
            }
            Ok::<(), String>(())
        },
    )
}

fn finish_report(mut results: Vec<crate::report::ValidationResult>) -> ValidationReport {
    // Deterministic sort key: (focus_node, component, source_shape, path, value,
    // message, severity). The message and severity tiebreakers make the ordering
    // TOTAL: two results that agree on the first five components (e.g. several
    // `sh:uniqueLang` violations on one focus, which differ only in their message
    // text) would otherwise keep their push order, which is a `FastMap`/`FastSet`
    // iteration order and thus not guaranteed stable across ahash versions or
    // targets. Sorting on the full serialized identity closes that leak so report
    // bytes are invariant under data-insertion order and platform.
    let sort_key = |result: &crate::report::ValidationResult| {
        (
            result.focus_node.to_string(),
            result.source_constraint_component.to_string(),
            result.source_shape.to_string(),
            result
                .result_path
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            result
                .value
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            result.message.clone().unwrap_or_default(),
            result.severity.clone(),
        )
    };
    results.sort_by_cached_key(sort_key);
    ValidationReport {
        conforms: results.is_empty(),
        results,
    }
}

fn validate_with_plan_and_focus_filter<F>(
    data: &ShaclData,
    shapes: &Shapes,
    bound: &BoundShapes,
    mut include_focus: F,
) -> Result<ValidationReport, String>
where
    F: FnMut(&Shape, &Term) -> bool,
{
    // This orchestration-thread scope covers target resolution; each focus chunk
    // installs the same SHACL-AF function and custom-aggregate registries on its
    // worker.
    let _function_scope = crate::sparql::enter_function_scope(Arc::clone(&shapes.functions));
    let _aggregate_scope = crate::sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
    let mut all_results = Vec::new();

    for (position, shape) in shapes.node_shapes.iter().enumerate() {
        if shape.deactivated {
            continue;
        }
        let mut focus_nodes =
            resolve_focus_nodes(data, &shape.targets, bound.binding(), bound.classes())?;
        // `FnMut` is intentionally applied serially in canonical focus order, so
        // existing callers observe the same calls even when evaluation dispatches
        // the retained set to workers. The filter is a caller-supplied predicate
        // over an owned term, so this route — and only this route — materializes
        // one per focus node; the change path, which has no filter, does not.
        focus_nodes.retain(|focus| include_focus(shape, &focus.to_term(data.core_view())));
        all_results.extend(evaluate_shape_focus_nodes(
            data,
            shapes,
            bound.plan(shape, position)?,
            &focus_nodes,
            |_| true,
        )?);
    }
    Ok(finish_report(all_results))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Immutable shape preparation reusable across independent dataset snapshots.
///
/// The parsed shapes and cycle-aware class-reference analysis are shared. Each
/// binding resolves IDs, class membership and every active target against its own
/// exact dataset. No target set, validation answer or negative dependency proof
/// is reused across bindings. Keep this value for a batch of related validations;
/// its storage is bounded by the supplied shape tree and released with its owners.
///
/// Every one carries the [`ValidatorProvenance`] of the route that built it, so a
/// report can be attributed to the artifact behind it rather than to whichever file
/// the caller remembers opening — read it with [`Self::provenance`].
#[derive(Debug, Clone)]
pub struct PreparedShapes {
    shapes: Arc<Shapes>,
    classes: Arc<ClassCatalog>,
    /// The stage-0 lowering of these shapes: their structure with every
    /// shape-constant derivation already made, shared with every binding.
    ///
    /// Derived on first BIND rather than in a constructor, and that is the whole
    /// point of the cell. [`Self::with_carried_analysis`] is the prepared-product
    /// admit seam, and a restore is promised "no RDF reparsing, no shape
    /// extraction, no repeated shared analysis" — a lowering built in
    /// [`Self::with_provenance`] only would either be absent on a restored
    /// preparation (leaving every `sh:in`, `sh:hasValue` and `sh:class` on it
    /// unable to constrain, silently) or would have to be rebuilt at admit, which
    /// is the repeated analysis the promise rules out. Deriving it here, once, on
    /// whichever bind comes first, is the only placement that is correct for both
    /// constructors.
    ///
    /// Shared by `Arc` so a clone of a preparation shares the memoized lowering
    /// rather than deriving a second one.
    lowered: OnceLock<Arc<LoweredShapes>>,
    /// Where this preparation came from, recorded by the expression that built it.
    ///
    /// Shared rather than owned so that binding a preparation to a dataset — the
    /// hot, repeated operation this whole type exists for — costs a reference-count
    /// bump instead of deep-copying an [`Identity`](purrdf_core::artifact::Identity)
    /// and every labelled component inside it, once per bind.
    provenance: Arc<ValidatorProvenance>,
}

impl PreparedShapes {
    /// Analyze the complete parsed shape tree once, without inspecting any data.
    ///
    /// The resulting preparation reports [`ValidatorProvenance::Parsed`]: these
    /// shapes were analyzed from a value this process holds, not restored from an
    /// artifact. That is stated HERE, at the construction site, rather than left for
    /// a caller to assert later — see [`ValidatorProvenance`] for why an accessor
    /// with an "unknown" answer would be worse than none.
    #[must_use]
    pub fn new(shapes: Arc<Shapes>) -> Self {
        Self::with_provenance(shapes, ValidatorProvenance::Parsed)
    }

    /// Analyze a shape tree and record a provenance other than a local parse.
    ///
    /// `pub(crate)` on purpose, and the reason is the one [`ParseProvenance`] gives:
    /// a provenance a caller can assign is a CLAIM about a preparation rather than a
    /// fact about it. The only expressions that may state "this came from product
    /// X" are the codec's own admission seams, which have the product in hand and
    /// have already checked what they are about to claim.
    ///
    /// [`ParseProvenance`]: crate::provenance::ParseProvenance
    pub(crate) fn with_provenance(shapes: Arc<Shapes>, provenance: ValidatorProvenance) -> Self {
        // ONE walk: the class catalog is this lowering's own output, not a second
        // traversal beside it. The lowering is retained rather than discarded, so a
        // locally parsed preparation reaches its first bind with the memo already
        // filled and pays for the walk exactly as many times as a restored one.
        let lowered = Arc::new(crate::plan::lower_shapes(shapes.node_shapes.iter()));
        let classes = Arc::clone(lowered.classes());
        let prepared = Self::with_carried_analysis(shapes, provenance, classes);
        let _ = prepared.lowered.set(lowered);
        prepared
    }

    /// Assemble a preparation around an analysis that was NOT derived here.
    ///
    /// This is the constructor the prepared-product admit seam uses, and it is the
    /// whole point of the product carrying the class catalog: a consumer restoring a
    /// product is promised "no RDF reparsing, no shape extraction, **no repeated
    /// shared analysis**", and a restore that ended at
    /// [`with_provenance`](Self::with_provenance) would honour the first two and
    /// quietly break the third — the walk would run again on every restore, for
    /// every process, forever, while every test still passed.
    ///
    /// `classes` is therefore a fact the caller has, not a value this constructor
    /// may second-guess. `pub(crate)` for exactly the reason
    /// [`with_provenance`](Self::with_provenance) is: an analysis a caller can
    /// supply is a CLAIM about a preparation rather than a derivation from it, and
    /// the only expression in the crate allowed to make that claim is the admission
    /// seam, which has the product in hand and proves the claim against the
    /// identity digest the product pinned before any preparation escapes
    /// (`crate::product::certified`).
    pub(crate) fn with_carried_analysis(
        shapes: Arc<Shapes>,
        provenance: ValidatorProvenance,
        classes: Arc<ClassCatalog>,
    ) -> Self {
        Self {
            shapes,
            classes,
            lowered: OnceLock::new(),
            provenance: Arc::new(provenance),
        }
    }

    /// The stage-0 lowering of these shapes, deriving it on first use.
    ///
    /// Idempotent and shared: the walk runs at most once per preparation however
    /// many datasets it is bound to, and a preparation restored from a prepared
    /// product reaches it by exactly the same route a locally parsed one does.
    fn lowered(&self) -> Arc<LoweredShapes> {
        Arc::clone(
            self.lowered.get_or_init(|| {
                Arc::new(crate::plan::lower_shapes(self.shapes.node_shapes.iter()))
            }),
        )
    }

    /// Where this preparation came from: parsed in this process, or restored from a
    /// named prepared product.
    ///
    /// TOTAL — every preparation has an answer, and none of them is "unknown". A
    /// report is only attributable to the artifact that produced it if the
    /// preparation can be asked, so this is the accessor a consumer pairs with a
    /// verdict when the shapes arrived as bytes.
    #[must_use]
    pub fn provenance(&self) -> &ValidatorProvenance {
        &self.provenance
    }

    /// The cycle-safe class analysis this preparation holds: derived from its shape
    /// tree by [`Self::new`], or carried in from a prepared product by
    /// [`Self::with_carried_analysis`].
    ///
    /// `pub(crate)` and shared rather than public: the catalog is a PURE derivation
    /// of the shapes, so handing it out publicly would offer callers a second,
    /// forgeable spelling of something they can always re-derive. The in-crate
    /// consumers are `crate::product::ast`, which writes it into the artifact so a
    /// restore does not have to walk the shape tree again, and
    /// `crate::product::identity`, which digests it so what a product carried can be
    /// checked against the digest the same product pinned.
    pub(crate) fn class_catalog(&self) -> Arc<ClassCatalog> {
        Arc::clone(&self.classes)
    }

    /// The parsed shapes this preparation analyzed.
    ///
    /// Public because of where a preparation can now come FROM. A caller that
    /// built one with [`Self::new`] already owns the `Arc<Shapes>` it passed in,
    /// so for that caller this is a second spelling of a value they hold — which
    /// is why it used to be `pub(crate)`. A caller that obtained one from
    /// [`ShapesProductView::admit`] holds no such value: the shapes graph was
    /// restored from bytes inside the codec and this accessor is its ONLY
    /// spelling. Without it an admitted product could be restored and then not
    /// handed to [`validate_dataset_with_shapes_graph`] or
    /// [`validate_dataset_with_governors`], which is the entire reason the product
    /// exists.
    ///
    /// The borrow is immutable and `Shapes` is immutable, so this hands out no
    /// authority to change a preparation after the fact.
    /// `class_catalog` stays `pub(crate)`: it is a PURE derivation of these
    /// shapes, so it really is re-derivable by anyone holding this.
    ///
    /// [`ShapesProductView::admit`]: crate::product::ShapesProductView::admit
    #[must_use]
    pub fn shapes(&self) -> &Arc<Shapes> {
        &self.shapes
    }

    /// Bind shared shape analysis to a new data holder. All dataset-dependent
    /// preparation is performed again, including SHACL-SPARQL target evaluation.
    ///
    /// # Errors
    /// Returns an error when an active target cannot be evaluated.
    pub fn bind(&self, data: ShaclData) -> Result<PreparedValidator, String> {
        PreparedValidator::bind(data, self)
    }

    /// Project a dataset and bind the shared shape analysis to that snapshot.
    ///
    /// # Errors
    /// Returns an error when projection or target evaluation fails.
    pub fn bind_dataset(&self, data: &RdfDataset) -> Result<PreparedValidator, String> {
        self.bind_projected_dataset(project_dataset(data)?)
    }

    /// Bind a shared native source through a borrowed SHACL projection.
    /// Data dictionaries and indexes are retained; graph union and RDF 1.2
    /// statement projection are read views. No owned projection is built.
    ///
    /// # Errors
    /// Returns an error when a target cannot be evaluated.
    pub fn bind_shared_dataset(&self, data: Arc<RdfDataset>) -> Result<PreparedValidator, String> {
        let view = Arc::new(ShaclDatasetView::project(data));
        self.bind_view(view)
    }

    /// Bind a complete immutable SHACL carrier, retaining its exact identity.
    ///
    /// The view's own read semantics are honoured as given, including a view
    /// built without graph union or without statement projection. That makes this
    /// the general door and [`Self::bind_delta_with_shapes_graph`] the specific
    /// one: the incremental change path needs the projected surface its soundness
    /// argument is written over, so a delta view bound here without statement
    /// projection validates normally but is refused by
    /// [`PreparedValidator::affected_focus_node_ids`].
    ///
    /// # Errors
    /// Returns an error when a target cannot be evaluated.
    pub fn bind_view(&self, view: Arc<ShaclDatasetView>) -> Result<PreparedValidator, String> {
        self.bind(ShaclData::from_views(Arc::clone(&view), view, None))
    }

    /// Bind a shared native source and expose its shapes graph without copying
    /// the data dictionary. Existing explicit blank scopes are preserved.
    ///
    /// # Errors
    /// Refuses retention limits, invalid graph placement or failed targets.
    pub fn bind_shared_dataset_with_shapes_graph(
        &self,
        data: Arc<RdfDataset>,
        shapes_graph_iri: Option<&str>,
        limits: ::purrdf::ir::ViewLimits,
    ) -> Result<PreparedValidator, String> {
        let core = Arc::new(ShaclDatasetView::project(Arc::clone(&data)));
        let (sparql, graph) = build_sparql_view(
            ::purrdf::ir::CompositeSource::new(data),
            Arc::clone(&core),
            &self.shapes,
            shapes_graph_iri,
            limits,
        )?;
        self.bind(ShaclData::from_views(core, sparql, graph))
    }

    /// Bind an immutable mutation snapshot and its shapes graph through shared
    /// indexes, freezing neither the base nor a combined data-plus-shapes graph.
    ///
    /// The binding this returns can expand the same snapshot into the focus nodes
    /// the change moved, through
    /// [`PreparedValidator::affected_focus_node_ids`] — which is the only sound
    /// input to [`PreparedValidator::validate_focus_node_ids`], because the
    /// subjects of the changed rows are not it.
    ///
    /// # Errors
    /// Refuses retention limits, invalid graph placement or failed targets.
    pub fn bind_delta_with_shapes_graph(
        &self,
        data: Arc<::purrdf::ir::DeltaDatasetView>,
        shapes_graph_iri: Option<&str>,
        limits: ::purrdf::ir::ViewLimits,
    ) -> Result<PreparedValidator, String> {
        let core = Arc::new(ShaclDatasetView::delta(Arc::clone(&data), true, limits)?);
        let (sparql, graph) = build_sparql_view(
            ::purrdf::ir::CompositeSource::from_delta(data),
            Arc::clone(&core),
            &self.shapes,
            shapes_graph_iri,
            limits,
        )?;
        self.bind(ShaclData::from_views(core, sparql, graph))
    }

    /// Bind an already-projected snapshot; Core and SPARQL share its `Arc`.
    ///
    /// # Errors
    /// Returns an error when an active target cannot be evaluated.
    pub fn bind_projected_dataset(
        &self,
        projected: Arc<RdfDataset>,
    ) -> Result<PreparedValidator, String> {
        self.bind(ShaclData::new(Arc::clone(&projected), projected, None))
    }

    /// Bind a projected snapshot with the shapes graph exposed to SPARQL under
    /// `shapes_graph_iri`, or the graph IRI declared by the parsed shapes.
    ///
    /// # Errors
    /// Returns an error when shapes-graph assembly or target evaluation fails.
    pub fn bind_projected_dataset_with_shapes_graph(
        &self,
        projected: Arc<RdfDataset>,
        shapes_graph_iri: Option<&str>,
    ) -> Result<PreparedValidator, String> {
        self.bind(build_projected_data(
            projected,
            &self.shapes,
            shapes_graph_iri,
        )?)
    }
}

/// Reusable validation state for one immutable projected dataset and shapes graph.
///
/// Preparation builds the shared class-membership view, resolves target
/// predicates, and evaluates explicit SHACL-SPARQL targets once. Core targets are
/// kept as compact membership predicates: [`Self::validate_focus_nodes`] and
/// [`Self::validate_focus_node_ids`] inspect only the caller-supplied candidates
/// and never enumerate every target node in the dataset. This is the preferred
/// surface for realtime validation over a large immutable snapshot.
///
/// A prepared validator is tied to the exact dataset snapshot it owns. Prepare a
/// new value after publishing an overlay or replacement snapshot. [`PreparedShapes`]
/// shares shape analysis across these bindings while rebuilding data-dependent state.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use purrdf::RdfDatasetBuilder;
/// use purrdf_shapes::engine::{PreparedValidator, parse_shapes};
/// use purrdf_shapes::term::NamedNode;
///
/// # fn main() -> Result<(), String> {
/// let mut builder = RdfDatasetBuilder::new();
/// let rdf_type = builder.intern_iri(
///     "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
/// );
/// let person = builder.intern_iri("https://example.org/Person");
/// let required = builder.intern_iri("https://example.org/required");
/// let alice = builder.intern_iri("https://example.org/alice");
/// let present = builder.intern_iri("https://example.org/present");
/// builder.push_quad(alice, rdf_type, person, None);
/// builder.push_quad(alice, required, present, None);
/// let dataset = Arc::new(builder.freeze().map_err(|error| error.to_string())?);
///
/// let shapes = Arc::new(parse_shapes(
///     r#"
///     @prefix sh: <http://www.w3.org/ns/shacl#> .
///     @prefix ex: <https://example.org/> .
///     ex:PersonShape a sh:NodeShape ;
///         sh:targetClass ex:Person ;
///         sh:property [ sh:path ex:required ; sh:minCount 1 ] .
///     "#,
///     // No base: every IRI in this shapes graph is already absolute.
///     None,
/// )?);
/// let validator = PreparedValidator::from_projected_dataset(
///     Arc::clone(&dataset),
///     shapes,
/// )?;
///
/// // Reuse `validator` for each affected focus set in this immutable snapshot.
/// // A focus id is minted BY the binding, so it cannot be confused with an id
/// // of the same number belonging to a different one.
/// let alice = validator
///     .term_id(&NamedNode::new_unchecked("https://example.org/alice").into_term())
///     .ok_or("alice must be interned")?;
/// let report = validator.validate_focus_node_ids(&[alice])?;
/// assert!(report.conforms);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PreparedValidator {
    data: ShaclData,
    shapes: Arc<Shapes>,
    /// The provenance of the [`PreparedShapes`] this binding came from, shared with
    /// it and with every sibling binding — a bind copies a reference count, never an
    /// identity.
    provenance: Arc<ValidatorProvenance>,
    /// Stage 0 × this dataset: the shape lowering, the class analysis, every
    /// identity they named and every active target, resolved once here so no focus
    /// node resolves any of them again.
    bound: BoundShapes,
}

impl PreparedValidator {
    /// Prepare an already-assembled [`ShaclData`] holder and parsed shapes.
    ///
    /// # Errors
    ///
    /// Returns an error when an active SHACL-SPARQL target cannot be evaluated.
    pub fn new(data: ShaclData, shapes: Arc<Shapes>) -> Result<Self, String> {
        PreparedShapes::new(shapes).bind(data)
    }

    fn bind(data: ShaclData, prepared: &PreparedShapes) -> Result<Self, String> {
        let shapes = Arc::clone(&prepared.shapes);
        let _function_scope = crate::sparql::enter_function_scope(Arc::clone(&shapes.functions));
        let _aggregate_scope = crate::sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
        data.prepare_class_membership();
        let bound = BoundShapes::bind(
            &data,
            &shapes.node_shapes,
            prepared.lowered(),
            Arc::clone(&prepared.classes),
        )?;
        Ok(Self {
            data,
            shapes,
            provenance: Arc::clone(&prepared.provenance),
            bound,
        })
    }

    /// Where the shapes this validator executes came from: parsed in this process,
    /// or restored from a named prepared product.
    ///
    /// The same answer [`PreparedShapes::provenance`] gives — literally the same
    /// shared value — so a caller holding only a binding can attribute its reports
    /// without keeping the preparation beside it.
    #[must_use]
    pub fn provenance(&self) -> &ValidatorProvenance {
        &self.provenance
    }

    /// Operational measurements for the retained Core and SPARQL carriers.
    /// Repeated validation reuses these exact views and their prepared analyses.
    #[must_use]
    pub fn view_stats(&self) -> [crate::data_view::ShaclViewStats; 2] {
        self.data.view_stats()
    }

    /// Return compact class-membership index dimensions.
    ///
    /// The array contains typed-class entries, stored subject ids, ancestor ids,
    /// superclass entries, source-class ids, and the virtual-row upper bound.
    #[doc(hidden)]
    #[must_use]
    pub fn __class_membership_dimensions(&self) -> [usize; 6] {
        self.data.class_view().dimensions()
    }

    /// Project and prepare a frozen dataset.
    ///
    /// # Errors
    ///
    /// Returns an error when projection fails or an active SHACL-SPARQL target
    /// cannot be evaluated.
    pub fn from_dataset(data: &RdfDataset, shapes: Arc<Shapes>) -> Result<Self, String> {
        Self::from_projected_dataset(project_dataset(data)?, shapes)
    }

    /// Prepare an already-SHACL-projected frozen dataset.
    ///
    /// Core and SHACL-SPARQL lookups share the same immutable dataset.
    ///
    /// # Errors
    ///
    /// Returns an error when an active SHACL-SPARQL target cannot be evaluated.
    pub fn from_projected_dataset(
        projected: Arc<RdfDataset>,
        shapes: Arc<Shapes>,
    ) -> Result<Self, String> {
        let data = ShaclData::new(Arc::clone(&projected), projected, None);
        Self::new(data, shapes)
    }

    /// Prepare an already-projected dataset with the shapes graph exposed to
    /// SHACL-SPARQL paths as a named graph.
    ///
    /// `shapes_graph_iri` overrides [`Shapes::shapes_graph`] when supplied.
    ///
    /// # Errors
    ///
    /// Returns an error when the combined dataset cannot be frozen or an active
    /// SHACL-SPARQL target cannot be evaluated.
    pub fn from_projected_dataset_with_shapes_graph(
        projected: Arc<RdfDataset>,
        shapes: Arc<Shapes>,
        shapes_graph_iri: Option<&str>,
    ) -> Result<Self, String> {
        let data = build_projected_data(projected, &shapes, shapes_graph_iri)?;
        Self::new(data, shapes)
    }

    /// Validate every target node using the prepared target plan.
    ///
    /// Core target sets are enumerated for this whole-bundle operation, but class
    /// closures, predicate identities, and SHACL-SPARQL target results are reused.
    ///
    /// # Errors
    ///
    /// Returns an error when a constraint evaluation hard-fails.
    pub fn validate(&self) -> Result<ValidationReport, String> {
        let mut all_results = Vec::new();
        for (position, shape) in self.shapes.node_shapes.iter().enumerate() {
            if shape.deactivated {
                continue;
            }
            let plan = self.bound.plan(shape, position)?;
            let focus_nodes = plan.targets().resolve_all(&self.data);
            all_results.extend(evaluate_shape_focus_nodes(
                &self.data,
                &self.shapes,
                plan,
                &focus_nodes,
                |_| true,
            )?);
        }
        Ok(finish_report(all_results))
    }

    /// Validate only the supplied candidate focus nodes that match each shape's
    /// prepared targets.
    ///
    /// Candidates are deduplicated and canonically ordered. Target membership is
    /// answered through direct IR index probes; whole target sets are not built.
    ///
    /// Expanding a graph change into the focus nodes it can move is
    /// [`Self::affected_focus_node_ids`]'s job, not a caller's: it derives the
    /// dependency footprint — inverse paths, sequence prefixes, closures,
    /// `sh:targetObjectsOf`, `sh:node` recursion, property-pair comparands and the
    /// class hierarchy — from the same walk that lowers the shapes graph, and
    /// reports a footprint it cannot bound rather than returning a short answer.
    /// Two obligations stay with the caller, and both are visible rather than
    /// implied: feeding it the mutation snapshot this binding was built over, and
    /// honouring a [`FocusExpansion::Everything`] answer by calling
    /// [`Self::validate`] instead.
    ///
    /// # Bounded allocation
    ///
    /// **Validating a CONFORMING focus set through this method allocates a bounded
    /// amount, independent of how many focus nodes were supplied — for every
    /// constraint kind and path form whose evaluation stays inside this crate.**
    /// Cost is proportional to the violations found, not to the focus nodes
    /// examined: a focus node is carried as its interned identity and materialized
    /// as an owned term only where a result is actually built. A shape that reads
    /// through SPARQL query text is the one qualification, and it is stated in full
    /// below rather than left to a reader to discover.
    ///
    /// This is a product claim and it is executed, not asserted in prose:
    /// `crates/shapes/tests/change_path_alloc.rs` measures the allocation delta of
    /// two conforming validations differing only in focus count, through both
    /// change-path entry points, over every constraint kind and path form it
    /// covers, and requires the two figures to be EQUAL. A companion test holds
    /// the violation count fixed while the conforming population doubles and the
    /// conforming population fixed while the violations double, so "bounded" cannot
    /// be satisfied by a validator that stopped validating.
    ///
    /// Two THIRD-PARTY residuals are documented there, neither of them a
    /// per-focus-node term: `rayon`'s global injector queue allocates one block
    /// every 63 submissions, and the `regex` crate's thread-sharded cache pool
    /// allocates when a worker finds its shard empty, which is reachable under
    /// `sh:pattern` above the parallel threshold.
    ///
    /// ## The SPARQL-bearing surfaces DO carry a per-focus-node term
    ///
    /// That term is this crate's own traffic, not a third party's, so it is stated
    /// here and not only in a test. A shape backed by query text — a `sh:sparql`
    /// constraint, a custom component's `sh:ask`/`sh:select` validator, a SHACL-AF
    /// `sh:expression` function call — runs one SPARQL query PER FOCUS NODE (per
    /// value node for an `ASK` validator, per argument tuple for an expression
    /// call), and a query evaluation is not allocation-free. Those four surfaces
    /// satisfy a closed form rather than zero growth:
    ///
    /// ```text
    /// allocations(N) == CHANGE_PATH_CONSTANT + per_focus_node * N
    /// ```
    ///
    /// `CHANGE_PATH_CONSTANT` is the entry cost the zero-growth cases already pin.
    /// `per_focus_node` is measured and asserted EXACTLY, at `N` and at `2N`, by
    /// `crates/shapes/tests/sparql_path_alloc.rs`: **78** for a `sh:sparql` SELECT
    /// constraint, **164** for a custom `sh:ask` component over a two-valued path,
    /// **92** for a custom `sh:select` component, and **180** for a
    /// `sh:expression` function call over two argument tuples.
    ///
    /// The term is FLAT in the data graph — a fixed focus count costs the same over
    /// 768 quads and over 24,576 — so it is the price of executing a query, not of
    /// scanning a graph. By measurement it divides into the SPARQL evaluator's
    /// per-execution setup (the larger share on three of the four surfaces) and the
    /// per-focus-node pre-binding rewrite, in which `purrdf_sparql_eval` clones the
    /// prepared algebra and rebuilds it and every pre-bound term crosses as an owned
    /// `TermValue` whose IRI is a fresh `String`. Neither share is zero today.
    ///
    /// The bind in front of this method is bounded too — independent of the data
    /// graph's size beyond the class catalog — so an incremental caller does not
    /// pay for the whole graph to ask about a handful of nodes.
    /// [`PreparedShapes::bind_dataset`] is the deliberate exception: it projects
    /// the data graph into an owned snapshot first, so it is linear in the graph by
    /// construction.
    ///
    /// # Errors
    ///
    /// Returns an error when a constraint evaluation hard-fails.
    pub fn validate_focus_nodes(&self, focus_nodes: &[Term]) -> Result<ValidationReport, String> {
        let focus_nodes = self.normalize_focus_nodes(focus_nodes);
        self.validate_bounded(&focus_nodes)
    }

    /// Id-native twin of [`Self::validate_focus_nodes`] for callers already using
    /// the prepared dataset's interned identities.
    ///
    /// # Provenance is checked, not assumed
    ///
    /// Every [`FocusId`] names the binding it was minted against, and one minted
    /// against a different binding is REFUSED here. That is the whole reason the
    /// argument is a `FocusId` and not a [`TermId`]: a `TermId` is an index, an
    /// index from another binding is very probably in range, and it would resolve
    /// to a different term and validate the wrong node without any lookup ever
    /// failing. Mint ids from this binding — [`Self::term_id`] for a caller
    /// holding a [`Term`], [`Self::affected_focus_node_ids`] for the change path —
    /// and the `delta` → expand → validate loop is provenance-safe end to end by
    /// type.
    ///
    /// # Compatibility
    ///
    /// This method shipped in 2.0.2 taking `&[TermId]`, and the change to
    /// `&[FocusId]` is a deliberate, un-versioned break of that signature. There
    /// is no `&[TermId]` shim beside it on purpose: the old door is the unguarded
    /// one, and keeping it open would leave the hazard reachable while claiming it
    /// was closed. Callers holding `TermId`s re-mint them through
    /// [`Self::term_id`], which is a lookup they were already entitled to do.
    ///
    /// # Bounded allocation
    ///
    /// **Validating a CONFORMING focus set through this method allocates a bounded
    /// amount, independent of how many ids were supplied, for every constraint kind
    /// and path form whose evaluation stays inside this crate** — the same
    /// guarantee [`Self::validate_focus_nodes`] carries, with the same
    /// qualification, measured through both entry points by the same tests in
    /// `crates/shapes/tests/change_path_alloc.rs`. Cost is proportional to the
    /// violations found, not to the focus nodes examined.
    ///
    /// A shape backed by SPARQL query text is the qualification and it is a
    /// FIRST-PARTY one: `sh:sparql`, a custom component's `sh:ask`/`sh:select`
    /// validator and a SHACL-AF `sh:expression` call each run one query per focus
    /// node and so carry a real per-focus-node term, pinned in closed form at
    /// 78 / 164 / 92 / 180 allocations by
    /// `crates/shapes/tests/sparql_path_alloc.rs`. See
    /// [`Self::validate_focus_nodes`] for that closed form, for what the term is
    /// made of, and for the two third-party residuals the guarantee also excludes.
    ///
    /// This is the crate's headline realtime surface, so the claim is stated where
    /// it is called rather than only in a design note: a caller sizing a latency
    /// budget around "re-validate only what changed" is relying on it.
    ///
    /// # Errors
    ///
    /// Returns an error for an id minted against another binding, for an
    /// out-of-range id, or when constraint evaluation hard-fails.
    pub fn validate_focus_node_ids(
        &self,
        focus_node_ids: &[FocusId],
    ) -> Result<ValidationReport, String> {
        // Read once, compared per id: the comparison is one `usize` against a
        // local, with no allocation and no lookup behind it.
        let dataset = self.data.identity();
        // INPUT-sized, and sized: `focus_node_ids.len()` is an exact upper bound
        // on the distinct ids this loop admits, and an unhinted set filling to N
        // reallocates about log2(N) times — a growth term in the focus count,
        // which is the one thing the change path may not carry.
        let mut seen: IdSet =
            IdSet::with_capacity_and_hasher(focus_node_ids.len(), ::purrdf::FastHasher::default());
        let mut focus_nodes = FocusSet::with_capacity(&self.data, focus_node_ids.len());
        for &focus in focus_node_ids {
            if focus.dataset != dataset {
                return Err(format!(
                    "focus node TermId {} was minted against a different dataset binding than the \
                     one this validator is bound to; TermIds are dataset-local, so an in-range id \
                     from another binding resolves to a different term",
                    focus.id.index()
                ));
            }
            // Unreachable through the public surface — an id minted by this
            // binding indexes this binding's table — and kept because the mint is
            // `pub(crate)`, so a future in-crate mint site that got the binding
            // right and the id wrong has to fail here rather than read past the
            // table.
            if focus.id.index() >= self.data.core_view().term_count() {
                return Err(format!(
                    "focus node TermId {} is outside the prepared dataset's {}-term table",
                    focus.id.index(),
                    self.data.core_view().term_count()
                ));
            }
            if seen.insert(focus.id) {
                focus_nodes.push(FocusNode::Interned(focus.id));
            }
        }
        focus_nodes.sort(&self.data);
        self.validate_bounded(&focus_nodes)
    }

    /// The identity this binding gives `term`, or `None` when the dataset never
    /// interned it.
    ///
    /// The bridge between a caller's owned terms and the id-native change path,
    /// and the only way a caller holding a [`Term`] mints a [`FocusId`]: the ids
    /// [`Self::affected_focus_node_ids`] returns and the ids
    /// [`Self::validate_focus_node_ids`] accepts are indices into THIS binding's
    /// term table, stamped with THIS binding, and this is how one is obtained
    /// without re-deriving the table.
    #[must_use]
    pub fn term_id(&self, term: &Term) -> Option<FocusId> {
        resolve_id(self.data.core_view(), term).map(|id| FocusId::new(self.data.identity(), id))
    }

    /// Expand a mutation snapshot into every focus node whose verdict the change
    /// can move — the SOUND input to [`Self::validate_focus_node_ids`].
    ///
    /// `delta` must be the snapshot this binding was built over, through
    /// [`PreparedShapes::bind_delta_with_shapes_graph`]. That is checked, not
    /// assumed: a [`TermId`] is an index, so ids derived from one dataset's changes
    /// would be perfectly valid indices into another dataset's table and would
    /// name the wrong nodes without any lookup ever failing.
    ///
    /// The binding must ALSO read the RDF 1.2 statement projection, and that is
    /// checked here too rather than left to the constructor a caller happened to
    /// reach for. [`ShaclDatasetView::delta`] is public and takes the projection
    /// as an argument, so a delta-backed binding that reads the plain table alone
    /// is reachable through [`PreparedShapes::bind_view`]. The change set is
    /// stated over the whole RDF surface — plain rows and both statement tables —
    /// so on a narrower view a row the overlay demotes off the plain table leaves
    /// the reads while appearing in no change stream, and this answer would be
    /// short by exactly the focus nodes that row moves. Refused, because a short
    /// answer here cannot be told from a clean bill of health.
    /// [`Self::validate_focus_node_ids`] carries no such check and needs none: it
    /// promises nothing about completeness, it validates the nodes it is handed
    /// under whatever read surface the binding has — the same surface
    /// [`Self::validate`] would use — and the only focus set that could be short
    /// is one minted here, which now cannot be minted at all.
    ///
    /// # What it guarantees
    ///
    /// The answer is a SUPERSET of the focus nodes whose validation outcome the
    /// change can alter — in either direction, a violation gained or a violation
    /// lost. It is derived from the dependency footprint of the shapes graph (see
    /// `crate::footprint`), which the shapes-lowering walk emits alongside the
    /// lowering itself, so it covers every route a read can take back to a focus
    /// node: a forward predicate step at any depth of a sequence, an
    /// `sh:inversePath`, a `sh:zeroOrMorePath` / `sh:oneOrMorePath` closure,
    /// `sh:targetSubjectsOf` / `sh:targetObjectsOf`, the `rdf:type` and
    /// `rdfs:subClassOf*` edges behind `sh:targetClass` and `sh:class`, a
    /// property-pair comparand predicate, and the whole nesting of `sh:node`,
    /// `sh:not`, `sh:and` / `sh:or` / `sh:xone` and `sh:qualifiedValueShape`.
    ///
    /// Over-approximating is the safe direction and this deliberately takes it: a
    /// focus node that did not need re-validating merely conforms again.
    ///
    /// # When there is no bounded answer
    ///
    /// A shapes graph that reads through query text this walk does not interpret —
    /// `sh:sparql`, a SPARQL target, a constraint component's `sh:ask` /
    /// `sh:select` validator, a `sh:SPARQLFunction` call, a SPARQL node expression
    /// — has no footprint anyone can bound from the shapes graph alone. That
    /// answers [`FocusExpansion::Everything`], carrying the reason, and the caller
    /// must run [`Self::validate`]. It is reported rather than silently
    /// under-approximated because a short answer here is indistinguishable from a
    /// clean bill of health.
    ///
    /// # Errors
    ///
    /// Returns an error when this validator is not bound to a mutation snapshot,
    /// when that binding does not project the RDF 1.2 statement layer, when
    /// `delta` is not the snapshot this binding reads, and when a changed row
    /// names a term this binding's own view does not map — which is a defect in
    /// this crate rather than in a caller's data.
    pub fn affected_focus_node_ids(
        &self,
        delta: &::purrdf::ir::DeltaDatasetView,
    ) -> Result<FocusExpansion, String> {
        let core = self.data.core_view();
        let bound = core.delta_source().ok_or_else(|| {
            "affected_focus_node_ids: this validator is not bound to a mutation snapshot, so it \
             has no change to expand; bind through PreparedShapes::bind_delta_with_shapes_graph"
                .to_owned()
        })?;
        // The third precondition, and the one that is a property of the BINDING
        // rather than of the argument: the change set names every row that joins
        // or leaves the RDF 1.2 surface — plain rows and both statement tables
        // together — so an expansion derived from it is a superset only for a
        // reader of that same surface. A delta view built without statement
        // projection reads the plain table alone, and the overlay demotes rows
        // off that table without touching the surface (`base_quad_is_ordinary`),
        // so such a row leaves this view's reads while appearing in no added and
        // no suppressed stream. The expansion would be short by exactly it, and a
        // short answer here is indistinguishable from "nothing changed".
        if !core.statements_projected() {
            return Err(
                "affected_focus_node_ids: this validator is bound to a mutation snapshot through a \
                 view that does not project the RDF 1.2 statement layer, so it reads the plain \
                 table alone; a row the overlay demotes onto the annotation table would leave that \
                 read surface without appearing in the change set, and the expansion would silently \
                 omit the focus nodes it moves; bind through \
                 PreparedShapes::bind_delta_with_shapes_graph, which projects statements"
                    .to_owned(),
            );
        }
        if !std::ptr::eq(Arc::as_ptr(bound), std::ptr::from_ref(delta)) {
            return Err(
                "affected_focus_node_ids: the supplied mutation snapshot is not the one this \
                 validator was bound to, and the ids it would produce would index this \
                 validator's term table while naming the other snapshot's terms"
                    .to_owned(),
            );
        }
        if let Some(reason) = self.bound.footprint().opaque() {
            return Ok(FocusExpansion::Everything { reason });
        }
        // The change set in THIS binding's id space, mapped once rather than once
        // per trigger: the trigger loop below rescans it for every read the shapes
        // graph performs.
        let mut changed: Vec<::purrdf::QuadIds> = Vec::new();
        for quad in delta.changed_quads() {
            let ids = [quad.s, quad.p, quad.o].map(|id| core.local_delta_id(id));
            let [Some(s), Some(p), Some(o)] = ids else {
                return Err(format!(
                    "internal change-path defect: a changed row of the bound mutation snapshot \
                     names a term the binding's own {}-term view does not map",
                    core.term_count()
                ));
            };
            changed.push(::purrdf::QuadIds { s, p, o, g: None });
        }
        // The stamp every id this walk emits carries, read once. Dedup and the
        // ordering below stay in the bare id space — the stamp is the same value
        // for every element of one expansion, so it is not part of the key.
        let dataset = self.data.identity();
        let mut seen: IdSet = IdSet::default();
        let mut affected: Vec<FocusId> = Vec::new();
        // The slot row the lowered trigger chains index, resolved once at bind —
        // the same row the validation beside this one reads.
        let binding = self.bound.binding();
        for trigger in self.bound.footprint().triggers() {
            let predicate = match &trigger.predicate {
                // A predicate this dataset never interned cannot be the predicate
                // of a changed row, so this read matches nothing. That is the
                // ordinary "the shapes graph names what the data does not" answer,
                // not a refusal.
                Some(predicate) => match core.term_id_by_iri(predicate.as_str()) {
                    Some(id) => Some(id),
                    None => continue,
                },
                // `sh:closed` binds no predicate: every changed row is a candidate.
                None => None,
            };
            for quad in &changed {
                if predicate.is_some_and(|predicate| predicate != quad.p) {
                    continue;
                }
                let read_node = match trigger.endpoint {
                    Endpoint::Subject => quad.s,
                    Endpoint::Object => quad.o,
                    // An RDF 1.2 reifier declaration carries the read node one level
                    // down, inside the triple term it reifies, so the row is
                    // unpacked before the chain is walked back. A changed
                    // `rdf:reifies` row whose object is NOT a triple term reifies
                    // nothing and is not this read — skipped, not guessed at.
                    Endpoint::ObjectTripleSubject => match core.resolve(quad.o) {
                        ::purrdf::TermRef::Triple { s, .. } => s,
                        ::purrdf::TermRef::Iri(_)
                        | ::purrdf::TermRef::Blank { .. }
                        | ::purrdf::TermRef::Literal { .. } => continue,
                    },
                };
                // The read described forwards, walked backwards. The reversal is
                // stage-0 work and was done there: this walks a LOWERED path, on
                // the evaluator validation itself runs, and neither rebuilds a
                // `Path` nor re-resolves a predicate IRI per changed row.
                match &trigger.reversed {
                    // An empty chain: the focus node IS the node the read happens
                    // at, so there is nothing to walk back.
                    None => {
                        if seen.insert(read_node) {
                            affected.push(FocusId::new(dataset, read_node));
                        }
                    }
                    Some(path) => {
                        for id in
                            crate::path::eval_planned_ids_from_id(core, read_node, path, binding)?
                        {
                            if seen.insert(id) {
                                affected.push(FocusId::new(dataset, id));
                            }
                        }
                    }
                }
            }
        }
        // Canonical order, because a change expansion is a value a caller may log,
        // compare or pin, and first-seen order is a fact about the walk rather than
        // about the change.
        affected.sort_unstable_by_key(|focus| focus.id.index());
        Ok(FocusExpansion::Bounded(affected))
    }

    /// Admit a caller's owned terms as this binding's focus set: resolved,
    /// deduplicated and canonically ordered.
    ///
    /// The owned-term counterpart of the admission loop inside
    /// [`Self::validate_focus_node_ids`], and it has the same two obligations.
    /// Deduplication is in ID space where the dataset interned the term and in
    /// term space where it did not, because two `Term` values that name the same
    /// node must not validate it twice. Ordering is applied here rather than left
    /// to the caller so that the report a focus set produces does not depend on
    /// the order the caller happened to list it in.
    ///
    /// A term the dataset never interned is kept as a FOREIGN focus node, not
    /// dropped. Asking about a node the data graph does not mention is a valid
    /// question with a real answer — a shape may well report a missing
    /// `sh:minCount` against it — and silently discarding it would answer a
    /// different question than the one asked.
    ///
    /// Both working sets are sized from the input, never from the graph, so this
    /// carries no term proportional to the data.
    fn normalize_focus_nodes(&self, focus_nodes: &[Term]) -> FocusSet {
        // Both sets are INPUT-sized, from the same slice, for the same reason the
        // id-native route's is.
        let mut seen_ids: IdSet =
            IdSet::with_capacity_and_hasher(focus_nodes.len(), ::purrdf::FastHasher::default());
        let mut seen_foreign: FastSet<Term> =
            FastSet::with_capacity_and_hasher(focus_nodes.len(), ::purrdf::FastHasher::default());
        let mut normalized = FocusSet::with_capacity(&self.data, focus_nodes.len());
        for term in focus_nodes {
            if let Some(id) = resolve_id(self.data.core_view(), term) {
                if seen_ids.insert(id) {
                    normalized.push(FocusNode::Interned(id));
                }
            } else if seen_foreign.insert(term.clone()) {
                normalized.push(FocusNode::Foreign(term.clone()));
            }
        }
        normalized.sort(&self.data);
        normalized
    }

    /// Validate an ALREADY-ADMITTED focus set — the shared tail of both
    /// focus-node entry points.
    ///
    /// The set is expected to have been through an admission step already
    /// ([`Self::normalize_focus_nodes`], or the id loop in
    /// [`Self::validate_focus_node_ids`]): deduplicated, ordered, and — the part
    /// this method actually re-checks — resolved against THIS binding's dataset.
    ///
    /// Shape selection is inverted here. Rather than asking each shape whether it
    /// contains each focus node, the dispatch is asked once for the whole set
    /// which shapes claim which node, so the cost is per node rather than per
    /// (shape, focus node) pair. Every non-deactivated shape is still PLANNED
    /// unconditionally, before its claims are consulted: a shape whose plan
    /// cannot be built is a defect in this crate, and a request that happened to
    /// claim none of its nodes must not be the reason it goes unreported.
    ///
    /// # Errors
    ///
    /// Returns an error when the focus set belongs to another binding, when a
    /// shape cannot be planned, or when a constraint evaluation hard-fails.
    fn validate_bounded(&self, focus_nodes: &FocusSet) -> Result<ValidationReport, String> {
        let focus_nodes = focus_nodes.nodes_of(&self.data)?;
        if focus_nodes.is_empty() {
            return Ok(finish_report(Vec::new()));
        }
        // The dual question, asked once for the whole focus set: not "shape, do
        // you contain this node?" once per (shape, focus node) pair, but "node,
        // which shapes claim you?" once per node. See [`TargetDispatch`].
        let claims =
            self.bound
                .dispatch()
                .claims(&self.data, focus_nodes, self.shapes.node_shapes.len());
        let mut all_results = Vec::new();
        for (position, shape) in self.shapes.node_shapes.iter().enumerate() {
            if shape.deactivated {
                continue;
            }
            // Planned unconditionally, before the empty-claims check: a shape
            // whose plan cannot be built is a defect in this crate, and a bounded
            // request that happened to claim none of its nodes must not be the
            // reason it goes unreported.
            let plan = self.bound.plan(shape, position)?;
            let Some(claimed) = claims.get(position).filter(|claimed| !claimed.is_empty()) else {
                continue;
            };
            all_results.extend(evaluate_shape_focus_nodes(
                &self.data,
                &self.shapes,
                plan,
                focus_nodes,
                |focus| claimed.contains(focus),
            )?);
        }
        Ok(finish_report(all_results))
    }
}

/// Construct the prepared validation-scoped class-membership dataset view.
#[doc(hidden)]
#[must_use]
pub fn __prepared_class_membership_view(
    dataset: Arc<RdfDataset>,
) -> impl ::purrdf::DatasetView<Id = TermId, ProbePlan = ::purrdf::ir::QuadProbePlan> + Clone + Send + Sync
{
    let view = crate::class_membership::ClassMembershipView::new(dataset);
    view.prepare();
    view
}

/// Validate a [`ShaclData`] holder against `shapes`.
///
/// This is the single engine core; [`validate_dataset`] (the IR entry point)
/// bottoms out here.
///
/// For every non-deactivated node shape, the focus node set is resolved from the
/// shape's target declarations and each focus node is validated against the shape
/// via [`crate::constraints::validate_shape`]. Results are sorted by `(focus_node,
/// source_constraint_component, source_shape, result_path, value)` so reports are
/// identical across runs.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure: a SHACL-SPARQL target
/// or constraint query the engine cannot evaluate.
pub fn validate_with(data: &ShaclData, shapes: &Shapes) -> Result<ValidationReport, String> {
    validate_with_focus_filter(data, shapes, |_, _| true)
}

/// The outcome of a **governed** validation: a complete report, or a budget that ran out
/// before one existed.
///
/// # Why there is no partial report
///
/// The evaluator hands a governed query its partial answers, because a caller who asked
/// for a bound on the answer can use a bound on the answer. A conformance verdict is not
/// that caller. Every SHACL constraint is a *negative* claim — "no solution of this
/// query violates the shape" — so a truncated solution bag and a complete one that found
/// nothing produce the identical sentence, and `conforms` computed from the first means
/// nothing at all. Even the violations already found are not safely reportable as a
/// report: a `ValidationReport` states which constraints were checked, and a truncated one
/// silently redefines that. So a trip yields the trip and the evidence, and no report.
#[derive(Debug, Clone)]
pub enum GovernedValidation {
    /// Every governor stayed intact and this is the validation's complete report.
    Complete {
        /// The deterministically-sorted report.
        report: ValidationReport,
        /// What the validation's SPARQL paths consumed, and against which ceilings.
        evidence: GovernorEvidence,
    },
    /// A governor stopped the validation. See [`GovernedValidation`] for why no partial
    /// report accompanies it.
    BudgetExhausted {
        /// The governor that stopped it.
        tripped: TrippedGovernor,
        /// What had been consumed when it stopped.
        evidence: GovernorEvidence,
    },
}

impl GovernedValidation {
    /// What the validation's SPARQL paths consumed, whichever outcome it reached.
    #[must_use]
    pub const fn evidence(&self) -> &GovernorEvidence {
        match self {
            Self::Complete { evidence, .. } | Self::BudgetExhausted { evidence, .. } => evidence,
        }
    }

    /// The governor that stopped this validation, or `None` if it completed.
    #[must_use]
    pub const fn tripped(&self) -> Option<TrippedGovernor> {
        match self {
            Self::Complete { .. } => None,
            Self::BudgetExhausted { tripped, .. } => Some(*tripped),
        }
    }
}

/// Validate under caller-supplied execution governors.
///
/// # One budget for the whole validation
///
/// SHACL runs one SPARQL query per focus node — `sh:SPARQLTarget` resolution, every
/// `sh:sparql` constraint, every SHACL-AF node expression and rule. All of them charge
/// **one** [`GovernorState`], built here and dropped with this call, so `governors` bounds
/// the validation a caller asked about rather than each of the hundreds of queries it
/// happens to decompose into. Governed focus nodes execute serially in source order so
/// the state has one deterministic charge order; ordinary validation retains parallel
/// focus execution.
///
/// Governors bound the **SPARQL** paths. Core constraint evaluation reads the IR
/// directly and spends no evaluator budget, so a shapes graph with no SHACL-SPARQL and no
/// SHACL-AF in it validates under any budget, including a zero one — which is the honest
/// answer, not an oversight.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]). A tripped
/// governor is **not** an error: it is the [`GovernedValidation::BudgetExhausted`]
/// outcome.
pub fn validate_with_governors(
    data: &ShaclData,
    shapes: &Shapes,
    governors: &QueryGovernors,
) -> Result<GovernedValidation, String> {
    let state = Arc::new(GovernorState::new(governors));
    let outcome = {
        let _governor_scope = crate::sparql::enter_governor_scope(Arc::clone(&state));
        validate_with_focus_filter(data, shapes, |_, _| true)
    };
    let evidence = state.evidence();
    // The trip is read from the state, not from the error text: `crate::sparql` turns a
    // trip into an `Err` at the query site so no truncated bag can reach a verdict, and
    // the state is where the TYPED identity of that trip survives. Reading it back here
    // is also what keeps the two reports of one trip — the outcome and the evidence —
    // from being two independently-derived answers that could disagree.
    match (outcome, state.tripped()) {
        (_, Some(tripped)) => Ok(GovernedValidation::BudgetExhausted { tripped, evidence }),
        (Ok(report), None) => Ok(GovernedValidation::Complete { report, evidence }),
        (Err(message), None) => Err(message),
    }
}

/// [`validate_with_governors`] over a frozen dataset, with the shapes graph exposed to
/// SHACL-SPARQL paths exactly as [`validate_dataset_with_shapes_graph`] exposes it.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_dataset_with_governors(
    data: &RdfDataset,
    shapes: &Shapes,
    shapes_graph_iri: Option<&str>,
    governors: &QueryGovernors,
) -> Result<GovernedValidation, String> {
    let projected = project_dataset(data)?;
    let data = build_projected_data(projected, shapes, shapes_graph_iri)?;
    validate_with_governors(&data, shapes, governors)
}

/// WHICH QUESTION a change-path report answered.
///
/// A [`ValidationReport`] alone cannot say. An incremental run reports about the
/// focus nodes the change could move, so `conforms` means *this change introduced
/// no violation*; a run whose footprint could not be bounded validated the whole
/// graph, so `conforms` means *the graph conforms*. Both are honest answers and
/// they are not the same answer, which is why [`validate_change`] returns this
/// beside the report rather than leaving a caller to assume one of them.
///
/// The two-answer shape of [`FocusExpansion`] is preserved deliberately: an empty
/// bounded expansion and "every focus node in the graph" are opposite
/// instructions, and a single count — with the fallback spelled as some large
/// number — would collapse them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeScope {
    /// The change's footprint was bounded, and the report covers exactly the
    /// focus nodes named by that expansion.
    Bounded {
        /// How many focus nodes the expansion named. `0` means the change moved
        /// nothing the shapes graph reads — not that nothing was checked.
        focus_nodes: usize,
    },
    /// No bounded footprint exists for this shapes graph, so the run fell back to
    /// a full [`PreparedValidator::validate`] of the mutated graph.
    Everything {
        /// Which construct made the footprint unreadable — actionable rather than
        /// decorative: it names what to change to get incremental validation back.
        reason: &'static str,
    },
}

impl ChangeScope {
    /// Whether the change's footprint could be bounded.
    #[must_use]
    pub const fn is_bounded(&self) -> bool {
        matches!(self, Self::Bounded { .. })
    }

    /// How many focus nodes the bounded expansion named, or `None` on the
    /// fallback — where "every focus node in the graph" is not a number.
    #[must_use]
    pub const fn focus_nodes(&self) -> Option<usize> {
        match self {
            Self::Bounded { focus_nodes } => Some(*focus_nodes),
            Self::Everything { .. } => None,
        }
    }

    /// Why the footprint was unbounded, or `None` when it was bounded.
    #[must_use]
    pub const fn reason(&self) -> Option<&'static str> {
        match self {
            Self::Bounded { .. } => None,
            Self::Everything { reason } => Some(*reason),
        }
    }
}

/// A change-path validation: the report, plus the [`ChangeScope`] it describes.
#[derive(Debug)]
pub struct ChangeValidation {
    /// The SHACL report. See [`Self::scope`] for what it is a report ABOUT.
    pub report: ValidationReport,
    /// Which question [`Self::report`] answered.
    pub scope: ChangeScope,
}

/// A governed change-path validation: the [`ChangeScope`], plus the governed
/// outcome — a report and its evidence, or the budget that stopped it.
///
/// The scope sits OUTSIDE the outcome because it is known before any constraint
/// is evaluated, so it survives a tripped budget: an operator whose incremental
/// run ran out of fuel still learns which question the run was asking.
#[derive(Debug)]
pub struct GovernedChangeValidation {
    /// Which question the run was asking, known before the first query ran.
    pub scope: ChangeScope,
    /// The governed outcome, exactly as [`validate_with_governors`] reports it.
    pub outcome: GovernedValidation,
}

/// Expand a bound mutation snapshot into the focus nodes it can move, then
/// validate exactly those — the `delta` → [`PreparedValidator::affected_focus_node_ids`]
/// → [`PreparedValidator::validate_focus_node_ids`] loop, as ONE call.
///
/// `validator` must be bound to `delta` through
/// [`PreparedShapes::bind_delta_with_shapes_graph`]; binding is the caller's,
/// because a caller owns how its change was assembled (a parsed patch document, a
/// store's copy-on-write delta, a pair of change graphs) and this owns only what
/// is done with it afterwards.
///
/// # The fallback is not optional
///
/// A shapes graph whose constraints read through SPARQL query text has no
/// footprint anyone can bound, and [`PreparedValidator::affected_focus_node_ids`]
/// says so rather than returning a short answer. This honours that by running a
/// full [`PreparedValidator::validate`] and reporting
/// [`ChangeScope::Everything`] — because a short expansion and a clean bill of
/// health are indistinguishable in a report, which is exactly why the expansion
/// refuses to guess.
///
/// # Nothing sits between the two halves
///
/// The ids the expansion answers are stamped with the binding that minted them
/// and [`PreparedValidator::validate_focus_node_ids`] refuses ids from any other,
/// so the expansion feeds the validator UNCONVERTED. That is the property this
/// entry point exists to hold in ONE place rather than in each surface that
/// drives the change path.
///
/// # Errors
///
/// Returns `Err(String)` when `validator` is not bound to `delta`, and on a hard
/// validation failure (see [`validate_with`]).
pub fn validate_change(
    validator: &PreparedValidator,
    delta: &::purrdf::ir::DeltaDatasetView,
) -> Result<ChangeValidation, String> {
    let (scope, report) = change_pass(validator, delta)?;
    report.map(|report| ChangeValidation { report, scope })
}

/// [`validate_change`] under caller-supplied execution governors.
///
/// # One budget for the whole change validation
///
/// The same arrangement [`validate_with_governors`] makes, over the same scope
/// guard: **one** [`GovernorState`], built here and dropped with this call, so a
/// ceiling bounds the incremental validation a caller asked about rather than
/// each of the queries it decomposes into. The trip is read back off the state
/// that latched it rather than parsed out of an error string, so the outcome and
/// the evidence cannot be two independently-derived answers about one trip.
///
/// It matters most on the [`ChangeScope::Everything`] fallback, which is
/// precisely the case where the shapes graph runs SPARQL. A bounded expansion
/// executes no query text at all — that is *why* it could be bounded — and so
/// consumes nothing, which is the honest answer rather than an oversight.
///
/// # Errors
///
/// Returns `Err(String)` when `validator` is not bound to `delta`, and on a hard
/// validation failure. A tripped governor is **not** an error: it is the
/// [`GovernedValidation::BudgetExhausted`] outcome, carried beside the scope.
pub fn validate_change_with_governors(
    validator: &PreparedValidator,
    delta: &::purrdf::ir::DeltaDatasetView,
    governors: &QueryGovernors,
) -> Result<GovernedChangeValidation, String> {
    let state = Arc::new(GovernorState::new(governors));
    let pass = {
        let _governor_scope = crate::sparql::enter_governor_scope(Arc::clone(&state));
        change_pass(validator, delta)
    };
    let evidence = state.evidence();
    // `?` before the trip is read, and it cannot swallow one: the expansion
    // executes no query text, so nothing can charge the state until the scope is
    // already decided. An `Err` here is therefore a refusal of the binding, never
    // a budget that stopped a query nobody ran.
    let (scope, outcome) = pass?;
    match (outcome, state.tripped()) {
        (_, Some(tripped)) => Ok(GovernedChangeValidation {
            scope,
            outcome: GovernedValidation::BudgetExhausted { tripped, evidence },
        }),
        (Ok(report), None) => Ok(GovernedChangeValidation {
            scope,
            outcome: GovernedValidation::Complete { report, evidence },
        }),
        (Err(message), None) => Err(message),
    }
}

/// The loop itself, with the scope reported SEPARATELY from the validation's own
/// result so a governed caller can still say which question was asked when the
/// answer is a tripped budget rather than a report.
fn change_pass(
    validator: &PreparedValidator,
    delta: &::purrdf::ir::DeltaDatasetView,
) -> Result<(ChangeScope, Result<ValidationReport, String>), String> {
    let expansion = validator.affected_focus_node_ids(delta)?;
    Ok(match &expansion {
        FocusExpansion::Bounded(ids) => (
            ChangeScope::Bounded {
                focus_nodes: ids.len(),
            },
            validator.validate_focus_node_ids(ids),
        ),
        FocusExpansion::Everything { reason } => {
            (ChangeScope::Everything { reason }, validator.validate())
        }
    })
}

/// Validate with an explicit focus-node filter.
///
/// The filter is called after target resolution and before constraint evaluation.
/// It lets callers that already know only a bounded set of focus nodes changed
/// avoid rechecking the clean base graph, while still resolving targets against
/// the full data graph. Because this compatibility surface still enumerates each
/// whole target set, realtime callers should prepare a [`PreparedValidator`] and
/// use [`PreparedValidator::validate_focus_nodes`] instead.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_with_focus_filter<F>(
    data: &ShaclData,
    shapes: &Shapes,
    mut include_focus: F,
) -> Result<ValidationReport, String>
where
    F: FnMut(&Shape, &Term) -> bool,
{
    let lowered = Arc::new(crate::plan::lower_shapes(shapes.node_shapes.iter()));
    let classes = Arc::clone(lowered.classes());
    let bound = BoundShapes::bind_untargeted(data, lowered, classes);
    validate_with_plan_and_focus_filter(data, shapes, &bound, &mut include_focus)
}

/// Validate a frozen [`::purrdf::RdfDataset`] against parsed SHACL shapes, IR-natively.
///
/// The generic engine reads pattern lookups DIRECTLY from a SHACL projection of
/// the IR. Native and SHACL-SPARQL evaluation share a validation-scoped view
/// that adds only class memberships implied by asserted `rdfs:subClassOf`
/// edges; it does not copy or expand the underlying graph. Named graphs are
/// flattened so GTS bundle partitions behave like the repository's Turtle
/// source merge, which loads all inputs into one default graph.
///
/// # Errors
///
/// Returns an error string if the SHACL projection cannot be frozen into the IR.
pub fn validate_dataset(data: &RdfDataset, shapes: &Shapes) -> Result<ValidationReport, String> {
    let dataset = project_dataset(data)?;
    // The engine reads pattern lookups directly from the frozen IR; SHACL-SPARQL
    // paths run the native SPARQL engine over the same `Arc<RdfDataset>`.
    validate_projected_dataset(dataset, shapes)
}

/// Validate an already-SHACL-projected dataset.
///
/// Call [`project_dataset`] first when the same base graph is reused across many
/// overlays; this avoids flattening/reifier-projecting the base graph on every
/// validation pass.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_projected_dataset(
    projected: Arc<RdfDataset>,
    shapes: &Shapes,
) -> Result<ValidationReport, String> {
    // Core lookups and the SHACL-SPARQL paths run over the same `Arc<RdfDataset>`.
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    validate_with(&data, shapes)
}

/// Validate an already-SHACL-projected dataset with a focus-node filter.
///
/// # Errors
///
/// Returns `Err(String)` on a hard validation failure (see [`validate_with`]).
pub fn validate_projected_dataset_with_focus_filter<F>(
    projected: Arc<RdfDataset>,
    shapes: &Shapes,
    include_focus: F,
) -> Result<ValidationReport, String>
where
    F: FnMut(&Shape, &Term) -> bool,
{
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    validate_with_focus_filter(&data, shapes, include_focus)
}

/// Assemble the Core and SHACL-SPARQL views of an already-projected dataset.
///
/// Without a shapes graph both consumers retain the same view, sharing its lazy
/// class-membership analysis. When a shapes-graph IRI is known, SPARQL receives a
/// composite with every shapes row placed into that named graph.
fn build_projected_data(
    data: Arc<RdfDataset>,
    shapes: &Shapes,
    override_graph: Option<&str>,
) -> Result<ShaclData, String> {
    let core = Arc::new(ShaclDatasetView::native(Arc::clone(&data)));
    let (sparql, graph) = build_sparql_view(
        ::purrdf::ir::CompositeSource::new(data),
        Arc::clone(&core),
        shapes,
        override_graph,
        ::purrdf::ir::ViewLimits::default(),
    )?;
    Ok(ShaclData::from_views(core, sparql, graph))
}

fn build_sparql_view(
    source: ::purrdf::ir::CompositeSource,
    core: Arc<ShaclDatasetView>,
    shapes: &Shapes,
    override_graph: Option<&str>,
    limits: ::purrdf::ir::ViewLimits,
) -> Result<(Arc<ShaclDatasetView>, Option<String>), String> {
    let graph_iri = override_graph
        .map(str::to_owned)
        .or_else(|| shapes.shapes_graph.clone());
    let Some(graph_iri) = graph_iri else {
        return Ok((core, None));
    };
    let shapes_source = ::purrdf::ir::CompositeSource::new(Arc::clone(&shapes.shapes_dataset))
        .with_graph_placement(::purrdf::ir::GraphPlacement::Named(
            ::purrdf::TermValue::iri(&graph_iri),
        ));
    // Parsed shape constants and data bindings already carry their explicit blank
    // scopes. Preserve those identities when placing their records into graphs.
    let composite = ::purrdf::ir::CompositeDatasetView::from_shared_sources(
        vec![
            source.with_graph_placement(::purrdf::ir::GraphPlacement::Default),
            shapes_source,
        ],
        limits,
    )
    .map_err(|error| error.to_string())?;
    let view = ShaclDatasetView::composite(Arc::new(composite), false, limits)?
        .with_statement_projection();
    Ok((Arc::new(view), Some(graph_iri)))
}

/// Validate a frozen [`RdfDataset`] against parsed SHACL shapes, exposing the
/// shapes graph as a named graph to SHACL-SPARQL paths.
///
/// `shapes_graph_iri` overrides [`Shapes::shapes_graph`] when both are present.
pub fn validate_dataset_with_shapes_graph(
    data: &RdfDataset,
    shapes: &Shapes,
    shapes_graph_iri: Option<&str>,
) -> Result<ValidationReport, String> {
    let projected = project_dataset(data)?;
    validate_projected_dataset_with_shapes_graph(projected, shapes, shapes_graph_iri)
}

/// Validate an already-projected dataset with a shapes-graph overlay.
pub fn validate_projected_dataset_with_shapes_graph(
    projected: Arc<RdfDataset>,
    shapes: &Shapes,
    shapes_graph_iri: Option<&str>,
) -> Result<ValidationReport, String> {
    let data = build_projected_data(projected, shapes, shapes_graph_iri)?;
    validate_with(&data, shapes)
}

/// Build a SHACL-projection dataset from the source [`RdfDataset`], flattening
/// every quad into the default graph and materializing reifier bindings as
/// `rdf:reifies` triples and statement annotations as plain triples.
pub fn project_dataset(data: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    use ::purrdf::RdfDatasetBuilder;
    use purrdf::{RdfQuad, RdfTerm};

    let mut builder = RdfDatasetBuilder::new();

    for mut quad in data.owned_quads() {
        // FlattenToDefaultGraph: drop the source graph name.
        quad.graph_name = None;
        builder.push_owned_quad(&quad);
    }

    // Reifiers → `(reifier, rdf:reifies, <<triple>>)` triples.
    for reifier in data.owned_reifiers() {
        builder.push_owned_quad(&RdfQuad::new(
            reifier.reifier,
            RDF_REIFIES,
            RdfTerm::triple(reifier.statement),
        ));
    }

    // Annotations → `(reifier, predicate, object)` triples.
    for annotation in data.owned_annotations() {
        builder.push_owned_quad(&RdfQuad::new(
            annotation.reifier,
            annotation.predicate,
            annotation.object,
        ));
    }

    builder.freeze().map_err(|e| e.to_string())
}

#[cfg(test)]
pub(crate) fn shacl_dataset_from_dataset(data: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    project_dataset(data)
}

/// The `rdf:reifies` predicate IRI, used to project reifier bindings into the
/// quad table so SHACL's reifier-shape lookups can find them.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Parse a SHACL shapes graph from a Turtle string, resolving its relative IRI
/// references against `base`.
///
/// Creates an in-memory store, loads the shapes graph with prefix extraction,
/// and parses it into a reusable [`Shapes`] model. The parsed model can be
/// shared across multiple data graphs via `validate`, eliminating the cost of
/// re-parsing shapes for every validation phase.
///
/// # `base` is required at the call, not defaulted
///
/// A shapes graph is RDF, and a SHACL author writes `<PersonShape>` in it exactly as
/// they would in any other Turtle document. `base` is therefore the same
/// caller-supplied base (RFC-3986 §5.1.2) [`parse_dataset`](::purrdf::parse_dataset)
/// takes, threaded to the identical resolution layer — so a shapes document read
/// through this boundary resolves byte-for-byte like the same document read through
/// any other one.
///
/// It is a positional parameter rather than a defaulted convenience overload on
/// purpose. The overload is what let this seam silently receive no base while every
/// sibling seam got one, and the divergence was invisible precisely because nothing at
/// a call site had to mention it. `None` remains a legitimate answer — an in-document
/// `@base` can still establish one, and with neither, a relative reference is a hard
/// `iri-relative-no-base` — but it is now an answer somebody gave.
///
/// # Errors
///
/// Returns an error string if the shapes Turtle fails to parse (including a relative
/// IRI reference with no base in scope) or contains unsupported SHACL constructs.
pub fn parse_shapes(shapes_ttl: &str, base: Option<&str>) -> Result<Shapes, String> {
    parse_shapes_with_config(shapes_ttl, base, None)
}

/// [`parse_shapes`] with the caller-supplied [`BoxRoleVocab`](crate::model::BoxRoleVocab)
/// (`crate::model::BoxRoleVocab`) threaded through.
///
/// PurRDF mints no vocabulary IRIs, so the box-role annotation feature has no
/// default vocabulary: with `box_role_vocab = None` it is INACTIVE (shapes
/// parse fine, but no role annotations are collected or stamped).
///
/// # Errors
///
/// Returns an error string if the shapes Turtle fails to parse or contains
/// unsupported SHACL constructs.
pub fn parse_shapes_with_config(
    shapes_ttl: &str,
    base: Option<&str>,
    box_role_vocab: Option<crate::model::BoxRoleVocab>,
) -> Result<Shapes, String> {
    // Parse the shapes graph via the native purrdf codecs — no
    // the oxigraph `io` parser. The native codec drops document prefixes once it folds to
    // the IR, so we recover the `@prefix`/SPARQL `PREFIX` map by scanning the
    // source text: SHACL-AF sh:select queries (and pySHACL) rely on prefixed
    // names. Syntax failures are accumulated per independently recoverable
    // statement so a SHACL author sees the complete actionable set in one pass.
    let shapes_dataset = crate::text_ingest::parse_turtle_to_dataset(shapes_ttl, base)
        .map_err(|errors| errors.join("\n"))?;
    let doc_prefixes = crate::text_ingest::extract_prefixes(shapes_ttl);

    // `base` and `doc_prefixes` are handed on rather than consumed and dropped:
    // this is the only seam that ever sees them, and both decided what the source
    // text means (the base resolved its relative IRI references; the prefix map is
    // baked into every SHACL-AF query body below). A `Shapes` that could not report
    // them would force any consumer needing its identity to accept that identity as
    // an argument, which makes it a caller's claim instead of a fact about the parse.
    crate::shapes::from_dataset_with_base(
        &shapes_dataset,
        base,
        &doc_prefixes,
        box_role_vocab,
        None,
    )
}

/// Validate data (N-Triples) against shapes (Turtle), returning a [`ValidationReport`].
///
/// Creates an in-memory data store, loads the data graph, parses shapes once
/// via [`parse_shapes`], and delegates to `validate`.
///
/// The data graph is loaded with the **lenient** RDF parser. A validator must be
/// able to ingest the data graph before it can validate any shapes against it,
/// and RDF lexical well-formedness is a separate concern from SHACL conformance.
/// Caller datasets can carry private-use language tags whose subtags exceed
/// BCP-47's eight-character limit. The strict parser rejects the complete graph
/// on those lexical forms; lenient parsing keeps RDF ingestion separate from the
/// SHACL conformance decision.
///
/// # Errors
///
/// Returns an error string if either graph fails to parse.
pub fn validate_graphs(
    data_nt: &str,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
) -> Result<ValidationReport, String> {
    validate_graphs_with_config(data_nt, shapes_ttl, shapes_base, None)
}

/// [`validate_graphs`] with the caller-supplied [`BoxRoleVocab`](crate::model::BoxRoleVocab)
/// (`crate::model::BoxRoleVocab`) threaded through to shape parsing and
/// validation. `None` leaves the box-role feature inactive.
///
/// # Errors
///
/// Returns an error string if either graph fails to parse.
pub fn validate_graphs_with_config(
    data_nt: &str,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    box_role_vocab: Option<crate::model::BoxRoleVocab>,
) -> Result<ValidationReport, String> {
    // Parse the data graph via the native codecs. Every independently malformed
    // N-Triples line is reported in one pass, matching `parse_shapes`' complete
    // syntax-diagnostic contract.
    let data = crate::text_ingest::parse_ntriples_to_dataset(data_nt)
        .map_err(|errors| errors.join("\n"))?;

    let shapes = parse_shapes_with_config(shapes_ttl, shapes_base, box_role_vocab)?;
    validate_dataset(data.as_ref(), &shapes)
}

/// Validate a frozen [`::purrdf::RdfDataset`] against a Turtle SHACL shapes graph.
///
/// # Errors
///
/// Returns an error string if the shapes graph fails to parse or if the SHACL
/// projection cannot be frozen.
pub fn validate_dataset_graphs(
    data: &RdfDataset,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
) -> Result<ValidationReport, String> {
    let shapes = parse_shapes(shapes_ttl, shapes_base)?;
    validate_dataset(data, &shapes)
}

/// Entail data (N-Triples) under shapes (Turtle), returning the materialized
/// dataset (the base graph plus every SHACL-AF `sh:rule` inference, run to a
/// fixpoint).
///
/// The text-in twin of [`crate::entail_dataset`], mirroring [`validate_graphs`]:
/// it parses the data graph (lenient N-Triples, one pass reporting every
/// malformed line) and the shapes graph (Turtle), then applies every active rule.
/// The returned [`Arc<RdfDataset>`] is a NEW frozen dataset of base ⊎ inferred
/// triples the caller serializes however its surface emits RDF.
///
/// # Errors
///
/// Returns an error string if either graph fails to parse or if rule application
/// fails (an illegal head term, an unresolvable `sh:condition`, or a rule set that
/// does not reach a fixpoint — see [`crate::apply_rules`]).
pub fn entail_graphs(
    data_nt: &str,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
) -> Result<Arc<RdfDataset>, String> {
    let data = crate::text_ingest::parse_ntriples_to_dataset(data_nt)
        .map_err(|errors| errors.join("\n"))?;
    let shapes = parse_shapes(shapes_ttl, shapes_base)?;
    crate::rules::entail_dataset(data.as_ref(), &shapes)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Severity;
    use crate::shapes::Shapes;

    const PREFIXES: &str = r"
        @prefix sh:   <http://www.w3.org/ns/shacl#> .
        @prefix ex:   <http://example.org/ns#> .
        @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
    ";

    fn load_data_nt(nt: &str) -> Arc<RdfDataset> {
        crate::text_ingest::parse_ntriples_to_dataset(nt).expect("data N-Triples must parse")
    }

    fn load_shapes_ttl(ttl: &str) -> Shapes {
        let dataset = crate::text_ingest::parse_turtle_to_dataset(ttl, None)
            .expect("shapes Turtle must parse");
        crate::shapes::from_dataset(&dataset).expect("shapes parse must succeed")
    }

    // ── Target selection: three implementations, one predicate ────────────────

    /// The namespace the target-agreement fixtures live in.
    const TARGET_NS: &str = "http://example.org/ns#";

    /// Bind `shapes_body` against `data_body` and return the bound validator.
    fn bind_target_fixture(shapes_body: &str, data_body: &str) -> (Arc<Shapes>, PreparedValidator) {
        let shapes = Arc::new(load_shapes_ttl(&format!("{PREFIXES}{shapes_body}")));
        let data =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{data_body}"), None)
                .expect("fixture data must parse");
        let data = project_dataset(&data).expect("fixture data must project");
        let validator = PreparedShapes::new(Arc::clone(&shapes))
            .bind_projected_dataset(data)
            .expect("fixture must bind");
        (shapes, validator)
    }

    /// EVERY node this dataset can name, plus terms it never interned.
    ///
    /// A target set is not only about nodes the graph mentions — `sh:targetNode`
    /// may name one that is absent — so the candidate set deliberately includes
    /// both an absent term that IS an explicit target and one that is not.
    fn every_candidate_focus_node(validator: &PreparedValidator) -> Vec<FocusNode> {
        let view = validator.data.core_view();
        let mut candidates: Vec<FocusNode> = (0..view.term_count())
            .map(|index| {
                FocusNode::Interned(TermId::from_index(
                    u32::try_from(index).expect("fixture fits in u32"),
                ))
            })
            .collect();
        for local in ["neverInterned", "neverInternedAndNotATarget"] {
            candidates.push(FocusNode::Foreign(Term::NamedNode(
                NamedNode::new_unchecked(format!("{TARGET_NS}{local}")),
            )));
        }
        candidates
    }

    /// The focus nodes each of the three implementations selects, per shape
    /// position: `(contains, dispatch, resolve_all)`.
    fn target_selections(
        shapes: &Shapes,
        validator: &PreparedValidator,
    ) -> Vec<(
        std::collections::BTreeSet<String>,
        std::collections::BTreeSet<String>,
        std::collections::BTreeSet<String>,
    )> {
        let candidates = every_candidate_focus_node(validator);
        let claims = validator.bound.dispatch().claims(
            &validator.data,
            &candidates,
            shapes.node_shapes.len(),
        );
        let key = |focus: &FocusNode| focus.to_term(validator.data.core_view()).to_string();
        (0..shapes.node_shapes.len())
            .map(|position| {
                let targets = &validator.bound.targets[position];
                let by_contains = candidates
                    .iter()
                    .filter(|focus| targets.contains(&validator.data, focus))
                    .map(key)
                    .collect();
                let by_dispatch = candidates
                    .iter()
                    .filter(|focus| claims[position].contains(focus))
                    .map(key)
                    .collect();
                let by_resolve_all = targets
                    .resolve_all(&validator.data)
                    .iter()
                    .map(key)
                    .collect();
                (by_contains, by_dispatch, by_resolve_all)
            })
            .collect()
    }

    /// **`PreparedTargets::contains`, `PreparedTargets::resolve_all` and
    /// `TargetDispatch` select the identical focus-node set — for every `Target`
    /// variant there is.**
    ///
    /// Three implementations of one predicate. `contains` DEFINES it, one
    /// (shape, node) pair at a time; `resolve_all` enumerates it forwards over the
    /// whole graph; the dispatch index answers it backwards, from the node. Two
    /// implementations of one predicate with no equivalence test is how they come
    /// to disagree, and a disagreement here is not a slow validator but a wrong
    /// one: a node the dispatch fails to claim is a node NO shape validates, which
    /// is a silent drop with a conforming report to hide behind.
    ///
    /// The candidate set is exhaustive over the dataset's whole term table rather
    /// than over a hand-listed set of "interesting" nodes, so a predicate that
    /// over-selects (an IRI used only as a datatype, say) fails here too — the
    /// mirror failure, and the one an under-selection test cannot see.
    #[test]
    fn target_dispatch_agrees_with_contains_and_resolve_all() {
        let (shapes, validator) = bind_target_fixture(
            r#"
            ex:ClassShape a sh:NodeShape ; sh:targetClass ex:Person ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:ImplicitShape a sh:NodeShape, rdfs:Class ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:SubjectsShape a sh:NodeShape ; sh:targetSubjectsOf ex:link ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:ObjectsShape a sh:NodeShape ; sh:targetObjectsOf ex:link ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:ExplicitShape a sh:NodeShape ;
                sh:targetNode ex:explicit, ex:neverInterned ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:SparqlShape a sh:NodeShape ;
                sh:target [ a sh:SPARQLTarget ; sh:select
                    "SELECT ?this WHERE { ?this <http://example.org/ns#active> true }" ] ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            "#,
            r"
            ex:Child rdfs:subClassOf ex:Person .
            ex:alice a ex:Child .
            ex:bob a ex:Person .
            ex:implicitInstance a ex:ImplicitShape .
            ex:tail ex:link ex:head .
            ex:explicit ex:required ex:anything .
            ex:activeNode ex:active true .
            ",
        );

        // The fixture's claim to be exhaustive is itself checked: a fixture that
        // quietly stopped covering a variant would leave this test green while
        // testing less, which is the failure mode an agreement test is for.
        let mut covered = [false; 6];
        for target in shapes.node_shapes.iter().flat_map(|shape| &shape.targets) {
            covered[match target {
                Target::Class(_) => 0,
                Target::SubjectsOf(_) => 1,
                Target::ObjectsOf(_) => 2,
                Target::Node(_) => 3,
                Target::ImplicitClass(_) => 4,
                Target::Sparql { .. } => 5,
            }] = true;
        }
        assert!(
            covered.iter().all(|&seen| seen),
            "the fixture must exercise EVERY Target variant, got {covered:?}",
        );

        for (position, (by_contains, by_dispatch, by_resolve_all)) in
            target_selections(&shapes, &validator)
                .into_iter()
                .enumerate()
        {
            let shape = &shapes.node_shapes[position].id;
            assert!(
                !by_contains.is_empty(),
                "{shape}: selects nothing, so its agreement is vacuous",
            );
            assert_eq!(
                by_contains, by_dispatch,
                "{shape}: the inverted dispatch disagrees with `contains`",
            );
            assert_eq!(
                by_contains, by_resolve_all,
                "{shape}: `resolve_all` disagrees with `contains`",
            );
        }
    }

    /// **The subjects-of and objects-of loops select opposite ends of the same
    /// quad.**
    ///
    /// `resolve_all`'s two predicate loops use an IDENTICAL quad pattern —
    /// `(?, predicate, ?)`, open on both ends — and differ only in taking
    /// `quad.s` or `quad.o`. That is correct by construction, and it is also
    /// invisible to any fixture whose subjects-of and objects-of sets overlap.
    /// This one makes them DISJOINT, so swapping the two ends, or "tidying" the
    /// object loop into binding its object, fails immediately.
    #[test]
    fn resolve_all_is_asymmetric_in_the_subject_and_object_directions() {
        let (shapes, validator) = bind_target_fixture(
            r"
            ex:SubjectsShape a sh:NodeShape ; sh:targetSubjectsOf ex:link ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:ObjectsShape a sh:NodeShape ; sh:targetObjectsOf ex:link ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ",
            "ex:tail ex:link ex:head .",
        );
        // `node_shapes` is not in declaration order, so the expected end is keyed
        // by the shape that declares it rather than by position.
        let selections = target_selections(&shapes, &validator);
        for (position, (by_contains, by_dispatch, by_resolve_all)) in
            selections.into_iter().enumerate()
        {
            let shape = shapes.node_shapes[position].id.to_string();
            let end = if shape.contains("SubjectsShape") {
                "tail"
            } else {
                "head"
            };
            let only = std::collections::BTreeSet::from([format!("<{TARGET_NS}{end}>")]);
            assert_eq!(by_resolve_all, only, "{shape}: wrong end of the quad");
            assert_eq!(by_contains, only, "{shape}: wrong end of the quad");
            assert_eq!(by_dispatch, only, "{shape}: wrong end of the quad");
        }
    }

    #[test]
    fn shared_shape_analysis_rebinds_class_ids_and_sparql_targets_per_dataset() {
        use rayon::prelude::*;

        let shapes = Arc::new(load_shapes_ttl(&format!(
            r#"{PREFIXES}
            ex:ClassShape a sh:NodeShape ; sh:targetClass ex:Person ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            ex:QueryShape a sh:NodeShape ;
                sh:target [ a sh:SPARQLTarget ; sh:select
                    "SELECT ?this WHERE {{ ?this <http://example.org/ns#active> true }}" ] ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            "#
        )));
        let prepared = PreparedShapes::new(Arc::clone(&shapes));
        let cases = [
            (
                "ex:alice a ex:Child ; ex:active true . ex:Child rdfs:subClassOf ex:Person .",
                2,
            ),
            ("ex:bob a ex:Other ; ex:active false .", 0),
            (
                "ex:carol a ex:Person ; ex:active true ; ex:required ex:present .",
                0,
            ),
            ("ex:dave a ex:Person ; ex:active false .", 1),
        ];
        cases.par_iter().for_each(|(source, expected)| {
            let data =
                crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES} {source}"), None)
                    .unwrap();
            let data = project_dataset(&data).unwrap();
            let binding = prepared.bind_projected_dataset(Arc::clone(&data)).unwrap();
            assert!(Arc::ptr_eq(&prepared.classes, &binding.bound.classes));
            assert!(Arc::ptr_eq(&data, &binding.data.core_arc()));
            let report = binding.validate().unwrap();
            assert_eq!(report.results.len(), *expected, "{source}");
            assert_eq!(
                report.to_ntriples(),
                validate_projected_dataset(data, &shapes)
                    .unwrap()
                    .to_ntriples(),
                "independent binding must equal full validation: {source}",
            );
        });
    }

    /// Validate native: the in-crate tests historically called `validate(&store, …)`;
    /// route them through the IR entrypoint, which is the only engine path now.
    fn validate(data: &Arc<RdfDataset>, shapes: &Shapes) -> ValidationReport {
        validate_dataset(data.as_ref(), shapes).expect("validate_dataset must succeed")
    }

    // ── Multi-error syntax reporting ──────────────────────────────────────

    #[test]
    fn parse_shapes_reports_all_syntax_errors() {
        // Two independently-malformed Turtle STATEMENTS, separated by a valid one.
        // oxttl recovers at statement granularity (resync on the `.` terminator),
        // so BOTH errors must surface in one report — proving the accumulator is
        // real, not a one-element surface. (A lexer-level break such as an
        // unterminated string literal instead consumes to EOF and yields a single
        // error; that is correct, not a regression. The recoverable case below is
        // what proves multi-error reporting works.) A regression to a single
        // error on recoverable input would violate the public diagnostic contract.
        let bad = concat!(
            "@prefix ex: <http://example.org/ns#> .\n",
            "ex:a ex:p .\n",                // missing object → recoverable error
            "ex:b ex:q ex:c .\n",           // valid, between the two errors
            "ex:d ex:r ex:s ex:t ex:u .\n", // too many terms → recoverable error
        );
        let err = parse_shapes(bad, None).expect_err("malformed Turtle must error");
        let n = err.matches("Turtle parse error").count();
        assert!(
            n >= 2,
            "expected >=2 accumulated Turtle errors, got {n}:\n{err}"
        );
    }

    #[test]
    fn validate_graphs_reports_all_data_syntax_errors() {
        // Multiple malformed N-Triples lines must all be reported in one pass
        // rather than short-circuiting on the first.
        let bad_data = concat!(
            "this is not a triple\n",
            "<http://example.org/s> <http://example.org/p> .\n",
            "neither is this\n",
        );
        let err = validate_graphs(bad_data, "", None).expect_err("malformed N-Triples must error");
        let n = err.matches("N-Triples parse error").count();
        assert!(
            n >= 2,
            "expected >=2 accumulated N-Triples errors, got {n}:\n{err}"
        );
    }

    #[test]
    fn parse_shapes_clean_input_still_succeeds() {
        // The accumulator must not turn a well-formed document into a failure.
        let ok = format!("{PREFIXES}\nex:Shape a sh:NodeShape ; sh:targetClass ex:Thing .\n");
        parse_shapes(&ok, None).expect("well-formed shapes must parse");
    }

    // ── The shapes graph's base ────────────────────────────────────────────────

    /// A shapes graph whose node and property shapes are RELATIVE IRI references.
    const RELATIVE_SHAPES: &str = concat!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
        "<PersonShape> a sh:NodeShape ;\n",
        "  sh:targetClass <http://example.org/Person> ;\n",
        "  sh:property <NamePropertyShape> .\n",
        "<NamePropertyShape> sh:path <http://example.org/name> ; sh:minCount 1 .\n",
    );

    #[test]
    fn parse_shapes_resolves_a_relative_iri_against_the_supplied_base() {
        let shapes = parse_shapes(RELATIVE_SHAPES, Some("http://example.org/shapes.ttl"))
            .expect("a based shapes graph parses");
        let shape = shapes
            .node_shapes
            .iter()
            .find(|s| s.id.to_string().contains("PersonShape"))
            .expect("the shape is present");
        // `Display` for a shape id renders the N-Triples term token, hence the brackets.
        assert_eq!(
            shape.id.to_string(),
            "<http://example.org/PersonShape>",
            "the shape node must be the RESOLVED absolute IRI, not the bare token"
        );
    }

    #[test]
    fn parse_shapes_without_a_base_refuses_a_relative_iri() {
        // The negative control. A shapes graph whose terms cannot be resolved must not
        // parse into shapes that quietly match nothing: a validator built from those
        // would report `conforms true` over a constraint it never evaluated.
        let err = parse_shapes(RELATIVE_SHAPES, None)
            .expect_err("a relative IRI with no base must be refused");
        assert!(
            err.contains("iri-relative-no-base"),
            "the refusal must name the actionable condition: {err}"
        );
    }

    #[test]
    fn an_in_document_base_overrides_the_supplied_base() {
        // RFC-3986 5.1.1 over 5.1.2: the document's own `@base` wins.
        let document = format!("@base <http://example.org/inner/> .\n{RELATIVE_SHAPES}");
        let shapes = parse_shapes(&document, Some("http://caller.example/outer.ttl"))
            .expect("the document base resolves it");
        assert!(
            shapes
                .node_shapes
                .iter()
                .any(|s| s.id.to_string() == "<http://example.org/inner/PersonShape>"),
            "the in-document `@base` must win over the caller-supplied base"
        );
    }

    #[test]
    fn the_shapes_base_is_read_and_changes_the_validation_outcome() {
        // The base is not decorative: resolving the shape under a base the DATA graph
        // shares is what makes the target select a focus node at all. Under a
        // different base the same shapes text finds the same violation, because the
        // target class here is absolute — so the discriminating assertion is on the
        // reported source shape, which moves with the base.
        let data = "<http://example.org/alice> \
                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                    <http://example.org/Person> .\n";

        let report = validate_graphs(data, RELATIVE_SHAPES, Some("http://a.example/s.ttl"))
            .expect("validation runs under a base");
        assert_eq!(report.results.len(), 1, "the shape applies");
        assert_eq!(
            report.results[0].source_shape.to_string(),
            "<http://a.example/NamePropertyShape>"
        );

        let other = validate_graphs(data, RELATIVE_SHAPES, Some("http://b.example/s.ttl"))
            .expect("validation runs under a different base");
        assert_eq!(other.results.len(), 1, "the same constraint still applies");
        assert_eq!(
            other.results[0].source_shape.to_string(),
            "<http://b.example/NamePropertyShape>",
            "a different base must produce a different resolved shape IRI — proving the \
             parameter is read rather than accepted and dropped"
        );
    }

    // ── Pre-existing tests ─────────────────────────────────────────────────────

    #[test]
    fn empty_inputs_return_conforming_report() {
        let report = validate_graphs("", "", None).expect("empty inputs must not error");
        assert!(report.conforms, "empty report must conform");
        assert!(
            report.results.is_empty(),
            "empty report must have no results"
        );
    }

    #[test]
    fn dataset_entrypoint_validates_gts_backed_graph() {
        use ::purrdf::RdfDatasetBuilder;
        let mut builder = RdfDatasetBuilder::new();
        let ids: Vec<_> = [
            "http://example.org/ns#a",
            "http://example.org/ns#p",
            "http://example.org/ns#b",
        ]
        .into_iter()
        .map(|value| builder.intern_iri(value))
        .collect();
        builder.push_quad(ids[0], ids[1], ids[2], None);
        let dataset = builder.freeze().expect("valid test dataset");

        let shapes_ttl = format!(
            "{PREFIXES}
            ex:Shape a sh:NodeShape ;
                sh:targetNode ex:a ;
                sh:property [
                    sh:path ex:missing ;
                    sh:minCount 1 ;
                ] ."
        );
        let report = validate_dataset_graphs(dataset.as_ref(), &shapes_ttl, None)
            .expect("GTS-backed store should validate");
        assert!(!report.conforms, "missing property must violate the shape");
        assert_eq!(report.results.len(), 1);
    }

    #[test]
    fn validate_stub_always_conforms() {
        let data = load_data_nt("");
        let shapes = Shapes::default();
        let report = validate(&data, &shapes);
        assert!(report.conforms);
        assert!(report.results.is_empty());
    }

    // ── Core constraint components over a targeted focus node ──────────────────

    // Test 1: targetClass + minCount — violating case (no ex:name on ex:alice)
    #[test]
    fn target_class_min_count_violating() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice is a Person but has no ex:name
        let data_nt = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(!report.conforms, "must NOT conform: alice has no ex:name");
        assert_eq!(report.results.len(), 1, "exactly one result expected");
        let r = &report.results[0];
        assert!(
            r.source_constraint_component.as_str().contains("MinCount"),
            "component must be MinCountConstraintComponent, got {}",
            r.source_constraint_component.as_str()
        );
        assert_eq!(
            r.focus_node.to_string(),
            "<http://example.org/ns#alice>",
            "focus node must be ex:alice"
        );
    }

    // Test 2: conforming case — adding ex:name makes it pass
    #[test]
    fn target_class_min_count_conforming() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        let data_nt = concat!(
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://example.org/ns#name> \"Alice\" .\n"
        );

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(report.conforms, "must conform: alice now has ex:name");
        assert!(report.results.is_empty(), "zero results expected");
    }

    // Test 3a: targetSubjectsOf — shape targets subjects of ex:knows
    #[test]
    fn target_subjects_of() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:KnowerShape a sh:NodeShape ;
                sh:targetSubjectsOf ex:knows ;
                sh:property [
                    sh:path ex:label ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice knows ex:bob, but alice has no ex:label
        let data_nt = "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            !report.conforms,
            "alice (subject of knows) must be a focus node and fail"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#alice>"
        );
    }

    // Test 3b: targetObjectsOf — shape targets objects of ex:knows
    #[test]
    fn target_objects_of() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:KnownShape a sh:NodeShape ;
                sh:targetObjectsOf ex:knows ;
                sh:property [
                    sh:path ex:label ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice knows ex:bob, bob has no ex:label
        let data_nt = "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            !report.conforms,
            "bob (object of knows) must be a focus node and fail"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#bob>"
        );
    }

    // sh:targetClass honors ASSERTED rdfs:subClassOf (SHACL §4.2.5).
    // This is NOT OWL inference — the subclass edge is asserted in the data; we
    // read it and materialize nothing.
    #[test]
    fn target_class_honors_asserted_subclass() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:bob is typed ex:Employee, and ex:Employee rdfs:subClassOf ex:Person
        // is ASSERTED → bob is a SHACL instance of ex:Person → it is a focus node
        // and, lacking ex:name, violates sh:minCount.
        let data_nt = concat!(
            "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n",
            "<http://example.org/ns#Employee> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
        );

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            !report.conforms,
            "ex:bob IS a focus node via asserted Employee ⊑ Person; report: {report:?}"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#bob>"
        );
    }

    // Test 4b: a class with NO asserted subClassOf edge is not reached — we
    // honor asserted edges only, inventing none.
    #[test]
    fn target_class_unasserted_subclass_not_reached() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [ sh:path ex:name ; sh:minCount 1 ; ] .
            "
        );
        // ex:carol is an ex:Robot; no ex:Robot rdfs:subClassOf ex:Person triple
        // exists → carol is not a Person-instance → conforms.
        let data_nt = "<http://example.org/ns#carol> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Robot> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            report.conforms,
            "carol must NOT be reached without an asserted subClassOf edge; report: {report:?}"
        );
    }

    // Test 5: deactivated shape produces no results even with violating data
    #[test]
    fn deactivated_shape_produces_no_results() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:deactivated true ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // alice is a Person with no ex:name — would fail if shape were active
        let data_nt = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(
            report.conforms,
            "deactivated shape must produce no results; report: {report:?}"
        );
        assert!(report.results.is_empty());
    }

    // Test 6: determinism — two runs on the same input yield identical results
    #[test]
    fn determinism_same_results_twice() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // Two persons, both missing ex:name, to get multiple results
        let data_nt = concat!(
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n",
        );

        let data1 = load_data_nt(data_nt);
        let shapes1 = load_shapes_ttl(&shapes_ttl);
        let report1 = validate(&data1, &shapes1);

        let data2 = load_data_nt(data_nt);
        let shapes2 = load_shapes_ttl(&shapes_ttl);
        let report2 = validate(&data2, &shapes2);

        assert_eq!(report1.conforms, report2.conforms);
        assert_eq!(report1.results.len(), report2.results.len());

        // Compare result tuples in order (not just as a set) to confirm stable sort.
        let tuples1: Vec<_> = report1
            .results
            .iter()
            .map(|r| {
                (
                    r.focus_node.to_string(),
                    r.source_constraint_component.to_string(),
                    r.source_shape.to_string(),
                    r.result_path.as_ref().map(ToString::to_string),
                    r.value.as_ref().map(ToString::to_string),
                    r.severity.clone(),
                )
            })
            .collect();
        let tuples2: Vec<_> = report2
            .results
            .iter()
            .map(|r| {
                (
                    r.focus_node.to_string(),
                    r.source_constraint_component.to_string(),
                    r.source_shape.to_string(),
                    r.result_path.as_ref().map(ToString::to_string),
                    r.value.as_ref().map(ToString::to_string),
                    r.severity.clone(),
                )
            })
            .collect();

        assert_eq!(
            tuples1, tuples2,
            "result ordering must be identical across runs"
        );

        // Also verify to_ntriples() is identical
        assert_eq!(
            report1.to_ntriples(),
            report2.to_ntriples(),
            "N-Triples output must be identical across runs"
        );
    }

    // Bonus: targetNode explicit
    #[test]
    fn target_node_explicit() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:AliceShape a sh:NodeShape ;
                sh:targetNode ex:alice ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                ] .
            "
        );
        // ex:alice explicitly targeted; no ex:name triple
        let data_nt = "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        assert!(!report.conforms, "ex:alice has no ex:name → must fail");
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node.to_string(),
            "<http://example.org/ns#alice>"
        );
    }

    // Severity-independence: a Warning result makes conforms=false
    #[test]
    fn warning_result_makes_report_non_conforming() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:WarnShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:severity sh:Warning ;
                sh:property [
                    sh:path ex:label ;
                    sh:minCount 1 ;
                    sh:severity sh:Warning ;
                ] .
            "
        );
        let data_nt = "<http://example.org/ns#x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Thing> .\n";

        let data = load_data_nt(data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);

        // SHACL: conforms is false if ANY result exists, regardless of severity
        assert!(
            !report.conforms,
            "Warning results must still make conforms=false"
        );
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].severity, Severity::Warning);
    }

    // Determinism under permuted DATA-INSERTION order.
    //
    // Interning assigns each term a TermId in first-appearance order, so
    // permuting the N-Triples lines assigns different ids to the same terms and
    // reorders every id-keyed FastSet/FastMap and path frontier. The report must
    // still be byte-identical, proving no id/hash iteration order reaches output.
    // The fixture deliberately includes TWO sh:uniqueLang violations on one focus
    // (duplicate `@en` and `@fr`), which share every sort component except the
    // message — the case the message/severity tiebreaker in the engine sort
    // exists to make total.
    #[test]
    fn determinism_under_permuted_insertion_order() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:uniqueLang true ; ] .
            "
        );
        let ty = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
        let name = "<http://example.org/ns#name>";
        let person = "<http://example.org/ns#Person>";
        // alice: two duplicated languages (@en, @fr) → two uniqueLang violations
        // with identical sort keys differing only in message. bob/carol: plain
        // members exercising focus ordering. dave: no name → minCount violation.
        let lines = vec![
            format!("<http://example.org/ns#alice> {ty} {person} .\n"),
            format!("<http://example.org/ns#alice> {name} \"a\"@en .\n"),
            format!("<http://example.org/ns#alice> {name} \"b\"@en .\n"),
            format!("<http://example.org/ns#alice> {name} \"c\"@fr .\n"),
            format!("<http://example.org/ns#alice> {name} \"d\"@fr .\n"),
            format!("<http://example.org/ns#bob> {ty} {person} .\n"),
            format!("<http://example.org/ns#bob> {name} \"Bob\" .\n"),
            format!("<http://example.org/ns#carol> {ty} {person} .\n"),
            format!("<http://example.org/ns#carol> {name} \"Carol\" .\n"),
            format!("<http://example.org/ns#dave> {ty} {person} .\n"),
        ];

        let render = |ordered: &[String]| {
            let data_nt: String = ordered.concat();
            let data = load_data_nt(&data_nt);
            let shapes = load_shapes_ttl(&shapes_ttl);
            validate(&data, &shapes).to_ntriples()
        };

        let forward = render(&lines);

        let mut reversed = lines.clone();
        reversed.reverse();
        assert_eq!(
            forward,
            render(&reversed),
            "report must be byte-identical under reversed insertion order"
        );

        // A rotation (different id assignment again) must also match.
        let mut rotated = lines.clone();
        rotated.rotate_left(3);
        assert_eq!(
            forward,
            render(&rotated),
            "report must be byte-identical under rotated insertion order"
        );

        // Sanity: the fixture actually produced the equal-sort-key uniqueLang pair
        // plus other violations, so the tiebreaker is genuinely exercised.
        let data = load_data_nt(&lines.concat());
        let shapes = load_shapes_ttl(&shapes_ttl);
        let report = validate(&data, &shapes);
        let unique_lang = report
            .results
            .iter()
            .filter(|r| {
                r.source_constraint_component
                    .as_str()
                    .contains("UniqueLang")
            })
            .count();
        assert_eq!(
            unique_lang, 2,
            "fixture must yield two uniqueLang violations (dup @en and @fr)"
        );
    }

    #[test]
    fn validation_plan_resolves_target_property_and_nested_classes() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PlannedShape a sh:NodeShape ;
                sh:targetClass ex:Root ;
                sh:property [
                    sh:path ex:member ;
                    sh:class ex:Value ;
                    sh:node [ sh:class ex:Nested ] ;
                ] .
            "
        );
        let data = load_data_nt(
            "<http://example.org/ns#Child> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Root> .\n\
             <http://example.org/ns#Leaf> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Value> .\n",
        );
        let shapes = load_shapes_ttl(&shapes_ttl);
        let lowered = crate::plan::lower_shapes(shapes.node_shapes.iter());
        let classes = lowered.classes();
        let binding = lowered.bind(data.as_ref(), classes);

        assert_eq!(classes.len(), 3);
        assert!(
            binding
                .class_id(classes, &NamedNode::from("http://example.org/ns#Root"))
                .expect("ex:Root is planned")
                .is_some()
        );
        assert!(
            binding
                .class_id(classes, &NamedNode::from("http://example.org/ns#Value"))
                .expect("ex:Value is planned")
                .is_some()
        );
        // Planned but absent from the DATA graph: a soft `None`, never an error.
        assert!(
            binding
                .class_id(classes, &NamedNode::from("http://example.org/ns#Nested"))
                .expect("ex:Nested is planned even though the data never names it")
                .is_none()
        );
    }

    /// The two "no id" answers are different conditions and must stay apart.
    ///
    /// A class the walk never collected is a defect in the PLAN, and it is
    /// reported as an error — it used to be an `expect`, and a panic aborts
    /// across the PyO3 and C ABI boundaries. A class the walk DID collect but the
    /// data graph never names is ordinary and stays a soft `None`, because a
    /// shapes graph is entitled to name a class its data lacks.
    #[test]
    fn an_unplanned_class_errors_while_a_planned_one_absent_from_the_data_stays_none() {
        let shapes_ttl = format!(
            r"{PREFIXES}
            ex:PlannedShape a sh:NodeShape ;
                sh:targetClass ex:Root ;
                sh:property [ sh:path ex:member ; sh:class ex:Ghost ] .
            "
        );
        let data = load_data_nt(
            "<http://example.org/ns#a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Root> .\n",
        );
        let shapes = load_shapes_ttl(&shapes_ttl);
        let lowered = crate::plan::lower_shapes(shapes.node_shapes.iter());
        let classes = lowered.classes();
        let binding = lowered.bind(data.as_ref(), classes);

        // Planned, but the data graph interns no such class term.
        assert_eq!(
            binding.class_id(classes, &NamedNode::from("http://example.org/ns#Ghost")),
            Ok(None),
            "a planned class the data never names is a soft None, not a refusal"
        );

        // Never collected by the walk at all.
        let error = binding
            .class_id(
                classes,
                &NamedNode::from("http://example.org/ns#NotInTheCatalog"),
            )
            .expect_err("a class outside the catalog is a plan defect");
        assert!(
            error.contains("NotInTheCatalog") && error.contains("class-planning walk"),
            "the error must name the class and the walk that missed it, got: {error}"
        );
    }

    fn membership_reuse_fixture() -> (Arc<RdfDataset>, Arc<Shapes>) {
        let data = load_data_nt(concat!(
            "<http://example.org/ns#Employee> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n",
        ));
        let shapes = Arc::new(load_shapes_ttl(&format!(
            r#"{PREFIXES}
            ex:MembershipShape a sh:NodeShape;
                sh:targetClass ex:Person;
                sh:target [ a sh:SPARQLTarget;
                    sh:select "SELECT ?this WHERE {{ ?this a <http://example.org/ns#Person> }}" ];
                sh:property [ sh:path ex:required; sh:minCount 1 ] .
            "#,
        )));
        (data, shapes)
    }

    #[test]
    fn projected_preparation_without_shapes_graph_builds_one_shared_class_index() {
        use crate::class_membership::{reset_thread_index_builds, thread_index_builds};

        let (data, shapes) = membership_reuse_fixture();
        let mut previous = None;
        for reuse_shapes in [false, true] {
            reset_thread_index_builds();
            let prepared = if reuse_shapes {
                PreparedShapes::new(Arc::clone(&shapes))
                    .bind_projected_dataset_with_shapes_graph(Arc::clone(&data), None)
            } else {
                PreparedValidator::from_projected_dataset_with_shapes_graph(
                    Arc::clone(&data),
                    Arc::clone(&shapes),
                    None,
                )
            }
            .expect("prepare");
            assert_eq!(
                thread_index_builds(),
                1,
                "Core and SPARQL share one initialization"
            );
            assert_eq!(prepared.data.class_view().build_count(), 1);
            assert_eq!(prepared.data.sparql_view().build_count(), 1);
            let report = prepared.validate().expect("validate");
            assert_eq!(
                report.results.len(),
                1,
                "both targets resolve the same derived instance"
            );
            let serialized = report.to_ntriples();
            if let Some(expected) = &previous {
                assert_eq!(&serialized, expected);
            }
            previous = Some(serialized);
            assert_eq!(
                thread_index_builds(),
                1,
                "validation reuses the prepared analysis"
            );
            assert!(
                prepared
                    .view_stats()
                    .iter()
                    .all(|stats| stats.materializations == 0)
            );
        }
    }

    #[test]
    fn projected_and_governed_validation_without_shapes_graph_reuse_class_analysis() {
        use crate::class_membership::{reset_thread_index_builds, thread_index_builds};

        let (data, shapes) = membership_reuse_fixture();
        reset_thread_index_builds();
        let ordinary =
            validate_projected_dataset_with_shapes_graph(Arc::clone(&data), &shapes, None)
                .expect("ordinary validation");
        assert_eq!(ordinary.results.len(), 1);
        assert_eq!(thread_index_builds(), 1);
        reset_thread_index_builds();
        let outcome =
            validate_dataset_with_governors(&data, &shapes, None, &QueryGovernors::UNBOUNDED)
                .expect("governed validation");
        let GovernedValidation::Complete { report, .. } = outcome else {
            panic!("unbounded validation must complete");
        };
        assert_eq!(report.to_ntriples(), ordinary.to_ntriples());
        assert_eq!(thread_index_builds(), 1);
    }

    #[test]
    fn an_exposed_shapes_graph_keeps_its_own_class_index_and_term_ids() {
        use crate::class_membership::{reset_thread_index_builds, thread_index_builds};

        let (data, shapes) = membership_reuse_fixture();
        reset_thread_index_builds();
        let prepared = PreparedValidator::from_projected_dataset_with_shapes_graph(
            data,
            shapes,
            Some("https://example.org/shapes"),
        )
        .expect("prepare composite");
        assert_eq!(
            thread_index_builds(),
            2,
            "distinct carriers require their own local IDs"
        );
        assert_eq!(
            prepared.data.shapes_graph_iri(),
            Some("https://example.org/shapes")
        );
        assert_eq!(prepared.validate().expect("validate").results.len(), 1);
        assert_eq!(thread_index_builds(), 2);
        assert!(
            prepared
                .view_stats()
                .iter()
                .all(|stats| stats.materializations == 0)
        );
    }

    #[test]
    fn prepared_bounded_validation_matches_filtered_and_whole_reports() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PreparedValidator>();

        let data = load_data_nt(concat!(
            "<http://example.org/ns#Employee> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n",
            "<http://example.org/ns#alice> <http://example.org/ns#knows> <http://example.org/ns#bob> .\n",
            "<http://example.org/ns#bob> <http://example.org/ns#knows> <http://example.org/ns#alice> .\n",
        ));
        let projected = project_dataset(data.as_ref()).expect("projection must succeed");
        let shapes = Arc::new(load_shapes_ttl(&format!(
            r#"{PREFIXES}
			ex:ClassShape a sh:NodeShape ;
				sh:targetClass ex:Person ;
				sh:property [ sh:path ex:required ; sh:minCount 1 ] .
			ex:SubjectsShape a sh:NodeShape ;
				sh:targetSubjectsOf ex:knows ;
				sh:property [ sh:path ex:required ; sh:minCount 1 ] .
			ex:ObjectsShape a sh:NodeShape ;
				sh:targetObjectsOf ex:knows ;
				sh:property [ sh:path ex:required ; sh:minCount 1 ] .
			ex:NodeShape a sh:NodeShape ;
				sh:targetNode ex:alice ;
				sh:property [ sh:path ex:required ; sh:minCount 1 ] .
			ex:SparqlTargetShape a sh:NodeShape ;
				sh:target [
					a sh:SPARQLTarget ;
					sh:select "SELECT ?this WHERE {{ ?this a <http://example.org/ns#Person> }}" ;
				] ;
				sh:property [ sh:path ex:required ; sh:minCount 1 ] .
			"#,
        )));
        let alice = NamedNode::new_unchecked("http://example.org/ns#alice").into_term();
        let carol = NamedNode::new_unchecked("http://example.org/ns#carol").into_term();

        let expected_bounded = validate_projected_dataset_with_focus_filter(
            Arc::clone(&projected),
            &shapes,
            |_, focus| focus == &alice,
        )
        .expect("filtered validation must succeed");
        let expected_whole = validate_projected_dataset(Arc::clone(&projected), &shapes)
            .expect("whole validation must succeed");
        let prepared =
            PreparedValidator::from_projected_dataset(Arc::clone(&projected), Arc::clone(&shapes))
                .expect("preparation must succeed");
        assert_eq!(prepared.data.class_view().build_count(), 1);
        assert_eq!(prepared.data.sparql_view().build_count(), 1);

        let bounded = prepared
            .validate_focus_nodes(&[alice.clone(), alice])
            .expect("bounded validation must succeed");
        assert_eq!(bounded.results.len(), 5, "alice matches every target kind");
        assert_eq!(bounded.to_ntriples(), expected_bounded.to_ntriples());
        assert_eq!(
            prepared
                .validate()
                .expect("prepared whole validation")
                .to_ntriples(),
            expected_whole.to_ntriples()
        );

        let alice_id = prepared
            .term_id(&NamedNode::new_unchecked("http://example.org/ns#alice").into_term())
            .expect("alice must be interned");
        assert_eq!(
            prepared
                .validate_focus_node_ids(&[alice_id, alice_id])
                .expect("id-native bounded validation")
                .to_ntriples(),
            expected_bounded.to_ntriples()
        );
        assert!(
            prepared
                .validate_focus_nodes(&[carol])
                .expect("untargeted bounded validation")
                .conforms
        );

        // Minted with the RIGHT binding and an id past the end of its table, which
        // is the only way to build one: the public surface cannot produce it. The
        // range check behind it is therefore an in-crate backstop, and this is
        // what keeps it honest.
        let invalid = FocusId::new(
            prepared.data.identity(),
            TermId::from_index(u32::try_from(projected.term_count()).expect("small test term")),
        );
        assert!(
            prepared.validate_focus_node_ids(&[invalid]).is_err(),
            "out-of-range ids must fail before dataset lookup"
        );
        assert_eq!(
            prepared.data.class_view().build_count(),
            1,
            "repeated realtime calls must reuse the prepared index"
        );
    }

    #[test]
    fn prepared_bounded_validation_keeps_foreign_explicit_targets() {
        let data = load_data_nt("");
        let projected = project_dataset(data.as_ref()).expect("projection must succeed");
        let shapes = Arc::new(load_shapes_ttl(&format!(
            "{PREFIXES}\n\
			 ex:ForeignShape a sh:NodeShape ;\n\
			     sh:targetNode ex:absent ;\n\
			     sh:property [ sh:path ex:required ; sh:minCount 1 ] ."
        )));
        let absent = NamedNode::new_unchecked("http://example.org/ns#absent").into_term();
        let prepared = PreparedValidator::from_projected_dataset(projected, shapes)
            .expect("preparation must succeed");
        let report = prepared
            .validate_focus_nodes(&[absent])
            .expect("foreign explicit target must validate");
        assert_eq!(report.results.len(), 1);
        assert!(!report.conforms);
    }

    #[test]
    fn parameterized_sparql_target_uses_subclass_membership_across_entry_points() {
        let data_nt = concat!(
            "<http://example.org/ns#Leaf> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Mid> .\n",
            "<http://example.org/ns#Mid> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Leaf> .\n",
        );
        let shapes_ttl = format!(
            "{PREFIXES}\n{}",
            r#"
            ex:ByClass a sh:SPARQLTargetType ;
                sh:parameter [ sh:path ex:class ; sh:order 0 ] ;
                sh:select "SELECT ?this WHERE { ?this a ?class }" .

            ex:TargetTypeShape a sh:NodeShape ;
                sh:target [ a ex:ByClass ; ex:class ex:Person ] ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            "#,
        );

        let text_report = validate_graphs(data_nt, &shapes_ttl, None).expect("text validation");
        assert_eq!(text_report.results.len(), 1);
        assert_eq!(
            text_report.results[0].focus_node,
            NamedNode::new_unchecked("http://example.org/ns#alice").into_term()
        );

        let source = load_data_nt(data_nt);
        let projected = project_dataset(source.as_ref()).expect("projection");
        let shapes = Arc::new(load_shapes_ttl(&shapes_ttl));
        let projected_report =
            validate_projected_dataset(Arc::clone(&projected), &shapes).expect("projected");
        let prepared = PreparedValidator::from_projected_dataset(projected, Arc::clone(&shapes))
            .expect("prepared validator");
        let first = prepared.validate().expect("first prepared validation");
        let second = prepared.validate().expect("second prepared validation");
        assert_eq!(text_report.to_ntriples(), projected_report.to_ntriples());
        assert_eq!(text_report.to_ntriples(), first.to_ntriples());
        assert_eq!(first.to_ntriples(), second.to_ntriples());

        let reversed = data_nt.lines().rev().collect::<Vec<_>>().join("\n");
        let reversed_report =
            validate_graphs(&reversed, &shapes_ttl, None).expect("permuted text validation");
        assert_eq!(text_report.to_ntriples(), reversed_report.to_ntriples());
    }

    #[test]
    fn ask_and_select_components_query_the_shared_membership_relation() {
        let data_nt = concat!(
            "<http://example.org/ns#Leaf> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Leaf> .\n",
            "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Other> .\n",
        );
        let shapes_ttl = format!(
            "{PREFIXES}\n{}",
            r#"
            ex:AskClassComponent a sh:ConstraintComponent ;
                sh:parameter [ sh:path ex:requiredClass ] ;
                sh:validator [
                    a sh:SPARQLAskValidator ;
                    sh:ask "ASK { $this a $requiredClass }" ;
                ] .

            ex:SelectClassComponent a sh:ConstraintComponent ;
                sh:parameter [ sh:path ex:selectedClass ] ;
                sh:nodeValidator [
                    a sh:SPARQLSelectValidator ;
                    sh:select "SELECT $this WHERE { FILTER NOT EXISTS { $this a $selectedClass } }" ;
                ] .

            ex:AskShape a sh:NodeShape ;
                sh:targetNode ex:alice, ex:bob ;
                ex:requiredClass ex:Person .

            ex:SelectShape a sh:NodeShape ;
                sh:targetNode ex:alice, ex:bob ;
                ex:selectedClass ex:Person .
            "#,
        );

        let report = validate_graphs(data_nt, &shapes_ttl, None).expect("component validation");
        assert_eq!(report.results.len(), 2, "only bob fails both validators");
        assert!(report.results.iter().all(|result| {
            result.focus_node == NamedNode::new_unchecked("http://example.org/ns#bob").into_term()
        }));
        let components: FastSet<_> = report
            .results
            .iter()
            .map(|result| result.source_constraint_component.as_str())
            .collect();
        assert_eq!(components.len(), 2);
        assert!(components.contains("http://example.org/ns#AskClassComponent"));
        assert!(components.contains("http://example.org/ns#SelectClassComponent"));
    }

    #[test]
    fn declared_function_body_queries_derived_membership() {
        let data_nt = concat!(
            "<http://example.org/ns#Leaf> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Leaf> .\n",
            "<http://example.org/ns#bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Other> .\n",
        );
        let shapes_ttl = format!(
            "{PREFIXES}\n{}",
            r#"
            ex:isPerson a sh:SPARQLFunction ;
                sh:parameter [ sh:path ex:node ; sh:nodeKind sh:IRI ] ;
                sh:returnType xsd:boolean ;
                sh:ask "ASK { ?node a <http://example.org/ns#Person> }" .

            ex:FunctionShape a sh:NodeShape ;
                sh:targetNode ex:alice, ex:bob ;
                sh:expression [ ex:isPerson ( sh:this ) ] .
            "#,
        );

        let report = validate_graphs(data_nt, &shapes_ttl, None).expect("function validation");
        assert_eq!(report.results.len(), 1);
        assert_eq!(
            report.results[0].focus_node,
            NamedNode::new_unchecked("http://example.org/ns#bob").into_term()
        );
    }

    #[test]
    fn native_rdf_type_path_remains_asserted_only() {
        let data_nt = concat!(
            "<http://example.org/ns#Leaf> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
            "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Leaf> .\n",
        );
        let shapes_ttl = format!(
            "{PREFIXES}\n{}",
            r"
            ex:AssertedPathShape a sh:NodeShape ;
                sh:targetNode ex:alice ;
                sh:property [ sh:path rdf:type ; sh:hasValue ex:Person ] .
            ",
        );
        let report = validate_graphs(data_nt, &shapes_ttl, None).expect("path validation");
        assert_eq!(report.results.len(), 1);
        assert!(
            report.results[0]
                .source_constraint_component
                .as_str()
                .ends_with("HasValueConstraintComponent")
        );
    }

    #[test]
    fn shapes_named_graph_cannot_create_default_graph_membership() {
        let shapes_ttl = format!(
            "{PREFIXES}\n{}",
            r#"
            ex:Ghost a ex:Leaf .
            ex:Leaf rdfs:subClassOf ex:Person .

            ex:IsolatedShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:target [
                    a sh:SPARQLTarget ;
                    sh:select "SELECT ?this WHERE { ?this a <http://example.org/ns#Person> }" ;
                ] ;
                sh:property [ sh:path ex:required ; sh:minCount 1 ] .
            "#,
        );
        let shapes = load_shapes_ttl(&shapes_ttl);
        let projected = project_dataset(load_data_nt("").as_ref()).expect("empty projection");
        let report = validate_projected_dataset_with_shapes_graph(
            projected,
            &shapes,
            Some("http://example.org/shapes"),
        )
        .expect("isolated validation");
        assert!(
            report.conforms,
            "named shapes rows must not target ex:Ghost"
        );
    }

    #[test]
    fn forced_parallel_matches_serial_with_user_functions() {
        use std::fmt::Write as _;

        let mut data_nt = String::new();
        data_nt.push_str(
            "<http://example.org/ns#Employee> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/ns#Person> .\n",
        );
        for index in 0..24 {
            write!(
                data_nt,
                "<http://example.org/ns#item{index}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Employee> .\n\
                 <http://example.org/ns#item{index}> <http://example.org/ns#amount> \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
            )
            .expect("writing to a String cannot fail");
        }
        let shapes_ttl = format!(
            r#"{PREFIXES}
            ex:tripled a sh:SPARQLFunction ;
                sh:parameter [ sh:path ex:x ; sh:datatype xsd:integer ] ;
                sh:returnType xsd:integer ;
                sh:select "SELECT ((?x * 3) AS ?result) WHERE {{}}" .

            ex:ParallelShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:sparql [
                    sh:select "SELECT $this WHERE {{ $this a <http://example.org/ns#Person> ; <http://example.org/ns#amount> ?amount . FILTER(<http://example.org/ns#tripled>(?amount) > 2) }}" ;
                ] .
            "#
        );
        let data = load_data_nt(&data_nt);
        let shapes = load_shapes_ttl(&shapes_ttl);
        let render = |threads, parallel, chunk_size| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("test pool must build")
                .install(|| {
                    let _guard = crate::parallel::force_scheduler_for_test(parallel, chunk_size);
                    validate(&data, &shapes).to_ntriples()
                })
        };

        let expected = render(1, false, 1);
        for (threads, chunk_size) in [(2, 1), (4, 3), (4, 16)] {
            assert_eq!(
                render(threads, true, chunk_size),
                expected,
                "user-function report drifted with {threads} workers and chunk size {chunk_size}"
            );
        }
    }

    // ── custom aggregates: installability, correctness, and fork survival ──────

    /// The `AGG(<iri>, …)` IRI the custom-aggregate fixture below registers under.
    const AGG_IRI: &str = "http://example.org/ns#sum";

    /// A `Commutative` custom `SUM`-alike over a single numeric-lexical argument:
    /// folds the integer lexical forms it is handed. The SHACL-facing twin of
    /// `purrdf_sparql_eval::agg_fn::tests::SumAccumulator` (private to that crate,
    /// so it cannot be reused directly here).
    #[derive(Debug)]
    struct AggSumAccumulator {
        total: i64,
    }

    impl purrdf_sparql_eval::AggregateAccumulator for AggSumAccumulator {
        fn step(
            &mut self,
            args: &[::purrdf::TermValue],
        ) -> Result<(), purrdf_sparql_eval::EvalError> {
            if let Some(::purrdf::TermValue::Literal { lexical_form, .. }) = args.first()
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        fn combine(
            &mut self,
            other: Box<dyn purrdf_sparql_eval::AggregateAccumulator>,
        ) -> Result<(), purrdf_sparql_eval::EvalError> {
            if let Some(::purrdf::TermValue::Literal { lexical_form, .. }) = other.finish()?
                && let Ok(n) = lexical_form.parse::<i64>()
            {
                self.total += n;
            }
            Ok(())
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn finish(
            self: Box<Self>,
        ) -> Result<Option<::purrdf::TermValue>, purrdf_sparql_eval::EvalError> {
            Ok(Some(::purrdf::TermValue::typed_literal(
                self.total.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            )))
        }
    }

    #[derive(Debug)]
    struct AggSum;

    impl purrdf_sparql_eval::CustomAggregate for AggSum {
        fn arity(&self) -> purrdf_sparql_eval::Arity {
            purrdf_sparql_eval::Arity::Exact(1)
        }
        fn volatility(&self) -> purrdf_sparql_eval::Volatility {
            purrdf_sparql_eval::Volatility::Stable
        }
        fn algebraic_class(&self) -> purrdf_sparql_eval::AlgebraicClass {
            purrdf_sparql_eval::AlgebraicClass::Commutative
        }
        fn state_bound(&self) -> u64 {
            0
        }
        fn init(
            &self,
            _scalarvals: &[(String, ::purrdf::TermValue)],
        ) -> Box<dyn purrdf_sparql_eval::AggregateAccumulator> {
            Box::new(AggSumAccumulator { total: 0 })
        }
    }

    /// A fresh registry with [`AggSum`] registered under [`AGG_IRI`].
    fn sum_aggregate_registry() -> purrdf_sparql_eval::AggregateRegistry {
        let mut registry = purrdf_sparql_eval::AggregateRegistry::new();
        registry.register(AGG_IRI, Arc::new(AggSum));
        registry
    }

    /// Shapes carrying a `sh:sparql` constraint whose `HAVING` clause resolves an
    /// `AGG(<iri>, …)` call: `$this` is grouped alone (SHACL pre-binds `$this` to
    /// exactly one focus node per call, so `GROUP BY $this` is always a single
    /// group), and the group is a violation when its `ex:amount` values sum
    /// negative.
    fn aggregate_shapes_ttl() -> String {
        format!(
            r#"{PREFIXES}
            ex:AggShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:sparql ex:AggConstraint .
            ex:AggConstraint sh:select """
                SELECT $this WHERE {{ $this <http://example.org/ns#amount> ?v }}
                GROUP BY $this
                HAVING (AGG(<{AGG_IRI}>, ?v) < 0)
            """ .
            "#
        )
    }

    /// `n` `ex:Thing` focus nodes: `ex:v0` carries a NEGATIVE amount (its group's
    /// sum is negative, so it must violate), every other `ex:v{i}` carries a
    /// positive amount (must conform). Used to prove the aggregate actually ran —
    /// a seam that silently no-ops would report zero violations, indistinguishable
    /// from an intrinsically conforming graph, which is exactly the always-passing
    /// failure mode these tests exist to forbid.
    fn aggregate_data_nt(n: usize) -> String {
        use std::fmt::Write as _;
        assert!(n >= 1, "at least the violating node is required");
        let mut nt = String::new();
        nt.push_str(
            "<http://example.org/ns#v0> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/ns#Thing> .\n\
             <http://example.org/ns#v0> <http://example.org/ns#amount> \
             \"-1\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        );
        for i in 1..n {
            writeln!(
                nt,
                "<http://example.org/ns#v{i}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                 <http://example.org/ns#Thing> .\n\
                 <http://example.org/ns#v{i}> <http://example.org/ns#amount> \
                 \"1\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
            )
            .expect("writing to a String cannot fail");
        }
        nt
    }

    /// The IRI of the `i`th `ex:v` focus node minted by [`aggregate_data_nt`] and
    /// [`pf_data_nt`] (both follow the same `ex:v{i}` naming scheme).
    fn v_iri(i: usize) -> String {
        format!("http://example.org/ns#v{i}")
    }

    /// Every `ex:v{i}` focus-node [`Term`] for `i` in `0..n`, in the SAME order the
    /// `0..n` corpus generators (`aggregate_data_nt`/`pf_data_nt`) mint them. Used to
    /// hand the bounded [`PreparedValidator`] routes a focus-node slice whose length
    /// genuinely crosses `crate::parallel::PARALLEL_MIN_FOCUS_NODES` — a scheduling
    /// claim a single-node slice could never establish.
    fn all_v_terms(n: usize) -> Vec<Term> {
        (0..n)
            .map(|i| NamedNode::new_unchecked(v_iri(i)).into_term())
            .collect()
    }

    /// Resolve each `terms` entry (all named nodes, all `ex:v{i}` focus nodes) to its
    /// [`TermId`] in `projected`, panicking with `label` context on a lookup miss.
    fn term_ids_for(prepared: &PreparedValidator, terms: &[Term], label: &str) -> Vec<FocusId> {
        terms
            .iter()
            .map(|term| {
                let Term::NamedNode(named) = term else {
                    panic!("{label}: focus term must be a named node: {term:?}")
                };
                prepared
                    .term_id(term)
                    .unwrap_or_else(|| panic!("{label}: {named} must be interned"))
            })
            .collect()
    }

    /// A registered custom aggregate installed through the PUBLIC `Shapes::aggregates`
    /// field resolves inside a `sh:sparql` `HAVING` clause and produces the correct
    /// verdict — the capability the module's own docs promise, not the refusal every
    /// SHACL query got before `Shapes` carried a place to install one.
    ///
    /// Nothing here calls `crate::sparql::enter_aggregate_scope` directly: the whole
    /// point is that setting the public field and calling `validate_dataset` is
    /// enough, exactly as it is for `Shapes::functions`.
    #[test]
    fn custom_aggregate_installed_via_shapes_field_resolves_and_verdict_is_correct() {
        let mut shapes = load_shapes_ttl(&aggregate_shapes_ttl());
        shapes.aggregates = Arc::new(sum_aggregate_registry());
        let data = load_data_nt(&aggregate_data_nt(10));

        let report = validate_dataset(data.as_ref(), &shapes).expect("validation must resolve");
        assert_eq!(
            report.results.len(),
            1,
            "exactly the negative-sum focus node violates: {:?}",
            report.results
        );
        assert_eq!(
            report.results[0].focus_node,
            NamedNode::new_unchecked("http://example.org/ns#v0").into_term()
        );
        assert!(!report.conforms);
    }

    /// The scheduling test the missing fork re-install let ship broken: the SAME
    /// shapes and registry, run once BELOW [`crate::parallel::PARALLEL_MIN_FOCUS_NODES`]
    /// and once ABOVE it, must reach the IDENTICAL verdict for the shared violating
    /// focus node. Before the fork fix, crossing the threshold flipped `Ok` into an
    /// `Err("no custom aggregate is registered …")` because the focus-chunk workers
    /// never saw the registry the orchestrating thread had installed.
    ///
    /// Both runs go through a real, multi-threaded rayon pool (not the crate-internal
    /// `force_scheduler_for_test` hook) so the large run genuinely forks across
    /// workers via the production `PARALLEL_MIN_FOCUS_NODES` threshold — driven
    /// entirely from the public `validate_dataset`/`Shapes::aggregates` surface.
    #[test]
    fn custom_aggregate_scope_survives_the_parallel_focus_chunk_fork() {
        let mut shapes = load_shapes_ttl(&aggregate_shapes_ttl());
        shapes.aggregates = Arc::new(sum_aggregate_registry());

        let below = load_data_nt(&aggregate_data_nt(10));
        let above_n = crate::parallel::PARALLEL_MIN_FOCUS_NODES + 200;
        let above = load_data_nt(&aggregate_data_nt(above_n));
        assert!(above_n > crate::parallel::PARALLEL_MIN_FOCUS_NODES);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("test pool must build");

        let below_report = pool
            .install(|| validate_dataset(below.as_ref(), &shapes))
            .expect("below-threshold validation must resolve the registered aggregate");
        let above_report = pool
            .install(|| validate_dataset(above.as_ref(), &shapes))
            .expect(
                "above-threshold (forked) validation must resolve the SAME registered \
                 aggregate as the sequential path — a scheduling-dependent verdict is the bug",
            );

        for (label, report) in [("below", &below_report), ("above", &above_report)] {
            assert_eq!(
                report.results.len(),
                1,
                "{label}-threshold: exactly the negative-sum focus node violates: {:?}",
                report.results
            );
            assert_eq!(
                report.results[0].focus_node,
                NamedNode::new_unchecked("http://example.org/ns#v0").into_term(),
                "{label}-threshold: the violating focus node must match"
            );
            assert!(!report.conforms, "{label}-threshold: must not conform");
        }
    }

    /// An `AGG(<iri>, …)` call to an UNREGISTERED IRI is a clear typed error, not a
    /// silent fallback that reads the call as ordinary data — checked on BOTH the
    /// sequential (below-threshold) and the forked (above-threshold) paths, so a
    /// future fork omission cannot turn a hard error into a wrong silent verdict on
    /// just one of the two scheduling branches.
    #[test]
    fn unregistered_custom_aggregate_errors_on_both_scheduling_paths() {
        // `Shapes::default()`'s `aggregates` field is empty — the seam is off,
        // exactly as it is for every host that never configured one.
        let shapes = load_shapes_ttl(&aggregate_shapes_ttl());
        assert!(shapes.aggregates.is_empty());

        let below = load_data_nt(&aggregate_data_nt(10));
        let above_n = crate::parallel::PARALLEL_MIN_FOCUS_NODES + 200;
        let above = load_data_nt(&aggregate_data_nt(above_n));

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("test pool must build");

        let below_err = pool
            .install(|| validate_dataset(below.as_ref(), &shapes))
            .expect_err("an unregistered aggregate IRI must be a hard error, not a silent skip");
        assert!(
            below_err.contains("no custom aggregate is registered"),
            "below-threshold error must name the refusal: {below_err}"
        );
        assert!(below_err.contains(AGG_IRI), "got: {below_err}");

        let above_err = pool
            .install(|| validate_dataset(above.as_ref(), &shapes))
            .expect_err("the forked path must refuse identically, not silently pass");
        assert!(
            above_err.contains("no custom aggregate is registered"),
            "above-threshold error must name the refusal: {above_err}"
        );
        assert!(above_err.contains(AGG_IRI), "got: {above_err}");
    }

    /// [`PreparedValidator`] must source `aggregates` from the `Shapes` value exactly as
    /// it already does `functions` — not from the caller's ambient thread-local scope,
    /// which `PreparedValidator::new` installs only for its OWN target-resolution pass
    /// and drops the moment `new` returns. Every later call on the SAME prepared
    /// validator — `validate`, `validate_focus_nodes`, `validate_focus_node_ids` — must
    /// still resolve the registered aggregate with NO scope installed anywhere in this
    /// test, at both a focus-node count below `crate::parallel::PARALLEL_MIN_FOCUS_NODES`
    /// (the serial path) and above it (the forked path), because
    /// `evaluate_shape_focus_nodes` now builds the aggregate scope for every focus chunk
    /// directly from `shapes`, the same way it already built the function scope.
    ///
    /// The bounded routes (`validate_focus_nodes`/`validate_focus_node_ids`) fork on the
    /// SIZE OF THE SLICE THE CALLER PASSES IN, not on the size of the underlying dataset
    /// — `should_parallelize` reads `work_items` from that slice. So the above-threshold
    /// case passes the WHOLE `> PARALLEL_MIN_FOCUS_NODES`-sized focus set to each bounded
    /// route (genuinely forking it), while the below-threshold case passes a single node
    /// (genuinely staying serial); a case that always hands over one node, whatever its
    /// label claims, could never fork and would not establish the above-threshold claim.
    #[test]
    fn custom_aggregate_resolves_across_every_prepared_validator_route() {
        let shapes = Arc::new({
            let mut shapes = load_shapes_ttl(&aggregate_shapes_ttl());
            shapes.aggregates = Arc::new(sum_aggregate_registry());
            shapes
        });

        let above_n = crate::parallel::PARALLEL_MIN_FOCUS_NODES + 200;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("test pool must build");
        let v0 = NamedNode::new_unchecked("http://example.org/ns#v0").into_term();

        for (label, n, bounded_focus_terms) in [
            ("below-threshold", 10usize, vec![v0]),
            ("above-threshold", above_n, all_v_terms(above_n)),
        ] {
            assert_eq!(
                bounded_focus_terms.len() > crate::parallel::PARALLEL_MIN_FOCUS_NODES,
                label == "above-threshold",
                "{label}: the bounded routes' focus-node slice must cross \
                 PARALLEL_MIN_FOCUS_NODES exactly when the label claims it does"
            );
            let data = load_data_nt(&aggregate_data_nt(n));
            pool.install(|| {
                let projected = project_dataset(data.as_ref()).expect("projection must succeed");
                let prepared = PreparedValidator::from_projected_dataset(
                    Arc::clone(&projected),
                    Arc::clone(&shapes),
                )
                .unwrap_or_else(|error| panic!("{label}: preparation must succeed: {error}"));

                let whole = prepared.validate().unwrap_or_else(|error| {
                    panic!("{label}: PreparedValidator::validate: {error}")
                });
                assert_eq!(
                    whole.results.len(),
                    1,
                    "{label}: PreparedValidator::validate must resolve the registered \
                     aggregate: {:?}",
                    whole.results
                );
                assert!(!whole.conforms, "{label}: PreparedValidator::validate");

                let bounded = prepared
                    .validate_focus_nodes(&bounded_focus_terms)
                    .unwrap_or_else(|error| {
                        panic!("{label}: PreparedValidator::validate_focus_nodes: {error}")
                    });
                assert_eq!(
                    bounded.results.len(),
                    1,
                    "{label}: validate_focus_nodes must resolve the registered aggregate \
                     across a {}-node slice: {:?}",
                    bounded_focus_terms.len(),
                    bounded.results
                );
                assert!(!bounded.conforms, "{label}: validate_focus_nodes");

                let bounded_ids_input = term_ids_for(&prepared, &bounded_focus_terms, label);
                let bounded_ids = prepared
                    .validate_focus_node_ids(&bounded_ids_input)
                    .unwrap_or_else(|error| {
                        panic!("{label}: PreparedValidator::validate_focus_node_ids: {error}")
                    });
                assert_eq!(
                    bounded_ids.results.len(),
                    1,
                    "{label}: validate_focus_node_ids must resolve the registered aggregate \
                     across a {}-node slice: {:?}",
                    bounded_ids_input.len(),
                    bounded_ids.results
                );
                assert!(!bounded_ids.conforms, "{label}: validate_focus_node_ids");
            });
        }
    }

    /// The GOVERNED branch of `evaluate_shape_focus_nodes` already sourced `aggregates`
    /// from the `Shapes` value before this fix — it never depended on a thread-local
    /// snapshot — but nothing proved that end-to-end. Close that gap: both
    /// `validate_with_governors` and `validate_dataset_with_governors` must resolve the
    /// SAME registered aggregate an ungoverned validation does.
    #[test]
    fn custom_aggregate_resolves_under_governed_validation() {
        let mut shapes = load_shapes_ttl(&aggregate_shapes_ttl());
        shapes.aggregates = Arc::new(sum_aggregate_registry());
        let data = load_data_nt(&aggregate_data_nt(10));

        let outcome = validate_dataset_with_governors(
            data.as_ref(),
            &shapes,
            None,
            &QueryGovernors::UNBOUNDED,
        )
        .expect("governed validation must resolve the registered aggregate");
        let GovernedValidation::Complete { report, .. } = outcome else {
            panic!("an unbounded governor budget must not exhaust");
        };
        assert_eq!(
            report.results.len(),
            1,
            "governed validation must resolve the registered aggregate: {:?}",
            report.results
        );
        assert!(!report.conforms);
    }

    // ── property functions: a caller-scoped ambient contract, not a `Shapes` field ────

    /// A property-function relation reporting `"flagged"` for a literal whose lexical
    /// form is exactly `"bad"`. The SHACL-facing analog of `crate::sparql`'s own
    /// `DenyListRelation` test fixture (private to that module).
    #[derive(Debug)]
    struct PfFlagged {
        modes: [purrdf_sparql_eval::BindingPattern; 1],
    }

    impl PfFlagged {
        fn new() -> Self {
            Self {
                modes: [purrdf_sparql_eval::BindingPattern::from_code("bf")],
            }
        }
    }

    impl purrdf_sparql_eval::PropertyFunction for PfFlagged {
        fn volatility(&self) -> purrdf_sparql_eval::Volatility {
            purrdf_sparql_eval::Volatility::Stable
        }

        fn arity(&self) -> purrdf_sparql_eval::PfArity {
            purrdf_sparql_eval::PfArity::new(1, 1)
        }

        fn modes(&self) -> &[purrdf_sparql_eval::BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: purrdf_sparql_eval::BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            args: &purrdf_sparql_eval::PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn purrdf_sparql_eval::PfCursor>, purrdf_sparql_eval::EvalError> {
            let flagged = matches!(
                args.get(0),
                Some(::purrdf::TermValue::Literal { lexical_form, .. }) if lexical_form == "bad"
            );
            let rows = if flagged {
                let subject = args.get(0).cloned().expect("the bound subject");
                vec![vec![
                    subject,
                    ::purrdf::TermValue::typed_literal(
                        "flagged",
                        "http://www.w3.org/2001/XMLSchema#string",
                    ),
                ]]
            } else {
                Vec::new()
            };
            Ok(Box::new(PfFlaggedCursor { rows, next: 0 }))
        }
    }

    struct PfFlaggedCursor {
        rows: Vec<purrdf_sparql_eval::PfRow>,
        next: usize,
    }

    impl purrdf_sparql_eval::PfCursor for PfFlaggedCursor {
        fn next(
            &mut self,
        ) -> Result<Option<purrdf_sparql_eval::PfRow>, purrdf_sparql_eval::EvalError> {
            let row = self.rows.get(self.next).cloned();
            self.next += 1;
            Ok(row)
        }
    }

    /// A fresh registry with [`PfFlagged`] registered under `ex:pfFlagged`.
    fn pf_flagged_registry() -> purrdf_sparql_eval::PropertyFunctionRegistry {
        let mut registry = purrdf_sparql_eval::PropertyFunctionRegistry::new();
        registry.register(
            "http://example.org/ns#pfFlagged",
            Arc::new(PfFlagged::new()),
        );
        registry
    }

    /// Shapes carrying a `sh:sparql` constraint whose body calls the `ex:pfFlagged`
    /// property function against each focus node's `ex:status` value.
    fn pf_shapes_ttl() -> String {
        format!(
            r#"{PREFIXES}
            ex:PfShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:sparql ex:PfConstraint .
            ex:PfConstraint sh:select """
                SELECT $this ?status WHERE {{
                    $this <http://example.org/ns#status> ?status .
                    ?status <http://example.org/ns#pfFlagged> ?why
                }}
            """ .
            "#
        )
    }

    /// `n` `ex:Thing` focus nodes: `ex:v0` carries `ex:status "bad"` (the relation flags
    /// it, so it must violate), every other `ex:v{i}` carries `"ok"` (must conform).
    fn pf_data_nt(n: usize) -> String {
        use std::fmt::Write as _;
        assert!(n >= 1, "at least the flagged node is required");
        let mut nt = String::new();
        nt.push_str(
            "<http://example.org/ns#v0> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/ns#Thing> .\n\
             <http://example.org/ns#v0> <http://example.org/ns#status> \"bad\" .\n",
        );
        for i in 1..n {
            writeln!(
                nt,
                "<http://example.org/ns#v{i}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
                 <http://example.org/ns#Thing> .\n\
                 <http://example.org/ns#v{i}> <http://example.org/ns#status> \"ok\" ."
            )
            .expect("writing to a String cannot fail");
        }
        nt
    }

    /// Property functions have no `Shapes`-owned field — unlike `functions` and
    /// `aggregates`, a registered relation is a genuinely call-scoped, caller-injected
    /// table (`crate::sparql::enter_property_function_scope`'s own docs call it the
    /// "exact twin" of the aggregate scope, but there is no `Shapes::relations` for it to
    /// live in). The documented contract is: install the scope around the call that
    /// NEEDS it, not merely around `PreparedValidator::new`. This is deliberately NOT the
    /// F10-2 shape (a value silently sourced from whichever caller happened to install a
    /// scope first): here every route resolves correctly as long as the scope wraps THAT
    /// route's own call, proven at both scheduling geometries.
    ///
    /// As with the aggregate twin above, the bounded routes fork on the SIZE OF THE
    /// SLICE THE CALLER PASSES IN (`should_parallelize` reads `work_items` from that
    /// slice, not from the dataset), so the above-threshold case hands each bounded
    /// route the WHOLE `> PARALLEL_MIN_FOCUS_NODES`-sized focus set — genuinely forking
    /// it — while the below-threshold case hands over a single node — genuinely staying
    /// serial.
    #[test]
    fn property_function_resolves_across_every_prepared_validator_route_when_the_caller_wraps_the_call()
     {
        let shapes = Arc::new(load_shapes_ttl(&pf_shapes_ttl()));
        let above_n = crate::parallel::PARALLEL_MIN_FOCUS_NODES + 200;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("test pool must build");
        let v0 = NamedNode::new_unchecked("http://example.org/ns#v0").into_term();

        for (label, n, bounded_focus_terms) in [
            ("below-threshold", 10usize, vec![v0]),
            ("above-threshold", above_n, all_v_terms(above_n)),
        ] {
            assert_eq!(
                bounded_focus_terms.len() > crate::parallel::PARALLEL_MIN_FOCUS_NODES,
                label == "above-threshold",
                "{label}: the bounded routes' focus-node slice must cross \
                 PARALLEL_MIN_FOCUS_NODES exactly when the label claims it does"
            );
            let data = load_data_nt(&pf_data_nt(n));
            let projected =
                pool.install(|| project_dataset(data.as_ref()).expect("projection must succeed"));
            let prepared = pool.install(|| {
                PreparedValidator::from_projected_dataset(
                    Arc::clone(&projected),
                    Arc::clone(&shapes),
                )
                .unwrap_or_else(|error| panic!("{label}: preparation must succeed: {error}"))
            });

            let whole = pool
                .install(|| {
                    let _scope = crate::sparql::enter_property_function_scope(Arc::new(
                        pf_flagged_registry(),
                    ));
                    prepared.validate()
                })
                .unwrap_or_else(|error| panic!("{label}: validate: {error}"));
            assert_eq!(
                whole.results.len(),
                1,
                "{label}: validate: {:?}",
                whole.results
            );
            assert!(!whole.conforms, "{label}: validate");

            let bounded = pool
                .install(|| {
                    let _scope = crate::sparql::enter_property_function_scope(Arc::new(
                        pf_flagged_registry(),
                    ));
                    prepared.validate_focus_nodes(&bounded_focus_terms)
                })
                .unwrap_or_else(|error| panic!("{label}: validate_focus_nodes: {error}"));
            assert_eq!(
                bounded.results.len(),
                1,
                "{label}: validate_focus_nodes across a {}-node slice: {:?}",
                bounded_focus_terms.len(),
                bounded.results
            );
            assert!(!bounded.conforms, "{label}: validate_focus_nodes");

            let bounded_ids_input = term_ids_for(&prepared, &bounded_focus_terms, label);
            let bounded_ids = pool
                .install(|| {
                    let _scope = crate::sparql::enter_property_function_scope(Arc::new(
                        pf_flagged_registry(),
                    ));
                    prepared.validate_focus_node_ids(&bounded_ids_input)
                })
                .unwrap_or_else(|error| panic!("{label}: validate_focus_node_ids: {error}"));
            assert_eq!(
                bounded_ids.results.len(),
                1,
                "{label}: validate_focus_node_ids across a {}-node slice: {:?}",
                bounded_ids_input.len(),
                bounded_ids.results
            );
            assert!(!bounded_ids.conforms, "{label}: validate_focus_node_ids");
        }
    }

    #[test]
    fn complete_corpus_matches_across_scheduler_geometries() {
        use std::fs;
        use std::path::PathBuf;

        let corpus = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/corpus"));
        let mut cases: Vec<_> = fs::read_dir(&corpus)
            .expect("corpus directory must be readable")
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                path.is_dir().then_some(path)
            })
            .collect();
        cases.sort();
        assert_eq!(cases.len(), 70, "first-party corpus cardinality drifted");

        let geometries: Vec<_> = [(2, 1), (4, 7), (4, 64)]
            .into_iter()
            .map(|(threads, chunk_size)| {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .expect("test pool must build");
                (threads, chunk_size, pool)
            })
            .collect();

        for case in cases {
            let case_name = case
                .file_name()
                .expect("corpus case must have a name")
                .to_string_lossy();
            let data = fs::read_to_string(case.join("data.nt"))
                .unwrap_or_else(|error| panic!("{case_name}: data.nt: {error}"));
            let shapes = fs::read_to_string(case.join("shapes.ttl"))
                .unwrap_or_else(|error| panic!("{case_name}: shapes.ttl: {error}"));
            let render = || {
                validate_graphs_with_config(
                    &data,
                    &shapes,
                    None,
                    Some(crate::model::BoxRoleVocab::for_namespace(
                        "https://example.org/meta/",
                    )),
                )
                .unwrap_or_else(|error| panic!("{case_name}: validation failed: {error}"))
                .to_ntriples()
            };

            let expected = {
                let _guard = crate::parallel::force_scheduler_for_test(false, 1);
                render()
            };
            for (threads, chunk_size, pool) in &geometries {
                let actual = pool.install(|| {
                    let _guard = crate::parallel::force_scheduler_for_test(true, *chunk_size);
                    render()
                });
                assert_eq!(
                    actual, expected,
                    "{case_name}: report drifted with {threads} workers and chunk size {chunk_size}"
                );
            }
        }
    }

    // ── Focus-set ordering and dataset identity ──────────────────────────────

    /// A dataset whose interning order is deliberately the REVERSE of its
    /// canonical order, plus the shapes graph that targets every node in it.
    ///
    /// Interning descending means a focus comparator that reached for an id's
    /// NUMBER — directly, or by shortcutting the interned/interned pair to an
    /// integer compare — sorts the set exactly backwards rather than plausibly
    /// wrong, which is the only way that mistake is visible in a test.
    fn anti_canonical_focus_dataset() -> Arc<RdfDataset> {
        // Descending locals, so ids ascend as the rendered IRIs descend.
        focus_dataset_interning(&DESCENDING_FOCUS_LOCALS)
    }

    /// The fixture's ten focus locals in the order
    /// [`anti_canonical_focus_dataset`] interns them.
    const DESCENDING_FOCUS_LOCALS: [&str; 10] =
        ["n9", "n8", "n7", "n6", "n5", "n4", "n3", "n2", "n1", "n0"];

    /// The same ten focus nodes and the same targeting class, interned in
    /// whatever order `locals` names.
    ///
    /// Interning order is what assigns ids, so two datasets built from the same
    /// IRIs in two orders give the SAME IRI two different ids — and therefore give
    /// one id two different meanings. That is the hazard a focus id's provenance
    /// stamp exists for, and it is what
    /// [`a_focus_id_from_another_binding_is_refused_and_its_own_is_accepted`]
    /// builds.
    fn focus_dataset_interning(locals: &[&str]) -> Arc<RdfDataset> {
        let mut builder = ::purrdf::RdfDatasetBuilder::new();
        let rdf_type = builder.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let class = builder.intern_iri("http://example.org/ns#Focus");
        for local in locals {
            let node = builder.intern_iri(&format!("http://example.org/ns#{local}"));
            builder.push_quad(node, rdf_type, class, None);
        }
        builder.freeze().expect("the fixture dataset freezes")
    }

    /// Every interned focus node of [`anti_canonical_focus_dataset`], in id
    /// order.
    fn anti_canonical_focus_ids(data: &ShaclData) -> Vec<TermId> {
        (0..data.core_view().term_count())
            .map(|index| TermId::from_index(u32::try_from(index).expect("fixture fits in u32")))
            .filter(|id| {
                matches!(
                    data.core_view().resolve(*id),
                    ::purrdf::TermRef::Iri(iri) if iri.contains("/ns#n")
                )
            })
            .collect()
    }

    /// Bind the anti-canonical fixture to a shapes graph that targets it.
    fn anti_canonical_validator() -> PreparedValidator {
        let shapes = load_shapes_ttl(&format!(
            "{PREFIXES}
            ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
                sh:property [ sh:path ex:absent ; sh:maxCount 1 ] ."
        ));
        PreparedShapes::new(Arc::new(shapes))
            .bind_shared_dataset(anti_canonical_focus_dataset())
            .expect("the fixture binds")
    }

    /// **The focus comparator orders by what a node DENOTES, in all three
    /// populations: all-interned, all-foreign, and interleaved.**
    ///
    /// The interleaved case is the one that matters. A comparator that shortcut
    /// the interned/interned pair to an integer compare, or that partitioned the
    /// two representations instead of interleaving them, passes the first two
    /// cases — each is internally self-consistent — and reorders every mixed
    /// focus set silently. The dataset is interned ANTI-CANONICALLY so that
    /// comparing ids by number cannot accidentally look right.
    #[test]
    fn focus_order_is_canonical_across_interned_and_foreign_nodes() {
        let validator = anti_canonical_validator();
        let data = &validator.data;
        let view = data.core_view();
        let interned = anti_canonical_focus_ids(data);
        assert!(
            interned.len() >= 10,
            "the fixture must hold every focus node"
        );

        // The fixture really is anti-canonical: id order is not canonical order.
        assert!(
            interned.windows(2).any(|pair| {
                focus_cmp(
                    view,
                    &FocusNode::Interned(pair[0]),
                    &FocusNode::Interned(pair[1]),
                ) == std::cmp::Ordering::Greater
            }),
            "the fixture must intern its focus nodes out of canonical order"
        );

        // Terms this dataset never interned, so they can only be `Foreign`, and
        // chosen to INTERLEAVE with the interned locals rather than sort as a
        // block on either side of them.
        let foreign: Vec<Term> = ["n05", "n15", "n25", "n35", "n45", "n55"]
            .iter()
            .map(|local| {
                Term::NamedNode(NamedNode::new_unchecked(format!(
                    "http://example.org/ns#{local}"
                )))
            })
            .collect();
        for term in &foreign {
            assert!(
                resolve_id(view, term).is_none(),
                "a foreign fixture term must not be interned: {term}"
            );
        }

        let all_interned: Vec<FocusNode> =
            interned.iter().copied().map(FocusNode::Interned).collect();
        let all_foreign: Vec<FocusNode> = foreign.iter().cloned().map(FocusNode::Foreign).collect();
        let mut interleaved = Vec::new();
        for (index, node) in all_interned.iter().enumerate() {
            interleaved.push(node.clone());
            if let Some(term) = foreign.get(index) {
                interleaved.push(FocusNode::Foreign(term.clone()));
            }
        }
        assert!(
            interleaved.len() > all_interned.len() && interleaved.len() > all_foreign.len(),
            "the interleaved population must really mix both representations"
        );

        for (label, mut population) in [
            ("all-interned", all_interned),
            ("all-foreign", all_foreign),
            ("interleaved", interleaved),
        ] {
            // The reference order: materialize, render, sort the strings. That is
            // the definition the comparator has to reproduce without rendering.
            let mut expected: Vec<String> = population
                .iter()
                .map(|focus| focus.to_term(view).to_string())
                .collect();
            expected.sort();
            sort_focus_nodes(view, &mut population);
            let actual: Vec<String> = population
                .iter()
                .map(|focus| focus.to_term(view).to_string())
                .collect();
            assert_eq!(
                actual, expected,
                "{label}: the focus comparator did not reproduce rendered byte order"
            );
        }
    }

    /// **A focus set resolved against one binding is refused by another, and the
    /// binding it came from still accepts it.**
    ///
    /// Both directions, because a refusal that fires on everything is not a
    /// check: the accepting half is what says the guard distinguishes the
    /// datasets rather than distrusting every caller. The refused ids are IN
    /// RANGE for the receiving binding — that is the whole hazard, since an
    /// out-of-range id is already rejected and a wrong-dataset one resolves
    /// quietly to a different term.
    #[test]
    fn a_focus_set_from_another_binding_is_refused_and_its_own_is_accepted() {
        let shapes = Arc::new(load_shapes_ttl(&format!(
            "{PREFIXES}
            ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
                sh:property [ sh:path ex:absent ; sh:maxCount 1 ] ."
        )));
        let prepared = PreparedShapes::new(shapes);
        let here = prepared
            .bind_shared_dataset(anti_canonical_focus_dataset())
            .expect("the first binding binds");
        // A SECOND, independently interned snapshot of the same shape of data —
        // exactly what `bind_delta_with_shapes_graph` leaves a caller holding
        // beside a base binding.
        let there = prepared
            .bind_shared_dataset(anti_canonical_focus_dataset())
            .expect("the second binding binds");

        let ids = anti_canonical_focus_ids(&there.data);
        assert!(!ids.is_empty(), "the fixture must hold focus nodes");
        for &id in &ids {
            assert!(
                id.index() < here.data.core_view().term_count(),
                "the refused ids must be IN RANGE for the receiving binding, or the existing \
                 range check would be what rejected them"
            );
        }

        // Built against `there`, handed to `there`: accepted.
        let mut own = FocusSet::with_capacity(&there.data, ids.len());
        for &id in &ids {
            own.push(FocusNode::Interned(id));
        }
        own.sort(&there.data);
        let accepted = there
            .validate_bounded(&own)
            .expect("a focus set from this very binding must be accepted");
        assert!(
            accepted.conforms,
            "the fixture focus nodes conform, so the accepting half is a real validation"
        );

        // Built against `there`, handed to `here`: refused.
        let mut foreign_set = FocusSet::with_capacity(&there.data, ids.len());
        for &id in &ids {
            foreign_set.push(FocusNode::Interned(id));
        }
        foreign_set.sort(&there.data);
        let refused = here.validate_bounded(&foreign_set);
        let message = refused.expect_err("a focus set from another binding must be refused");
        assert!(
            message.contains("dataset-local"),
            "the refusal must say why TermIds are not portable: {message}"
        );
    }

    /// The fixture's ten focus locals as owned terms, in interning order.
    fn descending_focus_terms() -> Vec<Term> {
        DESCENDING_FOCUS_LOCALS
            .iter()
            .map(|local| {
                NamedNode::new_unchecked(format!("http://example.org/ns#{local}")).into_term()
            })
            .collect()
    }

    /// The shapes graph both focus-id provenance tests bind.
    ///
    /// `sh:minCount 1` on a predicate NOTHING in the fixture carries, so every
    /// focus node produces a result. That is what lets the accepting half below
    /// compare report CONTENT: a bare `conforms` flag is satisfied just as well by
    /// a validator that stopped validating.
    fn focus_provenance_shapes() -> Arc<Shapes> {
        Arc::new(load_shapes_ttl(&format!(
            "{PREFIXES}
            ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
                sh:property [ sh:path ex:absent ; sh:minCount 1 ] ."
        )))
    }

    /// **The PUBLIC id-native entry point refuses a focus id minted by another
    /// binding, and still validates one minted by its own.**
    ///
    /// The sibling test above drives `validate_bounded`, which carries no
    /// visibility modifier at all and so is private to this module; a guard that
    /// only ever runs behind a door no caller can open is not a guard on the door
    /// callers use. This one drives
    /// [`PreparedValidator::validate_focus_node_ids`] itself.
    ///
    /// The two bindings intern the SAME IRIs in OPPOSITE orders, so an id is not
    /// merely foreign — it is in range and it denotes a different term, which is
    /// asserted here rather than assumed. Both directions are executed: a refusal
    /// that fired on everything would close the hazard by closing the feature,
    /// and the accepting half is what distinguishes the two.
    #[test]
    fn a_focus_id_from_another_binding_is_refused_and_its_own_is_accepted() {
        let prepared = PreparedShapes::new(focus_provenance_shapes());
        let here = prepared
            .bind_shared_dataset(focus_dataset_interning(&DESCENDING_FOCUS_LOCALS))
            .expect("the first binding binds");
        let mut ascending = DESCENDING_FOCUS_LOCALS;
        ascending.reverse();
        // A SECOND, independently interned snapshot of the same data — exactly
        // what `bind_delta_with_shapes_graph` leaves a caller holding beside a
        // base binding — with the interning order reversed.
        let there = prepared
            .bind_shared_dataset(focus_dataset_interning(&ascending))
            .expect("the second binding binds");

        let terms = descending_focus_terms();
        let mint = |validator: &PreparedValidator| -> Vec<FocusId> {
            terms
                .iter()
                .map(|term| {
                    validator
                        .term_id(term)
                        .unwrap_or_else(|| panic!("{term} must be interned"))
                })
                .collect()
        };
        let here_ids = mint(&here);
        let there_ids = mint(&there);

        // The hazard, made concrete. Every id minted by `there` is IN RANGE for
        // `here`, and at least one of them denotes a DIFFERENT term there — so
        // without the stamp it would resolve, dispatch and validate the wrong
        // node, reporting on a focus set nobody asked about.
        let mut denoting_differently = 0_usize;
        for (term, focus) in terms.iter().zip(&there_ids) {
            assert!(
                focus.term_id().index() < here.data.core_view().term_count(),
                "{term}'s id from the other binding must be IN RANGE here, or the range check \
                 would be what rejected it rather than the provenance stamp"
            );
            if let ::purrdf::TermRef::Iri(denoted) = here.data.core_view().resolve(focus.term_id())
                && NamedNode::new_unchecked(denoted).into_term() != *term
            {
                denoting_differently += 1;
            }
        }
        assert!(
            denoting_differently > 0,
            "the two bindings must give at least one IRI two different ids, or this fixture is \
             not the hazard the stamp exists for"
        );

        // REFUSED: minted by `there`, handed to `here`.
        let refused = here
            .validate_focus_node_ids(&there_ids)
            .expect_err("focus ids minted by another binding must be refused");
        assert!(
            refused.contains("minted against a different dataset binding"),
            "the refusal must name the mismatch it found: {refused}"
        );
        assert!(
            refused.contains("dataset-local"),
            "the refusal must say why an id is not portable: {refused}"
        );

        // ACCEPTED: each binding still validates its own, and produces the report
        // the term-keyed route produces over the same nodes — same content, same
        // order.
        for (label, validator, ids) in [("here", &here, &here_ids), ("there", &there, &there_ids)] {
            let accepted = validator
                .validate_focus_node_ids(ids)
                .unwrap_or_else(|error| {
                    panic!("{label}: its own focus ids must validate: {error}")
                });
            assert_eq!(
                accepted.results.len(),
                DESCENDING_FOCUS_LOCALS.len(),
                "{label}: every focus node violates sh:minCount, so the accepting half has to be \
                 a real validation rather than an empty pass"
            );
            assert_eq!(
                accepted.to_ntriples(),
                validator
                    .validate_focus_nodes(&terms)
                    .unwrap_or_else(|error| panic!("{label}: the term-keyed route: {error}"))
                    .to_ntriples(),
                "{label}: the id-native route must still produce the report it produced before, \
                 content and ORDER alike"
            );
        }
    }

    /// **The change loop is type-closed: what the expansion answers is exactly
    /// what the id-native entry point takes, with no conversion in between.**
    ///
    /// That is the point of carrying provenance on the id rather than documenting
    /// it. `delta` → [`PreparedValidator::affected_focus_node_ids`] →
    /// [`PreparedValidator::validate_focus_node_ids`] passes a `&[FocusId]`
    /// straight through, so there is no place for a caller to strip the stamp and
    /// no place for one to be attached to an id that did not earn it.
    #[test]
    fn the_change_expansion_feeds_the_id_native_entry_point_unconverted() {
        let base = focus_dataset_interning(&DESCENDING_FOCUS_LOCALS);
        let mut mutation = ::purrdf::MutableDataset::new(base);
        assert!(
            ::purrdf::DatasetMut::insert(
                &mut mutation,
                ::purrdf::QuadValues {
                    s: ::purrdf::TermValue::iri("http://example.org/ns#n10"),
                    p: ::purrdf::TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
                    o: ::purrdf::TermValue::iri("http://example.org/ns#Focus"),
                    g: None,
                }
            )
            .expect("the insert applies"),
            "the fixture row must really change the graph, or the expansion below is empty for \
             the wrong reason"
        );
        let snapshot = Arc::new(mutation.snapshot_view().expect("the mutation snapshots"));
        let validator = PreparedShapes::new(focus_provenance_shapes())
            .bind_delta_with_shapes_graph(
                Arc::clone(&snapshot),
                None,
                ::purrdf::ir::ViewLimits::default(),
            )
            .expect("the delta binds");

        let expansion = validator
            .affected_focus_node_ids(&snapshot)
            .expect("the expansion succeeds");
        let ids = expansion.ids().unwrap_or_else(|| {
            panic!(
                "this shapes graph is readable, so the expansion must be bounded ({:?})",
                expansion.reason()
            )
        });
        assert_eq!(
            ids.len(),
            1,
            "the inserted row makes exactly one new focus node, and an expansion that named \
             more or fewer is not the loop this test closes"
        );

        // The loop, with nothing between its two halves.
        let report = validator
            .validate_focus_node_ids(ids)
            .expect("the expansion must validate through the entry point it feeds");
        assert_eq!(
            report.results.len(),
            1,
            "the new focus node carries no ex:absent, so re-validating the expansion must report \
             its violation"
        );
    }

    /// Bind the one-row change of the test above against `shapes`, returning the
    /// snapshot and the validator every change-path entry point below drives.
    fn change_fixture(
        shapes: Arc<Shapes>,
    ) -> (Arc<::purrdf::ir::DeltaDatasetView>, PreparedValidator) {
        let base = focus_dataset_interning(&DESCENDING_FOCUS_LOCALS);
        let mut mutation = ::purrdf::MutableDataset::new(base);
        assert!(
            ::purrdf::DatasetMut::insert(
                &mut mutation,
                ::purrdf::QuadValues {
                    s: ::purrdf::TermValue::iri("http://example.org/ns#n10"),
                    p: ::purrdf::TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
                    o: ::purrdf::TermValue::iri("http://example.org/ns#Focus"),
                    g: None,
                }
            )
            .expect("the insert applies"),
            "the fixture row must really change the graph"
        );
        let snapshot = Arc::new(mutation.snapshot_view().expect("the mutation snapshots"));
        let validator = PreparedShapes::new(shapes)
            .bind_delta_with_shapes_graph(
                Arc::clone(&snapshot),
                None,
                ::purrdf::ir::ViewLimits::default(),
            )
            .expect("the delta binds");
        (snapshot, validator)
    }

    /// A shapes graph whose constraint reads through SPARQL query text, so its
    /// change footprint is TOP and the entry points below must fall back.
    fn opaque_footprint_shapes() -> Arc<Shapes> {
        Arc::new(load_shapes_ttl(&format!(
            "{PREFIXES}
            ex:Shape a sh:NodeShape ; sh:targetClass ex:Focus ;
                sh:sparql [ a sh:SPARQLConstraint ;
                    sh:message \"every focus node needs an ex:absent\" ;
                    sh:select \"\"\"SELECT $this WHERE {{ \
                        FILTER NOT EXISTS {{ $this <http://example.org/ns#absent> ?v }} }}\"\"\" ] ."
        )))
    }

    /// **[`validate_change`] IS the loop, and it says which arm it took.**
    ///
    /// Both arms are executed, because the fallback is the one a surface can omit
    /// and still pass every happy-path test: a bounded expansion and a full
    /// validation are indistinguishable in a report, and the scope is the only
    /// thing that distinguishes them.
    #[test]
    fn the_change_entry_point_reaches_both_arms_and_names_which() {
        let (snapshot, validator) = change_fixture(focus_provenance_shapes());
        let validation =
            validate_change(&validator, &snapshot).expect("the bounded change validates");
        assert_eq!(validation.scope, ChangeScope::Bounded { focus_nodes: 1 });
        assert_eq!(validation.scope.focus_nodes(), Some(1));
        assert_eq!(validation.scope.reason(), None);
        let expansion = validator
            .affected_focus_node_ids(&snapshot)
            .expect("the expansion succeeds");
        assert_eq!(
            validation.report.to_ntriples(),
            validator
                .validate_focus_node_ids(expansion.ids().expect("bounded"))
                .expect("the hand-driven loop validates")
                .to_ntriples(),
            "the one call and the hand-driven loop must reach one report, content and ORDER alike",
        );

        let (snapshot, validator) = change_fixture(opaque_footprint_shapes());
        let validation =
            validate_change(&validator, &snapshot).expect("the unbounded change validates");
        assert!(!validation.scope.is_bounded(), "{:?}", validation.scope);
        assert_eq!(
            validation.scope.focus_nodes(),
            None,
            "a fallback covers no COUNT: every focus node in the graph is not a number",
        );
        assert!(validation.scope.reason().is_some_and(|why| !why.is_empty()));
        assert_eq!(
            validation.report.to_ntriples(),
            validator
                .validate()
                .expect("the full validation succeeds")
                .to_ntriples(),
            "the fallback must BE the full validation, not a short answer wearing its name",
        );
    }

    /// **The governed change entry point installs ONE budget and reports the
    /// scope either way — including when the budget stops the run.**
    ///
    /// The scope is settled before the first query executes, so a tripped budget
    /// cannot take it away; an operator whose incremental run ran out of fuel
    /// still learns which question it was asking.
    #[test]
    fn the_governed_change_entry_point_reports_its_scope_through_a_trip() {
        // A SPARQL-free shapes graph spends no evaluator budget, so it completes
        // under a ZERO one. That is the honest answer rather than an oversight,
        // and it is the neighbouring valid case for the trip below.
        let (snapshot, validator) = change_fixture(focus_provenance_shapes());
        let ungoverned =
            validate_change(&validator, &snapshot).expect("the bounded change validates");
        for governors in [
            QueryGovernors::UNBOUNDED,
            QueryGovernors::UNBOUNDED.with_fuel(0),
        ] {
            let governed = validate_change_with_governors(&validator, &snapshot, &governors)
                .expect("the governed change validates");
            assert_eq!(governed.scope, ungoverned.scope);
            let GovernedValidation::Complete { report, .. } = governed.outcome else {
                panic!("core constraint evaluation charges no evaluator budget, so this completes");
            };
            assert_eq!(report.to_ntriples(), ungoverned.report.to_ntriples());
        }

        // The fallback arm is where the budget bites, because it is the arm that
        // runs the query text. The scope survives the trip.
        let (snapshot, validator) = change_fixture(opaque_footprint_shapes());
        let governed =
            validate_change_with_governors(&validator, &snapshot, &QueryGovernors::UNBOUNDED)
                .expect("the unbounded run completes");
        assert!(!governed.scope.is_bounded());
        assert!(matches!(
            governed.outcome,
            GovernedValidation::Complete { .. }
        ));

        let stopped = validate_change_with_governors(
            &validator,
            &snapshot,
            &QueryGovernors::UNBOUNDED.with_fuel(0),
        )
        .expect("a trip is an outcome, not an error");
        assert!(
            !stopped.scope.is_bounded(),
            "the scope is decided before the first query, so a trip cannot erase it",
        );
        assert!(
            matches!(stopped.outcome, GovernedValidation::BudgetExhausted { .. }),
            "zero fuel against a shapes graph that runs SPARQL must stop the run",
        );
    }
}
