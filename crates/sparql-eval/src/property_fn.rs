// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Host-injected **property functions** — relations invoked from predicate position.
//!
//! A predicate IRI under a caller-configured
//! [`ParserOptions::property_fn_namespaces`](purrdf_sparql_algebra::ParserOptions)
//! (prefix match), or one that exactly matches an entry of
//! [`ParserOptions::property_fn_iris`](purrdf_sparql_algebra::ParserOptions) — the
//! set [`NativeSparqlEngine`](crate::NativeSparqlEngine) derives one-to-one from
//! this registry's keys, so a registered relation is reachable without also
//! reclassifying every other IRI under its namespace — is lowered by the parser to
//! [`purrdf_sparql_algebra::GraphPattern::PropertyFunction`]
//! instead of an ordinary triple pattern, and resolved at evaluation time against a
//! caller-injected [`PropertyFunctionRegistry`]. Where [`crate::user_fn`] injects
//! *functions* — one value per call, in an expression position — this module injects
//! *relations*: a call is a row source in a graph-pattern position, and it may emit
//! zero, one, or many rows per invocation.
//!
//! # Why a relation is not a function with extra steps
//!
//! Three properties follow from being a row source rather than a value, and each one
//! shows up as a distinct piece of this module's contract:
//!
//! * **Access patterns.** A relation is rarely computable in every direction. A
//!   `split(?whole, ?part)` relation can enumerate parts from a whole but not wholes
//!   from a part. So a relation *declares* the argument-binding patterns it can serve
//!   ([`PropertyFunction::modes`]), and an invocation is admitted only when one of
//!   them is general enough to cover it ([`PropertyFunction::admits`]).
//! * **Cardinality.** A call can be an unbounded generator, so the planner needs a
//!   declared upper bound per access pattern ([`PropertyFunction::rows_per_invocation`])
//!   to order calls and to admit them against a ceiling.
//! * **Emission order.** A bag of rows has an order, and SPARQL results are
//!   reproducible only if that order is. So the order a relation emits in is part of
//!   its contract, and the engine preserves it.
//!
//! # The trust boundary
//!
//! A relation is arbitrary host Rust. Every invariant the evaluator needs from it is
//! therefore either **checked before host code runs** (arity, in
//! [`open_contained`]), **contained when host code misbehaves** (a panic, in
//! [`open_contained`]/[`next_contained`] for `open`/`next`, and in
//! [`declaration_contained`] for `arity`/`modes`/`volatility`/`rows_per_invocation` —
//! every one of the four is host code exactly as `open`/`next` is, so nothing reads one
//! directly), or **applied to what host code returns** (equality on bound positions, and
//! the row-width check, in the dispatch). A relation that ignores its own declarations
//! cannot make the engine unsound; it can only make it slow, or make its own call fail.
//!
//! The row ceiling [`PropertyFunction::open`] receives is the one place that asks a
//! relation for cooperation rather than checking it, because "I stopped early" and "I
//! am exhausted" are the same empty cursor and no engine-side measure can tell them
//! apart. The obligation is kept as small as the seam allows: the ceiling counts rows
//! the relation emits that agree with the bound values it was handed — everything the
//! engine filters on that the relation cannot see makes the engine withhold the ceiling
//! instead (see [`PropertyFunction`]'s ceiling contract).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{DatasetView, GraphMatch, Iri, TermValue};

use crate::DetHashMap;
use crate::error::EvalError;
use crate::user_fn::Volatility;

// ---------------------------------------------------------------------------
// Positional shape
// ---------------------------------------------------------------------------

/// A relation's declared positional arity: how many arguments it takes on the
/// subject side of the predicate and how many on the object side.
///
/// Checked against the call site **before any host code runs** — the same fail-fast
/// doctrine [`crate::user_fn::Arity`] applies to a native function, and for the same
/// reason: a relation must never be handed a short or long argument vector.
///
/// # The flattening order
///
/// Everything downstream of arity — [`PropertyFunction::modes`], [`PfArgs`], [`PfRow`]
/// — indexes **flattened** positions: the subject-side arguments first, in written
/// order, then the object-side arguments, in written order. Flattened position `p` is
/// subject argument `p` when `p < subject`, and object argument `p - subject`
/// otherwise. One numbering, used by every surface, so a mode and a row cannot
/// disagree about which position they are talking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PfArity {
    /// The number of subject-side arguments.
    pub subject: usize,
    /// The number of object-side arguments.
    pub object: usize,
}

impl PfArity {
    /// A declared arity of `subject` subject-side and `object` object-side arguments.
    #[must_use]
    pub const fn new(subject: usize, object: usize) -> Self {
        Self { subject, object }
    }

    /// The total number of flattened positions (`subject + object`).
    #[must_use]
    pub const fn total(self) -> usize {
        self.subject + self.object
    }

    /// The all-free [`BindingPattern`] over this arity — the ⊥ of the lattice, the
    /// most general mode, and the one a relation declares when it can serve every
    /// access pattern (see [`PropertyFunction::modes`]).
    #[must_use]
    pub fn all_free_mode(self) -> BindingPattern {
        BindingPattern::from_bound_positions(self.total(), [])
    }
}

impl core::fmt::Display for PfArity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} subject / {} object", self.subject, self.object)
    }
}

/// The already-evaluated argument values of one invocation, per position.
///
/// A `Some` cell is a **bound** input — a constant written at the call site, or a
/// variable the incoming solution row binds. A `None` cell is **free**: the relation
/// is being asked to produce a value there. The two slices are the subject side and
/// the object side; together they are the flattened positions
/// [`PfArity`] documents.
///
/// The values are lent as borrows for the duration of the call, exactly as
/// [`crate::user_fn::NativeFnBody`] lends its arguments: they already live in the
/// evaluator's per-invocation buffer, and deep-cloning heap-string-owning
/// [`TermValue`]s on a per-row invocation path would be pure overhead. A relation
/// that must retain a value past [`PropertyFunction::open`] clones it itself.
#[derive(Debug, Clone, Copy)]
pub struct PfArgs<'a> {
    subject: &'a [Option<&'a TermValue>],
    object: &'a [Option<&'a TermValue>],
}

impl<'a> PfArgs<'a> {
    /// Build the argument view of one invocation from its two sides.
    #[must_use]
    pub const fn new(
        subject: &'a [Option<&'a TermValue>],
        object: &'a [Option<&'a TermValue>],
    ) -> Self {
        Self { subject, object }
    }

    /// The subject-side arguments, in written order.
    #[must_use]
    pub const fn subject(&self) -> &'a [Option<&'a TermValue>] {
        self.subject
    }

    /// The object-side arguments, in written order.
    #[must_use]
    pub const fn object(&self) -> &'a [Option<&'a TermValue>] {
        self.object
    }

    /// The invocation's positional shape, as counted from the argument vectors
    /// themselves (what the call site actually supplied, which
    /// [`open_contained`] checks against what the relation declared).
    #[must_use]
    pub const fn arity(&self) -> PfArity {
        PfArity::new(self.subject.len(), self.object.len())
    }

    /// Every position's value in flattened order (subject side, then object side).
    pub fn flattened(&self) -> impl Iterator<Item = Option<&'a TermValue>> + '_ {
        self.subject.iter().chain(self.object.iter()).copied()
    }

    /// The value at flattened position `pos`, or `None` when that position is free
    /// **or** out of range.
    #[must_use]
    pub fn get(&self, pos: usize) -> Option<&'a TermValue> {
        if pos < self.subject.len() {
            self.subject[pos]
        } else {
            self.object.get(pos - self.subject.len()).copied().flatten()
        }
    }

    /// The invocation's own access pattern: bit `p` set iff flattened position `p` is
    /// bound. This is the pattern a declared mode must
    /// [`subsume`](BindingPattern::subsumes) for the invocation to be feasible.
    #[must_use]
    pub fn mode(&self) -> BindingPattern {
        BindingPattern::from_bools(self.flattened().map(|value| value.is_some()))
    }
}

/// One row produced by an invocation: a value for **every** flattened position, in
/// the order [`PfArity`] documents — free positions and bound positions alike.
///
/// A bound position may be echoed back (the usual, and cheapest, thing for a relation
/// to do) or filled with any candidate the relation likes, because **the engine
/// applies equality filtering on bound positions**: a row whose value at a bound
/// position differs from the input value there is dropped before it becomes a
/// solution. A relation is therefore free to emit candidates and let the engine
/// filter, and echoing the input is simply the case where nothing is ever dropped.
pub type PfRow = Vec<TermValue>;

// ---------------------------------------------------------------------------
// What a cursor attests about the index behind it
// ---------------------------------------------------------------------------

/// Which version of a relation's backing index answered one invocation.
///
/// # Why the engine cannot mint this
///
/// A relation is host code over host state. Two runs of the same query, over the same
/// dataset, under the same registry, can legitimately give different rows because the
/// index behind a relation was rebuilt between them — and nothing the evaluator can
/// observe changes. The query text is the same, the dataset snapshot is the same, the
/// registry fingerprint (`crate::property_fn_plan::registry_fingerprint`) is the same,
/// because a rebuild changes no declaration. The one party that knows a rebuild
/// happened is the cursor, so the generation has to travel from there or not at all.
///
/// # Caller-declared, recorded verbatim
///
/// [`Self::Declared`] carries the host's own spelling of its generation, byte-for-byte,
/// and the engine never parses, orders by meaning, or interprets it. It is emphatically
/// **not** minted here from a clock, a wall time, a counter, or an RNG: a value this
/// crate invented would be a different number on every run and on every machine, which
/// would make every receipt disagree with every other one and prove nothing. It would
/// also be untrue — the engine has no idea when the index was built. (`purrdf-retrieval`
/// has a sibling notion in its `Statistics::revision`, and the relationship is
/// deliberately one of analogy only: this crate takes no dependency on that one, and a
/// host that has such a revision passes its spelling in here itself.)
///
/// # Why `Undeclared` is a first-class value
///
/// Most relations are not index-backed at all — an in-memory table, a computation over
/// its arguments, a walk over the dataset already being queried. Asking those to invent
/// a generation would be asking them to fabricate evidence. [`Self::Undeclared`] is the
/// honest absence, and it is what the default [`PfCursor::generation`] returns, so a
/// relation written before this seam existed keeps saying the true thing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexGeneration {
    /// The relation declared no generation: it is not index-backed, or its index is
    /// unversioned. An absence, never a claim that the index was current.
    Undeclared,
    /// The relation's own spelling of the generation that answered, recorded verbatim.
    Declared(String),
}

/// Whether a relation served an invocation from a WHOLE index, said so far as it can
/// say anything: the one fact only the relation knows, and the one it is asked for.
///
/// # Why there is no `Whole` variant, and never will be
///
/// The obvious third variant — a relation certifying that its index was complete — is
/// absent by design, because the seam that would carry it cannot distinguish the two
/// situations it would have to distinguish.
///
/// The first is the module header's argument: "I stopped early" and "I am exhausted"
/// are the same empty cursor. Nothing the engine sees on the way out of a drained
/// invocation tells it which of the two happened, which is exactly why the row ceiling
/// [`PropertyFunction::open`] receives is documented as a licence and not a contract
/// (see [`PropertyFunction`]'s "The ceiling is a licence, not a contract"). A relation
/// that stopped at the engine's ceiling is **not** incomplete — it answered the
/// question it was licensed to answer, in full — so it must not be recorded as such;
/// and it equally could not honestly certify wholeness, because it never looked at the
/// rows it was licensed to skip.
///
/// So the seam asks the narrower question, the one with an honest answer on every path:
/// *was your index NOT whole?* A shard that failed to load, a segment still being
/// rebuilt, a replica that has not caught up — those are facts the relation holds
/// directly and can state without inspecting anything it skipped.
///
/// # `Undeclared` is silence, not a completeness claim
///
/// [`Self::Undeclared`] means the relation said nothing, and a reader must not upgrade
/// it to "the index was whole". It is the default for every relation that never
/// overrides [`PfCursor::service_level`], which is every relation written before this
/// seam existed — reading silence as certification would retroactively put a claim in
/// each of their mouths that none of them made.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ServiceLevel {
    /// The relation declared nothing about its index's wholeness. The default, and an
    /// absence rather than a certificate of completeness.
    Undeclared,
    /// The relation declares that its index was **not** whole when it served this
    /// invocation, and states why in its own words.
    Incomplete {
        /// The relation's own description of what was missing — a shard name, a
        /// rebuild phase, a replica lag. Recorded verbatim and never parsed, for the
        /// same reason [`IndexGeneration::Declared`]'s string is.
        reason: String,
    },
}

/// The pair of facts one invocation attests about the index behind it: which version
/// answered, and whether that version was whole.
///
/// Carried as one value rather than two loose fields because the two are read at
/// different instants of the same invocation (see [`PfCursor::generation`] and
/// [`PfCursor::service_level`]) and are only meaningful together: a generation without
/// a service level says which index answered but not whether it was all there, and a
/// service level without a generation says an index was short without saying which one
/// a caller would have to rebuild.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PfAttestation {
    /// The generation read immediately after the cursor opened.
    pub generation: IndexGeneration,
    /// The service level read when the invocation ended.
    pub service: ServiceLevel,
}

impl PfAttestation {
    /// The attestation of a relation that declared neither fact — what every cursor
    /// that overrides neither method attests, named once so the "said nothing" value
    /// has one spelling everywhere instead of being re-assembled at each site.
    pub const UNDECLARED: Self = Self {
        generation: IndexGeneration::Undeclared,
        service: ServiceLevel::Undeclared,
    };
}

/// The row stream of one invocation, drained by the engine.
///
/// A cursor is opened, drained, and dropped inside a single invocation, so it never
/// crosses a thread boundary and carries no `Send` bound; the *relation* is shared
/// and therefore `Send + Sync` (see [`PropertyFunction`]).
///
/// # Emission order is the relation's contract
///
/// `next` MUST return rows in an order that is a pure function of the invocation's
/// arguments — never of iteration over a randomly-seeded map, of wall-clock time, or
/// of thread scheduling. The engine preserves that order and never re-sorts it, so it
/// reaches the query's answer verbatim: a relation with an unstable order makes the
/// query's result unstable, which no engine-side measure can repair.
///
/// # The deaf-relation doctrine
///
/// The evaluator polls its stop signal before opening a cursor and between successive
/// `next` calls, and charges the work a row costs **before** that row is consumed. A
/// relation that blocks forever inside one `next` call therefore degrades the
/// stop-check granularity to one call — it can make a stop *late*, never a partial
/// answer unsound, and never an answer wrong.
pub trait PfCursor {
    /// The next row, or `None` when the invocation is exhausted.
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the relation raises. Per the hard-fail doctrine this aborts
    /// the query rather than silently truncating the row stream — a short stream
    /// offered as complete is exactly the wrong answer the doctrine forbids.
    fn next(&mut self) -> Result<Option<PfRow>, EvalError>;

