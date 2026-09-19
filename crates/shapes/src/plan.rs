// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SHACL validation plan: the binding-time staging of a shapes graph.
//!
//! Validation has three binding times, and every derivation belongs to exactly
//! one of them. Naming them is the whole point of this module, because the defect
//! it replaces was a derivation running at the WRONG one — shape-constant work
//! repeated once per focus node, which is asymptotically correct and quietly costs
//! what the incremental change path exists to avoid.
//!
//! | Stage | Depends on | Holds |
//! |-------|------------|-------|
//! | **0** | the shapes graph alone | structure, the class analysis, and every shape-constant derivation |
//! | **1** | stage 0 × one dataset | [`TermId`] resolution, class membership, target indexes |
//! | **2** | × one focus node | the only work that may look at a focus node |
//!
//! * Stage 0 is [`LoweredShapes`]: ONE total walk over `Constraint`, `Path` and
//!   `NodeExpr` that both collects the class references (the [`ClassCatalog`]) and
//!   lowers every constraint into a [`LoweredConstraint`]. It is derived once per
//!   shapes graph and memoized, so a preparation restored from a prepared product
//!   pays for it exactly once — on its first bind — rather than never (which would
//!   leave its constraints silently unable to constrain) or on every restore.
//! * Stage 1 is [`DatasetBinding`]: the flat row of dataset identities the stage-0
//!   walk asked for, resolved once per bind. Every slot is an ARRAY INDEX the
//!   lowering recorded, never a dictionary lookup repeated per focus node.
//! * Stage 2 is the evaluator in [`crate::constraints`], which receives a
//!   [`ShapePlan`] and may not resolve anything the plan does not already hold.
//!
//! # The two negative answers
//!
//! A term the plan names but this DATA GRAPH does not intern is entirely ordinary:
//! a shapes graph may name an IRI the data never mentions, and the right answer is
//! "nothing can match", not a refusal. That is `None` in a
//! [`DatasetBinding`] slot and it stays soft forever.
//!
//! A term the EVALUATOR asks about that the LOWERING never recorded is the other
//! condition entirely: the walk failed to reach something the evaluator went on to
//! need, which is a defect in this crate rather than in a caller's data. Those are
//! `Err`, never a panic — a panic aborts across the PyO3 and C ABI boundaries where
//! no host has a frame to catch it.

use std::sync::{Arc, OnceLock};

use ::purrdf::{FastMap, FastSet, IdSet, TermId};

use crate::data::resolve_id;
use crate::data_view::ShaclRead;
use crate::expression::{FnCall, NodeExpr, ShapeArg};
use crate::shapes::{
    ComponentValidator, Constraint, NodeKindValue, Path, PropertyShape, Shape, Target,
};
use crate::term::{NamedNode, Term};

// ── Slots ───────────────────────────────────────────────────────────────────────

/// The position of one term's dataset identity in a [`DatasetBinding`]'s row.
///
/// An index, not a key. The whole reason the lowering exists is that resolving a
/// shape constant is stage-1 work, so the evaluator must never hold anything it
/// could look a constant UP by.
pub(crate) type TermSlot = u32;

/// The position of one id SET in a [`DatasetBinding`]'s set row.
type SetSlot = u32;

// ── The class catalog ───────────────────────────────────────────────────────────

/// Dataset-independent class references from the complete, cycle-aware shape walk.
/// Dataset bindings retain only resolved IDs; class names are owned here once.
#[derive(Debug)]
pub(crate) struct ClassCatalog {
    indices: FastMap<NamedNode, usize>,
}

impl ClassCatalog {
    /// Every planned class with the position its resolved [`TermId`] occupies in a
    /// [`DatasetBinding`]'s class row, in unspecified order.
    ///
    /// `pub(crate)` for the prepared-product codec, which both WRITES these pairs
    /// into the artifact (`crate::product::ast`) and digests them into the
    /// product's identity (`crate::product::identity::class_catalog_digest`), so a
    /// restore can prove the body it carried is the analysis the identity pinned.
    /// The order is the backing map's and is therefore NOT a fact about the
    /// catalog — every consumer sorts.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&NamedNode, usize)> {
        self.indices
            .iter()
            .map(|(class, &position)| (class, position))
    }

    /// Rebuild a catalog from `(class, position)` pairs a prepared product carried,
    /// or `None` when those pairs are not a catalog any walk could have produced.
    ///
    /// This is the codec's re-entry point for the reusable analysis, and it is
    /// deliberately the narrowest possible gate: it checks only what the TYPE's own
    /// invariants require, and leaves the question of whether these are the RIGHT
    /// classes to the identity digest that already binds them.
    ///
    /// Two conditions are structural rather than a matter of taste, because
    /// [`LoweredShapes::bind`] indexes a `Vec` of exactly `indices.len()` slots by
    /// the position it reads back out, and [`DatasetBinding::class_id`] expects
    /// every class it is asked about to be present:
    ///
    /// * a class IRI may appear **once**, because a repeat would silently collapse
    ///   two entries into one and leave the binding row short by a slot; and
    /// * the positions must be a **permutation of `0..len`**, because a position at
    ///   or past the row's length is an out-of-bounds index — a panic, which is an
    ///   abort no caller of a decoder could handle, arriving from bytes a caller
    ///   supplied.
    ///
    /// What it deliberately does NOT check is that each class sits at the rank
    /// [`Self::from_walk`] would have given it. That rule belongs to the
    /// derivation, and re-stating it here would be a second transcription of the
    /// reachability rule inside the reader — the drift this codec spends a stage id
    /// preventing. A permutation that is not the derivation's own is caught where
    /// every other content claim is caught: the position is folded into
    /// `class_catalog_digest`, so a product carrying one is refused on the
    /// class-catalog dimension rather than restored.
    pub(crate) fn from_entries(entries: Vec<(NamedNode, usize)>) -> Option<Self> {
        let mut seen = vec![false; entries.len()];
        for &(_, position) in &entries {
            let slot = seen.get_mut(position)?;
            if std::mem::replace(slot, true) {
                return None;
            }
        }
        let indices: FastMap<NamedNode, usize> = entries.into_iter().collect();
        // A duplicate class IRI collapses in the map and is visible only as a
        // shortfall against the slots just proven to be a permutation.
        (indices.len() == seen.len()).then_some(Self { indices })
    }

    /// Assign each class the walk reached its binding-row position.
    ///
    /// The rule is total order on the IRI, then rank: the catalog travels inside a
    /// prepared product and is digested there, so a position that depended on the
    /// walk's arrival order — or on a hash iteration order — would make one shapes
    /// graph produce several different, equally valid analyses and no product would
    /// ever verify against another build of the same sources.
    pub(crate) fn from_walk(classes: FastSet<NamedNode>) -> Self {
        let mut classes: Vec<_> = classes.into_iter().collect();
        classes.sort_unstable();
        let indices = classes
            .into_iter()
            .enumerate()
            .map(|(position, class)| (class, position))
            .collect();
        Self { indices }
    }

    /// The binding-row position of `class`, or `None` when the walk never reached
    /// it.
    #[inline]
    pub(crate) fn position(&self, class: &NamedNode) -> Option<usize> {
        self.indices.get(class).copied()
    }

    /// How many classes the catalog holds — the width of a binding's class row.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.indices.len()
    }
}

// ── Stage 0: the lowered shape tree ─────────────────────────────────────────────

/// The stage-0 lowering of one whole shapes graph: the single artifact the class
/// walk and the constraint lowering both produce.
///
/// Derived once per shapes graph. It is not tied to any dataset, holds no focus
/// node, and is shared across every bind by `Arc`, which is what lets a
/// preparation restored from a prepared product pay for the walk once rather than
/// once per restore.
#[derive(Debug)]
pub(crate) struct LoweredShapes {
    /// Every term whose dataset identity some lowered node names, by [`TermSlot`].
    ///
    /// Owned rather than borrowed from the AST because a binding resolves this row
    /// straight through without touching the shape tree at all: per-bind cost is a
    /// flat sweep, not a second traversal that could disagree with the first.
    terms: Box<[Term]>,
    /// Every id SET some lowered node names, as the [`Self::terms`] range its
    /// members occupy. `sh:in`'s members are already in `terms`, so a set is a
    /// range rather than a second copy of them.
    sets: Box<[std::ops::Range<u32>]>,
    /// One lowering per shape the walk was given, in the order it was given them.
    shapes: Box<[LoweredShape]>,
    /// The class references this same walk collected.
    ///
    /// Shared rather than owned so a preparation can hold the SAME catalog this
    /// derivation produced, instead of a second copy that could drift from it.
    classes: Arc<ClassCatalog>,
    /// The `sh:nodeByExpression` / `shnex:conformsToShape` shape indexes, lowered.
    indexes: Box<[LoweredIndex]>,
    /// The lowered body of every custom node-expression function the walk reached,
    /// by function IRI.
    ///
    /// A map rather than a subtree because SHACL 1.2 Node Expressions §6.1/§6.2
    /// hold a function body BY REFERENCE precisely so a body may call the function
    /// it belongs to. That cycle has no finite tree, so the body is lowered once
    /// under its own IRI and every call site resolves it by name.
    bodies: FastMap<String, LoweredExpr>,
    /// The empty target index every shape reached through a CONSTRAINT carries.
    ///
    /// One shared value rather than one per nested plan: `sh:not`, `sh:and`,
    /// `sh:node` and their siblings reach shapes that are never a target's shape,
    /// so they have no targets to index, and minting an empty index per descent
    /// would put an allocation back on the per-focus-node path this lowering
    /// exists to clear.
    no_targets: PreparedTargets,
}

impl LoweredShapes {
    /// The class references this walk collected.
    ///
    /// The derivation's own answer, which a preparation restored from a prepared
    /// product deliberately does NOT use — it carries a catalog of its own, and the
    /// product's identity already proved that catalog is this derivation's output.
    /// The two are held to agreement by a test rather than by one silently
    /// overwriting the other.
    #[inline]
    pub(crate) fn classes(&self) -> &Arc<ClassCatalog> {
        &self.classes
    }

    /// Resolve every identity the lowering asked for against `dataset`, once.
    ///
    /// This is the whole of stage 1 for the constraint evaluator: a flat sweep of
    /// the slot row, with no second traversal of the shape tree that could reach a
    /// different set of terms than the walk did.
    ///
    /// `classes` is the catalog to bind the class row from — the one the
    /// preparation holds, which for a restored product is the catalog the product
    /// carried rather than [`Self::classes`].
    pub(crate) fn bind(&self, dataset: &impl ShaclRead, classes: &ClassCatalog) -> DatasetBinding {
        let terms: Box<[Option<TermId>]> = self
            .terms
            .iter()
            .map(|term| resolve_id(dataset, term))
            .collect();
        let sets: Box<[FastSet<TermId>]> = self
            .sets
            .iter()
            .map(|range| {
                // A member the data graph does not intern cannot equal an interned
                // value node, because the interner is injective. Dropping it here
                // is not a loss of the member: the evaluator keeps the AST's own
                // term list for the non-interned value-node fallback, which is the
                // only case in which a dropped member could still match.
                terms[range.start as usize..range.end as usize]
                    .iter()
                    .copied()
                    .flatten()
                    .collect()
            })
            .collect();
        let mut class_ids = vec![None; classes.len()].into_boxed_slice();
        for (class, position) in classes.entries() {
            class_ids[position] = dataset.term_id_by_iri(class.as_str());
        }
        DatasetBinding {
            terms,
            sets,
            class_ids,
        }
    }

