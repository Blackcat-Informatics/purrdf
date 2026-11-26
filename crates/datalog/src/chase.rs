// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **restricted existential chase**: a value-inventing fixpoint over the DL-clause IR,
//! with a computed termination certificate and blank-node Skolem witnesses.
//!
//! [`crate::seminaive`] computes the least model of a set of DEFINITE clauses. It refuses an
//! existential head by name, because a semi-naive evaluator has no witness to derive. This
//! module is the consumer that head form was represented for: given
//! `A(x) → ∃y. (r(x, y) ∧ C(y))` it invents a witness for `y`, asserts BOTH head atoms over
//! it, and runs to a fixpoint.
//!
//! # Three properties this module is built around
//!
//! **The chase is RESTRICTED, not oblivious.** Before a clause fires, its head is run as a
//! conjunctive query with the existentials FREE, seeded with the frontier binding. If any
//! solution exists the firing is SKIPPED — the obligation is already witnessed. Without
//! that check the fixpoint does not converge even for a trivially terminating program:
//! `A(a)` would mint a witness, the next round would see `A(a)` again and mint another, and
//! so on forever. The check is what makes "no new fact this round" a real fixpoint rather
//! than an accident of witness naming.
//!
//! **A witness is a BLANK NODE, never a minted IRI.** PurRDF mints no vocabulary, so it
//! does not mint an individual either. An invented null is exactly what RDF already has a
//! term kind for — a blank node — and its surface is the repository's blank-node surface
//! `_:<scope>.<label>` in the default scope `0`. There is no IRI-namespace parameter
//! anywhere in this module's API, because there is no IRI to name.
//!
//! **Termination is COMPUTED, never demanded.** [`certify`] is a pure function of the
//! clause set: it either proves the program weakly acyclic or names the existential edges
//! that lie inside a cycle. [`chase`] runs the certificate first and refuses an uncertified
//! program by name ([`ChaseError::Termination`]) rather than looping. A caller supplies
//! no acyclicity assertion: preparation is fallible and can be governed independently.
//! There is no caller-chosen depth bound — see the crate docs for why a
//! caller-supplied budget is not offered here either.
//!
//! # Why the position graph is refined by constants
//!
//! Weak acyclicity is a reachability property of the *position dependency graph*: nodes are
//! argument positions, a normal edge runs from a frontier variable's body position to its
//! head position, a special edge runs from a frontier body position to an EXISTENTIAL head
//! position, and the program is weakly acyclic when no special edge lies inside a cycle.
//!
//! Taken over bare `(predicate, slot)` nodes that analysis is far too coarse for RDF,
//! because a description-logic concept name is not a predicate here — it is a CONSTANT in
//! the object slot of a `type` atom. Consider the plainly terminating restriction
//! `C ⊑ ∃p.D`:
//!
//! ```text
//! type(?x, <C>)  →  ∃?y. ( p(?x, ?y) ∧ type(?y, <D>) )
//! ```
//!
//! Unrefined, `?x`'s body position and `?y`'s head position are the SAME node
//! `type[subject]`, so the special edge `type[subject] → type[subject]` is a self-loop and
//! the analysis fails to certify a terminating program. Every `C ⊑ ∃p.D` axiom in every
//! ontology would be refused.
//!
//! A position is therefore refined by the constants co-occurring in the atom's OTHER slots.
//! `?x` occurs at `[s=?, p=<type>, o=<C>, g=…]` and `?y` at `[s=?, p=<type>, o=<D>, g=…]`,
//! which are different nodes, there is no cycle, and the axiom certifies — which is exactly
//! what the test `a_constant_refined_restriction_is_weakly_acyclic` pins.
//!
//! The refinement is SOUND because it only ever SPLITS a position by a constant that
//! genuinely partitions which body atoms can consume a null: a null typed `<D>` sits at the
//! object-constant-`<D>` refinement and cannot be read by a body atom that demands `<C>`
//! there. Where a slot holds a VARIABLE the refinement is the wildcard `*`, and any two
//! refinements of one slot that are compatible — equal, or differing only where one side is
//! a wildcard — are conservatively connected in BOTH directions, so reachability is
//! over-approximated and never under-approximated. A wildcard PREDICATE is the same rule
//! applied to the predicate slot: a rule that quantifies over the predicate is connected to
//! every constant-predicate refinement of the same slot, because at run time it can consume
//! and produce any of them.
//!
//! # Static triple or quad atoms, with the predicate as data
//!
//! A [`ClauseAtom`] stores three triple positions or four quad positions. Every stored
//! position may be a variable, so a position node is keyed by those slots rather than by
//! a relation symbol: `slot` identifies the variable's position and the other stored
//! positions carry the refinement. That
//! makes the predicate position an ordinary analysed slot instead of a special case, which
//! is what an OWL 2 RL meta-rule needs.
//!
//! # What this module REFUSES, by name
//!
//! Each of these is a permanent refusal with a documented reason, never a stub:
//!
//! * [`ChaseError::Termination`] — the analysis found an existential edge in a cycle.
//! * [`ChaseError::DisjunctiveExistentialHead`] — `∃ȳ. (C₁ ∨ … ∨ Cₘ)` with `m > 1`. A
//!   disjunctive head needs a hypertableau case split; there is none here, and PICKING a
//!   disjunct would assert something the program does not entail.
//! * [`ChaseError::DisjunctiveHead`] — `m > 1` with no existential, for the same reason.
//! * [`ChaseError::InconsistencyClause`] — `body → false` derives nothing and asserts its
//!   body is unsatisfiable; a chase has no consistency verdict to return, so firing it is
//!   meaningless and ignoring it would silently answer a different question.
//! * [`ChaseError::NegatedBodyAtom`] — negation as failure has no meaning under a chase
//!   that invents values: a witness minted in a later round can falsify a NAF test an
//!   earlier round already used, so the fixpoint would not be a model of the program. There
//!   is no stratifier here to make the test decidable.
//! * [`ChaseError::UnboundHeadVariable`] — a head variable that is neither existentially
//!   quantified nor bound by the body cannot be grounded without fabricating a term.
//! * [`ChaseError::BudgetExhausted`] — the stored-fact or join-step limit in force
//!   ([`EvalOptions`]), or the caller-selected term-arena ceiling, was passed.
//!
//! The forms that DO fire are the atomic head (a Datalog rule), the conjunctive head
//! (`m = 1`, several conjuncts, no existential) and the single-disjunct existential head.
//! The first two are simply zero-existential conjunctive heads, so ONE loop handles all
//! three: an atomic clause fires exactly as it would in the semi-naive evaluator, and a
//! conjunctive head asserts its conjuncts atomically. Firing a conjunctive head is sound
//! precisely because it is not a disjunction — `→ p ∧ q` entails both conjuncts.
//!
//! # Determinism
//!
//! Every map and set that reaches an output is a `BTreeMap`/`BTreeSet`. A round's pending
//! facts are accumulated in a `BTreeMap<ShapeFact<N>, clause index>`, so they are committed in total
//! lexical `(subject, predicate, object, graph)` order regardless of which clause produced
//! them or in which order the join enumerated rows, and a fact several clauses derive is
//! credited to the lowest authored index rather than to whichever ran first. A witness
//! surface is a content digest of its address, so it does not depend on mint order. There is
//! no wall clock, no RNG, no filesystem and no thread: the module is
//! `wasm32-unknown-unknown`-clean.

use purrdf_hash::Domain;
use purrdf_hash::frame::frame_le_into;
use purrdf_lex::allocation::{Memory, Resident, StorageError, shared::Shared};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct, HeadForm};
use crate::cursor::RowPosition;
use crate::id::TermId;
use crate::plan::{POSITION_OBJECT, POSITION_SUBJECT};
use crate::seminaive::{BudgetReport, BudgetResource, EvalOptions, render_capacity_refusal};
use crate::stop::{StopSignal, is_stopped};
use crate::store::{
    Arity, Bound, PartitionPosition, RelationStore, ShapeFact, StoreShape, TermInterner,
};

mod termination;
pub use termination::{
    AnalysisLimits, AnalysisReport, ChaseCertificate, PreparedChase, SourcePosition,
    TerminationCriterion, TerminationError, TerminationViolation, WA_LAW_VERSION, WaCertificate,
    certify, certify_with,
};

/// What the termination analysis proved about a clause set.
///
/// Computed by [`certify`] and carried on [`ChaseOutcome`] so a caller can put the
/// certificate in a report next to the facts it justifies.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChaseTermination {
    /// No existential edge of the position dependency graph lies in a cycle.
    ///
    /// The chase over such a program terminates: every null is created at a position from
    /// which no path returns to a position that creates nulls, so the number of nulls is
    /// bounded by the program rather than by the data.
    WeaklyAcyclic {
        /// How many distinct refined positions the graph holds — the proof's size.
        positions: usize,
        /// How many distinct existential edges were checked.
        existential_edges: usize,
    },
    /// The analysis found an existential edge inside a cycle.
    ///
    /// This does NOT prove the program non-terminating — weak acyclicity is sufficient, not
    /// necessary — but it is the only honest answer this analysis can give, and [`chase`]
    /// refuses rather than risk a fixpoint that never closes.
    Uncertified {
        /// First offending existential edge in canonical source-position order.
        violation: TerminationViolation,
    },
}

impl ChaseTermination {
    /// Whether the analysis certified termination.
    pub const fn is_certified(&self) -> bool {
        match self {
            Self::WeaklyAcyclic { .. } => true,
            Self::Uncertified { .. } => false,
        }
    }

    /// The sufficient criterion proved by this summary, absent for a refusal.
    #[must_use]
    pub const fn criterion(&self) -> Option<TerminationCriterion> {
        match self {
            Self::WeaklyAcyclic { .. } => Some(TerminationCriterion::WeaklyAcyclic),
            Self::Uncertified { .. } => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for ChaseTermination {
    purrdf_hash::error_visit! {
        WeaklyAcyclic { positions, existential_edges } => "chase-weakly-acyclic" {
            "positions" => Unsigned(*positions as u128), "existential_edges" => Unsigned(*existential_edges as u128)
        },
        Uncertified { violation } => "chase-wa-uncertified" {
            "edge" => Record(violation)
        },
    }
}

impl fmt::Display for ChaseTermination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WeaklyAcyclic {
                positions,
                existential_edges,
            } => write!(
                f,
                "weakly acyclic: {positions} refined position(s), {existential_edges} \
                 existential edge(s), none in a cycle"
            ),
            Self::Uncertified { violation } => violation.fmt(f),
        }
    }
}

// ── Witnesses ───────────────────────────────────────────────────────────────────

/// The blank-node scope a chase witness is minted into: the DEFAULT scope.
///
/// The repository renders a blank node as `_:<scope ordinal>.<label>`, and scope `0` is the
/// default scope. A chase witness belongs to the dataset being closed, so it is minted
/// there rather than into a scope of its own.
const WITNESS_SCOPE: &str = "0";

/// The first character of every witness label.
///
/// A leading letter keeps the label a well-formed blank-node label whatever the digest
/// happens to start with, and it makes a witness visually identifiable in a fact dump.
const WITNESS_LABEL_PREFIX: char = 'w';

/// How many digest bytes a witness label carries (rendered as twice as many hex digits).
const WITNESS_LABEL_BYTES: usize = 16;

/// The domain-separation tag hashed into every witness address.
///
/// Framed like every other field, so a witness digest can never coincide with some other
/// BLAKE3 digest this crate computes over a different kind of value.
const WITNESS_DIGEST_TAG: Domain = Domain::new(b"purrdf-datalog restricted chase witness v1");
const WITNESS_ESCAPE_TAG: Domain =
    Domain::new(b"purrdf-datalog restricted chase witness freshness v1");

/// The address a Skolem witness is minted against: the SKOLEM FUNCTION APPLICATION that
/// produced it.
///
/// A witness is a function of the clause, the existential's ordinal within that clause's
/// quantifier list, and the bound frontier VALUES — never of the lexical variable names
/// alone, and never of the order firings happened to arrive in. Two different frontier
/// bindings therefore get two different witnesses, and re-deriving the same obligation on
/// the same frontier recovers the SAME witness, which is what lets the fixpoint close.
///
/// The address is decomposable, which an opaque digest is not: [`SkolemRegistry::explain`]
/// hands it back so a caller can say what an invented null stands for. A frontier value may
/// itself be a previously invented witness, so an explanation is recursively decomposable
/// through the same registry. A separately versioned salt keeps an authored blank with the
/// original label from becoming that witness.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WitnessAddress {
    /// The producing clause's index in authored program order.
    clause: usize,
    /// The existential's position in the clause's authored quantifier list.
    ordinal: usize,
    /// The bound frontier values, as lexical surfaces, in the clause's fixed frontier
    /// order (the frontier variables in lexical name order).
    frontier: Vec<String>,
    /// Zero retains the original label. Later candidates avoid occupied terms.
    salt: u64,
}

impl WitnessAddress {
    /// The producing clause's index in authored program order.
    pub fn clause(&self) -> usize {
        self.clause
    }

    /// The existential's position in the clause's authored quantifier list.
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// The bound frontier values, as lexical surfaces, in the clause's fixed frontier order.
    pub fn frontier(&self) -> &[String] {
        &self.frontier
    }

    /// Zero for the original label, or the deterministic collision escape.
    pub fn salt(&self) -> u64 {
        self.salt
    }
}