    /// The **internal work** this cursor has performed since this method last returned,
    /// *taken* — the count resets to zero, so consecutive calls partition the work rather
    /// than re-report it.
    ///
    /// This is the seam's answer to a question the engine cannot answer for itself: what
    /// did that call actually cost? The two quantities the engine can see — invocations
    /// driven and rows accepted — describe the *answer*, and for a generator relation the
    /// answer is not where the work is. A nearest-neighbour search that examines a million
    /// vectors to return the five closest emits five rows, and priced by rows it is
    /// indistinguishable from a five-row table. Reporting the million is what makes a
    /// caller's budget a bound on the execution rather than on the result set.
    ///
    /// The engine charges [`ChargePoint::PropertyFunctionWork`](crate::governor::ChargePoint::PropertyFunctionWork)
    /// once per reported unit, after every [`Self::next`] — including the terminating call
    /// that returns `None`, so a cursor that searches lazily on its first pull and one
    /// that searched eagerly in [`PropertyFunction::open`] are charged the same total. A
    /// cursor that reports work it has not yet done is not wrong, merely early.
    ///
    /// # What a unit is
    ///
    /// Whatever the implementing relation's own documentation says it is: one candidate
    /// examined, one posting decoded, one row of an external table read. The engine cannot
    /// define the unit for host code and does not try; it prices each reported unit at one
    /// and requires the relation to say what it counted. A relation whose work is
    /// genuinely proportional to the rows it emits has nothing to add here and keeps the
    /// default.
    ///
    /// # Why over-reporting is not a hazard, and under-reporting is not a hole
    ///
    /// The count is *spent*, not merely recorded, so a relation that inflates it exhausts
    /// its own caller's budget — an incentive pointing the right way. A relation that
    /// under-reports (or, by default, reports nothing) makes its query cheaper than it
    /// should be, but every other ceiling stays in force unchanged: the invocation point,
    /// the row point, the intermediate-cell peak, the answer cap and the wall deadline all
    /// bound it exactly as they did before this method existed. Under-reporting can
    /// therefore cost a caller precision in a receipt; it can never cost them soundness,
    /// and no engine-side measure can see inside host code to do better.
    ///
    /// # Default
    ///
    /// Zero. Every relation written before this method existed reports no work and charges
    /// nothing, which is what makes a budget sized against the previous profile version
    /// buy the same execution.
    fn take_work(&mut self) -> u64 {
        0
    }

    /// Which version of the backing index is answering this invocation.
    ///
    /// Read by the evaluator **immediately after [`open_contained`] returns**, and once
    /// per invocation. That instant is not an implementation convenience: an index-backed
    /// relation pins its snapshot when it opens, so "which generation is answering" is
    /// true from that moment and stays true for every row this cursor goes on to emit.
    /// Reading it later would let a rebuild that landed mid-drain be reported as the
    /// generation that produced rows it did not produce.
    ///
    /// # Default
    ///
    /// [`IndexGeneration::Undeclared`] — the same defaulting precedent
    /// [`Self::take_work`] set, and for the same reason: every relation written before
    /// this method existed keeps compiling, keeps answering, and says the one true thing
    /// about itself rather than being made to invent a version it does not have. See
    /// [`IndexGeneration`] for why this value is never minted engine-side.
    fn generation(&self) -> IndexGeneration {
        IndexGeneration::Undeclared
    }

    /// Whether the index behind this cursor was **not** whole while it served this
    /// invocation.
    ///
    /// Read by the evaluator when the invocation **ENDS** — after this cursor has
    /// returned `Ok(None)`, and equally when the engine stopped pulling at its own row
    /// ceiling, at a governor trip, or at a stop signal. The end, not the beginning,
    /// because a shard discovered missing on the four-hundredth pull is exactly the case
    /// this channel exists for, and a reading taken at `open` would have no way to carry
    /// it. A cursor that knew at `open` that it was short simply answers the same way at
    /// both instants; one that learns late still has somewhere to say so.
    ///
    /// # Default
    ///
    /// [`ServiceLevel::Undeclared`], by the same defaulting precedent as
    /// [`Self::take_work`] and [`Self::generation`]. Note carefully what the default is
    /// NOT: it is silence, not a certificate that the index was whole — see
    /// [`ServiceLevel`] for why no such certificate is askable at this seam at all.
    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }
}

// ---------------------------------------------------------------------------
// Ranked-retrieval declarations
// ---------------------------------------------------------------------------

/// The kind of RDF term a [`TermPattern`] accepts.
///
/// This is a closed, declarative classification, not a predicate function: a
/// consumer matches an incoming request term against it by a lookup over data,
/// which is what makes a capability declaration serializable and stable across
/// processes. `Any` is the explicit "no kind restriction" declaration and is
/// deliberately distinct from a wildcard implementation — callers read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermKind {
    /// Any term kind is accepted.
    Any,
    /// An IRI term.
    Iri,
    /// A blank-node term.
    Blank,
    /// A literal term.
    Literal,
    /// A quoted triple term (RDF 1.2).
    Triple,
}

impl TermKind {
    /// The stable spelling used in the canonical description.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Iri => "iri",
            Self::Blank => "blank",
            Self::Literal => "literal",
            Self::Triple => "triple",
        }
    }
}

/// One declarative shape of request term a ranked producer accepts.
///
/// A pattern is matched by field equality over an incoming term's kind,
/// datatype IRI, language tag, and (for an associated predicate) IRI. There is
/// no function pointer here by construction: function pointers cannot be
/// serialized, cannot be compared for equality, and are not a stable
/// description of accepted request language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TermPattern {
    /// The term kind this pattern accepts.
    pub kind: TermKind,
    /// If set, the literal datatype IRI the term must carry.
    pub datatype: Option<String>,
    /// If set, the literal language tag the term must carry.
    pub language: Option<String>,
    /// If set, the predicate IRI the term's triple must carry.
    pub predicate: Option<String>,
}

impl TermPattern {
    /// A pattern accepting any term of `kind`, with no further constraint.
    #[must_use]
    pub const fn of_kind(kind: TermKind) -> Self {
        Self {
            kind,
            datatype: None,
            language: None,
            predicate: None,
        }
    }

    /// Append this pattern's canonical, injective description to `out`.
    fn push_canonical(&self, out: &mut String) {
        push_canonical_field(out, self.kind.as_str());
        push_canonical_option(out, self.datatype.as_deref());
        push_canonical_option(out, self.language.as_deref());
        push_canonical_option(out, self.predicate.as_deref());
    }
}

/// A ranked producer's duplicate handling within one invocation's stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DuplicatePolicy {
    /// An item appears at most once in the stream.
    Unique,
    /// The stream may repeat an item; a consumer must de-duplicate.
    Allowed,
}

impl DuplicatePolicy {
    /// The stable spelling used in the canonical description.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unique => "unique",
            Self::Allowed => "allowed",
        }
    }
}

/// Whether a producer names every row that was due to it, above the boundary it
/// read to.
///
/// The first of the two axes of [`RankFidelity`]. It answers *did you find
/// everything*, and it is the axis an approximate index fails: an HNSW beam that
/// does not visit a node cannot emit the row that node holds, however good that
/// row was.
///
/// # There is no `Unknown`, and the reason is structural
///
/// Unlike [`ServiceLevel`], which needs [`ServiceLevel::Undeclared`] because
/// [`PfCursor::service_level`] has a default and a relation may honestly stay
/// silent, this axis has no silence available: it is a required field of
/// [`RankedDeclaration`], a bare-field struct with no builder and no `Default`,
/// so every producer that registers states it. "Declared no loss" and "is
/// complete" are therefore the same state, and naming it [`Self::Complete`]
/// reports the true thing rather than an unknown.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Completeness {
    /// The producer's search is exhaustive over its own corpus: every row that
    /// ranks above the boundary it read to is emitted.
    Complete,
    /// The producer's search may omit rows that were due, and states why in its
    /// own words.
    Lossy {
        /// The producer's own evidence, recorded verbatim and never parsed.
        ///
        /// An `Arc<str>` rather than a `String` because it is immutable,
        /// producer-owned, and identical across every stream that producer
        /// serves, while the contract carrying it is cloned once per stream per
        /// fusion — the same reason the text relations hold their generation
        /// this way.
        evidence: Arc<str>,
    },
}

/// Whether a producer's emitted rank is a lower bound on the row's true rank.
///
/// The second axis of [`RankFidelity`], independent of [`Completeness`], and
/// **the precondition for any finite bound on a fused score**.
///
/// # Why this is a separate axis and not a shade of the first
///
/// Under [`Self::Faithful`] the emitted sequence is a *subsequence* of the true
/// ranking: rows may be missing, but those that arrive arrive in true order, so
/// a row's true rank is at least its emitted rank and its true contribution is
/// therefore at most the contribution its emitted rank earns. That inequality is
/// the whole of what makes a consumer's upper bound computable.
///
/// Under [`Self::Perturbed`] it does not hold. A producer that compares
/// *approximated* values — quantized vectors, a sketched score — can rank a row
/// it did find **better** than that row was due, so no finite bound on its
/// contribution exists at all. A consumer must be told that rather than handed a
/// number, and an answer containing such a stratum says so instead of inventing
/// one.
///
/// HNSW is [`Self::Faithful`]: it computes exact distances for every candidate
/// it visits and fails only to visit. A product-quantized index is
/// [`Self::Perturbed`]. The two are both "approximate" in ordinary speech and
/// differ in exactly the property that decides whether an answer can be bounded,
/// which is why one flag cannot carry both.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderFidelity {
    /// Rows arrive in true relative order: an emitted rank is a lower bound on
    /// the row's true rank.
    Faithful,
    /// A row may be emitted at a better rank than it was due, and the producer
    /// states why in its own words.
    Perturbed {
        /// The producer's own evidence, recorded verbatim and never parsed.
        evidence: Arc<str>,
    },
}

/// What a producer promises about the rows it emits, on two independent axes.
///
/// [`Completeness`] answers *did you find every row that was due*;
/// [`OrderFidelity`] answers *are the rows you did find in true relative order*.
/// The product is a four-element lattice ordered by strength, with
/// [`Self::EXACT`] (`Complete` × `Faithful`) at the top, and a fused answer's own
/// guarantee is the **meet** over the streams it was handed.
///
/// | Producer | Class |
/// |---|---|
/// | BM25 over a full inverted index | `Complete` × `Faithful` |
/// | an exhaustive kNN scan | `Complete` × `Faithful` |
/// | an HNSW graph | `Lossy` × `Faithful` |
/// | a product-quantized index | `Lossy` × `Perturbed` |
///
/// # Why two axes rather than one `approximate` flag
///
/// Because the two failures have different consequences for a consumer and only
/// one of them is boundable. Loss alone deflates a missed candidate's score and
/// — because reciprocal-rank fusion scores by **rank** and nothing else —
/// simultaneously *inflates* the score of every row behind the missing one,
/// which moves up a rank and collects more than it earned. Both effects are
/// bounded under `Faithful`. Under `Perturbed` neither is.
///
/// # There is no silence here, and that is what lets `Complete` mean `Complete`
///
/// A consumer reads an absent declaration as *exact*, which is only honest if a
/// producer cannot fail to declare one. It cannot: [`RankedDeclaration`] is a
/// bare-field struct with no builder and no `Default`, so a literal that omits
/// the field does not compile —
///
/// ```compile_fail
/// # use purrdf_sparql_eval::{RankedDeclaration, DuplicatePolicy, CandidateDomains};
/// # let stratum = purrdf_core::parse_iri("http://example.org/s").unwrap();
/// // No `fidelity`. This is a compile error, not a silent default.
/// let _ = RankedDeclaration {
///     stratum,
///     accepted_terms: Vec::new(),
///     depth_placement: None,
///     candidate_position: 0,
///     duplicates: DuplicatePolicy::Unique,
///     domains: CandidateDomains::Unrestricted,
///     mandatory: false,
/// };
/// ```
///
/// — while the same literal supplying it does. The pair is what proves the
/// point: a `compile_fail` block alone passes for *any* error, including a typo,
/// so the twin that differs only in the field under test is the half that shows
/// the field is the reason.
///
/// ```
/// # use purrdf_sparql_eval::{RankedDeclaration, RankFidelity, DuplicatePolicy, CandidateDomains};
/// # let stratum = purrdf_core::parse_iri("http://example.org/s").unwrap();
/// let declaration = RankedDeclaration {
///     stratum,
///     accepted_terms: Vec::new(),
///     depth_placement: None,
///     candidate_position: 0,
///     duplicates: DuplicatePolicy::Unique,
///     fidelity: RankFidelity::EXACT,
///     domains: CandidateDomains::Unrestricted,
///     mandatory: false,
/// };
/// assert_eq!(declaration.fidelity, RankFidelity::EXACT);
/// ```
///
/// And there is no `Default` to reach for instead:
///
/// ```compile_fail
/// # use purrdf_sparql_eval::RankFidelity;
/// let _: RankFidelity = Default::default();
/// ```
///
/// [`Self::EXACT`] is the value a host spells when it has nothing to disclose,
/// and spelling it is the point.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RankFidelity {
    /// Whether every row that was due is named.
    pub completeness: Completeness,
    /// Whether a named row's emitted rank is a lower bound on its true rank.
    pub order: OrderFidelity,
}

impl RankFidelity {
    /// Complete and order-faithful: the top of the lattice.
    ///
    /// A named constant the caller must spell, deliberately **not** a `Default`.
    /// The value is a promise about the producer's own search, and only the host
    /// knows it — the same refusal [`RankedDeclaration::domains`] already makes,
    /// and the same discipline [`PfAttestation::UNDECLARED`] follows.
    pub const EXACT: Self = Self {
        completeness: Completeness::Complete,
        order: OrderFidelity::Faithful,
    };