    /// The plan for the `position`-th shape this lowering covers.
    ///
    /// # Errors
    /// Returns an error when `position` is past the shapes the walk lowered, which
    /// is a defect in the caller pairing a shape list with a lowering of a
    /// different one.
    pub(crate) fn plan<'a>(
        &'a self,
        shape: &'a Shape,
        position: usize,
        binding: &'a DatasetBinding,
        classes: &'a ClassCatalog,
        targets: &'a PreparedTargets,
    ) -> Result<ShapePlan<'a>, String> {
        let lowered = self.shapes.get(position).ok_or_else(|| {
            format!(
                "internal validation-plan defect: shape {position} was evaluated against a \
                 lowering covering only {} shape(s), so the shape list and its lowering have \
                 diverged",
                self.shapes.len()
            )
        })?;
        Ok(ShapePlan {
            shape,
            lowered,
            graph: self,
            binding,
            classes,
            targets,
        })
    }

    /// The empty target index nested plans carry.
    #[inline]
    pub(crate) fn no_targets(&self) -> &PreparedTargets {
        &self.no_targets
    }
}

/// One `sh:nodeByExpression` / `shnex:conformsToShape` shape index, lowered.
#[derive(Debug)]
struct LoweredIndex {
    /// The shared cell this lowering belongs to.
    ///
    /// Retained, not just addressed: the index is identified by the address of its
    /// `OnceLock` cell, and an address is only unique while the allocation is
    /// alive. Holding the `Arc` is what makes the key stable for as long as the
    /// lowering that uses it.
    cell: Arc<OnceLock<FastMap<String, Shape>>>,
    /// The lowering of every shape the index can resolve to, by shape IRI.
    shapes: FastMap<String, LoweredShape>,
}

/// The stage-0 lowering of one shape: its own constraints and its property shapes.
///
/// Positionally parallel to the `Shape` it was lowered from — `constraints[i]` is
/// the lowering of `shape.constraints[i]` — and produced by the same walk, so the
/// pairing is established once, at construction, rather than re-derived by a second
/// traversal that could drift from the first.
#[derive(Debug)]
pub(crate) struct LoweredShape {
    constraints: Box<[LoweredConstraint]>,
    properties: Box<[LoweredProperty]>,
}

/// The stage-0 lowering of one property shape.
#[derive(Debug)]
pub(crate) struct LoweredProperty {
    /// The lowered property path — every predicate step resolved to a slot.
    path: LoweredPath,
    constraints: Box<[LoweredConstraint]>,
    properties: Box<[Self]>,
    reifiers: Box<[LoweredShape]>,
}

/// The stage-0 lowering of one SHACL property path.
///
/// Self-sufficient: evaluating it needs the binding row and nothing else, so a path
/// step never re-hashes its predicate IRI per focus node.
#[derive(Debug)]
pub(crate) enum LoweredPath {
    /// A predicate step, as the slot holding the predicate's dataset identity.
    Predicate(TermSlot),
    /// `sh:inversePath` over a plain predicate — the one inverse form that needs no
    /// structural rewriting, so it lowers to its predicate's slot directly.
    InversePredicate(TermSlot),
    /// `sh:inversePath` over a COMPOSITE path.
    ///
    /// Carried as the composite itself, which the evaluator inverts exactly as it
    /// always has. Inverting a composite is a rewrite of the path STRUCTURE rather
    /// than a resolution of a term, so hoisting it is a separate piece of work from
    /// this one; the variant is here so that hoist has somewhere to land.
    InverseComposite(Path),
    /// A sequence path — the frontier is folded through each step in order.
    Sequence(Box<[Self]>),
    /// An alternative path — the union of the branches.
    Alternative(Box<[Self]>),
    /// `sh:zeroOrMorePath` — the reflexive transitive closure of the inner path.
    ZeroOrMore(Box<Self>),
    /// `sh:oneOrMorePath` — the transitive closure of the inner path.
    OneOrMore(Box<Self>),
    /// `sh:zeroOrOnePath` — the inner path's values plus the focus node.
    ZeroOrOne(Box<Self>),
}

/// The stage-0 lowering of one constraint.
///
/// There is one variant per `Constraint` variant and the walk's match over them
/// carries no wildcard, so a new constraint kind does not COMPILE until it is
/// lowered. That is deliberate and it is the only guard that works here: a
/// constraint the walk forgot would stop constraining while every existing test
/// still passed.
///
/// Several kinds carry no payload, and that is a fact about SHACL rather than an
/// omission — `sh:minCount` is a number, `sh:datatype` is compared as an IRI
/// string, `sh:pattern` caches its own compiled regex on the AST node. Each still
/// has its OWN variant rather than sharing an "other" arm, because the variant is
/// what lets the evaluator prove the lowering it was handed describes the
/// constraint it is evaluating.
#[derive(Debug)]
pub(crate) enum LoweredConstraint {
    /// `sh:class` — the slot holding the class's dataset identity.
    Class(TermSlot),
    /// `sh:datatype` — compared as a datatype IRI string, never interned.
    Datatype,
    /// `sh:nodeKind` — a discriminant test on the value node.
    NodeKind,
    /// `sh:minCount` — a cardinality of the value-node set.
    MinCount,
    /// `sh:maxCount` — a cardinality of the value-node set.
    MaxCount,
    /// `sh:in` — the slot holding the allowed values' id set.
    In(SetSlot),
    /// `sh:hasValue` — the slot holding the required value's dataset identity.
    HasValue(TermSlot),
    /// `sh:pattern` — the compiled regex is cached on the constraint itself.
    Pattern,
    /// `sh:minLength` — a length of the value node's lexical form.
    MinLength,
    /// `sh:maxLength` — a length of the value node's lexical form.
    MaxLength,
    /// `sh:uniqueLang` — a comparison among the value nodes' language tags.
    UniqueLang,
    /// `sh:languageIn` — a comparison against literal language tags.
    LanguageIn,
    /// `sh:not` — the lowering of the negated shape.
    Not(Box<LoweredShape>),
    /// `sh:closed` — the permitted predicate set is still derived per focus node.
    ///
    /// Hoisting it is a rewrite of how the permitted set is COMPUTED (it is the
    /// union of the sibling property shapes' simple paths plus
    /// `sh:ignoredProperties`), so it is separate work from resolving a term; the
    /// variant is here so that hoist has somewhere to land.
    Closed,
    /// `sh:minInclusive` — the bound is parsed in the value space per comparison.
    MinInclusive,
    /// `sh:maxInclusive` — the bound is parsed in the value space per comparison.
    MaxInclusive,
    /// `sh:minExclusive` — the bound is parsed in the value space per comparison.
    MinExclusive,
    /// `sh:maxExclusive` — the bound is parsed in the value space per comparison.
    MaxExclusive,
    /// `sh:and` — the lowering of each conjunct, in declaration order.
    And(Box<[LoweredShape]>),
    /// `sh:or` — the lowering of each disjunct, in declaration order.
    Or(Box<[LoweredShape]>),
    /// `sh:xone` — the lowering of each alternative, in declaration order.
    Xone(Box<[LoweredShape]>),
    /// `sh:node` — the lowering of the referenced shape.
    Node(Box<LoweredShape>),
    /// `sh:sparql` — the query text is executed by the SPARQL engine.
    Sparql,
    /// `sh:equals` — the slot holding the compared predicate's dataset identity.
    Equals(TermSlot),
    /// `sh:disjoint` — the slot holding the compared predicate's dataset identity.
    Disjoint(TermSlot),
    /// `sh:lessThan` — the slot holding the compared predicate's dataset identity.
    LessThan(TermSlot),
    /// `sh:lessThanOrEquals` — the slot holding the compared predicate's identity.
    LessThanOrEquals(TermSlot),
    /// `sh:qualifiedValueShape` — the lowered qualified shape and its siblings.
    QualifiedValueShape {
        /// The lowering of the qualified value shape.
        shape: Box<LoweredShape>,
        /// The lowering of each sibling qualified value shape, in order.
        siblings: Box<[LoweredShape]>,
    },
    /// `sh:expression` — the lowered node expression.
    Expression(Box<LoweredExpr>),
    /// `sh:nodeByExpression` — the lowered node expression and its shape index.
    NodeByExpression {
        /// The lowering of the expression producing the shape IRIs.
        expr: Box<LoweredExpr>,
        /// The position of the lowered shape index in [`LoweredShapes::indexes`].
        index: u32,
    },
    /// A SHACL-SPARQL custom constraint component — the validator is a query.
    Component,
}

/// The stage-0 lowering of one node expression.
///
/// A UNIFORM node rather than a mirror of every `NodeExpr` variant: the expression
/// grammar is thirty arms wide and only four of them name anything a dataset can
/// resolve, so a one-to-one mirror would be thirty variants of which twenty-six
/// carried nothing but their operands. The walk's own match over `NodeExpr` is
/// still wildcard-free — that is where a new expression kind fails to compile —
/// and it fills these four slots in DECLARATION ORDER, which is the order the
/// evaluator reads them back in.
#[derive(Debug, Default)]
pub(crate) struct LoweredExpr {
    /// The slot holding the dataset identity of the class this expression selects
    /// the instances of (`shnex:instancesOf`); `None` for every other kind.
    class: Option<TermSlot>,
    /// The position of the shape index this expression resolves shape IRIs against
    /// (`shnex:conformsToShape` with a computed shape argument); `None` otherwise.
    index: Option<u32>,
    /// The IRI of the custom node-expression function this expression calls, whose
    /// lowered body lives in [`LoweredShapes::bodies`]; `None` otherwise.
    body: Option<String>,
    /// The lowering of each PATH this expression names, in declaration order.
    paths: Box<[LoweredPath]>,
    /// The lowering of each SHAPE this expression names, in declaration order.
    shapes: Box<[LoweredShape]>,
    /// The lowering of each SUB-EXPRESSION, in declaration order.
    operands: Box<[Self]>,
}

// ── Stage 1: the dataset binding ────────────────────────────────────────────────

/// Stage 0 × one dataset: every identity the lowering asked for, resolved once.
///
/// A row of arrays indexed by the slots the walk handed out. `None` in a slot means
/// this data graph interns no such term — an ordinary answer a shapes graph is
/// entitled to produce, read everywhere as "nothing can match".
#[derive(Debug)]
pub(crate) struct DatasetBinding {
    /// The dataset identity of every term the lowering named, by [`TermSlot`].
    terms: Box<[Option<TermId>]>,
    /// The dataset identity set of every set the lowering named, by [`SetSlot`].
    sets: Box<[FastSet<TermId>]>,
    /// The dataset identity of every catalogued class, by catalog position.
    ///
    /// Kept beside [`Self::terms`] rather than folded into it because the two have
    /// different CONSUMERS: the term row answers the constraint evaluator, and this
    /// row answers target resolution, which is given a class IRI by name out of a
    /// `Target` and has no slot to index with.
    class_ids: Box<[Option<TermId>]>,
}

impl DatasetBinding {
    /// The dataset identity of a planned class, by IRI.
    ///
    /// The two negative answers are DIFFERENT conditions and are deliberately
    /// spelled differently, because conflating them would turn an ordinary shapes
    /// graph into a refusal:
    ///
    /// * `Ok(None)` — the class IS planned, but this DATA GRAPH interns no term
    ///   for it. Entirely normal: a shapes graph may name a class the data never
    ///   mentions. Every caller reads it as "nothing is an instance", which is the
    ///   right answer, and it must never become an error.
    /// * `Err(_)` — the class is absent from the CATALOG, so the class-planning
    ///   walk failed to reach a class the evaluator went on to ask about. That is
    ///   a defect in the walk, never in the caller's data. It was an `expect`,
    ///   i.e. a panic — and a panic ABORTS across the PyO3 and C ABI boundaries,
    ///   where a host has no frame to catch it. Loud means an error.
    #[inline]
    pub(crate) fn class_id(
        &self,
        classes: &ClassCatalog,
        class: &NamedNode,
    ) -> Result<Option<TermId>, String> {
        let position = classes.position(class).ok_or_else(|| {
            format!(
                "internal validation-plan defect: the class <{}> is reached during evaluation but \
                 was never collected by the class-planning walk, so its dataset identity was \
                 never resolved",
                class.as_str()
            )
        })?;
        Ok(self.class_ids[position])
    }