/// The blank-node surface addressed by `address`.
///
/// The label is a BLAKE3 digest over a length-framed encoding of the address: every field is
/// preceded by its byte length, so no frontier value can forge a field boundary whatever
/// bytes it holds, and the encoding is injective. The surface is a deterministic,
/// collision-resistant function of the address alone, identical on every target. The
/// original domain remains exact for unoccupied labels.
fn witness_digest(address: &WitnessAddress) -> purrdf_hash::blake3::Hash {
    let mut hasher = purrdf_hash::blake3::RecordHasher::new();
    let domain = if address.salt == 0 {
        WITNESS_DIGEST_TAG
    } else {
        WITNESS_ESCAPE_TAG
    };
    frame_le_into(&mut hasher, domain.as_bytes());
    if address.salt != 0 {
        hasher.update(&address.salt.to_le_bytes());
    }
    hasher.update(&(address.clause as u64).to_le_bytes());
    hasher.update(&(address.ordinal as u64).to_le_bytes());
    hasher.update(&(address.frontier.len() as u64).to_le_bytes());
    for value in &address.frontier {
        frame_le_into(&mut hasher, value.as_bytes());
    }
    hasher.finalize()
}

fn witness_surface(address: &WitnessAddress) -> Result<String, StorageError> {
    let digest = witness_digest(address);
    let mut surface = String::new();
    Memory::new(&mut Resident).reserve_string(
        &mut surface,
        "_:.".len() + WITNESS_SCOPE.len() + 1 + 2 * WITNESS_LABEL_BYTES,
    )?;
    surface.push_str("_:");
    surface.push_str(WITNESS_SCOPE);
    surface.push('.');
    surface.push(WITNESS_LABEL_PREFIX);
    purrdf_hash::hex::encode_into(&digest.as_bytes()[..WITNESS_LABEL_BYTES], &mut surface);
    Ok(surface)
}

/// The witnesses one chase invented, keyed by blank-node surface.
///
/// **Idempotent**: minting the same address twice returns the same surface and retains the
/// same recipe. That is not a convenience — it is the second half of the termination
/// argument. The restricted-satisfaction check skips a firing whose obligation is already
/// witnessed; where a check cannot see a witness yet (two body solutions sharing one
/// frontier inside a single round, say) the registry collapses the repeat firings onto one
/// witness and the round's fact set deduplicates. Without idempotence each re-derivation
/// would invent a fresh null and the fixpoint would never close.
///
/// **Deterministic**: a `BTreeMap`, so every sweep is sorted. The key is a content digest,
/// with further deterministic candidates if a term already owns the original label.
#[derive(Debug, Clone, Default)]
pub struct SkolemRegistry {
    /// Witness surface → the address it was minted against.
    witnesses: BTreeMap<String, WitnessAddress>,
}

impl SkolemRegistry {
    /// Mint — or recover — the witness for `address`, returning its blank-node surface.
    fn mint(
        &mut self,
        mut address: WitnessAddress,
        occupied: impl Fn(&str) -> bool,
    ) -> Result<String, StorageError> {
        loop {
            let surface = witness_surface(&address)?;
            match self.witnesses.get(&surface) {
                Some(previous) if *previous == address => return Ok(surface),
                None if !occupied(&surface) => {
                    self.witnesses
                        .insert(Memory::new(&mut Resident).string(&surface)?, address);
                    return Ok(surface);
                }
                _ => {
                    address.salt = address
                        .salt
                        .checked_add(1)
                        .ok_or(StorageError::SizeOverflow)?;
                }
            }
        }
    }

    /// The address behind an invented witness, or `None` if this registry never minted it.
    ///
    /// This is the "explain an invented individual" surface: a witness is a blank node whose
    /// label is a digest and therefore says nothing on its own, but the registry retains the
    /// Skolem-function application — clause, ordinal, frontier binding — that produced it.
    pub fn explain(&self, witness: &str) -> Option<&WitnessAddress> {
        self.witnesses.get(witness)
    }

    /// Every invented witness surface, in sorted order.
    pub fn witnesses(&self) -> impl Iterator<Item = &str> {
        self.witnesses.keys().map(String::as_str)
    }

    /// How many distinct witnesses were invented.
    pub fn len(&self) -> usize {
        self.witnesses.len()
    }

    /// Whether no witness was invented.
    pub fn is_empty(&self) -> bool {
        self.witnesses.is_empty()
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────────

/// Why a chase could not run, or could not run to completion.
///
/// Every variant is a hard refusal with a named reason. There is no partial answer and no
/// best-effort mode: a [`ChaseOutcome`] this module returns is a complete universal model of
/// the program it was given.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChaseError {
    /// Immutable-base body reads require the ordered scheduler.
    BaseReadClause {
        /// The clause requiring ordered immutable-base semantics.
        clause: usize,
    },
    /// Operational no-output heads belong to the ordered executor.
    NoOutputClause {
        /// The clause's index in authored program order.
        clause: usize,
    },
    /// The original logical capacity or physical storage refusal.
    Storage(StorageError),
    /// The complete existential rule bundle violates a declaration.
    NeverDerive(crate::admission::AdmissionRefusal),
    /// Native termination preparation, checking or account reclamation refused.
    Termination(TerminationError),
    /// An EXISTENTIAL head has more than one disjunct.
    ///
    /// `∃ȳ. (C₁ ∨ … ∨ Cₘ)` with `m > 1` has no single universal model: satisfying it means
    /// branching on which disjunct holds, which is a hypertableau case split. This module
    /// has none. Picking a disjunct — even the first — would assert something the program
    /// does not entail, so the clause is refused instead.
    DisjunctiveExistentialHead {
        /// The clause's index in authored program order.
        clause: usize,
        /// How many disjuncts the head has.
        disjuncts: usize,
    },
    /// A head is disjunctive with no existential, `m > 1` and `ȳ = ∅`.
    ///
    /// Refused for the same reason as [`Self::DisjunctiveExistentialHead`]: a disjunction
    /// has no least model, and the chase computes one model, not a set of them.
    DisjunctiveHead {
        /// The clause's index in authored program order.
        clause: usize,
        /// How many disjuncts the head has.
        disjuncts: usize,
    },
    /// A clause's head is `false` — the inconsistency clause `body → false`.
    ///
    /// It derives nothing and instead asserts that its body is unsatisfiable. A chase
    /// returns a model, not a consistency verdict, so there is no answer to give: firing it
    /// is meaningless and dropping it would silently answer a different question.
    InconsistencyClause {
        /// The clause's index in authored program order.
        clause: usize,
    },
    /// A body atom is negated.
    ///
    /// Negation as failure and value invention do not compose without a stratification
    /// argument: a witness invented in a later round can falsify a NAF test an earlier
    /// round already relied on, so the result would not be a model of the program. There is
    /// no stratifier here, so the clause is refused rather than evaluated under a semantics
    /// this module cannot justify.
    NegatedBodyAtom {
        /// The clause's index in authored program order.
        clause: usize,
        /// The negated atom's index in the clause's authored body.
        body_index: usize,
    },
    /// A clause carries a guard literal or a negated conjunction ([`crate::guard`]).
    ///
    /// A restricted chase decides whether an obligation is already witnessed by matching
    /// the clause text against the model; a guard's truth is caller code that match cannot
    /// run, and a negated conjunction is negation as failure, which a chase has no meaning
    /// for (see [`Self::NegatedBodyAtom`]).
    GuardedClause {
        /// The clause's index in authored program order.
        clause: usize,
    },
    /// A head variable is neither existentially quantified nor bound by the body.
    ///
    /// Grounding the head would mean fabricating a term for it. An existential head variable
    /// is the SUPPORTED way to say "some value exists here"; an unquantified one is a typo.
    UnboundHeadVariable {
        /// The clause's index in authored program order.
        clause: usize,
        /// The unbindable variable, as authored.
        variable: String,
    },
    /// A limit was passed. The report is accurate at the point the chase stopped.
    ///
    /// The limits are the stored-fact and join-step limits of the caller's [`EvalOptions`]
    /// ([`chase_with`]; unlimited capacity for [`chase`] and [`chase_until`]) and the
    /// caller-selected [`EvalOptions::max_term_arena_bytes`], charged exactly as [`crate::seminaive::evaluate`]
    /// charges them. There is never a truncated answer presented as complete.
    BudgetExhausted {
        /// Which ceiling.
        resource: BudgetResource,
        /// Consumption, and the limits in force, when the chase stopped.
        report: BudgetReport,
    },
    /// The caller's [`crate::stop::StopSignal`] fired at a round boundary.
    ///
    /// As total a refusal as [`Self::BudgetExhausted`]: the rounds already committed are not
    /// a universal model of anything, and no witness the chase had minted crosses this
    /// boundary. Distinct from it because a limit being passed is a fact about the program,
    /// the data and the limits, while this is a decision the host made about this one run.
    Stopped {
        /// Consumption, and the limits in force, when the signal was observed.
        report: BudgetReport,
    },
}

impl purrdf_hash::error::StructuredError for ChaseError {
    purrdf_hash::error_visit! {
        BaseReadClause { clause } => "chase-base-read-clause" { "clause" => Unsigned(*clause as u128) },
        NoOutputClause { clause } => "chase-no-output-clause" { "clause" => Unsigned(*clause as u128) },
        Storage(cause) => "chase-storage" { "cause" => Cause(cause) },
        NeverDerive(cause) => "chase-never-derive" { "cause" => Cause(cause) },
        Termination(cause) => "chase-termination-refused" { "cause" => Cause(cause) },
        DisjunctiveExistentialHead { clause, disjuncts } => "chase-disjunctive-existential-head" { "clause" => Unsigned(*clause as u128), "disjuncts" => Unsigned(*disjuncts as u128) },
        DisjunctiveHead { clause, disjuncts } => "chase-disjunctive-head" { "clause" => Unsigned(*clause as u128), "disjuncts" => Unsigned(*disjuncts as u128) },
        InconsistencyClause { clause } => "chase-inconsistency-clause" { "clause" => Unsigned(*clause as u128) },
        NegatedBodyAtom { clause, body_index } => "chase-negated-body-atom" { "clause" => Unsigned(*clause as u128), "body_index" => Unsigned(*body_index as u128) },
        GuardedClause { clause } => "chase-guarded-clause" { "clause" => Unsigned(*clause as u128) },
        UnboundHeadVariable { clause, variable } => "chase-unbound-head-variable" { "clause" => Unsigned(*clause as u128), "variable" => Text(variable) },
        BudgetExhausted { resource, report } => "chase-budget-exhausted" { "resource" => Record(resource), "report" => Record(report) },
        Stopped { report } => "chase-stopped" { "report" => Record(report) },
    }
}

impl fmt::Display for ChaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BaseReadClause { clause } => write!(
                f,
                "clause {clause} reads immutable base facts and requires ordered evaluation"
            ),
            Self::NoOutputClause { clause } => write!(
                f,
                "clause {clause} has an operational no-output head: use the ordered scheduler"
            ),
            Self::Storage(error) => error.fmt(f),
            Self::NeverDerive(refusal) => fmt::Display::fmt(refusal, f),
            Self::Termination(error) => error.fmt(f),
            Self::DisjunctiveExistentialHead { clause, disjuncts } => write!(
                f,
                "clause {clause} has an existential head with {disjuncts} disjuncts: the chase \
                 has no case split, and choosing a disjunct would assert what the program does \
                 not entail"
            ),
            Self::DisjunctiveHead { clause, disjuncts } => write!(
                f,
                "clause {clause} has a disjunctive head with {disjuncts} disjuncts: the chase \
                 computes one model, and a disjunction has no least one"
            ),
            Self::InconsistencyClause { clause } => write!(
                f,
                "clause {clause} has an empty (false) head: the chase returns a model, not a \
                 consistency verdict"
            ),
            Self::GuardedClause { clause } => write!(
                f,
                "clause {clause} carries guard literals, which the chase cannot evaluate"
            ),
            Self::NegatedBodyAtom { clause, body_index } => write!(
                f,
                "clause {clause} body atom {body_index} is negated: negation as failure has no \
                 semantics under a chase that invents values, and there is no stratifier here"
            ),
            Self::UnboundHeadVariable { clause, variable } => write!(
                f,
                "clause {clause} head variable {variable} is neither existentially quantified \
                 nor bound by the body, so it cannot be grounded"
            ),
            Self::BudgetExhausted { resource, report } => {
                let knob = match resource {
                    BudgetResource::StoredFacts => "EvalOptions::with_max_stored_facts",
                    _ => "EvalOptions::with_max_join_steps",
                };
                match render_capacity_refusal(*resource, *report, knob) {
                    Some(rendered) => write!(f, "the chase did not complete: {rendered}"),
                    None => write!(
                        f,
                        "the chase exceeded a configured ceiling ({resource:?}): {} join step(s), \
                         {} stored fact(s), {} term arena byte(s)",
                        report.join_steps(),
                        report.stored_facts(),
                        report.term_arena_bytes()
                    ),
                }
            }
            Self::Stopped { report } => write!(
                f,
                "the chase was stopped by the caller's stop signal after {} join step(s), \
                 holding {} fact(s): no universal model was computed",
                report.join_steps(),
                report.stored_facts()
            ),
        }
    }
}

purrdf_lex::error_source!(ChaseError {
    Storage,
    Termination
});

purrdf_lex::variant_from!(ChaseError { Storage(StorageError), Termination(TerminationError) });

// ── The chase ───────────────────────────────────────────────────────────────────

/// A variable binding: variable name → the bound term's lexical surface.
///
/// A `BTreeMap` because a binding is walked when a witness address is assembled, and that
/// walk reaches an output.
type Binding = BTreeMap<String, String>;

/// The candidate-solution meter, charged exactly as [`crate::seminaive`] charges the
/// join-step limit: one step per partial or complete solution appended by an atom
/// extension.
///
/// Charging inside the join — rather than counting committed facts afterwards — is what
/// bounds a body that is accidentally Cartesian: such a body enumerates millions of
/// candidates while committing three facts, and a ceiling on commits alone would never see
/// it. The meter permits ONE step past the ceiling so that reaching it is proof the ceiling
/// was passed; the refusal is then raised by [`ChaseState::check_budget`].
#[derive(Debug, Clone, Copy)]
struct StepMeter {
    /// Candidate solutions enumerated so far.
    consumed: u64,
    /// The join-step limit in force.
    limit: u64,
    /// A further candidate cannot be represented by the full-width observation.
    overflowed: bool,
}

