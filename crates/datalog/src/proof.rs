// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Independently checkable PROOF TERMS: hash-consed derivations whose checker RE-DERIVES
//! the conclusion instead of believing it.
//!
//! # A log is not a proof
//!
//! [`Derivation`](crate::seminaive::Derivation) is a log. It says "rule 3 produced `t(a, c)` from `p(a, b)` and `p(b, c)`",
//! and to believe it you have to believe the engine that wrote it: a join kernel that
//! admitted a tuple it should have rejected writes a perfectly well-formed record of its own
//! mistake. A [`ProofArena`] term is a different kind of object. Its checker takes the proof
//! and the CLAUSE PROGRAM, walks the premises to the facts they establish, matches the named
//! rule's body against those facts, instantiates the rule's head from the resulting
//! substitution, and returns **the fact it derived itself**. The proof's stated conclusion is
//! never an input to that computation — it is only ever compared against the result. A step
//! the rule does not license cannot be made to check by writing a nicer record of it.
//!
//! That is the property the later reasoning phases need. A hypertableau's soundness is not
//! observable from its output: a wrong answer and a right answer are both just answers. A
//! wrong answer carrying a proof that fails [`ProofArena::check`] is a gate failure.
//!
//! # Two constructors
//!
//! * [`ProofArena::axiom`] — a fact was GIVEN. It checks against the seeded EDB and nothing
//!   else. Critically it does NOT check against the saturated model: if it did, a proof
//!   could assert its own conclusion as an axiom and every circular "derivation" would pass.
//! * [`ProofArena::by_rule`] — a conclusion follows from premises by a named clause. It
//!   checks by re-derivation, as above.
//!
//! Nothing else is representable, so there is no third constructor through which an
//! unjustified step could enter.
//!
//! # Hash-consing
//!
//! Terms are interned: two structurally identical proofs are ONE node. A fact derived many
//! ways, or one axiom appealed to by a hundred steps, is stored once, so a proof of a
//! saturated model is linear in the derivation count rather than exponential in the
//! branching. Interning also makes structural equality an id comparison, which is what lets
//! [`check`](ProofArena::check) memoize by node and stay linear in the DAG.
//!
//! # Identity is a content digest, never an IRI
//!
//! PurRDF mints no vocabulary, so a proof term is NOT named by a fabricated derivation IRI.
//! It is identified by [`ProofArena::digest`] — BLAKE3 over the canonical
//! [`encode`](ProofArena::encode)d form. If an RDF-facing identifier is ever wanted, the
//! namespace is caller-supplied configuration and the digest is what goes in it.
//!
//! # Determinism
//!
//! Arena ids are assigned in interning order, and interning order is a function of the
//! construction sequence alone — no map iteration, no clock, no address. The SERIALIZED
//! form does not carry an arena id at all: [`encode`](ProofArena::encode) numbers nodes by
//! their position in a post-order first-visit walk of the proof DAG, so two arenas that
//! built the same proof through different sequences encode to the same bytes, on every
//! target. `decode(encode(p))` re-interns and re-encodes to the identical bytes.
//!
//! # Negation is a statement about a model
//!
//! Negation-as-failure is not derivable from a finite proof: "no such fact exists" is a
//! closed-world claim about an extension, not a step. [`ProofContext`] therefore carries the
//! saturated model alongside the seeded EDB, and a `by_rule` step's negated body atoms are
//! re-decided against it. Stratification is what makes that sound — a negated atom's
//! predicate is complete before the stratum that reads it runs, and is never extended
//! afterwards — so re-deciding against the FINAL model gives the same answer the engine had
//! and rejects an engine that decided negation too early.

use core::hash::Hasher;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use hashbrown::HashTable;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};
use crate::id::ProofId;
use crate::plan::{
    ATOM_ARITY, POSITION_GRAPH, POSITION_OBJECT, POSITION_PREDICATE, POSITION_SUBJECT,
};
use crate::seminaive::Evaluation;
use crate::store::{Bound, Fact, RelationStore};

// ── The wire format's fixed vocabulary ──────────────────────────────────────────

/// Domain-separation tag leading every [`ProofArena::encode`]d proof.
///
/// Bumped whenever the encoding changes shape, so bytes written under an older layout can
/// never be decoded as if they were current.
const PROOF_ENCODING_TAG: &str = "purrdf-datalog-proof-v1";

/// Wire kind byte: an axiom (assertion) leaf.
const KIND_AXIOM: u8 = 0;

/// Wire kind byte: a rule application.
const KIND_BY_RULE: u8 = 1;

// ── Rejection ───────────────────────────────────────────────────────────────────

/// Why a proof term is not a proof of the conclusion it states.
///
/// Every variant is a NORMAL rejection of an invalid proof, never an engine fault: a checker
/// that could not tell a bad proof from a good one would defeat the point of having one.
///
/// Every [`Fact`] payload is boxed. A rejection travels through a `Result` whose success arm
/// is one fact, and four owned surfaces per inlined fact would make the error arm several
/// times wider than the answer it displaces on every call, including the calls that succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofError {
    /// The proof is not a well-formed term: a reference outside the arena, a premise that
    /// does not precede the step it justifies, or — for a decoded proof — bytes that are
    /// truncated, mis-tagged, or carry an unknown node kind.
    Malformed {
        /// A human-readable account of the structural defect.
        detail: String,
    },
    /// An axiom leaf's fact is not in the seeded EDB.
    ///
    /// This is the variant that closes circularity: a DERIVED fact asserted as an axiom is
    /// reported here, because the EDB is the only extension an axiom may appeal to.
    NotAsserted {
        /// The fact that was claimed as given but is not seeded.
        goal: Box<Fact>,
    },
    /// A rule application names a clause index the program does not have.
    UnknownRule {
        /// The cited clause index.
        rule: usize,
    },
    /// A rule application names a clause whose head is not a single unquantified atom, so
    /// it licenses no Datalog step at all.
    NonDatalogRule {
        /// The cited clause index.
        rule: usize,
        /// The head form that has no Datalog semantics.
        form: HeadForm,
    },
    /// A rule application supplies a premise count differing from the clause's POSITIVE
    /// body arity. A negated body atom is a refutation obligation, not a premise, so it
    /// carries no subproof.
    PremiseCountMismatch {
        /// The cited clause index.
        rule: usize,
        /// The clause's positive body arity.
        body: usize,
        /// The number of premises supplied.
        premises: usize,
    },
    /// A positive body atom does not match the fact its premise established, under the
    /// substitution the earlier premises forced.
    PremiseMismatch {
        /// The cited clause index.
        rule: usize,
        /// The unmatched atom's index in the clause's AUTHORED body.
        body_position: usize,
        /// The fact the premise actually proved.
        proven: Box<Fact>,
    },
    /// A negated body atom IS satisfied in the model, so the step it guards does not hold.
    NegatedPremiseSatisfied {
        /// The cited clause index.
        rule: usize,
        /// The satisfied atom's index in the clause's AUTHORED body.
        body_position: usize,
    },
    /// The clause head carries a variable the checked premises never bound, so no
    /// conclusion can be instantiated. A program that came through
    /// [`compile`](crate::seminaive::compile) is range-restricted and cannot produce this;
    /// a proof presented against a different program can.
    UnboundHeadVariable {
        /// The cited clause index.
        rule: usize,
        /// The unbound head variable, as authored.
        variable: String,
    },
    /// The conclusion the checker DERIVED from the premises and the named rule is not the
    /// conclusion the proof stated.
    GoalMismatch {
        /// The cited clause index.
        rule: usize,
        /// The conclusion re-derived by the checker.
        derived: Box<Fact>,
        /// The conclusion the proof claimed.
        stated: Box<Fact>,
    },
}

/// Render a fact as `subject predicate object graph`, for a diagnostic.
fn render(fact: &Fact) -> String {
    format!(
        "{} {} {} {}",
        fact.subject, fact.predicate, fact.object, fact.graph
    )
}

impl fmt::Display for ProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { detail } => write!(f, "malformed proof term: {detail}"),
            Self::NotAsserted { goal } => write!(
                f,
                "the axiom {} is not in the seeded EDB, so it was not given",
                render(goal)
            ),
            Self::UnknownRule { rule } => {
                write!(
                    f,
                    "the proof cites clause {rule}, which the program has not"
                )
            }
            Self::NonDatalogRule { rule, form } => write!(
                f,
                "clause {rule} has {} {form} head, so it licenses no Datalog step",
                form.article()
            ),
            Self::PremiseCountMismatch {
                rule,
                body,
                premises,
            } => write!(
                f,
                "clause {rule} has {body} positive body atom(s) but the proof supplies \
                 {premises} premise(s)"
            ),
            Self::PremiseMismatch {
                rule,
                body_position,
                proven,
            } => write!(
                f,
                "clause {rule} body atom {body_position} does not match the premise fact {}",
                render(proven)
            ),
            Self::NegatedPremiseSatisfied {
                rule,
                body_position,
            } => write!(
                f,
                "clause {rule} negated body atom {body_position} is satisfied in the model, \
                 so the step is not licensed"
            ),
            Self::UnboundHeadVariable { rule, variable } => write!(
                f,
                "clause {rule} head variable {variable} is not bound by the checked premises"
            ),
            Self::GoalMismatch {
                rule,
                derived,
                stated,
            } => write!(
                f,
                "clause {rule} derives {} from the checked premises, not the stated {}",
                render(derived),
                render(stated)
            ),
        }
    }
}

impl std::error::Error for ProofError {}

// ── The term ────────────────────────────────────────────────────────────────────

/// One node of the proof DAG — the crate's whole proof language.
///
/// Two constructors, and no third: a fact was given, or a conclusion follows from premises
/// by a named clause. `premises` hold [`ProofId`]s of nodes interned EARLIER, so the term
/// set is acyclic by construction and a checker can process it in ascending id order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProofTerm {
    /// A fact was given: it must be a member of the seeded EDB.
    Axiom {
        /// The asserted fact.
        goal: Fact,
    },
    /// A conclusion follows from premises by the clause at this authored index.
    ByRule {
        /// The STATED conclusion. The checker never reads it as an input to re-derivation;
        /// it only compares its own result against it.
        goal: Fact,
        /// The producing clause's index in authored program order.
        rule: usize,
        /// One premise per POSITIVE body atom, in authored body order.
        premises: Vec<ProofId>,
    },
}