    /// The dataset identity in `slot`.
    ///
    /// `Err` is the build defect — a slot the lowering never handed out — and
    /// `Ok(None)` is the ordinary "this data graph does not intern that term".
    ///
    /// # Errors
    /// Returns an error when the slot is not one the lowering allocated.
    #[inline]
    pub(crate) fn term(&self, slot: TermSlot) -> Result<Option<TermId>, String> {
        self.terms
            .get(slot as usize)
            .copied()
            .ok_or_else(|| slot_defect("term", slot))
    }

    /// The dataset identity set in `slot`.
    #[inline]
    fn set(&self, slot: SetSlot) -> Result<&FastSet<TermId>, String> {
        self.sets
            .get(slot as usize)
            .ok_or_else(|| slot_defect("id-set", slot))
    }
}

/// The message a slot the lowering never handed out is refused with.
#[cold]
fn slot_defect(kind: &str, slot: u32) -> String {
    format!(
        "internal validation-plan defect: the evaluator asked for {kind} slot {slot}, which the \
         shape lowering never allocated, so the constraint it belongs to was never lowered"
    )
}

/// The message a lowering that does not describe the constraint beside it is
/// refused with.
#[cold]
pub(crate) fn lowering_defect(constraint: &str, lowered: &impl std::fmt::Debug) -> String {
    format!(
        "internal validation-plan defect: the constraint {constraint} was paired with the \
         lowering {lowered:?}, which describes a different constraint kind, so the shape lowering \
         and the shape it lowered have diverged"
    )
}

// ── The plan handed to the evaluator ────────────────────────────────────────────

/// **Everything about a shape that does not depend on the focus node.**
///
/// That sentence is the type's cohesion invariant, and it is stated here rather
/// than in a commit message because it is the only thing that keeps this from
/// growing into a god-struct: a field belongs here if, and only if, it is constant
/// across every focus node the shape is evaluated at. Its structure (stage 0), the
/// dataset identities that structure named (stage 1), the class analysis and the
/// prepared target index all qualify. A value node, a path result, a partial report
/// — none of them do, and none of them may be added.
///
/// It is a borrowed CURSOR rather than an owned tree, and deliberately `Copy`: the
/// owned halves live for the whole bind, so descending into a nested `sh:node` or
/// `sh:and` shape must not cost an allocation, which is exactly the growth term
/// this whole exercise removes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapePlan<'a> {
    /// The shape this plan lowered, paired with its lowering by construction.
    shape: &'a Shape,
    /// This shape's stage-0 lowering.
    lowered: &'a LoweredShape,
    /// The whole shapes graph's stage-0 lowering, for the shape indexes and custom
    /// node-expression bodies a constraint can only resolve by name.
    graph: &'a LoweredShapes,
    /// This dataset's stage-1 identities.
    binding: &'a DatasetBinding,
    /// The class analysis, shared with the preparation that derived or carried it.
    classes: &'a ClassCatalog,
    /// This shape's stage-1 target index. Empty for a shape reached through a
    /// constraint: only a top-level node shape has targets to resolve.
    targets: &'a PreparedTargets,
}

impl<'a> ShapePlan<'a> {
    /// The shape this plan describes.
    #[inline]
    pub(crate) fn shape(&self) -> &'a Shape {
        self.shape
    }

    /// This shape's prepared target index.
    #[inline]
    pub(crate) fn targets(&self) -> &'a PreparedTargets {
        self.targets
    }

    /// The dataset binding behind this plan.
    #[inline]
    pub(crate) fn binding(&self) -> &'a DatasetBinding {
        self.binding
    }

    /// This shape's node-level constraints, each paired with its lowering.
    ///
    /// # Errors
    /// Returns an error when the shape and its lowering carry different numbers of
    /// constraints, which is a defect in the walk rather than in any input.
    pub(crate) fn constraints(
        &self,
    ) -> Result<impl Iterator<Item = (&'a Constraint, &'a LoweredConstraint)>, String> {
        pair(&self.shape.constraints, &self.lowered.constraints, "shape")
    }

    /// This shape's property shapes, each paired with its lowering.
    ///
    /// # Errors
    /// Returns an error when the shape and its lowering disagree on how many
    /// property shapes it has.
    pub(crate) fn properties(
        &self,
    ) -> Result<impl Iterator<Item = (&'a PropertyShape, PropertyPlan<'a>)>, String> {
        let plan = *self;
        Ok(pair(
            &self.shape.property_shapes,
            &self.lowered.properties,
            "shape",
        )?
        .map(move |(property, lowered)| (property, PropertyPlan { plan, lowered })))
    }

    /// The plan for a shape reached through one of this shape's constraints.
    #[inline]
    pub(crate) fn nested(&self, shape: &'a Shape, lowered: &'a LoweredShape) -> Self {
        Self {
            shape,
            lowered,
            graph: self.graph,
            binding: self.binding,
            classes: self.classes,
            // A shape reached through a constraint is never a target's shape, so it
            // has no target index of its own to carry.
            targets: &self.graph.no_targets,
        }
    }

    /// The plan for the shape a `sh:nodeByExpression` / `shnex:conformsToShape`
    /// index resolves `iri` to.
    ///
    /// `None` when the index holds no such shape, which the evaluator already
    /// treats as an unresolved shape IRI.
    ///
    /// # Errors
    /// Returns an error when the index position is not one the lowering handed out.
    pub(crate) fn indexed(
        &self,
        index: u32,
        iri: &str,
        shapes: &'a FastMap<String, Shape>,
    ) -> Result<Option<Self>, String> {
        let lowered = self
            .graph
            .indexes
            .get(index as usize)
            .ok_or_else(|| slot_defect("shape-index", index))?;
        let (Some(shape), Some(lowered)) = (shapes.get(iri), lowered.shapes.get(iri)) else {
            return Ok(None);
        };
        Ok(Some(self.nested(shape, lowered)))
    }

    /// The lowered body of the custom node-expression function named `iri`.
    #[inline]
    pub(crate) fn body(&self, iri: &str) -> Option<&'a LoweredExpr> {
        self.graph.bodies.get(iri)
    }
}

/// The focus-node-independent state of one property shape: its parent's plan plus
/// its own lowering.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PropertyPlan<'a> {
    plan: ShapePlan<'a>,
    lowered: &'a LoweredProperty,
}

impl<'a> PropertyPlan<'a> {
    /// This property shape's lowered path.
    #[inline]
    pub(crate) fn path(&self) -> &'a LoweredPath {
        &self.lowered.path
    }

    /// This property shape's constraints, each paired with its lowering.
    ///
    /// # Errors
    /// Returns an error when the property shape and its lowering disagree on how
    /// many constraints it has.
    pub(crate) fn constraints(
        &self,
        property: &'a PropertyShape,
    ) -> Result<impl Iterator<Item = (&'a Constraint, &'a LoweredConstraint)>, String> {
        pair(
            &property.constraints,
            &self.lowered.constraints,
            "property shape",
        )
    }

    /// The property shapes nested under this one, each paired with its lowering.
    ///
    /// # Errors
    /// Returns an error when the property shape and its lowering disagree on how
    /// many nested property shapes it has.
    pub(crate) fn properties(
        &self,
        property: &'a PropertyShape,
    ) -> Result<impl Iterator<Item = (&'a PropertyShape, Self)>, String> {
        let plan = self.plan;
        Ok(pair(
            &property.property_shapes,
            &self.lowered.properties,
            "property shape",
        )?
        .map(move |(nested, lowered)| (nested, Self { plan, lowered })))
    }

    /// The reifier shapes of this property shape, each paired with its lowering.
    ///
    /// # Errors
    /// Returns an error when the property shape and its lowering disagree on how
    /// many reifier shapes it has.
    pub(crate) fn reifiers(
        &self,
        property: &'a PropertyShape,
    ) -> Result<impl Iterator<Item = (&'a Shape, ShapePlan<'a>)>, String> {
        let plan = self.plan;
        Ok(pair(
            &property.reifier_shapes,
            &self.lowered.reifiers,
            "property shape",
        )?
        .map(move |(shape, lowered)| (shape, plan.nested(shape, lowered))))
    }
}

// ── The constraint the evaluator receives ───────────────────────────────────────

/// A list of shapes reached through one constraint, paired with its lowering.
///
/// Borrowed and `Copy`: `sh:and` over twenty shapes must not allocate a plan
/// vector once per focus node, which is exactly the growth term this lowering
/// removes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapeList<'a> {
    shapes: &'a [Shape],
    lowered: &'a [LoweredShape],
}