    /// The evidence this producer published, if it declared any loss on either
    /// axis, in axis order.
    ///
    /// Carried verbatim. A consumer renders these; nothing in this workspace
    /// parses them.
    pub fn evidence(&self) -> impl Iterator<Item = &Arc<str>> {
        let completeness = match &self.completeness {
            Completeness::Complete => None,
            Completeness::Lossy { evidence } => Some(evidence),
        };
        let order = match &self.order {
            OrderFidelity::Faithful => None,
            OrderFidelity::Perturbed { evidence } => Some(evidence),
        };
        completeness.into_iter().chain(order)
    }

    /// Whether this producer may have failed to name a row that was due.
    ///
    /// True for [`Completeness::Lossy`]. A consumer reads it to decide whether a
    /// stratum's terminal status is, on its own, a completeness claim — it is
    /// not when this is true.
    #[must_use]
    pub const fn may_omit(&self) -> bool {
        matches!(self.completeness, Completeness::Lossy { .. })
    }

    /// Whether this producer may have named a row at a better rank than it was
    /// due, which is the case in which **no finite score bound exists**.
    #[must_use]
    pub const fn order_is_unbounded(&self) -> bool {
        matches!(self.order, OrderFidelity::Perturbed { .. })
    }

    /// Append this fidelity's canonical, injective description to `out`.
    ///
    /// Each axis contributes a length-framed discriminant followed by an
    /// explicit present/absent evidence byte, so an absent evidence and an empty
    /// one stay distinguishable — the framing discipline the rest of this
    /// module's canonical encoding uses, with no escaping anywhere.
    fn push_canonical(&self, out: &mut String) {
        match &self.completeness {
            Completeness::Complete => {
                push_canonical_field(out, "complete");
                push_canonical_option(out, None);
            }
            Completeness::Lossy { evidence } => {
                push_canonical_field(out, "lossy");
                push_canonical_option(out, Some(evidence));
            }
        }
        match &self.order {
            OrderFidelity::Faithful => {
                push_canonical_field(out, "faithful");
                push_canonical_option(out, None);
            }
            OrderFidelity::Perturbed { evidence } => {
                push_canonical_field(out, "perturbed");
                push_canonical_option(out, Some(evidence));
            }
        }
    }
}

/// One caller-named block of the candidate universe.
///
/// A domain tag is an IRI the **caller** chooses — `documents`, `people`,
/// `places`, `chunks-of-the-2019-corpus`. Nothing here mints one, and nothing
/// here interprets one: the tag is compared for equality against other tags and
/// is otherwise opaque data, exactly as a stratum label is.
///
/// # A tag names a block of a partition, not a label an item may collect
///
/// The whole arithmetic value of a declared domain rests on this reading, so it
/// is stated rather than implied: the tags a host uses **partition** the
/// candidate universe, and a candidate lies in exactly one block. That is what
/// lets a consumer bound the score of an item it has not seen yet by the best
/// single block rather than by every stream at once — an unseen item cannot
/// collect contributions from two disjoint blocks, because it is not in two
/// blocks.
///
/// A host that tags one entity into two blocks has not made the declaration
/// weaker, it has made it false, and the consumer says so rather than quietly
/// scoring it: `purrdf-retrieval`'s fusion refuses a stream that names a
/// candidate its declared domains cannot reach. A host whose entity really does
/// belong to two categories declares a producer over **both** tags
/// ([`CandidateDomains::Within`] takes a set) or declares
/// [`CandidateDomains::Unrestricted`], and both of those are honest.
///
/// # Why this is a newtype rather than a bare [`Iri`]
///
/// A set of tags has to be ordered to be a set, to be compared, and to be
/// encoded injectively into a registry's fingerprint, and the kernel's [`Iri`]
/// is a parsed value with neither an `Ord` nor a `Hash` impl — deliberately, as
/// the ring-fenced leaf it is. So the ordering lives here, over the IRI's text,
/// which is the same thing `purrdf-retrieval`'s plan-facing IRI wrapper does
/// and for the same reason. Ordering by text is what makes the canonical
/// encoding a pure function of the value rather than of the order a host
/// happened to insert its tags in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainTag(Iri);

impl DomainTag {
    /// Carry `iri` as a domain tag.
    #[must_use]
    pub const fn new(iri: Iri) -> Self {
        Self(iri)
    }

    /// Parse and validate `text` as a domain tag.
    ///
    /// # Errors
    ///
    /// The parser's own [`IriError`](purrdf_core::IriError) when `text` is not
    /// a valid IRI. Nothing is guessed or repaired: a tag that is not an IRI is
    /// refused where it is written.
    pub fn parse(text: &str) -> Result<Self, purrdf_core::IriError> {
        purrdf_core::parse_iri(text).map(Self)
    }

    /// The tag's IRI text, verbatim.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The tag's underlying IRI.
    #[must_use]
    pub const fn as_iri(&self) -> &Iri {
        &self.0
    }
}

impl From<Iri> for DomainTag {
    fn from(iri: Iri) -> Self {
        Self(iri)
    }
}

impl core::fmt::Display for DomainTag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl PartialOrd for DomainTag {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DomainTag {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.as_str().cmp(other.0.as_str())
    }
}

impl core::hash::Hash for DomainTag {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

/// Which blocks of the candidate universe a ranked producer may name.
///
/// This is the second promise a producer makes about its own rows, beside
/// [`DuplicatePolicy`], and like that one it is host-supplied configuration
/// read at registration rather than anything a consumer infers. It exists
/// because a consumer fusing several ranked streams has exactly one way to
/// learn that a stream will *not* name a candidate — read that stream to its
/// end — and over strata whose candidate sets do not overlap that is every row
/// of every stream, however small the caller's top-k.
///
/// # What it buys, stated as the arithmetic it licenses
///
/// A fused score is exact only when every stream that could still name the
/// candidate has named it. With no declaration, "could still name it" is true
/// of every open stream, so a candidate in one stratum waits for a stratum that
/// was never going to mention it — the fusion drains, the frontier grows with
/// the input, and the terminal report says `Exhausted` about streams that were
/// emptied rather than bounded. With a declaration, the consumer may skip the
/// streams that *provably* cannot name the candidate, and only those. The
/// consumer's finality test does not get weaker; its quantifier gets smaller.
///
/// # [`Self::Unrestricted`] is the wider promise, not the weaker one
///
/// It says "this producer may name anything", which is a statement about the
/// producer that happens to license the consumer to assume nothing. It is
/// today's behaviour exactly, it is the honest value for a host that does not
/// know how its indexes partition, and a fusion whose every stream declares it
/// computes precisely the numbers it computed before this existed — the same
/// rows, the same scores, the same provenance, and the same reading cost.
///
/// # Nothing is defaulted from a stratum or from a graph
///
/// The tags come from the **host**, because the host is the only party that
/// knows whether its text index and its vector index name the same entities. A
/// consumer deriving a tag per stratum would hand two producers over one entity
/// space a pair of tags it reads as disjoint, and there is no safe way for that
/// to be wrong: the pair either makes the consumer refuse a query that was
/// valid, or lets it certify a score that is missing a contribution the other
/// stream was about to make. A wrong guess here is a wrong answer, so there is
/// no guess.
///
/// # What verification can and cannot reach
///
/// A consumer verifies this declaration only over the rows it actually pulls: a
/// stream that names a candidate another stream's declaration has already
/// placed elsewhere is refused, and nothing else is checkable. A false
/// declaration that no pulled row contradicts produces a score that declaration
/// made wrong. That is the same trust the seam already places in
/// [`DuplicatePolicy::Unique`], whose breach is likewise detected only when the
/// repeat is actually read, and it is stated here rather than dressed up: a
/// declaration is a promise, and the consumer checks the promises it is in a
/// position to check.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateDomains {
    /// The producer may name any candidate at all.
    Unrestricted,
    /// Every candidate this producer names lies in one of these blocks.
    ///
    /// Never empty. A producer that promises to name nothing has not restricted
    /// its domain, it has described a producer that should not be registered,
    /// and
    /// [`register_ranked`](PropertyFunctionRegistry::register_ranked) refuses
    /// it where it is written.
    Within(std::collections::BTreeSet<DomainTag>),
}

impl CandidateDomains {
    /// A restriction to the tags `tags` yields, in any order.
    ///
    /// A convenience over building the set, and it is deliberately **not** the
    /// place the empty case is refused: this is plain data with public
    /// variants, so a host can spell an empty restriction either way and the
    /// one refusal that matters is the one at registration, where the
    /// declaration is committed.
    pub fn within<I: IntoIterator<Item = DomainTag>>(tags: I) -> Self {
        Self::Within(tags.into_iter().collect())
    }

    /// Whether some candidate could satisfy both restrictions at once.
    ///
    /// [`Self::Unrestricted`] intersects everything, including itself: it is
    /// "any block", so there is always a block in common. Two restricted
    /// declarations intersect exactly when they name a block in common —
    /// because a candidate lies in exactly one block ([`DomainTag`]), so a
    /// candidate both could name must be in a block both declared.
    ///
    /// Allocates nothing: the set intersection is the lazy iterator's first
    /// step and no result set is built.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unrestricted, _) | (_, Self::Unrestricted) => true,
            (Self::Within(left), Self::Within(right)) => left.intersection(right).next().is_some(),
        }
    }

    /// Whether a candidate in block `tag` could be named under this
    /// declaration.
    #[must_use]
    pub fn admits(&self, tag: &DomainTag) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Within(tags) => tags.contains(tag),
        }
    }

    /// The blocks this declaration names, or `None` when it names no
    /// restriction at all.
    #[must_use]
    pub const fn tags(&self) -> Option<&std::collections::BTreeSet<DomainTag>> {
        match self {
            Self::Unrestricted => None,
            Self::Within(tags) => Some(tags),
        }
    }

    /// Append this declaration's canonical, injective description to `out`.
    ///
    /// A present/absent discriminant, then — for a restriction — the tag count
    /// and every tag length-framed in canonical order. The count is what makes
    /// a set of two tags unreadable as a set of one followed by whatever came
    /// next, and the `BTreeSet`'s own order is what makes the bytes a function
    /// of the value rather than of the host's insertion order.
    fn push_canonical(&self, out: &mut String) {
        match self {
            Self::Unrestricted => out.push('0'),
            Self::Within(tags) => {
                out.push('1');
                out.push_str(&tags.len().to_string());
                out.push(':');
                for tag in tags {
                    push_canonical_field(out, tag.as_str());
                }
            }
        }
        out.push(';');
    }
}

/// The canonical description of "this relation does not participate in ranked
/// retrieval" — the one byte a non-ranked producer contributes to a registry's
/// content fingerprint, named once so the ranked and non-ranked spellings can
/// never drift apart.
pub(crate) const NOT_RANKED_CANONICAL: &str = "n";

/// Which facet of a request term a placement renders into an argument position.
///
/// A request term is not always one value at the call site. A lexical search
/// for an English needle is a needle *and* a language tag, and a producer that
/// takes them in two different argument positions needs both rendered, or the
/// position left free answers a narrower question than the caller asked — the
/// silent widening this enum exists to make impossible to express by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RequestFacet {
    /// The term's own value — the needle, the vector, the IRI.
    Value,
    /// The term's language tag.
    Language,
    /// The predicate IRI the term is associated with.
    Predicate,
    /// The maximum distance the term's match may lie at.
    MaxDistance,
    /// The inclusive lower endpoint of a term that names an interval.
    ///
    /// An interval is two constants, not one, so it cannot ride in
    /// [`Self::Value`]: a producer that received only one endpoint would answer
    /// a strictly wider question than the caller asked, which is the silent
    /// widening this enum exists to make impossible to express by accident.
    LowerBound,
    /// The inclusive upper endpoint of a term that names an interval.
    UpperBound,
}

impl RequestFacet {
    /// The stable spelling used in the canonical description.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Language => "language",
            Self::Predicate => "predicate",
            Self::MaxDistance => "max-distance",
            Self::LowerBound => "lower-bound",
            Self::UpperBound => "upper-bound",
        }
    }
}

/// Where one facet of an accepted request term binds, and how it is rendered.
///
/// `position` is a **flattened** argument position in the sense [`PfArity`]
/// documents: the subject-side arguments first, in written order, then the
/// object-side arguments, 0-based across both.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TermPlacement {
    /// The facet of the request term this position receives.
    pub facet: RequestFacet,
    /// The flattened argument position the facet is rendered into.
    pub position: usize,
    /// If set, the literal datatype IRI the rendered value carries. No
    /// vocabulary is minted here: an absent datatype is an absent datatype, and
    /// a producer that needs one says which.
    pub datatype: Option<String>,
}

/// One accepted request-term shape, with where its facets bind.
///
/// The pattern is the *matching* half — which incoming terms this producer will
/// take — and the placements are the *rendering* half — where each facet of a
/// matched term goes. They are paired rather than carried as two lists, so a
/// reader cannot mis-align a placement with the shape it renders.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AcceptedTerm {
    /// The declarative shape of request term this entry accepts.
    pub pattern: TermPattern,
    /// Where each facet of a matched term binds, in the order the producer
    /// declared them. The order is identity-bearing and is never sorted.
    pub placements: Vec<TermPlacement>,
}

/// Where a producer takes its per-stratum depth as an argument, if it does.
///
/// Some producers are bounded by the consumer's `LIMIT`; others take the depth
/// as an argument and are *refused* at prepare time when it is left free,
/// because an unbounded generator cannot be admitted against a row ceiling. A
/// declaration says which, rather than leaving a consumer to guess.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepthPlacement {
    /// The flattened argument position the depth is rendered into.
    pub position: usize,
    /// The literal datatype IRI the rendered depth carries. Required, not
    /// optional: a fabricated default would be a minted vocabulary IRI.
    pub datatype: String,
}