impl ProofTerm {
    /// The conclusion this node states.
    fn goal(&self) -> &Fact {
        match self {
            Self::Axiom { goal } | Self::ByRule { goal, .. } => goal,
        }
    }
}

/// Fold `fact`'s four length-prefixed surfaces into `hasher`.
fn hash_fact(hasher: &mut impl Hasher, fact: &Fact) {
    for surface in [&fact.subject, &fact.predicate, &fact.object, &fact.graph] {
        hasher.write_u64(surface.len() as u64);
        hasher.write(surface.as_bytes());
    }
}

/// The interning hash of one proof term.
///
/// Fixed-key `ahash`, exactly as [`crate::store::TermInterner`] uses: seeded from constants
/// rather than from ambient entropy, which does not exist on `wasm32-unknown-unknown`. The
/// table this feeds is NEVER iterated and the hash is NEVER persisted — a proof's stable
/// identity is [`ProofArena::digest`], a BLAKE3 digest over the canonical encoding — so the
/// hasher's lack of version stability cannot reach an output.
fn term_hash(term: &ProofTerm) -> u64 {
    let mut hasher = ahash::AHasher::default();
    match term {
        ProofTerm::Axiom { goal } => {
            hasher.write_u8(KIND_AXIOM);
            hash_fact(&mut hasher, goal);
        }
        ProofTerm::ByRule {
            goal,
            rule,
            premises,
        } => {
            hasher.write_u8(KIND_BY_RULE);
            hash_fact(&mut hasher, goal);
            hasher.write_u64(*rule as u64);
            hasher.write_u64(premises.len() as u64);
            for premise in premises {
                hasher.write_u64(premise.index() as u64);
            }
        }
    }
    hasher.finish()
}

// ── The checking context ────────────────────────────────────────────────────────

/// What a proof is checked AGAINST: the clause program, the seeded EDB and the model.
///
/// The three are separate on purpose.
///
/// * `rules` is the program the proof's rule indices name. Presenting a proof against a
///   different program is a legitimate question — "does this derivation still hold under
///   that calculus?" — and it is answered by re-derivation, not by trust.
/// * `edb` is the ONLY extension an [`axiom`](ProofArena::axiom) may appeal to. Checking an
///   axiom against the saturated model instead would let a proof assert its own conclusion
///   as given, which is the one hole that makes a proof checker decorative.
/// * `model` decides negation-as-failure, and nothing else. See the [module docs](self) for
///   why a finite proof cannot carry that obligation itself.
#[derive(Debug, Clone, Copy)]
pub struct ProofContext<'a> {
    /// The clause program, in authored order.
    rules: &'a [DlClause],
    /// The seeded EDB — the axioms.
    edb: &'a RelationStore,
    /// The saturated model — where negation-as-failure is decided.
    model: &'a RelationStore,
}

impl<'a> ProofContext<'a> {
    /// A context over a program, its seeded EDB and the model the evaluation produced.
    pub fn new(rules: &'a [DlClause], edb: &'a RelationStore, model: &'a RelationStore) -> Self {
        Self { rules, edb, model }
    }

    /// The clause program, in authored order.
    pub fn rules(&self) -> &'a [DlClause] {
        self.rules
    }

    /// The seeded EDB — the only extension an axiom may appeal to.
    pub fn edb(&self) -> &'a RelationStore {
        self.edb
    }

    /// The saturated model — where negation-as-failure is decided.
    pub fn model(&self) -> &'a RelationStore {
        self.model
    }
}

// ── The arena ───────────────────────────────────────────────────────────────────

/// A hash-consed arena of proof terms.
///
/// Terms are interned, so a shared subproof is stored ONCE and structural equality is a
/// [`ProofId`] comparison. Ids are dense and assigned in interning order; a premise always
/// has a smaller id than the step it justifies, which is what makes the DAG acyclic by
/// construction and lets [`check`](Self::check) run in one ascending pass with no recursion
/// and no cycle detection.
///
/// The arena is owned, never global: two proofs of the same fact built by two callers are
/// two arenas, and [`digest`](Self::digest) — not an arena-local id — is what says they are
/// the same proof.
#[derive(Debug, Clone, Default)]
pub struct ProofArena {
    /// The interned terms, indexed by [`ProofId`] slot.
    terms: Vec<ProofTerm>,
    /// Content hash → id, for O(1) interning. Probed, NEVER iterated: its order is a hash
    /// order and would be an illegal emission source.
    by_content: HashTable<ProofId>,
}

impl ProofArena {
    /// A fresh, empty arena.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of interned terms.
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether the arena holds no terms.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Intern an AXIOM leaf: `goal` was given.
    ///
    /// Constructing one asserts nothing — [`check`](Self::check) is what tests the claim,
    /// against the seeded EDB.
    pub fn axiom(&mut self, goal: Fact) -> ProofId {
        self.intern(ProofTerm::Axiom { goal })
    }

    /// Intern a RULE APPLICATION: `goal` follows from `premises` by clause `rule`.
    ///
    /// `premises` are one per POSITIVE body atom of the clause, in authored body order. The
    /// stated `goal` is recorded but is never an input to checking; it exists so that
    /// [`check`](Self::check) has something to compare its own re-derivation against, and so
    /// that a tampered conclusion is a detectable forgery rather than an unrepresentable one.
    ///
    /// # Panics
    ///
    /// Panics if a premise id was not minted by this arena. Ids are per-arena handles, so a
    /// foreign one is a programming error; a proof arriving from OUTSIDE the process comes
    /// through [`decode`](Self::decode), which reports the same defect as a
    /// [`ProofError::Malformed`] rejection instead.
    pub fn by_rule(&mut self, goal: Fact, rule: usize, premises: &[ProofId]) -> ProofId {
        for premise in premises {
            assert!(
                premise.index() < self.terms.len(),
                "a premise must be interned in this arena before the step it justifies"
            );
        }
        self.intern(ProofTerm::ByRule {
            goal,
            rule,
            premises: premises.to_vec(),
        })
    }

    /// Intern `term`, returning the existing id if an identical term is already held.
    fn intern(&mut self, term: ProofTerm) -> ProofId {
        let hash = term_hash(&term);
        let terms = &self.terms;
        if let Some(&id) = self.by_content.find(hash, |&id| terms[id.index()] == term) {
            return id;
        }
        let id = ProofId::from_index(self.terms.len());
        self.terms.push(term);
        let terms = &self.terms;
        self.by_content
            .insert_unique(hash, id, |&id| term_hash(&terms[id.index()]));
        id
    }

    /// The conclusion the node STATES — which is exactly what a checker must not believe.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this arena.
    pub fn goal(&self, id: ProofId) -> &Fact {
        self.term(id).goal()
    }

    /// Whether the node is an axiom leaf.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this arena.
    pub fn is_axiom(&self, id: ProofId) -> bool {
        matches!(self.term(id), ProofTerm::Axiom { .. })
    }

    /// The clause index a rule application cites, or `None` for an axiom leaf.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this arena.
    pub fn rule(&self, id: ProofId) -> Option<usize> {
        match self.term(id) {
            ProofTerm::Axiom { .. } => None,
            ProofTerm::ByRule { rule, .. } => Some(*rule),
        }
    }

    /// The node's premises, in authored positive-body order; empty for an axiom leaf.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this arena.
    pub fn premises(&self, id: ProofId) -> &[ProofId] {
        match self.term(id) {
            ProofTerm::Axiom { .. } => &[],
            ProofTerm::ByRule { premises, .. } => premises,
        }
    }

    /// The term at `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not minted by this arena. Ids are per-arena handles, exactly as
    /// [`crate::store::TermInterner`]'s are, so a foreign id is a programming error rather
    /// than a data state.
    fn term(&self, id: ProofId) -> &ProofTerm {
        self.terms.get(id.index()).unwrap_or_else(|| {
            panic!(
                "ProofId {id:?} was not minted by this arena (len {}): proof ids are \
                 per-arena handles and must never cross arena boundaries",
                self.terms.len()
            )
        })
    }

    // ── The checker ─────────────────────────────────────────────────────────────

    /// RE-DERIVE `root` against `ctx`, returning the conclusion the CHECKER computed.
    ///
    /// This is the whole point of the module. The returned [`Fact`] is not read out of the
    /// proof — it is instantiated from the named clause's head under the substitution the
    /// checked premises forced. The proof's stated conclusion participates in exactly one
    /// way: as the value the derived conclusion is compared against, so a mismatch is a
    /// [`ProofError::GoalMismatch`] rejection.
    ///
    /// Each node is checked ONCE and memoized, so a maximally shared proof costs one step
    /// per distinct node rather than one per path. Nodes are visited in ascending id order,
    /// which is a topological order because a premise is always interned before the step it
    /// justifies — no recursion, so a deep proof cannot overflow a stack.
    ///
    /// Only the sub-DAG reachable from `root` is checked: an arena holding several proofs
    /// answers for the one it was asked about, and an unrelated defective term elsewhere in
    /// the arena neither excuses nor condemns it.
    pub fn check(&self, root: ProofId, ctx: &ProofContext<'_>) -> Result<Fact, ProofError> {
        let reachable = self.reachable_from(root)?;
        let mut proven: BTreeMap<usize, Fact> = BTreeMap::new();
        for &index in &reachable {
            let fact = self.check_step(index, &proven, ctx)?;
            proven.insert(index, fact);
        }
        proven
            .remove(&root.index())
            .ok_or_else(|| Self::malformed("the root of a proof is reachable from itself"))
    }

    /// A [`ProofError::Malformed`] carrying `detail`.
    fn malformed(detail: &str) -> ProofError {
        ProofError::Malformed {
            detail: detail.to_owned(),
        }
    }

