// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Core constraint implementations.
//!
//! Evaluates all non-SPARQL SHACL Core constraint components plus the
//! recursive shape evaluator.  PyO3-free.

use crate::data_view::ShaclRead;

use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::OnceLock;

use ::purrdf::{FastMap, FastSet, IdSet, TermId, TermRef};
use smallvec::SmallVec;

use crate::data::{GraphFilter, ShaclData, native_quads, quads_for_pattern_ids, resolve_id};
use crate::engine::FocusNode;
use crate::model::{BoxRoleVocab, rdf, sh};
use crate::path;
use crate::plan::{PlannedConstraint, PropertyPlan, RangeBound, ShapePlan};
use crate::report::ValidationResult;
use crate::shapes::{ComponentValidator, NodeKindValue, Path, PropertyShape, Shape};
use crate::term::{NamedNode, Term, Triple, canonical_cmp_ids, term_id_to_native};

/// Internal value-node currency for the constraint layer.
///
/// A property shape's value nodes carry their identity in `TermId` space for the
/// common case (`Interned`, value nodes originating from interned data), and fall
/// back to an owned [`Term`] only for non-interned SHACL-AF node-expression-produced
/// terms that have no `TermId` (`Foreign`).
///
/// Identity/set/membership constraint arms compare `TermId`s directly (a `Copy`
/// integer hash), so a value node that only participates in those operations is
/// never materialized on the conforming path. Content-needing arms (datatype,
/// pattern, length, node-kind, numeric/string comparisons) resolve to an owned
/// [`Term`] on demand, and the report boundary always records an owned [`Term`].
#[derive(Clone)]
enum ValueNode {
    /// An interned value node — carries its `TermId`, resolved to a [`Term`] only
    /// when a constraint needs its content or a violation is recorded.
    Interned(TermId),
    /// A non-interned value node produced by a SHACL-AF node expression: it has no
    /// `TermId`, so its owned [`Term`] is carried verbatim.
    Foreign(Term),
}

impl ValueNode {
    /// Resolve to an owned native [`Term`] (the report boundary and content-check
    /// input): materialize an interned id, or clone the foreign term.
    fn to_term(&self, ds: &impl ShaclRead) -> Term {
        match self {
            Self::Interned(id) => term_id_to_native(ds, *id),
            Self::Foreign(term) => term.clone(),
        }
    }

    /// This value node as a focus node, for the constraints that recurse into a
    /// shape at it.
    ///
    /// Free for the interned arm, which is the arm the change path produces:
    /// the recursion focus is the id, and nothing is materialized. The foreign
    /// arm clones the term it already owns — the same clone the owned-term
    /// recursion boundary used to make for EVERY value node, interned or not.
    fn as_focus(&self, ds: &impl ShaclRead) -> FocusNode {
        match self {
            Self::Interned(id) => FocusNode::Interned(*id),
            Self::Foreign(term) => FocusNode::resolve(ds, term),
        }
    }

    /// The single value node a NODE-level constraint sees: the focus node itself.
    fn of_focus(focus: &FocusNode) -> Self {
        match focus {
            FocusNode::Interned(id) => Self::Interned(*id),
            FocusNode::Foreign(term) => Self::Foreign(term.clone()),
        }
    }

    /// The interned id of this value node, if it has one. A `Foreign` term is
    /// resolved against `ds` in case it happens to be interned (usually it is not).
    fn as_id(&self, ds: &impl ShaclRead) -> Option<TermId> {
        match self {
            Self::Interned(id) => Some(*id),
            Self::Foreign(term) => resolve_id(ds, term),
        }
    }

    /// Borrow the lexical surface used by `sh:pattern`/length constraints
    /// without materializing an interned value node.
    fn lexical<'a>(&'a self, ds: &'a impl ShaclRead) -> Option<&'a str> {
        match self {
            Self::Interned(id) => match ds.resolve(*id) {
                TermRef::Iri(iri) => Some(iri),
                TermRef::Literal { lexical, .. } => Some(lexical),
                TermRef::Blank { .. } | TermRef::Triple { .. } => None,
            },
            Self::Foreign(Term::NamedNode(node)) => Some(node.as_str()),
            Self::Foreign(Term::Literal(literal)) => Some(literal.value()),
            Self::Foreign(Term::BlankNode(_) | Term::Triple(_)) => None,
        }
    }

    /// The node kind of this value node without materializing an interned id.
    /// Mirrors `term_ref_to_native` one-to-one: `Iri`→`NamedNode`,
    /// `Blank`→`BlankNode`, `Literal`→`Literal`, `Triple`→`Triple`.
    fn kind(&self, ds: &impl ShaclRead) -> ValueKind {
        match self {
            Self::Interned(id) => match ds.resolve(*id) {
                TermRef::Iri(_) => ValueKind::Iri,
                TermRef::Blank { .. } => ValueKind::Blank,
                TermRef::Literal { .. } => ValueKind::Literal,
                TermRef::Triple { .. } => ValueKind::Triple,
            },
            Self::Foreign(term) => ValueKind::of_term(term),
        }
    }

    /// Borrow the language tag of a language-tagged literal value node without
    /// materializing an interned id; `None` for any other node.
    fn language<'a>(&'a self, ds: &'a impl ShaclRead) -> Option<&'a str> {
        match self {
            Self::Interned(id) => match ds.resolve(*id) {
                TermRef::Literal { language, .. } => language,
                TermRef::Iri(_) | TermRef::Blank { .. } | TermRef::Triple { .. } => None,
            },
            Self::Foreign(Term::Literal(literal)) => literal.language(),
            Self::Foreign(Term::NamedNode(_) | Term::BlankNode(_) | Term::Triple(_)) => None,
        }
    }

    /// Borrow `(lexical, datatype IRI)` for a literal value node.
    fn literal_parts<'a>(&'a self, ds: &'a impl ShaclRead) -> Option<(&'a str, &'a str)> {
        let view = self.literal_view(ds)?;
        Some((view.lexical, view.datatype))
    }

    /// Borrow the whole comparison surface of a literal value node.
    ///
    /// This is the one derivation of "what a literal looks like to the value
    /// comparisons": [`Self::literal_parts`] and the property-pair order
    /// constraints both read it, so the interned and foreign spellings of a
    /// literal are transcribed once rather than once per caller.
    fn literal_view<'a>(&'a self, ds: &'a impl ShaclRead) -> Option<LiteralView<'a>> {
        match self {
            Self::Interned(id) => literal_view_of_id(ds, *id),
            Self::Foreign(term) => literal_view_of_term(term),
        }
    }
}

/// A literal's comparison surface, BORROWED rather than materialized.
///
/// Every value comparison SHACL defines over literals — the range facets and the
/// property-pair order constraints — reads exactly these three fields. Carrying
/// them as borrows is what lets a conforming focus node be compared without an
/// owned [`Term`] ever existing: an interned literal's lexical form, datatype IRI
/// and language tag all live in the dataset's interner already.
#[derive(Clone, Copy, Debug)]
struct LiteralView<'a> {
    /// The literal's lexical form.
    lexical: &'a str,
    /// The literal's datatype IRI.
    datatype: &'a str,
    /// The literal's language tag, if it carries one.
    language: Option<&'a str>,
}

/// The comparison surface of an INTERNED term, or `None` when it is not a literal.
fn literal_view_of_id(ds: &impl ShaclRead, id: TermId) -> Option<LiteralView<'_>> {
    let TermRef::Literal {
        lexical,
        datatype,
        language,
        ..
    } = ds.resolve(id)
    else {
        return None;
    };
    let TermRef::Iri(datatype) = ds.resolve(datatype) else {
        return None;
    };
    Some(LiteralView {
        lexical,
        datatype,
        language,
    })
}

/// The comparison surface of an owned term, or `None` when it is not a literal.
fn literal_view_of_term(term: &Term) -> Option<LiteralView<'_>> {
    match term {
        Term::Literal(literal) => Some(LiteralView {
            lexical: literal.value(),
            datatype: literal.datatype_str(),
            language: literal.language(),
        }),
        Term::NamedNode(_) | Term::BlankNode(_) | Term::Triple(_) => None,
    }
}

/// The RDF node kind of a value node — the discriminant `sh:nodeKind` tests, so
/// the conforming path never builds an owned [`Term`] just to look at its variant.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ValueKind {
    Iri,
    Blank,
    Literal,
    Triple,
}

impl ValueKind {
    fn of_term(term: &Term) -> Self {
        match term {
            Term::NamedNode(_) => Self::Iri,
            Term::BlankNode(_) => Self::Blank,
            Term::Literal(_) => Self::Literal,
            Term::Triple(_) => Self::Triple,
        }
    }
}

/// Borrowed result metadata for one constraint source.
///
/// Keeping this separate from [`Shape`] lets property-shape evaluation borrow
/// the three fields it needs instead of cloning an entire synthetic shape for
/// every focus node.
#[derive(Clone, Copy)]
struct ConstraintSource<'a> {
    id: &'a Term,
    severity: &'a crate::report::Severity,
    message: &'a Option<String>,
}

impl<'a> From<&'a Shape> for ConstraintSource<'a> {
    fn from(shape: &'a Shape) -> Self {
        Self {
            id: &shape.id,
            severity: &shape.severity,
            message: &shape.message,
        }
    }
}

/// Immutable state shared by recursive constraint and property evaluation.
#[derive(Clone, Copy)]
struct ValidationContext<'a, 'memo> {
    store: &'a ShaclData,
    box_role_vocab: Option<&'a BoxRoleVocab>,
    /// The shape being evaluated, with every derivation that does not depend on
    /// this focus node already made.
    plan: ShapePlan<'a>,
    depth: u32,
    /// The `(value node, shape)` conformance answers this run already computed.
    memo: &'memo ConformanceMemo<'a>,
}

impl<'a> ValidationContext<'a, '_> {
    /// This context re-aimed at a shape reached through a constraint.
    ///
    /// The box-role vocabulary is deliberately dropped. A recursive conformance
    /// check REPORTS nothing, and a box role only ever drives result
    /// attribution, so carrying one would make every `sh:or` member scan the data
    /// graph for its path's roles and then discard the answer. This reproduces
    /// what the recursive entry point has always passed.
    #[inline]
    fn inner(self, plan: ShapePlan<'a>) -> Self {
        Self {
            box_role_vocab: None,
            plan,
            ..self
        }
    }
}

// ── The result sink ────────────────────────────────────────────────────────────

/// What a [`ResultSink`] tells the traversal to do once it has been handed a
/// violation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Flow {
    /// Keep going: this sink wants every violation the focus node produces.
    Continue,
    /// Unwind: this sink has learned everything this focus node can tell it.
    Stop,
}

impl Flow {
    #[inline]
    const fn stopped(self) -> bool {
        matches!(self, Self::Stop)
    }
}

/// Where one traversal of a shape sends the violations it finds.
///
/// **The evaluator is polymorphic in its sink, and in nothing else.** There is
/// one traversal, one set of constraint arms and one definition of what
/// conformance means; the sink decides only what happens to a violation once the
/// traversal has found it. That is the whole point of stating it this way. A
/// `fast: bool` parameter, or a separate "just tell me whether it conforms"
/// entry point, would give a future reader two code paths that can — and
/// eventually do — disagree about conformance, and the disagreement would show
/// up as a report that contradicts its own `sh:conforms` flag.
///
/// The cheap half is [`ResultSink::RECORDS_RESULTS`]. A recursive constraint
/// (`sh:node`, `sh:and`, `sh:or`, `sh:xone`, `sh:not`,
/// `sh:qualifiedValueShape`) asks only whether the inner shape holds, and the
/// answer is one bit; building a full [`ValidationResult`] for it — cloning the
/// focus term, the result path, the path structure, the source shape, the
/// severity and the message — and then dropping the vector is pure waste, paid
/// once per member per value node per focus node. A sink that does not record
/// results never runs the builder at all.
trait ResultSink {
    /// Whether this sink consumes the result VALUE, or only the fact that a
    /// violation happened.
    ///
    /// It is an associated constant rather than a method so that the branch on
    /// it is resolved when the traversal is monomorphized: the result builder of
    /// a conformance-only traversal is not merely skipped at run time, it is not
    /// compiled into that traversal.
    const RECORDS_RESULTS: bool;

    /// Record one violation.
    ///
    /// `build` produces the result this violation would be REPORTED as. It is
    /// invoked if and only if [`Self::RECORDS_RESULTS`], so a caller may put
    /// arbitrary reporting work inside it.
    fn violation(&mut self, build: impl FnOnce() -> ValidationResult) -> Flow;
}

/// The reporting sink: every result is kept, and the traversal always runs to
/// completion because a SHACL report names every violation, not the first.
#[derive(Default)]
struct Collect {
    results: Vec<ValidationResult>,
}

impl ResultSink for Collect {
    const RECORDS_RESULTS: bool = true;

    #[inline]
    fn violation(&mut self, build: impl FnOnce() -> ValidationResult) -> Flow {
        self.results.push(build());
        Flow::Continue
    }
}

/// The conformance sink: records THAT a violation exists and stops the traversal.
///
/// SHACL conformance is "produces no results", so the first violation settles it
/// and everything after it is work whose answer is already known.
#[derive(Default)]
struct AnyViolation {
    seen: bool,
}

impl AnyViolation {
    /// The conformance verdict this probe reached.
    #[inline]
    const fn conforms(&self) -> bool {
        !self.seen
    }
}

impl ResultSink for AnyViolation {
    const RECORDS_RESULTS: bool = false;

    #[inline]
    fn violation(&mut self, _build: impl FnOnce() -> ValidationResult) -> Flow {
        self.seen = true;
        Flow::Stop
    }
}

/// A sink that stamps a node shape's graph-box roles onto each result on its way
/// to the enclosing sink.
///
/// Every `eval_constraint` arm returns results with all three role vectors
/// empty, so stamping a ROLELESS shape onto them is the identity; the emptiness
/// test is kept here rather than at each call site so the traversal has one
/// shape whatever the vocabulary configuration is.
struct NodeRoles<'a, S> {
    inner: &'a mut S,
    roles: &'a [NamedNode],
}

impl<S: ResultSink> ResultSink for NodeRoles<'_, S> {
    const RECORDS_RESULTS: bool = S::RECORDS_RESULTS;

    #[inline]
    fn violation(&mut self, build: impl FnOnce() -> ValidationResult) -> Flow {
        let roles = self.roles;
        self.inner.violation(move || {
            let mut result = build();
            if !roles.is_empty() {
                result.apply_box_roles(roles, &[]);
            }
            result
        })
    }
}

/// The report-only metadata a property shape stamps onto every result its
/// constraints produce.
///
/// All four are LAZY, because a conforming focus node needs none of them: it
/// never allocates a native path term, never clones a complex path structure,
/// never merges the source graph-box roles and never scans the graph for the
/// path's roles.
#[derive(Default)]
struct PropertyLazies {
    path_term: OnceLock<Term>,
    path_structure: OnceLock<Option<Path>>,
    source_roles: OnceLock<Vec<NamedNode>>,
    path_roles: OnceLock<Vec<NamedNode>>,
}

/// The borrowed inputs those lazies are derived from, plus the focus node each
/// result is re-homed onto.
#[derive(Clone, Copy)]
struct PropertyStamp<'a> {
    store: &'a ShaclData,
    box_role_vocab: Option<&'a BoxRoleVocab>,
    focus: &'a FocusNode,
    ps: &'a PropertyShape,
    parent_box_roles: &'a [NamedNode],
    lazy: &'a PropertyLazies,
}

impl PropertyStamp<'_> {
    /// This property shape's source roles, merged with its parent's on first use.
    fn source_roles(&self) -> &[NamedNode] {
        self.lazy
            .source_roles
            .get_or_init(|| merge_box_roles(self.parent_box_roles, &self.ps.box_roles))
    }

    /// The graph-box roles of this property shape's path, resolved on first use.
    fn path_roles(&self) -> &[NamedNode] {
        self.lazy
            .path_roles
            .get_or_init(|| path_box_roles(self.store, &self.ps.path, self.box_role_vocab))
    }

    /// Stamp the property shape's path, focus node and roles onto one result.
    ///
    /// A path the CONSTRAINT itself bound is preserved: a `sh:sparql` query may
    /// project `?path` (SHACL-AF §3.4.2.2), which is more specific than the
    /// shape's declared path and must not be clobbered.
    fn apply(&self, result: &mut ValidationResult) {
        if result.result_path.is_none() {
            result.result_path = Some(
                self.lazy
                    .path_term
                    .get_or_init(|| path::path_to_term(&self.ps.path))
                    .clone(),
            );
            result
                .path_structure
                .clone_from(self.lazy.path_structure.get_or_init(|| {
                    if matches!(self.ps.path, Path::Predicate(_)) {
                        None
                    } else {
                        Some(self.ps.path.clone())
                    }
                }));
        }
        // THE materialization boundary for the focus node on the property path:
        // a result is being built, so the owned term is finally needed.
        result.focus_node = self.focus.to_term(self.store.core_view());
        result.apply_box_roles(self.source_roles(), self.path_roles());
    }
}

/// A sink that applies a [`PropertyStamp`] to each result on its way to the
/// enclosing sink.
struct Stamped<'a, S> {
    inner: &'a mut S,
    stamp: PropertyStamp<'a>,
}

impl<S: ResultSink> ResultSink for Stamped<'_, S> {
    const RECORDS_RESULTS: bool = S::RECORDS_RESULTS;

    #[inline]
    fn violation(&mut self, build: impl FnOnce() -> ValidationResult) -> Flow {
        let stamp = self.stamp;
        self.inner.violation(move || {
            let mut result = build();
            stamp.apply(&mut result);
            result
        })
    }
}

// ── The per-run conformance memo ───────────────────────────────────────────────

/// One `(value node, shape, depth)` conformance question.
///
/// Field ORDER is the comparison order a derived `PartialEq` uses, and the
/// cheapest discriminator is first: within one value node's member loop the
/// `node` matches and the `shape` is what differs, but across value nodes — the
/// far commoner miss — the `node` settles it without touching the IRI.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct MemoKey<'a> {
    /// The value node's INTERNED identity. A non-interned value node has none
    /// and never reaches this type — see [`ConformanceMemo`].
    node: TermId,
    /// The ambient `sh:filterShape` / `sh:exists` re-entry depth. It is part of
    /// the key because the depth ceiling is what turns a cyclic shapes graph
    /// into a hard error instead of a stack overflow: an answer reached with
    /// depth to spare is not an answer to a deeper ask.
    depth: u32,
    /// The shape's IRI, borrowed out of the shapes graph.
    ///
    /// The IRI and not the lowering's address, because a shape reached through
    /// a constraint is stored INLINE at each constraint site
    /// (`Constraint::Node(Box<Shape>)`): `sh:node ex:T` written at two sites is
    /// two `Shape` values at two addresses, and an address key would refuse to
    /// connect them — which is precisely the overlap this memo exists to
    /// collapse. In the SHACL data model a shape IS its node in the shapes
    /// graph, so two constraint sites naming the same IRI name the same shape
    /// and must reach the same verdict.
    ///
    /// Only IRI-identified shapes are keyed. A blank-node-identified shape is
    /// anonymous and belongs to exactly one constraint site, so it has no
    /// second site to share an answer with, and excluding it keeps the key clear
    /// of every question about whether a parser might reuse a blank label
    /// between a shape and something derived from it.
    shape: &'a str,
}

/// The conformance answers one validation run has already computed.
///
/// `sh:and`, `sh:or`, `sh:xone`, `sh:node` and `sh:qualifiedValueShape` all ask
/// "does this node conform to that shape?", and overlapping value sets make them
/// ask the SAME question more than once inside a single focus node's validation.
/// The answer is a pure function of the data graph, the node and the shape, so
/// every ask after the first is waste.
///
/// Two properties are load-bearing rather than incidental.
///
/// * **Keyed on the interned id alone.** A value node the data graph never
///   interned has no identity to key on, and "not interned" is not an identity:
///   two non-interned terms can be equal, which is exactly why `sh:hasValue` and
///   `sh:in` fall back to term comparison for them (see
///   `tests/foreign_value_nodes.rs`). Foreign value nodes therefore skip the
///   memo entirely and are recomputed every time — correct, and costing nothing
///   that the un-memoized evaluator did not already pay.
/// * **Scoped to one call.** [`crate::parallel`] shares one `&Shape` across rayon
///   workers, so a memo owned by a `PreparedValidator` would be contended
///   mutable state on the one path this crate parallelizes. This one is created
///   per top-level validation and dropped with it, which makes it worker-local
///   by construction.
#[derive(Default)]
struct ConformanceMemo<'a> {
    answers: RefCell<MemoTable<'a>>,
}

/// How many answers the memo keeps inline before it hashes.
///
/// This constant is the whole reason the memo is not itself a per-focus-node
/// allocation. A `FastMap` allocates its table on the FIRST insert, and the
/// shapes that ask only a handful of conformance questions per focus node — one
/// `sh:node`, a two-member `sh:and`, a `sh:qualifiedValueShape` over a short
/// value set — ask too few for a cache to ever pay that back. Measured: giving
/// those shapes a map cost exactly one extra allocation per focus node and
/// returned nothing, which is the growth term this whole exercise exists to
/// remove, reintroduced by the optimization meant to remove it.
///
/// Inline, the same shapes allocate nothing and answer a repeated question with
/// a linear scan over at most eight `Copy` keys. Past eight the shape is asking
/// enough questions that one allocation is noise beside the traversals a hashed
/// lookup saves, so the overflow goes to a `FastMap` and lookup stays O(1).
const MEMO_INLINE: usize = 8;

/// The memo's storage, small-first.
///
/// The two halves are held side by side rather than as an either/or, so filling
/// the inline block never copies it into the map: the first [`MEMO_INLINE`]
/// answers stay where they are and only the overflow is hashed. `spilled` is an
/// empty `FastMap` until that overflow happens, and an empty map has not
/// allocated — which is the property the whole small-first arrangement exists
/// to preserve.
#[derive(Default)]
struct MemoTable<'a> {
    /// The first [`MEMO_INLINE`] answers, scanned linearly.
    inline: SmallVec<[(MemoKey<'a>, bool); MEMO_INLINE]>,
    /// Every answer past the inline block.
    spilled: FastMap<MemoKey<'a>, bool>,
}

impl<'a> ConformanceMemo<'a> {
    /// The recorded verdict for `key`, if this run has already reached one.
    fn get(&self, key: MemoKey<'a>) -> Option<bool> {
        let table = self.answers.borrow();
        if let Some(&(_, verdict)) = table.inline.iter().find(|(recorded, _)| *recorded == key) {
            return Some(verdict);
        }
        // Probing an empty map would hash the key for nothing, and for the shapes
        // this memo is cheapest on the map is ALWAYS empty.
        if table.spilled.is_empty() {
            return None;
        }
        table.spilled.get(&key).copied()
    }

    /// Record the verdict for `key`.
    fn insert(&self, key: MemoKey<'a>, verdict: bool) {
        let mut table = self.answers.borrow_mut();
        if table.inline.len() < MEMO_INLINE {
            table.inline.push((key, verdict));
        } else {
            table.spilled.insert(key, verdict);
        }
    }
}

/// Does `focus` conform to `plan`'s shape, reusing `context`'s per-run memo?
///
/// This is the recursive entry every logical and shape-valued constraint arm
/// goes through. It runs the SAME traversal a report-producing validation runs,
/// with [`AnyViolation`] in place of [`Collect`]: the traversal cannot disagree
/// with the report about conformance, because it is the same traversal.
fn conforms_memoized<'a>(
    context: ValidationContext<'a, '_>,
    plan: ShapePlan<'a>,
    focus: &FocusNode,
) -> Result<bool, String> {
    // Both halves of the key must be present for this question to be memoizable:
    // a non-interned value node has no identity to key on (two non-interned terms
    // can be equal), and a blank-node-identified shape has no second site to share
    // an answer with. Either missing simply means the question is recomputed.
    let key = match (focus.id(), &plan.shape().id) {
        (Some(node), Term::NamedNode(iri)) => Some(MemoKey {
            node,
            depth: context.depth,
            shape: iri.as_str(),
        }),
        _ => None,
    };
    if let Some(key) = key
        && let Some(recorded) = context.memo.get(key)
    {
        return Ok(recorded);
    }
    let mut probe = AnyViolation::default();
    walk_shape(context.inner(plan), focus, &mut probe)?;
    let verdict = probe.conforms();
    if let Some(key) = key {
        context.memo.insert(key, verdict);
    }
    Ok(verdict)
}