/// A producer's ranked-retrieval declaration, supplied at registration.
///
/// This is caller-supplied configuration about a relation, not a property *of*
/// the relation's Rust type: the same generic relation can be wired up as a
/// ranked producer in one host and as an ordinary row source in another, and
/// only the host wiring it knows which. So it is supplied where the producer is
/// registered ([`PropertyFunctionRegistry::register_ranked`]) and read back by
/// IRI ([`PropertyFunctionRegistry::ranked_declaration`]), and every relation
/// that has nothing to do with ranked retrieval says nothing at all.
///
/// Every field is owned, declarative data, so the whole value is
/// `Clone + PartialEq + Eq + Debug` and has a canonical description; there is
/// deliberately no function pointer anywhere in this type. It is not `Hash`
/// because [`Iri`] is not.
///
/// # The rank law is single and unconditional
///
/// A declaration says nothing about rank order because there is nothing to say:
/// one law holds for every ranked producer without exception. The ranks of one
/// invocation's rows are 1-based, contiguous and ascending — rank 1, then 2,
/// then 3, with no gap, no repeat and no step backwards. A producer emits rows
/// best-first and numbers them as it goes, and rows that a producer considers
/// equally good still receive distinct consecutive ranks under whatever total
/// tie-break it applies.
///
/// The law is enforced on the consuming side, per row, as the rows arrive: the
/// fusion engine in `purrdf-retrieval` holds the next expected rank for every
/// stream it reads and refuses a lower rank as `OutOfOrderRanks` and a higher
/// one as `NonContiguousRanks`. Because it is checked rather than believed, it
/// is not a field a registration can vary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedDeclaration {
    /// The caller-supplied stratum label these rows are ranked within. No
    /// vocabulary is minted here; the caller names its own strata.
    pub stratum: Iri,
    /// The request-term shapes this producer accepts, each with the positions
    /// its facets render into. Matching is a lookup over these declarations,
    /// never inference. Caller order is identity-bearing.
    pub accepted_terms: Vec<AcceptedTerm>,
    /// Where the per-stratum depth binds, or `None` when this producer is
    /// bounded by the consumer's row ceiling instead of by an argument.
    pub depth_placement: Option<DepthPlacement>,
    /// The flattened argument position the ranked candidate is projected from.
    pub candidate_position: usize,
    /// The producer's duplicate handling.
    pub duplicates: DuplicatePolicy,
    /// What this producer promises about the rows it emits: whether it names
    /// every row that was due, and whether a named row's rank is a lower bound
    /// on its true rank.
    ///
    /// The third promise a producer makes about its own rows, and the one that
    /// decides whether a consumer may read a terminal status as a completeness
    /// claim. A [`Completeness::Lossy`] producer that runs out of rows has run
    /// out of rows its *search* found, which is a different statement from the
    /// one an exhaustive producer makes with the same status — see
    /// [`RankFidelity`] for why the two axes are independent and why only one of
    /// them admits a finite bound.
    pub fidelity: RankFidelity,
    /// Which blocks of the candidate universe this producer may name.
    ///
    /// The second promise a producer makes about its own rows. A consumer
    /// fusing several ranked streams reads it to learn which streams *cannot*
    /// name a given candidate, which is the only way it can certify a score
    /// before reading a stream to its end — see [`CandidateDomains`] for the
    /// whole argument, including why [`CandidateDomains::Unrestricted`] is the
    /// wider promise rather than the weaker one, and why nothing derives a tag
    /// from a stratum or a graph on a host's behalf.
    pub domains: CandidateDomains,
    /// Whether a request that reaches this producer must actually be served by
    /// it. Declared by the host, never inferred by a consumer: admission
    /// enforces whatever the registry declared and adds nothing of its own.
    pub mandatory: bool,
}

impl RankedDeclaration {
    /// A canonical, injective, length-framed description of this declaration.
    ///
    /// A pure function of the value: it does not depend on registration order,
    /// on iteration order, or on the host that built it. Every string is
    /// length-prefixed and every optional string carries an explicit
    /// present/absent byte, so an absent field and an empty one stay
    /// distinguishable; every list is preceded by its own length, so a reader
    /// knows exactly how many elements to consume.
    #[must_use]
    pub fn canonical_description(&self) -> String {
        let mut out = String::new();
        out.push('r');
        push_canonical_field(&mut out, self.stratum.as_str());
        push_canonical_field(&mut out, self.duplicates.as_str());
        // Beside the duplicate policy, because the three are the same kind of
        // fact: a promise the producer makes about its own rows that a consumer
        // holds it to. All must reach the registry's content fingerprint, or
        // two registries that fuse differently could share a digest and a plan
        // admitted against one would run against the other. Fidelity in
        // particular: a plan whose producers approximate is a different plan
        // from one whose producers do not, and the answers differ in what they
        // may be read to claim.
        self.fidelity.push_canonical(&mut out);
        self.domains.push_canonical(&mut out);
        push_canonical_field(&mut out, &self.candidate_position.to_string());
        out.push(if self.mandatory { '1' } else { '0' });
        out.push(';');
        match self.depth_placement.as_ref() {
            None => out.push('0'),
            Some(depth) => {
                out.push('1');
                push_canonical_field(&mut out, &depth.position.to_string());
                push_canonical_field(&mut out, &depth.datatype);
            }
        }
        out.push(';');
        out.push_str(&self.accepted_terms.len().to_string());
        out.push(':');
        for term in &self.accepted_terms {
            out.push('\u{1}');
            term.pattern.push_canonical(&mut out);
            out.push_str(&term.placements.len().to_string());
            out.push(':');
            for placement in &term.placements {
                out.push('\u{6}');
                push_canonical_field(&mut out, placement.facet.as_str());
                push_canonical_field(&mut out, &placement.position.to_string());
                push_canonical_option(&mut out, placement.datatype.as_deref());
            }
        }
        out
    }

    /// Every placement of every accepted term, in declaration order.
    fn placements(&self) -> impl Iterator<Item = &TermPlacement> {
        self.accepted_terms
            .iter()
            .flat_map(|term| term.placements.iter())
    }
}

/// Append a length-framed canonical field to `out`.
///
/// `pub(crate)` rather than private because [`crate::witness`] encodes its own
/// canonical record with the identical framing: one length-framed-field discipline for
/// the whole crate means two encodings written years apart cannot come to disagree
/// about what "framed" means.
pub(crate) fn push_canonical_field(out: &mut String, value: &str) {
    out.push_str(&value.len().to_string());
    out.push(':');
    out.push_str(value);
}

/// Append a present/absent discriminant and, when present, a framed value.
fn push_canonical_option(out: &mut String, value: Option<&str>) {
    match value {
        None => out.push('0'),
        Some(value) => {
            out.push('1');
            push_canonical_field(out, value);
        }
    }
    out.push(';');
}

// ---------------------------------------------------------------------------
// The relation trait
// ---------------------------------------------------------------------------

/// A host-injected relation invoked from predicate position.
///
/// Object-safe and shared behind an [`Arc`] across the whole evaluation (including
/// fork-join workers), hence `Send + Sync`. It works entirely in dataset-independent
/// [`TermValue`] space: a relation never sees a dataset-local
/// [`TermId`](purrdf_core::TermId), so a registry built once is valid against any
/// dataset.
///
/// # Feasibility: which invocations a relation can serve
///
/// [`modes`](Self::modes) declares the access patterns the relation can compute,
/// over the flattened positions [`PfArity`] documents. An invocation whose own
/// pattern is `p` is **feasible** iff some declared mode `m` satisfies
/// `m.subsumes(p)` — that is, `bound(m) ⊆ bound(p)`, the subsumption order
/// [`BindingPattern`] defines.
///
/// The subset direction is the useful one: a relation that can serve `fb` (object
/// bound, subject free) can also serve `bb`, by producing the `fb` rows and letting
/// the engine's equality filter on the now-bound subject position discard the
/// mismatches — generate-then-filter. The converse does not hold, which is why a
/// relation that declares only `bf` genuinely cannot answer an `ff` invocation. A
/// relation that can serve everything declares exactly one mode, the all-free
/// [`PfArity::all_free_mode`], which subsumes every pattern of its arity.
///
/// # The ceiling is a licence, not a contract
///
/// [`open`](Self::open) receives the row ceiling the engine's answer-cap pushdown
/// computed for the node this invocation's rows belong to, when it has one — a `LIMIT`
/// or an answer cap, arrived at through the plan's soundness certificate. It is
/// permission to stop early, so that a generator (a text index, a nearest-neighbour
/// search) can bound its own work instead of producing rows nobody will read. The
/// engine stops consuming at its own ceiling regardless, so a relation that ignores the
/// argument entirely is correct, merely less efficient.
///
/// A relation that *does* use it counts **rows it emits that agree with the bound
/// positions it was handed**. That distinction is the whole of the obligation, and it
/// exists because a relation is entitled to generate candidates and let the engine's
/// equality filter cut them (see [`PfRow`]): candidates the relation itself can see are
/// doomed must not be counted against the licence, or a stop at `k` would hand back
/// fewer than `k` usable rows and the engine would read the short bag as an exhausted
/// one. Everything the engine filters on that a relation could *not* see — a repeated
/// variable across two free positions, a partially-bound quoted triple — is handled on
/// the engine's side: it withholds the ceiling entirely for such a call rather than ask
/// a relation to account for something it was never told.
pub trait PropertyFunction: Send + Sync {
    /// The relation's determinism class. Read by the fork-join parallel gate exactly
    /// as [`crate::user_fn::NativeFunction`]'s is: only
    /// [`Volatility::Stable`] may run across workers, and a
    /// relation that misdeclares itself diverges silently under parallel evaluation.
    fn volatility(&self) -> Volatility;

    /// The relation's declared positional arity. Checked against the call site before
    /// any of this trait's other methods run.
    fn arity(&self) -> PfArity;

    /// The access patterns this relation can compute, over the flattened positions
    /// [`PfArity`] documents. See the trait docs for the subsumption rule that turns
    /// this list into a yes/no answer for one invocation.
    ///
    /// Every returned pattern must have arity [`PfArity::total`]; a pattern of any
    /// other arity is incomparable with every invocation and so admits nothing.
    fn modes(&self) -> &[BindingPattern];

    /// The declared upper bound on the number of rows one invocation emits under
    /// `mode` — the cardinality class the planner reads.
    ///
    /// This is consulted twice: to order a call against the other operators of its
    /// group (a call that emits at most one row belongs before one that emits
    /// thousands), and to admit the call against a row ceiling before it runs. It is
    /// held to the same honesty contract as
    /// [`purrdf_core::DatasetView::cardinality_estimate`]:
    /// it must be an upper bound the relation actually respects, not a guess, because
    /// a bound that under-states reality turns an admission decision into a wrong one.
    /// A genuinely unbounded generator declares [`u64::MAX`].
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64;