impl StepMeter {
    /// A meter charging against `limit`.
    fn new(limit: u64) -> Self {
        Self {
            consumed: 0,
            limit,
            overflowed: false,
        }
    }

    /// Record one enumerated candidate.
    fn charge(&mut self) {
        self.overflowed |= self.consumed == u64::MAX;
        self.consumed = self.consumed.saturating_add(1);
    }

    /// Whether the ceiling has been passed, so no further candidate may be enumerated.
    fn spent(self) -> bool {
        self.overflowed || self.consumed > self.limit
    }
}

/// Extend `solution` by every row of the store that `atom` matches, appending each
/// extension to `out`.
///
/// The stored positions are resolved once: a constant renders to its lexical surface, a bound
/// variable reads its binding, an unbound variable is free. The predicate and optional graph
/// positions select the partitions, the subject and object positions drive the partition's
/// `(subject, object)` index, and the two are independent — exactly the access path
/// [`crate::seminaive`] uses, so an atom that names its predicate costs one ordered-map
/// probe and one that quantifies over it costs a lexical partition sweep.
///
/// A repeated variable — within the atom or shared with `solution` — is enforced per row
/// rather than through the index, so `p(?x, ?p, ?x, ?g)` behaves as the diagonal it is.
#[derive(Debug)]
struct AtomScan<const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    partitions: PartitionPosition<TermId, N>,
    bound: Bound,
    active: Option<(crate::id::PartitionId, RowPosition)>,
}

impl<const N: usize> AtomScan<N>
where
    Arity<N>: StoreShape<N>,
{
    fn new(
        atom: &ClauseAtom<N>,
        store: &RelationStore<TermInterner, N>,
        solution: &Binding,
    ) -> Option<Self> {
        let terms = atom.terms();
        let known: [Option<String>; N] = std::array::from_fn(|index| match terms[index] {
            ClauseTerm::Var(name) => solution.get(name).cloned(),
            constant => constant.surface(),
        });
        let mut ids: [Option<TermId>; N] = [None; N];
        for (index, surface) in known.iter().enumerate() {
            if let Some(surface) = surface {
                ids[index] = Some(store.term_id(surface)?);
            }
        }
        let bound = match (ids[POSITION_SUBJECT], ids[POSITION_OBJECT]) {
            (Some(subject), Some(object)) => Bound::Both(subject, object),
            (Some(subject), None) => Bound::Subject(subject),
            (None, Some(object)) => Bound::Object(object),
            (None, None) => Bound::Any,
        };
        Some(Self {
            partitions: PartitionPosition::new(store, ids),
            bound,
            active: None,
        })
    }

    fn next(&mut self, store: &RelationStore<TermInterner, N>) -> Option<[TermId; N]> {
        loop {
            if let Some((id, rows)) = self.active.as_mut() {
                let partition = store.partition_with_id(*id);
                if let Some((subject, object, _)) = rows.next(partition.arrangement()) {
                    return Some(partition.row(subject, object));
                }
            }
            let partition = self.partitions.next(store, None)?;
            self.active = Some((
                partition.id(),
                RowPosition::new(partition.arrangement(), self.bound),
            ));
        }
    }

    fn advance(
        &mut self,
        atom: &ClauseAtom<N>,
        store: &RelationStore<TermInterner, N>,
        solution: &Binding,
        out: &mut Vec<Binding>,
        meter: &mut StepMeter,
    ) -> bool {
        let terms = atom.terms();
        let interner = store.interner();
        while !meter.spent() {
            let Some(row) = self.next(store) else {
                return true;
            };
            let matched = row.map(|id| interner.resolve(id));
            // Agreement first, allocation second: a row that fails the diagonal never
            // clones the binding.
            let mut fresh: [Option<(&str, &str)>; N] = [None; N];
            let mut agrees = true;
            for (index, term) in terms.iter().enumerate() {
                let Some(name) = term.variable() else {
                    // A constant position was pinned through the index above, so it agrees.
                    continue;
                };
                let already = solution.get(name).map(String::as_str).or_else(|| {
                    fresh
                        .iter()
                        .flatten()
                        .find(|(bound_name, _)| *bound_name == name)
                        .map(|&(_, value)| value)
                });
                match already {
                    Some(value) if value != matched[index] => {
                        agrees = false;
                        break;
                    }
                    Some(_) => {}
                    None => fresh[index] = Some((name, matched[index])),
                }
            }
            if !agrees {
                continue;
            }
            let mut merged = solution.clone();
            for &(name, value) in fresh.iter().flatten() {
                merged.insert(name.to_owned(), value.to_owned());
            }
            meter.charge();
            out.push(merged);
        }
        false
    }
}

/// Join the atoms of a clause over its original store.
///
/// Used twice, and that reuse is the point: once for a clause BODY seeded with the empty
/// binding, and once for the restricted-satisfaction probe over a head disjunct seeded with
/// the frontier binding and the existentials left FREE. The blocking condition is literally
/// "does this conjunctive query have a solution", so writing it as a second, subtly
/// different traversal would be an opportunity for the two to disagree.
#[derive(Debug)]
struct JoinFrame<const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    atom: usize,
    input: usize,
    solutions: Vec<Binding>,
    next: Vec<Binding>,
    scan: Option<AtomScan<N>>,
}

impl<const N: usize> JoinFrame<N>
where
    Arity<N>: StoreShape<N>,
{
    fn new(seed: &Binding) -> Self {
        Self {
            atom: 0,
            input: 0,
            solutions: vec![seed.clone()],
            next: Vec::new(),
            scan: None,
        }
    }

    fn advance(
        &mut self,
        atoms: &[ClauseAtom<N>],
        store: &RelationStore<TermInterner, N>,
        meter: &mut StepMeter,
    ) -> bool {
        while let Some(atom) = atoms.get(self.atom) {
            if self.solutions.is_empty() {
                return true;
            }
            while let Some(solution) = self.solutions.get(self.input) {
                if meter.spent() {
                    return false;
                }
                if self.scan.is_none() {
                    self.scan = AtomScan::new(atom, store, solution);
                }
                if let Some(scan) = &mut self.scan
                    && !scan.advance(atom, store, solution, &mut self.next, meter)
                {
                    return false;
                }
                self.scan = None;
                self.input += 1;
            }
            self.solutions = core::mem::take(&mut self.next);
            self.atom += 1;
            self.input = 0;
        }
        true
    }
}

/// The restriction of `solution` to `variables`.
fn restrict(solution: &Binding, variables: &[String]) -> Binding {
    variables
        .iter()
        .filter_map(|name| {
            solution
                .get(name)
                .map(|value| (name.clone(), value.clone()))
        })
        .collect()
}

/// Ground one head atom under a binding that covers every variable it mentions.
///
/// # Panics
///
/// Panics on a variable the binding does not cover. [`plan_firings`] refuses a clause whose
/// head carries a variable that is neither a frontier variable nor an existential, the
/// frontier is bound by the body join, and every existential is bound to a freshly minted
/// witness before this is reached, so an uncovered variable here is a contradiction in that
/// check rather than a data state.
fn ground<const N: usize>(atom: &ClauseAtom<N>, binding: &Binding) -> ShapeFact<N>
where
    Arity<N>: StoreShape<N>,
{
    let terms = atom.terms();
    let surfaces: [String; N] = std::array::from_fn(|index| match terms[index] {
        ClauseTerm::Var(name) => binding
            .get(name)
            .cloned()
            .expect("every head variable is a bound frontier variable or a minted witness"),
        constant => constant
            .surface()
            .expect("a non-variable term always has a lexical surface"),
    });
    Arity::<N>::fact(surfaces)
}

/// One clause the chase will actually fire, with its loop-invariant shapes hoisted out of
/// every round.
#[derive(Debug, Clone)]
struct Firing {
    /// The clause's index in authored program order.
    clause: usize,
    /// The frontier variables, in lexical name order — the FIXED order a witness address's
    /// frontier values are listed in.
    frontier: Vec<String>,
}

impl Firing {
    /// The one head disjunct this clause fires.
    ///
    /// Every fireable head form has exactly one disjunct: the atomic and conjunctive forms
    /// by definition, and the existential form because [`plan_firings`] refuses a
    /// multi-disjunct existential head.
    fn disjunct<'a, const N: usize>(&self, clause: &'a DlClause<N>) -> &'a HeadDisjunct<N> {
        clause
            .head_disjuncts()
            .first()
            .expect("a fireable clause has exactly one head disjunct")
    }
}

/// Decide which clauses the chase fires, refusing every head form it has no semantics for.
///
/// The head form is decided FIRST, because the later checks are defined in terms of a head
/// the clause may not have: naming an unbound head variable in a clause whose real defect is
/// that its head is `false` would report a consequence rather than the cause.
fn plan_firings<const N: usize>(program: &[DlClause<N>]) -> Result<Vec<Firing>, ChaseError> {
    let mut firings = Vec::with_capacity(program.len());
    for (clause_index, clause) in program.iter().enumerate() {
        if clause.reads_base() {
            return Err(ChaseError::BaseReadClause {
                clause: clause_index,
            });
        }
        let disjuncts = clause.head_disjuncts().len();
        match clause.head_form() {
            HeadForm::NoOutput => {
                return Err(ChaseError::NoOutputClause {
                    clause: clause_index,
                });
            }
            HeadForm::Inconsistency => {
                return Err(ChaseError::InconsistencyClause {
                    clause: clause_index,
                });
            }
            HeadForm::Disjunctive => {
                return Err(ChaseError::DisjunctiveHead {
                    clause: clause_index,
                    disjuncts,
                });
            }
            HeadForm::Existential if disjuncts > 1 => {
                return Err(ChaseError::DisjunctiveExistentialHead {
                    clause: clause_index,
                    disjuncts,
                });
            }
            // An atomic head is a one-atom, zero-existential conjunctive head and a
            // conjunctive head is a zero-existential one, so all three fire through the
            // same loop.
            HeadForm::Atomic | HeadForm::Conjunctive | HeadForm::Existential => {}
        }

        for (body_index, atom) in clause.body().iter().enumerate() {
            if atom.is_negated() {
                return Err(ChaseError::NegatedBodyAtom {
                    clause: clause_index,
                    body_index,
                });
            }
        }
        if clause.is_guarded() {
            return Err(ChaseError::GuardedClause {
                clause: clause_index,
            });
        }

        let frontier = clause.frontier_variables();
        let existentials: BTreeSet<&str> =
            clause.existentials().iter().map(String::as_str).collect();
        for atom in clause.head_atoms() {
            for term in atom.terms() {
                if let Some(name) = term.variable()
                    && !frontier.contains(name)
                    && !existentials.contains(name)
                {
                    return Err(ChaseError::UnboundHeadVariable {
                        clause: clause_index,
                        variable: name.to_owned(),
                    });
                }
            }
        }

        firings.push(Firing {
            clause: clause_index,
            frontier: frontier.into_iter().collect(),
        });
    }
    Ok(firings)
}

/// The mutable working set carried across the chase's rounds.
#[derive(Debug)]
struct ChaseState<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The accumulated store: the seeded EDB plus everything derived so far.
    store: RelationStore<TermInterner, N>,
    /// Every derived fact with its producing clause, in commit order.
    derivations: Vec<ChaseDerivation<N>>,
    /// Optional firing evidence, indexed by committed derivation position.
    proofs: Option<Vec<ChaseFiring<N>>>,
    /// The witnesses invented so far.
    witnesses: SkolemRegistry,
    /// The candidate-solution meter.
    meter: StepMeter,
    /// The limits in force.
    options: EvalOptions,
}

impl<const N: usize> ChaseState<N>
where
    Arity<N>: StoreShape<N>,
{
    /// The budget consumption observed so far.
    fn report(&self) -> BudgetReport {
        BudgetReport::new(
            self.meter.consumed,
            self.store.retained_rows(),
            self.store.term_bytes(),
        )
        .governed_by(&self.options)
    }

    /// Refuse if any limit is already passed.
    ///
    /// Mirrors [`crate::seminaive`]'s own check exactly, including the order the ceilings
    /// are tested in, so the same overrun is reported under the same resource name whichever
    /// engine observed it.
    fn check_budget(&self) -> Result<(), ChaseError> {
        let report = self.report();
        if self.meter.spent() {
            return Err(self.join_refusal());
        }
        if u64::try_from(report.stored_facts()).unwrap_or(u64::MAX) > report.stored_fact_limit() {
            return Err(ChaseError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                report,
            });
        }
        if report.term_arena_bytes() as u64 > report.term_arena_limit() {
            return Err(ChaseError::BudgetExhausted {
                resource: BudgetResource::TermArenaBytes,
                report,
            });
        }
        Ok(())
    }

    fn join_refusal(&self) -> ChaseError {
        ChaseError::BudgetExhausted {
            resource: BudgetResource::JoinSteps,
            report: self.report(),
        }
    }
}

/// One fact the chase derived, and the clause that concluded it.
///
/// The chase's counterpart to [`crate::seminaive::Derivation`], and deliberately smaller
/// than it: a chase firing's antecedents are not a fixed-arity body match (a conjunctive
/// head asserts several facts from one firing, and a witness has no source row at all), so
/// this records the ATTRIBUTION rather than a proof. A consumer that credits a derived
/// triple to the specification rule that concluded it — which is what a reasoning report
/// does — needs exactly the authored clause index, and mapping that index back to a rule is
/// the consumer's own table, because one rule may author several clauses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaseDerivation<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The derived fact.
    fact: ShapeFact<N>,
    /// The producing clause's index in authored program order.
    clause: usize,
}

/// Evidence for one committed head atom, in the authored clause's body order.
#[derive(Debug)]
pub struct ChaseFiring<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    head: usize,
    premises: Shared<Vec<ShapeFact<N>>>,
}