    /// Every node index reachable from `root`, ASCENDING.
    ///
    /// Ascending id order is a topological order of the proof DAG, because
    /// [`by_rule`](Self::by_rule) and [`decode`](Self::decode) both refuse a premise that
    /// does not already exist. The walk re-checks that invariant rather than assuming it, so
    /// a term set that somehow violated it is rejected instead of silently mis-checked.
    fn reachable_from(&self, root: ProofId) -> Result<BTreeSet<usize>, ProofError> {
        if root.index() >= self.terms.len() {
            return Err(Self::malformed("the root is outside this arena"));
        }
        let mut reachable = BTreeSet::new();
        let mut stack = vec![root.index()];
        while let Some(index) = stack.pop() {
            if !reachable.insert(index) {
                continue;
            }
            if let ProofTerm::ByRule { premises, .. } = &self.terms[index] {
                for premise in premises {
                    if premise.index() >= index {
                        return Err(Self::malformed(
                            "a premise must be interned before the step it justifies",
                        ));
                    }
                    stack.push(premise.index());
                }
            }
        }
        Ok(reachable)
    }

    /// Check ONE node, given the facts its premises were already proved to establish.
    fn check_step(
        &self,
        index: usize,
        proven: &BTreeMap<usize, Fact>,
        ctx: &ProofContext<'_>,
    ) -> Result<Fact, ProofError> {
        match &self.terms[index] {
            ProofTerm::Axiom { goal } => {
                // The seeded EDB, never the saturated model: see [`ProofContext`].
                if ctx
                    .edb
                    .contains(&goal.subject, &goal.predicate, &goal.object, &goal.graph)
                {
                    Ok(goal.clone())
                } else {
                    Err(ProofError::NotAsserted {
                        goal: Box::new(goal.clone()),
                    })
                }
            }
            ProofTerm::ByRule {
                goal,
                rule,
                premises,
            } => self.check_rule_step(*rule, premises, goal, proven, ctx),
        }
    }

    /// Re-derive one rule application and compare the result against the stated conclusion.
    fn check_rule_step(
        &self,
        rule: usize,
        premises: &[ProofId],
        stated: &Fact,
        proven: &BTreeMap<usize, Fact>,
        ctx: &ProofContext<'_>,
    ) -> Result<Fact, ProofError> {
        let clause = ctx
            .rules
            .get(rule)
            .ok_or(ProofError::UnknownRule { rule })?;
        // A clause with any other head form licenses no Datalog step, so there is no head
        // to instantiate and nothing to compare — that is a refusal, not a mismatch.
        let head = clause
            .datalog_head()
            .ok_or_else(|| ProofError::NonDatalogRule {
                rule,
                form: clause.head_form(),
            })?;

        // A negated body atom is a refutation obligation, not a premise, so the premises
        // pair with the POSITIVE atoms alone — in authored body order, which is the order
        // a `Derivation` reports its sources in.
        let positive: Vec<(usize, &ClauseAtom)> = clause
            .body()
            .iter()
            .enumerate()
            .filter(|(_, atom)| !atom.is_negated())
            .collect();
        if positive.len() != premises.len() {
            return Err(ProofError::PremiseCountMismatch {
                rule,
                body: positive.len(),
                premises: premises.len(),
            });
        }

        // The join, re-run from the clause text: one substitution shared across the body,
        // so a variable repeated across atoms — or twice within one atom — must agree.
        let mut subst: BTreeMap<&str, &str> = BTreeMap::new();
        for (&(body_position, atom), premise) in positive.iter().zip(premises) {
            let fact = proven.get(&premise.index()).ok_or_else(|| {
                Self::malformed("a premise was not established before the step that uses it")
            })?;
            if !match_atom(atom, fact, &mut subst) {
                return Err(ProofError::PremiseMismatch {
                    rule,
                    body_position,
                    proven: Box::new(fact.clone()),
                });
            }
        }

        // Negation-as-failure, re-decided against the model.
        for (body_position, atom) in clause
            .body()
            .iter()
            .enumerate()
            .filter(|(_, atom)| atom.is_negated())
        {
            if negated_atom_is_satisfied(atom, &subst, ctx.model) {
                return Err(ProofError::NegatedPremiseSatisfied {
                    rule,
                    body_position,
                });
            }
        }

        // Instantiate the head — this is the checker's own conclusion.
        let derived = ground_atom(head, rule, &subst)?;
        if &derived == stated {
            Ok(derived)
        } else {
            Err(ProofError::GoalMismatch {
                rule,
                derived: Box::new(derived),
                stated: Box::new(stated.clone()),
            })
        }
    }

    // ── Canonical serialization ─────────────────────────────────────────────────

    /// The canonical byte encoding of the proof rooted at `root`.
    ///
    /// The encoding carries NO arena id. Nodes are emitted in a post-order first-visit walk
    /// of the DAG — every premise before the step that uses it, the root last — and a
    /// premise is written as its zero-based position in THAT emission, so the bytes depend
    /// on the proof's shape alone. Two arenas that interned the same proof through different
    /// construction sequences therefore encode identically, on every target.
    ///
    /// Layout, all integers little-endian:
    ///
    /// ```text
    /// u64 tag_len, tag bytes                      -- PROOF_ENCODING_TAG
    /// u64 node_count
    /// node_count times, in emission order:
    ///     u8  kind                                -- 0 axiom, 1 by_rule
    ///     u64 len + bytes, four times             -- subject, predicate, object, graph
    ///     if kind == 1:
    ///         u64 rule
    ///         u64 premise_count
    ///         u64 premise position, premise_count times   -- each < this node's position
    /// ```
    ///
    /// Every variable-length field is length-prefixed, so no concatenation of two surfaces
    /// can be confused with a different split of the same bytes. The root is the LAST node,
    /// which is why a `by_rule` root's premise list is the encoding's final field.
    ///
    /// # Panics
    ///
    /// Panics if `root` was not minted by this arena.
    pub fn encode(&self, root: ProofId) -> Vec<u8> {
        let order = self.emission_order(root);
        let position: BTreeMap<usize, usize> = order
            .iter()
            .enumerate()
            .map(|(position, &index)| (index, position))
            .collect();

        let mut out = Vec::new();
        frame(&mut out, PROOF_ENCODING_TAG.as_bytes());
        out.extend_from_slice(&(order.len() as u64).to_le_bytes());
        for &index in &order {
            match &self.terms[index] {
                ProofTerm::Axiom { goal } => {
                    out.push(KIND_AXIOM);
                    frame_fact(&mut out, goal);
                }
                ProofTerm::ByRule {
                    goal,
                    rule,
                    premises,
                } => {
                    out.push(KIND_BY_RULE);
                    frame_fact(&mut out, goal);
                    out.extend_from_slice(&(*rule as u64).to_le_bytes());
                    out.extend_from_slice(&(premises.len() as u64).to_le_bytes());
                    for premise in premises {
                        let back = position[&premise.index()];
                        out.extend_from_slice(&(back as u64).to_le_bytes());
                    }
                }
            }
        }
        out
    }

    /// The BLAKE3 digest of [`encode`](Self::encode) — a proof term's stable identity.
    ///
    /// BLAKE3 rather than the interning hasher: `ahash` is explicitly not version-stable, so
    /// it cannot address content across a dependency bump. Only `update` is used, never
    /// `update_rayon`, so hashing is sequential on every target and the `wasm32` build
    /// carries no thread pool.
    ///
    /// PurRDF mints no vocabulary, so this digest — not a fabricated derivation IRI — is how
    /// a proof term is named.
    ///
    /// # Panics
    ///
    /// Panics if `root` was not minted by this arena.
    pub fn digest(&self, root: ProofId) -> [u8; 32] {
        *blake3::hash(&self.encode(root)).as_bytes()
    }

    /// The node indices reachable from `root`, in post-order first-visit order.
    ///
    /// Iterative, so proof depth costs heap rather than stack. A shared node is emitted at
    /// its FIRST completion and referenced by position afterwards, which is what keeps the
    /// encoding linear in the DAG rather than in its paths.
    fn emission_order(&self, root: ProofId) -> Vec<usize> {
        /// One entry of the explicit traversal stack.
        enum Step {
            /// Descend into this node's premises.
            Visit(usize),
            /// Emit this node, its premises having been emitted.
            Emit(usize),
        }

        // Touch the root through the checked accessor so a foreign id panics by name.
        let _ = self.term(root);
        let mut out = Vec::new();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut stack = vec![Step::Visit(root.index())];
        while let Some(step) = stack.pop() {
            match step {
                Step::Visit(index) => {
                    if !seen.insert(index) {
                        continue;
                    }
                    stack.push(Step::Emit(index));
                    if let ProofTerm::ByRule { premises, .. } = &self.terms[index] {
                        // Reversed, so the premises are descended into in authored order.
                        for premise in premises.iter().rev() {
                            stack.push(Step::Visit(premise.index()));
                        }
                    }
                }
                Step::Emit(index) => out.push(index),
            }
        }
        out
    }