    /// Begin one invocation, returning its row cursor.
    ///
    /// `ceiling` is the optimization licence described in the trait docs — the number
    /// of further emitted-and-agreeing rows that can still reach the query's answer, or
    /// `None` for "no bound, or none the engine can offer here". `args` carries the
    /// bound/free shape of this call.
    ///
    /// # Errors
    ///
    /// Any [`EvalError`] the relation raises — including its refusal of an argument
    /// value it cannot use. Per the hard-fail doctrine a refused invocation aborts the
    /// query rather than contributing zero rows, which would be indistinguishable from
    /// an honest empty answer.
    fn open(&self, args: &PfArgs<'_>, ceiling: Option<u64>)
    -> Result<Box<dyn PfCursor>, EvalError>;

    /// Whether this relation can serve an invocation whose access pattern is
    /// `invocation`: some declared [`mode`](Self::modes) subsumes it.
    ///
    /// Provided rather than required — the rule is the lattice's, not the relation's,
    /// and a relation that could restate it could also restate it wrongly.
    fn admits(&self, invocation: BindingPattern) -> bool {
        self.modes().iter().any(|mode| mode.subsumes(invocation))
    }
}

// ---------------------------------------------------------------------------
// Panic containment
// ---------------------------------------------------------------------------

/// Arity-check `args` against `relation`'s declaration, then open the invocation with
/// the host call contained.
///
/// This is the ONLY way the evaluator enters a relation's `open`, and the supported way
/// for any other caller to do so. It is where the two guarantees that must hold before
/// host code runs are established:
///
/// 1. **Fail-fast arity.** A call whose argument vectors do not match the declared
///    [`PfArity`] never reaches the relation.
/// 2. **Panic containment.** A panicking relation becomes a clean
///    [`EvalError::Function`] rather than aborting a rayon worker. The message is
///    fixed and payload-free, so it is identical no matter which worker panicked —
///    the same treatment [`crate::user_fn`]'s native-function entry gives a native
///    closure, for the same determinism reason.
///
/// # Errors
///
/// [`EvalError::Function`] on an arity mismatch or a caught panic; otherwise the
/// relation's own error, propagated unchanged.
pub fn open_contained(
    relation: &dyn PropertyFunction,
    iri: &str,
    args: &PfArgs<'_>,
    ceiling: Option<u64>,
) -> Result<Box<dyn PfCursor>, EvalError> {
    let declared = declaration_contained(iri, "arity", || relation.arity())?;
    let supplied = args.arity();
    if declared != supplied {
        return Err(EvalError::function(format!(
            "property function <{iri}> expects {declared} argument(s), got {supplied}"
        )));
    }
    match catch_unwind(AssertUnwindSafe(|| relation.open(args, ceiling))) {
        Ok(opened) => opened,
        Err(_) => Err(EvalError::function(format!(
            "property function <{iri}> panicked while opening an invocation"
        ))),
    }
}

/// Pull one row from `cursor` with the host call contained.
///
/// The [`open_contained`] twin, for the other half of the invocation: a relation can
/// panic on any `next`, not merely the first, so every pull crosses this boundary.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise the cursor's own error,
/// propagated unchanged.
pub fn next_contained(cursor: &mut dyn PfCursor, iri: &str) -> Result<Option<PfRow>, EvalError> {
    match catch_unwind(AssertUnwindSafe(|| cursor.next())) {
        Ok(row) => row,
        Err(_) => Err(EvalError::function(format!(
            "property function <{iri}> panicked while producing a row"
        ))),
    }
}

/// Take `cursor`'s reported work with the host call contained.
///
/// The third member of the [`open_contained`]/[`next_contained`] family, for the third
/// thing a cursor can be asked. [`PfCursor::take_work`] is host code exactly as `next`
/// is — a counter that overflows an index, an assertion left in by mistake — and its
/// answer is *spent* against the caller's fuel, so it crosses the same boundary. A panic
/// becomes a clean, payload-free [`EvalError::Function`] rather than aborting a worker.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of the reported count.
pub fn take_work_contained(cursor: &mut dyn PfCursor, iri: &str) -> Result<u64, EvalError> {
    match catch_unwind(AssertUnwindSafe(|| cursor.take_work())) {
        Ok(units) => Ok(units),
        Err(_) => Err(EvalError::function(format!(
            "property function <{iri}> panicked while reporting its work"
        ))),
    }
}

/// Read `cursor`'s [`PfCursor::generation`] with the host call contained.
///
/// The fourth member of the [`open_contained`]/[`next_contained`]/[`take_work_contained`]
/// family, for the fourth thing a cursor can be asked. A generation read is host code
/// exactly as `next` is — a lazily-formatted version string that indexes a slice out of
/// bounds, an assertion left in by mistake — and it runs on whichever worker drove the
/// invocation, so it crosses the same boundary. The message is fixed and payload-free
/// for the determinism reason the shared containment helper states: a query's reported
/// text must not depend on which thread panicked.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of the declared generation.
pub fn generation_contained(
    cursor: &dyn PfCursor,
    iri: &str,
) -> Result<IndexGeneration, EvalError> {
    declaration_contained(iri, "index generation", || cursor.generation())
}

/// Read `cursor`'s [`PfCursor::service_level`] with the host call contained.
///
/// The [`generation_contained`] twin, for the other attested fact and the other instant
/// it is read at (the end of the invocation rather than its start — see
/// [`PfCursor::service_level`]). Same containment, same fixed payload-free message
/// shape, for the same reason.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of the declared service
/// level.
pub fn service_level_contained(
    cursor: &dyn PfCursor,
    iri: &str,
) -> Result<ServiceLevel, EvalError> {
    declaration_contained(iri, "service level", || cursor.service_level())
}

/// Read one of a relation's DECLARATIONS — `arity`, `modes`, `volatility`, or
/// `rows_per_invocation` — with the panic contained.
///
/// The [`open_contained`]/[`next_contained`] twin for the other host calls a relation
/// receives. `arity`, `modes`, `volatility` and `rows_per_invocation` are exactly as much
/// host Rust as `open`/`next` — a lazily-built mode table indexed out of bounds, an
/// assertion a relation's author left in by mistake — and every one of them is reachable
/// from a caller-supplied `dyn PropertyFunction` exactly as `open`/`next` are. Routing
/// every declaration read through this is what makes this module's trust-boundary claim
/// (every invariant is checked, contained, or applied) true of the declaration surface
/// too, not merely of `open`/`next`: `what` names the declaration being read, so every
/// caller's message says which one panicked without leaking the panic's own payload.
///
/// # Errors
///
/// [`EvalError::Function`] on a caught panic; otherwise `Ok` of `read`'s result.
pub fn declaration_contained<T>(
    iri: &str,
    what: &str,
    read: impl FnOnce() -> T,
) -> Result<T, EvalError> {
    // Delegates to the shared containment helper every extension seam uses
    // (`crate::contain`) — see that module's docs for why the panic payload is
    // never interpolated. Kept as a thin `kind = "property function"` wrapper
    // here rather than inlined at call sites, so this function's public
    // signature and its message shape (`property function <iri> panicked while
    // reporting its {what}`) stay exactly what every existing caller and test
    // already depends on.
    crate::contain::declaration_contained("property function", iri, what, read)
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// One declared mode of a relation, as reported by [`PropertyFunctionRegistry::describe`]:
/// the mode's lattice [`code`](BindingPattern::code) paired with the row bound the
/// relation declares for it.
///
/// Paired rather than carried as two parallel vectors, so a reader cannot mis-align a
/// cardinality with a mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfMode {
    /// The mode's per-position code (`'b'` bound, `'f'` free, position 0 first).
    pub code: String,
    /// The relation's declared upper bound on rows per invocation under this mode.
    pub rows_per_invocation: u64,
}

/// A registered relation's self-description — the channel through which a host, a
/// diagnostic, or an explain surface can read what a registry actually contains
/// without holding a `dyn` reference to each relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PfDescriptor {
    /// The IRI the relation is registered under, byte-exact.
    pub iri: String,
    /// The declared number of subject-side arguments.
    pub subject_arity: usize,
    /// The declared number of object-side arguments.
    pub object_arity: usize,
    /// The declared determinism class.
    pub volatility: Volatility,
    /// The declared access patterns, in the order the relation returns them, each
    /// with its declared row bound.
    pub modes: Vec<PfMode>,
    /// The ranked declaration the host supplied for this IRI at registration
    /// ([`PropertyFunctionRegistry::register_ranked`]), or `None` when the
    /// relation was registered without one — which is what "does not
    /// participate in ranked retrieval" looks like when the declaration lives
    /// beside the registry rather than on the relation trait.
    pub ranked: Option<RankedDeclaration>,
}

/// A caller-injected table of property functions, keyed by predicate IRI.
///
/// Built once per host configuration and borrowed into evaluation via
/// [`EvalCtx::with_property_functions`](crate::eval::EvalCtx::with_property_functions)
/// or [`NativeSparqlEngine::query_with_options_view`](crate::NativeSparqlEngine::query_with_options_view)
/// (via `QueryOptions::property_functions`).
/// Deterministic by construction: the map is the crate's fixed-key
/// deterministic fixed-key hash map, and every ordered surface
/// ([`describe`](Self::describe), [`Debug`]) sorts by IRI rather than reading
/// iteration order.
///
/// # Registration is fail-fast on a duplicate — deliberately stricter than
/// [`UserFunctionRegistry`](crate::user_fn::UserFunctionRegistry)
///
/// [`register`](Self::register) **panics** when an IRI is already registered, where
/// the user-function registry lets a second registration of the same kind win. The
/// asymmetry is not an oversight. A shadowed *function* returns a different value
/// under an IRI the host chose twice; a shadowed *relation* silently changes which
/// rows a graph pattern produces — it is a wrong-answer channel that no query text
/// can reveal, because both spellings of the call are identical. Two relations under
/// one IRI is a host misconfiguration, and it is caught where it is committed.
///
/// # Instance identity, not just declared contents
///
/// `id` is a `RegistryId` (`crate::registry_id::RegistryId`) minted fresh by
/// `Default`/[`new`](Self::new) — see that type's docs for why a counter, why a
/// counter is enough, and why [`Clone`] inherits rather than re-mints it. It
/// exists because DECLARED metadata (arity, volatility, modes and their row
/// bounds) cannot distinguish two independently built registries that happen to
/// register the SAME IRI to two DIFFERENT [`PropertyFunction`] implementations
/// with identical declarations — `crate::property_fn_plan::registry_fingerprint`
/// folds this id in ahead of the declaration digest so those two registries can
/// never be mistaken for each other by a prepared plan's identity.
#[derive(Default, Clone)]
pub struct PropertyFunctionRegistry {
    id: crate::registry_id::RegistryId,
    relations: DetHashMap<String, Arc<dyn PropertyFunction>>,
    /// The ranked declarations supplied at registration, keyed by the same IRI
    /// as `relations`. A side table rather than a field of the relation,
    /// because a producer's participation in ranked retrieval is host wiring,
    /// not a property of the relation's Rust type.
    ///
    /// Never iterated: every ordered surface reads it **by key** while
    /// iterating the already-sorted `relations`, so a fixed-key hash map's
    /// iteration order can never reach an output.
    ranked: DetHashMap<String, RankedDeclaration>,
}

impl core::fmt::Debug for PropertyFunctionRegistry {
    /// A `dyn PropertyFunction` has no `Debug` impl, so this lists the registered
    /// IRIs, sorted for deterministic output, rather than deriving. The instance
    /// id rides along too, since it is exactly the thing that can make two
    /// otherwise-identical-looking registries diagnostically distinguishable.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iris: Vec<&str> = self.relations.keys().map(String::as_str).collect();
        iris.sort_unstable();
        let mut ranked: Vec<&str> = self.ranked.keys().map(String::as_str).collect();
        ranked.sort_unstable();
        f.debug_struct("PropertyFunctionRegistry")
            .field("id", &self.id)
            .field("relations", &iris)
            .field("ranked", &ranked)
            .finish()
    }
}

impl PropertyFunctionRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The canonical empty registry — the non-optional "no relations
    /// registered" value every registry-carrying seam
    /// ([`crate::engine::QueryOptions::property_functions`],
    /// [`crate::eval::EvalCtx::property_functions`](crate::eval::EvalCtx),
    /// `crate::parallel::SafetyRegistries::relations`) now uses in place of the
    /// old `Option::None` spelling. See
    /// [`crate::agg_fn::AggregateRegistry::EMPTY`]'s docs for why a single shared
    /// `'static` value, with a fixed reserved
    /// `RegistryId` (`crate::registry_id::RegistryId`), is the correct (not merely
    /// convenient) choice here: an empty registry resolves no IRI regardless of
    /// which `EMPTY` value is asked, so no plan's admitted behavior can depend on
    /// which one it was prepared against.
    pub const EMPTY: Self = Self {
        id: crate::registry_id::RegistryId::EMPTY,
        relations: DetHashMap::with_hasher(crate::DetHasher::new()),
        ranked: DetHashMap::with_hasher(crate::DetHasher::new()),
    };

    /// Register `relation` under `iri`.
    ///
    /// # Panics
    ///
    /// Panics if `iri` is already registered — see the type's docs for why a relation
    /// may not be silently shadowed.
    pub fn register(&mut self, iri: impl Into<String>, relation: Arc<dyn PropertyFunction>) {
        self.insert(iri.into(), relation, None);
    }

    /// Register `relation` under `iri` as a ranked producer described by `decl`.
    ///
    /// The declaration is stored beside the relation, keyed by the same IRI, and
    /// read back with [`Self::ranked_declaration`]. Registering with this method
    /// and registering with [`Self::register`] are the same registration in every
    /// other respect: the relation resolves, describes and evaluates identically.
    ///
    /// # Panics
    ///
    /// Panics, leaving the registry untouched, if:
    ///
    /// * `iri` is already registered — the same refusal, and the same message,
    ///   [`Self::register`] raises.
    /// * `decl.candidate_position` is not a position `relation` declares.
    /// * any placement of any accepted term binds outside `relation`'s positions.
    /// * `decl.depth_placement` binds outside `relation`'s positions.
    /// * `decl.depth_placement` and a term placement target the same position —
    ///   one position renders one value.
    /// * `decl.candidate_position` is also a placement or depth target: a
    ///   position filled with a constant cannot also be the projected candidate.
    /// * `decl.domains` is an empty [`CandidateDomains::Within`] — a promise to
    ///   name nothing is not a narrow domain, it is a producer that should not
    ///   be registered; a host that does not want to restrict its candidates
    ///   declares [`CandidateDomains::Unrestricted`].
    /// * `decl.stratum` is already claimed by another registered producer — one
    ///   stratum carries one producer, because a rank means something only inside
    ///   the list that assigned it. The panic message carries the whole argument:
    ///   the two exits a host splits such a configuration through, and the third
    ///   configuration that shares a scoring law and still belongs at the second
    ///   exit, because a differential weight over a bounded score can act only in
    ///   rank space.
    ///
    /// A declaration that cannot be rendered is host misconfiguration, and like a
    /// duplicate registration it is caught where it is committed rather than at
    /// the first query that reaches it.
    pub fn register_ranked(
        &mut self,
        iri: impl Into<String>,
        relation: Arc<dyn PropertyFunction>,
        decl: RankedDeclaration,
    ) {
        self.insert(iri.into(), relation, Some(decl));
    }

    /// The one path both registration methods take, so the duplicate-IRI refusal
    /// and the order the two tables are written in cannot drift apart.
    ///
    /// `relations` is written LAST: a declaration that fails validation panics
    /// before anything is inserted, leaving the registry exactly as it was.
    fn insert(
        &mut self,
        iri: String,
        relation: Arc<dyn PropertyFunction>,
        decl: Option<RankedDeclaration>,
    ) {
        assert!(
            !self.relations.contains_key(&iri),
            "IRI <{iri}> is already registered as a property function; a relation may not be \
             silently shadowed, because both spellings of the call are identical and the only \
             observable difference is which rows the query returns"
        );
        if let Some(decl) = decl {
            validate_declaration(&iri, &decl, relation.arity());
            assert_stratum_unclaimed(&iri, &decl, &self.ranked);
            self.ranked.insert(iri.clone(), decl);
        }
        self.relations.insert(iri, relation);
    }

    /// The ranked declaration supplied for `iri` at registration, if any.
    ///
    /// `None` is the whole of "this relation does not participate in ranked
    /// retrieval": a relation registered with [`Self::register`] declares
    /// nothing, and nothing is what a consumer reads back.
    #[must_use]
    pub fn ranked_declaration(&self, iri: &str) -> Option<&RankedDeclaration> {
        self.ranked.get(iri)
    }

    /// Resolve a predicate IRI to its registered relation, if any.
    #[must_use]
    pub fn resolve(&self, iri: &str) -> Option<&Arc<dyn PropertyFunction>> {
        self.relations.get(iri)
    }

    /// Whether the registry holds no relations — the common case, in which evaluation
    /// carries no registry at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }

    /// The number of registered relations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.relations.len()
    }

    /// This registry's instance identity — read by
    /// `crate::property_fn_plan::registry_fingerprint`, which lives in a sibling
    /// module and so cannot reach the private `id` field directly. See the type's
    /// docs' "Instance identity" section for why this exists and why `Clone`
    /// inherits rather than re-mints it.
    ///
    /// Public because the distinction between instance identity and declared
    /// contents is exactly what a composition layer over the registry seam needs:
    /// two registries that describe themselves identically (same IRIs, arities,
    /// volatilities, modes, row bounds) but were built independently MUST remain
    /// distinguishable, and [`Self::content_fingerprint`] deliberately cannot tell
    /// them apart. A host that needs "are these the same registry instance?"
    /// compares [`RegistryId`](crate::registry_id::RegistryId) values; a host that
    /// needs "do these registries declare the same shape?" compares
    /// [`Self::content_fingerprint`] values.
    pub const fn instance_id(&self) -> crate::registry_id::RegistryId {
        self.id
    }

    /// A durable, instance-independent fingerprint of this registry's *declared*
    /// contents: every registered IRI's subject/object arity, its declared
    /// volatility, each declared mode with its row bound, and its ranked-retrieval
    /// declaration, IRI-sorted — rendered as the lowercase hex of a SHA-256 digest.
    ///
    /// # Durable and cross-process, unlike [`Self::instance_id`]
    ///
    /// The instance id is ephemeral — a per-process counter, meaningful only while
    /// the process that minted it is alive (see
    /// [`RegistryId`](crate::registry_id::RegistryId)'s docs). This method
    /// deliberately **excludes** it, so two independently constructed registries
    /// that declare the same relations under the same IRIs produce the *identical*
    /// fingerprint, here or in another process. That is what makes it usable as a
    /// durable identity — to compare a registry a caller built against one decoded
    /// from a persisted configuration, or to address a registry's declared shape in
    /// a cache key that outlives the process.
    ///
    /// # What it does and does not capture
    ///
    /// This is a pure function of [`Self::describe`]'s output, which is already
    /// IRI-sorted, so it does not depend on registration order. It captures the
    /// declarations a plan's rewrite and an execution's parallel-safety depend on,
    /// plus the ranked-retrieval declaration that decides whether a request may
    /// draw fused candidates from an IRI at all. It does **not** capture which
    /// trait-object implementation answers an IRI — two registries registering the
    /// same IRI to different implementations with identical declarations share a
    /// content fingerprint by design, and only
    /// `property_fn_plan::registry_fingerprint` (which folds the instance id in
    /// ahead of this content digest, and is what the plan cache and governed
    /// receipts use) can tell them apart.
    ///
    /// # Compare it, do not parse it
    ///
    /// The value is a digest rendering, not a description: it answers "do these two
    /// registries declare the same shape?" by equality and nothing else. Nothing in
    /// it can be read back, and a caller must not try — the declared contents are
    /// available in structured form from [`Self::describe`], which is what this
    /// digests.
    ///
    /// # The empty registry
    ///
    /// A registry with no relations is not special-cased: it digests the
    /// property-function domain separator alone, so the canonical [`Self::EMPTY`]
    /// and a freshly built [`Self::new`] registry fingerprint identically (they are
    /// observationally interchangeable; see
    /// [`RegistryId`](crate::registry_id::RegistryId)). The value is a fixed
    /// constant rather than an empty string, which is what lets a consumer
    /// distinguish "this registry declares nothing" from "no fingerprint was
    /// recorded".
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if any registered relation's declaration methods
    /// panic — [`Self::describe`]'s own failure, propagated unchanged. Never
    /// raised for an empty registry, which has no declaration to read.
    pub fn content_fingerprint(&self) -> Result<String, EvalError> {
        Ok(crate::property_fn_plan::content_fingerprint(self)?.to_hex())
    }

    /// Describe every registered relation, sorted by IRI.
    ///
    /// The sort makes the output a pure function of the registry's contents rather
    /// than of its construction order, so two hosts that register the same relations
    /// in different orders describe identically.
    ///
    /// Every declaration read (`arity`, `volatility`, `modes`, `rows_per_invocation`)
    /// goes through [`declaration_contained`], because this runs on every prepare (it
    /// feeds the plan cache's fingerprint) — a relation whose declaration methods panic
    /// must fail that prepare cleanly, not abort it.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] if any registered relation's declaration methods panic.
    pub fn describe(&self) -> Result<Vec<PfDescriptor>, EvalError> {
        let mut out: Vec<PfDescriptor> = Vec::with_capacity(self.relations.len());
        for (iri, relation) in &self.relations {
            let arity = declaration_contained(iri, "arity", || relation.arity())?;
            let volatility =
                declaration_contained(iri, "determinism class", || relation.volatility())?;
            let modes = declaration_contained(iri, "declared modes", || relation.modes().to_vec())?;
            let mut described_modes = Vec::with_capacity(modes.len());
            for mode in modes {
                let rows_per_invocation =
                    declaration_contained(iri, "row bound", || relation.rows_per_invocation(mode))?;
                described_modes.push(PfMode {
                    code: mode.code(),
                    rows_per_invocation,
                });
            }
            out.push(PfDescriptor {
                iri: iri.clone(),
                subject_arity: arity.subject,
                object_arity: arity.object,
                volatility,
                modes: described_modes,
                // Read BY KEY out of the side table while iterating `relations`,
                // never by iterating `ranked`: the fixed-key map's order stays
                // unobservable, and the output is sorted by IRI below.
                ranked: self.ranked.get(iri).cloned(),
            });
        }
        out.sort_by(|a, b| a.iri.cmp(&b.iri));
        Ok(out)
    }
}