impl<const N: usize> ChaseFiring<N>
where
    Arity<N>: StoreShape<N>,
{
    /// Index of the concluded atom within the clause's sole head conjunction.
    pub fn head(&self) -> usize {
        self.head
    }

    /// Grounded antecedents in authored order, including repeats.
    pub fn premises(&self) -> &[ShapeFact<N>] {
        &self.premises
    }
}

/// Independent checking of committed firing evidence failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChaseProofError {
    /// The chase did not record firing evidence.
    NotRecorded,
    /// The proof stream does not cover the complete committed prefix.
    IncompletePrefix,
    /// The firing at this commit position does not follow from the source
    /// and preceding checked facts under the authored clause.
    InvalidFiring(usize),
    /// Independent verification could not allocate its working set.
    Storage(StorageError),
}

purrdf_lex::variant_from!(ChaseProofError { Storage(StorageError) });

impl fmt::Display for ChaseProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRecorded => f.write_str("chase did not record firing evidence"),
            Self::IncompletePrefix => {
                f.write_str("recorded chase proof does not cover the committed prefix")
            }
            Self::InvalidFiring(index) => {
                write!(f, "invalid recorded chase firing at commit {index}")
            }
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ChaseProofError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

impl<const N: usize> ChaseDerivation<N>
where
    Arity<N>: StoreShape<N>,
{
    /// The derived fact.
    pub fn fact(&self) -> &ShapeFact<N> {
        &self.fact
    }

    /// The producing clause's index in AUTHORED program order.
    ///
    /// Indexes the `program` slice [`chase`] was called with, so a caller can map it back to
    /// whatever authored the clause. A conjunctive or existential head asserts several facts
    /// from ONE firing, and every one of them carries that same clause index — which is
    /// correct for a consumer counting one credit per triple that entered the closure.
    pub fn clause(&self) -> usize {
        self.clause
    }
}

/// A completed chase: the closed store, the facts it derived, the witnesses it invented,
/// what it consumed, and the certificate that justified running it at all.
#[derive(Debug, Clone)]
pub struct ChaseOutcome<const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The universal model: the seeded EDB plus every derived fact.
    facts: RelationStore<TermInterner, N>,
    /// Every derived fact, in commit order — round by round, and lexically within a round.
    derived: Vec<ShapeFact<N>>,
    /// The same facts with their producing clause, in the same order as `derived`.
    derivations: Vec<ChaseDerivation<N>>,
    /// The witnesses the chase invented.
    witnesses: SkolemRegistry,
    /// What the run consumed, against the limits it ran under.
    budget: BudgetReport,
    /// The termination certificate that admitted the program.
    termination: ChaseTermination,
}

impl<const N: usize> ChaseOutcome<N>
where
    Arity<N>: StoreShape<N>,
{
    /// The universal model: the seeded EDB plus every derived fact.
    pub fn facts(&self) -> &RelationStore<TermInterner, N> {
        &self.facts
    }

    /// Take ownership of the universal model.
    pub fn into_facts(self) -> RelationStore<TermInterner, N> {
        self.facts
    }

    /// Every derived fact, in commit order.
    ///
    /// The order is round by round and, within a round, total lexical order
    /// of each fact's terms — so it is byte-deterministic while still
    /// showing which layer of the fixpoint produced each fact. It is NOT globally sorted,
    /// because flattening the layers would discard that.
    pub fn derived(&self) -> &[ShapeFact<N>] {
        &self.derived
    }

    /// Every derived fact with the clause that concluded it, in commit order.
    ///
    /// Entry `i` here is entry `i` of [`Self::derived`] — the same facts, in the same
    /// order, with the attribution attached.
    ///
    /// **Which clause gets the credit.** Exactly one derivation per COMMITTED fact, and the
    /// credit goes to the FIRST commit: a fact a later round re-derives is already in the
    /// store and never enters the vector again, so it keeps the attribution it entered the
    /// closure with. Within a single round the round's candidates are accumulated in a
    /// `BTreeMap` keyed by the fact's lexical surfaces and attributed by the LOWEST
    /// authored clause index that produced them, so "first" is decided by the program and
    /// the data — never by which clause the loop happened to reach first, and never by the
    /// order the join enumerated rows in.
    pub fn derivations(&self) -> &[ChaseDerivation<N>] {
        &self.derivations
    }

    /// The witnesses the chase invented, so an invented null can be explained.
    pub fn witnesses(&self) -> &SkolemRegistry {
        &self.witnesses
    }

    /// What the run consumed, against the limits it ran under.
    pub fn budget(&self) -> BudgetReport {
        self.budget
    }

    /// The termination certificate that admitted the program.
    ///
    /// Its native criterion certified the program before execution. The
    /// [`Uncertified`](ChaseTermination::Uncertified) case is returned as
    /// [`ChaseError::Termination`] instead. The certified summary accompanies the facts
    /// so a caller can report the proof alongside the conclusions it justifies.
    pub fn termination(&self) -> &ChaseTermination {
        &self.termination
    }
}

/// Run the restricted existential chase of `program` over the seeded store `edb`.
///
/// The store is consumed and returned closed inside [`ChaseOutcome`], so the universal model
/// and its witnesses stay one value.
///
/// # What it does, in order
///
/// 1. [`certify`] the clause set. An [`Uncertified`](ChaseTermination::Uncertified) verdict is
///    refused as [`ChaseError::Termination`] — the ONLY programs this function rejects
///    for termination are the ones the analysis actually failed to certify.
/// 2. Refuse every head form and body shape the chase has no semantics for; see
///    [`ChaseError`] for the complete list and the reason attached to each.
/// 3. Run the fixpoint. Each round joins every fireable clause's body against the whole
///    accumulated store; for each body solution the head disjunct is run as a conjunctive
///    query with the existentials FREE and the firing is SKIPPED if it already has a
///    solution (the restricted-chase blocking condition — without it the fixpoint does not
///    converge); otherwise one witness is minted per existential, addressed on the frontier
///    binding, and EVERY atom of the disjunct is grounded under the same extended binding,
///    so `∃y. (r(x, y) ∧ C(y))` shares one witness across both atoms. The round's facts are
///    committed in total lexical order and the chase stops when a round commits nothing.
///
/// # Budgets
///
/// Consumption uses [`EvalOptions::default`], with no logical stored-fact,
/// join-step or term-arena ceiling. [`chase_with`] takes the caller's options;
/// any selected finite ceiling is charged as in [`crate::seminaive::evaluate`].
/// Passing a ceiling or overflowing a work counter returns
/// [`ChaseError::BudgetExhausted`] with an accurate report, never a truncated answer.
///
/// # Errors
///
/// Any [`ChaseError`].
pub fn chase<const N: usize>(
    program: &[DlClause<N>],
    edb: RelationStore<TermInterner, N>,
) -> Result<ChaseOutcome<N>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    chase_until(program, edb, None)
}

/// [`chase`], polling a caller-owned [`StopSignal`] once per chase round.
///
/// Identical to [`chase`] but for the poll: if `stop` reports stopped at a round boundary
/// the fixpoint refuses with [`ChaseError::Stopped`] rather than running the next round. It
/// is not a budget — see [`crate::stop`] — and it cannot change an answer, because the only
/// two outcomes are the answer [`chase`] would have given and no answer at all.
///
/// # Errors
///
/// [`ChaseError::Stopped`] if `stop` fired, and every error [`chase`] returns.
pub fn chase_until<const N: usize>(
    program: &[DlClause<N>],
    edb: RelationStore<TermInterner, N>,
    stop: Option<&dyn StopSignal>,
) -> Result<ChaseOutcome<N>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    chase_with(program, edb, &EvalOptions::default(), stop)
}

/// The original [`chase_with`] with sourced full-IR never-derive admission.
/// # Errors
/// Returns exact protected/undecidable head refusal before the ordinary
/// executor's fragment checks, then the same errors as [`chase_with`].
pub fn chase_with_declarations<const N: usize>(
    program: &[DlClause<N>],
    edb: RelationStore<TermInterner, N>,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
    declarations: &crate::admission::NeverDeriveDeclarations,
) -> Result<ChaseOutcome<N>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    declarations
        .admit(program)
        .map_err(ChaseError::NeverDerive)?;
    chase_with(program, edb, options, stop)
}

/// [`chase_until`] under the caller's stored-fact and join-step limits
/// ([`EvalOptions::with_max_stored_facts`], [`EvalOptions::with_max_join_steps`]).
///
/// The limits can only refuse: a run inside them returns exactly what [`chase`] returns
/// for the same program and store, and a run past one returns
/// [`ChaseError::BudgetExhausted`] naming it. The term limits of `options` have no effect
/// here: the chase refuses a guarded clause, so no term it commits is computed by a guard.
///
/// # Errors
///
/// Every error [`chase_until`] returns.
pub fn chase_with<const N: usize>(
    program: &[DlClause<N>],
    edb: RelationStore<TermInterner, N>,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<ChaseOutcome<N>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    let prepared = certify_with(program, Resident, AnalysisLimits::default(), stop)?.prepare(
        program,
        AnalysisLimits::default(),
        stop,
    )?;
    let outcome = prepared.chase_with(edb, options, stop);
    prepared.close()?;
    outcome
}

/// A restricted chase paused between rounds or ready to start.
///
/// The original store, immutable checked program, committed witnesses and
/// cumulative work remain bound to this session. It cannot be rebound to
/// another source or expose a complete model before reaching its fixpoint.
pub struct ChaseSession<'p, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    program: &'p [DlClause<N>],
    termination: ChaseTermination,
    firings: Vec<Firing>,
    // These constants reserve names without changing the logical seed budget.
    witness_literals: Vec<&'p str>,
    state: ChaseState<N>,
    round: Option<Box<RoundFrame<N>>>,
}

impl<const N: usize> fmt::Debug for ChaseSession<'_, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChaseSession")
            .field("budget", &self.budget())
            .finish_non_exhaustive()
    }
}

/// An advanced chase either reached its fixpoint or retained its state at a
/// caller-requested stop before starting another round.
#[derive(Debug)]
pub enum ChaseProgress<'p, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The complete result and its original termination evidence.
    Complete(ChaseOutcome<N>),
    /// No complete model is available. A fresh signal can advance this session.
    Paused(ChaseSession<'p, N>),
}

/// Progress under caller-replaceable logical ceilings.
#[derive(Debug)]
#[non_exhaustive]
pub enum ChaseAdvance<'p, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// The complete result and original termination certificate.
    Complete(ChaseOutcome<N>),
    /// A stop or logical ceiling retained the original execution.
    Paused {
        /// The original refusal and its observed cumulative budget.
        reason: ChaseError,
        /// The only state which can continue that execution.
        session: ChaseSession<'p, N>,
    },
}

impl<'p, const N: usize> ChaseSession<'p, N>
where
    Arity<N>: StoreShape<N>,
{
    fn start(
        program: &'p [DlClause<N>],
        edb: RelationStore<TermInterner, N>,
        options: &EvalOptions,
        termination: ChaseTermination,
        record: bool,
    ) -> Result<Self, ChaseError> {
        let firings = plan_firings(program)?;
        let witness_literals = authored_witness_literals(program)?;
        let state = ChaseState {
            store: edb,
            derivations: Vec::new(),
            proofs: record.then(Vec::new),
            witnesses: SkolemRegistry::default(),
            meter: StepMeter::new(options.max_join_steps()),
            options: *options,
        };
        if !record {
            state.check_budget()?;
        }
        Ok(Self {
            program,
            termination,
            firings,
            witness_literals,
            state,
            round: None,
        })
    }

    /// Cumulative work and stored fact and term counts at this round boundary.
    #[must_use]
    pub fn budget(&self) -> BudgetReport {
        self.state.report()
    }

    /// The committed prefix and its firing evidence, if recording was enabled.
    /// Pending heads and speculative witnesses do not appear here.
    pub fn recorded_prefix(&self) -> Option<(&[ChaseDerivation<N>], &[ChaseFiring<N>])> {
        self.state
            .proofs
            .as_deref()
            .map(|proofs| (self.state.derivations.as_slice(), proofs))
    }

    /// Whether this term of a committed conclusion is an invented witness.
    pub fn is_invented_term(&self, surface: &str) -> bool {
        self.state.witnesses.explain(surface).is_some()
    }

    /// Check the prefix against a fresh copy of the original seed without
    /// executing a join or inspecting the suspended round.
    ///
    /// # Errors
    /// Returns a missing recording, an invalid firing, or a hard native
    /// storage failure. Callers that bind a source identity must verify it
    /// before seeding this independent check.
    pub fn verify_recorded_prefix(
        &self,
        mut seed: RelationStore<TermInterner, N>,
    ) -> Result<(), ChaseProofError> {
        let (derived, proofs) = self.recorded_prefix().ok_or(ChaseProofError::NotRecorded)?;
        if derived.len() != proofs.len() {
            return Err(ChaseProofError::IncompletePrefix);
        }
        // Resident term identities are dense and never recycled. New checked
        // facts can add terms, but never turn one into an original seed term.
        let source_terms =
            u64::try_from(seed.interner().len()).map_err(|_| StorageError::SizeOverflow)?;
        let witness_literals = authored_witness_literals(self.program)?;
        for (position, (derivation, proof)) in derived.iter().zip(proofs).enumerate() {
            let Some(clause) = self.program.get(derivation.clause) else {
                return Err(ChaseProofError::InvalidFiring(position));
            };
            if !verify_firing(
                clause,
                derivation,
                proof,
                &seed,
                &self.state.witnesses,
                |surface| {
                    seed.term_id(surface)
                        .is_some_and(|id| id.index() < source_terms)
                        || witness_literals.binary_search(&surface).is_ok()
                },
            )? || seed
                .insert_row(Arity::<N>::fact_terms(&derivation.fact))?
                .is_none()
            {
                return Err(ChaseProofError::InvalidFiring(position));
            }
        }
        Ok(())
    }

    /// Continue with the original limits and a fresh caller signal if needed.
    ///
    /// The signal is polled before the next round. A pause does not replay a
    /// committed round. Budget and execution failures do not retain a session.
    ///
    /// # Errors
    /// Returns the original execution or budget failure.
    pub fn advance(
        self,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ChaseProgress<'p, N>, ChaseError> {
        advance_session(self, stop)
    }

    /// Continue this execution under the supplied cumulative ceilings.
    ///
    /// A refusal preserves the original indexed scan and pending commits. The
    /// ceilings govern all work since the initial seed, not just this call.
    /// # Errors
    /// An operational failure returns the original cause and no continuation.
    pub fn advance_with(
        mut self,
        options: &EvalOptions,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ChaseAdvance<'p, N>, ChaseError> {
        self.state.options = *options;
        self.state.meter.limit = options.max_join_steps();
        drive_chase(self, stop, true)
    }
}