// ── Public surface ─────────────────────────────────────────────────────────────

/// Validate a single focus node against a shape, returning all `ValidationResult`s.
///
/// Any result ⇒ non-conformance (regardless of severity).  Recurses for
/// `sh:and`, `sh:or`, `sh:xone`, and `sh:node` constraints.
///
/// A `deactivated` shape produces no results.
///
/// # Errors
///
/// Returns `Err(String)` when a SHACL-SPARQL constraint fails to EVALUATE (a
/// hard validation failure per SHACL-SPARQL — e.g. a query construct the
/// native engine cannot execute). Ordinary constraint violations are `Ok`
/// results, never errors.
pub fn validate_shape(
    store: &ShaclData,
    focus: &Term,
    shape: &Shape,
) -> Result<Vec<ValidationResult>, String> {
    validate_shape_with(store, focus, shape, None)
}

/// [`validate_shape`] with the caller-supplied [`BoxRoleVocab`] threaded in.
///
/// PurRDF mints no vocabulary IRIs, so the box-role feature has no default:
/// with `box_role_vocab = None` no data-graph role lookup is performed and no
/// role individual is stamped onto results — the feature is inactive, not
/// defaulted. Conformance (result existence) is identical either way; the
/// vocab only drives result role ATTRIBUTION.
///
/// # Errors
///
/// Returns `Err(String)` when a SHACL-SPARQL constraint fails to evaluate
/// (see [`validate_shape`]).
pub fn validate_shape_with(
    store: &ShaclData,
    focus: &Term,
    shape: &Shape,
    box_role_vocab: Option<&BoxRoleVocab>,
) -> Result<Vec<ValidationResult>, String> {
    // A caller holding only a parsed shape has no preparation to reuse, so the
    // lowering is derived here for this one call. A caller validating MANY focus
    // nodes should hold a `PreparedShapes`, which memoizes it.
    let lowering = OneShotLowering::of(store, shape);
    let focus = FocusNode::resolve(store.core_view(), focus);
    validate_shape_with_plan_at(store, &focus, box_role_vocab, lowering.plan()?)
}

pub(crate) fn validate_shape_with_plan_at(
    store: &ShaclData,
    focus: &FocusNode,
    box_role_vocab: Option<&BoxRoleVocab>,
    plan: ShapePlan<'_>,
) -> Result<Vec<ValidationResult>, String> {
    validate_shape_with_depth(store, focus, box_role_vocab, plan, 0)
}

/// A shape lowering built for ONE call, for the entry points that are handed a
/// bare [`Shape`] and have no preparation to memoize against.
///
/// It exists so those entry points can own the three values a [`ShapePlan`]
/// borrows — the lowering, the catalog it produced and the dataset binding — for
/// the duration of the call. Every repeated-validation surface goes through
/// `PreparedShapes` instead, where the lowering is derived once per shapes graph
/// rather than once per call.
struct OneShotLowering<'a> {
    lowered: crate::plan::LoweredShapes,
    binding: crate::plan::DatasetBinding,
    shape: &'a Shape,
}

impl<'a> OneShotLowering<'a> {
    fn of(store: &ShaclData, shape: &'a Shape) -> Self {
        let lowered = crate::plan::lower_shapes(std::iter::once(shape));
        let binding = lowered.bind(store.core_view(), lowered.classes());
        Self {
            lowered,
            binding,
            shape,
        }
    }

    fn plan(&self) -> Result<ShapePlan<'_>, String> {
        self.lowered.plan(
            self.shape,
            0,
            &self.binding,
            self.lowered.classes(),
            self.lowered.no_targets(),
        )
    }
}

/// [`validate_shape_with`] carrying the ambient `sh:filterShape` / `sh:exists`
/// re-entry depth (see [`conforms_with_depth`]). A depth past
/// [`crate::expression::MAX_RECURSION_DEPTH`] is a hard error — a mutually
/// recursive filter/exists cycle fails closed here rather than overflowing the
/// native stack.
fn validate_shape_with_depth(
    store: &ShaclData,
    focus: &FocusNode,
    box_role_vocab: Option<&BoxRoleVocab>,
    plan: ShapePlan<'_>,
    depth: u32,
) -> Result<Vec<ValidationResult>, String> {
    let memo = ConformanceMemo::default();
    let context = ValidationContext {
        store,
        box_role_vocab,
        plan,
        depth,
        memo: &memo,
    };
    collect_shape(context, focus)
}

/// [`walk_shape`] with the reporting sink: every result the focus node produces,
/// in traversal order.
fn collect_shape(
    context: ValidationContext<'_, '_>,
    focus: &FocusNode,
) -> Result<Vec<ValidationResult>, String> {
    let mut sink = Collect::default();
    walk_shape(context, focus, &mut sink)?;
    Ok(sink.results)
}

/// **The one traversal.**
///
/// Evaluates `context.plan`'s shape at one focus node and hands every violation
/// to `sink`. A report and a conformance check differ only in which
/// [`ResultSink`] is passed here — never in what is evaluated — so the two can
/// no more disagree about conformance than a function can disagree with itself.
///
/// A depth past [`crate::expression::MAX_RECURSION_DEPTH`] is a hard error: a
/// mutually recursive filter/exists cycle fails closed here rather than
/// overflowing the native stack.
fn walk_shape<S: ResultSink>(
    context: ValidationContext<'_, '_>,
    focus: &FocusNode,
    sink: &mut S,
) -> Result<Flow, String> {
    let plan = context.plan;
    let shape = plan.shape();
    if context.depth > crate::expression::MAX_RECURSION_DEPTH {
        return Err(format!(
            "SHACL validation recursion depth exceeded ({} > {}) at shape {}: a cyclic sh:filterShape / sh:exists reference",
            context.depth,
            crate::expression::MAX_RECURSION_DEPTH,
            shape.id
        ));
    }
    if shape.deactivated {
        return Ok(Flow::Continue);
    }

    // --- Node-level constraints (value nodes = [focus], no path) ---
    // The single value node IS the focus term. Keep an interned focus id-native;
    // only a genuinely foreign SHACL-AF term needs an owned clone.
    let node_value_nodes = [ValueNode::of_focus(focus)];
    {
        // A node-level result carries the shape's roles and no path roles.
        let mut node_sink = NodeRoles {
            inner: &mut *sink,
            roles: &shape.box_roles,
        };
        for (constraint, lowered) in plan.constraints()? {
            if eval_constraint(
                context,
                focus,
                &node_value_nodes,
                plan.planned(constraint, lowered)?,
                None,
                ConstraintSource::from(shape),
                &mut node_sink,
            )?
            .stopped()
            {
                return Ok(Flow::Stop);
            }
        }
    }

    // --- Property shapes ---
    for (ps, property_plan) in plan.properties()? {
        if eval_property_shape(context, focus, ps, property_plan, &shape.box_roles, sink)?.stopped()
        {
            return Ok(Flow::Stop);
        }
    }

    // --- sh:closed (node-shape-level; needs the sibling property shapes) ---
    // `eval_closed` stamps each result's box roles itself — the source roles plus
    // the OFFENDING PREDICATE's path roles — so closed-world violations carry the
    // same predicate attribution that property-shape results do — violations
    // must not drop their predicate role.
    for (constraint, lowered) in plan.constraints()? {
        if let PlannedConstraint::Closed { permitted } = plan.planned(constraint, lowered)?
            && eval_closed(context, focus, shape, permitted, sink).stopped()
        {
            return Ok(Flow::Stop);
        }
    }

    Ok(Flow::Continue)
}

/// Evaluate `sh:closed` against a focus node (SHACL §4.8.1).
///
/// The permitted predicate set is the union of:
/// - every simple-predicate `sh:path` of the shape's property shapes (an inverse
///   path constrains incoming, not outgoing, triples and so does not permit an
///   outgoing predicate); and
/// - the `sh:ignoredProperties` list.
///
/// `rdf:type` is NOT implicitly permitted: per the spec (and W3C
/// `core/node/closed-001`), a closed shape reports EVERY predicate not
/// declared by `sh:property` or listed in `sh:ignoredProperties` — shapes that
/// want to allow `rdf:type` must list it in `sh:ignoredProperties`
/// (`core/node/closed-002` does exactly that).
///
/// One result per focus-node outgoing triple whose predicate is not permitted.
///
/// Both halves of this are id-native, and neither was:
///
/// * `permitted` is the stage-0 union resolved at BIND — see
///   [`crate::plan::LoweredConstraint::Closed`]. It used to be rebuilt, as a set
///   of borrowed IRI strings, once per focus node, even though it is a function
///   of the shapes graph alone.
/// * the outgoing quads are walked in id space. The previous `native_quads` call
///   materialized an owned `(Term, NamedNode, Term)` for EVERY outgoing quad of
///   every focus node, only to throw all three away again for the permitted ones
///   — which, on a conforming graph, is all of them. Here the subject is never
///   materialized (it is the focus node, which the caller already holds), and the
///   predicate and object are materialized only inside a violation.
///
/// The subject-legality guard `native_quads` applied per quad is applied once,
/// against the focus term, because every quad this probe matches has the focus as
/// its subject.
fn eval_closed<S: ResultSink>(
    context: ValidationContext<'_, '_>,
    focus: &FocusNode,
    shape: &Shape,
    permitted: &FastSet<TermId>,
    sink: &mut S,
) -> Flow {
    // A focus node this data graph does not intern has no outgoing quads at all,
    // so a closed shape has nothing to report against it — the same empty answer
    // the materializing probe gave, reached without a dictionary lookup.
    let store = context.store;
    let box_role_vocab = context.box_role_vocab;
    let ds = store.core_view();
    let (Some(focus_id), true) = (focus.id(), focus.is_subject(ds)) else {
        return Flow::Continue;
    };
    for quad in quads_for_pattern_ids(ds, Some(focus_id), None, None, GraphFilter::AnyGraph) {
        if permitted.contains(&quad.p) {
            continue;
        }
        // Past the permitted probe this quad IS a violation, so materializing its
        // predicate is reporting work, not traversal work. It is materialized
        // ahead of the sink because a term that is not an IRI cannot be a result
        // path and is skipped rather than reported; everything downstream of that
        // decision — the object, the predicate's roles and the result itself — is
        // built inside the sink, so a conformance-only traversal builds none of it.
        let Term::NamedNode(predicate) = term_id_to_native(ds, quad.p) else {
            continue;
        };
        let flow = sink.violation(|| {
            // Resolve the offending predicate's graph-box roles (the same
            // resolution property shapes use for their path) so closed-world
            // results are not left with empty path attribution.
            let path_roles =
                path_box_roles(store, &Path::Predicate(predicate.clone()), box_role_vocab);
            let mut result = ValidationResult {
                focus_node: focus.to_term(ds),
                result_path: Some(Term::NamedNode(predicate)),
                path_structure: None,
                value: Some(term_id_to_native(ds, quad.o)),
                source_constraint_component: NamedNode::from(sh::CLOSED_CONSTRAINT_COMPONENT),
                source_shape: shape.id.clone(),
                severity: shape.severity.clone(),
                message: shape.message.clone(),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            };
            result.apply_box_roles(&shape.box_roles, &path_roles);
            result
        });
        if flow.stopped() {
            return Flow::Stop;
        }
    }
    Flow::Continue
}

/// Returns `true` iff the focus node produces zero validation results against
/// the shape (i.e., it fully conforms).
///
/// This convenience entry point lowers `shape` on every call — the whole
/// cycle-aware walk, plus one interning probe per constant it names. A caller that
/// checks MANY focus nodes against the same shape should lower it once instead:
/// [`PreparedShapes`](crate::engine::PreparedShapes) is the public route, and it
/// is what `rules` reuses internally for its `sh:condition` checks.
///
/// # Errors
///
/// Returns `Err(String)` when a SHACL-SPARQL constraint fails to evaluate
/// (see [`validate_shape`]).
pub fn conforms(store: &ShaclData, focus: &Term, shape: &Shape) -> Result<bool, String> {
    let lowering = OneShotLowering::of(store, shape);
    conforms_with_id_depth(
        store,
        &FocusNode::resolve(store.core_view(), focus),
        lowering.plan()?,
        0,
    )
}

/// [`conforms`] against a plan the caller already built.
///
/// `plan` must cover `shape` — i.e. it was built from a shape iterator that
/// included it — or an `sh:class` / `sh:targetClass` this evaluation reaches will
/// be absent from the catalog and reported as the plan defect it is.
///
/// # Errors
///
/// Returns `Err(String)` when a constraint fails to evaluate (see
/// [`validate_shape`]).
pub(crate) fn conforms_with_plan(
    store: &ShaclData,
    focus: &Term,
    plan: ShapePlan<'_>,
) -> Result<bool, String> {
    conforms_with_id_depth(
        store,
        &FocusNode::resolve(store.core_view(), focus),
        plan,
        0,
    )
}

/// The conformance entry for a caller OUTSIDE one validation run — `sh:condition`
/// in `rules`, `sh:filterShape` in the node-expression evaluator, and the two
/// public wrappers above.
///
/// It opens a fresh [`ConformanceMemo`] because it is the start of a run; the
/// recursive constraint arms go through [`conforms_memoized`] instead, which
/// reuses the memo their ambient context already carries.
pub(crate) fn conforms_with_id_depth(
    store: &ShaclData,
    focus: &FocusNode,
    plan: ShapePlan<'_>,
    depth: u32,
) -> Result<bool, String> {
    let memo = ConformanceMemo::default();
    let context = ValidationContext {
        store,
        box_role_vocab: None,
        plan,
        depth,
        memo: &memo,
    };
    let mut probe = AnyViolation::default();
    walk_shape(context, focus, &mut probe)?;
    Ok(probe.conforms())
}

// ── Property shape evaluator ───────────────────────────────────────────────────

fn eval_property_shape<'a, S: ResultSink>(
    context: ValidationContext<'a, '_>,
    focus: &FocusNode,
    ps: &'a PropertyShape,
    property_plan: PropertyPlan<'a>,
    parent_box_roles: &[NamedNode],
    sink: &mut S,
) -> Result<Flow, String> {
    let store = context.store;
    // A deactivated property shape validates nothing (SHACL §2.1.3.3).
    if ps.deactivated {
        return Ok(Flow::Continue);
    }
    // Carry the value nodes id-native for the common interned-focus case
    // ([`path::eval_ids`]); a non-interned SHACL-AF focus falls back to the
    // owned-`Term` producer. Identity/set constraint arms then compare `TermId`s
    // without materializing, and only the value nodes a violation records — or a
    // content constraint inspects — are resolved to owned terms.
    // The path is the LOWERED one: every predicate step carries the dataset
    // identity resolved at bind, so a property shape evaluated across a million
    // focus nodes hashes its predicate IRI zero times.
    let lowered_path = property_plan.path();
    let binding = context.plan.binding();
    let value_nodes: SmallVec<[ValueNode; 4]> = match focus {
        FocusNode::Interned(id) => {
            path::eval_planned_ids_from_id(store.core_view(), *id, lowered_path, binding)?
                .into_iter()
                .map(ValueNode::Interned)
                .collect()
        }
        FocusNode::Foreign(term) => {
            path::eval_planned(store.core_view(), term, lowered_path, binding)?
                .into_iter()
                .map(ValueNode::Foreign)
                .collect()
        }
    };
    // Report-only materialization is lazy: a conforming focus node never
    // allocates a native path term, clones a complex path structure, merges the
    // source graph-box roles, or scans the graph for the path's roles. All four
    // exist only to be stamped onto a `ValidationResult`, and a conforming focus
    // node produces none.
    let lazy = PropertyLazies::default();
    let stamp = PropertyStamp {
        store,
        box_role_vocab: context.box_role_vocab,
        focus,
        ps,
        parent_box_roles,
        lazy: &lazy,
    };
    let constraint_source = ConstraintSource {
        id: &ps.id,
        severity: &ps.severity,
        message: &ps.message,
    };

    for (constraint, lowered) in property_plan.constraints(ps)? {
        // The stamp travels with the sink rather than being applied to a returned
        // vector, so the property shape's path, focus and roles are resolved
        // inside the result builder — which a conformance-only traversal never
        // runs.
        let mut stamped = Stamped {
            inner: &mut *sink,
            stamp,
        };
        if eval_constraint(
            context,
            focus,
            &value_nodes,
            context.plan.planned(constraint, lowered)?,
            Some(&ps.path),
            constraint_source,
            &mut stamped,
        )?
        .stopped()
        {
            return Ok(Flow::Stop);
        }
    }

    // --- Nested property shapes (sh:property on a property shape) ---
    // Spec §2.1: sh:property may appear on ANY shape. On a property shape it
    // constrains this shape's VALUE nodes: each value node becomes the focus
    // node of the nested property shape (W3C core/property/property-001,
    // core/validation-reports/shared — a nested shape reached via two parents
    // fires once per reach, so results are NOT deduplicated here).
    for (nested, nested_plan) in property_plan.properties(ps)? {
        for value in &value_nodes {
            // A value node becomes the focus of the nested shape, and it becomes
            // one in whichever representation it already has: an interned value
            // node recurses as an interned focus node, with nothing materialized.
            if eval_property_shape(
                context,
                &value.as_focus(store.core_view()),
                nested,
                nested_plan,
                stamp.source_roles(),
                sink,
            )?
            .stopped()
            {
                return Ok(Flow::Stop);
            }
        }
    }

    // Reifier shapes ask the RDF 1.2 statement layer about the quoted triple
    // `<< focus predicate value >>`, which is an identity question end to end: the
    // focus node, the path predicate and each value node are already carried as
    // interned ids, and the quoted triple term this data graph holds for them — if
    // it holds one — is found by an id-native dictionary lookup. So the value
    // nodes, the focus term and the path term are NOT materialized here; each is
    // rendered inside a result builder, which a conforming focus node never runs.
    if !ps.reifier_shapes.is_empty() || ps.reification_required {
        return eval_reifier_shapes(
            ReifierEvalContext {
                context,
                focus,
                value_nodes: &value_nodes,
                ps,
                source_roles: stamp.source_roles(),
                path_roles: stamp.path_roles(),
                lazy: &lazy,
                property_plan,
            },
            sink,
        );
    }
    Ok(Flow::Continue)
}

#[derive(Clone, Copy)]
struct ReifierEvalContext<'a, 'memo, 'stamp> {
    /// The ambient traversal state, on the shapes-graph lifetime `'a`.
    context: ValidationContext<'a, 'memo>,
    /// This property shape and its lowered reifier shapes, likewise `'a`.
    ps: &'a PropertyShape,
    property_plan: PropertyPlan<'a>,
    /// The traversal-currency inputs and the report-only material, both derived
    /// per call and so living only as long as the `eval_property_shape` frame that
    /// built them. The focus node and the value nodes arrive in whichever
    /// representation the path produced — interned for everything the data graph
    /// holds — and are materialized only inside a result builder.
    focus: &'stamp FocusNode,
    value_nodes: &'stamp [ValueNode],
    source_roles: &'stamp [NamedNode],
    path_roles: &'stamp [NamedNode],
    /// The enclosing property shape's deferred report material, shared so that the
    /// path term is rendered at most once per focus node and not at all for one
    /// that conforms.
    lazy: &'stamp PropertyLazies,
}

impl ReifierEvalContext<'_, '_, '_> {
    /// This property shape's path as the term a result's `sh:resultPath` carries.
    ///
    /// Rendering it clones the predicate IRI, so it is deferred to the report
    /// boundary rather than computed on entry: a focus node whose statements are
    /// all correctly reified renders no path term at all. The same
    /// [`PropertyLazies`] cell the property stamp uses backs it, so a focus node
    /// that produces results across both routes still renders it once.
    fn path_term(&self) -> &Term {
        self.lazy
            .path_term
            .get_or_init(|| path::path_to_term(&self.ps.path))
    }
}

/// Evaluate a property shape's `sh:reifierShape` / `sh:reificationRequired`.
///
/// The inner shape's results are an INPUT here rather than discarded output: a
/// reifier-shape result inherits the inner result's message and source roles. So
/// a reporting traversal collects them, while a conformance-only traversal —
/// which needs to know only that a reifier shape failed — probes with
/// [`AnyViolation`] and stops at the first inner violation instead of building
/// the rest.
fn eval_reifier_shapes<S: ResultSink>(
    ctx: ReifierEvalContext<'_, '_, '_>,
    sink: &mut S,
) -> Result<Flow, String> {
    let ReifierEvalContext {
        context,
        focus,
        value_nodes,
        ps,
        source_roles,
        path_roles,
        lazy: _,
        property_plan,
    } = ctx;
    if ps.reifier_shapes.is_empty() && !ps.reification_required {
        return Ok(Flow::Continue);
    }
    let Path::Predicate(predicate) = &ps.path else {
        return Ok(Flow::Continue);
    };
    let store = context.store;
    let ds = store.core_view();

    // A quoted triple's subject must be an IRI or a blank node — a fact about the
    // FOCUS node, which every value node reached along this path shares. Settled
    // once, from the interner, rather than re-derived from a materialized term per
    // value node.
    if !focus.is_subject(ds) {
        return Ok(Flow::Continue);
    }
    // The loop-invariant halves of the statement-layer lookup. A focus node this
    // view does not intern, a path predicate it does not intern, or an
    // `rdf:reifies` it does not intern each mean the same thing for EVERY value
    // node: no quoted triple term on this path is interned, or no row could name
    // one, so nothing is reified. Resolving them here keeps that answer out of the
    // loop entirely.
    let focus_id = focus.id();
    let predicate_id = ds.term_id_by_iri(predicate.as_str());
    let reifies_id = ds.term_id_by_iri(rdf::REIFIES);

    let source_roles = with_cbox_role(source_roles, context.box_role_vocab);
    // The reifier identities of ONE value node's statement, in canonical term
    // order. Inline for the sizes a statement layer actually carries — a handful
    // of reifiers per statement — so the common case keeps the whole arm free of
    // heap traffic; a statement with more than four reifiers spills, which is a
    // cost per REIFIER and not per focus node.
    let mut reifiers: SmallVec<[TermId; 4]> = SmallVec::new();
    for value in value_nodes {
        // A `None` here is not a skip and not an error: the data graph interns no
        // quoted triple term for this statement, so the statement HAS no reifier —
        // which is exactly the answer `sh:reificationRequired` reports a violation
        // for, and the answer `sh:reifierShape` has no reifier to judge against.
        let triple_id = match (focus_id, predicate_id, value.as_id(ds)) {
            (Some(subject), Some(predicate), Some(object)) => {
                ds.term_id_by_triple(subject, predicate, object)
            }
            _ => None,
        };
        reifiers.clear();
        let reified = match (triple_id, reifies_id) {
            (Some(triple), Some(reifies)) => {
                if ps.reifier_shapes.is_empty() {
                    // `sh:reificationRequired` alone asks whether a reifier
                    // EXISTS. No identity is consumed downstream, so none is kept
                    // and the probe stops at the first row instead of deduplicating
                    // and canonically ordering a collection nobody reads.
                    reifier_ids(ds, reifies, triple).next().is_some()
                } else {
                    reifiers.extend(reifier_ids(ds, reifies, triple));
                    // The owned probe this replaced deduplicated through a set and
                    // then sorted the result canonically. Interned ids are in
                    // INSERTION order, which is not canonical order, so the order
                    // is reproduced by comparing the terms the ids denote — which
                    // `canonical_cmp_ids` does by streaming both renderings, with
                    // nothing materialized. Equal ids denote one term and so land
                    // adjacent, which is what makes the dedup after it exact.
                    reifiers.sort_unstable_by(|left, right| canonical_cmp_ids(ds, *left, *right));
                    reifiers.dedup();
                    !reifiers.is_empty()
                }
            }
            _ => false,
        };
        if !reified && ps.reification_required {
            let flow = sink.violation(|| {
                let mut result = ValidationResult {
                    focus_node: focus.to_term(ds),
                    result_path: Some(ctx.path_term().clone()),
                    path_structure: None,
                    value: Some(reified_triple_term(ds, focus, predicate, value)),
                    source_constraint_component: NamedNode::from(
                        sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT,
                    ),
                    source_shape: ps.id.clone(),
                    severity: ps.severity.clone(),
                    message: ps.message.clone(),
                    source_box_roles: vec![],
                    path_box_roles: vec![],
                    result_box_roles: vec![],
                    attributions: vec![],
                };
                result.apply_box_roles(&source_roles, path_roles);
                result
            });
            if flow.stopped() {
                return Ok(Flow::Stop);
            }
            continue;
        }

        for &reifier in &reifiers {
            for (reifier_shape, reifier_plan) in property_plan.reifiers(ps)? {
                let reifier_context = ValidationContext {
                    plan: reifier_plan,
                    ..context
                };
                // The reifier came OUT of this view's statement layer, so it is
                // interned in it by construction: it recurses as its identity, with
                // no term materialized to resolve back into one.
                let reifier_focus = FocusNode::Interned(reifier);
                if !S::RECORDS_RESULTS {
                    // Conformance only: the inner results exist solely to supply
                    // this result's message, and this sink keeps no message. Probe
                    // for the first inner violation and stop there rather than
                    // collecting the rest.
                    let mut probe = AnyViolation::default();
                    walk_shape(reifier_context, &reifier_focus, &mut probe)?;
                    if probe.conforms() {
                        continue;
                    }
                    let flow = sink.violation(|| {
                        let message = reifier_shape.message.clone().or_else(|| ps.message.clone());
                        reifier_result(&ctx, predicate, value, &source_roles, message, &[])
                    });
                    if flow.stopped() {
                        return Ok(Flow::Stop);
                    }
                    continue;
                }
                for inner in collect_shape(reifier_context, &reifier_focus)? {
                    let flow = sink.violation(|| {
                        let message = inner
                            .message
                            .or_else(|| reifier_shape.message.clone())
                            .or_else(|| ps.message.clone());
                        reifier_result(
                            &ctx,
                            predicate,
                            value,
                            &source_roles,
                            message,
                            &inner.source_box_roles,
                        )
                    });
                    if flow.stopped() {
                        return Ok(Flow::Stop);
                    }
                }
            }
        }
    }
    Ok(Flow::Continue)
}