    /// Rebuild an arena and its root from [`encode`](Self::encode)d bytes.
    ///
    /// This is the UNTRUSTED entrance, and it is where a corrupted or forged proof is a
    /// rejection rather than a panic: a mis-tagged, truncated or over-long stream, an
    /// unknown node kind, a premise reference that is out of range or does not precede the
    /// step that uses it, and a rule index too large for the target's `usize` are all
    /// [`ProofError::Malformed`].
    ///
    /// Decoding RE-INTERNS, so the rebuilt arena is maximally shared exactly like a
    /// freshly-built one and `encode(decode(encode(p))) == encode(p)`. A forged reference
    /// that is nonetheless structurally legal — a premise pointing at the WRONG earlier node
    /// — decodes cleanly and is caught where it should be: by
    /// [`check`](Self::check)'s re-derivation.
    pub fn decode(bytes: &[u8]) -> Result<(Self, ProofId), ProofError> {
        let mut reader = Reader::new(bytes);
        if reader.frame()? != PROOF_ENCODING_TAG.as_bytes() {
            return Err(Self::malformed(
                "the proof encoding tag is absent or from another layout",
            ));
        }
        let count = reader.length()?;
        if count == 0 {
            return Err(Self::malformed("a proof has at least one node"));
        }

        let mut arena = Self::new();
        let mut ids: Vec<ProofId> = Vec::new();
        for position in 0..count {
            let kind = reader.byte()?;
            let goal = reader.fact()?;
            let id = match kind {
                KIND_AXIOM => arena.axiom(goal),
                KIND_BY_RULE => {
                    let rule = reader.length()?;
                    let premise_count = reader.length()?;
                    // Never sized from the stream: a forged count must not be able to ask
                    // for an allocation before its bytes have been shown to exist.
                    let mut premises = Vec::new();
                    for _ in 0..premise_count {
                        let back = reader.length()?;
                        if back >= position {
                            return Err(Self::malformed(
                                "a premise reference must point at an earlier node",
                            ));
                        }
                        premises.push(ids[back]);
                    }
                    arena.by_rule(goal, rule, &premises)
                }
                other => {
                    return Err(ProofError::Malformed {
                        detail: format!("unknown proof node kind {other}"),
                    });
                }
            };
            ids.push(id);
        }
        if !reader.is_exhausted() {
            return Err(Self::malformed(
                "trailing bytes after the proof's last node",
            ));
        }
        let root = *ids
            .last()
            .ok_or_else(|| Self::malformed("a proof has at least one node"))?;
        Ok((arena, root))
    }
}

// ── Proofs for a completed evaluation ───────────────────────────────────────────

/// A proof term for every derivation of one completed [`Evaluation`].
///
/// The builder is deliberately incapable of laundering an engine defect. It reads a
/// derivation's declared sources and its declared proof height and nothing else: a source it
/// has not already built a proof for is recorded as an AXIOM, and an axiom that is not in
/// the seeded EDB is rejected by [`ProofArena::check`]. So an engine that mis-stated a proof
/// height, invented a source, or derived a fact circularly produces proofs that FAIL to
/// check rather than proofs that quietly agree with it.
#[derive(Debug, Clone)]
pub struct EvaluationProofs {
    /// The hash-consed terms; a subproof shared by two derivations is stored once.
    arena: ProofArena,
    /// One root per derived fact, in lexical fact order.
    roots: Vec<(Fact, ProofId)>,
}

impl EvaluationProofs {
    /// Build a proof term for every derivation `evaluation` recorded.
    ///
    /// Derivations are processed in `(proof height, fact)` order, so every DERIVED source of
    /// a step already has its proof interned when the step is built — a source's proof
    /// height is strictly below its consumer's, by the definition of proof height. Both
    /// components are content-derived, so the interning sequence, and hence every arena id,
    /// is a pure function of the evaluation.
    pub fn of(evaluation: &Evaluation) -> Self {
        let derivations = evaluation.derivations();
        let mut order: Vec<usize> = (0..derivations.len()).collect();
        order.sort_by(|&left, &right| {
            (derivations[left].proof_height(), derivations[left].fact())
                .cmp(&(derivations[right].proof_height(), derivations[right].fact()))
        });

        let mut arena = ProofArena::new();
        let mut built: BTreeMap<&Fact, ProofId> = BTreeMap::new();
        for &index in &order {
            let derivation = &derivations[index];
            let mut premises = Vec::with_capacity(derivation.sources().len());
            for source in derivation.sources() {
                let premise = match built.get(source) {
                    Some(&id) => id,
                    None => arena.axiom(source.clone()),
                };
                premises.push(premise);
            }
            let root = arena.by_rule(derivation.fact().clone(), derivation.rule(), &premises);
            built.insert(derivation.fact(), root);
        }

        let roots = built
            .into_iter()
            .map(|(fact, id)| (fact.clone(), id))
            .collect();
        Self { arena, roots }
    }

    /// The arena the roots address.
    pub fn arena(&self) -> &ProofArena {
        &self.arena
    }

    /// One `(derived fact, proof)` pair per derivation, in LEXICAL fact order.
    pub fn roots(&self) -> &[(Fact, ProofId)] {
        &self.roots
    }

    /// The proof of `fact`, if the evaluation derived it.
    pub fn root_for(&self, fact: &Fact) -> Option<ProofId> {
        self.roots
            .binary_search_by(|(candidate, _)| candidate.cmp(fact))
            .ok()
            .map(|position| self.roots[position].1)
    }
}

// ── Re-derivation primitives ────────────────────────────────────────────────────

/// Match one clause atom against the fact a premise established, extending `subst`.
///
/// A constant position must equal the fact's surface exactly — through
/// [`ClauseTerm::surface`], the crate's single rendering convention, so a clause constant and
/// stored data are always compared as the same bytes. A variable position must agree with
/// every other occurrence of the same variable, anywhere in the body, which is how the
/// generalized diagonal (a variable repeated inside one atom) and the join (a variable shared
/// across atoms) are BOTH re-checked by one rule.
fn match_atom<'r, 'f>(
    atom: &'r ClauseAtom,
    fact: &'f Fact,
    subst: &mut BTreeMap<&'r str, &'f str>,
) -> bool {
    let surfaces = [
        fact.subject.as_str(),
        fact.predicate.as_str(),
        fact.object.as_str(),
        fact.graph.as_str(),
    ];
    for (term, surface) in atom.terms().into_iter().zip(surfaces) {
        match term {
            ClauseTerm::Var(name) => match subst.entry(name.as_str()) {
                Entry::Occupied(bound) => {
                    if *bound.get() != surface {
                        return false;
                    }
                }
                Entry::Vacant(slot) => {
                    slot.insert(surface);
                }
            },
            constant => {
                let rendered = constant
                    .surface()
                    .expect("a non-variable term always has a lexical surface");
                if rendered != surface {
                    return false;
                }
            }
        }
    }
    true
}

/// The lexical surface a clause term denotes under `subst`, or `None` for a free variable.
fn resolved_surface(term: &ClauseTerm, subst: &BTreeMap<&str, &str>) -> Option<String> {
    match term {
        ClauseTerm::Var(name) => subst.get(name.as_str()).map(|&surface| surface.to_owned()),
        constant => Some(
            constant
                .surface()
                .expect("a non-variable term always has a lexical surface"),
        ),
    }
}

/// Instantiate a rule head under `subst` — the checker's OWN conclusion.
fn ground_atom(
    head: &ClauseAtom,
    rule: usize,
    subst: &BTreeMap<&str, &str>,
) -> Result<Fact, ProofError> {
    let mut surfaces: [String; ATOM_ARITY] = std::array::from_fn(|_| String::new());
    for (position, term) in head.terms().into_iter().enumerate() {
        surfaces[position] = match resolved_surface(term, subst) {
            Some(surface) => surface,
            None => {
                let variable = term
                    .variable()
                    .expect("only a variable position can be unresolved")
                    .to_owned();
                return Err(ProofError::UnboundHeadVariable { rule, variable });
            }
        };
    }
    let [subject, predicate, object, graph] = surfaces;
    Ok(Fact {
        subject,
        predicate,
        object,
        graph,
    })
}

/// Whether a NEGATED body atom is satisfied in `model` under `subst` — i.e. some matching
/// fact exists, so the step it guards does NOT hold.
///
/// The two binding modes are the reference negation-as-failure semantics, re-decided here
/// from the clause text alone:
///
/// * a fully ground atom is a membership probe;
/// * a partially bound atom is existential — an unbound position is unconstrained, so
///   `not p(?x, ?y)` with `?y` free reads as "`?x` has no `p` at all", and an unbound
///   PREDICATE or GRAPH position is unconstrained in exactly the same way.
///
/// A ground position whose surface the model never interned constrains to zero rows, so the
/// atom is not satisfied.
fn negated_atom_is_satisfied(
    atom: &ClauseAtom,
    subst: &BTreeMap<&str, &str>,
    model: &RelationStore,
) -> bool {
    let mut values = [None; ATOM_ARITY];
    for (position, term) in atom.terms().into_iter().enumerate() {
        let Some(surface) = resolved_surface(term, subst) else {
            continue; // free: unconstrained
        };
        match model.term_id(&surface) {
            Some(id) => values[position] = Some(id),
            None => return false,
        }
    }
    let bound = match (values[POSITION_SUBJECT], values[POSITION_OBJECT]) {
        (Some(subject), Some(object)) => Bound::Both(subject, object),
        (Some(subject), None) => Bound::Subject(subject),
        (None, Some(object)) => Bound::Object(object),
        (None, None) => Bound::Any,
    };
    model
        .partitions(values[POSITION_PREDICATE], values[POSITION_GRAPH])
        .any(|partition| partition.select(bound).any_remaining())
}

// ── Wire primitives ─────────────────────────────────────────────────────────────

/// Length-prefix `bytes` into `out`.
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Length-prefix a fact's four surfaces into `out`, in `(s, p, o, g)` order.
fn frame_fact(out: &mut Vec<u8>, fact: &Fact) {
    for surface in [&fact.subject, &fact.predicate, &fact.object, &fact.graph] {
        frame(out, surface.as_bytes());
    }
}