fn advance_session<'p, const N: usize>(
    session: ChaseSession<'p, N>,
    stop: Option<&dyn StopSignal>,
) -> Result<ChaseProgress<'p, N>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    match drive_chase(session, stop, false)? {
        ChaseAdvance::Complete(outcome) => Ok(ChaseProgress::Complete(outcome)),
        ChaseAdvance::Paused {
            reason: ChaseError::Stopped { .. },
            session,
        } => Ok(ChaseProgress::Paused(session)),
        ChaseAdvance::Paused { reason, .. } => Err(reason),
    }
}

fn drive_chase<'p, const N: usize>(
    mut session: ChaseSession<'p, N>,
    stop: Option<&dyn StopSignal>,
    retain_budget: bool,
) -> Result<ChaseAdvance<'p, N>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    loop {
        if let Err(reason) = session.state.check_budget() {
            return if retain_budget {
                Ok(ChaseAdvance::Paused { reason, session })
            } else {
                Err(reason)
            };
        }
        if is_stopped(stop) {
            let reason = ChaseError::Stopped {
                report: session.state.report(),
            };
            return Ok(ChaseAdvance::Paused { reason, session });
        }
        let mut round = session
            .round
            .take()
            .map_or_else(RoundFrame::new, |saved| *saved);
        match round.advance(
            session.program,
            &session.firings,
            &session.witness_literals,
            &mut session.state,
        ) {
            Ok(RoundDone::Fixpoint) => break,
            Ok(RoundDone::Committed) => {}
            Err(reason @ ChaseError::BudgetExhausted { .. }) if retain_budget => {
                // Only a mid-round refusal allocates an owned suspended frame.
                session.round = Some(Memory::new(&mut Resident).boxed(round)?);
                return Ok(ChaseAdvance::Paused { reason, session });
            }
            Err(reason) => return Err(reason),
        }
    }

    let state = session.state;
    Ok(ChaseAdvance::Complete(ChaseOutcome {
        budget: state.report(),
        facts: state.store,
        // The two accessors retain their original committed ordering.
        derived: state
            .derivations
            .iter()
            .map(|derivation| derivation.fact.clone())
            .collect(),
        derivations: state.derivations,
        witnesses: state.witnesses,
        termination: session.termination,
    }))
}

enum RoundDone {
    Fixpoint,
    Committed,
}

struct HeadProbe<const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    frontier: Binding,
    join: JoinFrame<N>,
    solution: Option<Binding>,
}

type PendingHead<const N: usize> = (usize, Option<ChaseFiring<N>>);
type PendingHeads<const N: usize> = BTreeMap<ShapeFact<N>, PendingHead<N>>;
type PendingIter<const N: usize> =
    std::iter::Peekable<std::collections::btree_map::IntoIter<ShapeFact<N>, PendingHead<N>>>;

enum RoundPhase<const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    Body(JoinFrame<N>),
    Heads {
        solutions: std::vec::IntoIter<Binding>,
        probe: Option<HeadProbe<N>>,
    },
    Commit {
        heads: PendingIter<N>,
        changed: bool,
    },
}

/// Owns one round's joins, lexical winner map, and commit coordinate.
struct RoundFrame<const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    firing: usize,
    phase: RoundPhase<N>,
    pending: PendingHeads<N>,
}

impl<const N: usize> RoundFrame<N>
where
    Arity<N>: StoreShape<N>,
{
    fn new() -> Self {
        Self {
            firing: 0,
            phase: RoundPhase::Body(JoinFrame::new(&Binding::new())),
            pending: BTreeMap::new(),
        }
    }

    fn advance(
        &mut self,
        program: &[DlClause<N>],
        firings: &[Firing],
        witness_literals: &[&str],
        state: &mut ChaseState<N>,
    ) -> Result<RoundDone, ChaseError> {
        loop {
            match &mut self.phase {
                RoundPhase::Body(join) => {
                    let Some(firing) = firings.get(self.firing) else {
                        self.phase = RoundPhase::Commit {
                            heads: core::mem::take(&mut self.pending).into_iter().peekable(),
                            changed: false,
                        };
                        continue;
                    };
                    if !join.advance(
                        program[firing.clause].body(),
                        &state.store,
                        &mut state.meter,
                    ) {
                        return Err(state.join_refusal());
                    }
                    self.phase = RoundPhase::Heads {
                        solutions: core::mem::take(&mut join.solutions).into_iter(),
                        probe: None,
                    };
                }
                RoundPhase::Heads { solutions, probe } => {
                    let firing = &firings[self.firing];
                    let clause = &program[firing.clause];
                    let disjunct = firing.disjunct(clause);
                    if probe.is_none() {
                        let Some(solution) = solutions.next() else {
                            self.firing += 1;
                            self.phase = RoundPhase::Body(JoinFrame::new(&Binding::new()));
                            continue;
                        };
                        // Existentials stay free in the head satisfaction probe.
                        let frontier = restrict(&solution, &firing.frontier);
                        let join = JoinFrame::new(&frontier);
                        *probe = Some(HeadProbe {
                            frontier,
                            join,
                            solution: state.proofs.is_some().then_some(solution),
                        });
                    }
                    let active = probe.as_mut().expect("the head probe was selected");
                    if !active
                        .join
                        .advance(disjunct.atoms(), &state.store, &mut state.meter)
                    {
                        return Err(state.join_refusal());
                    }
                    let active = probe.take().expect("the completed head probe is retained");
                    if active.join.solutions.is_empty() {
                        stage_firing(
                            firing,
                            clause,
                            active,
                            &mut self.pending,
                            state,
                            witness_literals,
                        )?;
                    }
                }
                RoundPhase::Commit { heads, changed } => {
                    while let Some((fact, _)) = heads.peek() {
                        let row = Arity::<N>::fact_terms(fact);
                        // Each staged key is unique, so a previous commit
                        // cannot change membership for this key.
                        if state.store.contains_row(row) {
                            heads.next();
                            continue;
                        }
                        state.report().admit_fact().map_err(|report| {
                            ChaseError::BudgetExhausted {
                                resource: BudgetResource::StoredFacts,
                                report,
                            }
                        })?;
                        let mut resident = Resident;
                        let mut memory = Memory::new(&mut resident);
                        if let Some(proofs) = &mut state.proofs {
                            memory.reserve_for_push(proofs)?;
                        }
                        memory.reserve_for_push(&mut state.derivations)?;
                        state.store.insert_row(row)?;
                        let (fact, (clause, proof)) =
                            heads.next().expect("the admitted derivation was peeked");
                        state.derivations.push(ChaseDerivation { fact, clause });
                        if let Some(proofs) = &mut state.proofs {
                            proofs.push(proof.expect("recorded heads carry their firing evidence"));
                        }
                        *changed = true;
                    }
                    return Ok(if *changed {
                        RoundDone::Committed
                    } else {
                        RoundDone::Fixpoint
                    });
                }
            }
        }
    }
}

fn stage_firing<const N: usize>(
    firing: &Firing,
    clause: &DlClause<N>,
    active: HeadProbe<N>,
    pending: &mut PendingHeads<N>,
    state: &mut ChaseState<N>,
    witness_literals: &[&str],
) -> Result<(), ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    let mut extended = active.frontier;
    let frontier: Vec<String> = firing
        .frontier
        .iter()
        .map(|name| {
            extended
                .get(name)
                .cloned()
                .expect("a frontier variable is bound by the body join")
        })
        .collect();
    for (ordinal, existential) in clause.existentials().iter().enumerate() {
        let witness = state.witnesses.mint(
            WitnessAddress {
                clause: firing.clause,
                ordinal,
                frontier: frontier.clone(),
                salt: 0,
            },
            |surface| {
                state.store.term_id(surface).is_some()
                    || witness_literals.binary_search(&surface).is_ok()
            },
        )?;
        extended.insert(existential.clone(), witness);
    }
    // First authored producer wins; lexical fact order determines commit order.
    let mut premises = None;
    for (head, atom) in firing.disjunct(clause).atoms().iter().enumerate() {
        if let std::collections::btree_map::Entry::Vacant(entry) =
            pending.entry(ground(atom, &extended))
        {
            let proof = match &active.solution {
                Some(solution) => {
                    let premises = match &premises {
                        Some(record) => record,
                        None => premises.insert(record_premises(clause, solution)?),
                    };
                    Some(ChaseFiring {
                        head,
                        premises: premises
                            .try_clone()
                            .map_err(|_| ChaseError::Storage(StorageError::SizeOverflow))?,
                    })
                }
                None => None,
            };
            entry.insert((firing.clause, proof));
        }
    }
    Ok(())
}

fn record_premises<const N: usize>(
    clause: &DlClause<N>,
    solution: &Binding,
) -> Result<Shared<Vec<ShapeFact<N>>>, ChaseError>
where
    Arity<N>: StoreShape<N>,
{
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut premises = Vec::new();
    memory
        .reserve(&mut premises, clause.body().len())
        .map_err(ChaseError::Storage)?;
    for atom in clause.body() {
        let mut surfaces = std::array::from_fn(|_| String::new());
        for (output, term) in surfaces.iter_mut().zip(atom.terms()) {
            *output = match term {
                ClauseTerm::Var(name) => memory.string(&solution[name]),
                constant => constant
                    .surface_in(&mut memory)
                    .map(|surface| surface.expect("an authored constant has a surface")),
            }
            .map_err(ChaseError::Storage)?;
        }
        premises.push(Arity::<N>::fact(surfaces));
    }
    Shared::try_new(premises).map_err(|_| ChaseError::Storage(StorageError::AllocationFailed))
}