impl<'a> ShapeList<'a> {
    /// The plan for each shape of the list, in declaration order.
    #[inline]
    pub(crate) fn iter(&self, plan: ShapePlan<'a>) -> impl Iterator<Item = ShapePlan<'a>> {
        self.shapes
            .iter()
            .zip(self.lowered.iter())
            .map(move |(shape, lowered)| plan.nested(shape, lowered))
    }
}

/// ONE constraint, with every derivation it needs that does not depend on the
/// focus node already resolved.
///
/// This is what stage 2 is allowed to see. There is no dataset in it and no way to
/// reach one: an `sh:in` arrives as the interned id set, an `sh:class` as the
/// class's identity, a property-pair constraint as its predicate's identity. A
/// constraint evaluated against a million focus nodes resolves each of those
/// exactly once, at bind, because there is nothing left here to re-resolve them
/// FROM.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PlannedConstraint<'a> {
    /// `sh:class` — the class's dataset identity, resolved at bind.
    ///
    /// `None` means this data graph interns no term for the class, so nothing can
    /// be an instance of it and every value node violates. That is a verdict, not
    /// a failure to compute one.
    Class(Option<TermId>),
    /// `sh:datatype` — the required datatype IRI.
    Datatype(&'a NamedNode),
    /// `sh:nodeKind` — the required node kind.
    NodeKind(&'a NodeKindValue),
    /// `sh:minCount` — the minimum value-node count.
    MinCount(u64),
    /// `sh:maxCount` — the maximum value-node count.
    MaxCount(u64),
    /// `sh:in` — the allowed values and their interned identities.
    In {
        /// The declared allowed values, for the non-interned value-node fallback.
        allowed: &'a [Term],
        /// The interned identities of those values. A member this data graph does
        /// not intern is absent, which is correct: the interner is injective, so it
        /// could not have equalled an interned value node anyway, and the only
        /// value node it could still match falls back to term equality on
        /// [`Self::In::allowed`].
        ids: &'a FastSet<TermId>,
    },
    /// `sh:hasValue` — the required value and its identity.
    HasValue {
        /// The declared required value.
        required: &'a Term,
        /// Its dataset identity; `None` when this data graph interns no such term.
        id: Option<TermId>,
    },
    /// `sh:pattern` — the regex, its flags and the constraint's own compiled cache.
    Pattern {
        /// The regex source text.
        regex: &'a str,
        /// The optional regex flags.
        flags: Option<&'a String>,
        /// The once-compiled regex cache carried by the constraint itself.
        compiled: &'a OnceLock<
            Result<purrdf_core::xsd_regex::CompiledPattern, purrdf_core::xsd_regex::XsdRegexError>,
        >,
    },
    /// `sh:minLength` — the minimum lexical length.
    MinLength(u64),
    /// `sh:maxLength` — the maximum lexical length.
    MaxLength(u64),
    /// `sh:uniqueLang` — whether uniqueness of language tags is required.
    UniqueLang(bool),
    /// `sh:languageIn` — the permitted language tags.
    LanguageIn(&'a [String]),
    /// `sh:not` — the plan of the shape the node must NOT conform to.
    Not(ShapePlan<'a>),
    /// `sh:closed` — the explicitly exempted predicates.
    Closed {
        /// `sh:ignoredProperties`.
        ignored: &'a [NamedNode],
    },
    /// `sh:minInclusive` — the declared bound.
    MinInclusive(&'a Term),
    /// `sh:maxInclusive` — the declared bound.
    MaxInclusive(&'a Term),
    /// `sh:minExclusive` — the declared bound.
    MinExclusive(&'a Term),
    /// `sh:maxExclusive` — the declared bound.
    MaxExclusive(&'a Term),
    /// `sh:and` — the plans of the conjuncts.
    And(ShapeList<'a>),
    /// `sh:or` — the plans of the disjuncts.
    Or(ShapeList<'a>),
    /// `sh:xone` — the plans of the alternatives.
    Xone(ShapeList<'a>),
    /// `sh:node` — the plan of the referenced shape.
    Node(ShapePlan<'a>),
    /// `sh:sparql` — the query text and its overrides.
    Sparql {
        /// The SPARQL SELECT query text.
        select: &'a str,
        /// The per-constraint message override.
        message: &'a Option<String>,
        /// The per-constraint severity override.
        severity: &'a Option<crate::report::Severity>,
    },
    /// `sh:equals` — the compared predicate's dataset identity.
    Equals(Option<TermId>),
    /// `sh:disjoint` — the compared predicate's dataset identity.
    Disjoint(Option<TermId>),
    /// `sh:lessThan` — the compared predicate's dataset identity.
    LessThan(Option<TermId>),
    /// `sh:lessThanOrEquals` — the compared predicate's dataset identity.
    LessThanOrEquals(Option<TermId>),
    /// `sh:qualifiedValueShape` — the qualified shape's plan and its counts.
    QualifiedValueShape {
        /// The plan of the qualified value shape.
        shape: ShapePlan<'a>,
        /// The plans of the sibling qualified value shapes.
        siblings: ShapeList<'a>,
        /// `sh:qualifiedMinCount`.
        min_count: Option<u64>,
        /// `sh:qualifiedMaxCount`.
        max_count: Option<u64>,
        /// `sh:qualifiedValueShapesDisjoint`.
        disjoint: bool,
    },
    /// `sh:expression` — the expression, its lowering and its overrides.
    Expression {
        /// The declared node expression.
        expr: &'a NodeExpr,
        /// Its lowering.
        lowered: &'a LoweredExpr,
        /// The per-constraint message override.
        message: &'a Option<String>,
        /// The per-constraint severity override.
        severity: &'a Option<crate::report::Severity>,
    },
    /// `sh:nodeByExpression` — the expression, its lowering, and its shape index.
    NodeByExpression {
        /// The declared node expression producing shape IRIs.
        expr: &'a NodeExpr,
        /// Its lowering.
        lowered: &'a LoweredExpr,
        /// The shapes graph's IRI-named shape index.
        shapes: &'a Arc<OnceLock<FastMap<String, Shape>>>,
        /// The position of that index's lowering.
        index: u32,
        /// The per-constraint message override.
        message: &'a Option<String>,
        /// The per-constraint severity override.
        severity: &'a Option<crate::report::Severity>,
    },
    /// A SHACL-SPARQL custom constraint component usage.
    Component {
        /// The component IRI.
        component: &'a NamedNode,
        /// The shape node that sourced this usage.
        source_shape: &'a Term,
        /// The parameter bindings.
        bindings: &'a [(String, Term)],
        /// The selected validator.
        validator: &'a ComponentValidator,
        /// The message override.
        message: &'a Option<String>,
        /// The severity override.
        severity: &'a Option<crate::report::Severity>,
    },
}

impl<'a> ShapePlan<'a> {
    /// Pair one constraint with its lowering, resolving every stage-1 derivation.
    ///
    /// This is the ONE place the declaration and the lowering are matched against
    /// each other, and the match is total over both: a lowering that does not
    /// describe the constraint beside it is refused loudly, because the two are
    /// produced by one walk and consumed by index, so a disagreement is a defect in
    /// this crate rather than in any input.
    ///
    /// # Errors
    /// Returns an error when the lowering does not describe the constraint, or when
    /// it names a slot the walk never handed out.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per SHACL constraint kind: the list is the point, and splitting it \
                  would hide which kinds are covered"
    )]
    pub(crate) fn planned(
        &self,
        constraint: &'a Constraint,
        lowered: &'a LoweredConstraint,
    ) -> Result<PlannedConstraint<'a>, String> {
        Ok(match (constraint, lowered) {
            (Constraint::Class(_), LoweredConstraint::Class(slot)) => {
                PlannedConstraint::Class(self.binding.term(*slot)?)
            }
            (Constraint::Datatype(datatype), LoweredConstraint::Datatype) => {
                PlannedConstraint::Datatype(datatype)
            }
            (Constraint::NodeKind(kind), LoweredConstraint::NodeKind) => {
                PlannedConstraint::NodeKind(kind)
            }
            (Constraint::MinCount(n), LoweredConstraint::MinCount) => {
                PlannedConstraint::MinCount(*n)
            }
            (Constraint::MaxCount(n), LoweredConstraint::MaxCount) => {
                PlannedConstraint::MaxCount(*n)
            }
            (Constraint::In(allowed), LoweredConstraint::In(slot)) => PlannedConstraint::In {
                allowed,
                ids: self.binding.set(*slot)?,
            },
            (Constraint::HasValue(required), LoweredConstraint::HasValue(slot)) => {
                PlannedConstraint::HasValue {
                    required,
                    id: self.binding.term(*slot)?,
                }
            }
            (
                Constraint::Pattern {
                    regex,
                    flags,
                    compiled,
                },
                LoweredConstraint::Pattern,
            ) => PlannedConstraint::Pattern {
                regex,
                flags: flags.as_ref(),
                compiled,
            },
            (Constraint::MinLength(n), LoweredConstraint::MinLength) => {
                PlannedConstraint::MinLength(*n)
            }
            (Constraint::MaxLength(n), LoweredConstraint::MaxLength) => {
                PlannedConstraint::MaxLength(*n)
            }
            (Constraint::UniqueLang(unique), LoweredConstraint::UniqueLang) => {
                PlannedConstraint::UniqueLang(*unique)
            }
            (Constraint::LanguageIn(tags), LoweredConstraint::LanguageIn) => {
                PlannedConstraint::LanguageIn(tags)
            }
            (Constraint::Not(shape), LoweredConstraint::Not(lowered)) => {
                PlannedConstraint::Not(self.nested(shape, lowered))
            }
            (Constraint::Closed { ignored }, LoweredConstraint::Closed) => {
                PlannedConstraint::Closed { ignored }
            }
            (Constraint::MinInclusive(bound), LoweredConstraint::MinInclusive) => {
                PlannedConstraint::MinInclusive(bound)
            }
            (Constraint::MaxInclusive(bound), LoweredConstraint::MaxInclusive) => {
                PlannedConstraint::MaxInclusive(bound)
            }
            (Constraint::MinExclusive(bound), LoweredConstraint::MinExclusive) => {
                PlannedConstraint::MinExclusive(bound)
            }
            (Constraint::MaxExclusive(bound), LoweredConstraint::MaxExclusive) => {
                PlannedConstraint::MaxExclusive(bound)
            }
            (Constraint::And(shapes), LoweredConstraint::And(lowered)) => {
                PlannedConstraint::And(shape_list(shapes, lowered)?)
            }
            (Constraint::Or(shapes), LoweredConstraint::Or(lowered)) => {
                PlannedConstraint::Or(shape_list(shapes, lowered)?)
            }
            (Constraint::Xone(shapes), LoweredConstraint::Xone(lowered)) => {
                PlannedConstraint::Xone(shape_list(shapes, lowered)?)
            }
            (Constraint::Node(shape), LoweredConstraint::Node(lowered)) => {
                PlannedConstraint::Node(self.nested(shape, lowered))
            }
            (
                Constraint::Sparql {
                    select,
                    message,
                    severity,
                },
                LoweredConstraint::Sparql,
            ) => PlannedConstraint::Sparql {
                select,
                message,
                severity,
            },
            (Constraint::Equals(_), LoweredConstraint::Equals(slot)) => {
                PlannedConstraint::Equals(self.binding.term(*slot)?)
            }
            (Constraint::Disjoint(_), LoweredConstraint::Disjoint(slot)) => {
                PlannedConstraint::Disjoint(self.binding.term(*slot)?)
            }
            (Constraint::LessThan(_), LoweredConstraint::LessThan(slot)) => {
                PlannedConstraint::LessThan(self.binding.term(*slot)?)
            }
            (Constraint::LessThanOrEquals(_), LoweredConstraint::LessThanOrEquals(slot)) => {
                PlannedConstraint::LessThanOrEquals(self.binding.term(*slot)?)
            }
            (
                Constraint::QualifiedValueShape {
                    shape,
                    siblings,
                    min_count,
                    max_count,
                    disjoint,
                },
                LoweredConstraint::QualifiedValueShape {
                    shape: lowered_shape,
                    siblings: lowered_siblings,
                },
            ) => PlannedConstraint::QualifiedValueShape {
                shape: self.nested(shape, lowered_shape),
                siblings: shape_list(siblings, lowered_siblings)?,
                min_count: *min_count,
                max_count: *max_count,
                disjoint: *disjoint,
            },
            (
                Constraint::Expression {
                    expr,
                    message,
                    severity,
                },
                LoweredConstraint::Expression(lowered),
            ) => PlannedConstraint::Expression {
                expr,
                lowered,
                message,
                severity,
            },
            (
                Constraint::NodeByExpression {
                    expr,
                    shapes,
                    message,
                    severity,
                },
                LoweredConstraint::NodeByExpression {
                    expr: lowered,
                    index,
                },
            ) => PlannedConstraint::NodeByExpression {
                expr,
                lowered,
                shapes,
                index: *index,
                message,
                severity,
            },
            (
                Constraint::Component {
                    component,
                    source_shape,
                    bindings,
                    validator,
                    message,
                    severity,
                },
                LoweredConstraint::Component,
            ) => PlannedConstraint::Component {
                component,
                source_shape,
                bindings,
                validator,
                message,
                severity,
            },
            (constraint, lowered) => {
                return Err(lowering_defect(constraint_kind(constraint), lowered));
            }
        })
    }
}

/// Pair a shape list with its lowering, refusing a length mismatch loudly.
fn shape_list<'a>(
    shapes: &'a [Shape],
    lowered: &'a [LoweredShape],
) -> Result<ShapeList<'a>, String> {
    if shapes.len() != lowered.len() {
        return Err(format!(
            "internal validation-plan defect: a constraint declares {} shape(s) but its lowering \
             holds {}, so the shape lowering and the shape it lowered have diverged",
            shapes.len(),
            lowered.len()
        ));
    }
    Ok(ShapeList { shapes, lowered })
}

/// The SHACL spelling of a constraint kind, for a plan-defect message.
///
/// Wildcard-free so a new constraint kind has to be named here too: a defect
/// message that could not say WHICH constraint diverged would send a reader back
/// to the shapes graph rather than to the walk.
fn constraint_kind(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::Class(_) => "sh:class",
        Constraint::Datatype(_) => "sh:datatype",
        Constraint::NodeKind(_) => "sh:nodeKind",
        Constraint::MinCount(_) => "sh:minCount",
        Constraint::MaxCount(_) => "sh:maxCount",
        Constraint::In(_) => "sh:in",
        Constraint::HasValue(_) => "sh:hasValue",
        Constraint::Pattern { .. } => "sh:pattern",
        Constraint::MinLength(_) => "sh:minLength",
        Constraint::MaxLength(_) => "sh:maxLength",
        Constraint::UniqueLang(_) => "sh:uniqueLang",
        Constraint::LanguageIn(_) => "sh:languageIn",
        Constraint::Not(_) => "sh:not",
        Constraint::Closed { .. } => "sh:closed",
        Constraint::MinInclusive(_) => "sh:minInclusive",
        Constraint::MaxInclusive(_) => "sh:maxInclusive",
        Constraint::MinExclusive(_) => "sh:minExclusive",
        Constraint::MaxExclusive(_) => "sh:maxExclusive",
        Constraint::And(_) => "sh:and",
        Constraint::Or(_) => "sh:or",
        Constraint::Xone(_) => "sh:xone",
        Constraint::Node(_) => "sh:node",
        Constraint::Sparql { .. } => "sh:sparql",
        Constraint::Equals(_) => "sh:equals",
        Constraint::Disjoint(_) => "sh:disjoint",
        Constraint::LessThan(_) => "sh:lessThan",
        Constraint::LessThanOrEquals(_) => "sh:lessThanOrEquals",
        Constraint::QualifiedValueShape { .. } => "sh:qualifiedValueShape",
        Constraint::Expression { .. } => "sh:expression",
        Constraint::NodeByExpression { .. } => "sh:nodeByExpression",
        Constraint::Component { .. } => "a SHACL-SPARQL constraint component",
    }
}

// ── The lowered node expression, as the expression evaluator reads it ───────────

impl LoweredExpr {
    /// The slot holding the identity of the class this expression selects the
    /// instances of.
    ///
    /// # Errors
    /// Returns an error when the lowering beside a `shnex:instancesOf` expression
    /// records no class, which is a defect in the walk.
    #[inline]
    pub(crate) fn class_id(&self, plan: ShapePlan<'_>) -> Result<Option<TermId>, String> {
        let slot = self.class.ok_or_else(|| {
            "internal validation-plan defect: a shnex:instancesOf expression was evaluated \
             against a lowering that records no class, so its class identity was never resolved"
                .to_owned()
        })?;
        plan.binding.term(slot)
    }

    /// The `position`-th path this expression names.
    ///
    /// # Errors
    /// Returns an error when the lowering holds no such path.
    #[inline]
    pub(crate) fn path(&self, position: usize) -> Result<&LoweredPath, String> {
        self.paths
            .get(position)
            .ok_or_else(|| expr_defect("path", position, self.paths.len()))
    }

    /// The `position`-th shape this expression names, as a plan under `plan`.
    ///
    /// # Errors
    /// Returns an error when the lowering holds no such shape.
    #[inline]
    pub(crate) fn shape<'a>(
        &'a self,
        plan: ShapePlan<'a>,
        position: usize,
        shape: &'a Shape,
    ) -> Result<ShapePlan<'a>, String> {
        let lowered = self
            .shapes
            .get(position)
            .ok_or_else(|| expr_defect("shape", position, self.shapes.len()))?;
        Ok(plan.nested(shape, lowered))
    }

    /// The `position`-th sub-expression's lowering.
    ///
    /// # Errors
    /// Returns an error when the lowering holds no such operand.
    #[inline]
    pub(crate) fn operand(&self, position: usize) -> Result<&Self, String> {
        self.operands
            .get(position)
            .ok_or_else(|| expr_defect("operand", position, self.operands.len()))
    }

    /// The position of the shape index this expression resolves IRIs against.
    ///
    /// # Errors
    /// Returns an error when the lowering records none.
    #[inline]
    pub(crate) fn index(&self) -> Result<u32, String> {
        self.index.ok_or_else(|| {
            "internal validation-plan defect: a computed shape argument was evaluated against a \
             lowering that records no shape index, so the shapes it resolves to were never lowered"
                .to_owned()
        })
    }

    /// The IRI of the custom node-expression function this expression calls.
    ///
    /// # Errors
    /// Returns an error when the lowering records none.
    #[inline]
    pub(crate) fn body_iri(&self) -> Result<&str, String> {
        self.body.as_deref().ok_or_else(|| {
            "internal validation-plan defect: a custom node-expression call was evaluated against \
             a lowering that records no function IRI, so its body was never lowered"
                .to_owned()
        })
    }
}

/// The message an expression lowering that is short of an operand is refused with.
#[cold]
fn expr_defect(what: &str, position: usize, held: usize) -> String {
    format!(
        "internal validation-plan defect: a node expression asked for {what} {position} of a \
         lowering holding {held}, so the expression and its lowering have diverged"
    )
}

/// Pair an AST slice with its lowering, refusing a length mismatch loudly.
///
/// A mismatch is a defect in the walk — the two are produced together and consumed
/// by index — so it is an `Err` rather than a truncating `zip`, which would silently
/// stop evaluating the constraints past the shorter of the two.
fn pair<'a, A, B>(
    ast: &'a [A],
    lowered: &'a [B],
    what: &str,
) -> Result<impl Iterator<Item = (&'a A, &'a B)>, String> {
    if ast.len() != lowered.len() {
        return Err(format!(
            "internal validation-plan defect: a {what} declares {} item(s) but its lowering holds \
             {}, so the shape lowering and the shape it lowered have diverged",
            ast.len(),
            lowered.len()
        ));
    }
    Ok(ast.iter().zip(lowered.iter()))
}

// ── Prepared targets ────────────────────────────────────────────────────────────

/// Dataset-bound target predicates used by a prepared validator.
///
/// Core targets are retained as compact membership indexes instead of eagerly
/// expanding every target node. A bounded request can therefore test only its
/// supplied candidates. Explicit and SHACL-SPARQL target results are resolved
/// once at preparation because they cannot be answered through a Core pattern
/// lookup.
#[derive(Debug, Default)]
pub(crate) struct PreparedTargets {
    pub(crate) explicit_ids: IdSet,
    pub(crate) explicit_foreign: FastSet<Term>,
    pub(crate) target_class_ids: IdSet,
    pub(crate) subject_predicates: IdSet,
    pub(crate) object_predicates: IdSet,
}

// ── The walk ────────────────────────────────────────────────────────────────────

/// The accumulator the one total shape walk carries.
///
/// It is a struct rather than a bare set because the walk has to be cycle-safe: a
/// custom node-expression function's `sh:bodyExpression` may CALL the very function
/// it belongs to (SHACL 1.2 Node Expressions §6.1/§6.2 hold the body by reference
/// precisely so that is expressible), so the IR genuinely contains a cycle. Walking
/// it without a record of the bodies already entered would recurse until the native
/// stack was exhausted — an ABORT, not an error any caller could handle.
#[derive(Default)]
struct ShapeWalk {
    /// Every class IRI the walk has reached.
    classes: FastSet<NamedNode>,
    /// Every term whose dataset identity a lowered node will need, in slot order.
    terms: Vec<Term>,
    /// Every id set a lowered node will need, as a range over [`Self::terms`].
    sets: Vec<std::ops::Range<u32>>,
    /// The `sh:nodeByExpression` / `shnex:conformsToShape` shape indexes already
    /// walked, in the order their positions were handed out.
    ///
    /// A shapes graph has exactly one such index, shared by `Arc` across every
    /// constraint that resolves against it — including the constraints of the
    /// shapes the index itself holds. Walking it a second time from one of those
    /// would not terminate, so it is entered once per walk and every later
    /// constraint is handed the position of the entry already made.
    indexes: Vec<LoweredIndex>,
    /// The custom node-expression function bodies already lowered, by IRI. A body
    /// names the same classes and resolves the same terms at every call site, so
    /// once is enough — and once is also all a cyclic body can be given.
    bodies: FastMap<String, LoweredExpr>,
    /// The function IRIs whose bodies are currently being lowered, so a body that
    /// calls its own function records the call and stops rather than recursing.
    in_flight: Vec<String>,
}

impl ShapeWalk {
    /// Record `term` and hand back the slot its dataset identity will occupy.
    fn slot(&mut self, term: Term) -> TermSlot {
        let slot = self.terms.len();
        self.terms.push(term);
        // The row is indexed by a `u32`, so a shapes graph naming more than four
        // billion distinct constants would silently alias two of them. No parse can
        // reach that from any input a machine can hold, and the alternative to
        // saying so is an index type that costs eight bytes in every lowered node.
        debug_assert!(
            u32::try_from(slot).is_ok(),
            "term slot row overflowed a u32"
        );
        slot as TermSlot
    }

    /// Record an id SET over `terms` and hand back its slot.
    fn set_slot(&mut self, terms: impl IntoIterator<Item = Term>) -> SetSlot {
        let start = self.terms.len() as u32;
        self.terms.extend(terms);
        let end = self.terms.len() as u32;
        let slot = self.sets.len();
        self.sets.push(start..end);
        slot as SetSlot
    }

    /// Record `class` as a planned class AND hand back the slot its own dataset
    /// identity will occupy.
    ///
    /// Both, from one call, because the two answers have different consumers and
    /// deriving them separately is precisely how the two would come to disagree:
    /// the catalog answers target resolution by IRI, the slot answers the
    /// constraint evaluator by index, and §8's binding test exists to keep them
    /// honest about being one walk's output.
    fn class_slot(&mut self, class: &NamedNode) -> TermSlot {
        self.classes.insert(class.clone());
        self.slot(Term::NamedNode(class.clone()))
    }

    /// Seal the walk into the lowering it produced.
    fn finish(self, shapes: Vec<LoweredShape>) -> LoweredShapes {
        LoweredShapes {
            terms: self.terms.into_boxed_slice(),
            sets: self.sets.into_boxed_slice(),
            shapes: shapes.into_boxed_slice(),
            classes: Arc::new(ClassCatalog::from_walk(self.classes)),
            indexes: self.indexes.into_boxed_slice(),
            bodies: self.bodies,
            no_targets: PreparedTargets::default(),
        }
    }

    /// The position of the lowered form of `index`, entering it exactly once.
    fn index_position(&mut self, index: &Arc<OnceLock<FastMap<String, Shape>>>) -> u32 {
        let cell = Arc::as_ptr(index).addr();
        if let Some(position) = self
            .indexes
            .iter()
            .position(|entry| Arc::as_ptr(&entry.cell).addr() == cell)
        {
            return position as u32;
        }
        // Claim the position BEFORE lowering the shapes inside, so a shape held by
        // the index that carries a constraint resolving against the same index
        // finds the entry already made instead of entering it again.
        let position = self.indexes.len() as u32;
        self.indexes.push(LoweredIndex {
            cell: Arc::clone(index),
            shapes: FastMap::default(),
        });
        // An unfilled index resolves nothing: the constraint refuses loudly at
        // evaluation rather than silently conforming, so there is no shape here
        // whose classes could go unplanned.
        if let Some(shapes) = index.get() {
            let lowered: FastMap<String, LoweredShape> = shapes
                .iter()
                .map(|(iri, shape)| (iri.clone(), lower_shape(shape, self)))
                .collect();
            self.indexes[position as usize].shapes = lowered;
        }
        position
    }
}

/// THE WALK: lower every shape in `shapes`, collecting the class references and the
/// dataset identities the lowering will need in ONE traversal.
///
/// This is the single derivation that used to be a class scan and a per-focus-node
/// re-resolution. Both answers come out of the same visit to each node, so a class
/// IRI is reached once and a constraint constant is recorded once.
pub(crate) fn lower_shapes<'a>(shapes: impl IntoIterator<Item = &'a Shape>) -> LoweredShapes {
    let mut walk = ShapeWalk::default();
    let lowered: Vec<LoweredShape> = shapes
        .into_iter()
        .map(|shape| lower_shape(shape, &mut walk))
        .collect();
    walk.finish(lowered)
}

/// THE STANDALONE WALK: lower ONE node expression that has no place in a shapes
/// graph, for the evaluation-time expressions that cannot have been lowered before
/// they existed.
///
/// Two expressions are genuinely of that kind, and both are dynamic by
/// construction rather than by omission: a `shnex:arg` argument, which travels in
/// the evaluation SCOPE (a frozen public type that carries an argument's
/// DECLARATION and not its lowering), and the synthetic call a SPARQL-invoked
/// custom function is built from at query-evaluation time out of values the engine
/// has just computed. Lowering them where they are built is the only placement
/// there is.
pub(crate) fn lower_standalone_expression(expr: &NodeExpr) -> StandaloneLowering {
    let mut walk = ShapeWalk::default();
    let lowered = lower_expression(expr, &mut walk);
    StandaloneLowering {
        graph: walk.finish(Vec::new()),
        expr: lowered,
        root: standalone_root(),
        root_lowered: LoweredShape {
            constraints: Box::new([]),
            properties: Box::new([]),
        },
    }
}

/// The lowering of one standalone node expression, with the values a
/// [`ShapePlan`] borrows.
#[derive(Debug)]
pub(crate) struct StandaloneLowering {
    graph: LoweredShapes,
    expr: LoweredExpr,
    /// The shape a standalone expression's plan is ROOTED on.
    ///
    /// A node expression is not a shape, and this one declares nothing — no
    /// target, no constraint, no property. It exists because a [`ShapePlan`] is
    /// the plan OF a shape, and the expression evaluator needs one in hand to
    /// descend into the shapes its own expression names. Nothing reads it: every
    /// shape the evaluation reaches arrives through
    /// [`ShapePlan::nested`], which replaces it.
    root: Shape,
    root_lowered: LoweredShape,
}

impl StandaloneLowering {
    /// Resolve this lowering against `dataset`.
    pub(crate) fn bind(&self, dataset: &impl ShaclRead) -> DatasetBinding {
        self.graph.bind(dataset, &self.graph.classes)
    }

    /// The expression's own lowering.
    #[inline]
    pub(crate) fn expr(&self) -> &LoweredExpr {
        &self.expr
    }

    /// The plan an evaluation of this expression runs under.
    #[inline]
    pub(crate) fn plan<'a>(&'a self, binding: &'a DatasetBinding) -> ShapePlan<'a> {
        ShapePlan {
            shape: &self.root,
            lowered: &self.root_lowered,
            graph: &self.graph,
            binding,
            classes: &self.graph.classes,
            targets: &self.graph.no_targets,
        }
    }
}

/// The shape a standalone node-expression plan is rooted on — see
/// [`StandaloneLowering::root`].
fn standalone_root() -> Shape {
    Shape {
        id: Term::BlankNode("standalone-node-expression".to_owned()),
        targets: Vec::new(),
        constraints: Vec::new(),
        property_shapes: Vec::new(),
        severity: crate::report::Severity::Violation,
        message: None,
        deactivated: false,
        box_roles: Vec::new(),
        rules: Vec::new(),
    }
}

/// Lower one shape: its targets' classes, its own constraints and its property
/// shapes.
fn lower_shape(shape: &Shape, walk: &mut ShapeWalk) -> LoweredShape {
    for target in &shape.targets {
        match target {
            Target::Class(class) | Target::ImplicitClass(Term::NamedNode(class)) => {
                walk.classes.insert(class.clone());
            }
            Target::SubjectsOf(_)
            | Target::ObjectsOf(_)
            | Target::Node(_)
            | Target::ImplicitClass(_)
            | Target::Sparql { .. } => {}
        }
    }
    LoweredShape {
        constraints: lower_constraints(&shape.constraints, walk),
        properties: shape
            .property_shapes
            .iter()
            .map(|property| lower_property(property, walk))
            .collect(),
    }
    // `shape.rules` is deliberately NOT descended into. A rule's own shapes are
    // planned by the rules engine, against a plan it builds for the rule's shape
    // and conditions together, so reaching them from here would put classes into
    // the catalog a prepared product's identity does not describe.
}

/// Lower one property shape: its path, its constraints, its nested property shapes
/// and its reifier shapes.
fn lower_property(property: &PropertyShape, walk: &mut ShapeWalk) -> LoweredProperty {
    LoweredProperty {
        path: lower_path(&property.path, walk),
        constraints: lower_constraints(&property.constraints, walk),
        properties: property
            .property_shapes
            .iter()
            .map(|nested| lower_property(nested, walk))
            .collect(),
        reifiers: property
            .reifier_shapes
            .iter()
            .map(|reifier| lower_shape(reifier, walk))
            .collect(),
    }
}

/// Lower one SHACL property path, resolving every predicate step to a slot.
fn lower_path(path: &Path, walk: &mut ShapeWalk) -> LoweredPath {
    match path {
        Path::Predicate(predicate) => {
            LoweredPath::Predicate(walk.slot(Term::NamedNode(predicate.clone())))
        }
        Path::Inverse(inner) => match inner.as_ref() {
            Path::Predicate(predicate) => {
                LoweredPath::InversePredicate(walk.slot(Term::NamedNode(predicate.clone())))
            }
            composite => LoweredPath::InverseComposite(composite.clone()),
        },
        Path::Sequence(parts) => {
            LoweredPath::Sequence(parts.iter().map(|part| lower_path(part, walk)).collect())
        }
        Path::Alternative(parts) => {
            LoweredPath::Alternative(parts.iter().map(|part| lower_path(part, walk)).collect())
        }
        Path::ZeroOrMore(inner) => LoweredPath::ZeroOrMore(Box::new(lower_path(inner, walk))),
        Path::OneOrMore(inner) => LoweredPath::OneOrMore(Box::new(lower_path(inner, walk))),
        Path::ZeroOrOne(inner) => LoweredPath::ZeroOrOne(Box::new(lower_path(inner, walk))),
    }
}

/// Lower a constraint list, in declaration order.
fn lower_constraints(constraints: &[Constraint], walk: &mut ShapeWalk) -> Box<[LoweredConstraint]> {
    constraints
        .iter()
        .map(|constraint| lower_constraint(constraint, walk))
        .collect()
}

/// Lower ONE constraint.
///
/// The match carries no wildcard on purpose: this is the list of every SHACL
/// constraint kind, and a new one has to be added here before the crate compiles.
/// A kind that were silently omitted would evaluate against a lowering that did not
/// describe it, and the evaluator would refuse it — a constraint that stopped
/// constraining, with every existing test still green.
fn lower_constraint(constraint: &Constraint, walk: &mut ShapeWalk) -> LoweredConstraint {
    match constraint {
        Constraint::Class(class) => LoweredConstraint::Class(walk.class_slot(class)),
        Constraint::Datatype(_) => LoweredConstraint::Datatype,
        Constraint::NodeKind(_) => LoweredConstraint::NodeKind,
        Constraint::MinCount(_) => LoweredConstraint::MinCount,
        Constraint::MaxCount(_) => LoweredConstraint::MaxCount,
        Constraint::In(allowed) => LoweredConstraint::In(walk.set_slot(allowed.iter().cloned())),
        Constraint::HasValue(required) => LoweredConstraint::HasValue(walk.slot(required.clone())),
        Constraint::Pattern { .. } => LoweredConstraint::Pattern,
        Constraint::MinLength(_) => LoweredConstraint::MinLength,
        Constraint::MaxLength(_) => LoweredConstraint::MaxLength,
        Constraint::UniqueLang(_) => LoweredConstraint::UniqueLang,
        Constraint::LanguageIn(_) => LoweredConstraint::LanguageIn,
        Constraint::Not(shape) => LoweredConstraint::Not(Box::new(lower_shape(shape, walk))),
        Constraint::Closed { .. } => LoweredConstraint::Closed,
        Constraint::MinInclusive(_) => LoweredConstraint::MinInclusive,
        Constraint::MaxInclusive(_) => LoweredConstraint::MaxInclusive,
        Constraint::MinExclusive(_) => LoweredConstraint::MinExclusive,
        Constraint::MaxExclusive(_) => LoweredConstraint::MaxExclusive,
        Constraint::And(shapes) => LoweredConstraint::And(lower_shape_list(shapes, walk)),
        Constraint::Or(shapes) => LoweredConstraint::Or(lower_shape_list(shapes, walk)),
        Constraint::Xone(shapes) => LoweredConstraint::Xone(lower_shape_list(shapes, walk)),
        Constraint::Node(shape) => LoweredConstraint::Node(Box::new(lower_shape(shape, walk))),
        Constraint::Sparql { .. } => LoweredConstraint::Sparql,
        Constraint::Equals(predicate) => {
            LoweredConstraint::Equals(walk.slot(Term::NamedNode(predicate.clone())))
        }
        Constraint::Disjoint(predicate) => {
            LoweredConstraint::Disjoint(walk.slot(Term::NamedNode(predicate.clone())))
        }
        Constraint::LessThan(predicate) => {
            LoweredConstraint::LessThan(walk.slot(Term::NamedNode(predicate.clone())))
        }
        Constraint::LessThanOrEquals(predicate) => {
            LoweredConstraint::LessThanOrEquals(walk.slot(Term::NamedNode(predicate.clone())))
        }
        Constraint::QualifiedValueShape {
            shape, siblings, ..
        } => LoweredConstraint::QualifiedValueShape {
            shape: Box::new(lower_shape(shape, walk)),
            siblings: lower_shape_list(siblings, walk),
        },
        Constraint::Expression { expr, .. } => {
            LoweredConstraint::Expression(Box::new(lower_expression(expr, walk)))
        }
        // `sh:nodeByExpression` (Node Expressions §7.2) judges each value node
        // against shapes the expression NAMES BY IRI, resolved per value node
        // against the shared shape index. The expression walk reaches the
        // IRI-producing computation, never the shapes it lands on, so the index
        // has to be entered here or a plan built for a single shape — the one
        // `conforms` and `conforms_with_depth` build — would leave the resolved
        // shape unlowered.
        Constraint::NodeByExpression { expr, shapes, .. } => {
            let expr = Box::new(lower_expression(expr, walk));
            LoweredConstraint::NodeByExpression {
                expr,
                index: walk.index_position(shapes),
            }
        }
        Constraint::Component { .. } => LoweredConstraint::Component,
    }
}

/// Lower a list of shapes reached through one constraint, in declaration order.
fn lower_shape_list(shapes: &[Shape], walk: &mut ShapeWalk) -> Box<[LoweredShape]> {
    shapes
        .iter()
        .map(|shape| lower_shape(shape, walk))
        .collect()
}

/// Lower ONE node expression.
///
/// The match carries no wildcard for the same reason [`lower_constraint`]'s does
/// not. Only four expression kinds name something a dataset can resolve; every
/// other arm exists to reach its operands, and the ones that reach a SHAPE are the
/// ones that would otherwise leave an `sh:class` inside a `sh:filterShape`
/// unplanned.
fn lower_expression(expr: &NodeExpr, walk: &mut ShapeWalk) -> LoweredExpr {
    match expr {
        NodeExpr::Constant(_)
        | NodeExpr::This
        | NodeExpr::Empty
        | NodeExpr::Var(_)
        | NodeExpr::List(_)
        // A SPARQL-based node expression (SPARQL Extensions §6.1/§6.2) names its
        // classes inside opaque query TEXT, which this walk does not read. The
        // membership view it would want is the SPARQL engine's own, not the
        // validation plan's, so there is nothing here to pre-resolve.
        | NodeExpr::Select { .. }
        // `shnex:arg` (§6.3) resolves to an argument expression bound at the call
        // site, which this walk already visited there.
        | NodeExpr::Arg(_) => LoweredExpr::default(),
        NodeExpr::Path(path) => LoweredExpr {
            paths: Box::new([lower_path(path, walk)]),
            ..LoweredExpr::default()
        },
        NodeExpr::PathValues { focus, path } => LoweredExpr {
            paths: Box::new([lower_path(path, walk)]),
            operands: Box::new([lower_expression(focus, walk)]),
            ..LoweredExpr::default()
        },
        // `shnex:instancesOf` (Node Expressions §4.5.1) selects the SHACL instances
        // of a class, so that class must be resolved in the validation plan exactly
        // like an `sh:class` constraint or the membership view answers "no
        // instances" for it.
        NodeExpr::InstancesOf(class) => LoweredExpr {
            class: Some(walk.class_slot(class)),
            ..LoweredExpr::default()
        },
        NodeExpr::Filter { nodes, shape }
        | NodeExpr::FindFirst { nodes, shape }
        | NodeExpr::MatchAll { nodes, shape } => LoweredExpr {
            shapes: Box::new([lower_shape(shape, walk)]),
            operands: Box::new([lower_expression(nodes, walk)]),
            ..LoweredExpr::default()
        },
        NodeExpr::NodesMatching(shape) => LoweredExpr {
            shapes: Box::new([lower_shape(shape, walk)]),
            ..LoweredExpr::default()
        },
        NodeExpr::ConformsToShape { node, shape } => {
            let node = lower_expression(node, walk);
            match shape {
                // A NAMED shape argument is only reachable through this
                // expression, so what its constraints mention has to be lowered
                // here or it would go unresolved in the plan.
                ShapeArg::Named(shape) => LoweredExpr {
                    shapes: Box::new([lower_shape(shape, walk)]),
                    operands: Box::new([node]),
                    ..LoweredExpr::default()
                },
                // A COMPUTED one resolves, at evaluation, to a shape out of the
                // shapes graph's own top-level index. That index is lowered here
                // rather than assumed already covered: the assumption holds only
                // for a plan built over the WHOLE shape list, and the single-shape
                // plans `conforms` / `conforms_with_depth` build are exactly the
                // ones that would come up short. The expression computing the IRI
                // is lowered for the same reason every other operand is.
                ShapeArg::Computed { expr, shapes } => {
                    let expr = lower_expression(expr, walk);
                    LoweredExpr {
                        index: Some(walk.index_position(shapes)),
                        operands: Box::new([node, expr]),
                        ..LoweredExpr::default()
                    }
                }
            }
        }
        NodeExpr::Remove { nodes, remove } => LoweredExpr {
            operands: Box::new([lower_expression(nodes, walk), lower_expression(remove, walk)]),
            ..LoweredExpr::default()
        },
        NodeExpr::FlatMap { nodes, map } => LoweredExpr {
            operands: Box::new([lower_expression(nodes, walk), lower_expression(map, walk)]),
            ..LoweredExpr::default()
        },
        // A custom node-expression function call (Node Expressions §6.1/§6.2): what
        // its BODY names must be pre-resolved too, because the body is what
        // actually evaluates. The arguments are lowered for the same reason every
        // other operand is.
        NodeExpr::CustomCall { func, args } => {
            let operands: Box<[LoweredExpr]> = args
                .iter()
                .map(|(_, arg)| lower_expression(arg, walk))
                .collect();
            let iri = func.iri.as_str().to_owned();
            // The body is lowered ONCE per function: it is shared by `Arc` and
            // names the same terms at every call site, and it may call this very
            // function, so re-entering it would not terminate. The in-flight list
            // is what stops that re-entry; the map alone could not, because the
            // entry is only made once the body has finished lowering.
            if !walk.bodies.contains_key(&iri)
                && !walk.in_flight.contains(&iri)
                && let Some(body) = func.body.get()
            {
                walk.in_flight.push(iri.clone());
                let lowered = lower_expression(body, walk);
                walk.in_flight.pop();
                walk.bodies.insert(iri.clone(), lowered);
            }
            LoweredExpr {
                body: Some(iri),
                operands,
                ..LoweredExpr::default()
            }
        }
        NodeExpr::Union(items) | NodeExpr::Intersection(items) | NodeExpr::Concat(items) => {
            LoweredExpr {
                operands: items
                    .iter()
                    .map(|item| lower_expression(item, walk))
                    .collect(),
                ..LoweredExpr::default()
            }
        }
        NodeExpr::If { cond, then, els } => LoweredExpr {
            operands: Box::new([
                lower_expression(cond, walk),
                lower_expression(then, walk),
                lower_expression(els, walk),
            ]),
            ..LoweredExpr::default()
        },
        NodeExpr::Count { of, .. }
        | NodeExpr::Distinct(of)
        | NodeExpr::Min(of)
        | NodeExpr::Max(of)
        | NodeExpr::Sum(of)
        | NodeExpr::Limit { of, .. }
        | NodeExpr::Offset { of, .. }
        | NodeExpr::Exists(of) => LoweredExpr {
            operands: Box::new([lower_expression(of, walk)]),
            ..LoweredExpr::default()
        },
        NodeExpr::OrderBy { of, key, .. } => LoweredExpr {
            operands: Box::new([lower_expression(of, walk), lower_expression(key, walk)]),
            ..LoweredExpr::default()
        },
        NodeExpr::Call(call) => {
            let args = match call {
                FnCall::Builtin { args, .. }
                | FnCall::UserDefined { args, .. }
                | FnCall::Sparql { args, .. } => args,
            };
            LoweredExpr {
                operands: args.iter().map(|arg| lower_expression(arg, walk)).collect(),
                ..LoweredExpr::default()
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        ClassCatalog, LoweredConstraint, PreparedTargets, ShapePlan, lower_shapes,
        lower_standalone_expression,
    };
    use crate::engine::parse_shapes;
    use crate::expression::NodeExpr;
    use crate::shapes::{Constraint, Shape, Shapes};
    use crate::term::{NamedNode, Term};

    /// Fixture IRIs are under `example.org`: PurRDF mints no vocabulary IRIs.
    const PREFIXES: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
@prefix ex: <http://example.org/ns#> .
";

    /// A shapes graph reaching a class by every route the walk has to cover: a
    /// `sh:targetClass`, an `sh:class` on a property shape, a class inside a
    /// logical constraint, one inside a `sh:qualifiedValueShape`, one behind a
    /// reifier shape, and one a `shnex:instancesOf` node expression selects.
    fn every_route_shapes() -> Shapes {
        parse_shapes(
            &format!(
                r"{PREFIXES}
ex:RootShape a sh:NodeShape ;
    sh:targetClass ex:Target ;
    sh:property [
        sh:path ex:direct ;
        sh:class ex:Direct ;
        sh:qualifiedValueShape [ sh:class ex:Qualified ] ;
        sh:qualifiedMinCount 1 ;
        sh:reifierShape [ sh:class ex:Reified ] ;
    ] ;
    sh:not [ sh:class ex:Negated ] ;
    sh:and ( [ sh:class ex:Conjunct ] ) ;
    sh:property [
        sh:path ex:computed ;
        sh:expression [
            shnex:count [ shnex:instancesOf ex:Selected ]
        ] ;
    ] .
"
            ),
            None,
        )
        .expect("the fixture shapes parse")
    }

    /// **The class catalog and the lowered plan agree on every class IRI.**
    ///
    /// Two resolvers answer "what is this class's dataset identity?" — the catalog,
    /// which target resolution asks BY IRI, and the lowered constraint, which the
    /// evaluator reads BY SLOT — and nothing else in the crate makes them the same
    /// answer. They are the same answer only because one walk hands them out
    /// together, in [`ShapeWalk::class_slot`], and a refactor that split that call
    /// in two would make this seam the next place a class silently goes unresolved:
    /// the constraint would read a slot that was never a class, the catalog would
    /// keep a class no constraint indexes, and every existing test would pass.
    ///
    /// So the two are bound here. Every class IRI the walk recorded a SLOT for must
    /// be in the catalog, and every class in the catalog must be reachable from the
    /// shape tree the walk was given.
    ///
    /// [`ShapeWalk::class_slot`]: super::ShapeWalk::class_slot
    #[test]
    fn class_catalog_and_lowered_plan_agree_on_every_class_iri() {
        let shapes = every_route_shapes();
        let lowered = lower_shapes(shapes.node_shapes.iter());

        // Every route really is exercised, or the agreement below would be an
        // agreement about a much smaller set than the one that matters.
        let mut catalogued: Vec<&str> = lowered
            .classes()
            .entries()
            .map(|(class, _)| class.as_str())
            .collect();
        catalogued.sort_unstable();
        assert_eq!(
            catalogued,
            vec![
                "http://example.org/ns#Conjunct",
                "http://example.org/ns#Direct",
                "http://example.org/ns#Negated",
                "http://example.org/ns#Qualified",
                "http://example.org/ns#Reified",
                "http://example.org/ns#Selected",
                "http://example.org/ns#Target",
            ],
            "the fixture no longer reaches every route the walk covers, so this test agrees \
             about less than it claims to"
        );

        // The slot side: every term the walk recorded for a CLASS constraint names
        // an IRI the catalog also holds, at a position inside the binding row.
        let mut slotted: Vec<&str> = Vec::new();
        collect_class_slots(&lowered, &mut slotted);
        slotted.sort_unstable();
        slotted.dedup();
        assert_eq!(
            slotted,
            vec![
                "http://example.org/ns#Conjunct",
                "http://example.org/ns#Direct",
                "http://example.org/ns#Negated",
                "http://example.org/ns#Qualified",
                "http://example.org/ns#Reified",
                "http://example.org/ns#Selected",
            ],
            "every class a CONSTRAINT names must have a slot; `ex:Target` is named only by a \
             `sh:targetClass`, which target resolution answers from the catalog by IRI and never \
             from a slot, so it is the one catalogued class with no slot and its absence here is \
             the shape of the two resolvers' division of labour"
        );
        for class in &slotted {
            let position = lowered
                .classes()
                .position(&NamedNode::new_unchecked(*class))
                .unwrap_or_else(|| {
                    panic!(
                        "the lowered plan resolves <{class}> but the class catalog has never \
                         heard of it, so the two resolvers disagree"
                    )
                });
            assert!(
                position < lowered.classes().len(),
                "<{class}> sits at position {position} of a {}-slot binding row",
                lowered.classes().len()
            );
        }

        // …and the catalog side: a class in the catalog that no route reaches would
        // be a catalog the walk did not derive.
        for (class, _) in lowered.classes().entries() {
            assert!(
                catalogued.contains(&class.as_str()),
                "the catalog holds <{}>, which the shape tree does not reach",
                class.as_str()
            );
        }
    }

    /// Every class IRI a `LoweredConstraint::Class` names, read back out of the
    /// term row the walk filled.
    fn collect_class_slots<'a>(lowered: &'a super::LoweredShapes, out: &mut Vec<&'a str>) {
        for shape in &lowered.shapes {
            collect_shape_class_slots(lowered, shape, out);
        }
        for index in &lowered.indexes {
            for shape in index.shapes.values() {
                collect_shape_class_slots(lowered, shape, out);
            }
        }
    }

    fn collect_shape_class_slots<'a>(
        lowered: &'a super::LoweredShapes,
        shape: &super::LoweredShape,
        out: &mut Vec<&'a str>,
    ) {
        for constraint in &shape.constraints {
            collect_constraint_class_slots(lowered, constraint, out);
        }
        for property in &shape.properties {
            collect_property_class_slots(lowered, property, out);
        }
    }

    fn collect_property_class_slots<'a>(
        lowered: &'a super::LoweredShapes,
        property: &super::LoweredProperty,
        out: &mut Vec<&'a str>,
    ) {
        for constraint in &property.constraints {
            collect_constraint_class_slots(lowered, constraint, out);
        }
        for nested in &property.properties {
            collect_property_class_slots(lowered, nested, out);
        }
        for reifier in &property.reifiers {
            collect_shape_class_slots(lowered, reifier, out);
        }
    }

    /// The same wildcard-free discipline as the walk itself, and for the same
    /// reason: a new constraint kind that could reach a shape — or a class — must
    /// not be able to slip past the test that binds the two resolvers together. A
    /// wildcard here would make this agreement quietly narrower than it claims,
    /// which is the exact failure mode it exists to catch.
    fn collect_constraint_class_slots<'a>(
        lowered: &'a super::LoweredShapes,
        constraint: &LoweredConstraint,
        out: &mut Vec<&'a str>,
    ) {
        match constraint {
            LoweredConstraint::Class(slot) => {
                let Term::NamedNode(class) = &lowered.terms[*slot as usize] else {
                    panic!("an sh:class slot holds something other than an IRI");
                };
                out.push(class.as_str());
            }
            LoweredConstraint::Not(shape) | LoweredConstraint::Node(shape) => {
                collect_shape_class_slots(lowered, shape, out);
            }
            LoweredConstraint::And(shapes)
            | LoweredConstraint::Or(shapes)
            | LoweredConstraint::Xone(shapes) => {
                for shape in shapes {
                    collect_shape_class_slots(lowered, shape, out);
                }
            }
            LoweredConstraint::QualifiedValueShape { shape, siblings } => {
                collect_shape_class_slots(lowered, shape, out);
                for sibling in siblings {
                    collect_shape_class_slots(lowered, sibling, out);
                }
            }
            // Kinds that name neither a class nor a shape.
            LoweredConstraint::Datatype
            | LoweredConstraint::NodeKind
            | LoweredConstraint::MinCount
            | LoweredConstraint::MaxCount
            | LoweredConstraint::In(_)
            | LoweredConstraint::HasValue(_)
            | LoweredConstraint::Pattern
            | LoweredConstraint::MinLength
            | LoweredConstraint::MaxLength
            | LoweredConstraint::UniqueLang
            | LoweredConstraint::LanguageIn
            | LoweredConstraint::Closed
            | LoweredConstraint::MinInclusive
            | LoweredConstraint::MaxInclusive
            | LoweredConstraint::MinExclusive
            | LoweredConstraint::MaxExclusive
            | LoweredConstraint::Sparql
            | LoweredConstraint::Equals(_)
            | LoweredConstraint::Disjoint(_)
            | LoweredConstraint::LessThan(_)
            | LoweredConstraint::LessThanOrEquals(_)
            | LoweredConstraint::Component => {}
            // The two expression-bearing kinds reach their shapes and classes
            // through a `LoweredExpr`, which is walked here rather than skipped.
            LoweredConstraint::Expression(expr) => {
                collect_expr_class_slots(lowered, expr, out);
            }
            LoweredConstraint::NodeByExpression { expr, index } => {
                collect_expr_class_slots(lowered, expr, out);
                for shape in lowered.indexes[*index as usize].shapes.values() {
                    collect_shape_class_slots(lowered, shape, out);
                }
            }
        }
    }