/// A bounds-checked forward reader over an encoded proof.
///
/// Every read is fallible and every length is validated against the bytes that actually
/// remain, so a truncated or hostile stream is a [`ProofError::Malformed`] rejection and
/// never a panic or an over-large allocation.
#[derive(Debug)]
struct Reader<'a> {
    /// The bytes still unread.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader over `bytes`.
    fn new(bytes: &'a [u8]) -> Self {
        Self { rest: bytes }
    }

    /// Whether every byte has been consumed.
    fn is_exhausted(&self) -> bool {
        self.rest.is_empty()
    }

    /// Take `n` bytes, or report truncation.
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProofError> {
        if self.rest.len() < n {
            return Err(ProofArena::malformed(
                "the proof encoding ends inside a field",
            ));
        }
        let (head, tail) = self.rest.split_at(n);
        self.rest = tail;
        Ok(head)
    }

    /// Read one kind byte.
    fn byte(&mut self) -> Result<u8, ProofError> {
        Ok(self.take(1)?[0])
    }

    /// Read one little-endian `u64`.
    fn u64(&mut self) -> Result<u64, ProofError> {
        let bytes = self.take(8)?;
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(buffer))
    }

    /// Read one little-endian `u64` as a `usize`.
    ///
    /// A value too large for the target's `usize` is a rejection rather than a wrap: a proof
    /// written on a 64-bit host and read on `wasm32` must fail loudly instead of decoding to
    /// a different proof.
    fn length(&mut self) -> Result<usize, ProofError> {
        usize::try_from(self.u64()?).map_err(|_| {
            ProofArena::malformed("a length in the proof encoding exceeds this target's usize")
        })
    }

    /// Read one length-prefixed field.
    fn frame(&mut self) -> Result<&'a [u8], ProofError> {
        let len = self.length()?;
        self.take(len)
    }

    /// Read one length-prefixed UTF-8 surface.
    fn surface(&mut self) -> Result<String, ProofError> {
        let bytes = self.frame()?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| ProofArena::malformed("a term surface in the proof encoding is not UTF-8"))
    }

    /// Read one fact's four surfaces, in `(s, p, o, g)` order.
    fn fact(&mut self) -> Result<Fact, ProofError> {
        Ok(Fact {
            subject: self.surface()?,
            predicate: self.surface()?,
            object: self.surface()?,
            graph: self.surface()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::clause::HeadDisjunct;
    use crate::seminaive::{compile, evaluate};
    use crate::synth_corpus;

    const P: &str = "https://example.org/p";
    const Q: &str = "https://example.org/q";
    const R: &str = "https://example.org/r";
    const T: &str = "https://example.org/t";

    fn v(name: &str) -> ClauseTerm {
        ClauseTerm::var(name)
    }

    fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(v(subject), predicate, v(object))
    }

    /// The lexical surface an IRI is stored under.
    fn surface(name: &str) -> String {
        format!("<{name}>")
    }

    fn store_of(triples: &[(&str, &str, &str)]) -> RelationStore {
        let mut store = RelationStore::new();
        for &(subject, predicate, object) in triples {
            store.insert(
                &surface(subject),
                &surface(predicate),
                &surface(object),
                RelationStore::DEFAULT_GRAPH,
            );
        }
        store
    }

    /// A fact over IRI terms, in the default graph.
    fn fact(subject: &str, predicate: &str, object: &str) -> Fact {
        Fact {
            subject: surface(subject),
            predicate: surface(predicate),
            object: surface(object),
            graph: RelationStore::DEFAULT_GRAPH.to_owned(),
        }
    }

    /// The two-hop chain program `t(?s, ?o) :- p(?s, ?m), p(?m, ?o).`
    fn chain_rules() -> Vec<DlClause> {
        vec![DlClause::datalog(
            atom("?s", T, "?o"),
            vec![atom("?s", P, "?m"), atom("?m", P, "?o")],
        )]
    }

    /// `p(a, b), p(b, c)` — the EDB the chain program derives `t(a, c)` from.
    fn chain_edb() -> RelationStore {
        store_of(&[("a", P, "b"), ("b", P, "c")])
    }

    /// A hand-built, VALID proof of `t(a, c)` over [`chain_rules`], plus its arena.
    fn chain_proof() -> (ProofArena, ProofId) {
        let mut arena = ProofArena::new();
        let first = arena.axiom(fact("a", P, "b"));
        let second = arena.axiom(fact("b", P, "c"));
        let root = arena.by_rule(fact("a", T, "c"), 0, &[first, second]);
        (arena, root)
    }

    // ── The positive obligation ─────────────────────────────────────────────────

    /// EVERY derivation an actual `evaluate` run produces over the whole synthetic corpus
    /// yields a proof that `check` accepts, and the CHECKED conclusion equals the derived
    /// fact.
    ///
    /// This is the obligation the whole module exists to discharge: the engine's answers
    /// are not merely logged, they are re-derived from the clause text by a checker that
    /// shares no code with the join kernels, the planner or the semi-naive decomposition.
    #[test]
    fn every_corpus_derivation_yields_a_checkable_proof() {
        for workload in synth_corpus::all() {
            let name = workload.name;
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let evaluation =
                evaluate(&exe, workload.edb()).expect("the corpus stays inside every ceiling");
            let proofs = EvaluationProofs::of(&evaluation);
            assert_eq!(
                proofs.roots().len(),
                evaluation.derivations().len(),
                "{name}: one proof per derivation"
            );

            let edb = workload.edb();
            let ctx = ProofContext::new(&workload.rules, &edb, evaluation.facts());
            for (derived, root) in proofs.roots() {
                let checked = proofs
                    .arena()
                    .check(*root, &ctx)
                    .unwrap_or_else(|error| panic!("{name}: {derived:?} failed to check: {error}"));
                assert_eq!(
                    &checked, derived,
                    "{name}: the checker's own conclusion must be the derived fact"
                );
            }
            assert!(
                !proofs.roots().is_empty(),
                "{name}: a corpus workload derives something, or it proves nothing"
            );
        }
    }

    /// The certified-cyclic path is proved too: the triangle routes through the leapfrog
    /// triejoin, and its derivations re-derive from the clause text exactly like the binary
    /// join's do.
    #[test]
    fn a_leapfrog_derivation_yields_a_checkable_proof() {
        let n = 5usize;
        let node = |i: usize| format!("https://example.org/n{i}");
        let sink = "https://example.org/s";
        let rules = vec![DlClause::datalog(
            atom("?X", sink, "?Z"),
            vec![
                atom("?X", P, "?Y"),
                atom("?Y", Q, "?Z"),
                atom("?Z", R, "?X"),
            ],
        )];
        let seed = || {
            let mut store = RelationStore::new();
            for i in 0..n {
                for j in 0..n {
                    store.insert(
                        &surface(&node(i)),
                        &surface(P),
                        &surface(&node(j)),
                        RelationStore::DEFAULT_GRAPH,
                    );
                }
                store.insert(
                    &surface(&node(i)),
                    &surface(Q),
                    &surface(&node((i + 1) % n)),
                    RelationStore::DEFAULT_GRAPH,
                );
                store.insert(
                    &surface(&node(i)),
                    &surface(R),
                    &surface(&node((i + 2) % n)),
                    RelationStore::DEFAULT_GRAPH,
                );
            }
            store
        };
        let exe = compile(rules.clone()).expect("the triangle compiles");
        assert!(
            (0..exe.rule_count()).any(|index| exe.rule_entry(index).1.has_cyclic_subplan()),
            "the fixture must route through the triejoin, or it tests the binary join twice"
        );
        let evaluation = evaluate(&exe, seed()).expect("the triangle stays inside every ceiling");
        let proofs = EvaluationProofs::of(&evaluation);
        assert_eq!(proofs.roots().len(), n, "one derived fact per node");

        let edb = seed();
        let ctx = ProofContext::new(&rules, &edb, evaluation.facts());
        for (derived, root) in proofs.roots() {
            assert_eq!(
                proofs.arena().check(*root, &ctx).as_ref(),
                Ok(derived),
                "a leapfrog derivation must re-derive from the clause text"
            );
        }
    }

    // ── The four mandatory rejections ───────────────────────────────────────────

    /// (1) A WRONG RULE ID: the named clause does not license the step.
    ///
    /// Both shapes are rejected — a clause index the program has not, and a clause that
    /// exists but whose body cannot be matched by the supplied premises.
    #[test]
    fn a_wrong_rule_id_is_rejected() {
        let edb = chain_edb();
        // Clause 0 is the chain rule; clause 1 is a DIFFERENT rule over a predicate the
        // premises do not carry, so it cannot license the same step.
        let rules = vec![
            chain_rules().remove(0),
            DlClause::datalog(atom("?s", T, "?o"), vec![atom("?s", Q, "?o")]),
        ];

        let mut arena = ProofArena::new();
        let first = arena.axiom(fact("a", P, "b"));
        let second = arena.axiom(fact("b", P, "c"));

        // The step is real, but it cites clause 1, which does not license it.
        let wrong = arena.by_rule(fact("a", T, "c"), 1, &[first, second]);
        let ctx = ProofContext::new(&rules, &edb, &edb);
        assert_eq!(
            arena.check(wrong, &ctx),
            Err(ProofError::PremiseCountMismatch {
                rule: 1,
                body: 1,
                premises: 2,
            }),
            "a clause of a different arity cannot license the step"
        );

        // A same-arity clause that names a predicate the premises do not carry.
        let rules = vec![
            chain_rules().remove(0),
            DlClause::datalog(
                atom("?s", T, "?o"),
                vec![atom("?s", Q, "?m"), atom("?m", Q, "?o")],
            ),
        ];
        let ctx = ProofContext::new(&rules, &edb, &edb);
        assert_eq!(
            arena.check(wrong, &ctx),
            Err(ProofError::PremiseMismatch {
                rule: 1,
                body_position: 0,
                proven: Box::new(fact("a", P, "b")),
            }),
            "a clause whose body atom does not match the premise is rejected"
        );

        // And a clause index the program simply has not.
        let absent = arena.by_rule(fact("a", T, "c"), 7, &[first, second]);
        assert_eq!(
            arena.check(absent, &ctx),
            Err(ProofError::UnknownRule { rule: 7 })
        );
    }

    /// (2) A MISSING PREMISE: a body atom is left unsatisfied.
    ///
    /// Two shapes, both rejected: the premise is simply absent (so the clause's positive
    /// body arity is not covered), and the premise is present but appeals to a fact the EDB
    /// does not hold, so the body atom it was meant to satisfy is unsupported.
    #[test]
    fn a_missing_premise_is_rejected() {
        let rules = chain_rules();
        let edb = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &edb);

        let mut arena = ProofArena::new();
        let first = arena.axiom(fact("a", P, "b"));

        // The second body atom has no premise at all.
        let short = arena.by_rule(fact("a", T, "c"), 0, &[first]);
        assert_eq!(
            arena.check(short, &ctx),
            Err(ProofError::PremiseCountMismatch {
                rule: 0,
                body: 2,
                premises: 1,
            })
        );

        // The premise exists but the fact it asserts was never given.
        let unsupported = arena.axiom(fact("b", P, "z"));
        let forged = arena.by_rule(fact("a", T, "z"), 0, &[first, unsupported]);
        assert_eq!(
            arena.check(forged, &ctx),
            Err(ProofError::NotAsserted {
                goal: Box::new(fact("b", P, "z")),
            }),
            "an axiom the EDB does not hold cannot satisfy a body atom"
        );

        // The same guard closes circularity: a DERIVED fact asserted as given is refused,
        // because an axiom may appeal to the seeded EDB alone.
        let circular_premise = arena.axiom(fact("a", T, "c"));
        let circular = arena.by_rule(fact("a", T, "c"), 0, &[first, circular_premise]);
        let exe = compile(rules.clone()).expect("the chain program compiles");
        let evaluation = evaluate(&exe, chain_edb()).expect("the chain stays inside every ceiling");
        assert!(
            evaluation.facts().contains(
                &surface("a"),
                &surface(T),
                &surface("c"),
                RelationStore::DEFAULT_GRAPH
            ),
            "t(a, c) really is in the model, so only the EDB restriction rejects it"
        );
        let model_ctx = ProofContext::new(&rules, &edb, evaluation.facts());
        assert_eq!(
            arena.check(circular, &model_ctx),
            Err(ProofError::NotAsserted {
                goal: Box::new(fact("a", T, "c")),
            }),
            "a proof may not assert its own conclusion as given"
        );
    }

    /// (3) A TAMPERED GOAL: the stated conclusion is not what the premises and the rule
    /// yield. The checker derives `t(a, c)` and reports it beside the claim.
    #[test]
    fn a_tampered_goal_is_rejected() {
        let rules = chain_rules();
        let edb = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &edb);

        let mut arena = ProofArena::new();
        let first = arena.axiom(fact("a", P, "b"));
        let second = arena.axiom(fact("b", P, "c"));
        let tampered = arena.by_rule(fact("a", T, "zzz"), 0, &[first, second]);
        assert_eq!(
            arena.check(tampered, &ctx),
            Err(ProofError::GoalMismatch {
                rule: 0,
                derived: Box::new(fact("a", T, "c")),
                stated: Box::new(fact("a", T, "zzz")),
            }),
            "the checker reports the conclusion IT derived, not the one it was handed"
        );

        // The untampered proof of the same shape checks, so the rejection is about the
        // goal and not about the fixture.
        let (honest, root) = chain_proof();
        assert_eq!(honest.check(root, &ctx), Ok(fact("a", T, "c")));
    }

    /// (4) A FORGED SHARING REFERENCE: a premise back-reference in the encoded arena is
    /// repointed at a different node.
    ///
    /// Three shapes. A reference to another VALID node is structurally legal and decodes
    /// cleanly, so only re-derivation can catch it — which is exactly the point. A
    /// reference out of range and a forward reference are refused at decode.
    #[test]
    fn a_forged_sharing_reference_is_rejected() {
        let rules = chain_rules();
        let edb = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &edb);
        let (arena, root) = chain_proof();
        let bytes = arena.encode(root);

        // The root is the last node and it is a `by_rule`, so the encoding's final field is
        // its premise list: `[…, premise[0], premise[1]]`, each a little-endian u64.
        let premise_0 = bytes.len() - 16;
        let premise_1 = bytes.len() - 8;
        assert_eq!(
            u64::from_le_bytes(bytes[premise_0..premise_1].try_into().expect("8 bytes")),
            0,
            "the layout must be the one this test patches"
        );
        assert_eq!(
            u64::from_le_bytes(bytes[premise_1..].try_into().expect("8 bytes")),
            1
        );

        // (a) Repoint premise 0 at node 1 — a real, already-emitted node, so the root now
        // shares one axiom twice instead of citing two. Nothing about the SHAPE is
        // detectable: the reference is in range, it precedes its step, and the node it
        // names is a genuine subproof of a genuine EDB fact. Only re-derivation catches it
        // — `p(b, c)` binds `?s = b, ?m = c`, and the second body atom `p(?m, ?o)` then
        // demands a premise whose subject is `c`, which `p(b, c)` is not.
        let mut shared = bytes.clone();
        shared[premise_0..premise_1].copy_from_slice(&1u64.to_le_bytes());
        let (forged, forged_root) = ProofArena::decode(&shared).expect("a legal shape decodes");
        assert_eq!(
            forged.premises(forged_root)[0],
            forged.premises(forged_root)[1],
            "the forgery really did turn two premises into one shared node"
        );
        assert_eq!(
            forged.check(forged_root, &ctx),
            Err(ProofError::PremiseMismatch {
                rule: 0,
                body_position: 1,
                proven: Box::new(fact("b", P, "c")),
            }),
            "a premise repointed at the wrong shared node is caught by re-derivation"
        );

        // (b) A reference past the end of the emitted nodes.
        let mut out_of_range = bytes.clone();
        out_of_range[premise_1..].copy_from_slice(&9u64.to_le_bytes());
        assert!(
            matches!(
                ProofArena::decode(&out_of_range),
                Err(ProofError::Malformed { .. })
            ),
            "a premise reference outside the proof is refused at decode"
        );

        // (c) A FORWARD reference — the root citing itself — which would be a cycle.
        let mut forward = bytes.clone();
        forward[premise_1..].copy_from_slice(&2u64.to_le_bytes());
        assert!(
            matches!(
                ProofArena::decode(&forward),
                Err(ProofError::Malformed { .. })
            ),
            "a premise that does not precede its step is refused at decode"
        );

        // The unpatched bytes still check, so the three rejections are about the patches.
        let (honest, honest_root) = ProofArena::decode(&bytes).expect("the honest proof decodes");
        assert_eq!(honest.check(honest_root, &ctx), Ok(fact("a", T, "c")));
    }

    // ── Would it catch an unsound ENGINE? ───────────────────────────────────────

    /// A MUTATED engine output is rejected. For every derivation of a real run, re-attribute
    /// it to each OTHER clause of the same program and demand that the proof fails to check.
    ///
    /// This is the difference between catching a corrupted record and catching an unsound
    /// engine: the record is perfectly well-formed and internally consistent — a real fact,
    /// real premises, a real clause of the real program — and it is rejected anyway, because
    /// the checker recomputes the conclusion instead of reading it.
    #[test]
    fn re_attributing_a_derivation_to_another_clause_is_rejected() {
        let workload = synth_corpus::transitive_closure(4);
        let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
        let evaluation =
            evaluate(&exe, workload.edb()).expect("the corpus stays inside every ceiling");
        let edb = workload.edb();
        let ctx = ProofContext::new(&workload.rules, &edb, evaluation.facts());
        let proofs = EvaluationProofs::of(&evaluation);

        let mut mutations = 0usize;
        for derivation in evaluation.derivations() {
            for other in 0..workload.rules.len() {
                if other == derivation.rule() {
                    continue;
                }
                // Rebuild THIS step against another clause, keeping its real premises.
                let mut arena = proofs.arena().clone();
                let premises: Vec<ProofId> = derivation
                    .sources()
                    .iter()
                    .map(|source| {
                        proofs
                            .root_for(source)
                            .unwrap_or_else(|| arena.axiom(source.clone()))
                    })
                    .collect();
                let mutated = arena.by_rule(derivation.fact().clone(), other, &premises);
                assert!(
                    arena.check(mutated, &ctx).is_err(),
                    "clause {other} does not license {:?} from {:?}",
                    derivation.fact(),
                    derivation.sources()
                );
                mutations += 1;
            }
        }
        assert!(mutations > 0, "the mutation sweep must actually mutate");
    }

    /// A fabricated conclusion — a fact the least model does NOT contain — cannot be given
    /// a proof out of real premises and a real rule.
    #[test]
    fn a_fabricated_conclusion_has_no_proof() {
        let rules = chain_rules();
        let edb = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &edb);
        let mut arena = ProofArena::new();
        let first = arena.axiom(fact("a", P, "b"));
        let second = arena.axiom(fact("b", P, "c"));
        // `t(a, a)` is not in the least model; no permutation of the real premises yields it.
        for premises in [
            vec![first, second],
            vec![second, first],
            vec![first, first],
            vec![second, second],
        ] {
            let claim = arena.by_rule(fact("a", T, "a"), 0, &premises);
            assert!(
                matches!(
                    arena.check(claim, &ctx),
                    Err(ProofError::GoalMismatch { .. } | ProofError::PremiseMismatch { .. })
                ),
                "no arrangement of the real premises derives t(a, a)"
            );
        }
    }

    // ── Negation ────────────────────────────────────────────────────────────────

    /// A negated body atom is RE-DECIDED against the model: a step whose guard is satisfied
    /// there is rejected even though every positive premise is genuine.
    #[test]
    fn a_satisfied_negated_premise_is_rejected() {
        // r(?s, ?o) :- base(?s, ?o), not q(?s, ?o).
        let base = "https://example.org/base";
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?o"),
            vec![
                atom("?s", base, "?o"),
                ClauseAtom::negated(v("?s"), Q, v("?o")),
            ],
        )];
        let edb = store_of(&[("a", base, "b"), ("c", base, "d"), ("a", Q, "b")]);
        let ctx = ProofContext::new(&rules, &edb, &edb);

        let mut arena = ProofArena::new();
        // `c base d` has no `q`, so the step holds.
        let allowed_premise = arena.axiom(fact("c", base, "d"));
        let allowed = arena.by_rule(fact("c", R, "d"), 0, &[allowed_premise]);
        assert_eq!(arena.check(allowed, &ctx), Ok(fact("c", R, "d")));

        // `a base b` DOES have a `q`, so the guard blocks it.
        let blocked_premise = arena.axiom(fact("a", base, "b"));
        let blocked = arena.by_rule(fact("a", R, "b"), 0, &[blocked_premise]);
        assert_eq!(
            arena.check(blocked, &ctx),
            Err(ProofError::NegatedPremiseSatisfied {
                rule: 0,
                body_position: 1,
            })
        );
    }

    /// Existential negation-as-failure: an unbound position is unconstrained, so
    /// `not q(?s, ?free)` reads "this subject has NO q at all" — and the checker decides it
    /// the same way the evaluator does.
    #[test]
    fn an_existential_negated_premise_probes_the_ground_positions_only() {
        let base = "https://example.org/base";
        let rules = vec![DlClause::datalog(
            atom("?s", R, "?o"),
            vec![
                atom("?s", base, "?o"),
                ClauseAtom::negated(v("?s"), Q, v("?free")),
            ],
        )];
        let edb = store_of(&[("a", base, "b"), ("c", base, "d"), ("a", Q, "zzz")]);
        let ctx = ProofContext::new(&rules, &edb, &edb);

        let mut arena = ProofArena::new();
        let allowed_premise = arena.axiom(fact("c", base, "d"));
        let allowed = arena.by_rule(fact("c", R, "d"), 0, &[allowed_premise]);
        assert_eq!(arena.check(allowed, &ctx), Ok(fact("c", R, "d")));

        let blocked_premise = arena.axiom(fact("a", base, "b"));
        let blocked = arena.by_rule(fact("a", R, "b"), 0, &[blocked_premise]);
        assert_eq!(
            arena.check(blocked, &ctx),
            Err(ProofError::NegatedPremiseSatisfied {
                rule: 0,
                body_position: 1,
            }),
            "a has SOME q, so the existential guard blocks it"
        );
    }

    /// A stratified-negation program's real derivations check, guard and all.
    #[test]
    fn a_stratified_negation_program_yields_checkable_proofs() {
        let base = "https://example.org/base";
        let rules = vec![
            DlClause::datalog(atom("?s", Q, "?o"), vec![atom("?s", P, "?o")]),
            DlClause::datalog(
                atom("?s", R, "?o"),
                vec![
                    atom("?s", base, "?o"),
                    ClauseAtom::negated(v("?s"), Q, v("?o")),
                ],
            ),
        ];
        let seed = || store_of(&[("a", P, "b"), ("a", base, "b"), ("c", base, "d")]);
        let exe = compile(rules.clone()).expect("the fixture compiles");
        let evaluation = evaluate(&exe, seed()).expect("the fixture stays inside every ceiling");
        let proofs = EvaluationProofs::of(&evaluation);
        let edb = seed();
        let ctx = ProofContext::new(&rules, &edb, evaluation.facts());
        assert!(!proofs.roots().is_empty());
        for (derived, root) in proofs.roots() {
            assert_eq!(proofs.arena().check(*root, &ctx).as_ref(), Ok(derived));
        }
    }

    // ── Hash-consing, ids, encoding ─────────────────────────────────────────────

    /// Structurally identical proofs intern to ONE id, so a shared subproof is stored once.
    #[test]
    fn structurally_identical_proofs_share_one_node() {
        let mut arena = ProofArena::new();
        let left = arena.axiom(fact("a", P, "b"));
        let right = arena.axiom(fact("a", P, "b"));
        assert_eq!(left, right, "identical axioms intern once");
        assert_eq!(arena.len(), 1);

        let first = arena.by_rule(fact("a", T, "c"), 0, &[left, left]);
        let second = arena.by_rule(fact("a", T, "c"), 0, &[right, right]);
        assert_eq!(first, second, "identical rule applications intern once");
        assert_eq!(arena.len(), 2);

        // A different rule index, a different goal and a different premise list are all
        // different terms.
        assert_ne!(first, arena.by_rule(fact("a", T, "c"), 1, &[left, left]));
        assert_ne!(first, arena.by_rule(fact("a", T, "d"), 0, &[left, left]));
        let other = arena.axiom(fact("b", P, "c"));
        assert_ne!(first, arena.by_rule(fact("a", T, "c"), 0, &[left, other]));
        assert!(!arena.is_empty());
    }

    /// The accessors report the term's shape without interpreting it.
    #[test]
    fn the_accessors_report_the_term_shape() {
        let (arena, root) = chain_proof();
        assert_eq!(arena.goal(root), &fact("a", T, "c"));
        assert_eq!(arena.rule(root), Some(0));
        assert_eq!(arena.premises(root).len(), 2);
        assert!(!arena.is_axiom(root));
        let premise = arena.premises(root)[0];
        assert!(arena.is_axiom(premise));
        assert_eq!(arena.rule(premise), None);
        assert!(arena.premises(premise).is_empty());
        assert_eq!(arena.goal(premise), &fact("a", P, "b"));
    }

    /// The encoding is canonical: it round-trips, it is byte-identical across rebuilds, and
    /// it depends on the proof's SHAPE rather than on the arena's construction sequence.
    #[test]
    fn the_encoding_is_canonical_and_round_trips() {
        let (arena, root) = chain_proof();
        let bytes = arena.encode(root);

        // Byte-identical across independent rebuilds.
        for _ in 0..8 {
            let (again, again_root) = chain_proof();
            assert_eq!(again.encode(again_root), bytes);
            assert_eq!(again.digest(again_root), arena.digest(root));
        }

        // The SAME proof built through a different interning sequence — an unrelated term
        // interned first, so every arena id moves — encodes to the same bytes.
        let mut shifted = ProofArena::new();
        let _unrelated = shifted.axiom(fact("z", P, "z"));
        let second = shifted.axiom(fact("b", P, "c"));
        let first = shifted.axiom(fact("a", P, "b"));
        let shifted_root = shifted.by_rule(fact("a", T, "c"), 0, &[first, second]);
        assert_ne!(
            shifted_root, root,
            "the arena ids genuinely differ, or the test proves nothing"
        );
        assert_eq!(
            shifted.encode(shifted_root),
            bytes,
            "the encoding numbers nodes by emission position, never by arena id"
        );

        // Round trip, and a fixed point under re-encoding.
        let (decoded, decoded_root) = ProofArena::decode(&bytes).expect("the encoding decodes");
        assert_eq!(decoded.encode(decoded_root), bytes);
        assert_eq!(decoded.digest(decoded_root), arena.digest(root));
        assert_eq!(decoded.goal(decoded_root), &fact("a", T, "c"));
    }

    /// A shared subproof is emitted ONCE and referenced twice, so the encoding is linear in
    /// the DAG rather than in its paths.
    #[test]
    fn a_shared_subproof_is_emitted_once() {
        let mut arena = ProofArena::new();
        let shared = arena.axiom(fact("a", P, "a"));
        let root = arena.by_rule(fact("a", T, "a"), 0, &[shared, shared]);
        let (decoded, decoded_root) =
            ProofArena::decode(&arena.encode(root)).expect("the encoding decodes");
        assert_eq!(decoded.len(), 2, "one axiom and one step, not three nodes");
        assert_eq!(
            decoded.premises(decoded_root)[0],
            decoded.premises(decoded_root)[1],
            "both premises address the same interned node"
        );
    }

    /// Every structural corruption of the encoding is a rejection, never a panic.
    #[test]
    fn a_corrupted_encoding_is_refused() {
        let (arena, root) = chain_proof();
        let bytes = arena.encode(root);

        // Truncation, at every prefix length.
        for cut in 0..bytes.len() {
            assert!(
                matches!(
                    ProofArena::decode(&bytes[..cut]),
                    Err(ProofError::Malformed { .. })
                ),
                "a stream truncated at {cut} must be refused"
            );
        }

        // Trailing bytes.
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(matches!(
            ProofArena::decode(&extra),
            Err(ProofError::Malformed { .. })
        ));

        // A wrong tag.
        let mut wrong_tag = bytes.clone();
        wrong_tag[8] ^= 0xff;
        assert!(matches!(
            ProofArena::decode(&wrong_tag),
            Err(ProofError::Malformed { .. })
        ));

        // An unknown node kind: the first node's kind byte follows the framed tag and the
        // node count.
        let kind_offset = 8 + PROOF_ENCODING_TAG.len() + 8;
        let mut wrong_kind = bytes;
        wrong_kind[kind_offset] = 9;
        assert!(matches!(
            ProofArena::decode(&wrong_kind),
            Err(ProofError::Malformed { .. })
        ));

        // An empty proof.
        let mut empty = Vec::new();
        frame(&mut empty, PROOF_ENCODING_TAG.as_bytes());
        empty.extend_from_slice(&0u64.to_le_bytes());
        assert!(matches!(
            ProofArena::decode(&empty),
            Err(ProofError::Malformed { .. })
        ));

        // A non-UTF-8 surface.
        let mut bad_utf8 = Vec::new();
        frame(&mut bad_utf8, PROOF_ENCODING_TAG.as_bytes());
        bad_utf8.extend_from_slice(&1u64.to_le_bytes());
        bad_utf8.push(KIND_AXIOM);
        frame(&mut bad_utf8, &[0xff, 0xfe]);
        for _ in 0..3 {
            frame(&mut bad_utf8, b"");
        }
        assert!(matches!(
            ProofArena::decode(&bad_utf8),
            Err(ProofError::Malformed { .. })
        ));
    }

    /// A clause with a non-Datalog head licenses no step and is refused by name.
    #[test]
    fn a_non_datalog_clause_licenses_no_step() {
        let rules = vec![DlClause::new(
            vec![
                HeadDisjunct::atom(atom("?s", T, "?o")),
                HeadDisjunct::atom(atom("?s", R, "?o")),
            ],
            Vec::new(),
            vec![atom("?s", P, "?m"), atom("?m", P, "?o")],
        )];
        let edb = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &edb);
        let (arena, root) = chain_proof();
        assert_eq!(
            arena.check(root, &ctx),
            Err(ProofError::NonDatalogRule {
                rule: 0,
                form: HeadForm::Disjunctive,
            })
        );
    }

    /// A head variable the checked premises never bound is refused rather than fabricated.
    ///
    /// `compile` refuses such a clause, so this proof is being presented against a program
    /// that was never compiled — which is a question the checker must still answer.
    #[test]
    fn an_unbound_head_variable_is_refused() {
        let rules = vec![DlClause::datalog(
            atom("?s", T, "?free"),
            vec![atom("?s", P, "?m"), atom("?m", P, "?o")],
        )];
        let edb = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &edb);
        let (arena, root) = chain_proof();
        assert_eq!(
            arena.check(root, &ctx),
            Err(ProofError::UnboundHeadVariable {
                rule: 0,
                variable: "?free".to_owned(),
            })
        );
    }

    /// A repeated variable is a filter the checker enforces: the same variable in two
    /// positions must bind the same surface.
    #[test]
    fn a_repeated_variable_must_agree() {
        // t(?s, ?s) :- p(?s, ?s).
        let rules = vec![DlClause::datalog(
            atom("?s", T, "?s"),
            vec![atom("?s", P, "?s")],
        )];
        let edb = store_of(&[("a", P, "a"), ("a", P, "b")]);
        let ctx = ProofContext::new(&rules, &edb, &edb);

        let mut arena = ProofArena::new();
        let diagonal = arena.axiom(fact("a", P, "a"));
        let ok = arena.by_rule(fact("a", T, "a"), 0, &[diagonal]);
        assert_eq!(arena.check(ok, &ctx), Ok(fact("a", T, "a")));

        let off_diagonal = arena.axiom(fact("a", P, "b"));
        let bad = arena.by_rule(fact("a", T, "a"), 0, &[off_diagonal]);
        assert_eq!(
            arena.check(bad, &ctx),
            Err(ProofError::PremiseMismatch {
                rule: 0,
                body_position: 0,
                proven: Box::new(fact("a", P, "b")),
            }),
            "p(a, b) does not match p(?s, ?s)"
        );
    }

    /// A rule with a VARIABLE PREDICATE re-derives: the checker binds the predicate position
    /// like any other, because it is a term.
    #[test]
    fn a_variable_predicate_step_re_derives() {
        let type_p = "https://example.org/type";
        let domain_p = "https://example.org/domain";
        let rules = vec![DlClause::datalog(
            ClauseAtom::positive(v("?x"), type_p, v("?c")),
            vec![
                ClauseAtom::positive(v("?p"), domain_p, v("?c")),
                ClauseAtom::quad(v("?x"), v("?p"), v("?y"), ClauseTerm::DefaultGraph),
            ],
        )];
        let seed = || {
            store_of(&[
                ("https://example.org/p1", domain_p, "https://example.org/C1"),
                (
                    "https://example.org/x",
                    "https://example.org/p1",
                    "https://example.org/y",
                ),
            ])
        };
        let exe = compile(rules.clone()).expect("the fixture compiles");
        let evaluation = evaluate(&exe, seed()).expect("the fixture stays inside every ceiling");
        let proofs = EvaluationProofs::of(&evaluation);
        let edb = seed();
        let ctx = ProofContext::new(&rules, &edb, evaluation.facts());
        assert_eq!(proofs.roots().len(), 1);
        for (derived, root) in proofs.roots() {
            assert_eq!(proofs.arena().check(*root, &ctx).as_ref(), Ok(derived));
        }
    }

    /// A rule over a NAMED GRAPH re-derives, graph position included.
    #[test]
    fn a_named_graph_step_re_derives() {
        let g1 = surface("https://example.org/g1");
        let rules = vec![DlClause::datalog(
            ClauseAtom::quad(
                v("?s"),
                ClauseTerm::iri(T),
                v("?o"),
                ClauseTerm::iri("https://example.org/g1"),
            ),
            vec![ClauseAtom::quad(
                v("?s"),
                ClauseTerm::iri(P),
                v("?o"),
                ClauseTerm::iri("https://example.org/g1"),
            )],
        )];
        let seed = || {
            let mut store = RelationStore::new();
            store.insert(&surface("a"), &surface(P), &surface("b"), &g1);
            store
        };
        let exe = compile(rules.clone()).expect("the fixture compiles");
        let evaluation = evaluate(&exe, seed()).expect("the fixture stays inside every ceiling");
        let proofs = EvaluationProofs::of(&evaluation);
        let edb = seed();
        let ctx = ProofContext::new(&rules, &edb, evaluation.facts());
        assert_eq!(proofs.roots().len(), 1);
        let (derived, root) = &proofs.roots()[0];
        assert_eq!(derived.graph, g1);
        assert_eq!(proofs.arena().check(*root, &ctx).as_ref(), Ok(derived));

        // Checking the same proof against a program that reads a different graph fails.
        let other = vec![DlClause::datalog(
            ClauseAtom::quad(
                v("?s"),
                ClauseTerm::iri(T),
                v("?o"),
                ClauseTerm::iri("https://example.org/g2"),
            ),
            vec![ClauseAtom::quad(
                v("?s"),
                ClauseTerm::iri(P),
                v("?o"),
                ClauseTerm::iri("https://example.org/g2"),
            )],
        )];
        let other_ctx = ProofContext::new(&other, &edb, evaluation.facts());
        assert!(proofs.arena().check(*root, &other_ctx).is_err());
    }

    /// An unconditional rule — an empty positive body — proves its ground head with no
    /// premises at all.
    #[test]
    fn an_unconditional_rule_proves_a_ground_head() {
        let rules = vec![DlClause::datalog(
            ClauseAtom::positive(
                ClauseTerm::iri("https://example.org/a"),
                T,
                ClauseTerm::iri("https://example.org/b"),
            ),
            Vec::new(),
        )];
        let edb = RelationStore::new();
        let ctx = ProofContext::new(&rules, &edb, &edb);
        let mut arena = ProofArena::new();
        let expected = fact("https://example.org/a", T, "https://example.org/b");
        let root = arena.by_rule(expected.clone(), 0, &[]);
        assert_eq!(arena.check(root, &ctx), Ok(expected));
    }

    /// The context hands back exactly what it was given.
    #[test]
    fn the_context_reports_its_three_inputs() {
        let rules = chain_rules();
        let edb = chain_edb();
        let model = chain_edb();
        let ctx = ProofContext::new(&rules, &edb, &model);
        assert_eq!(ctx.rules().len(), 1);
        assert_eq!(ctx.edb().row_count(), 2);
        assert_eq!(ctx.model().row_count(), 2);
    }

    /// `EvaluationProofs` is addressable by fact and is built in lexical fact order.
    #[test]
    fn evaluation_proofs_are_addressable_and_lexically_ordered() {
        let workload = synth_corpus::transitive_closure(3);
        let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
        let evaluation =
            evaluate(&exe, workload.edb()).expect("the corpus stays inside every ceiling");
        let proofs = EvaluationProofs::of(&evaluation);
        let facts: Vec<&Fact> = proofs.roots().iter().map(|(fact, _)| fact).collect();
        let mut sorted = facts.clone();
        sorted.sort();
        assert_eq!(facts, sorted, "roots are in lexical fact order");
        for (fact, root) in proofs.roots() {
            assert_eq!(proofs.root_for(fact), Some(*root));
        }
        assert_eq!(proofs.root_for(&fact("nope", P, "nope")), None);
    }

    /// Proof building is DETERMINISTIC end to end: two runs of the same program over the
    /// same facts produce the same arena size, the same roots and byte-identical encodings.
    ///
    /// The encodings are what the assertion is really about. Arena ids could in principle
    /// agree by luck; identical bytes for every proof of every derived fact means the
    /// interning sequence, the emission walk and the surfaces all agree.
    #[test]
    fn proofs_are_byte_reproducible() {
        let workload = synth_corpus::same_generation(2);
        let build = || {
            let exe = compile(workload.rules.clone()).expect("the corpus program compiles");
            let evaluation =
                evaluate(&exe, workload.edb()).expect("the corpus stays inside every ceiling");
            let proofs = EvaluationProofs::of(&evaluation);
            let encodings: Vec<(Fact, Vec<u8>, [u8; 32])> = proofs
                .roots()
                .iter()
                .map(|(fact, root)| {
                    (
                        fact.clone(),
                        proofs.arena().encode(*root),
                        proofs.arena().digest(*root),
                    )
                })
                .collect();
            (proofs.arena().len(), encodings)
        };
        let reference = build();
        assert!(reference.1.len() > 1, "the fixture must derive something");
        for _ in 0..4 {
            assert_eq!(build(), reference);
        }
    }

    /// Every rejection renders a diagnostic naming what went wrong, and is a
    /// `std::error::Error`.
    #[test]
    fn rejections_render() {
        let errors = [
            ProofError::Malformed {
                detail: "truncated".to_owned(),
            },
            ProofError::NotAsserted {
                goal: Box::new(fact("a", P, "b")),
            },
            ProofError::UnknownRule { rule: 3 },
            ProofError::NonDatalogRule {
                rule: 1,
                form: HeadForm::Existential,
            },
            ProofError::PremiseCountMismatch {
                rule: 0,
                body: 2,
                premises: 1,
            },
            ProofError::PremiseMismatch {
                rule: 0,
                body_position: 1,
                proven: Box::new(fact("a", P, "b")),
            },
            ProofError::NegatedPremiseSatisfied {
                rule: 0,
                body_position: 1,
            },
            ProofError::UnboundHeadVariable {
                rule: 0,
                variable: "?free".to_owned(),
            },
            ProofError::GoalMismatch {
                rule: 0,
                derived: Box::new(fact("a", T, "c")),
                stated: Box::new(fact("a", T, "z")),
            },
        ];
        for error in &errors {
            let rendered = error.to_string();
            assert!(!rendered.is_empty(), "{error:?} renders nothing");
            let _: &dyn std::error::Error = error;
        }
        assert!(errors[1].to_string().contains("not in the seeded EDB"));
        assert!(errors[2].to_string().contains("clause 3"));
        assert!(errors[8].to_string().contains("not the stated"));
    }
}