fn verify_firing<const N: usize>(
    clause: &DlClause<N>,
    derivation: &ChaseDerivation<N>,
    proof: &ChaseFiring<N>,
    seed: &RelationStore<TermInterner, N>,
    witnesses: &SkolemRegistry,
    source_contains: impl Fn(&str) -> bool,
) -> Result<bool, StorageError>
where
    Arity<N>: StoreShape<N>,
{
    let [head] = clause.head_disjuncts() else {
        return Ok(false);
    };
    let Some(head) = head.atoms().get(proof.head) else {
        return Ok(false);
    };
    if clause.body().len() != proof.premises().len() {
        return Ok(false);
    }
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut binding: Vec<(&str, &str)> = Vec::new();
    for (atom, fact) in clause.body().iter().zip(proof.premises()) {
        let surfaces = Arity::<N>::fact_terms(fact);
        if !seed.contains_row(surfaces) {
            return Ok(false);
        }
        for (term, value) in atom.terms().into_iter().zip(surfaces) {
            if let ClauseTerm::Var(name) = term {
                match binding.iter().find(|(variable, _)| *variable == name) {
                    Some((_, prior)) if *prior != value => return Ok(false),
                    Some(_) => {}
                    None => memory.push(&mut binding, (name, value))?,
                }
            } else if term.surface_in(&mut memory)?.as_deref() != Some(value) {
                return Ok(false);
            }
        }
    }
    let bound = |name: &str| {
        binding
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
    };
    for (term, value) in head
        .terms()
        .into_iter()
        .zip(Arity::<N>::fact_terms(&derivation.fact))
    {
        let matches = match term {
            ClauseTerm::Var(name) => match clause.existentials().iter().position(|var| var == name)
            {
                None => bound(name) == Some(value),
                Some(ordinal) => {
                    let Some(address) = witnesses.explain(value) else {
                        return Ok(false);
                    };
                    if source_contains(value) {
                        return Ok(false);
                    }
                    let mut frontier = Vec::new();
                    for variable in clause.head_atoms().flat_map(ClauseAtom::terms) {
                        if let ClauseTerm::Var(variable) = variable
                            && let Some(value) = bound(variable)
                            && !clause.existentials().contains(variable)
                        {
                            memory.push(&mut frontier, (variable.as_str(), value))?;
                        }
                    }
                    frontier.sort_unstable();
                    frontier.dedup();
                    address.clause == derivation.clause
                        && address.ordinal == ordinal
                        && address
                            .frontier
                            .iter()
                            .map(String::as_str)
                            .eq(frontier.iter().map(|(_, value)| *value))
                        && matches_witness_surface(address, value)
                }
            },
            constant => constant.surface_in(&mut memory)?.as_deref() == Some(value),
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(true)
}

fn authored_witness_literals<const N: usize>(
    program: &[DlClause<N>],
) -> Result<Vec<&str>, StorageError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut reserved = Vec::new();
    for term in program
        .iter()
        .flat_map(|clause| clause.head_atoms().chain(clause.body()))
        .flat_map(ClauseAtom::terms)
    {
        // Verbatim literals can spell a blank surface. IRI and default-graph
        // constants cannot, and variables get their names from an admitted row.
        if let ClauseTerm::Literal(surface) = term
            && surface.starts_with("_:")
        {
            memory.push(&mut reserved, surface.as_str())?;
        }
    }
    reserved.sort_unstable();
    reserved.dedup();
    Ok(reserved)
}

fn matches_witness_surface(address: &WitnessAddress, surface: &str) -> bool {
    let Some(label) = surface
        .strip_prefix("_:")
        .and_then(|suffix| suffix.strip_prefix(WITNESS_SCOPE))
        .and_then(|suffix| suffix.strip_prefix('.'))
        .and_then(|label| label.strip_prefix(WITNESS_LABEL_PREFIX))
    else {
        return false;
    };
    let mut decoded = [0; WITNESS_LABEL_BYTES];
    purrdf_hash::hex::decode_canonical_to_slice(label, &mut decoded) == Ok(decoded.len())
        && decoded == witness_digest(address).as_bytes()[..WITNESS_LABEL_BYTES]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `rdf:type`-shaped predicate a concept assertion lowers through. PurRDF mints no
    /// vocabulary, so the fixture names its own, under `example.org` like every other
    /// fixture IRI.
    const TYPE: &str = "https://example.org/type";
    const P: &str = "https://example.org/p";
    const R: &str = "https://example.org/r";
    const A: &str = "https://example.org/A";
    const CONCEPT_C: &str = "https://example.org/C";
    const CONCEPT_D: &str = "https://example.org/D";

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    /// The lexical surface an IRI is stored under.
    fn iri(name: &str) -> String {
        format!("<{name}>")
    }

    /// `C(?var)` as the quad `?var TYPE <C>` in the default graph.
    fn concept(var: &str, concept_iri: &str) -> ClauseAtom {
        ClauseAtom::positive(v(var), TYPE, ClauseTerm::iri(concept_iri))
    }

    /// `p(?subject, ?object)` in the default graph.
    fn role(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(v(subject), predicate, v(object))
    }

    /// A store seeded with `(subject, predicate, object)` triples in the default graph.
    fn store_of(triples: &[(&str, &str, &str)]) -> RelationStore {
        let mut store = RelationStore::new();
        for &(subject, predicate, object) in triples {
            store
                .insert(
                    &iri(subject),
                    &iri(predicate),
                    &iri(object),
                    RelationStore::DEFAULT_GRAPH,
                )
                .unwrap();
        }
        store
    }

    /// `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )` — the canonical `A ⊑ ∃r.C` restriction.
    fn restriction() -> DlClause {
        DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", R, "?y"),
                concept("?y", CONCEPT_C),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", A)],
        )
    }

    /// The facts of `outcome`'s closed store, as sorted `(s, p, o, g)` surface tuples.
    fn closure(outcome: &ChaseOutcome) -> Vec<(String, String, String, String)> {
        outcome
            .facts()
            .facts_sorted()
            .into_iter()
            .map(|fact| (fact.subject, fact.predicate, fact.object, fact.graph))
            .collect()
    }

    /// The chase of `program` over `edb`, asserting it ran.
    fn run(program: &[DlClause], edb: RelationStore) -> ChaseOutcome {
        chase(program, edb).expect("the fixture program is certified and fireable")
    }

    // ── The caller's stop signal ────────────────────────────────────────────────

    /// A stop signal fixed at one answer, honouring the latching contract trivially.
    #[derive(Debug)]
    struct Fixed(bool);

    impl StopSignal for Fixed {
        fn stopped(&self) -> bool {
            self.0
        }
    }

    /// A latching signal that permits `remaining` polls before firing.
    #[derive(Debug)]
    struct AfterNPolls(std::sync::atomic::AtomicU64);

    impl StopSignal for AfterNPolls {
        fn stopped(&self) -> bool {
            use std::sync::atomic::Ordering;
            // `try_update`, the non-deprecated name a newer toolchain prefers for this
            // API, postdates this workspace's MSRV floor, so the original `fetch_update`
            // name is kept and its deprecation suppressed at this single call site.
            #[allow(deprecated)]
            let previous = self
                .0
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
                    Some(left.saturating_sub(1))
                })
                .unwrap_or(0);
            previous == 0
        }
    }

    /// A SIGNAL THAT NEVER FIRES CHANGES NOTHING the chase produces — the same universal
    /// model, the same witnesses, the same budget.
    ///
    /// The property that makes a stop signal admissible where a caller-supplied ceiling is
    /// refused: it is observationally nothing until it fires. A minted witness is addressed
    /// by a content digest, so "the same witnesses" is a byte comparison rather than a shape
    /// comparison.
    #[test]
    fn an_unfired_stop_signal_changes_no_universal_model() {
        let program = [restriction()];
        let edb = || store_of(&[("a", TYPE, A), ("b", TYPE, A)]);
        let plain = chase(&program, edb()).expect("the fixture is certified");
        let watched =
            chase_until(&program, edb(), Some(&Fixed(false))).expect("the fixture is certified");
        assert_eq!(plain.derived(), watched.derived());
        assert_eq!(
            plain.witnesses().witnesses().collect::<Vec<_>>(),
            watched.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert_eq!(
            format!("{:?}", plain.budget()),
            format!("{:?}", watched.budget())
        );
    }

    /// A SIGNAL THAT FIRES PRODUCES NO MODEL AND NO WITNESS.
    ///
    /// A stopped chase is a refusal, not a smaller universal model: there is no
    /// [`ChaseOutcome`] to read a witness out of, which is what keeps a minted existential
    /// from reaching a caller through a run that was cut short.
    #[test]
    fn a_fired_stop_signal_refuses_and_mints_nothing_a_caller_can_see() {
        let program = [restriction()];
        let error = chase_until(
            &program,
            store_of(&[("a", TYPE, A), ("b", TYPE, A)]),
            Some(&Fixed(true)),
        )
        .expect_err("a signal that is already firing must stop the chase");
        assert!(matches!(
            error,
            ChaseError::Termination(TerminationError::Stopped { work: 0 })
        ));
    }

    #[test]
    fn a_mid_fixpoint_stop_refuses_after_recording_committed_work() {
        let program = [restriction()];
        let signal = AfterNPolls(std::sync::atomic::AtomicU64::new(1));
        let prepared = certify(&program)
            .unwrap()
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap();
        let error = prepared
            .chase_with(
                store_of(&[("a", TYPE, A), ("b", TYPE, A)]),
                &EvalOptions::default(),
                Some(&signal),
            )
            .expect_err("the second round boundary must observe the latched stop");
        let ChaseError::Stopped { report } = error else {
            panic!("a mid-fixpoint stop must use the typed stopped channel");
        };
        assert!(
            report.join_steps() > 0,
            "the first admitted round must have recorded work before the refusal"
        );
    }

    /// A stop between rounds preserves the same witnesses, derivations and
    /// work as completing the same existential chain without interruption.
    #[test]
    fn stopped_chase_resumes_without_replaying_committed_rounds() {
        let program = [
            restriction(),
            DlClause::new(
                vec![HeadDisjunct::new(vec![concept("?z", CONCEPT_D)])],
                Vec::new(),
                vec![concept("?z", CONCEPT_C)],
            ),
        ];
        let edb = || store_of(&[("a", TYPE, A), ("b", TYPE, A)]);
        let options = EvalOptions::default();
        let uninterrupted = chase_with(&program, edb(), &options, None).unwrap();
        let prepared = certify(&program)
            .unwrap()
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap();
        let session = prepared.start_session(edb(), &options).unwrap();
        prepared.close().unwrap();
        let stop = AfterNPolls(std::sync::atomic::AtomicU64::new(1));
        let ChaseProgress::Paused(paused) = session.advance(Some(&stop)).unwrap() else {
            panic!("the second round boundary must retain a continuation");
        };
        let partial = paused.budget();
        assert_eq!(partial.stored_facts(), 6);
        assert!(partial.join_steps() > 0);
        let ChaseProgress::Paused(paused) = paused.advance(Some(&Fixed(true))).unwrap() else {
            panic!("an already-fired replacement signal must not start another round");
        };
        assert_eq!(paused.budget(), partial);
        let ChaseProgress::Complete(resumed) = paused.advance(None).unwrap() else {
            panic!("an unstopped certified chase reaches its fixpoint");
        };
        assert_eq!(resumed.budget().stored_facts(), 8);
        assert_eq!(closure(&resumed), closure(&uninterrupted));
        assert_eq!(resumed.derivations(), uninterrupted.derivations());
        assert!(
            resumed
                .witnesses()
                .witnesses()
                .eq(uninterrupted.witnesses().witnesses())
        );
        assert_eq!(resumed.termination(), uninterrupted.termination());
        assert_eq!(resumed.budget(), uninterrupted.budget());
    }

    /// A mid-join and a mid-commit refusal keep the same work and attribution.
    #[test]
    fn cumulative_chase_limits_resume_without_replaying_a_round() {
        let program = [restriction()];
        let edb = || store_of(&[("a", TYPE, A), ("b", TYPE, A)]);
        let unlimited = EvalOptions::default();
        let expected = chase_with(&program, edb(), &unlimited, None).unwrap();
        let prepared = certify(&program)
            .unwrap()
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap();
        let join_limit = unlimited.with_max_join_steps(1);
        let session = prepared.start_recorded_session(edb(), &join_limit).unwrap();
        prepared.close().unwrap();

        let ChaseAdvance::Paused {
            reason:
                ChaseError::BudgetExhausted {
                    resource: BudgetResource::JoinSteps,
                    report: join_report,
                },
            session,
        } = session.advance_with(&join_limit, None).unwrap()
        else {
            panic!("the unfinished body join is retained on fuel refusal");
        };
        assert_eq!(join_report.join_steps(), 2);
        assert_eq!(join_report.stored_facts(), 2);

        let fact_limit = unlimited.with_max_stored_facts(3);
        let ChaseAdvance::Paused {
            reason:
                ChaseError::BudgetExhausted {
                    resource: BudgetResource::StoredFacts,
                    report: fact_report,
                },
            session,
        } = session.advance_with(&fact_limit, None).unwrap()
        else {
            panic!("the uncommitted lexical suffix is retained on fact refusal");
        };
        assert_eq!(fact_report.stored_facts(), 3);
        let (committed, proofs) = session.recorded_prefix().unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(proofs[0].premises().len(), 1);
        session.verify_recorded_prefix(edb()).unwrap();
        assert_eq!(
            session.verify_recorded_prefix(RelationStore::new()),
            Err(ChaseProofError::InvalidFiring(0)),
        );

        let ChaseAdvance::Complete(actual) = session.advance_with(&unlimited, None).unwrap() else {
            panic!("the original chase reaches its fixpoint after raising both limits");
        };
        assert_eq!(closure(&actual), closure(&expected));
        assert_eq!(actual.derivations(), expected.derivations());
        assert_eq!(actual.budget(), expected.budget());
        assert!(
            actual
                .witnesses()
                .witnesses()
                .eq(expected.witnesses().witnesses())
        );
    }

    /// An authored blank can spell the original digest without earning its
    /// existential meaning. Do not join facts about it to invented facts.
    #[test]
    fn an_authored_blank_cannot_impersonate_a_recorded_witness() {
        let program = vec![
            restriction(),
            DlClause::datalog(
                concept("?x", CONCEPT_D),
                vec![role("?x", R, "?y"), concept("?y", P)],
            ),
        ];
        let occupied = witness_surface(&WitnessAddress {
            clause: 0,
            ordinal: 0,
            frontier: vec![iri("a")],
            salt: 0,
        })
        .unwrap();
        for authored_in_program in [false, true] {
            let mut program = program.clone();
            if authored_in_program {
                program.push(DlClause::datalog(
                    ClauseAtom::positive(
                        ClauseTerm::literal(occupied.clone()),
                        TYPE,
                        ClauseTerm::iri(P),
                    ),
                    Vec::new(),
                ));
            }
            let seed = || {
                let mut store = store_of(&[("a", TYPE, A)]);
                if !authored_in_program {
                    store
                        .insert(&occupied, &iri(TYPE), &iri(P), RelationStore::DEFAULT_GRAPH)
                        .unwrap();
                }
                store
            };
            let options = EvalOptions::default();
            let prepared = certify(&program)
                .unwrap()
                .prepare(&program, AnalysisLimits::default(), None)
                .unwrap();
            let session = prepared.start_recorded_session(seed(), &options).unwrap();
            prepared.close().unwrap();
            let signal = AfterNPolls(std::sync::atomic::AtomicU64::new(1));
            let ChaseAdvance::Paused { session, .. } =
                session.advance_with(&options, Some(&signal)).unwrap()
            else {
                panic!("the witness should have committed in the first round");
            };
            session.verify_recorded_prefix(seed()).unwrap();
            let witness = session.state.witnesses.witnesses().next().unwrap();
            assert_ne!(witness, occupied);
            assert_eq!(session.state.witnesses.explain(witness).unwrap().salt(), 1);
            let mut forged_seed = seed();
            forged_seed
                .insert(witness, &iri(TYPE), &iri(P), RelationStore::DEFAULT_GRAPH)
                .unwrap();
            assert!(matches!(
                session.verify_recorded_prefix(forged_seed),
                Err(ChaseProofError::InvalidFiring(_))
            ));
            let ChaseAdvance::Complete(outcome) = session.advance_with(&options, None).unwrap()
            else {
                panic!("the witnessed restriction closes");
            };
            assert!(!outcome.facts().contains(
                &iri("a"),
                &iri(TYPE),
                &iri(CONCEPT_D),
                RelationStore::DEFAULT_GRAPH
            ));
        }
    }

    /// A store that cannot mint the next term must not publish a partial result.
    #[test]
    fn recorded_chase_does_not_pause_on_store_capacity_failure() {
        let program = [restriction()];
        let prepared = certify(&program)
            .unwrap()
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap();
        let mut store = store_of(&[("a", TYPE, A)]);
        store.limit_terms_for_test(Some(u64::try_from(store.interner().len()).unwrap()));
        let options = EvalOptions::default();
        let session = prepared.start_recorded_session(store, &options).unwrap();
        prepared.close().unwrap();
        assert!(matches!(
            session.advance_with(&options, None),
            Err(ChaseError::Storage(StorageError::Capacity(_)))
        ));
    }

    // ── The caller's stored-fact and join-step limits ─────────────────────────

    /// The chase honours the caller's limits exactly as the semi-naive evaluator does: a
    /// stored-fact limit of exactly the universal model's size admits it — the same model
    /// [`chase`] returns — one fewer refuses it naming the knob, and a join-step limit the
    /// run passes refuses it naming that knob.
    #[test]
    fn the_callers_limits_refuse_exactly_and_admit_the_same_model() {
        let program = [restriction()];
        let edb = || store_of(&[("a", TYPE, A), ("b", TYPE, A)]);
        let unlimited = run(&program, edb());
        let size = unlimited.facts().row_count() as u64;
        assert_eq!(size, 6, "two seeds, two witnesses, two head atoms each");

        let exact = EvalOptions::default().with_max_stored_facts(size);
        let admitted = chase_with(&program, edb(), &exact, None).expect("exactly the model");
        assert_eq!(closure(&admitted), closure(&unlimited));
        assert_eq!(admitted.budget().stored_fact_limit(), size);
        assert!(admitted.budget().stored_fact_limit_stated());

        let short = EvalOptions::default().with_max_stored_facts(size - 1);
        let refused = chase_with(&program, edb(), &short, None).expect_err("one fact short");
        assert!(matches!(
            refused,
            ChaseError::BudgetExhausted {
                resource: BudgetResource::StoredFacts,
                ..
            }
        ));
        assert_eq!(
            refused.to_string(),
            "the chase did not complete: evaluation exceeded the stored-fact limit: 6 facts \
             observed, 5 permitted (the caller's limit); raise it with \
             EvalOptions::with_max_stored_facts"
        );

        let steps = EvalOptions::default().with_max_join_steps(1);
        let refused = chase_with(&program, edb(), &steps, None).expect_err("one step");
        assert!(
            refused.to_string().ends_with(
                "1 permitted (the caller's limit); raise it with EvalOptions::with_max_join_steps"
            ),
            "{refused}"
        );
        let steps = EvalOptions::default().with_max_join_steps(unlimited.budget().join_steps());
        assert_eq!(
            closure(&chase_with(&program, edb(), &steps, None).expect("exactly its steps")),
            closure(&unlimited)
        );
        // The last representable candidate is admitted; a further one must refuse
        // before the chase can commit a result, even with an unstated ceiling.
        let options = EvalOptions::default();
        let mut state = ChaseState {
            store: edb(),
            derivations: Vec::new(),
            proofs: None,
            witnesses: SkolemRegistry::default(),
            meter: StepMeter::new(options.max_join_steps()),
            options,
        };
        state.meter.consumed = u64::MAX - 1;
        state.meter.charge();
        state.check_budget().expect("the full-width final credit");
        state.meter.charge();
        assert!(state.meter.spent());
        let ChaseError::BudgetExhausted { resource, report } =
            state.check_budget().expect_err("unrepresentable join work")
        else {
            panic!("counter overflow must use the typed budget refusal");
        };
        assert_eq!(resource, BudgetResource::JoinSteps);
        assert_eq!(report.join_steps(), u64::MAX);
        assert_eq!(report.join_step_limit(), u64::MAX);
        assert_eq!(state.store.row_count(), 2);
        assert_eq!(state.derivations, []);
    }

    // ── The restriction fires, terminates and mints blank nodes ─────────────────

    /// `A ⊑ ∃r.C` over two individuals terminates, mints exactly ONE witness per distinct
    /// `?x`, asserts BOTH head atoms over that one witness, and every witness surface is a
    /// blank node.
    #[test]
    fn a_restriction_mints_one_blank_node_witness_per_subject() {
        let program = [restriction()];
        assert!(certify(&program).is_ok());
        let outcome = run(&program, store_of(&[("a", TYPE, A), ("b", TYPE, A)]));

        assert_eq!(
            outcome.witnesses().len(),
            2,
            "one witness per distinct frontier binding, not per head atom: {:?}",
            outcome.witnesses().witnesses().collect::<Vec<_>>()
        );
        // Two head atoms per witness.
        assert_eq!(outcome.derived().len(), 4, "{:?}", outcome.derived());

        for witness in outcome.witnesses().witnesses() {
            assert!(
                witness.starts_with("_:"),
                "a witness is a blank node, never an IRI: {witness}"
            );
            assert!(
                !witness.starts_with('<'),
                "a witness is never an IRI: {witness}"
            );
            assert!(
                witness.starts_with("_:0."),
                "a witness lives in the default blank scope: {witness}"
            );
        }

        // Each witness carries BOTH head atoms — the shared-witness property that makes
        // `A ⊑ ∃r.C` one clause rather than two.
        for subject in ["a", "b"] {
            let witness = outcome
                .derived()
                .iter()
                .find(|fact| fact.subject == iri(subject) && fact.predicate == iri(R))
                .map(|fact| fact.object.clone())
                .expect("every A individual gets an r-successor");
            assert!(outcome.facts().contains(
                &witness,
                &iri(TYPE),
                &iri(CONCEPT_C),
                RelationStore::DEFAULT_GRAPH
            ));
        }
    }

    /// A witness explains itself: the registry hands back the clause, the ordinal and the
    /// frontier VALUES it was addressed on.
    #[test]
    fn a_witness_explains_its_skolem_address() {
        let program = [restriction()];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        let witness = outcome
            .witnesses()
            .witnesses()
            .next()
            .expect("one witness")
            .to_owned();
        let address = outcome
            .witnesses()
            .explain(&witness)
            .expect("a minted witness is explainable");
        assert_eq!(address.clause(), 0);
        assert_eq!(address.ordinal(), 0);
        assert_eq!(address.frontier(), [iri("a")]);
        assert_eq!(outcome.witnesses().explain("_:0.wdeadbeef"), None);
        assert!(!outcome.witnesses().is_empty());
    }

    // ── The restricted check actually blocks ────────────────────────────────────

    /// Seeding the store with a SATISFYING `r(a, b) ∧ C(b)` blocks the firing for `a`: the
    /// obligation is already witnessed, so the restricted chase invents nothing.
    ///
    /// This is the difference between the restricted and the oblivious chase, and it is
    /// what makes the fixpoint converge.
    #[test]
    fn the_restricted_check_blocks_an_already_witnessed_obligation() {
        let program = [restriction()];
        let seeded = store_of(&[
            ("a", TYPE, A),
            ("a", R, "b"),
            ("b", TYPE, CONCEPT_C),
            ("c", TYPE, A),
        ]);
        let before = seeded.facts_sorted();
        let outcome = run(&program, seeded);

        assert!(
            outcome.witnesses().len() == 1,
            "only `c` is unwitnessed: {:?}",
            outcome.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert!(
            outcome
                .derived()
                .iter()
                .all(|fact| fact.subject == iri("c") || fact.subject.starts_with("_:")),
            "nothing was invented for `a`: {:?}",
            outcome.derived()
        );
        // The seeded facts survive untouched.
        for fact in before {
            assert!(outcome.facts().contains(
                &fact.subject,
                &fact.predicate,
                &fact.object,
                &fact.graph
            ));
        }
    }

    /// A PARTIALLY satisfied obligation is not satisfied: `r(a, b)` without `C(b)` leaves
    /// the conjunctive head unwitnessed, so the clause still fires.
    #[test]
    fn a_partially_satisfied_conjunctive_head_still_fires() {
        let program = [restriction()];
        let outcome = run(&program, store_of(&[("a", TYPE, A), ("a", R, "b")]));
        assert_eq!(outcome.witnesses().len(), 1, "{:?}", outcome.derived());
    }

    // ── Idempotence ─────────────────────────────────────────────────────────────

    /// Two runs of the same program over the same EDB produce byte-identical derived facts,
    /// byte-identical DERIVATIONS (fact and clause index alike), byte-identical witnesses
    /// and an identical closure.
    ///
    /// The two runs seed the store in OPPOSITE insertion orders, so anything that leaked
    /// mint order or arrival order into the attribution would show up here.
    #[test]
    fn two_runs_are_byte_identical() {
        let program = [restriction()];
        let first = run(&program, store_of(&[("a", TYPE, A), ("b", TYPE, A)]));
        let second = run(&program, store_of(&[("b", TYPE, A), ("a", TYPE, A)]));
        assert_eq!(first.derived(), second.derived());
        assert_eq!(first.derivations(), second.derivations());
        assert_eq!(closure(&first), closure(&second));
        assert_eq!(
            first.witnesses().witnesses().collect::<Vec<_>>(),
            second.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert_eq!(
            first.budget().stored_facts(),
            second.budget().stored_facts()
        );
    }

    /// Re-deriving the same frontier INSIDE one run reuses ONE witness.
    ///
    /// The body `A(?x) ∧ p(?x, ?z)` has two solutions for `x = a` (one per `z`), and both
    /// share the frontier `{?x = a}`. The registry is a function of the frontier, so both
    /// firings recover the same witness and the round's fact set collapses them — which is
    /// exactly why the fixpoint closes.
    #[test]
    fn one_frontier_reuses_one_witness_within_a_round() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", R, "?y"),
                concept("?y", CONCEPT_C),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", A), role("?x", P, "?z")],
        )];
        let outcome = run(
            &program,
            store_of(&[("a", TYPE, A), ("a", P, "m"), ("a", P, "n")]),
        );
        assert_eq!(
            outcome.witnesses().len(),
            1,
            "two body solutions, one frontier, one witness: {:?}",
            outcome.witnesses().witnesses().collect::<Vec<_>>()
        );
        assert_eq!(outcome.derived().len(), 2, "{:?}", outcome.derived());
    }

    /// Distinct frontier bindings mint DISTINCT witnesses — a witness is a function of the
    /// bound values, not of the clause alone.
    #[test]
    fn distinct_frontiers_mint_distinct_witnesses() {
        let program = [restriction()];
        let outcome = run(
            &program,
            store_of(&[("a", TYPE, A), ("b", TYPE, A), ("c", TYPE, A)]),
        );
        let witnesses: BTreeSet<&str> = outcome.witnesses().witnesses().collect();
        assert_eq!(witnesses.len(), 3);
    }

    // ── Termination analysis ────────────────────────────────────────────────────

    /// The classic non-terminating rule `r(?x, ?y) → ∃?z. r(?y, ?z)` is reported uncertified,
    /// and [`chase`] refuses it rather than looping.
    #[test]
    fn the_classic_existential_cycle_is_uncertified_and_refused() {
        let program = [DlClause::new(
            vec![HeadDisjunct::atom(role("?y", R, "?z"))],
            vec!["?z".to_owned()],
            vec![role("?x", R, "?y")],
        )];
        let failure = certify(&program).expect_err("WA cannot certify the existential cycle");
        let TerminationError::Uncertified(edge) = failure else {
            panic!("expected uncertified WA: {failure}");
        };
        assert_eq!(edge.from.clause, 0);
        assert_eq!(edge.to.clause, 0);
        assert_eq!(certify(&program).unwrap_err(), failure);
        let refusal = chase(&program, store_of(&[("a", R, "b")])).unwrap_err();
        assert_eq!(refusal, ChaseError::Termination(failure));
        drop(program);
        assert!(refusal.to_string().contains("uncertified"));
        assert!(!refusal.to_string().contains("non-terminating"));
    }

    /// The constant-refined `type(?x, <C>) → ∃?y. ( p(?x, ?y) ∧ type(?y, <D>) )` IS weakly
    /// acyclic.
    ///
    /// This is the case UNREFINED weak acyclicity gets wrong: without the refinement the
    /// body's `type[subject]` and the head's `type[subject]` are the same node, the
    /// existential edge is a self-loop, and every `C ⊑ ∃p.D` axiom in every ontology would
    /// be refused. It is the whole reason the refinement exists, so it is asserted here.
    #[test]
    fn a_constant_refined_restriction_is_weakly_acyclic() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", P, "?y"),
                concept("?y", CONCEPT_D),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", CONCEPT_C)],
        )];
        let termination = certify(&program).unwrap();
        assert!(
            termination.summary().is_certified(),
            "the constant refinement must separate the C-typed and D-typed positions: \
             {termination:?}"
        );
        let ChaseTermination::WeaklyAcyclic {
            positions,
            existential_edges,
        } = termination.summary()
        else {
            unreachable!("just asserted certified");
        };
        assert!(positions > 0);
        assert!(existential_edges > 0, "the analysis is not vacuous");

        // And it really runs.
        let outcome = run(&program, store_of(&[("a", TYPE, CONCEPT_C)]));
        assert_eq!(outcome.witnesses().len(), 1);
    }

    /// Swapping the head's class constant for the BODY's turns the same shape into the
    /// non-terminating `C ⊑ ∃p.C`, and the refinement no longer separates the positions —
    /// so the analysis cannot certify it. The refinement is precise, not permissive.
    #[test]
    fn a_self_typed_restriction_is_uncertified() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", P, "?y"),
                concept("?y", CONCEPT_C),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", CONCEPT_C)],
        )];
        assert!(certify(&program).is_err());
    }

    /// A program with NO existential head is trivially weakly acyclic however cyclic its
    /// Datalog recursion is: transitive closure invents nothing.
    #[test]
    fn a_recursive_datalog_program_is_trivially_weakly_acyclic() {
        let program = [DlClause::datalog(
            role("?x", R, "?z"),
            vec![role("?x", R, "?y"), role("?y", R, "?z")],
        )];
        let termination = certify(&program).unwrap().summary();
        assert_eq!(
            termination,
            ChaseTermination::WeaklyAcyclic {
                positions: termination_positions(&termination),
                existential_edges: 0,
            }
        );
        assert!(termination.to_string().contains("weakly acyclic"));
    }

    /// The position count of a certified verdict, for an assertion that does not restate
    /// the graph's size.
    fn termination_positions(termination: &ChaseTermination) -> usize {
        match termination {
            ChaseTermination::WeaklyAcyclic { positions, .. } => *positions,
            ChaseTermination::Uncertified { .. } => panic!("expected a certified verdict"),
        }
    }

    /// A WILDCARD predicate is conservatively connected to every constant-predicate position
    /// of the same slot, so a meta-rule that quantifies over the predicate cannot hide a
    /// cycle from the analysis.
    #[test]
    fn a_wildcard_predicate_is_conservatively_connected() {
        // `T(?x, ?p, ?y) → ∃?z. r(?y, ?z)`: the invented null sits at `r[object]`, and the
        // wildcard-predicate body position subsumes it, so the existential edge is in a
        // cycle even though no atom names `r` in the body.
        let program = [DlClause::new(
            vec![HeadDisjunct::atom(role("?y", R, "?z"))],
            vec!["?z".to_owned()],
            vec![ClauseAtom::quad(
                v("?x"),
                v("?p"),
                v("?y"),
                ClauseTerm::DefaultGraph,
            )],
        )];
        assert!(
            certify(&program).is_err(),
            "a variable predicate must not be treated as a fresh, unconnected relation"
        );
    }

    // ── Mixed programs: atomic and conjunctive heads fire ───────────────────────

    /// An ATOMIC head fires inside the same fixpoint as an existential one, and the
    /// existential clause consumes what the Datalog clause derived.
    #[test]
    fn an_atomic_head_fires_in_the_same_fixpoint() {
        // `C(?x) → A(?x)` and `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )`.
        let program = [
            DlClause::datalog(concept("?x", A), vec![concept("?x", CONCEPT_D)]),
            restriction(),
        ];
        assert!(certify(&program).is_ok());
        let outcome = run(&program, store_of(&[("a", TYPE, CONCEPT_D)]));
        assert!(outcome.facts().contains(
            &iri("a"),
            &iri(TYPE),
            &iri(A),
            RelationStore::DEFAULT_GRAPH
        ));
        assert_eq!(
            outcome.witnesses().len(),
            1,
            "the existential clause consumed the Datalog conclusion: {:?}",
            outcome.derived()
        );
        // The Datalog conclusion is committed in an earlier round than the witness facts.
        assert_eq!(outcome.derived()[0].predicate, iri(TYPE));
        assert_eq!(outcome.derived()[0].object, iri(A));
    }

    // ── Attribution: which clause concluded a fact ──────────────────────────────

    /// Both atoms of `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )` are credited to the SAME clause,
    /// and that clause is the one's AUTHORED position in the program that was passed in.
    ///
    /// The clause is authored at index 1 behind an unrelated clause that never fires, so a
    /// hard-coded `0` — or an index into the fireable subset rather than into the program —
    /// would fail here.
    #[test]
    fn a_conjunctive_existential_head_credits_one_clause() {
        let program = [
            // Authored FIRST and never fires: no individual is a `D`.
            DlClause::datalog(concept("?x", A), vec![concept("?x", CONCEPT_D)]),
            restriction(),
        ];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));

        assert_eq!(outcome.derivations().len(), 2, "{:?}", outcome.derived());
        for derivation in outcome.derivations() {
            assert_eq!(
                derivation.clause(),
                1,
                "both head atoms of one firing credit the authoring clause: {derivation:?}"
            );
        }
        // The two accessors describe the same sequence.
        assert_eq!(
            outcome
                .derivations()
                .iter()
                .map(ChaseDerivation::fact)
                .cloned()
                .collect::<Vec<_>>(),
            outcome.derived()
        );
    }

    /// A mixed program credits each derived fact to the RIGHT clause: the atomic clause at
    /// index 0 for its conclusion, the existential clause at index 1 for both of its head
    /// atoms.
    #[test]
    fn a_mixed_program_credits_each_fact_to_its_authoring_clause() {
        // 0: `D(?x) → A(?x)`.  1: `A(?x) → ∃?y. ( r(?x, ?y) ∧ C(?y) )`.
        let program = [
            DlClause::datalog(concept("?x", A), vec![concept("?x", CONCEPT_D)]),
            restriction(),
        ];
        let outcome = run(&program, store_of(&[("a", TYPE, CONCEPT_D)]));

        let credited: Vec<(String, String, usize)> = outcome
            .derivations()
            .iter()
            .map(|derivation| {
                (
                    derivation.fact().predicate.clone(),
                    derivation.fact().object.clone(),
                    derivation.clause(),
                )
            })
            .collect();
        assert_eq!(credited.len(), 3, "{credited:?}");

        // Round one: the Datalog conclusion `A(a)`, credited to clause 0.
        assert_eq!(credited[0], (iri(TYPE), iri(A), 0));
        // Round two: `r(a, w)` and `C(w)`, both credited to clause 1.
        let witness = outcome
            .witnesses()
            .witnesses()
            .next()
            .expect("one witness")
            .to_owned();
        let later: BTreeSet<(String, String, usize)> = credited[1..].iter().cloned().collect();
        assert_eq!(
            later,
            BTreeSet::from([(iri(R), witness, 1), (iri(TYPE), iri(CONCEPT_C), 1),]),
            "{credited:?}"
        );
    }

    /// A fact TWO clauses conclude is credited once, to the lower authored index — the
    /// attribution is a function of the program, not of which clause the loop reached first.
    #[test]
    fn a_shared_conclusion_is_credited_to_the_lower_authored_clause() {
        // Both clauses conclude `C(?x)` from `A(?x)`; only one derivation may result.
        let program = [
            DlClause::datalog(concept("?x", CONCEPT_C), vec![concept("?x", A)]),
            DlClause::datalog(concept("?x", CONCEPT_C), vec![concept("?x", A)]),
        ];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        assert_eq!(outcome.derivations().len(), 1, "{:?}", outcome.derived());
        assert_eq!(outcome.derivations()[0].clause(), 0);
        assert_eq!(outcome.derivations()[0].fact().object, iri(CONCEPT_C));
    }

    /// A CONJUNCTIVE head (`m = 1`, several conjuncts, no existential) fires: it is a
    /// zero-existential conjunctive head, and `→ p ∧ q` entails both conjuncts.
    #[test]
    fn a_conjunctive_head_fires() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                concept("?x", CONCEPT_C),
                concept("?x", CONCEPT_D),
            ])],
            Vec::new(),
            vec![concept("?x", A)],
        )];
        assert_eq!(program[0].head_form(), HeadForm::Conjunctive);
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        assert!(outcome.witnesses().is_empty(), "nothing was invented");
        assert_eq!(
            outcome
                .derived()
                .iter()
                .map(|fact| fact.object.clone())
                .collect::<Vec<_>>(),
            [iri(CONCEPT_C), iri(CONCEPT_D)]
        );
    }

    // ── Refusals ────────────────────────────────────────────────────────────────

    /// A MULTI-DISJUNCT existential head is refused by name: there is no case split here,
    /// and choosing a disjunct would assert what the program does not entail.
    #[test]
    fn a_multi_disjunct_existential_head_is_refused() {
        let program = [DlClause::new(
            vec![
                HeadDisjunct::atom(role("?x", R, "?y")),
                HeadDisjunct::atom(role("?x", P, "?y")),
            ],
            vec!["?y".to_owned()],
            vec![concept("?x", A)],
        )];
        assert_eq!(program[0].head_form(), HeadForm::Existential);
        let refusal =
            chase(&program, store_of(&[("a", TYPE, A)])).expect_err("a case split is refused");
        assert_eq!(
            refusal,
            ChaseError::DisjunctiveExistentialHead {
                clause: 0,
                disjuncts: 2
            }
        );
        assert!(refusal.to_string().contains("existential head with 2"));
    }

    /// A non-existential DISJUNCTIVE head is refused under its own name.
    #[test]
    fn a_disjunctive_head_is_refused() {
        let program = [DlClause::new(
            vec![
                HeadDisjunct::atom(concept("?x", CONCEPT_C)),
                HeadDisjunct::atom(concept("?x", CONCEPT_D)),
            ],
            Vec::new(),
            vec![concept("?x", A)],
        )];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("no least model"),
            ChaseError::DisjunctiveHead {
                clause: 0,
                disjuncts: 2
            }
        );
    }

    /// An INCONSISTENCY clause is refused: the chase returns a model, not a verdict.
    #[test]
    fn an_inconsistency_clause_is_refused() {
        let program = [DlClause::inconsistency(vec![concept("?x", A)])];
        let refusal = chase(&program, RelationStore::new()).expect_err("no model to return");
        assert_eq!(refusal, ChaseError::InconsistencyClause { clause: 0 });
        assert!(refusal.to_string().contains("empty (false) head"));
    }

    /// A NEGATED body atom is refused: NAF and value invention do not compose without a
    /// stratification argument, and there is no stratifier here.
    #[test]
    fn a_negated_body_atom_is_refused() {
        let program = [DlClause::new(
            vec![HeadDisjunct::atom(role("?x", R, "?y"))],
            vec!["?y".to_owned()],
            vec![
                concept("?x", A),
                ClauseAtom::negated(v("?x"), TYPE, ClauseTerm::iri(CONCEPT_C)),
            ],
        )];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("NAF is refused"),
            ChaseError::NegatedBodyAtom {
                clause: 0,
                body_index: 1
            }
        );
    }

    /// An UNBOUND head variable — neither existential nor body-bound — is refused rather
    /// than grounded with a fabricated term.
    #[test]
    fn an_unbound_head_variable_is_refused() {
        let program = [DlClause::new(
            vec![HeadDisjunct::new(vec![
                role("?x", R, "?y"),
                role("?w", P, "?y"),
            ])],
            vec!["?y".to_owned()],
            vec![concept("?x", A)],
        )];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("`?w` cannot be grounded"),
            ChaseError::UnboundHeadVariable {
                clause: 0,
                variable: "?w".to_owned()
            }
        );
    }

    /// The head form is decided BEFORE the body and head-variable checks, so a clause with
    /// several defects reports the most fundamental one.
    #[test]
    fn the_head_form_is_decided_first() {
        let program = [DlClause::inconsistency(vec![
            concept("?x", A),
            ClauseAtom::negated(v("?x"), TYPE, ClauseTerm::iri(CONCEPT_C)),
        ])];
        assert_eq!(
            chase(&program, RelationStore::new()).expect_err("refused"),
            ChaseError::InconsistencyClause { clause: 0 }
        );
    }

    // ── Reporting surfaces ──────────────────────────────────────────────────────

    /// A successful run carries the certificate and an accurate budget report.
    #[test]
    fn an_outcome_carries_its_certificate_and_budget() {
        let program = [restriction()];
        let outcome = run(&program, store_of(&[("a", TYPE, A)]));
        assert!(outcome.termination().is_certified());
        assert!(outcome.budget().join_steps() > 0);
        assert_eq!(
            outcome.budget().stored_facts(),
            outcome.facts().row_count(),
            "the report describes the store it was taken from"
        );
        assert_eq!(
            outcome.budget().term_arena_bytes(),
            outcome.facts().term_bytes()
        );
        let facts = outcome.facts().facts_sorted().len();
        assert_eq!(outcome.into_facts().facts_sorted().len(), facts);
    }

    /// An EMPTY program closes immediately and derives nothing — the EDB is already its own
    /// universal model.
    #[test]
    fn an_empty_program_derives_nothing() {
        let outcome = run(&[], store_of(&[("a", TYPE, A)]));
        assert_eq!(outcome.derived(), []);
        assert!(outcome.witnesses().is_empty());
        assert_eq!(outcome.facts().row_count(), 1);
        assert_eq!(
            certify::<4>(&[]).unwrap().summary(),
            ChaseTermination::WeaklyAcyclic {
                positions: 0,
                existential_edges: 0
            }
        );
    }

    /// The witness label is frozen: a moved label renames every invented blank node a
    /// chase has ever emitted.
    #[test]
    fn a_witness_surface_is_frozen() {
        let address = WitnessAddress {
            clause: 1,
            ordinal: 2,
            frontier: vec![iri("a"), iri("b")],
            salt: 0,
        };
        assert_eq!(
            witness_surface(&address).unwrap(),
            "_:0.wbbdbca6f8eab62b90b50735b5a380b76"
        );
    }

    /// The witness surface is a pure function of the address: the same address always
    /// renders to the same blank node, and any component change renders to a different one.
    #[test]
    fn a_witness_surface_is_a_function_of_its_address() {
        let base = WitnessAddress {
            clause: 0,
            ordinal: 0,
            frontier: vec![iri("a")],
            salt: 0,
        };
        assert_eq!(witness_surface(&base), witness_surface(&base.clone()));
        let surface = witness_surface(&base).unwrap();
        assert!(surface.starts_with("_:0.w"), "{surface}");
        assert_eq!(
            surface.len(),
            "_:0.w".len() + 2 * WITNESS_LABEL_BYTES,
            "{surface}"
        );

        let mut others = BTreeSet::from([surface]);
        for changed in [
            WitnessAddress {
                clause: 1,
                ..base.clone()
            },
            WitnessAddress {
                ordinal: 1,
                ..base.clone()
            },
            WitnessAddress {
                frontier: vec![iri("b")],
                ..base.clone()
            },
            WitnessAddress {
                frontier: vec![iri("a"), iri("a")],
                ..base.clone()
            },
            WitnessAddress {
                frontier: Vec::new(),
                ..base
            },
        ] {
            assert!(
                others.insert(witness_surface(&changed).unwrap()),
                "every address component reaches the digest: {changed:?}"
            );
        }
    }

    /// The registry is idempotent: minting one address twice yields one entry and one
    /// surface.
    #[test]
    fn the_registry_is_idempotent() {
        let address = WitnessAddress {
            clause: 3,
            ordinal: 1,
            frontier: vec![iri("a"), iri("b")],
            salt: 0,
        };
        let mut registry = SkolemRegistry::default();
        assert!(registry.is_empty());
        let first = registry.mint(address.clone(), |_| false).unwrap();
        let second = registry.mint(address.clone(), |_| false).unwrap();
        assert_eq!(first, second);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.explain(&first), Some(&address));
    }
}