/// Refuse a stratum some already-registered producer serves: one stratum carries
/// one producer.
///
/// # Why a second producer under one stratum has no meaning to fall back on
///
/// A rank is meaningful only inside the list that assigned it. Merging two ranked
/// lists needs either a comparable score — which a rank is not, and which no
/// projection back out of a rank recovers — or a fusion rule, and the fusion rule is
/// precisely what a consumer of these declarations *is*. Two producers under one
/// stratum have neither, so their rows can only be concatenated, and concatenation
/// is wrong in three visible ways at once: the second producer's best row is ranked
/// below every row of the first and decays as though it lost to rows it never
/// competed with; a candidate both produce arrives twice in one list, which an
/// honest `Unique` declaration says cannot happen; and a first producer that fills
/// the stratum's depth leaves the second contributing nothing at all.
///
/// # Every configuration splits cleanly, which is why this refuses
///
/// Co-stratum ranks are well defined iff the two producers' ranks are comparable;
/// ranks are comparable iff the producers share a scoring law; and producers sharing
/// a scoring law can merge internally, by their own scores, below this seam. So the
/// concatenation case is the empty set between two exits, and the message names
/// **both** of them:
///
/// * **same scoring law, and no weighting the score cannot itself carry** —
///   shards, per-language segments, a partitioned index — merge inside ONE
///   producer, which owns the comparability its own scores already have;
/// * **different scoring laws** — separate strata, where the fusion sum across them
///   is the design rather than an accident.
///
/// Naming only the second exit would be a defect rather than a shorter message.
/// A summing reciprocal-rank fusion treats each stratum as a summand, so a candidate
/// surfacing in two shards-recast-as-strata collects two contributions where the
/// host meant one family's worth — a quiet score distortion recommended by the
/// refusal itself.
///
/// # The first exit takes two things, not one
///
/// "Do they share a scoring law" is the wrong test on its own, because it routes a
/// third configuration to the wrong exit. The first exit requires **both** that the
/// two producers' scores are comparable **and** that whatever weighting the host
/// intends between them can ride *inside* the score. Two producers over one
/// embedding space — a heading class and a body class, say — satisfy the first and
/// can fail the second: same formula, same space, directly comparable scores, but if
/// the host means one class to **outweigh** the other and the shared score is a
/// *bounded* metric (a cosine distance in `[0, 2]`, lower-better), the merge cannot
/// carry that weight.
///
/// Merging and sorting by `d / w`, a heading beats a body hit iff
/// `d_h / w_h < d_c / w_c`, which rearranges to a required similarity edge of
/// `(1 - s_c)(1 - w_h / w_c)`. At `w_h / w_c = 0.5` that edge is `0.45` against a
/// body similarity of `0.1`, `0.05` against `0.9`, `0.0005` against `0.999`, and
/// exactly zero against a perfect match. It **decays to zero as matches approach
/// perfect** — which is where the top-k contest is decided — and is largest for the
/// worst matches, so weighting a bounded score penalises bad matches hardest, which
/// is backwards. An unbounded score is untouched by this: BM25F's field weights are
/// natively a multiplicative weight the score carries at every magnitude, so that
/// case really does belong at the first exit.
///
/// Nor is the margin the whole of it. For a weighted threshold `s'`, the expected
/// number of competitors outranking a hit is `n · (1 - F(s'))` — **linear in corpus
/// size** — so no fixed weight ratio survives corpus growth even away from the
/// boundary. The conclusion is structural rather than a tuning problem.
///
/// The failure is silent if the message does not say so: a host takes the first exit
/// in good faith, loses its class weighting, and an unweighted merge still returns a
/// plausible ranking. So the message asks the question a host can answer while
/// reading it — *do you want these two weighted differently, and is the score
/// bounded?* — and routes that answer to **separate strata** despite the shared law,
/// where the weight acts in rank space, which is what a stratum is.
///
/// # Why scanning the side table keeps its iteration order unobservable
///
/// `ranked` is a fixed-key hash map whose iteration order is deliberately not an
/// output anywhere, and this is the one place it is walked. It stays unobservable,
/// because the invariant this function establishes holds by induction: at most ONE
/// registered declaration can name any given stratum, so a search with at most one
/// match yields the same match in every order. Nothing is written before the scan,
/// so a refused registration leaves the registry exactly as it was — the discipline
/// [`PropertyFunctionRegistry::insert`] applies to the duplicate-IRI refusal.
///
/// # Panics
///
/// Panics if another registered producer already declares `decl.stratum`.
fn assert_stratum_unclaimed(
    iri: &str,
    decl: &RankedDeclaration,
    ranked: &DetHashMap<String, RankedDeclaration>,
) {
    let claimed = ranked
        .iter()
        .find(|(_, declared)| declared.stratum == decl.stratum);
    let Some((holder, _)) = claimed else {
        return;
    };
    panic!(
        "ranked declaration for <{iri}> claims stratum <{}>, which the registered producer \
         <{holder}> already serves; one stratum carries one producer, because a rank is \
         meaningful only inside the list that assigned it and two lists concatenated rank the \
         second producer's best row below every row of the first. Merging takes TWO things, \
         not one: scores that are already comparable, AND a weighting that can ride inside the \
         score. Shards, per-language segments, a partitioned index with no weight standing \
         between them have both — merge them inside ONE producer, which owns that \
         comparability. If they score by different laws, give each its own stratum, where the \
         fusion sum across strata is the design rather than an accident. Ask the second \
         question even when the law is shared: do you want these two weighted differently, and \
         is the score bounded (a cosine distance in [0,2], say)? Then separate strata as well \
         — weighting a bounded score buys an edge of only (1 - s)(1 - w_low/w_high), largest \
         for the worst matches and zero for a perfect one, which is exactly where the top-k \
         contest is decided, so such a weight can act only in rank space, and rank space is \
         what a stratum is. Only an unbounded score — BM25F-style field weights — carries a \
         differential weight through a merge",
        decl.stratum
    );
}