    /// Every class slot a lowered node expression reaches, its operands and the
    /// shapes it names included.
    fn collect_expr_class_slots<'a>(
        lowered: &'a super::LoweredShapes,
        expr: &super::LoweredExpr,
        out: &mut Vec<&'a str>,
    ) {
        if let Some(slot) = expr.class {
            let Term::NamedNode(class) = &lowered.terms[slot as usize] else {
                panic!("a shnex:instancesOf slot holds something other than an IRI");
            };
            out.push(class.as_str());
        }
        for shape in &expr.shapes {
            collect_shape_class_slots(lowered, shape, out);
        }
        for operand in &expr.operands {
            collect_expr_class_slots(lowered, operand, out);
        }
    }

    /// A `shnex:instancesOf` the walk lowered really carries its class slot, and a
    /// lowering that carries none refuses loudly rather than answering "no
    /// instances".
    ///
    /// The believed-INVALID direction of the refusal: a term the lowering never
    /// recorded is a defect in this crate, and it is an `Err` rather than a `None`
    /// because a silent `None` here would make a `shnex:instancesOf` expression
    /// select nothing at all — a constraint that stopped constraining, with every
    /// existing test still green.
    #[test]
    fn an_expression_lowering_without_its_class_is_refused_loudly() {
        let lowering = lower_standalone_expression(&NodeExpr::InstancesOf(
            NamedNode::new_unchecked("http://example.org/ns#Selected"),
        ));
        let shapes = every_route_shapes();
        let data = crate::text_ingest::parse_turtle_to_dataset(
            "<http://example.org/ns#a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
             <http://example.org/ns#Selected> .",
            None,
        )
        .expect("the fixture data parses");
        let binding = lowering.bind(data.as_ref());
        let plan = lowering.plan(&binding);

        // The lowering the walk really made answers, and answers with an identity.
        assert!(
            lowering
                .expr()
                .class_id(plan)
                .expect("a lowered shnex:instancesOf carries its class")
                .is_some(),
            "the fixture data interns the class, so the identity must be present"
        );

        // A DIFFERENT expression's lowering carries no class at all. Asking it for
        // one is the defect, and it is loud.
        let empty = lower_standalone_expression(&NodeExpr::This);
        let empty_binding = empty.bind(data.as_ref());
        let error = empty
            .expr()
            .class_id(empty.plan(&empty_binding))
            .expect_err("a lowering that records no class cannot answer for one");
        assert!(
            error.contains("shnex:instancesOf") && error.contains("never resolved"),
            "the refusal must name what was asked for and why it could not be answered, got: \
             {error}"
        );

        // …and the neighbouring VALID case: the same lowered expression over a data
        // graph that interns no such class is a soft `None`, never a refusal. A
        // shapes graph is entitled to name a class its data lacks.
        let bare = crate::text_ingest::parse_turtle_to_dataset(
            "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#b> .",
            None,
        )
        .expect("the fixture data parses");
        let bare_binding = lowering.bind(bare.as_ref());
        assert_eq!(
            lowering.expr().class_id(lowering.plan(&bare_binding)),
            Ok(None),
            "a class the data graph never names is 'nothing is an instance', not a refusal"
        );

        let _ = &shapes;
    }

    /// A lowering that describes a DIFFERENT constraint kind from the one beside it
    /// is refused, naming both.
    ///
    /// The pairing is made by one walk and consumed by index, so a disagreement is a
    /// defect in this crate and never in a caller's data. An evaluator that read
    /// past it would evaluate `sh:in` against an `sh:hasValue`'s identity — a
    /// constraint that still runs and no longer means what the shapes graph says.
    #[test]
    fn a_lowering_that_describes_another_constraint_is_refused_loudly() {
        let shapes = parse_shapes(
            &format!(
                r"{PREFIXES}
ex:Shape a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:property [ sh:path ex:p ; sh:in ( ex:x ) ] .
"
            ),
            None,
        )
        .expect("the fixture shapes parse");
        let lowered = lower_shapes(shapes.node_shapes.iter());
        let data = crate::text_ingest::parse_turtle_to_dataset(
            "<http://example.org/ns#a> <http://example.org/ns#p> <http://example.org/ns#x> .",
            None,
        )
        .expect("the fixture data parses");
        let binding = lowered.bind(data.as_ref(), lowered.classes());
        let targets = PreparedTargets::default();
        let plan: ShapePlan<'_> = lowered
            .plan(
                &shapes.node_shapes[0],
                0,
                &binding,
                lowered.classes(),
                &targets,
            )
            .expect("the fixture shape has a lowering");

        // The honest pairing resolves.
        let property = &shapes.node_shapes[0].property_shapes[0];
        let (constraint, lowering) = plan
            .properties()
            .expect("the shape's property shapes pair with their lowerings")
            .next()
            .expect("the fixture declares one property shape")
            .1
            .constraints(property)
            .expect("the property shape's constraints pair with their lowerings")
            .next()
            .expect("the fixture declares one constraint");
        assert!(
            matches!(constraint, Constraint::In(_)),
            "the fixture's one constraint is an sh:in"
        );
        plan.planned(constraint, lowering)
            .expect("an sh:in beside its own lowering resolves");

        // A DIFFERENT declaration beside that lowering does not.
        let other = Constraint::HasValue(Term::NamedNode(NamedNode::new_unchecked(
            "http://example.org/ns#x",
        )));
        let error = plan
            .planned(&other, lowering)
            .expect_err("an sh:hasValue beside an sh:in's lowering is a plan defect");
        assert!(
            error.contains("sh:hasValue") && error.contains("In("),
            "the refusal must name the constraint and the lowering it was paired with, got: \
             {error}"
        );
    }

    /// A catalog rebuilt from the walk's own entries is the walk's own catalog.
    ///
    /// The decoder's gate ([`ClassCatalog::from_entries`]) and the derivation
    /// ([`ClassCatalog::from_walk`]) are different code; this is the standing check
    /// that a product carrying the derivation's output is one the decoder accepts.
    #[test]
    fn the_decoder_accepts_the_walks_own_catalog() {
        let shapes = every_route_shapes();
        let lowered = lower_shapes(shapes.node_shapes.iter());
        let entries: Vec<(NamedNode, usize)> = lowered
            .classes()
            .entries()
            .map(|(class, position)| (class.clone(), position))
            .collect();
        let rebuilt = ClassCatalog::from_entries(entries).expect("the walk's own catalog decodes");
        for (class, position) in lowered.classes().entries() {
            assert_eq!(rebuilt.position(class), Some(position));
        }
    }

    /// A shape the lowering does not cover is refused rather than evaluated against
    /// another shape's lowering.
    #[test]
    fn a_shape_past_the_lowering_is_refused_loudly() {
        let shapes = every_route_shapes();
        let lowered = lower_shapes(shapes.node_shapes.iter());
        let data = crate::text_ingest::parse_turtle_to_dataset("", None)
            .expect("an empty data graph parses");
        let binding = lowered.bind(data.as_ref(), lowered.classes());
        let targets = PreparedTargets::default();
        let stray: &Shape = &shapes.node_shapes[0];
        let error = lowered
            .plan(stray, 99, &binding, lowered.classes(), &targets)
            .expect_err("position 99 is past the shapes this lowering covers");
        assert!(
            error.contains("diverged"),
            "the refusal must say the shape list and its lowering disagree, got: {error}"
        );
    }

    /// The lowering reaches shapes held ONLY by a `sh:nodeByExpression` index.
    #[test]
    fn the_shape_index_is_lowered_once_and_reachable() {
        let shapes = parse_shapes(
            &format!(
                r"{PREFIXES}
ex:Outer a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:nodeByExpression [ shnex:constant ex:Inner ] .

ex:Inner a sh:NodeShape ;
    sh:class ex:Indexed .
"
            ),
            None,
        )
        .expect("the fixture shapes parse");
        let lowered = lower_shapes(shapes.node_shapes.iter());
        assert_eq!(
            lowered.indexes.len(),
            1,
            "a shapes graph has exactly one shape index, entered exactly once"
        );
        assert!(
            lowered
                .classes()
                .position(&NamedNode::new_unchecked("http://example.org/ns#Indexed"))
                .is_some(),
            "a class reachable only through the shape index must still be catalogued"
        );
    }

    /// A lowering is shareable across rayon workers: it is immutable behind `&`.
    #[test]
    fn a_lowering_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::LoweredShapes>();
        assert_send_sync::<super::DatasetBinding>();
        assert_send_sync::<Arc<super::LoweredShapes>>();
        assert_send_sync::<ShapePlan<'static>>();
    }
}