/// One `sh:reifierShape` result: the enclosing property shape's focus node, path
/// and quoted triple term, carrying `message` and the roles the inner result
/// contributed.
fn reifier_result(
    ctx: &ReifierEvalContext<'_, '_, '_>,
    predicate: &NamedNode,
    value: &ValueNode,
    source_roles: &[NamedNode],
    message: Option<String>,
    inner_source_roles: &[NamedNode],
) -> ValidationResult {
    let ds = ctx.context.store.core_view();
    let mut result = ValidationResult {
        focus_node: ctx.focus.to_term(ds),
        result_path: Some(ctx.path_term().clone()),
        path_structure: None,
        value: Some(reified_triple_term(ds, ctx.focus, predicate, value)),
        source_constraint_component: NamedNode::from(sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT),
        source_shape: ctx.ps.id.clone(),
        severity: ctx.ps.severity.clone(),
        message,
        source_box_roles: vec![],
        path_box_roles: vec![],
        result_box_roles: vec![],
        attributions: vec![],
    };
    let merged = merge_box_roles(source_roles, inner_source_roles);
    result.apply_box_roles(&merged, ctx.path_roles);
    result
}

/// The quoted triple term `<< focus predicate value >>` as a report value.
///
/// **A materialization boundary, and the only one this arm has.** It is called
/// from inside a result builder and nowhere else, so a statement that is reified
/// as its shape requires never builds one — which is the whole difference between
/// this and the owned triple the arm used to construct for every value node before
/// anything was known about it.
///
/// It is built rather than looked up because it must exist even when the data
/// graph interns no such term: a `sh:reificationRequired` violation REPORTS the
/// statement that is missing its reifier, and a statement the graph never quoted
/// is precisely the one most likely to be missing one.
fn reified_triple_term(
    ds: &impl ShaclRead,
    focus: &FocusNode,
    predicate: &NamedNode,
    value: &ValueNode,
) -> Term {
    Term::Triple(Box::new(Triple::new(
        focus.to_term(ds),
        predicate.clone(),
        value.to_term(ds),
    )))
}

/// The reifier resources declared for one quoted triple term, id-native.
///
/// Read through the VIEW rather than through whatever carrier sits under it. The
/// change path binds a projected delta view, whose statement layer is the union of
/// the base reifier table and the delta's own additions and suppressions; a lookup
/// aimed at the base dataset would answer about a world the caller no longer has,
/// making a reifier the delta added invisible and one it suppressed eternal. This
/// is the same `(?, rdf:reifies, triple)` probe the owned lookup it replaces
/// issued through the same view — minus the owned rows, the hash set and the
/// vector.
///
/// The subject-legality filter is the one the owned probe applied per row: a term
/// that cannot occupy a subject position is not a reifier.
fn reifier_ids<D: ShaclRead>(
    ds: &D,
    reifies: TermId,
    triple: TermId,
) -> impl Iterator<Item = TermId> + '_ {
    quads_for_pattern_ids(
        ds,
        None,
        Some(reifies),
        Some(triple),
        GraphFilter::DefaultGraph,
    )
    .filter(move |quad| matches!(ds.resolve(quad.s), TermRef::Iri(_) | TermRef::Blank { .. }))
    .map(|quad| quad.s)
}

fn path_box_roles(
    store: &ShaclData,
    path: &Path,
    box_role_vocab: Option<&BoxRoleVocab>,
) -> Vec<NamedNode> {
    // The box-role feature is caller-configured; with no vocab it is INACTIVE.
    let Some(vocab) = box_role_vocab else {
        return vec![];
    };
    // Composite paths key their role lookup on the first reachable predicate —
    // the same representative the report's `result_path` approximation uses.
    let Some(predicate) = path::primary_predicate(path) else {
        return vec![];
    };
    let predicate_term = Term::NamedNode(predicate.clone());
    let box_role = Term::NamedNode(NamedNode::from(vocab.graph_box_role.as_str()));
    let mut roles: Vec<NamedNode> = native_quads(
        store.core_view(),
        Some(&predicate_term),
        Some(&box_role),
        None,
        GraphFilter::DefaultGraph,
    )
    .into_iter()
    .filter_map(|(_, _, object)| match object {
        Term::NamedNode(node) => Some(node),
        _ => None,
    })
    .collect();
    roles.sort_unstable();
    roles.dedup();
    roles
}

/// Merge the caller vocabulary's CBox role individual into `source_roles`.
///
/// With no vocab configured the box-role feature is INACTIVE and this is the
/// identity — no role is minted, and the borrowed input is handed straight back
/// rather than copied, so the inactive case costs nothing at all.
fn with_cbox_role<'roles>(
    source_roles: &'roles [NamedNode],
    box_role_vocab: Option<&BoxRoleVocab>,
) -> Cow<'roles, [NamedNode]> {
    let Some(vocab) = box_role_vocab else {
        return Cow::Borrowed(source_roles);
    };
    Cow::Owned(merge_box_roles(
        source_roles,
        &[NamedNode::from(vocab.box_cbox.as_str())],
    ))
}

fn merge_box_roles(left: &[NamedNode], right: &[NamedNode]) -> Vec<NamedNode> {
    let mut roles = left.to_vec();
    roles.extend_from_slice(right);
    roles.sort_unstable();
    roles.dedup();
    roles
}

// ── Per-constraint evaluator ───────────────────────────────────────────────────