/// Check that `decl` can actually be rendered against a relation of `arity`.
///
/// Every failure here is host misconfiguration committed at registration, so it
/// panics and names the offending value rather than deferring to the first query
/// that tries to render the declaration and finds it cannot.
///
/// # Panics
///
/// See [`PropertyFunctionRegistry::register_ranked`] for the full list.
fn validate_declaration(iri: &str, decl: &RankedDeclaration, arity: PfArity) {
    let total = arity.total();
    // An empty restriction is not a narrow producer, it is a producer that
    // promised to name nothing. Read literally by a consumer it is a stream
    // whose every row contradicts its own declaration, so the first row it
    // emitted would be refused and the registration would have bought a
    // configuration that cannot answer. Refused where it is committed, like
    // every other declaration a relation cannot honour — and distinctly from
    // `CandidateDomains::Unrestricted`, which is one line away and is what a
    // host that does not want to restrict its candidates actually means.
    if let CandidateDomains::Within(tags) = &decl.domains {
        assert!(
            !tags.is_empty(),
            "ranked declaration for <{iri}> restricts its candidates to an empty set of \
             domains, which promises that this producer names nothing at all; a consumer \
             holds a producer to this declaration row by row, so every row it emitted would \
             contradict it. Name the domains this producer really draws from, or declare \
             CandidateDomains::Unrestricted, which is the honest statement that it may name \
             anything"
        );
    }
    // A declaration of loss that says nothing is the mirror of the empty domain
    // set above: it spends the consumer's trust without buying it anything. The
    // consumer is told a stratum may be short and told nothing it can act on or
    // render, and a blank string is indistinguishable in a rendered answer from
    // a producer that declared no loss at all — so the one field whose entire
    // purpose is to be read verbatim would arrive empty. Refused where it is
    // committed, and distinctly from `Completeness::Complete`, which is one line
    // away and is what a producer with nothing to disclose actually means.
    for (axis, evidence) in [
        (
            "completeness",
            match &decl.fidelity.completeness {
                Completeness::Complete => None,
                Completeness::Lossy { evidence } => Some(evidence),
            },
        ),
        (
            "order",
            match &decl.fidelity.order {
                OrderFidelity::Faithful => None,
                OrderFidelity::Perturbed { evidence } => Some(evidence),
            },
        ),
    ] {
        if let Some(evidence) = evidence {
            assert!(
                !evidence.trim().is_empty(),
                "ranked declaration for <{iri}> declares a loss on its {axis} axis but supplies \
                 no evidence for it; a consumer carries this string into its answer verbatim, so \
                 an empty one reports a degraded stratum while saying nothing a reader can act \
                 on. State what the producer does not promise — the measurement, its limit, and \
                 what an empty result does not prove — or declare the exhaustive variant, which \
                 is the honest statement that there is nothing to disclose"
            );
        }
    }
    assert!(
        decl.candidate_position < total,
        "ranked declaration for <{iri}> projects its candidate from position {} but the relation \
         declares only {total} argument position(s) ({arity})",
        decl.candidate_position
    );
    for placement in decl.placements() {
        assert!(
            placement.position < total,
            "ranked declaration for <{iri}> binds the {} facet of an accepted term at position {} \
             but the relation declares only {total} argument position(s) ({arity})",
            placement.facet.as_str(),
            placement.position
        );
        assert!(
            placement.position != decl.candidate_position,
            "ranked declaration for <{iri}> binds the {} facet of an accepted term at position {} \
             and also projects its candidate from there; a position filled with a request value \
             cannot also be the projected candidate",
            placement.facet.as_str(),
            placement.position
        );
    }
    if let Some(depth) = decl.depth_placement.as_ref() {
        assert!(
            depth.position < total,
            "ranked declaration for <{iri}> binds its per-stratum depth at position {} but the \
             relation declares only {total} argument position(s) ({arity})",
            depth.position
        );
        assert!(
            depth.position != decl.candidate_position,
            "ranked declaration for <{iri}> binds its per-stratum depth at position {} and also \
             projects its candidate from there; a position filled with a request value cannot \
             also be the projected candidate",
            depth.position
        );
        for placement in decl.placements() {
            assert!(
                placement.position != depth.position,
                "ranked declaration for <{iri}> binds both its per-stratum depth and the {} facet \
                 of an accepted term at position {}; one argument position renders one value",
                placement.facet.as_str(),
                depth.position
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The in-memory reference relation
// ---------------------------------------------------------------------------

/// A deterministic in-memory relation: a fixed table of rows, scanned linearly.
///
/// This is the reference implementation of [`PropertyFunction`] — the shape every
/// other relation is measured against — and it is directly useful: a host with a
/// small extensional table (a code list, a unit-conversion table, a fixture in a
/// test) can expose it to SPARQL without writing an evaluator.
///
/// * **Every mode is served.** It declares exactly one mode, the all-free
///   [`PfArity::all_free_mode`], which subsumes every access pattern of its arity —
///   so any invocation is feasible. Bound positions are applied as per-position term
///   equality during the scan; the engine's own equality filter on bound positions
///   then has nothing left to remove.
/// * **Order is insertion order.** Rows are emitted in the order they were supplied,
///   filtered but never reordered.
/// * **The row ceiling is honoured.** Given a ceiling, the cursor stops after emitting
///   that many rows — counting the rows it emits, not the rows it skips, which is the
///   accounting the licence requires (see [`PropertyFunction`]'s ceiling contract).
///   Because the emitted rows are the first ones the unbounded scan would have produced,
///   the answer is the same one, computed over less of the table.
/// * **The row bound is exact.** [`PropertyFunction::rows_per_invocation`] reports the
///   table's row count, which no invocation can exceed.
/// * **It is [`Volatility::Stable`]**: a frozen table is deterministic for the
///   lifetime of a query, so it may run across fork-join workers.
///
/// ```
/// use std::sync::Arc;
///
/// use purrdf_core::TermValue;
/// use purrdf_sparql_eval::{MemoryRelation, PfArgs, PropertyFunction};
///
/// let iri = |s: &str| TermValue::iri(s);
/// // A two-column relation: one subject-side argument, one object-side argument.
/// let relation = MemoryRelation::new(
///     1,
///     1,
///     vec![
///         vec![iri("http://example.org/a"), iri("http://example.org/1")],
///         vec![iri("http://example.org/b"), iri("http://example.org/2")],
///     ],
/// )
/// .expect("every row is two columns wide");
///
/// // Invoke it with the subject bound and the object free: `bf`.
/// let subject_value = iri("http://example.org/b");
/// let subject = [Some(&subject_value)];
/// let object = [None];
/// let args = PfArgs::new(&subject, &object);
/// assert_eq!(args.mode().code(), "bf");
/// assert!(relation.admits(args.mode()));
///
/// let mut cursor = relation.open(&args, None).expect("open");
/// let row = cursor.next().expect("no error").expect("one matching row");
/// assert_eq!(row, vec![iri("http://example.org/b"), iri("http://example.org/2")]);
/// assert!(cursor.next().expect("no error").is_none());
/// ```
#[derive(Debug, Clone)]
pub struct MemoryRelation {
    arity: PfArity,
    rows: Arc<Vec<PfRow>>,
    /// The single declared mode (all-free), materialized once so
    /// [`PropertyFunction::modes`] can hand out a slice.
    modes: [BindingPattern; 1],
}

impl MemoryRelation {
    /// Build a relation over `rows`, split into `subject_arity` subject-side and
    /// `object_arity` object-side positions.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] if any row's width differs from
    /// `subject_arity + object_arity`. A ragged table is a host configuration error,
    /// caught where it is supplied rather than surfacing later as a row the engine
    /// cannot bind.
    pub fn new(
        subject_arity: usize,
        object_arity: usize,
        rows: Vec<PfRow>,
    ) -> Result<Self, EvalError> {
        let arity = PfArity::new(subject_arity, object_arity);
        let width = arity.total();
        for (index, row) in rows.iter().enumerate() {
            if row.len() != width {
                return Err(EvalError::config(format!(
                    "in-memory property-function row {index} has {} value(s); the declared \
                     arity ({arity}) requires {width}",
                    row.len()
                )));
            }
        }
        Ok(Self {
            arity,
            rows: Arc::new(rows),
            modes: [arity.all_free_mode()],
        })
    }

    /// Build a relation by reading its rows out of `dataset`: an `rdf:List` whose head
    /// is `head` and whose members are themselves `rdf:List`s, one per row, each
    /// holding that row's values in flattened order.
    ///
    /// The nested-list encoding is the one RDF already has for an ordered tuple, so a
    /// host can ship a relation as data — in the same graph as the shapes or the
    /// configuration that names it — instead of as Rust.
    ///
    /// ```text
    /// @prefix ex: <http://example.org/> .
    ///
    /// ex:codes ex:table ( ( ex:a ex:1 ) ( ex:b ex:2 ) ) .
    /// ```
    ///
    /// Reading `ex:table`'s object as `head`, with `subject_arity` 1 and
    /// `object_arity` 1, yields the two-row relation of the [`MemoryRelation`] example.
    /// Row order is list order, so the relation's emission order is the order the data
    /// was written in.
    ///
    /// # Errors
    ///
    /// * [`EvalError::Data`] if `head` or any row head is a torn or malformed
    ///   `rdf:List` (a cell missing its `rdf:first`, a multi-valued `rdf:first`, or an
    ///   `rdf:rest` pointing at a non-cell), or if a row's length differs from
    ///   `subject_arity + object_arity`. Both are bad *input* rather than a caller API
    ///   misuse, so they are distinguished from [`Self::new`]'s
    ///   [`EvalError::Config`].
    /// * [`EvalError::Data`] if `head` is not interned in `dataset` at all: a head
    ///   naming a list that does not exist is a configuration pointing at nothing, not
    ///   an empty relation.
    pub fn from_graph<D: DatasetView>(
        dataset: &D,
        head: &TermValue,
        graph: GraphMatch<D::Id>,
        subject_arity: usize,
        object_arity: usize,
    ) -> Result<Self, EvalError> {
        let arity = PfArity::new(subject_arity, object_arity);
        let width = arity.total();
        let Some(head_id) = dataset.term_id_by_value(head) else {
            return Err(EvalError::data(format!(
                "property-function table head {head:?} is not present in the dataset"
            )));
        };
        let row_heads = dataset.rdf_list(head_id, graph).map_err(|e| {
            EvalError::data(format!("property-function table is not an rdf:List: {e}"))
        })?;

        let mut rows: Vec<PfRow> = Vec::with_capacity(row_heads.len());
        for (index, row_head) in row_heads.into_iter().enumerate() {
            let cells = dataset.rdf_list(row_head, graph).map_err(|e| {
                EvalError::data(format!(
                    "property-function table row {index} is not an rdf:List: {e}"
                ))
            })?;
            if cells.len() != width {
                return Err(EvalError::data(format!(
                    "property-function table row {index} has {} value(s); the declared arity \
                     ({arity}) requires {width}",
                    cells.len()
                )));
            }
            rows.push(
                cells
                    .into_iter()
                    .map(|id| crate::scratch::term_id_to_value(dataset, id))
                    .collect(),
            );
        }

        Ok(Self {
            arity,
            rows: Arc::new(rows),
            modes: [arity.all_free_mode()],
        })
    }

    /// The relation's rows, in emission order.
    #[must_use]
    pub fn rows(&self) -> &[PfRow] {
        &self.rows
    }
}

impl PropertyFunction for MemoryRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        // Exact, and mode-independent: a filtered scan of the table cannot emit more
        // rows than the table holds, whichever positions are bound.
        self.rows.len() as u64
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // `open_contained` already checked this for every engine-driven call; a
        // direct caller (a doctest, a host wiring its own harness) gets the same
        // answer rather than a panic on a short row.
        let supplied = args.arity();
        if supplied != self.arity {
            return Err(EvalError::function(format!(
                "in-memory property function expects {} argument(s), got {supplied}",
                self.arity
            )));
        }
        // The bound positions are cloned into the cursor: a cursor outlives the
        // borrow `args` lends, and a bound argument vector is at most `arity` values
        // wide — the per-invocation clone the row scan then reads.
        let filter: Vec<Option<TermValue>> =
            args.flattened().map(<Option<&TermValue>>::cloned).collect();
        Ok(Box::new(MemoryCursor {
            rows: Arc::clone(&self.rows),
            filter,
            next_index: 0,
            remaining: ceiling,
        }))
    }
}

/// The cursor [`MemoryRelation::open`] returns: a linear scan of the shared row
/// table, emitting the rows that agree with every bound position.
#[derive(Debug)]
struct MemoryCursor {
    rows: Arc<Vec<PfRow>>,
    /// The invocation's bound values by flattened position (`None` = free).
    filter: Vec<Option<TermValue>>,
    next_index: usize,
    /// The rows this invocation may still emit under the engine's licence, or `None`
    /// when it was given no ceiling.
    ///
    /// This is the reference implementation of the licence, and the shape a host
    /// relation copies. Two properties make it sound, and both are load-bearing:
    ///
    /// * It counts **emitted** rows, never scanned ones. The rows this scan skips
    ///   disagree with a bound position and would have been cut by the engine's own
    ///   equality filter anyway, so counting them would spend the licence on rows the
    ///   engine was never going to keep — the miscount the trait docs warn about.
    /// * It stops **producing**, it does not report an error or a short-but-different
    ///   bag: the rows already emitted are the first rows of the full scan, in the
    ///   same order, which is exactly what the licence was granted against.
    remaining: Option<u64>,
}

impl PfCursor for MemoryCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.remaining == Some(0) {
            return Ok(None);
        }
        while let Some(row) = self.rows.get(self.next_index) {
            self.next_index += 1;
            let matches = self
                .filter
                .iter()
                .zip(row.iter())
                .all(|(bound, value)| bound.as_ref().is_none_or(|bound| bound == value));
            if matches {
                if let Some(remaining) = self.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row.clone()));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

    use super::*;

    const EX_A: &str = "http://example.org/a";
    const EX_B: &str = "http://example.org/b";
    const EX_ONE: &str = "http://example.org/1";
    const EX_TWO: &str = "http://example.org/2";
    const EX_SPLIT: &str = "http://example.org/ns#split";
    const EX_OTHER: &str = "http://example.org/ns#other";

    fn iri(s: &str) -> TermValue {
        TermValue::iri(s)
    }

    /// The two-row, one-subject/one-object reference table.
    fn table() -> MemoryRelation {
        MemoryRelation::new(
            1,
            1,
            vec![vec![iri(EX_A), iri(EX_ONE)], vec![iri(EX_B), iri(EX_TWO)]],
        )
        .expect("uniform rows")
    }

    /// Drain a cursor into a vector of rows.
    fn drain(cursor: &mut dyn PfCursor) -> Vec<PfRow> {
        let mut out = Vec::new();
        while let Some(row) = cursor.next().expect("no error") {
            out.push(row);
        }
        out
    }

    /// Open `relation` with the given per-position bindings (flattened, split at
    /// `subject_arity`) and drain it.
    fn invoke(relation: &MemoryRelation, bound: &[Option<TermValue>]) -> Vec<PfRow> {
        let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
        let (subject, object) = refs.split_at(relation.arity().subject);
        let args = PfArgs::new(subject, object);
        let mut cursor = relation.open(&args, None).expect("open");
        drain(&mut *cursor)
    }

    // ---- the positional model --------------------------------------------

    #[test]
    fn flattened_positions_are_subject_then_object() {
        let arity = PfArity::new(2, 1);
        assert_eq!(arity.total(), 3);
        let s0 = iri(EX_A);
        let o0 = iri(EX_ONE);
        let subject = [Some(&s0), None];
        let object = [Some(&o0)];
        let args = PfArgs::new(&subject, &object);
        assert_eq!(args.arity(), arity);
        assert_eq!(args.get(0), Some(&s0));
        assert_eq!(args.get(1), None);
        assert_eq!(args.get(2), Some(&o0));
        assert_eq!(args.get(3), None, "out of range reads as free");
        assert_eq!(args.mode().code(), "bfb");
    }

    // ---- registry --------------------------------------------------------

    #[test]
    fn register_and_resolve() {
        let mut registry = PropertyFunctionRegistry::new();
        assert!(registry.is_empty());
        registry.register(EX_SPLIT, Arc::new(table()));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.resolve(EX_SPLIT).is_some());
        assert!(registry.resolve(EX_OTHER).is_none());
    }

    #[test]
    #[should_panic(expected = "already registered as a property function")]
    fn duplicate_registration_panics() {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(table()));
        registry.register(EX_SPLIT, Arc::new(table()));
    }

    #[test]
    fn describe_is_sorted_by_iri_and_pairs_modes_with_bounds() {
        let mut registry = PropertyFunctionRegistry::new();
        // Registered in reverse IRI order, so the sort is observable.
        registry.register(EX_SPLIT, Arc::new(table()));
        registry.register(EX_OTHER, Arc::new(table()));
        let described = registry.describe().expect("no relation panics");
        assert_eq!(
            described.iter().map(|d| d.iri.as_str()).collect::<Vec<_>>(),
            vec![EX_OTHER, EX_SPLIT],
            "describe sorts by IRI, not by registration order"
        );
        let first = &described[0];
        assert_eq!(first.subject_arity, 1);
        assert_eq!(first.object_arity, 1);
        assert_eq!(first.volatility, Volatility::Stable);
        assert_eq!(
            first.modes,
            vec![PfMode {
                code: "ff".to_owned(),
                rows_per_invocation: 2,
            }]
        );
    }

    #[test]
    fn debug_lists_iris_sorted() {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(table()));
        registry.register(EX_OTHER, Arc::new(table()));
        let rendered = format!("{registry:?}");
        let other_at = rendered.find(EX_OTHER).expect("other listed");
        let split_at = rendered.find(EX_SPLIT).expect("split listed");
        assert!(
            other_at < split_at,
            "Debug output is IRI-sorted: {rendered}"
        );
    }

    // ---- feasibility -----------------------------------------------------

    #[test]
    fn the_all_free_mode_admits_every_invocation() {
        let relation = table();
        for code in ["ff", "bf", "fb", "bb"] {
            assert!(
                relation.admits(BindingPattern::from_code(code)),
                "the all-free declaration must admit {code}"
            );
        }
    }

    #[test]
    fn a_declared_mode_admits_only_its_supersets() {
        // A relation that can only enumerate objects from a bound subject declares
        // `bf`: it serves `bf` and `bb` (generate-then-filter), never `ff` or `fb`.
        let bf = BindingPattern::from_code("bf");
        assert!(bf.subsumes(BindingPattern::from_code("bf")));
        assert!(bf.subsumes(BindingPattern::from_code("bb")));
        assert!(!bf.subsumes(BindingPattern::from_code("ff")));
        assert!(!bf.subsumes(BindingPattern::from_code("fb")));
    }

    // ---- MemoryRelation over every mode ----------------------------------

    #[test]
    fn memory_relation_serves_every_mode() {
        let relation = table();

        // ff — the whole table, in insertion order.
        assert_eq!(
            invoke(&relation, &[None, None]),
            vec![vec![iri(EX_A), iri(EX_ONE)], vec![iri(EX_B), iri(EX_TWO)],]
        );
        // bf — bound subject.
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_B)), None]),
            vec![vec![iri(EX_B), iri(EX_TWO)]]
        );
        // fb — bound object.
        assert_eq!(
            invoke(&relation, &[None, Some(iri(EX_ONE))]),
            vec![vec![iri(EX_A), iri(EX_ONE)]]
        );
        // bb — both bound, agreeing.
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_A)), Some(iri(EX_ONE))]),
            vec![vec![iri(EX_A), iri(EX_ONE)]]
        );
        // bb — both bound, disagreeing: no row.
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_A)), Some(iri(EX_TWO))]),
            [] as [Vec<TermValue>; 0]
        );
    }

    #[test]
    fn memory_relation_emission_order_is_insertion_order() {
        let relation = MemoryRelation::new(
            1,
            1,
            vec![vec![iri(EX_B), iri(EX_TWO)], vec![iri(EX_A), iri(EX_ONE)]],
        )
        .expect("uniform rows");
        assert_eq!(
            invoke(&relation, &[None, None]),
            vec![vec![iri(EX_B), iri(EX_TWO)], vec![iri(EX_A), iri(EX_ONE)],],
            "rows are emitted as supplied, never sorted"
        );
    }

    /// Open `relation` all-free under `ceiling` and drain it.
    fn invoke_under_ceiling(relation: &MemoryRelation, ceiling: Option<u64>) -> Vec<PfRow> {
        let subject = [None];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, ceiling).expect("open");
        drain(&mut *cursor)
    }

    #[test]
    fn memory_relation_stops_at_the_row_ceiling() {
        // The reference implementation of the licence, and the behaviour a host relation
        // copies: a ceiling of `k` yields the FIRST `k` rows of the unbounded scan, in
        // the same order, and then reports exhaustion.
        let rows: Vec<PfRow> = (0..100)
            .map(|i| vec![iri(EX_A), iri(&format!("http://example.org/r{i:03}"))])
            .collect();
        let relation = MemoryRelation::new(1, 1, rows.clone()).expect("uniform rows");

        let full = invoke_under_ceiling(&relation, None);
        assert_eq!(full.len(), 100, "no ceiling scans the whole table");

        let capped = invoke_under_ceiling(&relation, Some(3));
        assert_eq!(
            capped,
            rows[..3].to_vec(),
            "a ceiling of 3 emits the first three rows and nothing else"
        );

        // A ceiling of zero emits nothing at all, without a first pull into the table.
        assert_eq!(
            invoke_under_ceiling(&relation, Some(0)),
            [] as [Vec<TermValue>; 0]
        );
        // A ceiling above the table is simply never reached.
        assert_eq!(invoke_under_ceiling(&relation, Some(1_000)).len(), 100);
    }

    #[test]
    fn memory_relation_counts_emitted_rows_not_scanned_ones() {
        // The accounting that makes the licence sound: rows the scan SKIPS disagree with
        // a bound position and would have been cut by the engine's equality filter
        // anyway, so spending the ceiling on them would hand back fewer usable rows than
        // the engine asked for.
        let relation = MemoryRelation::new(
            1,
            1,
            vec![
                vec![iri(EX_A), iri(EX_ONE)],
                vec![iri(EX_B), iri(EX_TWO)],
                vec![iri(EX_B), iri(EX_ONE)],
            ],
        )
        .expect("uniform rows");

        let subject_value = iri(EX_B);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, Some(1)).expect("open");
        assert_eq!(
            drain(&mut *cursor),
            vec![vec![iri(EX_B), iri(EX_TWO)]],
            "the skipped ex:a row must not have consumed the single-row licence"
        );
    }

    #[test]
    fn memory_relation_row_bound_is_the_row_count() {
        let relation = table();
        for code in ["ff", "bf", "fb", "bb"] {
            assert_eq!(
                relation.rows_per_invocation(BindingPattern::from_code(code)),
                2
            );
        }
    }

    #[test]
    fn ragged_rows_are_a_configuration_error() {
        let error = MemoryRelation::new(1, 1, vec![vec![iri(EX_A)]])
            .expect_err("a one-value row cannot fill two positions");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        assert!(error.to_string().contains("requires 2"), "got {error}");
    }

    #[test]
    fn wrong_argument_count_is_refused_before_the_scan() {
        let relation = table();
        let subject: [Option<&TermValue>; 0] = [];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let Err(error) = relation.open(&args, None) else {
            panic!("a zero-argument subject side does not match the declaration");
        };
        assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
    }

    // ---- from_graph ------------------------------------------------------

    /// `( ( ex:a ex:1 ) ( ex:b ex:2 ) )` reachable from `ex:codes ex:table ?head`,
    /// plus the `rows`-controlled row widths.
    fn table_dataset(rows: &[Vec<&str>]) -> (Arc<RdfDataset>, TermValue) {
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rest = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let nil = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

        // Build each row list, then the outer list of row heads, tail-first so every
        // `rdf:rest` target already exists.
        let mut row_heads = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            let mut cell = nil;
            for (cell_index, value) in row.iter().enumerate().rev() {
                let node = b.intern_blank(
                    &format!("r{row_index}c{cell_index}"),
                    purrdf_core::BlankScope::DEFAULT,
                );
                let value = b.intern_iri(value);
                b.push_quad(node, first, value, None);
                b.push_quad(node, rest, cell, None);
                cell = node;
            }
            row_heads.push(cell);
        }
        let mut outer = nil;
        for (index, row_head) in row_heads.iter().enumerate().rev() {
            let node = b.intern_blank(&format!("t{index}"), purrdf_core::BlankScope::DEFAULT);
            b.push_quad(node, first, *row_head, None);
            b.push_quad(node, rest, outer, None);
            outer = node;
        }
        let head_label = format!("t{}", 0);
        let subject = b.intern_iri("http://example.org/codes");
        let predicate = b.intern_iri("http://example.org/table");
        b.push_quad(subject, predicate, outer, None);
        let dataset = b.freeze().expect("freeze");
        (
            dataset,
            TermValue::Blank {
                label: head_label,
                scope: purrdf_core::BlankScope::DEFAULT,
            },
        )
    }

    #[test]
    fn from_graph_reads_a_list_of_lists_in_order() {
        let (dataset, head) = table_dataset(&[vec![EX_A, EX_ONE], vec![EX_B, EX_TWO]]);
        let relation =
            MemoryRelation::from_graph(&*dataset, &head, GraphMatch::Default, 1, 1).expect("read");
        assert_eq!(
            relation.rows(),
            &[vec![iri(EX_A), iri(EX_ONE)], vec![iri(EX_B), iri(EX_TWO)],]
        );
        assert_eq!(
            invoke(&relation, &[Some(iri(EX_B)), None]),
            vec![vec![iri(EX_B), iri(EX_TWO)]]
        );
    }

    #[test]
    fn from_graph_rejects_a_row_of_the_wrong_width() {
        let (dataset, head) = table_dataset(&[vec![EX_A, EX_ONE], vec![EX_B]]);
        let error = MemoryRelation::from_graph(&*dataset, &head, GraphMatch::Default, 1, 1)
            .expect_err("a one-value row cannot fill two positions");
        assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
        assert!(error.to_string().contains("row 1"), "got {error}");
    }

    #[test]
    fn from_graph_rejects_an_absent_head() {
        let (dataset, _head) = table_dataset(&[vec![EX_A, EX_ONE]]);
        let error = MemoryRelation::from_graph(
            &*dataset,
            &iri("http://example.org/missing"),
            GraphMatch::Default,
            1,
            1,
        )
        .expect_err("a head that is not interned names no list");
        assert!(matches!(error, EvalError::Data(_)), "got {error:?}");
    }

    // ---- panic containment ------------------------------------------------

    /// A relation that panics wherever the constructor says to.
    #[derive(Debug)]
    struct PanickingRelation {
        on_open: bool,
        modes: [BindingPattern; 1],
    }

    impl PanickingRelation {
        fn new(on_open: bool) -> Self {
            Self {
                on_open,
                modes: [PfArity::new(1, 1).all_free_mode()],
            }
        }
    }

    impl PropertyFunction for PanickingRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            1
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            assert!(!self.on_open, "relation exploded in open");
            Ok(Box::new(PanickingCursor))
        }
    }

    struct PanickingCursor;

    impl PfCursor for PanickingCursor {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            panic!("relation exploded in next")
        }

        fn take_work(&mut self) -> u64 {
            panic!("relation exploded in take_work")
        }
    }

    /// Run `body` with the default panic hook suppressed, so an *expected*, caught
    /// panic does not dump to stderr (mirrors `user_fn`'s panic test).
    fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let out = body();
        std::panic::set_hook(default_hook);
        out
    }

    #[test]
    fn a_panic_in_open_is_a_clean_payload_free_error() {
        let relation = PanickingRelation::new(true);
        let subject_value = iri(EX_A);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let error = without_panic_output(|| {
            let Err(error) = open_contained(&relation, EX_SPLIT, &args, None) else {
                panic!("a panicking open must not escape");
            };
            error
        });
        assert!(
            error.to_string().contains("panicked while opening"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn a_panic_in_next_is_a_clean_payload_free_error() {
        let relation = PanickingRelation::new(false);
        let subject_value = iri(EX_A);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = open_contained(&relation, EX_SPLIT, &args, None).expect("open");
        let error = without_panic_output(|| {
            next_contained(&mut *cursor, EX_SPLIT).expect_err("a panicking next must not escape")
        });
        assert!(
            error.to_string().contains("panicked while producing a row"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn take_work_defaults_to_zero_and_is_not_a_charge_a_relation_must_opt_out_of() {
        // The default is what makes the work channel additive rather than breaking: every
        // relation written before it existed reports nothing and is charged nothing.
        let relation = table();
        let subject = [None];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, None).expect("open");
        assert_eq!(cursor.take_work(), 0);
        assert!(cursor.next().expect("no error").is_some());
        assert_eq!(
            cursor.take_work(),
            0,
            "an in-memory scan's cost IS its rows, so it has nothing to add"
        );
    }

    #[test]
    fn a_panic_in_take_work_is_a_clean_payload_free_error() {
        // `take_work` is host code exactly as `next` is, and its answer is SPENT against
        // the caller's fuel, so it crosses the same containment boundary.
        let relation = PanickingRelation::new(false);
        let subject_value = iri(EX_A);
        let subject = [Some(&subject_value)];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = open_contained(&relation, EX_SPLIT, &args, None).expect("open");
        let error = without_panic_output(|| {
            take_work_contained(&mut *cursor, EX_SPLIT)
                .expect_err("a panicking take_work must not escape")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its work"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn open_contained_checks_arity_before_the_relation_runs() {
        // The relation would panic in `open`; the arity check must refuse the call
        // first, so the error names the arity rather than a panic.
        let relation = PanickingRelation::new(true);
        let subject: [Option<&TermValue>; 0] = [];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        let Err(error) = open_contained(&relation, EX_SPLIT, &args, None) else {
            panic!("a mismatched arity is refused");
        };
        assert!(error.to_string().contains("expects"), "got {error}");
        assert!(
            !error.to_string().contains("panicked"),
            "the check must run before the host code: {error}"
        );
    }

    // ---- declaration-read panic containment --------------------------------

    /// A relation whose `arity` panics — the declaration-read half of the trust
    /// boundary, distinct from [`PanickingRelation`]'s `open`/`next` half.
    #[derive(Debug)]
    struct PanickingArityRelation;

    impl PropertyFunction for PanickingArityRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            panic!("arity exploded")
        }

        fn modes(&self) -> &[BindingPattern] {
            &[]
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            0
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            unreachable!("arity panics before open is ever reached")
        }
    }

    #[test]
    fn declaration_contained_catches_a_panic() {
        let error = without_panic_output(|| {
            declaration_contained(EX_SPLIT, "arity", || {
                let relation = PanickingArityRelation;
                relation.arity()
            })
            .expect_err("a panicking read must not escape")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn describe_contains_a_panicking_relations_declaration() {
        // `describe()` is the registry fingerprint's engine — read on every prepare — so
        // a relation whose declaration methods panic must fail it cleanly, not abort.
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(PanickingArityRelation));
        let error = without_panic_output(|| {
            registry
                .describe()
                .expect_err("a panicking arity must not escape describe")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
        assert!(
            !error.to_string().contains("exploded"),
            "the payload must not leak into the deterministic message: {error}"
        );
    }

    #[test]
    fn prepare_contains_a_panicking_relations_declaration() {
        // The same failure, reached through the query-lane entry a host actually calls:
        // `registry_fingerprint` (which `prepare_with_relations`/`prepare_for` consult on
        // every prepare) drives `describe()` internally, so a panicking declaration must
        // surface as a contained `EvalError`, never an abort of the caller's thread.
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_SPLIT, Arc::new(PanickingArityRelation));
        let error = without_panic_output(|| {
            crate::property_fn_plan::registry_fingerprint(&registry)
                .expect_err("a panicking declaration must not escape the fingerprint")
        });
        assert!(
            error
                .to_string()
                .contains("panicked while reporting its arity"),
            "got {error}"
        );
    }
}