/// Evaluate a single constraint against the provided value node set.
///
/// `focus_node` is the SHACL focus node (subject) — always the real focus, never
/// a path value.  For node-level constraints `focus_node == value_nodes[0]`; for
/// property shapes `focus_node` is the subject while `value_nodes` are the path
/// objects.  `sh:sparql`'s `$this` must bind to `focus_node` in both contexts
/// (SHACL-AF spec: `$this` = focus node, not value node).
///
/// `path` is `None` for node-level constraints, `Some` for property shapes.
fn eval_constraint<'a, S: ResultSink>(
    context: ValidationContext<'a, '_>,
    focus_node: &FocusNode,
    value_nodes: &[ValueNode],
    constraint: PlannedConstraint<'a>,
    path: Option<&Path>,
    source: ConstraintSource<'_>,
    sink: &mut S,
) -> Result<Flow, String> {
    let store = context.store;
    let depth = context.depth;
    let ds = store.core_view();
    let result_path = || path.map(path::path_to_term);
    // The full SHACL path structure travels alongside a COMPLEX result path
    // (its result_path term is a deterministic blank node) so the report
    // serialization can emit the spec-mandated structure.
    let path_structure = || path.filter(|p| !matches!(p, Path::Predicate(_))).cloned();
    let severity = source.severity;
    let message = source.message;
    let source_shape = source.id;
    let shapes_graph_iri = store.shapes_graph_iri();

    macro_rules! result {
        ($component:expr, $value:expr) => {
            ValidationResult {
                focus_node: focus_node.to_term(ds),
                result_path: result_path(),
                path_structure: path_structure(),
                value: $value,
                source_constraint_component: NamedNode::from($component),
                source_shape: source_shape.clone(),
                severity: severity.clone(),
                message: message.clone(),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            }
        };
        ($component:expr, $focus:expr, $value:expr) => {
            ValidationResult {
                focus_node: $focus,
                result_path: result_path(),
                path_structure: path_structure(),
                value: $value,
                source_constraint_component: NamedNode::from($component),
                source_shape: source_shape.clone(),
                severity: severity.clone(),
                message: message.clone(),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            }
        };
    }

    // Hand one violation to the sink, unwinding the traversal if it says to stop.
    //
    // The result expression becomes the sink's BUILDER, so it runs only for a
    // sink that records results. That is what makes a conformance check cheap
    // without giving it a second definition of conformance: the decision — the
    // `if` above each of these — is shared, and only the reporting is skipped.
    macro_rules! emit {
        ($result:expr) => {
            if sink.violation(|| $result).stopped() {
                return Ok(Flow::Stop);
            }
        };
    }

    // The result GENERATOR a constraint folds to when stage 1 decided its verdict
    // for every value node at once.
    //
    // A folded constraint may not collapse to a boolean, and this is the whole
    // reason the fold is spelled as a generator: `sh:class` against a class this
    // data graph does not intern violates for every value node, and SHACL reports
    // ONE RESULT PER VIOLATING VALUE NODE, each stamped with that value node's own
    // term. Folding to "violates — report once" would leave every allocation
    // measurement green, every conformance boolean correct, and the report quietly
    // short by n−1 results. So the constant verdict is expressed as the results it
    // produces, in value-node order, which is byte-identical to what the unfolded
    // arm produced.
    macro_rules! violate_every_value_node {
        ($component:expr) => {{
            for value in value_nodes {
                emit!(result!($component, Some(value.to_term(ds))));
            }
            Flow::Continue
        }};
    }

    Ok(match constraint {
        // ── Count constraints (operate on the SET) ─────────────────────────────
        PlannedConstraint::MinCount(n) => {
            let count = value_nodes.len() as u64;
            if count < n {
                emit!(result!(sh::MIN_COUNT_CONSTRAINT_COMPONENT, None));
            }
            Flow::Continue
        }
        PlannedConstraint::MaxCount(n) => {
            let count = value_nodes.len() as u64;
            if count > n {
                emit!(result!(sh::MAX_COUNT_CONSTRAINT_COMPONENT, None));
            }
            Flow::Continue
        }

        // ── Class (per value node; honors asserted rdfs:subClassOf, §4.2.5) ────
        //
        // The CONSTANT-FOLDED branch: `None` = the data graph interns no term for
        // this class, so no value node can be an instance of it and every one of
        // them violates. That is a real verdict, not a failure to compute one, and
        // it was decided at BIND — once for the whole validation, not once per
        // value node of every focus node. The per-value-node arm below still runs
        // its own `is_none_or` test, so this fold is a shortcut around work whose
        // answer is already known and not a second statement of the rule.
        PlannedConstraint::Class(None) => {
            violate_every_value_node!(sh::CLASS_CONSTRAINT_COMPONENT)
        }
        PlannedConstraint::Class(id) => {
            let class_id = id;
            for vn in value_nodes {
                // Id-native instance test: an interned value node's class membership
                // is decided entirely in `TermId` space (literal check + `rdf:type`
                // edge walk), so a conforming value is never materialized. A
                // non-interned value node has no `rdf:type` edge and is no instance.
                let violates = match vn.as_id(ds) {
                    Some(id) => {
                        class_id.is_none_or(|class| !store.class_view().is_instance(id, class))
                    }
                    None => true,
                };
                if violates {
                    emit!(result!(
                        sh::CLASS_CONSTRAINT_COMPONENT,
                        Some(vn.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Datatype (per value node) ──────────────────────────────────────────
        PlannedConstraint::Datatype(dt_iri) => {
            for value in value_nodes {
                if !check_value_datatype(value, ds, dt_iri) {
                    emit!(result!(
                        sh::DATATYPE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── NodeKind (per value node) ──────────────────────────────────────────
        PlannedConstraint::NodeKind(kind) => {
            for value in value_nodes {
                // Kind-only check on the borrowed discriminant: the owned term is
                // built only for a violation, not per conforming value node.
                if !check_value_kind(value.kind(ds), kind) {
                    emit!(result!(
                        sh::NODE_KIND_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── In (per value node) ────────────────────────────────────────────────
        //
        // The CONSTANT-FOLDED branch: `sh:in ()` permits nothing, so every value
        // node violates whatever the data graph contains. This one is a constant
        // of the SHAPE rather than of the binding, which is why it is stated on
        // `allowed` and not on the id set — see the arm below for why an id set
        // that is empty is NOT the same condition.
        PlannedConstraint::In { allowed: &[], .. } => {
            violate_every_value_node!(sh::IN_CONSTRAINT_COMPONENT)
        }
        PlannedConstraint::In {
            allowed,
            ids: allowed_ids,
        } => {
            // Membership is pure identity, and the allowed set was resolved to ids
            // ONCE, at bind: an interned value node's check is a `TermId` set
            // lookup with no materialization and no per-focus-node set to build.
            //
            // The interner is injective, so an allowed term that is not interned
            // can never equal an INTERNED value node. That is also why an empty
            // `allowed_ids` is not a constant verdict: a value node produced by a
            // SHACL-AF node expression need not be interned either, and two
            // non-interned terms can perfectly well be equal. Folding "no member
            // is interned" to "everything violates" would refuse a conforming
            // graph. What the injectivity DOES buy is the shape of the match
            // below: only a foreign value node with no identity of its own can
            // reach the term comparison, so an interned value node is never
            // materialized to run it.
            for vn in value_nodes {
                let allowed_here = match vn {
                    ValueNode::Interned(id) => allowed_ids.contains(id),
                    ValueNode::Foreign(term) => match resolve_id(ds, term) {
                        Some(id) => allowed_ids.contains(&id),
                        None => allowed.iter().any(|a| terms_equal(a, term)),
                    },
                };
                if !allowed_here {
                    emit!(result!(sh::IN_CONSTRAINT_COMPONENT, Some(vn.to_term(ds))));
                }
            }
            Flow::Continue
        }

        // ── HasValue (on the SET, one result if missing) ───────────────────────
        PlannedConstraint::HasValue { required, id } => {
            // Identity check: if `required` is interned, membership is a `TermId`
            // comparison and no value node is materialized. If it is not interned,
            // no interned value node can equal it, so only the (rare) non-interned
            // value nodes could match — fall back to term equality. Which of the
            // two it is was decided at bind.
            let found = match id {
                Some(req_id) => value_nodes.iter().any(|v| v.as_id(ds) == Some(req_id)),
                // The required term has no identity in this data graph, so by the
                // interner's injectivity no INTERNED value node can equal it: a
                // term that is equal to a term the graph does not intern is itself
                // not interned. Only a genuinely foreign value node — a SHACL-AF
                // node expression's product — can still match, so the comparison
                // is restricted to those and no interned value node is
                // materialized to lose it.
                //
                // Note what this deliberately does NOT do: fold to "not found".
                // Two non-interned terms can be equal, so a shape whose
                // `sh:hasValue` names a term the data never mentions still
                // CONFORMS against an expression that produces that very term, and
                // declaring the whole constraint violated would refuse it.
                None => value_nodes.iter().any(|v| match v {
                    ValueNode::Interned(_) => false,
                    ValueNode::Foreign(term) => terms_equal(term, required),
                }),
            };
            if !found {
                emit!(result!(sh::HAS_VALUE_CONSTRAINT_COMPONENT, None));
            }
            Flow::Continue
        }

        // ── Pattern (per value node) ───────────────────────────────────────────
        PlannedConstraint::Pattern {
            regex,
            flags,
            compiled,
        } => {
            // Compile at most once per Constraint instance (across all focus
            // nodes and value nodes) using the OnceLock cache.  Behaviour is
            // identical to the per-call path: Err ⇒ violation on every value.
            let compiled = compiled.get_or_init(|| build_regex(regex, flags.map(String::as_str)));
            // SHACL has no shape-error channel on this path, so a pattern that
            // does not compile stays a violation (the W3C suite depends on
            // that). But discarding the compiler's typed error made a BROKEN
            // SHAPE indistinguishable from bad data; carry its precise message
            // into every result's `sh:resultMessage` instead.
            let compile_error = compiled.as_ref().err().map(|error| {
                format!(
                    "invalid sh:pattern {regex:?}{}: {error}",
                    flags
                        .map(|f| format!(" with sh:flags {f:?}"))
                        .unwrap_or_default()
                )
            });
            let violation_message = match (&compile_error, message) {
                (Some(error), Some(shape_message)) => Some(format!("{shape_message} ({error})")),
                (Some(error), None) => Some(error.clone()),
                (None, shape_message) => shape_message.clone(),
            };
            for value in value_nodes {
                let violates = match (compiled, value.lexical(ds)) {
                    (Err(_), _) => true,   // bad regex → violation on every value node
                    (Ok(_), None) => true, // blank node → violation
                    (Ok(pattern), Some(lex)) => !pattern.as_regex().is_match(lex),
                };
                if violates {
                    emit!(ValidationResult {
                        focus_node: focus_node.to_term(ds),
                        result_path: result_path(),
                        path_structure: path_structure(),
                        value: Some(value.to_term(ds)),
                        source_constraint_component: NamedNode::from(
                            sh::PATTERN_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity: severity.clone(),
                        message: violation_message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            Flow::Continue
        }

        // ── MinLength (per value node) ─────────────────────────────────────────
        PlannedConstraint::MinLength(n) => {
            for value in value_nodes {
                // Length from the borrowed lexical surface (`ValueNode::lexical`
                // agrees with `lexical_length` variant-for-variant): the owned term
                // is built only for a violation, not per conforming value node.
                let len_opt = value.lexical(ds).map(|s| s.chars().count());
                let violates = match len_opt {
                    None => true, // blank node
                    Some(len) => (len as u64) < n,
                };
                if violates {
                    emit!(result!(
                        sh::MIN_LENGTH_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── MaxLength (per value node) ─────────────────────────────────────────
        PlannedConstraint::MaxLength(n) => {
            for value in value_nodes {
                // Length from the borrowed lexical surface (`ValueNode::lexical`
                // agrees with `lexical_length` variant-for-variant): the owned term
                // is built only for a violation, not per conforming value node.
                let len_opt = value.lexical(ds).map(|s| s.chars().count());
                let violates = match len_opt {
                    None => true, // blank node
                    Some(len) => (len as u64) > n,
                };
                if violates {
                    emit!(result!(
                        sh::MAX_LENGTH_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── LanguageIn (per value node) ────────────────────────────────────────
        PlannedConstraint::LanguageIn(tags) => {
            for value in value_nodes {
                // Tag match on the borrowed language (`None` = not a language-tagged
                // literal, which never matches): the owned term is built only for a
                // violation, not per conforming value node.
                let matches = value
                    .language(ds)
                    .is_some_and(|lang| language_tag_matches_any(lang, tags));
                if !matches {
                    emit!(result!(
                        sh::LANGUAGE_IN_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Not (per value node, recursive) ────────────────────────────────────
        PlannedConstraint::Not(inner_shape) => {
            for value in value_nodes {
                // The recursion focus IS the value node, in the representation it
                // already has: no materialization, and no reverse hash probe to
                // recover an identity the value node was already carrying.
                let focus = value.as_focus(ds);
                // Violation iff the value node DOES conform to the negated shape.
                if conforms_memoized(context, inner_shape, &focus)? {
                    emit!(result!(
                        sh::NOT_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Closed (node-shape-level; evaluated in validate_shape) ─────────────
        // The closed-world check needs the SET of permitted predicates, derived
        // from the sibling property shapes — data `eval_constraint` does not
        // receive. It is evaluated directly in `validate_shape`; here it is a
        // no-op so the match stays exhaustive.
        PlannedConstraint::Closed { .. } => Flow::Continue,

        // ── UniqueLang (on the SET) ────────────────────────────────────────────
        PlannedConstraint::UniqueLang(true) => {
            // Tallied over BORROWED tags in an inline buffer, and the buffer is
            // indexed by DISTINCT language rather than by value node — a focus
            // node carries a handful of languages however many labels it has, so
            // the scan is over a few entries and the whole tally allocates
            // nothing. The two things this replaced both cost one allocation per
            // conforming focus node: the map's table on its first insert, and an
            // owned lowercased key per value node. Case folding is now a
            // comparison rather than a new string, which is the same relation
            // BCP 47 tags are compared under (they are ASCII).
            let mut seen_langs: SmallVec<[(&str, usize); UNIQUE_LANG_INLINE]> = SmallVec::new();
            for value in value_nodes {
                // Content arm on the BORROWED language tag: `ValueNode::language`
                // is `None` for every node that is not a language-tagged literal,
                // which is exactly the set the owned-term match skipped, so the
                // counted population is unchanged and no value node is
                // materialized to be counted.
                if let Some(lang) = value.language(ds) {
                    if let Some(entry) = seen_langs
                        .iter_mut()
                        .find(|(seen, _)| seen.eq_ignore_ascii_case(lang))
                    {
                        entry.1 += 1;
                    } else {
                        seen_langs.push((lang, 1));
                    }
                }
            }
            // First-seen order, where the map this replaced iterated in hash
            // order. Both are re-sorted by `finish_report` on the full serialized
            // result identity, so the report bytes are unchanged — and this one
            // does not depend on the hasher.
            for (lang, count) in &seen_langs {
                if *count > 1 {
                    emit!(ValidationResult {
                        focus_node: focus_node.to_term(ds),
                        result_path: result_path(),
                        path_structure: path_structure(),
                        value: None,
                        source_constraint_component: NamedNode::from(
                            sh::UNIQUE_LANG_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity: severity.clone(),
                        message: message.clone().or_else(|| Some(format!(
                            "duplicate language tag: {}",
                            lang.to_lowercase()
                        ))),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            Flow::Continue
        }
        PlannedConstraint::UniqueLang(false) => Flow::Continue,

        // ── MinInclusive / MaxInclusive (per value node) ───────────────────────
        PlannedConstraint::MinInclusive(bound) => {
            for value in value_nodes {
                // The comparison reads the value node's lexical form and datatype
                // IRI, both of which it can BORROW out of the dataset. The owned
                // term is built below, inside the violation, because that is the
                // only place a result needs one.
                let violates = !matches!(
                    range_facet_cmp(value.literal_parts(ds), bound),
                    Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                );
                if violates {
                    emit!(result!(
                        sh::MIN_INCLUSIVE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }
        PlannedConstraint::MaxInclusive(bound) => {
            for value in value_nodes {
                // The comparison reads the value node's lexical form and datatype
                // IRI, both of which it can BORROW out of the dataset. The owned
                // term is built below, inside the violation, because that is the
                // only place a result needs one.
                let violates = !matches!(
                    range_facet_cmp(value.literal_parts(ds), bound),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                );
                if violates {
                    emit!(result!(
                        sh::MAX_INCLUSIVE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── MinExclusive / MaxExclusive (per value node) ───────────────────────
        PlannedConstraint::MinExclusive(bound) => {
            for value in value_nodes {
                // The comparison reads the value node's lexical form and datatype
                // IRI, both of which it can BORROW out of the dataset. The owned
                // term is built below, inside the violation, because that is the
                // only place a result needs one.
                let violates = !matches!(
                    range_facet_cmp(value.literal_parts(ds), bound),
                    Some(std::cmp::Ordering::Greater)
                );
                if violates {
                    emit!(result!(
                        sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }
        PlannedConstraint::MaxExclusive(bound) => {
            for value in value_nodes {
                // The comparison reads the value node's lexical form and datatype
                // IRI, both of which it can BORROW out of the dataset. The owned
                // term is built below, inside the violation, because that is the
                // only place a result needs one.
                let violates = !matches!(
                    range_facet_cmp(value.literal_parts(ds), bound),
                    Some(std::cmp::Ordering::Less)
                );
                if violates {
                    emit!(result!(
                        sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── And (per value node, recursive) ───────────────────────────────────
        PlannedConstraint::And(members) => {
            for value in value_nodes {
                // The recursion focus keeps the value node's own representation,
                // so an interned one is never materialized to be recursed into.
                let focus = value.as_focus(ds);
                let mut all_conform = true;
                for member in members.iter(context.plan) {
                    if !conforms_memoized(context, member, &focus)? {
                        all_conform = false;
                        break;
                    }
                }
                if !all_conform {
                    emit!(result!(
                        sh::AND_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Or (per value node, recursive) ────────────────────────────────────
        PlannedConstraint::Or(members) => {
            for value in value_nodes {
                // As `sh:and`: the recursion focus keeps the value node's own
                // representation.
                let focus = value.as_focus(ds);
                let mut any_conforms = false;
                for member in members.iter(context.plan) {
                    if conforms_memoized(context, member, &focus)? {
                        any_conforms = true;
                        break;
                    }
                }
                if !any_conforms {
                    emit!(result!(
                        sh::OR_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Xone (per value node, recursive) ──────────────────────────────────
        //
        // The verdict is a COUNT and not a boolean — "exactly one" is broken by
        // nought conforming members and by two alike — so, unlike `sh:or`, this
        // arm cannot stop at the first conforming member. Every member is asked,
        // each through the same conformance traversal, and the count is compared
        // against one afterwards.
        PlannedConstraint::Xone(members) => {
            for value in value_nodes {
                // As `sh:and`: the recursion focus keeps the value node's own
                // representation.
                let focus = value.as_focus(ds);
                let mut count = 0usize;
                for member in members.iter(context.plan) {
                    if conforms_memoized(context, member, &focus)? {
                        count += 1;
                    }
                }
                if count != 1 {
                    emit!(result!(
                        sh::XONE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Node (per value node, recursive) ──────────────────────────────────
        PlannedConstraint::Node(inner_shape) => {
            for value in value_nodes {
                // The recursion focus IS the value node, in the representation it
                // already has: no materialization, and no reverse hash probe to
                // recover an identity the value node was already carrying.
                let focus = value.as_focus(ds);
                if !conforms_memoized(context, inner_shape, &focus)? {
                    emit!(result!(
                        sh::NODE_CONSTRAINT_COMPONENT,
                        Some(value.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }

        // ── Property-pair constraints (§4.3): compare the value nodes against
        //    the objects of the given predicate from the SAME focus node. ──────
        PlannedConstraint::Equals(pred) => {
            // Membership is identity, so the comparison stays in `TermId` space
            // end to end: the comparands are collected as ids and never
            // materialized, and only an OFFENDING term is ever built.
            let others = PairComparands::collect(ds, focus_node, pred);
            // The dedup set is only ever touched by an OFFENDING value node, so a
            // conforming focus node never allocates it.
            let mut seen: FastSet<Term> = FastSet::default();
            // Value nodes missing from the predicate's objects…
            for v in value_nodes {
                if !others.contains_value(ds, v) {
                    let term = v.to_term(ds);
                    if seen.insert(term.clone()) {
                        emit!(result!(
                            sh::EQUALS_CONSTRAINT_COMPONENT,
                            focus_node.to_term(ds),
                            Some(term)
                        ));
                    }
                }
            }
            // …and predicate objects missing from the value nodes. Building the
            // value-node index is itself gated on there being a comparand to ask
            // about, so an empty comparand set costs nothing.
            if !others.is_empty() {
                let value_ids = ValueNodeIds::of(ds, value_nodes);
                for oid in others.iter() {
                    if !value_ids.contains(ds, oid) {
                        let other = term_id_to_native(ds, oid);
                        if seen.insert(other.clone()) {
                            emit!(result!(
                                sh::EQUALS_CONSTRAINT_COMPONENT,
                                focus_node.to_term(ds),
                                Some(other)
                            ));
                        }
                    }
                }
            }
            Flow::Continue
        }
        PlannedConstraint::Disjoint(pred) => {
            // Identity check in `TermId` space; only offending value nodes are
            // materialized.
            let others = PairComparands::collect(ds, focus_node, pred);
            for v in value_nodes {
                if others.contains_value(ds, v) {
                    emit!(result!(
                        sh::DISJOINT_CONSTRAINT_COMPONENT,
                        focus_node.to_term(ds),
                        Some(v.to_term(ds))
                    ));
                }
            }
            Flow::Continue
        }
        PlannedConstraint::LessThan(pred) => {
            // Order comparison reads the numeric/string/temporal value space, but
            // it reads it through BORROWED lexical forms on both sides, so neither
            // the value nodes nor the comparands are materialized to compare them.
            for value in pair_order_offenders(ds, focus_node, value_nodes, pred, false) {
                emit!(result!(
                    sh::LESS_THAN_CONSTRAINT_COMPONENT,
                    focus_node.to_term(ds),
                    Some(value)
                ));
            }
            Flow::Continue
        }
        PlannedConstraint::LessThanOrEquals(pred) => {
            for value in pair_order_offenders(ds, focus_node, value_nodes, pred, true) {
                emit!(result!(
                    sh::LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT,
                    focus_node.to_term(ds),
                    Some(value)
                ));
            }
            Flow::Continue
        }

        // ── Qualified value shapes (§4.5.4–4.5.5) ──────────────────────────────
        PlannedConstraint::QualifiedValueShape {
            shape: qshape,
            siblings,
            min_count,
            max_count,
            disjoint,
        } => {
            // A value node counts iff it conforms to the qualified shape AND —
            // under sibling disjointness — conforms to NO sibling qualified shape.
            let mut count = 0u64;
            for v in value_nodes {
                // Each value node is a recursion focus for the qualified shape,
                // in the representation it already has, across every recursive
                // check.
                let focus = v.as_focus(ds);
                if !conforms_memoized(context, qshape, &focus)? {
                    continue;
                }
                let mut sibling_conforms = false;
                if disjoint {
                    for sibling in siblings.iter(context.plan) {
                        if conforms_memoized(context, sibling, &focus)? {
                            sibling_conforms = true;
                            break;
                        }
                    }
                }
                if !sibling_conforms {
                    count += 1;
                }
            }
            if let Some(min) = min_count
                && count < min
            {
                emit!(result!(
                    sh::QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT,
                    focus_node.to_term(ds),
                    None
                ));
            }
            if let Some(max) = max_count
                && count > max
            {
                emit!(result!(
                    sh::QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT,
                    focus_node.to_term(ds),
                    None
                ));
            }
            Flow::Continue
        }

        // ── Sparql (SHACL-AF — $this always binds to the focus node, never to a
        //           path value node.  SHACL-AF spec §3.4: for sh:sparql on a
        //           property shape, $this is still the focus subject; the path
        //           objects are NOT auto-bound.)
        //
        // The constraint blank node may carry its own sh:message / sh:severity;
        // those override the shape-level defaults at eval time.
        // SELECT-form and the SHACL-SPARQL pre-binding restrictions are enforced
        // at shape-load (shapes.rs); a residual evaluation failure (a construct
        // the native engine cannot execute) is surfaced as a hard validation
        // error rather than a panic.
        // The native SPARQL engine runs the validated query text over the dataset,
        // substituting $this for this focus node (SparqlRequest.substitutions).
        PlannedConstraint::Sparql {
            select,
            message: cmsg,
            severity: csev,
        } => {
            let sev = csev.clone().unwrap_or_else(|| severity.clone());
            let msg = cmsg.clone().or_else(|| message.clone());
            // SHACL-SPARQL §5.3.2: on a property shape, the `$PATH` placeholder
            // stands for the shape's path in SPARQL surface syntax.
            let query = substitute_path_placeholder(select, path);
            // SHACL-SPARQL binds `$this` as an owned term in a pre-binding, so
            // this arm materializes the focus node — beside a query evaluation
            // that dwarfs it, and only for shapes that carry a `sh:sparql`.
            let focus_term = focus_node.to_term(ds);
            let produced = crate::sparql::eval_sparql_constraint_view(
                store.sparql_view(),
                &focus_term,
                &query,
                &NamedNode::from(sh::SPARQL_CONSTRAINT_COMPONENT),
                source_shape,
                &sev,
                msg.as_ref(),
                shapes_graph_iri,
                Some(source_shape),
            )
            .map_err(|e| format!("sh:sparql constraint on shape {source_shape}: {e}"))?;
            // A SHACL-SPARQL validator answers in RESULTS rather than in a
            // per-value-node verdict, so the query runs whatever the sink is;
            // what the sink decides is only whether they are kept.
            for produced_result in produced {
                emit!(produced_result);
            }
            Flow::Continue
        }

        // ── Expression (SHACL-AF §5.7) ─────────────────────────────────────────
        // Each value node is evaluated as the focus of the node expression; the
        // constraint is satisfied iff the result is exactly the canonical
        // `"true"^^xsd:boolean` term (`is_true`). A sub-expression evaluation
        // failure is a hard validation error (mirroring sh:sparql). The
        // expression node may carry its own sh:message / sh:severity overriding
        // the shape defaults.
        PlannedConstraint::Expression {
            expr,
            lowered,
            message: cmsg,
            severity: csev,
        } => {
            let sev = csev.clone().unwrap_or_else(|| severity.clone());
            let msg = cmsg.clone().or_else(|| message.clone());
            // Seed the guard with the ambient filter/exists depth so a
            // `sh:filterShape` re-entry through this expression keeps the
            // cross-shape recursion count monotone (fail-closed at the depth
            // ceiling) rather than resetting it per constraint. The guard is
            // hoisted above the loop: `enter`/`exit` are balanced on every path,
            // so its in-flight set is empty between value nodes, and `depth` is
            // loop-invariant — reusing it only avoids re-allocating the set.
            let mut guard = crate::expression::RecursionGuard::with_depth(depth);
            for value_node in value_nodes {
                // A node expression evaluates over the owned term model (it may
                // produce non-interned terms), so resolve the value node here.
                let value_node = value_node.to_term(ds);
                // SHACL 1.2 Node Expressions §7.1 evaluates an expression
                // constraint as `evalExpr(expr, data graph, focusNode,
                // {value: v})`, so the value node under test is BOUND under the
                // name `value` — this is what makes `[ shnex:var "value" ]` resolve
                // inside an expression constraint. The binding is a borrowed stack
                // frame, so the per-value cost is a pointer write, not an
                // allocation.
                let binding = crate::expression::Binding::new(
                    crate::expression::VALUE_VAR,
                    &value_node,
                    crate::expression::Scope::EMPTY,
                );
                let out = crate::expression::eval_planned_node_expr_in_scope(
                    store,
                    &value_node,
                    expr,
                    lowered,
                    context.plan,
                    &mut guard,
                    crate::expression::Scope::bound(&binding),
                )
                .map_err(|e| format!("sh:expression constraint on shape {source_shape}: {e}"))?;
                if !crate::expression::is_true(&out) {
                    emit!({
                        let mut r = result!(
                            sh::EXPRESSION_CONSTRAINT_COMPONENT,
                            Some(value_node.clone())
                        );
                        r.severity.clone_from(&sev);
                        r.message.clone_from(&msg);
                        r
                    });
                }
            }
            Flow::Continue
        }

        // ── NodeByExpression (SHACL 1.2 Node Expressions §7.2) ─────────────────
        // The mirror image of `sh:expression`: the node expression computes the
        // node SHAPES rather than a boolean, and every value node must conform to
        // each shape it produces. Per the spec's own
        // `evalExpr(expr, data graph, v, {})` the value node is the FOCUS NODE and
        // the scope is EMPTY — unlike §7.1, which binds `value`.
        PlannedConstraint::NodeByExpression {
            expr,
            lowered,
            shapes,
            index,
            message: cmsg,
            severity: csev,
        } => {
            let sev = csev.clone().unwrap_or_else(|| severity.clone());
            let msg = cmsg.clone().or_else(|| message.clone());
            // The index is filled at the end of the shapes-graph parse; an unfilled
            // one means this constraint escaped that parse, which would silently
            // make every conformance check vacuous. Refuse loudly instead.
            let resolved = shapes.get().ok_or_else(|| {
                format!(
                    "sh:nodeByExpression constraint on shape {source_shape}: the shapes graph's \
                     shape index was never filled"
                )
            })?;
            let mut guard = crate::expression::RecursionGuard::with_depth(depth);
            let next_depth = depth.saturating_add(1);
            for value_node in value_nodes {
                // Preserve the interned identity before materializing the term, so
                // the conformance re-entry below does not pay a reverse hash probe
                // to recover what the value node already knew.
                let value_id = value_node.as_id(ds);
                let value_node = value_node.to_term(ds);
                let produced = crate::expression::eval_planned_node_expr(
                    store,
                    &value_node,
                    expr,
                    lowered,
                    context.plan,
                    &mut guard,
                )
                .map_err(|e| {
                    format!("sh:nodeByExpression constraint on shape {source_shape}: {e}")
                })?;
                for shape_node in produced {
                    let shape_plan = context
                        .plan
                        .indexed(index, &shape_node.to_string(), resolved)?
                        .ok_or_else(|| {
                            format!(
                                "sh:nodeByExpression constraint on shape {source_shape}: \
                                 {shape_node} is not a shape of this shapes graph"
                            )
                        })?;
                    // A failing conformance check is a FAILURE the spec requires be
                    // produced, so it propagates rather than counting as "does not
                    // conform".
                    //
                    // The AMBIENT lowering is threaded through, exactly as every
                    // other recursive arm does: rebuilding one here would run the
                    // whole cycle-aware shape walk once per value node per produced
                    // shape. The lowering covers the shapes this index resolves to
                    // because the walk ENTERS the index itself.
                    let conforms = conforms_with_id_depth(
                        store,
                        &value_id.map_or_else(
                            || FocusNode::Foreign(value_node.clone()),
                            FocusNode::Interned,
                        ),
                        shape_plan,
                        next_depth,
                    )
                    .map_err(|e| {
                        format!("sh:nodeByExpression constraint on shape {source_shape}: {e}")
                    })?;
                    if !conforms {
                        emit!({
                            let mut r = result!(
                                sh::NODE_BY_EXPRESSION_CONSTRAINT_COMPONENT,
                                Some(value_node.clone())
                            );
                            r.severity.clone_from(&sev);
                            r.message.clone_from(&msg);
                            r
                        });
                    }
                }
            }
            Flow::Continue
        }

        // ── Custom constraint components (SHACL-SPARQL) ─────────────────────────
        PlannedConstraint::Component {
            component,
            source_shape,
            bindings,
            validator,
            message: cmsg,
            severity: csev,
        } => {
            let sev = csev.clone().unwrap_or_else(|| severity.clone());
            let msg = cmsg.clone().or_else(|| message.clone());
            let dataset = store.sparql_view();
            // The custom-component validators run over the owned term model; resolve
            // the value nodes for the ASK validator's per-value binding.
            let value_terms: Vec<Term> = value_nodes.iter().map(|v| v.to_term(ds)).collect();
            // As `sh:sparql`: the validators speak owned terms, so the focus node
            // is materialized here, once, for a shape that declares one.
            let focus_term = focus_node.to_term(ds);
            let produced = match validator {
                ComponentValidator::Ask { .. } => crate::components::eval_ask_validator(
                    dataset,
                    &focus_term,
                    &value_terms,
                    validator,
                    bindings,
                    component,
                    source_shape,
                    path,
                    &sev,
                    msg.as_ref(),
                    shapes_graph_iri,
                    Some(source_shape),
                ),
                ComponentValidator::Select { .. } => crate::components::eval_select_validator(
                    dataset,
                    &focus_term,
                    validator,
                    bindings,
                    component,
                    source_shape,
                    path,
                    &sev,
                    msg.as_ref(),
                    shapes_graph_iri,
                    Some(source_shape),
                ),
            }
            .map_err(|e| format!("component validator on shape {source_shape}: {e}"))?;
            // As for `sh:sparql`: a custom component answers in results, so the
            // validator runs whatever the sink is.
            for produced_result in produced {
                emit!(produced_result);
            }
            Flow::Continue
        }
    })
}

/// Replace the SHACL-SPARQL `$PATH` / `?PATH` placeholder with the property
/// shape's path rendered in SPARQL property-path surface syntax. A node-shape
/// constraint (`path == None`) and a query without the placeholder pass
/// through unchanged.
pub(crate) fn substitute_path_placeholder(select: &str, path: Option<&Path>) -> String {
    static PATH_PLACEHOLDER: OnceLock<regex::Regex> = OnceLock::new();
    let Some(path) = path else {
        return select.to_owned();
    };
    let re = PATH_PLACEHOLDER
        .get_or_init(|| regex::Regex::new(r"[$?]PATH\b").expect("static regex is valid"));
    if !re.is_match(select) {
        return select.to_owned();
    }
    let rendered = path::path_to_sparql(path);
    re.replace_all(select, regex::NoExpand(&rendered))
        .into_owned()
}

// ── Helper functions ───────────────────────────────────────────────────────────

/// Apply the `whiteSpace` = `collapse` normalization every XSD atomic datatype
/// below fixes, as far as a one-token lexical space can observe it.
///
/// XSD 1.1 Part 2 §4.3.6 defines the two steps `collapse` performs:
///
/// > `replace` — All occurrences of `#x9` (tab), `#xA` (line feed) and `#xD`
/// > (carriage return) are replaced with `#x20` (space).
/// >
/// > `collapse` — After the processing implied by `replace`, contiguous
/// > sequences of `#x20`s are collapsed to a single `#x20`, and any `#x20` at
/// > the start or end of the string are then removed.
///
/// So the whole normalization quantifies over exactly four code points — the
/// same four as XML `S`, "`S ::= (#x20 | #x9 | #xD | #xA)+`" (XML 1.0 5e §2.3
/// `[3]`) — and [`purrdf_iri::terminals::is_ws_char`] is that class.
///
/// # Why trimming is the whole of `collapse` for these datatypes
///
/// `collapse` also squeezes INTERNAL runs, which this does not. That is sound
/// here and only here: every lexical space this helper feeds
/// (`xsd:integer`, `xsd:decimal`, `xsd:double`, `xsd:float`, `xsd:boolean`) is a
/// single token containing no `#x20` at all, so an internal run survives
/// `collapse` as one `#x20` and is refused by the token grammar either way. The
/// verdict is identical; only the trimming is observable.
///
/// # The direction of the error this replaces
///
/// These sites called [`str::trim`], which trims the Unicode `White_Space`
/// property — twenty-six code points where the datatype names four. That is
/// **over-acceptance**: a SHACL validator handed `"\u{A0}42"` as an
/// `xsd:integer` stripped the NO-BREAK SPACE and reported a conforming typed
/// literal, though U+00A0 is not touched by `replace` or `collapse` and the
/// value is not in `xsd:integer`'s lexical space at all. `sh:datatype`
/// conformance is a claim about the datatype, so accepting a literal the
/// datatype refuses makes the report wrong, not merely lenient.
fn collapse_trim(s: &str) -> &str {
    s.trim_matches(purrdf_iri::terminals::is_ws_char)
}

/// `xsd:integer` lexical space: optional sign then one-or-more ASCII digits.
/// Unbounded — no native-int overflow.
///
/// `xsd:integer` fixes `whiteSpace` = `collapse` (XSD 1.1 Part 2 §3.4.13), so
/// the lexical form is trimmed with [`collapse_trim`] and not with
/// [`str::trim`].
fn is_xsd_integer_lexical(s: &str) -> bool {
    let s = collapse_trim(s);
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `xsd:decimal` lexical space: optional sign then digits with an optional
/// single '.' — NO exponent. At least one digit must be present.
///
/// `xsd:decimal` fixes `whiteSpace` = `collapse` (XSD 1.1 Part 2 §3.3.3), so the
/// lexical form is trimmed with [`collapse_trim`] and not with [`str::trim`].
fn is_xsd_decimal_lexical(s: &str) -> bool {
    let s = collapse_trim(s);
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() {
        return false;
    }
    let mut seen_dot = false;
    let mut seen_digit = false;
    for b in body.bytes() {
        match b {
            b'0'..=b'9' => seen_digit = true,
            b'.' if !seen_dot => seen_dot = true,
            _ => return false, // rejects 'e'/'E' (scientific notation) and any other char
        }
    }
    seen_digit
}

/// Check that a `Term` satisfies `sh:datatype` requirements.
///
/// - Must be a `Literal` whose datatype IRI matches `dt_iri` EXACTLY (spec
///   §4.1.2: sh:datatype compares the rdf:type of the literal, so a
///   `"55"^^xsd:integer` value violates a shape requiring `xsd:byte` even
///   though 55 fits in a byte — W3C `core/property/datatype-ill-formed`).
/// - On the exact match, additionally validates the lexical form for common
///   XSD types (xsd:integer unbounded, xsd:decimal no scientific notation,
///   xsd:double/float, xsd:boolean), and for a DERIVED integer type validates
///   the VALUE space: the native codec keeps `"-2"^^xsd:nonNegativeInteger`
///   faithfully typed, but the value is outside the derived range and must
///   violate.
#[cfg(test)]
fn check_datatype(value: &Term, dt_iri: &NamedNode) -> bool {
    let Term::Literal(lit) = value else {
        return false;
    };
    check_datatype_parts(lit.value(), lit.datatype_str(), dt_iri)
}

fn check_value_datatype(value: &ValueNode, ds: &impl ShaclRead, dt_iri: &NamedNode) -> bool {
    value
        .literal_parts(ds)
        .is_some_and(|(lexical, datatype)| check_datatype_parts(lexical, datatype, dt_iri))
}

fn check_datatype_parts(lex: &str, stored_dt: &str, dt_iri: &NamedNode) -> bool {
    if stored_dt != dt_iri.as_str() {
        return false;
    }
    // Exact datatype-IRI match. For the primitive types validate the lexical
    // form; for a DERIVED integer type additionally validate the VALUE space.
    // `XSD_INTEGER` is the canonical fold target the value-space check keys
    // on, so check the stored value against xsd:integer first.
    if is_derived_integer_type(dt_iri.as_str()) {
        return derived_integer_matches(XSD_INTEGER, dt_iri.as_str(), lex);
    }
    xsd_lexical_valid(dt_iri.as_str(), lex)
}

/// The XSD integer-derived datatype IRIs whose VALUE space is narrower than
/// `xsd:integer` (so an exact datatype match still requires a range check).
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

fn is_derived_integer_type(dt: &str) -> bool {
    matches!(
        dt,
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
            | "http://www.w3.org/2001/XMLSchema#positiveInteger"
            | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
            | "http://www.w3.org/2001/XMLSchema#negativeInteger"
            | "http://www.w3.org/2001/XMLSchema#long"
            | "http://www.w3.org/2001/XMLSchema#int"
            | "http://www.w3.org/2001/XMLSchema#short"
            | "http://www.w3.org/2001/XMLSchema#byte"
            | "http://www.w3.org/2001/XMLSchema#unsignedLong"
            | "http://www.w3.org/2001/XMLSchema#unsignedInt"
            | "http://www.w3.org/2001/XMLSchema#unsignedShort"
            | "http://www.w3.org/2001/XMLSchema#unsignedByte"
    )
}

/// Lexical-form validity for an exact datatype-IRI match. Unknown datatypes are
/// accepted (no lexical facet enforced).
fn xsd_lexical_valid(dt: &str, lex: &str) -> bool {
    match dt {
        "http://www.w3.org/2001/XMLSchema#integer" => is_xsd_integer_lexical(lex),
        "http://www.w3.org/2001/XMLSchema#decimal" => is_xsd_decimal_lexical(lex),
        // `xsd:double` / `xsd:float` / `xsd:boolean` each fix
        // `whiteSpace` = `collapse` (XSD 1.1 Part 2 §3.3.5, §3.3.4, §3.3.2), so
        // they are trimmed with the four code points `collapse` names and not
        // with `str::trim`'s Unicode `White_Space` property.
        "http://www.w3.org/2001/XMLSchema#double" => {
            purrdf_xsd::parse_double_xsd10(collapse_trim(lex)).is_ok()
        }
        "http://www.w3.org/2001/XMLSchema#float" => {
            purrdf_xsd::parse_float_xsd10(collapse_trim(lex)).is_ok()
        }
        "http://www.w3.org/2001/XMLSchema#boolean" => {
            matches!(collapse_trim(lex), "true" | "false" | "1" | "0")
        }
        _ => true,
    }
}

/// Whether a literal that oxigraph stored as the canonical base type satisfies a
/// shape's required XSD *derived* integer type, by validating the lexical value
/// against the derived type's value space. Every XSD integer-derived type
/// canonicalizes to `xsd:integer` in oxigraph; only that base is considered here.
fn derived_integer_matches(stored_dt: &str, required_dt: &str, lex: &str) -> bool {
    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    if stored_dt != XSD_INTEGER || !is_xsd_integer_lexical(lex) {
        return false;
    }
    // The same `whiteSpace` = `collapse` trim `is_xsd_integer_lexical` just
    // applied: two trims of one lexical form that disagreed about the class
    // would let a value pass the lexical gate and then be re-read differently by
    // the bound check.
    let trimmed = collapse_trim(lex);
    // For sign-constrained but unbounded types, fall back to a lexical sign check
    // when the magnitude exceeds i128 (astronomically large; never in practice).
    let value = trimmed.parse::<i128>().ok();
    let is_negative = || value.map_or_else(|| trimmed.starts_with('-'), |n| n < 0);
    let is_positive = || value.map_or_else(|| !trimmed.starts_with('-'), |n| n > 0);
    let is_zero = || value == Some(0);
    match required_dt {
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger" => !is_negative(),
        "http://www.w3.org/2001/XMLSchema#positiveInteger" => is_positive(),
        "http://www.w3.org/2001/XMLSchema#nonPositiveInteger" => is_negative() || is_zero(),
        "http://www.w3.org/2001/XMLSchema#negativeInteger" => is_negative(),
        "http://www.w3.org/2001/XMLSchema#long" => trimmed.parse::<i64>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#int" => trimmed.parse::<i32>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#short" => trimmed.parse::<i16>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#byte" => trimmed.parse::<i8>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedLong" => trimmed.parse::<u64>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedInt" => trimmed.parse::<u32>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedShort" => trimmed.parse::<u16>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedByte" => trimmed.parse::<u8>().is_ok(),
        _ => false,
    }
}

/// Check that a `Term` satisfies `sh:nodeKind`.
///
/// The constraint arm evaluates [`check_value_kind`] on the borrowed
/// [`ValueKind`] so a conforming value node is never materialized; this
/// owned-`Term` form is kept verbatim as the oracle the tests pin it against.
#[cfg(test)]
fn check_node_kind(value: &Term, kind: &NodeKindValue) -> bool {
    matches!(
        (value, kind),
        (
            Term::NamedNode(_),
            NodeKindValue::Iri | NodeKindValue::BlankNodeOrIri | NodeKindValue::IriOrLiteral
        ) | (
            Term::BlankNode(_),
            NodeKindValue::BlankNode
                | NodeKindValue::BlankNodeOrIri
                | NodeKindValue::BlankNodeOrLiteral
        ) | (
            Term::Literal(_),
            NodeKindValue::Literal
                | NodeKindValue::BlankNodeOrLiteral
                | NodeKindValue::IriOrLiteral
        )
    )
}

/// `sh:nodeKind` on the bare discriminant: `Iri`/`Blank`/`Literal` match the
/// `sh:nodeKind` values that include them; a `Triple` (quoted triple term)
/// matches NO node kind, exactly as the owned-`Term` form's fall-through.
fn check_value_kind(value: ValueKind, kind: &NodeKindValue) -> bool {
    matches!(
        (value, kind),
        (
            ValueKind::Iri,
            NodeKindValue::Iri | NodeKindValue::BlankNodeOrIri | NodeKindValue::IriOrLiteral
        ) | (
            ValueKind::Blank,
            NodeKindValue::BlankNode
                | NodeKindValue::BlankNodeOrIri
                | NodeKindValue::BlankNodeOrLiteral
        ) | (
            ValueKind::Literal,
            NodeKindValue::Literal
                | NodeKindValue::BlankNodeOrLiteral
                | NodeKindValue::IriOrLiteral
        )
    )
}

/// Return the character count of the lexical form of `value`, or `None` for
/// blank nodes (which violate `sh:minLength`).
///
/// The constraint arms take the same count from `ValueNode::lexical` (which
/// agrees with this variant-for-variant) so a conforming value node is never
/// materialized; this owned-term form remains the reference the tests pin.
#[cfg(test)]
fn lexical_length(value: &Term) -> Option<usize> {
    match value {
        Term::Literal(lit) => Some(lit.value().chars().count()),
        Term::NamedNode(nn) => Some(nn.as_str().chars().count()),
        _ => None,
    }
}

/// Whether a value node's language tag matches any entry in an `sh:languageIn`
/// list, using SHACL basic-filtering / prefix semantics (RFC 4647 §3.3.1).
///
/// A value tag matches an entry iff, comparing case-insensitively, it equals the
/// entry or extends it at a subtag boundary (e.g. `"en"` matches `"en"` and
/// `"en-US"`, but not `"eng"`). A non-language-tagged literal (or any non-literal)
/// never matches, so it always violates the constraint.
///
/// The constraint arm feeds `ValueNode::language` straight into
/// [`language_tag_matches_any`]; this owned-`Term` form is the tests' oracle.
#[cfg(test)]
fn language_matches_any(value: &Term, tags: &[String]) -> bool {
    let Term::Literal(lit) = value else {
        return false;
    };
    let Some(lang) = lit.language() else {
        return false;
    };
    language_tag_matches_any(lang, tags)
}

/// [`language_matches_any`] on an already-borrowed language tag.
fn language_tag_matches_any(lang: &str, tags: &[String]) -> bool {
    // RFC 4647 basic filtering, case-insensitive, allocation-free: compare ASCII
    // slices in place rather than lowercasing `lang` and each `entry` per call.
    tags.iter().any(|entry| {
        if lang.eq_ignore_ascii_case(entry) {
            return true;
        }
        lang.len() > entry.len()
            && lang.as_bytes()[entry.len()] == b'-'
            && lang[..entry.len()].eq_ignore_ascii_case(entry)
    })
}

/// Parse a numeric value (xsd:integer, xsd:decimal, xsd:double) as `f64`.
///
/// `pub(crate)` for [`crate::plan`], which runs it over a range facet's BOUND at
/// stage 0 — the bound is a constant of the shape, so this whole test belongs
/// there and not on the per-value-node path. `None` is an ordinary answer for a
/// bound, never an error: see the range-facet comparison below for what happens
/// to a bound that is not numeric.
pub(crate) fn numeric_value(term: &Term) -> Option<f64> {
    let Term::Literal(lit) = term else {
        return None;
    };
    numeric_parts(lit.value(), lit.datatype_str())
}

/// [`numeric_value`] over a literal's BORROWED lexical form and datatype IRI.
///
/// The value-node side of every range facet reaches the comparison this way, out
/// of [`ValueNode::literal_parts`], so a conforming value node is compared without
/// ever being materialized into an owned [`Term`].
fn numeric_parts(lexical: &str, datatype: &str) -> Option<f64> {
    const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
    // The full XSD numeric lattice: the primitives plus EVERY derived integer
    // datatype. The set must match the rest of the engine (see
    // `instance.rs::numeric_or_bool_scalar`); the previous list omitted the
    // derived/unsigned integers (e.g. `xsd:nonNegativeInteger`), so a faithful
    // `"1"^^xsd:nonNegativeInteger` value read as non-numeric and spuriously
    // violated every `sh:minInclusive`/`sh:maxInclusive` facet. (The omission was
    // masked while data round-tripped through oxigraph's NT serializer, which
    // value-space-normalized such literals to `xsd:integer`; the oxigraph-free
    // path is the faithful one and exposes the gap.)
    let local = datatype.strip_prefix(XSD_NS)?;
    if matches!(
        local,
        "integer"
            | "decimal"
            | "double"
            | "float"
            | "long"
            | "int"
            | "short"
            | "byte"
            | "nonNegativeInteger"
            | "positiveInteger"
            | "nonPositiveInteger"
            | "negativeInteger"
            | "unsignedLong"
            | "unsignedInt"
            | "unsignedShort"
            | "unsignedByte"
    ) {
        // Every datatype listed above fixes `whiteSpace` = `collapse`, so the
        // lexical form is trimmed with the four code points that names — see
        // [`collapse_trim`].
        collapse_trim(lexical).parse::<f64>().ok()
    } else {
        None
    }
}

/// Value-space comparison for the range facets (`sh:minInclusive`,
/// `sh:maxInclusive`, `sh:minExclusive`, `sh:maxExclusive`):
///
/// - two numeric literals compare by numeric value ([`numeric_value`], the
///   full XSD numeric lattice);
/// - two temporal literals (`xsd:dateTime` / `xsd:date` / `xsd:time`) compare
///   in the XSD VALUE space via `purrdf-xsd` — a timezone-carrying value
///   against a timezone-less one follows the ±14:00 rule, whose indeterminate
///   overlap is `None`;
/// - anything else is incomparable → `None`, which every facet treats as a
///   violation (per spec, a value that cannot be compared to the bound fails).
///
/// The BOUND arrives with its numeric parse already done — it is a constant of
/// the shape, so [`numeric_value`] ran over it once at stage 0 instead of once
/// per value node. The three-way structure below is unchanged and deliberately
/// so: the bound's parse yielding `None` is **not** an error and never was. It
/// means one of two ordinary things, and both still happen here exactly as they
/// did when the parse ran inline:
///
/// * the bound is an `xsd:dateTime` / `xsd:date` / `xsd:time` literal, so the
///   fall-through hands it to [`temporal_value_cmp`] and the facet compares in
///   the XSD temporal value space; or
/// * the bound is neither numeric nor temporal (including a numeric datatype
///   whose lexical form does not parse), so the temporal comparison also yields
///   `None`, the pair is incomparable, and every facet reports the violation the
///   spec calls for.
///
/// Refusing such a shapes graph at preparation time would reject inputs that
/// validate today — an over-refusal, which is the same defect as a dropped result
/// wearing the costume of strictness.
/// The VALUE NODE arrives as [`ValueNode::literal_parts`] — its borrowed lexical
/// form and datatype IRI — rather than as an owned [`Term`], and `None` is a value
/// node that is not a literal at all. Both halves of the comparison read exactly
/// those two strings, so materializing the term to run them allocated a `String`
/// per value node per facet and threw both away again for every conforming one.
/// `None` reaches the same verdict it always did: a non-literal is neither numeric
/// nor temporal, so it is incomparable, so it violates.
fn range_facet_cmp(
    value: Option<(&str, &str)>,
    bound: RangeBound<'_>,
) -> Option<std::cmp::Ordering> {
    let (lexical, datatype) = value?;
    if let (Some(v), Some(b)) = (numeric_parts(lexical, datatype), bound.numeric()) {
        return v.partial_cmp(&b);
    }
    // The temporal fall-through, preserved verbatim. A bound that is not a literal
    // is not a temporal one either, and was already `None` from the two-literal
    // guard this replaces.
    let Term::Literal(bound_literal) = bound.term() else {
        return None;
    };
    temporal_parts_cmp(
        lexical,
        datatype,
        bound_literal.value(),
        bound_literal.datatype_str(),
    )
}

/// XSD temporal value-space comparison of two literals, by their borrowed lexical
/// forms and datatype IRIs. `None` when either side is not an
/// `xsd:dateTime`/`xsd:date`/`xsd:time` literal, when a lexical form is invalid,
/// or when the XSD partial order is indeterminate.
fn temporal_parts_cmp(
    a_lexical: &str,
    a_datatype: &str,
    b_lexical: &str,
    b_datatype: &str,
) -> Option<std::cmp::Ordering> {
    const TEMPORAL: [&str; 3] = [
        "http://www.w3.org/2001/XMLSchema#dateTime",
        "http://www.w3.org/2001/XMLSchema#date",
        "http://www.w3.org/2001/XMLSchema#time",
    ];
    if !TEMPORAL.contains(&a_datatype) || !TEMPORAL.contains(&b_datatype) {
        return None;
    }
    let va = purrdf_xsd::parse_by_iri(a_lexical, a_datatype).ok()??;
    let vb = purrdf_xsd::parse_by_iri(b_lexical, b_datatype).ok()??;
    purrdf_xsd::value_cmp(&va, &vb)
}

/// Term equality: two terms are equal iff their string representations match
/// (oxigraph's `PartialEq` does the right thing for typed literals).
fn terms_equal(a: &Term, b: &Term) -> bool {
    a == b
}

/// Distinct comparands a [`PairComparands`] holds inline before it allocates.
///
/// A property-pair constraint compares against the objects of ONE predicate from
/// ONE focus node, which in practice is a handful of terms; inline storage is what
/// makes the conforming change path allocate nothing for the comparand set.
/// Distinct language tags one `sh:uniqueLang` tally holds before spilling.
///
/// The tally is indexed by DISTINCT tag, not by value node, so this is a bound
/// on how many languages one focus node labels itself in — not on how many
/// labels it has. A focus node past this bound spills the buffer once and keeps
/// working; nothing about the verdict or the reported messages changes.
const UNIQUE_LANG_INLINE: usize = 8;

const PAIR_COMPARANDS_INLINE: usize = 4;

/// Distinct comparands past which [`PairComparands`] builds a membership index.
///
/// Below it, dedup and membership are a linear scan over at most this many `u32`s
/// — cheaper than a hash probe and, crucially, free of allocation. Above it the
/// scan would be quadratic in the predicate's degree, so the one allocation buys
/// back O(1) probes.
const PAIR_COMPARANDS_INDEX_AT: usize = 16;

/// The distinct objects of `(focus, pred, ?)` in the default graph, first-seen
/// order — the "other" side of a property-pair constraint (§4.3) — held as
/// INTERNED IDS.
///
/// Every comparand comes out of the data graph and is therefore interned, and the
/// interner is injective, so identity between a comparand and an interned value
/// node is exactly id equality, and order between two comparable literals is
/// decided from their borrowed lexical forms. Neither needs an owned [`Term`], so
/// the comparand set is never materialized: on the change path a conforming focus
/// node used to pay two string allocations per comparand for terms that were only
/// ever compared and then dropped.
///
/// **Interning order is insertion order, which is NOT value order.** The ids are a
/// comparand's IDENTITY only; `sh:lessThan`/`sh:lessThanOrEquals` resolve each id
/// back to a borrowed [`LiteralView`] and compare in the value space, exactly as
/// they did when the terms were owned.
#[derive(Debug, Default)]
struct PairComparands {
    /// The distinct comparands in first-seen order. The report is order-sensitive
    /// (`sh:equals` emits one result per unmatched comparand), so this is a
    /// sequence, never a set.
    ids: SmallVec<[TermId; PAIR_COMPARANDS_INLINE]>,
    /// Membership index over `ids`, populated only once `ids` grows past
    /// [`PAIR_COMPARANDS_INDEX_AT`]. Empty means "scan `ids`".
    index: IdSet,
}

impl PairComparands {
    /// Collect the comparands of `(focus, pred, ?)` from the default graph.
    fn collect(ds: &impl ShaclRead, focus: &FocusNode, pred: Option<TermId>) -> Self {
        let mut out = Self::default();
        // The comparand predicate's identity was resolved at BIND. `None` means
        // this data graph interns no such IRI, so it has no objects at all — an
        // ordinary empty comparand set, not a failure. A focus node that is not
        // interned has no outgoing quads for the same reason — and it says so
        // from the identity it already carries, without a dictionary probe.
        let (Some(predicate), Some(focus)) = (pred, focus.id()) else {
            return out;
        };
        for quad in quads_for_pattern_ids(
            ds,
            Some(focus),
            Some(predicate),
            None,
            GraphFilter::DefaultGraph,
        ) {
            out.push(quad.o);
        }
        out
    }

    /// Record one comparand, ignoring a repeat.
    ///
    /// Dedup is in id space: every object comes out of the data graph and is
    /// therefore interned, and the interner is injective, so id equality and term
    /// equality are the same relation here.
    fn push(&mut self, id: TermId) {
        if self.contains_id(id) {
            return;
        }
        self.ids.push(id);
        if !self.index.is_empty() {
            self.index.insert(id);
        } else if self.ids.len() > PAIR_COMPARANDS_INDEX_AT {
            self.index.extend(self.ids.iter().copied());
        }
    }

    /// Whether an interned id is one of the comparands.
    fn contains_id(&self, id: TermId) -> bool {
        if self.index.is_empty() {
            self.ids.contains(&id)
        } else {
            self.index.contains(&id)
        }
    }

    /// Whether a VALUE NODE is one of the comparands.
    ///
    /// The id probe is deliberately not the only path. A value node a SHACL-AF
    /// node expression produced may be [`ValueNode::Foreign`] — a term this
    /// dataset never interned — and "not interned, therefore cannot match" is
    /// FALSE: two terms with no shared identity can still be equal. Treating the
    /// id probe as total would silently drop `sh:equals`/`sh:disjoint` violations
    /// over foreign values, which is the exact failure `tests/foreign_value_nodes.rs`
    /// exists to catch. A value node with no id therefore falls back to term
    /// equality against the materialized comparands — an owned term per comparand,
    /// paid only on the foreign path, which no interned value node ever takes.
    fn contains_value(&self, ds: &impl ShaclRead, value: &ValueNode) -> bool {
        match value.as_id(ds) {
            Some(id) => self.contains_id(id),
            None => {
                let term = value.to_term(ds);
                self.iter()
                    .any(|id| terms_equal(&term_id_to_native(ds, id), &term))
            }
        }
    }

    /// Whether there are no comparands at all.
    fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// The comparands in first-seen order.
    fn iter(&self) -> impl Iterator<Item = TermId> + '_ {
        self.ids.iter().copied()
    }
}

/// Value nodes past which [`ValueNodeIds`] indexes rather than scans.
///
/// Same trade as [`PAIR_COMPARANDS_INDEX_AT`]: the index allocates, and the whole
/// point of the change path is that a conforming focus node does not.
const VALUE_NODE_INDEX_AT: usize = 16;

/// Membership over the value nodes' INTERNED ids — the reverse direction of
/// `sh:equals`, which asks which comparands no value node matches.
///
/// A value node with no interned id contributes nothing here, exactly as it did
/// when this was an eagerly collected `FastSet`: a comparand comes out of the data
/// graph and is interned, so a non-interned value node cannot BE that comparand.
/// (That is the opposite direction from [`PairComparands::contains_value`], where
/// the missing id is the value node's own and term equality really can still hold.)
struct ValueNodeIds<'a> {
    /// The value nodes being asked about.
    nodes: &'a [ValueNode],
    /// Populated only above [`VALUE_NODE_INDEX_AT`]; otherwise `nodes` is scanned.
    index: Option<FastSet<TermId>>,
}

impl<'a> ValueNodeIds<'a> {
    /// Index `nodes` if there are enough of them to be worth an allocation.
    fn of(ds: &impl ShaclRead, nodes: &'a [ValueNode]) -> Self {
        let index = (nodes.len() > VALUE_NODE_INDEX_AT)
            .then(|| nodes.iter().filter_map(|v| v.as_id(ds)).collect());
        Self { nodes, index }
    }

    /// Whether some value node carries this interned id.
    fn contains(&self, ds: &impl ShaclRead, id: TermId) -> bool {
        match &self.index {
            Some(index) => index.contains(&id),
            None => self.nodes.iter().any(|v| v.as_id(ds) == Some(id)),
        }
    }
}

/// The value nodes violating `sh:lessThan` (`allow_equal = false`) or
/// `sh:lessThanOrEquals` (`allow_equal = true`) against the objects of `pred`
/// from the same focus node.
///
/// Per spec §4.3.3–4.3.4 a result exists for every offending `(value, other)`
/// pair; a result records only the value node, so a value offending against
/// N comparands yields N results (duplicate tuples — the report is a
/// multiset, matching the W3C suite's expectations). An incomparable pair
/// (per SPARQL `<` semantics) is a violation.
///
/// Both sides are compared through borrowed [`LiteralView`]s, so a CONFORMING
/// focus node materializes neither its value nodes nor its comparands: the
/// returned `Vec` is `Vec::new()` until an offender exists, and `Vec::new()` does
/// not allocate.
fn pair_order_offenders(
    ds: &impl ShaclRead,
    focus: &FocusNode,
    value_nodes: &[ValueNode],
    pred: Option<TermId>,
    allow_equal: bool,
) -> Vec<Term> {
    let others = PairComparands::collect(ds, focus, pred);
    let mut offending: Vec<Term> = Vec::new();
    for v in value_nodes {
        // Identity is an id here; ORDER is not — interning order is insertion
        // order. The comparison key is the value space, read back off each id.
        let left = v.literal_view(ds);
        for o in others.iter() {
            let ok = match compare_literal_views(left, literal_view_of_id(ds, o)) {
                Some(std::cmp::Ordering::Less) => true,
                Some(std::cmp::Ordering::Equal) => allow_equal,
                Some(std::cmp::Ordering::Greater) | None => false,
            };
            if !ok {
                offending.push(v.to_term(ds));
            }
        }
    }
    offending
}

/// SPARQL-style `<` comparison of two terms, as used by `sh:lessThan` /
/// `sh:lessThanOrEquals` (and the same value machinery as the range facets):
///
/// - two numeric literals compare by numeric value ([`numeric_parts`] — the
///   full XSD numeric lattice);
/// - two plain/`xsd:string` literals compare by codepoint order;
/// - two `xsd:boolean` literals compare with `false < true`;
/// - two temporal literals of the SAME datatype (`xsd:dateTime`, `xsd:date`,
///   `xsd:time`) compare lexically — faithful for the canonical same-timezone
///   forms the engine ingests;
/// - anything else (language-tagged literals, IRIs, blank nodes, mixed
///   datatypes) is incomparable → `None`, which the pair constraints treat as a
///   violation per spec.
///
/// `None` on either side is a NON-LITERAL — an IRI, a blank node or a quoted
/// triple — which is incomparable by the last rule above.
fn compare_literal_views(
    a: Option<LiteralView<'_>>,
    b: Option<LiteralView<'_>>,
) -> Option<std::cmp::Ordering> {
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
    const TEMPORAL: [&str; 3] = [
        "http://www.w3.org/2001/XMLSchema#dateTime",
        "http://www.w3.org/2001/XMLSchema#date",
        "http://www.w3.org/2001/XMLSchema#time",
    ];

    let (Some(a), Some(b)) = (a, b) else {
        return None;
    };
    if let (Some(x), Some(y)) = (
        numeric_parts(a.lexical, a.datatype),
        numeric_parts(b.lexical, b.datatype),
    ) {
        return x.partial_cmp(&y);
    }
    // SPARQL `<` is undefined for language-tagged literals.
    if a.language.is_some() || b.language.is_some() {
        return None;
    }
    let (da, db) = (a.datatype, b.datatype);
    if da == XSD_STRING && db == XSD_STRING {
        return Some(a.lexical.cmp(b.lexical));
    }
    if da == XSD_BOOLEAN && db == XSD_BOOLEAN {
        // `xsd:boolean` fixes `whiteSpace` = `collapse` (XSD 1.1 Part 2 §3.3.2),
        // so the lexical form is trimmed with [`collapse_trim`].
        let bool_of = |lex: &str| match collapse_trim(lex) {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        };
        return Some(bool_of(a.lexical)?.cmp(&bool_of(b.lexical)?));
    }
    if da == db && TEMPORAL.contains(&da) {
        return Some(a.lexical.cmp(b.lexical));
    }
    None
}

/// Build a compiled [`CompiledPattern`](purrdf_core::xsd_regex::CompiledPattern)
/// from a `sh:pattern` string and optional `sh:flags` string.
///
/// The typed [`XsdRegexError`](purrdf_core::xsd_regex::XsdRegexError) is
/// returned rather than stringified: the caller records it in the validation
/// report, so a broken shape names its exact defect instead of a bare
/// `PatternConstraintComponent` violation.
///
/// SHACL §4.5.3 defines `sh:pattern` by the SPARQL `REGEX` function, which SPARQL
/// 1.1 §17.4.3.14 in turn defines as an invocation of XPath F&O 3.1 `fn:matches` —
/// so this delegates to [`purrdf_core::xsd_regex::compile`], the one shared
/// translation from that XSD/XPath `regExp` dialect onto the `regex` crate, rather
/// than reimplementing flag mapping or construct translation a second time
/// (ETHOS §O). `sh:pattern`, SPARQL `REGEX`/`REPLACE`, and ShEx `PATTERN` all
/// carry the same accept set and the same semantics as a result.
///
/// Supported flags: `i` (case-insensitive), `s` (dot-all), `m` (multi-line), `x`
/// (remove `#x9`/`#xA`/`#xD`/`#x20` outside character class expressions), and `q`
/// (literal match — the pattern is matched verbatim, with no metacharacters).
/// Any other flag character is a hard error.
///
/// The shared compiler is a **recognizer** of the XSD/XPath grammar, not a
/// pass-through to `regex`: a construct outside that grammar is a named
/// [`XsdRegexError`](purrdf_core::xsd_regex::XsdRegexError), never silently
/// compiled with Rust `regex` semantics. Constructs the grammar *does* define
/// are translated rather than left to `regex`-crate defaults: character-class
/// subtraction (`[a-z-[aeiou]]`), the multi-character escapes `\i \I \c \C`,
/// the narrowed `\s \S \w \W` classes, and `\p{IsX}`/`\P{IsX}` Unicode block
/// escapes.
///
/// The full enumerated accept/reject list lives in
/// [`purrdf_core::xsd_regex`]'s module doc. Two divergences remain, both of
/// which a caller must know: backreferences (`\1`..`\9`) are refused
/// permanently by design, because the `regex` crate's DFA engine cannot
/// backtrack and no rewrite of the source reaches that capability; and under
/// the `m` flag `^` matches immediately after a trailing newline where XPath
/// `fn:matches` does not (pinned by `known_divergence_m_flag_trailing_newline`).
/// Compilation is additionally bounded by three named resource limits
/// ([`MAX_SOURCE_BYTES`](purrdf_core::xsd_regex::MAX_SOURCE_BYTES),
/// [`MAX_TRANSLATED_BYTES`](purrdf_core::xsd_regex::MAX_TRANSLATED_BYTES), and
/// [`MAX_FOLDED_CLASS_ESCAPES`](purrdf_core::xsd_regex::MAX_FOLDED_CLASS_ESCAPES)),
/// each a hard error rather than a truncation or a fallback. All of this is
/// recorded in `docs/CONFORMANCE.md` and pinned by the first-party corpus under
/// `crates/rdf-core/corpus/xsd-regex/`.
fn build_regex(
    pattern: &str,
    flags: Option<&str>,
) -> Result<purrdf_core::xsd_regex::CompiledPattern, purrdf_core::xsd_regex::XsdRegexError> {
    purrdf_core::xsd_regex::compile(pattern, flags.unwrap_or(""))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use ::purrdf::RdfDataset;

    use super::*;
    use crate::report::Severity;
    use crate::shapes::Constraint;
    use crate::term::{Literal, NamedNode};

    /// Build a [`ShaclData`] holder over a projected test dataset: Core lookups and
    /// the SPARQL dataset are the same frozen graph (no shapes-graph overlay).
    fn shacl_data(store: &Arc<RdfDataset>) -> ShaclData {
        ShaclData::new(Arc::clone(store), Arc::clone(store), None)
    }

    const EX: &str = "http://example.org/ns#";
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn ex(local: &str) -> Term {
        nn(&format!("{EX}{local}"))
    }

    fn xsd_lit(value: &str, dt: &str) -> Term {
        Term::Literal(Literal::new_typed_literal(
            value,
            NamedNode::new_unchecked(format!("{XSD}{dt}")),
        ))
    }

    /// The borrowed `ValueNode` accessors the length / node-kind / language
    /// arms now use must agree, variant-for-variant, with the owned-`Term`
    /// oracles (`lexical_length`, `check_node_kind`, `language_matches_any`)
    /// they replaced — for interned AND foreign value nodes of every node kind.
    #[test]
    fn borrowed_value_node_accessors_match_owned_term_oracles() {
        use crate::data::quads_for_pattern_ids;

        let nt = format!(
            "<{EX}s> <{EX}p> <{EX}iri-object> .\n\
             <{EX}s> <{EX}p> _:b0 .\n\
             <{EX}s> <{EX}p> \"plain\" .\n\
             <{EX}s> <{EX}p> \"\" .\n\
             <{EX}s> <{EX}p> \"ünïcödé 日本語\" .\n\
             <{EX}s> <{EX}p> \"tagged\"@en .\n\
             <{EX}s> <{EX}p> \"tagged-region\"@en-US .\n\
             <{EX}s> <{EX}p> \"directional\"@ar--rtl .\n\
             <{EX}s> <{EX}p> \"42\"^^<{XSD}integer> .\n\
             <{EX}s> <{EX}p> <<( <{EX}qs> <{EX}qp> \"qo\" )>> .\n"
        );
        let store = crate::text_ingest::parse_ntriples_to_dataset(&nt).expect("N-Triples parse");
        let ds: &RdfDataset = &store;
        let subject = ds
            .term_id_by_iri(&format!("{EX}s"))
            .expect("subject interned");
        let interned: Vec<ValueNode> =
            quads_for_pattern_ids(ds, Some(subject), None, None, GraphFilter::AnyGraph)
                .map(|q| ValueNode::Interned(q.o))
                .collect();
        assert_eq!(interned.len(), 10, "every object row must be interned");

        let foreign_terms = vec![
            ex("foreign-iri"),
            Term::BlankNode("foreign-blank".to_owned()),
            Term::Literal(Literal::new_simple_literal("foreign plain ünïcödé")),
            Term::Literal(Literal::new_language_tagged_literal_unchecked(
                "foreign tagged",
                "fr-CA",
            )),
            xsd_lit("7", "integer"),
            Term::Triple(Box::new(Triple::new(
                ex("fs"),
                NamedNode::new_unchecked(format!("{EX}fp")),
                Term::Literal(Literal::new_simple_literal("fo")),
            ))),
        ];
        let foreign: Vec<ValueNode> = foreign_terms.into_iter().map(ValueNode::Foreign).collect();

        let kinds = [
            NodeKindValue::Iri,
            NodeKindValue::BlankNode,
            NodeKindValue::Literal,
            NodeKindValue::BlankNodeOrIri,
            NodeKindValue::BlankNodeOrLiteral,
            NodeKindValue::IriOrLiteral,
        ];
        let tag_lists: [Vec<String>; 4] = [
            vec![],
            vec!["en".to_owned()],
            vec!["EN-us".to_owned(), "fr".to_owned()],
            vec!["ar".to_owned(), "eng".to_owned()],
        ];

        let mut seen_kinds = FastSet::default();
        for value in interned.iter().chain(&foreign) {
            let term = value.to_term(ds);
            seen_kinds.insert(value.kind(ds));
            assert_eq!(value.kind(ds), ValueKind::of_term(&term), "{term}");
            assert_eq!(
                value.lexical(ds).map(|s| s.chars().count()),
                lexical_length(&term),
                "{term}"
            );
            let expected_language = match &term {
                Term::Literal(lit) => lit.language(),
                _ => None,
            };
            assert_eq!(value.language(ds), expected_language, "{term}");
            for kind in &kinds {
                assert_eq!(
                    check_value_kind(value.kind(ds), kind),
                    check_node_kind(&term, kind),
                    "{term} / {kind:?}"
                );
            }
            for tags in &tag_lists {
                assert_eq!(
                    value
                        .language(ds)
                        .is_some_and(|lang| language_tag_matches_any(lang, tags)),
                    language_matches_any(&term, tags),
                    "{term} / {tags:?}"
                );
            }
        }
        assert_eq!(
            seen_kinds.len(),
            4,
            "IRI, blank, literal and triple all covered"
        );
    }

    #[test]
    fn numeric_value_covers_all_derived_integer_datatypes() {
        // `numeric_value` must read EVERY xsd numeric-derived
        // datatype, not just the primitives. The omission of the derived/unsigned
        // integers (e.g. xsd:nonNegativeInteger) made a faithful
        // `"1"^^xsd:nonNegativeInteger` value read as non-numeric and spuriously
        // violate sh:minInclusive/sh:maxInclusive — masked only while data
        // round-tripped through oxigraph's value-space-normalizing NT serializer.
        for dt in [
            "integer",
            "decimal",
            "double",
            "float",
            "long",
            "int",
            "short",
            "byte",
            "nonNegativeInteger",
            "positiveInteger",
            "nonPositiveInteger",
            "negativeInteger",
            "unsignedLong",
            "unsignedInt",
            "unsignedShort",
            "unsignedByte",
        ] {
            // nonPositive/negative datatypes accept a non-positive lexical; use "0"
            // for those, "1" otherwise — both must parse to a numeric value.
            let lexical = if dt.contains("nonPositive") || dt.starts_with("negative") {
                "0"
            } else {
                "1"
            };
            assert!(
                numeric_value(&xsd_lit(lexical, dt)).is_some(),
                "xsd:{dt} must be read as numeric"
            );
        }
        // A non-numeric typed literal stays non-numeric.
        assert!(numeric_value(&xsd_lit("x", "string")).is_none());
        // A plain IRI is never numeric.
        assert!(numeric_value(&ex("thing")).is_none());
    }

    fn shape_with(id: &str, constraints: Vec<Constraint>) -> Shape {
        Shape {
            id: ex(id),
            targets: vec![],
            constraints,
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        }
    }

    fn prop_shape(id: &str, path_iri: &str, constraints: Vec<Constraint>) -> Shape {
        use crate::shapes::Path;
        Shape {
            id: ex(id),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex(&format!("{id}-property")),
                path: Path::Predicate(NamedNode::new_unchecked(path_iri)),
                constraints,
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        }
    }

    /// Result-unwrapping shim over [`super::validate_shape`]: the in-crate
    /// tests exercise only infallible constraint paths, so a hard validation
    /// error is a test bug. (An explicitly-defined item shadows the glob
    /// import from `use super::*`.)
    fn validate_shape(
        store: &Arc<RdfDataset>,
        focus: &Term,
        shape: &Shape,
    ) -> Vec<ValidationResult> {
        super::validate_shape(&shacl_data(store), focus, shape)
            .expect("constraint evaluation must not error")
    }

    /// The caller-supplied box-role vocabulary the box-role tests configure
    /// (purrdf mints no vocabulary of its own — these are test terms).
    fn meta_vocab() -> BoxRoleVocab {
        BoxRoleVocab::for_namespace("https://example.org/meta/")
    }

    /// Result-unwrapping shim over [`super::validate_shape_with`], with the
    /// test box-role vocabulary configured.
    fn validate_shape_with_roles(
        store: &Arc<RdfDataset>,
        focus: &Term,
        shape: &Shape,
    ) -> Vec<ValidationResult> {
        validate_shape_with(&shacl_data(store), focus, shape, Some(&meta_vocab()))
            .expect("constraint evaluation must not error")
    }

    fn load_store(ttl: &str) -> Arc<RdfDataset> {
        let dataset = crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("Turtle parse");
        // Apply the same SHACL projection `validate_dataset` uses, so RDF-1.2
        // reifier bindings are materialized as `rdf:reifies` quads the engine's
        // reifier-shape lookup can find (the IR keeps reifiers in a side table).
        crate::engine::shacl_dataset_from_dataset(&dataset).expect("SHACL projection")
    }

    fn component_iri(results: &[ValidationResult]) -> Vec<String> {
        results
            .iter()
            .map(|r| r.source_constraint_component.as_str().to_owned())
            .collect()
    }

    fn role_iris(roles: &[NamedNode]) -> Vec<&str> {
        roles
            .iter()
            .map(super::super::term::NamedNode::as_str)
            .collect()
    }

    fn named_role(role: &str) -> NamedNode {
        NamedNode::from(role)
    }

    // ── minCount ───────────────────────────────────────────────────────────────

    #[test]
    fn min_count_pass() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a ex:p ex:b .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MinCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "should pass with 1 value");
    }

    /// **Two reifiers of one statement are judged in CANONICAL term order, not in
    /// the order the data graph interned them.**
    ///
    /// The reifier lookup is id-native, and interned ids are in INSERTION order —
    /// which is not canonical order and, for this fixture, is its reverse. A
    /// lookup that took id order for canonical order would still find both
    /// reifiers, still judge both, and still report two violations; only their
    /// SEQUENCE would be wrong, and a SHACL report's result order is observable
    /// output. So the two reifiers are made distinguishable in the report — each
    /// breaks a different half of one reifier shape, and the two halves carry
    /// different messages — because two identical results cannot show which order
    /// they were produced in.
    ///
    /// The fixture is deliberately anti-canonical: `ex:zeta` is declared first and
    /// so interns first, while `ex:alpha` sorts first canonically. Reading ids as
    /// if they were canonical yields exactly the reverse of the asserted order,
    /// which is what makes this assertion able to fail.
    #[test]
    fn multiple_reifiers_of_one_statement_are_judged_in_canonical_order() {
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> .\n\
             @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             ex:alice ex:knows ex:bob .\n\
             ex:zeta rdf:reifies <<( ex:alice ex:knows ex:bob )>> .\n\
             ex:zeta ex:source ex:doc .\n\
             ex:alpha rdf:reifies <<( ex:alice ex:knows ex:bob )>> .\n\
             ex:alpha ex:date \"2026-01-01\" .\n",
        );
        let shapes = crate::engine::parse_shapes(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
             @prefix ex: <http://example.org/ns#> .\n\
             ex:Shape a sh:NodeShape ;\n\
                 sh:property [ sh:path ex:knows ; sh:reifierShape ex:ReifierShape ] .\n\
             ex:ReifierShape a sh:NodeShape ;\n\
                 sh:property [ sh:path ex:source ; sh:minCount 1 ; sh:message \"no source\" ] ;\n\
                 sh:property [ sh:path ex:date ; sh:minCount 1 ; sh:message \"no date\" ] .\n",
            None,
        )
        .expect("the reifier shapes graph must parse");
        let shape = shapes
            .node_shapes
            .iter()
            .find(|shape| shape.id == ex("Shape"))
            .expect("ex:Shape must be parsed as a node shape");

        let results = validate_shape(&store, &ex("alice"), shape);
        let messages: Vec<Option<&str>> = results
            .iter()
            .map(|result| result.message.as_deref())
            .collect();
        assert_eq!(
            messages,
            vec![Some("no source"), Some("no date")],
            "ex:alpha sorts before ex:zeta canonically and interns after it, so this order is \
             the canonical one and its reverse is the insertion one"
        );
    }

    #[test]
    fn min_count_fail() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a a ex:Thing .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MinCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinCount"));
    }

    #[test]
    fn property_shape_box_roles_augment_parent_roles() {
        use crate::shapes::Path;

        let vocab = meta_vocab();
        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix meta: <https://example.org/meta/> .\n\
             ex:p meta:graphBoxRole meta:boxRBox .\n\
             ex:a a ex:Thing .\n"
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![named_role(&vocab.box_config_box)],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![named_role(&vocab.box_tbox)],
            rules: vec![],
        };

        let results = validate_shape_with_roles(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        let source_roles = role_iris(&results[0].source_box_roles);
        assert!(source_roles.contains(&vocab.box_tbox.as_str()));
        assert!(source_roles.contains(&vocab.box_config_box.as_str()));
        assert_eq!(
            role_iris(&results[0].path_box_roles),
            [vocab.box_rbox.as_str()]
        );
        let result_roles = role_iris(&results[0].result_box_roles);
        assert!(result_roles.contains(&vocab.box_tbox.as_str()));
        assert!(result_roles.contains(&vocab.box_config_box.as_str()));
        assert!(result_roles.contains(&vocab.box_rbox.as_str()));
    }

    #[test]
    fn box_roles_inactive_without_configured_vocab() {
        use crate::shapes::Path;

        // Same data as `property_shape_box_roles_augment_parent_roles`, but the
        // vocab is NOT configured: the violation still fires, yet no role is
        // looked up or minted — the feature is inactive, not defaulted.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix meta: <https://example.org/meta/> .\n\
             ex:p meta:graphBoxRole meta:boxRBox .\n\
             ex:a a ex:Thing .\n"
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        };

        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "the violation itself must still fire");
        assert_eq!(results[0].source_box_roles, [] as [_; 0]);
        assert_eq!(results[0].path_box_roles, [] as [_; 0]);
        assert_eq!(results[0].result_box_roles, [] as [_; 0]);
    }

    #[test]
    fn reifier_shape_box_roles_preserve_inner_roles() {
        use crate::shapes::Path;

        let vocab = meta_vocab();
        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix rdf: <{RDF}> .\n\
             ex:a ex:p ex:b .\n\
             ex:reifier rdf:reifies <<( ex:a ex:p ex:b )>> .\n"
        ));
        let reifier_shape = Shape {
            id: ex("ReifierShape"),
            targets: vec![],
            constraints: vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}RequiredReifierClass"
            )))],
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![named_role(&vocab.box_config_box)],
            rules: vec![],
        };
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![],
                property_shapes: vec![],
                reifier_shapes: vec![reifier_shape],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![named_role(&vocab.box_tbox)],
            rules: vec![],
        };

        let results = validate_shape_with_roles(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("ReifierShapeConstraintComponent"));
        let source_roles = role_iris(&results[0].source_box_roles);
        assert!(source_roles.contains(&vocab.box_tbox.as_str()));
        assert!(source_roles.contains(&vocab.box_cbox.as_str()));
        assert!(source_roles.contains(&vocab.box_config_box.as_str()));
        let result_roles = role_iris(&results[0].result_box_roles);
        assert!(result_roles.contains(&vocab.box_tbox.as_str()));
        assert!(result_roles.contains(&vocab.box_cbox.as_str()));
        assert!(result_roles.contains(&vocab.box_config_box.as_str()));
    }

    // ── maxCount ───────────────────────────────────────────────────────────────

    #[test]
    fn max_count_pass() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a ex:p ex:b .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty());
    }

    #[test]
    fn max_count_fail() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a ex:p ex:b, ex:c .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxCount"));
    }

    // ── class ──────────────────────────────────────────────────────────────────

    #[test]
    fn class_pass() {
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . ex:a ex:p ex:b . ex:b rdf:type ex:Foo .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty());
    }

    #[test]
    fn class_fail_no_direct_type() {
        // ex:b is typed ex:SubFoo, and there is NO asserted ex:SubFoo
        // rdfs:subClassOf ex:Foo triple in the data — so b is not a SHACL
        // instance of ex:Foo and the constraint fails. (We honor asserted
        // subClassOf, but invent none: no reasoner runs.)
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . ex:a ex:p ex:b . ex:b rdf:type ex:SubFoo .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Class"));
    }

    #[test]
    fn class_pass_asserted_subclass() {
        // ex:b is typed ex:SubFoo and the data ASSERTS ex:SubFoo rdfs:subClassOf
        // ex:Foo, so b is a SHACL instance of ex:Foo (SHACL §4.2.5) and the
        // sh:class ex:Foo constraint conforms — matching pySHACL.
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . ex:a ex:p ex:b . ex:b rdf:type ex:SubFoo . ex:SubFoo rdfs:subClassOf ex:Foo .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "asserted subClassOf must make ex:b a SHACL instance of ex:Foo"
        );
    }

    #[test]
    fn class_pass_transitive_subclass() {
        // Transitive: ex:b a ex:C, ex:C ⊑ ex:B, ex:B ⊑ ex:A → b is an A-instance.
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . ex:a ex:p ex:b . ex:b rdf:type ex:C . ex:C rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}A"
            )))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "transitive asserted subClassOf must be honored"
        );
    }

    // ── datatype ───────────────────────────────────────────────────────────────

    #[test]
    fn datatype_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"42\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}integer"
            )))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn datatype_fail_wrong_type() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"hello\"^^<{XSD}string> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}integer"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Datatype"));
    }

    #[test]
    fn datatype_fail_lexically_invalid() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:n \"notanumber\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}n"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}integer"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Datatype"));
    }

    // ── datatype derived-integer (oxigraph canonicalization) ────────────────────

    #[test]
    fn datatype_derived_nonneg_integer_pass() {
        // Oxigraph stores "5"^^xsd:nonNegativeInteger as "5"^^xsd:integer, but a
        // shape requiring xsd:nonNegativeInteger must still accept it (value 5 is
        // in range) — matching pySHACL. Pre-fix this produced a false violation.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:n \"5\"^^<{XSD}nonNegativeInteger> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}n"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}nonNegativeInteger"
            )))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "in-range derived-integer value must conform under canonicalization"
        );
    }

    #[test]
    fn derived_integer_value_space() {
        let int = "http://www.w3.org/2001/XMLSchema#integer";
        let nn = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
        let pos = "http://www.w3.org/2001/XMLSchema#positiveInteger";
        let neg = "http://www.w3.org/2001/XMLSchema#negativeInteger";
        let byte = "http://www.w3.org/2001/XMLSchema#byte";
        // nonNegativeInteger: >= 0
        assert!(derived_integer_matches(int, nn, "5"));
        assert!(derived_integer_matches(int, nn, "0"));
        assert!(!derived_integer_matches(int, nn, "-3"));
        // positiveInteger: > 0 (zero excluded)
        assert!(derived_integer_matches(int, pos, "1"));
        assert!(!derived_integer_matches(int, pos, "0"));
        // negativeInteger: < 0
        assert!(derived_integer_matches(int, neg, "-2"));
        assert!(!derived_integer_matches(int, neg, "0"));
        // byte: -128..=127
        assert!(derived_integer_matches(int, byte, "127"));
        assert!(!derived_integer_matches(int, byte, "128"));
        // only the xsd:integer base is the canonical fold target; a non-integer
        // stored type or a non-numeric lexical form never matches a derived type.
        assert!(!derived_integer_matches(
            "http://www.w3.org/2001/XMLSchema#string",
            nn,
            "5"
        ));
        assert!(!derived_integer_matches(int, nn, "x"));
    }

    // ── nodeKind ───────────────────────────────────────────────────────────────

    #[test]
    fn node_kind_iri_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::NodeKind(NodeKindValue::Iri)],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn node_kind_iri_fail_literal() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::NodeKind(NodeKindValue::Iri)],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("NodeKind"));
    }

    // ── in ─────────────────────────────────────────────────────────────────────

    #[test]
    fn in_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:color \"red\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}color"),
            vec![Constraint::In(vec![
                Term::Literal(Literal::new_simple_literal("red")),
                Term::Literal(Literal::new_simple_literal("green")),
            ])],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn in_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:color \"blue\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}color"),
            vec![Constraint::In(vec![
                Term::Literal(Literal::new_simple_literal("red")),
                Term::Literal(Literal::new_simple_literal("green")),
            ])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("In"));
    }

    // ── hasValue ───────────────────────────────────────────────────────────────

    #[test]
    fn has_value_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b, ex:c ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::HasValue(ex("b"))]);
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn has_value_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:c ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::HasValue(ex("b"))]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("HasValue"));
    }

    // ── pattern ────────────────────────────────────────────────────────────────

    #[test]
    fn pattern_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"ABC\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "^[A-Z]+$".to_owned(),
                flags: None,
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn pattern_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"abc\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "^[A-Z]+$".to_owned(),
                flags: None,
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Pattern"));
    }

    #[test]
    fn pattern_with_flags_case_insensitive() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"abc\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "^[A-Z]+$".to_owned(),
                flags: Some("i".to_owned()),
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        // With flag "i", lowercase should now pass.
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    /// `sh:flags "q"` compiles and matches its pattern literally, through the
    /// real `Constraint::Pattern` validation path. It used to hard-fail as an
    /// "unsupported flag", so this pins the behaviour change end to end rather
    /// than only at `build_regex`.
    #[test]
    fn pattern_q_flag_literal_match_end_to_end() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"a.c\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "a.c".to_owned(),
                flags: Some("q".to_owned()),
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        // Under "q", "." is a literal dot, so the exact literal value matches.
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());

        // A value where "." would only match under wildcard semantics must
        // still be rejected: literal-match discipline, not accidental laxity.
        let store2 = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"abc\" ."));
        let results = validate_shape(&store2, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Pattern"));
    }

    /// `\i`/`\c` (XSD's XML-name multi-character escapes) used to fail to
    /// compile at all under the raw `regex`-crate pass-through — `\i` is not one
    /// of that crate's escapes — and now validate correctly through the real
    /// `Constraint::Pattern` path.
    #[test]
    fn pattern_xml_name_escapes_end_to_end() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"abc123\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: r"^\i\c*$".to_owned(),
                flags: None,
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        // "abc123": 'a' is a valid XML NameStartChar, and 'b','c','1','2','3'
        // are all valid XML NameChars.
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());

        // "1abc" starts with a digit, which is a NameChar but not a
        // NameStartChar, so it must fail \i at the first position.
        let store2 = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"1abc\" ."));
        let results = validate_shape(&store2, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Pattern"));
    }

    /// The dialect the SHACL seam actually validates in, end to end through
    /// `Constraint::Pattern` rather than through `build_regex` alone.
    ///
    /// The SPARQL seam has an equivalent test
    /// (`regex_evaluates_the_xsd_dialect_not_the_regex_crates`); this is the
    /// SHACL twin, because `sh:pattern` is a separate call site of the same
    /// shared translator and nothing pinned its dialect behaviour end to end.
    /// Each row is `(value, pattern, flags, expected violations)`; a row with
    /// violations must name the `PatternConstraintComponent`.
    #[test]
    fn pattern_dialect_is_in_force_end_to_end() {
        let cases: &[(&str, &str, Option<&str>, usize)] = &[
            // `x`: `#` is an ordinary character and only #x9/#xA/#xD/#x20 are
            // removed. `RegexBuilder::ignore_whitespace` would read `#` as a
            // comment and compile `a#b c` to `a`, matching `az`.
            ("a#bc", "a#b c", Some("x"), 0),
            ("az", "a#b c", Some("x"), 1),
            // U+3000 carries the Unicode `White_Space` property and is not one
            // of the four code points `x` names, so it survives as a literal.
            ("a\u{3000}b", "a\u{3000}b", Some("x"), 0),
            ("ab", "a\u{3000}b", Some("x"), 1),
            // `\s` is XSD's four code points, not Unicode `White_Space`:
            // U+00A0 carries the property but is not one of the four.
            ("\u{A0}", r"^\s$", None, 1),
            (" ", r"^\s$", None, 0),
            // `.` excludes BOTH #x0A and #x0D; the `regex` crate excludes only
            // #x0A.
            ("a\rb", "^a.b$", None, 1),
            ("a\nb", "^a.b$", None, 1),
            ("axb", "^a.b$", None, 0),
            // XSD character-class subtraction is not `regex`-crate syntax.
            ("bcd", r"^[a-z-[aeiou]]+$", None, 0),
            ("abc", r"^[a-z-[aeiou]]+$", None, 1),
            // `\p{Is…}` is a Unicode BLOCK: U+1F00 is in the Greek Extended
            // block and in the Greek SCRIPT, but not in Greek and Coptic.
            ("\u{0391}", r"^\p{IsGreekandCoptic}$", None, 0),
            ("\u{1F00}", r"^\p{IsGreekandCoptic}$", None, 1),
        ];
        for (value, regex, flags, expected) in cases {
            let results = pattern_results(value, regex, *flags);
            assert_eq!(
                results.len(),
                *expected,
                "{value:?} / {regex:?} / {flags:?} expected {expected} violation(s)"
            );
            if *expected > 0 {
                assert!(
                    component_iri(&results)[0].contains("Pattern"),
                    "{regex:?} must report the Pattern constraint component"
                );
            }
        }
    }

    /// Run one `sh:pattern`/`sh:flags` pair through the real
    /// `Constraint::Pattern` validation path against a single literal value.
    ///
    /// The value is written into a Turtle string literal, so the five
    /// characters that would break it are escaped; every other scalar (NBSP,
    /// U+3000, the Greek letters the block cases use) is legal raw and stays
    /// verbatim.
    fn pattern_results(value: &str, regex: &str, flags: Option<&str>) -> Vec<ValidationResult> {
        let mut escaped = String::with_capacity(value.len());
        for c in value.chars() {
            match c {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(c),
            }
        }
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:code \"{escaped}\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: regex.to_owned(),
                flags: flags.map(str::to_owned),
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        validate_shape(&store, &ex("a"), &shape)
    }

    /// A `sh:pattern` that does not compile is still a violation on every
    /// value node — SHACL's Core path has no shape-error channel, and the W3C
    /// suite depends on that behaviour. But the compiler's precise
    /// [`XsdRegexError`](purrdf_core::xsd_regex::XsdRegexError) must NOT be
    /// discarded: it is carried into `sh:resultMessage` so a broken SHAPE is
    /// distinguishable from bad DATA. Before this, every result said only
    /// `PatternConstraintComponent` and the reason was lost.
    #[test]
    fn malformed_pattern_violates_but_names_the_construct_in_the_message() {
        // Two value nodes, so both the per-value-node contract and the
        // report message are exercised.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:code \"v1\", \"v2\" ."
        ));
        let cases: &[(&str, Option<&str>, &str)] = &[
            // A backreference is a permanent, by-design limitation.
            (r"(a)\1", None, "backreference"),
            // An invalid flag character.
            ("^[A-Z]+$", Some("z"), "'z'"),
            // An unterminated character class.
            ("[abc", None, "malformed"),
            // An inline-flag group, which the XSD grammar does not define.
            ("(?i)x", None, "malformed"),
            // A Unicode script name, where the dialect admits only a block.
            (r"\p{Greek}", None, "Greek"),
        ];
        for (regex, flags, expected) in cases {
            let shape = prop_shape(
                "S",
                &format!("{EX}code"),
                vec![Constraint::Pattern {
                    regex: (*regex).to_owned(),
                    flags: (*flags).map(str::to_owned),
                    compiled: Arc::new(OnceLock::new()),
                }],
            );
            let results = validate_shape(&store, &ex("a"), &shape);
            assert_eq!(
                results.len(),
                2,
                "{regex:?} / {flags:?} must violate on every value node"
            );
            assert!(
                component_iri(&results)[0].contains("Pattern"),
                "{regex:?} must still report the Pattern constraint component"
            );
            for result in &results {
                let message = result
                    .message
                    .as_deref()
                    .unwrap_or_else(|| panic!("{regex:?} / {flags:?} must carry a message"));
                assert!(
                    message.contains("invalid sh:pattern"),
                    "{regex:?} / {flags:?}: message {message:?} must mark the SHAPE as invalid"
                );
                assert!(
                    message.contains(expected),
                    "{regex:?} / {flags:?}: message {message:?} must name {expected:?}"
                );
            }
        }
    }

    // ── build_regex ────────────────────────────────────────────────────────────

    #[test]
    fn build_regex_q_flag_is_supported_and_literal() {
        // 'q' (XPath literal-match flag) is a valid, supported flag: the
        // pattern is matched verbatim rather than rejected.
        let re = build_regex("a.c", Some("q")).expect("'q' flag should be accepted");
        assert!(
            re.as_regex().is_match("xa.cx"),
            "under 'q', '.' is a literal dot"
        );
        assert!(
            !re.as_regex().is_match("abc"),
            "under 'q', '.' must not act as a wildcard"
        );
    }

    #[test]
    fn build_regex_rejects_genuinely_unknown_flag() {
        // 'z' is not part of the XPath F&O 3.1 §5.6.2 flag alphabet (i s m x q)
        // and must still hard-fail.
        assert!(
            build_regex("foo", Some("z")).is_err(),
            "build_regex should reject unknown flag 'z'"
        );
        // Verify the error message identifies the offending character.
        let err = build_regex("foo", Some("z")).unwrap_err();
        assert!(
            err.to_string().contains('z'),
            "error message should mention the rejected flag character"
        );
    }

    #[test]
    fn build_regex_accepts_supported_flags() {
        // All five supported flags must compile without error.
        assert!(
            build_regex("foo", Some("i")).is_ok(),
            "flag 'i' should be accepted"
        );
        assert!(
            build_regex("foo", Some("s")).is_ok(),
            "flag 's' should be accepted"
        );
        assert!(
            build_regex("foo", Some("m")).is_ok(),
            "flag 'm' should be accepted"
        );
        assert!(
            build_regex("foo", Some("x")).is_ok(),
            "flag 'x' should be accepted"
        );
        assert!(
            build_regex("foo", Some("q")).is_ok(),
            "flag 'q' should be accepted"
        );
        assert!(
            build_regex("foo", Some("ismx")).is_ok(),
            "combined flags should be accepted"
        );
    }

    /// Every example XPath F&O 3.1 §5.6.2 gives for the `x` flag, executed.
    #[test]
    fn xpath_x_flag_matches_the_specifications_examples() {
        let matches = |input: &str, pattern: &str, flags: &str| {
            build_regex(pattern, Some(flags))
                .expect("pattern compiles")
                .as_regex()
                .is_match(input)
        };

        // `fn:matches("helloworld", "hello world", "x")` returns `true()`
        assert!(matches("helloworld", "hello world", "x"));
        // `fn:matches("helloworld", "hello[ ]world", "x")` returns `false()`
        // — whitespace inside a character class expression is NOT removed.
        assert!(!matches("helloworld", "hello[ ]world", "x"));
        // `fn:matches("hello world", "hello\ sworld", "x")` returns `true()`
        // — removal is textual and prior to parsing, so the escape re-binds and
        // `\ s` becomes `\s`.
        assert!(matches("hello world", r"hello\ sworld", "x"));
        // `fn:matches("hello world", "hello world", "x")` returns `false()`
        assert!(!matches("hello world", "hello world", "x"));

        // The valid neighbour that must be unaffected: the same patterns without
        // the flag keep their literal spaces.
        assert!(matches("hello world", "hello world", ""));
        assert!(!matches("helloworld", "hello world", ""));
        assert!(matches("hello world", "hello[ ]world", ""));
    }

    /// The `x` flag removes exactly four code points, not the Unicode
    /// `White_Space` property, and `#` is not a comment.
    #[test]
    fn xpath_x_flag_is_not_rusts_ignore_whitespace() {
        let matches = |input: &str, pattern: &str, flags: &str| {
            build_regex(pattern, Some(flags))
                .expect("pattern compiles")
                .as_regex()
                .is_match(input)
        };

        // The four XPath names, each removed.
        assert!(matches("ab", "a\u{9}b", "x"));
        assert!(matches("ab", "a\u{A}b", "x"));
        assert!(matches("ab", "a\u{D}b", "x"));
        assert!(matches("ab", "a\u{20}b", "x"));

        // U+00A0 NO-BREAK SPACE and U+3000 IDEOGRAPHIC SPACE carry the Unicode
        // `White_Space` property and are NOT named by the flag, so they stay in
        // the pattern as literals to match. `ignore_whitespace` deleted them,
        // which made a pattern stop matching the text it was written for.
        assert!(matches("a\u{A0}b", "a\u{A0}b", "x"));
        assert!(!matches("ab", "a\u{A0}b", "x"));
        assert!(matches("a\u{3000}b", "a\u{3000}b", "x"));
        assert!(!matches("ab", "a\u{3000}b", "x"));

        // XPath regex has no comment syntax: `#` is an ordinary character.
        // `ignore_whitespace` compiled `"a#b c"` to `a`, which matches "az".
        assert!(matches("a#bc", "a#b c", "x"));
        assert!(!matches("az", "a#b c", "x"));

        // Escaped brackets delimit nothing, so the exempt region is not entered
        // and the space after `\[` is still removed.
        assert!(matches("a[bc", r"a\[ bc", "x"));
        // ... while a real class keeps its space, even a `\]`-bearing one.
        assert!(matches("a]b", r"a[\] ]b", "x"));
        assert!(matches("a b", r"a[\] ]b", "x"));
        assert!(!matches("ab", r"a[\] ]b", "x"));
    }

    /// `collapse_trim` strips a scalar from an edge if and only if the
    /// `whiteSpace` = `collapse` facet names it — stated as a total function
    /// over every Unicode scalar, so the class cannot drift toward either the
    /// Unicode `White_Space` property or the ASCII one.
    #[test]
    fn collapse_trim_strips_exactly_the_four_code_points_the_facet_names() {
        for cp in 0..=0x0010_FFFF_u32 {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            let named = matches!(c, '\u{20}' | '\u{9}' | '\u{D}' | '\u{A}');
            let padded = format!("{c}x{c}");
            assert_eq!(
                collapse_trim(&padded) == "x",
                named,
                "{c:?} ({cp:#06X}) must be stripped iff whiteSpace=collapse names it"
            );
        }
        assert_eq!(collapse_trim(" \t\r\n42\n\r\t "), "42");
        assert_eq!(collapse_trim("4 2"), "4 2");
        assert_eq!(collapse_trim(""), "");
        assert_eq!(collapse_trim("   "), "");
    }

    /// XSD lexical spaces are trimmed with the four code points
    /// `whiteSpace` = `collapse` names, not with the Unicode property.
    #[test]
    fn xsd_lexical_forms_collapse_over_xml_s_only() {
        const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
        const DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

        // The valid neighbours, unchanged: `collapse` still strips every one of
        // `#x20`, `#x9`, `#xD` and `#xA`, in any combination and at either end.
        for padded in ["42", " 42 ", "\t42\n", "\r\n 42 \t", "\n42"] {
            assert!(
                xsd_lexical_valid(INTEGER, padded),
                "{padded:?} is xsd:integer padded only with XML S"
            );
        }
        assert!(xsd_lexical_valid(DECIMAL, " -4.25\t"));
        assert!(xsd_lexical_valid(DOUBLE, "\r\n1.5e3 "));
        assert!(xsd_lexical_valid(BOOLEAN, "\ttrue\n"));

        // The over-acceptance removed: U+00A0 is not `#x20`, so `collapse` never
        // touches it and the literal is not in the datatype's lexical space.
        // `str::trim` stripped it and called the value conforming.
        for nbsp in ["\u{A0}42", "42\u{A0}", "\u{A0}42\u{A0}"] {
            assert!(
                !xsd_lexical_valid(INTEGER, nbsp),
                "{nbsp:?} is not in the xsd:integer lexical space"
            );
        }
        assert!(!xsd_lexical_valid(DECIMAL, "\u{A0}4.25"));
        assert!(!xsd_lexical_valid(DOUBLE, "1.5e3\u{2003}"));
        assert!(!xsd_lexical_valid(BOOLEAN, "true\u{A0}"));
        // U+000B VERTICAL TAB and U+000C FORM FEED are the other two traps: the
        // Unicode property admits both, `collapse` names neither.
        assert!(!xsd_lexical_valid(INTEGER, "\u{B}42"));
        assert!(!xsd_lexical_valid(INTEGER, "\u{C}42"));

        // Internal whitespace is refused either way, which is why trimming is
        // the whole of `collapse` for these one-token lexical spaces.
        assert!(!xsd_lexical_valid(INTEGER, "4 2"));
        assert!(!xsd_lexical_valid(INTEGER, "4\t2"));

        // The property the replaced code actually asked, pinned so the
        // over-acceptance above is executed rather than asserted: `str::trim`
        // stripped every one of these and handed the bare digits on.
        for unicode_ws in ['\u{A0}', '\u{2003}', '\u{B}', '\u{C}', '\u{3000}'] {
            assert!(unicode_ws.is_whitespace(), "{unicode_ws:?}");
            assert_eq!(format!("{unicode_ws}42").trim(), "42");
        }
    }

    /// The derived-integer bound check trims with the same class the lexical
    /// gate does, so the two cannot disagree about one literal.
    #[test]
    fn derived_integer_bounds_trim_the_same_class_as_the_lexical_gate() {
        const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const NON_NEGATIVE: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
        const POSITIVE: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";

        // Valid neighbours: XML `S` padding still reaches the bound check.
        assert!(derived_integer_matches(INTEGER, NON_NEGATIVE, " 7\t"));
        assert!(derived_integer_matches(INTEGER, POSITIVE, "\n7\r"));
        assert!(!derived_integer_matches(INTEGER, POSITIVE, " -7 "));
        // U+00A0 padding is refused at the lexical gate, so it never reaches the
        // bound check at all.
        assert!(!derived_integer_matches(INTEGER, NON_NEGATIVE, "\u{A0}7"));
    }

    // ── minLength ──────────────────────────────────────────────────────────────

    #[test]
    fn min_length_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:name \"Alice\" ."));
        let shape = prop_shape("S", &format!("{EX}name"), vec![Constraint::MinLength(3)]);
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn min_length_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:name \"Al\" ."));
        let shape = prop_shape("S", &format!("{EX}name"), vec![Constraint::MinLength(3)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinLength"));
    }

    // ── uniqueLang ─────────────────────────────────────────────────────────────

    #[test]
    fn unique_lang_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:label \"Hello\"@en, \"Bonjour\"@fr ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}label"),
            vec![Constraint::UniqueLang(true)],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn unique_lang_fail() {
        // Load two English-tagged literals via N-Triples (Turtle deduplicates in the store).
        let nt = format!("<{EX}a> <{EX}label> \"Hello\"@en .\n<{EX}a> <{EX}label> \"Hi\"@en .\n");
        let store = crate::text_ingest::parse_ntriples_to_dataset(&nt).expect("N-Triples parse");
        let shape = prop_shape(
            "S",
            &format!("{EX}label"),
            vec![Constraint::UniqueLang(true)],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(!results.is_empty());
        assert!(component_iri(&results)[0].contains("UniqueLang"));
    }

    // ── minInclusive ───────────────────────────────────────────────────────────

    #[test]
    fn min_inclusive_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"18\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinInclusive(xsd_lit("18", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn min_inclusive_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"17\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinInclusive(xsd_lit("18", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinInclusive"));
    }

    /// **A range-facet bound that is not an XSD numeric literal is an ordinary
    /// shapes graph, and must never become a refusal.**
    ///
    /// The bound's numeric parse moved to stage 0, which is exactly the shape of
    /// change that turns a soft `None` into a hard error: the parse now happens
    /// during PREPARATION, where returning `Err` looks like proper strictness and
    /// costs nothing to write. It would reject shapes graphs that validate today.
    ///
    /// So both neighbours are executed here, not just the one that looks invalid:
    ///
    /// 1. an `xsd:dateTime` bound is not numeric, and must still fall through to
    ///    the XSD temporal value-space comparison — which really compares, so this
    ///    half asserts a CONFORMING case as well as a violating one. A refusal
    ///    would take out a whole datatype's worth of working shapes.
    /// 2. a lexically invalid numeric bound is not numeric either, and must still
    ///    produce the outcome it always produced: incomparable, therefore a
    ///    violation — a validation result, not an error.
    #[test]
    fn a_non_numeric_range_facet_bound_is_never_refused() {
        // ── Half one: a temporal bound still reaches the temporal comparison ──
        let temporal = load_store(&format!(
            "@prefix ex: <{EX}> . \
             ex:after  ex:at \"2020-06-01T00:00:00Z\"^^<{XSD}dateTime> . \
             ex:before ex:at \"2019-06-01T00:00:00Z\"^^<{XSD}dateTime> ."
        ));
        let temporal_shape = prop_shape(
            "S",
            &format!("{EX}at"),
            vec![Constraint::MinInclusive(xsd_lit(
                "2020-01-01T00:00:00Z",
                "dateTime",
            ))],
        );
        assert!(
            validate_shape(&temporal, &ex("after"), &temporal_shape).is_empty(),
            "a dateTime at or after a dateTime bound must CONFORM: the non-numeric bound has to \
             reach the temporal comparison, not be refused and not be treated as incomparable"
        );
        let later = validate_shape(&temporal, &ex("before"), &temporal_shape);
        assert_eq!(
            later.len(),
            1,
            "a dateTime before the bound must violate, which is what says the temporal \
             comparison really ran rather than conforming vacuously"
        );
        assert!(component_iri(&later)[0].contains("MinInclusive"));

        // ── Half two: an unparseable numeric bound still violates, not errors ──
        let numeric = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"18\"^^<{XSD}integer> ."
        ));
        let broken_bound = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinInclusive(xsd_lit("not-a-number", "integer"))],
        );
        let incomparable = validate_shape(&numeric, &ex("a"), &broken_bound);
        assert_eq!(
            incomparable.len(),
            1,
            "a bound whose lexical form does not parse leaves every value node incomparable, \
             which SHACL reports as a violation — preparing the shape must not fail instead"
        );
        assert!(component_iri(&incomparable)[0].contains("MinInclusive"));

        // ...and the neighbouring VALID bound over the very same data still
        // conforms, which is the half an over-refusal would silently take with it.
        let good_bound = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinInclusive(xsd_lit("18", "integer"))],
        );
        assert!(
            validate_shape(&numeric, &ex("a"), &good_bound).is_empty(),
            "the valid neighbour of the refused case must still validate"
        );
    }

    // ── maxInclusive ───────────────────────────────────────────────────────────

    #[test]
    fn max_inclusive_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"100\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxInclusive(xsd_lit("100", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn max_inclusive_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"101\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxInclusive(xsd_lit("100", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxInclusive"));
    }

    // ── minExclusive ─────────────────────────────────────────────────────────────

    #[test]
    fn min_exclusive_pass() {
        // The bound is exclusive: 19 > 18 passes.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"19\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinExclusive(xsd_lit("18", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn min_exclusive_fail_on_equal() {
        // Equal to the bound must FAIL under sh:minExclusive (unlike minInclusive).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"18\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinExclusive(xsd_lit("18", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinExclusive"));
    }

    // ── maxExclusive ─────────────────────────────────────────────────────────────

    #[test]
    fn max_exclusive_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"99\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxExclusive(xsd_lit("100", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn max_exclusive_fail_on_equal() {
        // Equal to the bound must FAIL under sh:maxExclusive.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"100\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxExclusive(xsd_lit("100", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxExclusive"));
    }

    // ── and ────────────────────────────────────────────────────────────────────

    #[test]
    fn and_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a rdf:type ex:Foo ."
        ));
        // sh:and ([ sh:nodeKind sh:IRI ] [ sh:class ex:Foo ]) on focus node directly.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with(
            "M2",
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let shape = shape_with("S", vec![Constraint::And(vec![member1, member2])]);
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn and_fail_second_member() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a rdf:type ex:Bar ."
        ));
        // ex:a is IRI (passes M1) but type is ex:Bar not ex:Foo (fails M2).
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with(
            "M2",
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let shape = shape_with("S", vec![Constraint::And(vec![member1, member2])]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("And"));
    }

    // ── or ─────────────────────────────────────────────────────────────────────

    #[test]
    fn or_pass_first_member() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // ex:b is an IRI, passes M1.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Literal)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Or(vec![member1, member2])],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn or_fail_no_member() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // Both members require Literal; ex:b is IRI → fails both.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Literal)]);
        let member2 = shape_with(
            "M2",
            vec![Constraint::MinLength(999)], // impossible length
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Or(vec![member1, member2])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Or"));
    }

    // ── xone ───────────────────────────────────────────────────────────────────

    #[test]
    fn xone_pass_exactly_one() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // ex:b is IRI: M1 (IRI) passes, M2 (Literal) fails → exactly 1.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Literal)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Xone(vec![member1, member2])],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn xone_fail_zero() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\" ."));
        // Both require IRI; literal fails both → 0 conforming → violation.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Xone(vec![member1, member2])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Xone"));
    }

    #[test]
    fn xone_fail_two() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // Both members allow IRI → 2 conforming → violation.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Xone(vec![member1, member2])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Xone"));
    }

    // ── node ───────────────────────────────────────────────────────────────────

    #[test]
    fn node_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // sh:node targets ex:b; inner shape requires IRI.
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Node(Box::new(inner))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn node_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"notAnIRI\" ."));
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Node(Box::new(inner))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("NodeConstraintComponent"));
    }

    // ── inverse path property shape ────────────────────────────────────────────

    #[test]
    fn inverse_path_property_shape() {
        use crate::shapes::Path;
        // ex:child ex:parent ex:parent_node .
        // Shape on ex:parent_node checks inverse(ex:parent) has minCount 1.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:child ex:parent ex:parent_node ."
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(
                    format!("{EX}parent"),
                )))),
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        };
        // ex:parent_node has 1 inverse-parent (ex:child) → passes minCount(1).
        let results = validate_shape(&store, &ex("parent_node"), &shape);
        assert!(results.is_empty(), "expected pass, got: {results:?}");
    }

    /// **An inverse over a COMPOSITE path still selects the same value nodes once
    /// the inversion is performed at lowering time instead of per focus node.**
    ///
    /// `^(p/q)` is the one path form whose lowering is a structural REWRITE rather
    /// than a slot resolution, and the rewrite is direction-sensitive: `^(p/q)` is
    /// `^q/^p`, not `^p/^q`, so getting it backwards produces a perfectly
    /// well-formed path that selects the wrong nodes. The `Path`-driven evaluator's
    /// own tests cover `invert` itself; this one drives the LOWERED path, through
    /// the validator, which is the route every prepared shape takes and the only
    /// one where a mis-lowering could hide.
    ///
    /// Stated as a conforming case and a violating one over the same data, because
    /// an inversion that selected nothing at all would satisfy a pass-only
    /// assertion on `sh:maxCount` and a fail-only assertion on `sh:minCount`.
    #[test]
    fn inverse_of_a_composite_path_is_lowered_in_the_right_direction() {
        use crate::shapes::Path;
        // ex:a ex:p ex:b . ex:b ex:q ex:d .  so  ex:a  p/q  ex:d,
        // and therefore  ex:d  ^(p/q)  ex:a  — while ex:a reaches nothing.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p ex:b . ex:b ex:q ex:d ."
        ));
        let composite = || {
            Path::Inverse(Box::new(Path::Sequence(vec![
                Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                Path::Predicate(NamedNode::new_unchecked(format!("{EX}q"))),
            ])))
        };
        let shape_with = |constraints: Vec<Constraint>| Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("S-property"),
                path: composite(),
                constraints,
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        };

        // The tail of the sequence reaches its head: one value node, ex:a.
        let at_tail = shape_with(vec![Constraint::MinCount(1), Constraint::MaxCount(1)]);
        assert!(
            validate_shape(&store, &ex("d"), &at_tail).is_empty(),
            "^(p/q) from the sequence's tail must yield exactly its head"
        );
        assert_eq!(
            validate_shape(
                &store,
                &ex("d"),
                &shape_with(vec![Constraint::HasValue(ex("a"))])
            )
            .len(),
            0,
            "and that one value node must be ex:a itself, not merely some node"
        );

        // The head reaches nothing: an inversion applied in the WRONG direction
        // would light up here instead.
        assert_eq!(
            validate_shape(&store, &ex("a"), &at_tail).len(),
            1,
            "^(p/q) from the sequence's HEAD must yield nothing, so sh:minCount 1 violates"
        );
    }

    #[test]
    fn inverse_path_property_shape_fail() {
        use crate::shapes::Path;
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:unrelated ex:something ex:other ."
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(
                    format!("{EX}parent"),
                )))),
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        };
        // ex:orphan has no inverse-parent triples → fails minCount(1).
        let results = validate_shape(&store, &ex("orphan"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinCount"));
    }

    // ── xsd lexical validators (Gap D fix) ────────────────────────────────────

    #[test]
    fn xsd_integer_accepts_large_value() {
        // A valid xsd:integer beyond i64::MAX must PASS (no overflow rejection).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}integer"));
        let value = Term::Literal(Literal::new_typed_literal(
            "99999999999999999999999",
            dt_iri.clone(),
        ));
        assert!(
            check_datatype(&value, &dt_iri),
            "large integer should conform"
        );
    }

    #[test]
    fn xsd_integer_rejects_decimal_point() {
        // "3.5"^^xsd:integer is lexically invalid.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}integer"));
        let value = Term::Literal(Literal::new_typed_literal("3.5", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "decimal point in integer should violate"
        );
    }

    #[test]
    fn xsd_decimal_rejects_scientific_notation() {
        // "1e3"^^xsd:decimal is NOT a valid xsd:decimal lexical form.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}decimal"));
        let value = Term::Literal(Literal::new_typed_literal("1e3", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "scientific notation should violate xsd:decimal"
        );
    }

    #[test]
    fn xsd_decimal_accepts_plain() {
        // "3.14"^^xsd:decimal is valid.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}decimal"));
        let value = Term::Literal(Literal::new_typed_literal("3.14", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "plain decimal should conform"
        );
    }

    #[test]
    fn xsd_double_accepts_scientific() {
        // "1e3"^^xsd:double is valid (scientific notation is allowed for double).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}double"));
        let value = Term::Literal(Literal::new_typed_literal("1e3", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "scientific notation should conform for xsd:double"
        );
    }

    #[test]
    fn xsd_double_accepts_inf() {
        // "INF"^^xsd:double is a valid XSD special value.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}double"));
        let value = Term::Literal(Literal::new_typed_literal("INF", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "INF should conform for xsd:double"
        );
    }

    #[test]
    fn xsd_double_rejects_plus_inf() {
        // "+INF" is NOT in the xsd:double/float lexical space (only INF, -INF, NaN).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}double"));
        let value = Term::Literal(Literal::new_typed_literal("+INF", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "+INF must not conform for xsd:double"
        );
    }

    #[test]
    fn xsd_float_accepts_inf() {
        // "INF"^^xsd:float is a valid XSD special value.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}float"));
        let value = Term::Literal(Literal::new_typed_literal("INF", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "INF should conform for xsd:float"
        );
    }

    #[test]
    fn xsd_float_rejects_plus_inf() {
        // "+INF" is NOT in the xsd:double/float lexical space (only INF, -INF, NaN).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}float"));
        let value = Term::Literal(Literal::new_typed_literal("+INF", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "+INF must not conform for xsd:float"
        );
    }

    #[test]
    fn xsd_1_0_double_lexical_space_is_pinned() {
        // Characterizes the XSD-1.0 double/float accept-set, exactly: the
        // three specials INF/-INF/NaN (not the XSD 1.1 "+INF"), a decimal
        // mantissa with an optional [eE][+-]?digits exponent, and the
        // SHACL-legacy whitespace leniency (the arm trims before
        // validating). This is not a differential test against an external
        // oracle — it directly pins the accept-set now owned by
        // `purrdf_xsd::parse_double_xsd10`, which SHACL's `xsd_lexical_valid`
        // relies on as defense-in-depth for float/double literals.
        let ok = |x: &str| purrdf_xsd::parse_double_xsd10(x.trim()).is_ok();
        for good in [
            "INF", "-INF", "NaN", "1", "1.", ".5", "+1.5", "1e10", "1E+5", "1e400", " 1.5 ",
        ] {
            assert!(ok(good), "{good:?} is in the XSD-1.0 double lexical space");
        }
        for bad in ["+INF", "inf", "Infinity", "1e", "1.5.5", "", "abc"] {
            assert!(
                !ok(bad),
                "{bad:?} is NOT in the XSD-1.0 double lexical space"
            );
        }
    }

    #[test]
    fn xsd_float_accepts_scientific() {
        // "1e3"^^xsd:float is valid — same lexical space as double.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}float"));
        let value = Term::Literal(Literal::new_typed_literal("1e3", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "scientific notation should conform for xsd:float"
        );
    }

    // ── deactivated shape ──────────────────────────────────────────────────────

    #[test]
    fn deactivated_shape_produces_no_results() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\" ."));
        // Would fail NodeKind(Iri) if active.
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![Constraint::NodeKind(NodeKindValue::Iri)],
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: true,
            box_roles: vec![],
            rules: vec![],
        };
        // Focus node is a literal — would fail, but shape is deactivated.
        let literal_focus = Term::Literal(Literal::new_simple_literal("anything"));
        assert!(validate_shape(&store, &literal_focus, &shape).is_empty());
    }

    // ── maxLength ─────────────────────────────────────────────────────────────

    #[test]
    fn max_length_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"abc\" ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxLength(5)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "\"abc\" (len 3) ≤ 5 must pass");
    }

    #[test]
    fn max_length_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"abcdef\" ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxLength(5)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxLength"));
    }

    // ── languageIn ────────────────────────────────────────────────────────────

    #[test]
    fn language_in_pass_prefix_match() {
        // "hello"@en-US matches the entry "en" by basic-filtering prefix match.
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\"@en-US ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::LanguageIn(vec!["en".into(), "fr".into()])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "en-US must match entry \"en\"");
    }

    #[test]
    fn language_in_fail_unlisted_and_untagged() {
        // "guten"@de is not in the list → violation; "plain" has no tag → violation.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p \"guten\"@de , \"plain\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::LanguageIn(vec!["en".into(), "fr".into()])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(
            results.len(),
            2,
            "both the de literal and the untagged literal violate"
        );
        assert!(component_iri(&results)[0].contains("LanguageIn"));
    }

    // ── not ───────────────────────────────────────────────────────────────────

    #[test]
    fn not_pass_when_inner_violated() {
        // Inner shape requires NodeKind(Iri); the value is a literal, so it does
        // NOT conform → sh:not is satisfied.
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"lit\" ."));
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Not(Box::new(inner))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(
            results.is_empty(),
            "literal does not conform to inner ⇒ not() passes"
        );
    }

    #[test]
    fn not_fail_when_inner_conforms() {
        // Inner shape requires NodeKind(Iri); the value IS an IRI, so it conforms
        // → sh:not is violated.
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Not(Box::new(inner))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("NotConstraintComponent"));
    }

    // ── closed ────────────────────────────────────────────────────────────────

    fn closed_shape(ignored: Vec<NamedNode>, path_iris: &[&str]) -> Shape {
        use crate::shapes::Path;
        let property_shapes = path_iris
            .iter()
            .enumerate()
            .map(|(index, p)| PropertyShape {
                id: ex(&format!("Property-{index}")),
                path: Path::Predicate(NamedNode::new_unchecked(*p)),
                constraints: vec![],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
            })
            .collect();
        Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![Constraint::Closed { ignored }],
            property_shapes,
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        }
    }

    #[test]
    fn closed_pass_only_declared_predicates() {
        // ex:a uses only ex:name (declared) and rdf:type (listed in
        // sh:ignoredProperties — per spec §4.8.1 / W3C closed-001, rdf:type is
        // NOT implicitly permitted; closed shapes must ignore it explicitly).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a a ex:Person ; ex:name \"Al\" ."
        ));
        let shape = closed_shape(
            vec![NamedNode::new_unchecked(rdf::TYPE)],
            &[&format!("{EX}name")],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(
            results.is_empty(),
            "declared + explicitly-ignored predicates ⇒ pass"
        );
    }

    #[test]
    fn closed_fail_rdf_type_not_implicitly_ignored() {
        // Without rdf:type in sh:ignoredProperties, a typed focus node
        // violates the closed shape ON rdf:type (W3C closed-001).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a a ex:Person ; ex:name \"Al\" ."
        ));
        let shape = closed_shape(vec![], &[&format!("{EX}name")]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "rdf:type must be reported");
        assert_eq!(
            results[0].result_path.as_ref().map(ToString::to_string),
            Some(format!("<{}>", rdf::TYPE))
        );
    }

    #[test]
    fn closed_fail_extra_predicate() {
        // ex:a also uses ex:age, which is neither declared nor ignored.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:name \"Al\" ; ex:age 30 ."
        ));
        let shape = closed_shape(vec![], &[&format!("{EX}name")]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "ex:age is an undeclared predicate");
        assert!(component_iri(&results)[0].contains("ClosedConstraintComponent"));
        assert_eq!(
            results[0].result_path.as_ref().map(ToString::to_string),
            Some(format!("<{EX}age>"))
        );
    }

    #[test]
    fn closed_violation_carries_predicate_box_roles() {
        // The offending (undeclared) predicate ex:age declares a graph-box role;
        // the closed-world result must carry it as PATH attribution — closed
        // violations must not drop predicate roles.
        let vocab = meta_vocab();
        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix meta: <https://example.org/meta/> .\n\
             ex:age meta:graphBoxRole meta:boxRBox .\n\
             ex:a ex:name \"Al\" ; ex:age 30 .\n"
        ));
        let shape = closed_shape(vec![], &[&format!("{EX}name")]);
        let results = validate_shape_with_roles(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "ex:age is an undeclared predicate");
        assert_eq!(
            role_iris(&results[0].path_box_roles),
            [vocab.box_rbox.as_str()],
            "closed-world violation must carry the offending predicate's box roles"
        );
        assert!(
            role_iris(&results[0].result_box_roles).contains(&vocab.box_rbox.as_str()),
            "merged result roles must include the predicate's path role"
        );
    }

    #[test]
    fn closed_pass_ignored_predicate() {
        // ex:age is undeclared but listed in sh:ignoredProperties ⇒ allowed.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:name \"Al\" ; ex:age 30 ."
        ));
        let shape = closed_shape(
            vec![NamedNode::new_unchecked(format!("{EX}age"))],
            &[&format!("{EX}name")],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "ignored predicate ex:age ⇒ pass");
    }

    // ── Property-pair constraints (§4.3) ───────────────────────────────────────

    fn pair_pred(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{EX}{local}"))
    }

    #[test]
    fn equals_pass_when_value_sets_match() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p \"x\" , \"y\" ; ex:q \"y\" , \"x\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Equals(pair_pred("q"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn equals_fail_reports_both_directions() {
        // ex:p has "x" (missing from ex:q); ex:q has "z" (missing from ex:p).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p \"x\" ; ex:q \"z\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Equals(pair_pred("q"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 2, "one result per asymmetric value");
        assert!(
            component_iri(&results)
                .iter()
                .all(|c| c.contains("EqualsConstraintComponent"))
        );
        let values: Vec<String> = results
            .iter()
            .map(|r| r.value.as_ref().unwrap().to_string())
            .collect();
        assert!(values.contains(&"\"x\"".to_owned()));
        assert!(values.contains(&"\"z\"".to_owned()));
    }

    #[test]
    fn disjoint_pass_and_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p \"x\" , \"shared\" ; ex:q \"shared\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Disjoint(pair_pred("q"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "only the shared value violates");
        assert!(component_iri(&results)[0].contains("DisjointConstraintComponent"));
        assert_eq!(
            results[0].value.as_ref().unwrap().to_string(),
            "\"shared\"".to_owned()
        );

        let store_ok = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p \"x\" ; ex:q \"y\" ."
        ));
        assert!(validate_shape(&store_ok, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn less_than_numeric_literals() {
        // start=10 is NOT less than end=5 → violation carrying the value node.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:start \"10\"^^<{XSD}integer> ; ex:end \"5\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}start"),
            vec![Constraint::LessThan(pair_pred("end"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("LessThanConstraintComponent"));
        assert_eq!(
            results[0].value.as_ref().unwrap().to_string(),
            format!("\"10\"^^<{XSD}integer>")
        );

        // start=3 < end=5 → conforms.
        let store_ok = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:start \"3\"^^<{XSD}integer> ; ex:end \"5\"^^<{XSD}integer> ."
        ));
        assert!(validate_shape(&store_ok, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn less_than_equal_values_violate_but_lte_passes() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:start \"5\"^^<{XSD}integer> ; ex:end \"5\"^^<{XSD}integer> ."
        ));
        let lt_shape = prop_shape(
            "S",
            &format!("{EX}start"),
            vec![Constraint::LessThan(pair_pred("end"))],
        );
        let results = validate_shape(&store, &ex("a"), &lt_shape);
        assert_eq!(results.len(), 1, "5 < 5 is false → lessThan violates");

        let lte_shape = prop_shape(
            "S",
            &format!("{EX}start"),
            vec![Constraint::LessThanOrEquals(pair_pred("end"))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &lte_shape).is_empty(),
            "5 <= 5 → lessThanOrEquals passes"
        );
    }

    #[test]
    fn less_than_string_literals_compare_lexically() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:start \"apple\" ; ex:end \"banana\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}start"),
            vec![Constraint::LessThan(pair_pred("end"))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "\"apple\" < \"banana\" lexically"
        );
    }

    #[test]
    fn less_than_incomparable_pair_violates() {
        // An IRI value cannot be compared to an integer → violation per spec.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:start ex:thing ; ex:end \"5\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}start"),
            vec![Constraint::LessThan(pair_pred("end"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "incomparable pair must violate");
    }

    #[test]
    fn compare_terms_covers_the_value_lattice() {
        use std::cmp::Ordering;
        // The production comparison reads BORROWED literal views; over owned terms
        // it is exactly this composition, which is what keeps the lattice below a
        // statement about the code the pair constraints actually run.
        let compare_terms = |a: &Term, b: &Term| {
            compare_literal_views(literal_view_of_term(a), literal_view_of_term(b))
        };
        // Mixed numeric datatypes compare by value.
        assert_eq!(
            compare_terms(&xsd_lit("2", "integer"), &xsd_lit("2.5", "decimal")),
            Some(Ordering::Less)
        );
        // Booleans: false < true.
        assert_eq!(
            compare_terms(&xsd_lit("false", "boolean"), &xsd_lit("true", "boolean")),
            Some(Ordering::Less)
        );
        // Same-datatype dateTime compares lexically (ISO 8601).
        assert_eq!(
            compare_terms(
                &xsd_lit("2024-01-01T00:00:00", "dateTime"),
                &xsd_lit("2025-01-01T00:00:00", "dateTime")
            ),
            Some(Ordering::Less)
        );
        // Language-tagged literals are incomparable under SPARQL `<`.
        let lang = Term::Literal(Literal::new_language_tagged_literal_unchecked("a", "en"));
        assert_eq!(compare_terms(&lang, &lang), None);
        // IRIs are incomparable.
        assert_eq!(compare_terms(&ex("x"), &ex("y")), None);
    }

    // ── Qualified value shapes (§4.5.4–4.5.5) ──────────────────────────────────

    fn qualified_constraint(
        class_local: &str,
        siblings: Vec<Shape>,
        min_count: Option<u64>,
        max_count: Option<u64>,
        disjoint: bool,
    ) -> Constraint {
        Constraint::QualifiedValueShape {
            shape: Box::new(shape_with(
                "Q",
                vec![Constraint::Class(NamedNode::new_unchecked(format!(
                    "{EX}{class_local}"
                )))],
            )),
            siblings,
            min_count,
            max_count,
            disjoint,
        }
    }

    #[test]
    fn qualified_min_count_pass_and_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:item ex:i1 , ex:i2 . ex:i1 a ex:Good ."
        ));
        // One value node (ex:i1) conforms to [sh:class ex:Good].
        let pass = prop_shape(
            "S",
            &format!("{EX}item"),
            vec![qualified_constraint("Good", vec![], Some(1), None, false)],
        );
        assert!(validate_shape(&store, &ex("a"), &pass).is_empty());

        let fail = prop_shape(
            "S",
            &format!("{EX}item"),
            vec![qualified_constraint("Good", vec![], Some(2), None, false)],
        );
        let results = validate_shape(&store, &ex("a"), &fail);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("QualifiedMinCountConstraintComponent"));
        assert!(
            results[0].value.is_none(),
            "count violations carry no value"
        );
    }

    #[test]
    fn qualified_max_count_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:item ex:i1 , ex:i2 . ex:i1 a ex:Good . ex:i2 a ex:Good ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}item"),
            vec![qualified_constraint("Good", vec![], None, Some(1), false)],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("QualifiedMaxCountConstraintComponent"));
    }

    #[test]
    fn qualified_disjoint_excludes_sibling_conforming_values() {
        // The thumb is typed BOTH ex:Thumb and ex:Finger. Without disjointness
        // the Thumb-qualified count is 1; with a Finger sibling and
        // disjoint=true the thumb is excluded → count 0 → violation.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:hand ex:digit ex:thumb . ex:thumb a ex:Thumb , ex:Finger ."
        ));
        let finger_sibling = shape_with(
            "FingerQ",
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Finger"
            )))],
        );

        let without_disjoint = prop_shape(
            "S",
            &format!("{EX}digit"),
            vec![qualified_constraint("Thumb", vec![], Some(1), None, false)],
        );
        assert!(
            validate_shape(&store, &ex("hand"), &without_disjoint).is_empty(),
            "without disjointness the thumb counts"
        );

        let with_disjoint = prop_shape(
            "S",
            &format!("{EX}digit"),
            vec![qualified_constraint(
                "Thumb",
                vec![finger_sibling],
                Some(1),
                None,
                true,
            )],
        );
        let results = validate_shape(&store, &ex("hand"), &with_disjoint);
        assert_eq!(
            results.len(),
            1,
            "sibling-conforming thumb is excluded before counting"
        );
        assert!(component_iri(&results)[0].contains("QualifiedMinCountConstraintComponent"));
    }

    // ── Shape metadata: deactivated property shape, severity, message ──────────

    #[test]
    fn deactivated_property_shape_produces_no_results() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a a ex:Thing ."));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                deactivated: true,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        };
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "a deactivated property shape validates nothing"
        );
    }

    #[test]
    fn property_shape_severity_and_message_propagate() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a a ex:Thing ."));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                id: ex("Property"),
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![Constraint::MinCount(1)],
                property_shapes: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Info,
                message: Some("p is recommended".to_owned()),
                deactivated: false,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
            rules: vec![],
        };
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].severity,
            Severity::Info,
            "the property shape's severity overrides the parent's"
        );
        assert_eq!(results[0].message.as_deref(), Some("p is recommended"));
    }
}
