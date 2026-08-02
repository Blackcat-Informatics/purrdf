// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Why an answer holds — two explanations, of two different KINDS, under two names.
//!
//! An entailment engine that answers "yes" and nothing else is unauditable: a wrong answer
//! and a right answer are both just answers. This module is what makes each of the two
//! engines say why — and it deliberately does NOT give them one return type, because they
//! do not explain the same thing and a single type that meant both would be a type that
//! meant neither.
//!
//! | lane | type | what it is | how it is checked |
//! |---|---|---|---|
//! | chase | [`ChaseProof`] | the DERIVATION — which rule, from which premises | re-derive the head from the premises, against the clause program |
//! | tableau | [`Justification`] | a MINIMAL ENTAILING SUBSET of the ontology | re-decide over the subset, and confirm no axiom can be dropped |
//!
//! # The chase has a derivation, so its explanation IS one
//!
//! `purrdf-datalog`'s [`ProofArena`] holds independently checkable proof terms: a checker
//! takes the proof and the CLAUSE PROGRAM, walks the premises to the facts they establish,
//! matches the named rule's body against those facts, instantiates the head from the
//! resulting substitution, and returns *the fact it derived itself*. The proof's stated
//! conclusion is never an input to that computation. A step the rule does not license
//! cannot be made to check by writing a nicer record of it.
//!
//! [`explain_conclusion`] is that machinery pointed at a materialized triple. It rebuilds
//! the very store the closure was produced from — the engine's seeding is a pure function
//! of `(dataset, regime, program, graph)` — evaluates the declared program, and lifts the
//! target fact's derivation out as a proof term. **The proof is checked before it is
//! returned**, so an unresolvable antecedent is a hard error at construction rather than a
//! surprise at the caller's first `check()`; there is no path by which an unchecked
//! `ChaseProof` exists.
//!
//! # The tableau has NO derivation, so its explanation is a JUSTIFICATION
//!
//! A tableau does not derive a conclusion from premises by named steps. It refutes: it
//! asserts the negation of the question and closes every branch of a search. There is no
//! rule to name, no premise list, and nothing for [`ProofArena::check`] to re-derive —
//! trying to force the Datalog proof machinery onto it would mean inventing a derivation the
//! reasoner never performed.
//!
//! What a description logic *does* have is the standard notion of a **justification**: a
//! subset `J ⊆ O` such that `J ⊨ α` and no proper subset of `J` entails `α`. That is a
//! statement about the ontology rather than about the search, it is exactly what a user
//! asking "why?" wants — *these* axioms, and every one of them is needed — and both halves
//! are CHECKABLE without trusting the search that found them:
//!
//! * SUFFICIENCY — re-decide `α` over `J` alone. [`Justification::is_sufficient`].
//! * MINIMALITY — for each axiom of `J`, re-decide `α` over `J` without it, and require
//!   every one of those decisions to be negative. [`Justification::is_minimal`].
//!
//! [`justify`] finds one by BLACK-BOX shrinking: it never looks inside the tableau, so it
//! cannot be fooled by the tableau's own bookkeeping, and it spends one entailment decision
//! per candidate axiom. That cost is reported as [`Justification::decisions`] rather than
//! hidden.
//!
//! # Identity is a content digest, never an IRI
//!
//! PurRDF mints no vocabulary, so neither explanation is NAMED by a fabricated derivation
//! IRI, and a justification introduces no term at all: it is a set of axioms already present
//! in the input, emitted as an RDF 1.2 dataset holding exactly those axioms' triples. Where
//! an identifier is genuinely useful it is a BLAKE3 content digest —
//! [`Justification::digest`] over the canonical N-Quads of the justification,
//! [`ChaseProof::digest`] over the proof term's canonical encoding — exposed as bytes or as
//! hex. If an RDF-facing identifier is ever wanted, the namespace is caller-supplied
//! configuration and the digest is what goes in it.
//!
//! # Determinism
//!
//! Both are functions of their input alone. A proof term's encoding numbers nodes by a
//! post-order first-visit walk rather than by arena id, so it carries no allocation order; a
//! justification shrinks over the dataset's quads in source order and emits in source order,
//! through ordered sets throughout. Two runs over one input produce byte-identical digests,
//! on native targets and on `wasm32` alike.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadForm};
use purrdf_datalog::id::ProofId;
use purrdf_datalog::proof::{EvaluationProofs, ProofArena, ProofContext, ProofError};
use purrdf_datalog::resolve_fol::{FolBudget, FolControl, FolStatus, solve_datalog_goal};
use purrdf_datalog::seminaive::{compile, evaluate};
use purrdf_datalog::store::{Fact, RelationStore};

use crate::calculus::{ChaseRule, program_with_attribution};
use crate::engine::{seed, surface_of};
use crate::interner::{Interner, intern_into};
use crate::reasoner::{DlAxiom, Reasoner, Verdict};
use crate::rules::RuleId;
use crate::{EntailError, Regime};

/// Why an explanation could not be produced.
///
/// Every variant is a REFUSAL rather than an empty answer. An explanation that silently came
/// back empty would be indistinguishable from "there is nothing to explain", which is the
/// one thing a caller asking *why* must never be told by accident.
#[derive(Debug, Clone)]
pub enum ExplainError {
    /// The conclusion is neither asserted in the graph nor derived by the lane's closure.
    ///
    /// The unresolvable-antecedent case, and a HARD ERROR by construction: there is no
    /// derivation to hand back, so an explanation of it would have to be invented.
    NotDerived {
        /// The triple that has no derivation, rendered as canonical N-Quads terms.
        conclusion: String,
    },
    /// The forward chase and the backward resolver disagree about the conclusion.
    ///
    /// Two independent engines answered the same question over the same clause program and
    /// reached opposite verdicts. Which one is wrong is exactly what is not known here, so
    /// the chase's proof is not handed back.
    BackwardDisagreement {
        /// The conclusion the chase derived and the resolver refuted.
        conclusion: String,
    },
    /// The ontology does not entail the axiom, so it has no justification.
    ///
    /// A justification is a subset that ENTAILS; if the whole ontology does not, no subset
    /// of it does either, and the shrinking search would return the empty set — an answer
    /// that reads as "nothing is needed" and means the opposite.
    NotEntailed,
    /// The tableau could not DECIDE the axiom within its step cap, so neither a positive nor
    /// a negative answer is available to shrink against.
    ///
    /// Distinct from [`Self::NotEntailed`]: that one is a decided `no`, this is no decision.
    /// Shrinking against an undecided answer would produce a subset whose minimality claim
    /// rests on decisions that were never made.
    Undecided,
    /// The lane is evaluated by the RESTRICTED CHASE, whose derivations are not checkable
    /// proof terms.
    ///
    /// `RDF` and `RDFS` state four rules — `rdfD1`, `rdfD1a`, `rdfs14`, `rdfs14a` — whose
    /// conclusion is about a FRESH blank node. An existentially quantified head has no
    /// Datalog semantics, so [`ProofArena::check`] has no head to instantiate and no
    /// substitution to instantiate it under: a "proof" of such a step could only be believed,
    /// which is precisely what a proof term exists not to require. The refusal names the
    /// regime rather than returning an unverifiable derivation.
    Existential(Regime),
    /// The run the explanation would have been drawn from failed.
    Entail(Box<EntailError>),
    /// The proof term did not CHECK against the program and the seeded store.
    ///
    /// Reachable only through an engine defect, and representable rather than a panic
    /// precisely because that is the defect this whole module exists to make observable: a
    /// wrong answer carrying a proof that fails to check is a gate failure, where a wrong
    /// answer carrying a panic is a crash report.
    Unchecked(ProofError),
}

impl std::fmt::Display for ExplainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotDerived { conclusion } => {
                write!(
                    f,
                    "no derivation for {conclusion}: it is neither asserted nor inferred"
                )
            }
            Self::BackwardDisagreement { conclusion } => write!(
                f,
                "the forward chase derived `{conclusion}` but backward SLG resolution \
                 searched the same clause program to a fixpoint and found no support for \
                 it; the two engines disagree and neither answer can be trusted"
            ),
            Self::NotEntailed => {
                write!(
                    f,
                    "the ontology does not entail the axiom, so it has no justification"
                )
            }
            Self::Undecided => write!(
                f,
                "the tableau did not decide the axiom within its step cap, so there is no \
                 answer to justify"
            ),
            Self::Existential(regime) => write!(
                f,
                "the {regime:?} lane is evaluated by the restricted chase, whose existential \
                 heads have no checkable proof term"
            ),
            Self::Entail(error) => write!(f, "the explained run failed: {error}"),
            Self::Unchecked(error) => write!(f, "the proof term did not check: {error}"),
        }
    }
}

impl std::error::Error for ExplainError {}

impl From<EntailError> for ExplainError {
    fn from(error: EntailError) -> Self {
        Self::Entail(Box::new(error))
    }
}

/// Render a 32-byte digest as lowercase hex.
fn hex(digest: [u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

// ── The chase lane: a checkable derivation ──────────────────────────────────────

/// WHY a chase conclusion holds: the derivation, re-derivable from the clause program.
///
/// Produced by [`explain_conclusion`], which CHECKS it before returning — so a value of this
/// type is one whose conclusion has already been re-derived from its premises by the named
/// clauses, over the same store the closure was produced from. [`Self::check`] is the same
/// computation a caller can run again, against the same context, and it re-derives rather
/// than re-reads: the proof's stated conclusion participates only as the value the derived
/// one is compared against.
///
/// This is emphatically NOT a [`Justification`]. A derivation says "this rule, on these
/// premises"; a justification says "these axioms, all of them needed". The chase has the
/// first and the tableau has only the second, and giving them one type would let a caller
/// write code that treats a tableau answer as though a rule had fired.
#[derive(Debug, Clone)]
pub struct ChaseProof {
    /// The lane whose calculus drew the conclusion.
    regime: Regime,
    /// The graph whose closure drew it; `None` is the default graph.
    graph: Option<TermValue>,
    /// The explained triple.
    conclusion: (TermValue, TermValue, TermValue),
    /// The hash-consed proof term — exactly the sub-DAG of this conclusion, re-interned
    /// from its own canonical encoding so nothing of the wider evaluation rides along.
    arena: ProofArena,
    /// The proof's root in [`Self::arena`].
    root: ProofId,
    /// The clause program the proof cites, by index.
    program: Vec<DlClause>,
    /// Clause index → the rule that clause states, for [`Self::rules`].
    attribution: Vec<ChaseRule>,
    /// The seeded store an axiom leaf must appear in.
    edb: RelationStore,
    /// The saturated store a negated body atom is re-decided against.
    model: RelationStore,
    /// What the independent backward re-derivation concluded.
    backward: BackwardCheck,
}

impl ChaseProof {
    /// The lane whose calculus drew the conclusion.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The graph whose closure drew the conclusion; `None` is the default graph.
    #[must_use]
    pub const fn graph(&self) -> Option<&TermValue> {
        self.graph.as_ref()
    }

    /// The explained triple, as `(subject, predicate, object)`.
    #[must_use]
    pub const fn conclusion(&self) -> &(TermValue, TermValue, TermValue) {
        &self.conclusion
    }

    /// Whether the conclusion was GIVEN rather than derived.
    ///
    /// An asserted triple is explained by the fact that it is asserted, which is a real
    /// explanation and a checkable one: [`ProofArena::axiom`] checks against the SEEDED
    /// store and nothing else, so a derived fact cannot be passed off as a given.
    #[must_use]
    pub fn is_asserted(&self) -> bool {
        self.arena.is_axiom(self.root)
    }

    /// How many distinct steps the proof has — interned nodes, so a premise appealed to
    /// twice is one step.
    ///
    /// The honest measure of an explanation's size, and the one a caller should look at
    /// before rendering it: a proof of a triple in a saturated closure can be deep.
    #[must_use]
    pub fn steps(&self) -> usize {
        self.arena.len()
    }

    /// What the independent backward re-derivation concluded about this proof.
    ///
    /// Reported on the certificate so a corroborated conclusion is distinguishable from an
    /// unexamined one. A [`BackwardCheck::Confirmed`] proof was derived twice, forward and
    /// backward, by engines sharing only the clause program.
    #[must_use]
    pub fn backward(&self) -> BackwardCheck {
        self.backward
    }

    /// Every rule the proof cites, in specification table order, deduplicated.
    ///
    /// The reader's summary of the derivation: not the shape of the proof, but which rules
    /// of the regime's own table had to fire for the conclusion to hold. An asserted
    /// conclusion cites none.
    #[must_use]
    pub fn rules(&self) -> Vec<RuleId> {
        let owl = matches!(self.regime, Regime::OwlRl);
        let mut cited: BTreeSet<usize> = BTreeSet::new();
        let mut stack = vec![self.root];
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node.index()) {
                continue;
            }
            if let Some(clause) = self.arena.rule(node) {
                cited.insert(clause);
            }
            stack.extend(self.arena.premises(node).iter().copied());
        }
        let ids: BTreeSet<RuleId> = cited
            .into_iter()
            .filter_map(|clause| self.attribution.get(clause))
            .map(|rule| rule.rule_id(owl))
            .collect();
        RuleId::ALL
            .iter()
            .copied()
            .filter(|id| ids.contains(id))
            .collect()
    }

    /// The proof term's canonical encoding.
    ///
    /// Node ids are positions in a post-order first-visit walk rather than arena handles,
    /// so two arenas that built the same proof through different sequences encode to the
    /// same bytes. This is what [`Self::digest`] digests.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.arena.encode(self.root)
    }

    /// The proof's identity: BLAKE3 over [`Self::encode`].
    ///
    /// A digest, never an IRI — PurRDF mints no vocabulary. Two runs over one input produce
    /// the same 32 bytes on every target.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        self.arena.digest(self.root)
    }

    /// [`Self::digest`] as lowercase hex.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        hex(self.digest())
    }

    /// RE-DERIVE the conclusion from the proof and the clause program.
    ///
    /// Returns the fact the CHECKER computed, which [`explain_conclusion`] has already
    /// compared against the stated conclusion once. Running it again is cheap and is the
    /// point: a consumer that received a `ChaseProof` over a wire need not trust the engine
    /// that produced it.
    ///
    /// # Errors
    ///
    /// [`ExplainError::Unchecked`] carrying the exact defect — an axiom leaf that is not in
    /// the seeded store, a premise the cited clause's body does not match, a head the
    /// premises do not bind, or a conclusion that differs from the one re-derived.
    pub fn check(&self) -> Result<Fact, ExplainError> {
        let ctx = ProofContext::new(&self.program, &self.edb, &self.model);
        self.arena
            .check(self.root, &ctx)
            .map_err(ExplainError::Unchecked)
    }
}

/// What the independent backward re-derivation concluded about a chase proof.
///
/// Reported on the certificate rather than kept internal, so a caller can tell a
/// corroborated conclusion from an unexamined one instead of assuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackwardCheck {
    /// SLG resolution found its own derivation of the conclusion.
    Confirmed,
    /// The search did not reach its fixpoint, so it has no opinion. `Partial` cannot
    /// support a refutation: "no answer yet" is not "no answer".
    Abstained,
    /// Not attempted, on COST rather than inability. `Rdfs` completes in ~4.8s and
    /// `OwlRl` is budget-cut at ~31s (release, this crate's fixtures); both would report
    /// `Confirmed` there. Neither is affordable per explanation, so neither is run.
    Skipped,
}

impl BackwardCheck {
    /// The certificate's word for this outcome.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Abstained => "abstained",
            Self::Skipped => "skipped",
        }
    }
}

/// The work budget the backward cross-check is charged against.
const BACKWARD_CROSS_CHECK_STEPS: u64 = 200_000;

/// One store surface string as the clause term that renders back to it.
fn clause_term_of(surface: &str) -> ClauseTerm {
    if surface.is_empty() {
        return ClauseTerm::default_graph();
    }
    match surface.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        Some(iri) => ClauseTerm::iri(iri),
        None => ClauseTerm::literal(surface),
    }
}

/// Re-derive `goal` BACKWARD and refuse if the two engines disagree.
///
/// The semi-naive chase reaches this conclusion forward from the seeded facts; SLG
/// resolution reaches it backward from the goal. They share the clause program and nothing
/// else — different algorithms, different data structures, different termination arguments
/// — so agreement is evidence and disagreement is a defect in one of them.
///
/// # Confirmation and refutation are not symmetric
///
/// A non-empty answer set CONFIRMS whatever the budget did: a derivation that was found
/// cannot be un-found by truncating the search. "No answer" means nothing until the search
/// has reached its fixpoint, so a truncated search abstains rather than manufacturing a
/// disagreement out of its own impatience.
///
/// # Which regimes are checked, and why not all of them
///
/// A refutation needs the search to reach its fixpoint; a confirmation does not. Both
/// halves of that are cheap for `Simple`, `Rdf` and `D`, whose rule tables are small and
/// largely schema-specific — they complete in microseconds.
///
/// `Rdfs` and `OwlRl` are skipped on COST, not on inability, and the difference matters
/// enough to state plainly. Measured in release on this crate's own fixtures: RDFS reaches
/// `Complete` in ~4.8s — its refutation branch is live, and skipping it gives up a check
/// that would fire — and OWL 2 RL is budget-cut to `Partial` at ~31s. Both report
/// `Confirmed` on those fixtures. Neither figure is affordable on a per-explanation
/// diagnostic, so both are [`BackwardCheck::Skipped`] and the certificate says so rather
/// than implying a corroboration that was never attempted.
///
/// The backward resolver has NO separate EDB channel — a clause program is the whole story
/// for it — so seeded facts are appended as empty-bodied clauses.
///
/// # Errors
///
/// [`ExplainError::BackwardDisagreement`] when the search reaches its fixpoint and finds no
/// support for a goal the chase derived.
fn confirm_backward(
    regime: Regime,
    program: &[DlClause],
    seeded: &RelationStore,
    goal: &Fact,
) -> Result<BackwardCheck, ExplainError> {
    if matches!(regime, Regime::Rdfs | Regime::OwlRl) {
        return Ok(BackwardCheck::Skipped);
    }
    let mut clauses = program.to_vec();
    for fact in seeded.facts_sorted() {
        clauses.push(DlClause::datalog(
            ClauseAtom::quad(
                clause_term_of(&fact.subject),
                clause_term_of(&fact.predicate),
                clause_term_of(&fact.object),
                clause_term_of(&fact.graph),
            ),
            vec![],
        ));
    }
    let atom = ClauseAtom::quad(
        clause_term_of(&goal.subject),
        clause_term_of(&goal.predicate),
        clause_term_of(&goal.object),
        clause_term_of(&goal.graph),
    );
    let budget = FolBudget {
        max_steps: BACKWARD_CROSS_CHECK_STEPS,
    };
    // A program this resolver cannot lower is an abstention, not a disagreement.
    let Ok((_dag, control)) = solve_datalog_goal(&clauses, &atom, &budget) else {
        return Ok(BackwardCheck::Abstained);
    };
    let FolControl::Decided(outcome) = control else {
        return Ok(BackwardCheck::Abstained);
    };
    if !outcome.answers.is_empty() {
        return Ok(BackwardCheck::Confirmed);
    }
    if outcome.status == FolStatus::Complete {
        return Err(ExplainError::BackwardDisagreement {
            conclusion: format!("{} {} {}", goal.subject, goal.predicate, goal.object),
        });
    }
    Ok(BackwardCheck::Abstained)
}

/// Explain ONE triple of `regime`'s closure over `graph`: which rules, from which premises.
///
/// `graph` names the closure to explain, under the defined dataset semantics
/// [`materialize`](crate::materialize) documents: `None` is the default graph, closed against
/// itself, and `Some(g)` is the named graph `g`, closed against the union of itself and the
/// default graph. A conclusion drawn in one graph therefore has an explanation in that
/// graph's run and, in general, in no other.
///
/// # Errors
///
/// [`ExplainError::Existential`] for `RDF` and `RDFS`, whose four blank-node-minting rules
/// have no checkable proof term; [`ExplainError::NotDerived`] if the triple is neither
/// asserted in the seed nor derived by the closure — a hard error, because there is nothing
/// to explain and an empty answer would read as though there were; [`ExplainError::Entail`]
/// if the run itself fails; [`ExplainError::Unchecked`] if the constructed proof does not
/// re-derive, which is an engine defect made visible rather than shipped.
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_entail::{Regime, RuleId, explain_conclusion};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let tom = b.intern_iri("http://example.org/tom");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// b.push_quad(cat, sub, animal, None);
/// b.push_quad(tom, ty, cat, None);
/// let dataset = b.freeze().expect("freeze");
///
/// // `tom a Animal` is not asserted; cax-sco derives it.
/// let proof = explain_conclusion(
///     &dataset,
///     Regime::OwlRl,
///     None,
///     &TermValue::iri("http://example.org/tom"),
///     &TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
///     &TermValue::iri("http://example.org/Animal"),
/// )
/// .expect("the closure derives it");
/// assert!(!proof.is_asserted());
/// assert!(proof.rules().contains(&RuleId::CaxSco));
/// // …and the proof re-derives, from the clauses rather than from its own claim.
/// assert!(proof.check().is_ok());
/// ```
pub fn explain_conclusion(
    ds: &RdfDataset,
    regime: Regime,
    graph: Option<&TermValue>,
    subject: &TermValue,
    predicate: &TermValue,
    object: &TermValue,
) -> Result<ChaseProof, ExplainError> {
    let (full_program, attribution) = program_with_attribution(regime);
    // An existential head has no Datalog semantics — there is no head for the checker to
    // instantiate — so those clauses cannot appear in a checkable proof. They are dropped
    // here rather than used to refuse the whole REGIME.
    //
    // The distinction is load-bearing. `Rdfs` has four existential rules out of eighteen
    // (`rdfD1`, `rdfD1a`, `rdfs14`, `rdfs14a`), and refusing the regime on their account meant
    // a conclusion derived purely by `rdfs9` — an ordinary Datalog rule — could not be
    // explained under `rdfs` while the identical conclusion explained fine under `owl-rl`.
    // Refusing to explain X because sibling rule Y is existential is not doing X.
    let existential_present = full_program
        .iter()
        .any(|clause| clause.head_form() == HeadForm::Existential);
    let program: Vec<_> = full_program
        .iter()
        .filter(|clause| clause.head_form() != HeadForm::Existential)
        .cloned()
        .collect();

    let (edb, _terms) = seed(ds, regime, &program, graph)?;
    let goal = Fact {
        subject: surface_of(subject),
        predicate: surface_of(predicate),
        object: surface_of(object),
        graph: RelationStore::DEFAULT_GRAPH.to_owned(),
    };
    // The seeded store is what an AXIOM leaf checks against, so it has to outlive the
    // evaluation that consumes it.
    let seeded = edb.clone();
    let executable = compile(program.clone()).map_err(EntailError::Evaluate)?;
    let evaluation = evaluate(&executable, edb).map_err(EntailError::Evaluate)?;

    let proofs = EvaluationProofs::of(&evaluation);
    // Re-intern through the canonical encoding, so the returned arena is EXACTLY this
    // conclusion's sub-DAG rather than a handle into the whole evaluation's.
    let (arena, root) = match proofs.root_for(&goal) {
        Some(id) => {
            ProofArena::decode(&proofs.arena().encode(id)).map_err(ExplainError::Unchecked)?
        }
        None => {
            // Not derived. It may still be GIVEN, which is an explanation in its own right
            // and a checkable one — and if it is neither, that is the unresolvable
            // antecedent, refused by name.
            if !seeded.contains(&goal.subject, &goal.predicate, &goal.object, &goal.graph) {
                // The Datalog subset did not reach it. If the regime HAS existential rules,
                // one of them may be what derives it, and this checker cannot produce a term
                // for such a step — so the refusal is named rather than reported as
                // "not entailed", which would be a different and false answer.
                if existential_present {
                    return Err(ExplainError::Existential(regime));
                }
                return Err(ExplainError::NotDerived {
                    conclusion: format!("{} {} {}", goal.subject, goal.predicate, goal.object),
                });
            }
            let mut arena = ProofArena::new();
            let root = arena.axiom(goal.clone());
            (arena, root)
        }
    };

    // INDEPENDENT RE-DERIVATION. The chase says this holds; a different engine — SLG
    // resolution over the same clause program — is asked the same question.
    let backward = confirm_backward(regime, &program, &seeded, &goal)?;

    let proof = ChaseProof {
        regime,
        graph: graph.cloned(),
        conclusion: (subject.clone(), predicate.clone(), object.clone()),
        arena,
        root,
        program,
        attribution,
        edb: seeded,
        model: evaluation.into_facts(),
        backward,
    };
    // CHECKED BEFORE IT ESCAPES. There is no constructor that skips this, so a `ChaseProof`
    // a caller holds is one whose conclusion has been re-derived at least once.
    let derived = proof.check()?;
    if derived != goal {
        return Err(ExplainError::Unchecked(ProofError::GoalMismatch {
            rule: usize::MAX,
            derived: Box::new(derived),
            stated: Box::new(goal),
        }));
    }
    Ok(proof)
}

// ── The tableau lane: a minimal entailing subset ────────────────────────────────

/// WHY a Description-Logic axiom is entailed: the minimal set of axioms that entails it.
///
/// A tableau performs no derivation steps — see the [module docs](self) — so this is not a
/// proof and is deliberately not called one. It is the standard notion of a
/// **justification**: a subset `J` of the ontology with `J ⊨ α`, from which no axiom can be
/// removed without losing the entailment. Both halves are re-decidable without trusting the
/// search that produced them, and both are asserted by the crate's own tests:
/// [`Self::is_sufficient`] and [`Self::is_minimal`].
///
/// It mints NO vocabulary. A justification is a set of axioms already present in the input,
/// so it is emitted as an ordinary RDF 1.2 dataset holding exactly those axioms' triples —
/// no `purrdf:` term, no caller-supplied namespace, no fabricated predicate. Where an
/// identifier is wanted, [`Self::digest`] is a BLAKE3 content digest over the canonical
/// N-Quads of that dataset.
#[derive(Debug, Clone)]
pub struct Justification {
    /// The axiom this justifies.
    axiom: DlAxiom,
    /// The minimal entailing subset, as a dataset holding exactly its triples.
    ontology: Arc<RdfDataset>,
    /// How many AXIOMS it holds — root triples, not the blank-node scaffolding they carry.
    axioms: usize,
    /// How many entailment decisions the shrinking search spent.
    decisions: u64,
}

impl Justification {
    /// The axiom this justifies.
    #[must_use]
    pub const fn axiom(&self) -> &DlAxiom {
        &self.axiom
    }

    /// The minimal entailing subset, as a dataset whose default graph holds its triples.
    #[must_use]
    pub const fn ontology(&self) -> &Arc<RdfDataset> {
        &self.ontology
    }

    /// How many AXIOMS the justification holds.
    ///
    /// Not its triple count: an axiom whose class expression lives under blank nodes carries
    /// that scaffolding with it, and half a class expression is not a class expression.
    #[must_use]
    pub const fn axioms(&self) -> usize {
        self.axioms
    }

    /// How many entailment decisions [`justify`] spent finding this.
    ///
    /// Exactly `1 + n` for an ontology of `n` axioms: the one decision that establishes the
    /// entailment before any shrinking begins, plus one per candidate axiom. Each is a full
    /// tableau decision under the reasoner's own step cap, so this is the number a caller
    /// sizing the call needs — shrinking is the expensive half of explaining a DL answer,
    /// and it is reported rather than reassured about.
    #[must_use]
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// The justification's identity: BLAKE3 over its canonical N-Quads.
    ///
    /// A CONTENT digest, never an IRI. Two justifications holding the same axioms have the
    /// same 32 bytes whatever order they were found in, because canonical N-Quads is a
    /// statement about a quad set rather than about an emission sequence.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        *blake3::hash(purrdf_core::canonicalize(&self.ontology).nquads.as_bytes()).as_bytes()
    }

    /// [`Self::digest`] as lowercase hex.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        hex(self.digest())
    }

    /// SUFFICIENCY, re-decided: does the justification alone still entail the axiom?
    ///
    /// The first half of the checkable analogue of a proof check. It re-runs the reasoner
    /// over the justification and nothing else, so it does not consult — and cannot be
    /// misled by — the search that produced it.
    ///
    /// # Errors
    ///
    /// [`EntailError::Parse`] if the justification is not a well-formed OWL graph, or
    /// [`EntailError::Unsatisfiable`] if it has no model at all.
    pub fn is_sufficient(&self) -> Result<bool, EntailError> {
        entails(&self.ontology, &self.axiom).map(|verdict| matches!(verdict, Verdict::True))
    }

    /// MINIMALITY, re-decided: does dropping any ONE axiom lose the entailment?
    ///
    /// The second half. For each axiom of the justification it re-decides over the
    /// justification without it and requires the answer to be anything but `True`. A `false`
    /// here means the justification is entailing but not minimal — it carries an axiom the
    /// entailment does not need — which is a weaker answer rather than a wrong one, and is
    /// why this is a QUESTION rather than an invariant baked into the type.
    ///
    /// The empty justification is minimal vacuously, and that is the right answer: an axiom
    /// entailed by the empty ontology (`⊤ ⊑ ⊤`, say) needs no axiom at all.
    ///
    /// # Errors
    ///
    /// As [`Self::is_sufficient`], for each of the subsets it decides.
    pub fn is_minimal(&self) -> Result<bool, EntailError> {
        let axioms = Axioms::of(&self.ontology);
        for dropped in 0..axioms.roots.len() {
            let kept: BTreeSet<usize> = axioms
                .roots
                .iter()
                .enumerate()
                .filter_map(|(index, _)| (index != dropped).then_some(index))
                .collect();
            let subset = axioms.emit(&kept)?;
            if matches!(entails(&subset, &self.axiom)?, Verdict::True) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Find a JUSTIFICATION for `axiom` in `ds`: a minimal subset of it that still entails.
///
/// Black-box shrinking — the tableau is never looked inside, only asked. One pass over the
/// ontology's axioms in source order, dropping each that turns out not to be needed, which
/// yields an IRREDUCIBLE subset: every axiom left is one whose removal loses the entailment,
/// which is exactly what [`Justification::is_minimal`] re-decides.
///
/// # Errors
///
/// [`ExplainError::NotEntailed`] if `ds` does not entail `axiom` — a hard error, because a
/// subset of a non-entailing ontology does not entail either and the search would return the
/// empty set, which reads as "nothing is needed" and means the opposite.
/// [`ExplainError::Undecided`] if the tableau ran out of step budget deciding the axiom over
/// the whole ontology, so there is no answer to shrink against. [`ExplainError::Entail`] for
/// a malformed ontology or one with no model.
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_entail::{DlAxiom, justify};
///
/// let mut b = RdfDatasetBuilder::new();
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let cat = b.intern_iri("http://example.org/Cat");
/// let mammal = b.intern_iri("http://example.org/Mammal");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let fish = b.intern_iri("http://example.org/Fish");
/// b.push_quad(cat, sub, mammal, None);
/// b.push_quad(mammal, sub, animal, None);
/// b.push_quad(fish, sub, animal, None);
/// let dataset = b.freeze().expect("freeze");
///
/// let axiom = DlAxiom::SubClassOf {
///     sub: TermValue::iri("http://example.org/Cat"),
///     sup: TermValue::iri("http://example.org/Animal"),
/// };
/// let justification = justify(&dataset, &axiom).expect("entailed");
/// // The chain, and NOT the sibling: two axioms of the three.
/// assert_eq!(justification.axioms(), 2);
/// assert!(justification.is_sufficient().expect("well formed"));
/// assert!(justification.is_minimal().expect("well formed"));
/// ```
pub fn justify(ds: &RdfDataset, axiom: &DlAxiom) -> Result<Justification, ExplainError> {
    match entails(ds, axiom)? {
        Verdict::True => {}
        Verdict::False => return Err(ExplainError::NotEntailed),
        Verdict::Unknown => return Err(ExplainError::Undecided),
    }

    // An unsatisfiable subset entails everything, which would let the search drop an axiom
    // for the wrong reason; it is not a `True` verdict, so `entails` reporting it as an
    // error keeps the candidate.
    let mut holds = |subset: &RdfDataset| Ok(matches!(entails(subset, axiom), Ok(Verdict::True)));
    // The tableau lane spends whatever the ontology costs and REPORTS it rather than
    // capping it; `Justification::decisions` is that number. A cap here would produce a
    // subset whose minimality claim quietly stopped being true partway through.
    let mut budget = u64::MAX;
    let shrunk = shrink_to_irreducible(ds, &mut budget, &mut holds)?;
    Ok(Justification {
        axiom: axiom.clone(),
        ontology: shrunk.subset,
        axioms: shrunk.axioms,
        // Plus the one decision that established the entailment before any shrinking began.
        decisions: 1 + shrunk.decisions,
    })
}

/// What a shrink produced: the subset, its size, its cost, and whether it finished.
///
/// See [`shrink_to_irreducible`] for why `irreducible` is reported rather than asserted.
#[derive(Debug)]
pub(crate) struct Shrunk {
    /// The surviving subset, holding exactly its axioms and the scaffolding they carry.
    pub(crate) subset: Arc<RdfDataset>,
    /// How many AXIOMS it holds — root triples, not the blank-node scaffolding they carry.
    pub(crate) axioms: usize,
    /// How many decisions the search spent.
    pub(crate) decisions: u64,
    /// Whether every candidate was tried, so no axiom of `subset` can be dropped.
    ///
    /// `false` only when `budget` ran out mid-pass. A caller that must not overclaim reads
    /// this rather than assuming: a subset that still SATISFIES `holds` is a correct answer
    /// whether or not the search got to try every axiom, and the difference is exactly
    /// whether "minimal" may be said about it.
    pub(crate) irreducible: bool,
}

/// One-pass BLACK-BOX shrink of `ds` to a subset that still satisfies `holds`.
///
/// # The mechanism is engine-agnostic, and that is the point of it living here
///
/// This is the search [`justify`] performs, with the decision procedure lifted out into a
/// parameter. Nothing in it knows what `holds` decides: it walks the ontology's axioms in
/// source order, drops each in turn, and puts back any whose removal loses the property.
/// What survives is IRREDUCIBLE with respect to `holds` — every axiom left is one whose
/// removal loses it — which is exactly what
/// [`Justification::is_minimal`] re-decides for the tableau lane and what
/// [`crate::entails::refutation`] re-decides for the refutation lane. Two callers, one
/// search; a second copy of this loop is how two "minimal" subsets come to mean two
/// different things.
///
/// The unit of removal is an AXIOM and not a triple, because half a class expression is not
/// a class expression: [`Axioms`] splits the input into the triples that STATE something
/// and the blank-node scaffolding they carry, and a dropped axiom takes its scaffolding
/// with it. That matters as much for a refutation over an `owl:AllDisjointProperties`
/// collection as for a justification over a general class inclusion.
///
/// `budget` is decremented once per decision and stops the pass when it reaches zero; the
/// result then reports `irreducible: false` rather than claiming a minimality the search did
/// not establish. A caller with nothing to cap passes [`u64::MAX`].
///
/// # Errors
///
/// [`EntailError::Build`] if a subset cannot be frozen, and whatever `holds` refuses with.
pub(crate) fn shrink_to_irreducible<F>(
    ds: &RdfDataset,
    budget: &mut u64,
    holds: &mut F,
) -> Result<Shrunk, EntailError>
where
    F: FnMut(&RdfDataset) -> Result<bool, EntailError>,
{
    let axioms = Axioms::of(ds);
    let mut kept: BTreeSet<usize> = (0..axioms.roots.len()).collect();
    let mut decisions = 0_u64;
    let mut irreducible = true;
    for candidate in 0..axioms.roots.len() {
        if *budget == 0 {
            irreducible = false;
            break;
        }
        if !kept.remove(&candidate) {
            continue;
        }
        let subset = axioms.emit(&kept)?;
        *budget -= 1;
        decisions += 1;
        if !holds(&subset)? {
            kept.insert(candidate);
        }
    }
    Ok(Shrunk {
        subset: axioms.emit(&kept)?,
        axioms: kept.len(),
        decisions,
        irreducible,
    })
}

/// Whether `ds` entails `axiom`, as the reasoner's own verdict.
///
/// An UNSATISFIABLE ontology is an error rather than `True`: it entails every axiom
/// vacuously, and letting that count as entailment would make the shrinking search drop
/// axioms because the remainder was inconsistent rather than because they were unneeded.
fn entails(ds: &RdfDataset, axiom: &DlAxiom) -> Result<Verdict, EntailError> {
    let mut reasoner = Reasoner::new(ds)?;
    Ok(*reasoner.entails(axiom)?.answer())
}

/// The ontology split into AXIOMS and the scaffolding they carry.
///
/// An "axiom" here is a triple that STATES something — one with a named subject, or one on a
/// blank subject nothing else points at, which is how a general class inclusion with a
/// complex left-hand side is written. Everything else on a blank subject is syntax for a
/// class expression and rides along with whichever axiom reaches it, because half a class
/// expression is not a class expression and a subset that truncated one would be malformed
/// rather than smaller.
///
/// Deterministic throughout: triples are indexed in source quad order, the blank index and
/// the kept set are ordered, and emission is in source order.
struct Axioms {
    /// The term interner the ids below index.
    interner: Interner,
    /// Every default-graph triple, in source quad order.
    triples: Vec<(u32, u32, u32)>,
    /// The indices of the AXIOM-stating triples, ascending.
    roots: Vec<usize>,
    /// Blank-node subject → the indices of the triples it carries.
    blanks: BTreeMap<u32, Vec<usize>>,
}

impl std::fmt::Debug for Axioms {
    /// The SHAPE of the split, not the interned ids.
    ///
    /// The term table is thousands of entries and printing it would bury the two numbers a
    /// reader of a debug line actually wants — how many triples there are and how many of
    /// them state an axiom — so it is elided, which `finish_non_exhaustive` says out loud.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Axioms")
            .field("triples", &self.triples.len())
            .field("roots", &self.roots.len())
            .finish_non_exhaustive()
    }
}

impl Axioms {
    /// Split `ds`.
    fn of(ds: &RdfDataset) -> Self {
        let mut interner = Interner::default();
        let mut triples: Vec<(u32, u32, u32)> = Vec::new();
        for quad in ds.quads() {
            if quad.g.is_some() {
                continue;
            }
            let s = interner.intern(ds.term_value(quad.s));
            let p = interner.intern(ds.term_value(quad.p));
            let o = interner.intern(ds.term_value(quad.o));
            triples.push((s, p, o));
        }
        let mut blanks: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (index, &(s, _, _)) in triples.iter().enumerate() {
            if matches!(interner.value(s), TermValue::Blank { .. }) {
                blanks.entry(s).or_default().push(index);
            }
        }
        // A blank node that appears as an OBJECT is scaffolding reached from somewhere; one
        // that never does is the subject of an axiom nothing points at, and dropping it
        // would silently lose a general class inclusion.
        let pointed: BTreeSet<u32> = triples.iter().map(|&(_, _, o)| o).collect();
        let roots: Vec<usize> = triples
            .iter()
            .enumerate()
            .filter(|&(_, &(s, _, _))| {
                !matches!(interner.value(s), TermValue::Blank { .. }) || !pointed.contains(&s)
            })
            .map(|(index, _)| index)
            .collect();
        Self {
            interner,
            triples,
            roots,
            blanks,
        }
    }

    /// Add the blank-node closure reachable from `term` to `out`.
    fn closure(&self, term: u32, out: &mut BTreeSet<usize>) {
        let mut stack = vec![term];
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        while let Some(node) = stack.pop() {
            if !matches!(self.interner.value(node), TermValue::Blank { .. }) || !seen.insert(node) {
                continue;
            }
            for &index in self.blanks.get(&node).map_or(&[][..], Vec::as_slice) {
                out.insert(index);
                stack.push(self.triples[index].2);
            }
        }
    }

    /// Freeze the subset holding the axioms at the root POSITIONS in `kept`, plus their
    /// blank-node closures.
    fn emit(&self, kept: &BTreeSet<usize>) -> Result<Arc<RdfDataset>, EntailError> {
        let mut emitted: BTreeSet<usize> = BTreeSet::new();
        for &position in kept {
            let index = self.roots[position];
            emitted.insert(index);
            let (s, _, o) = self.triples[index];
            self.closure(s, &mut emitted);
            self.closure(o, &mut emitted);
        }
        let mut b = RdfDatasetBuilder::new();
        for &index in &emitted {
            let (s, p, o) = self.triples[index];
            let s = intern_into(&mut b, self.interner.value(s));
            let p = intern_into(&mut b, self.interner.value(p));
            let o = intern_into(&mut b, self.interner.value(o));
            b.push_quad(s, p, o, None);
        }
        b.freeze()
            .map_err(|e| EntailError::Build(format!("freeze justification: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;

    /// `rdfs:subClassOf`.
    const SUB: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    /// `rdf:type`.
    const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    /// A fixture class.
    const CAT: &str = "http://example.org/Cat";
    /// A fixture class.
    const MAMMAL: &str = "http://example.org/Mammal";
    /// A fixture class.
    const ANIMAL: &str = "http://example.org/Animal";
    /// A fixture class the entailment does not need.
    const FISH: &str = "http://example.org/Fish";
    /// A fixture individual.
    const TOM: &str = "http://example.org/tom";

    /// Build a default-graph dataset from `(s, p, o)` IRI triples.
    fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for &(s, p, o) in triples {
            let s = b.intern_iri(s);
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// The chain fixture: `Cat ⊑ Mammal ⊑ Animal`, plus an irrelevant sibling and an
    /// irrelevant instance.
    fn chain() -> Arc<RdfDataset> {
        dataset(&[
            (CAT, SUB, MAMMAL),
            (MAMMAL, SUB, ANIMAL),
            (FISH, SUB, ANIMAL),
            (TOM, TYPE, CAT),
        ])
    }

    /// `Cat ⊑ Animal`.
    fn cat_is_an_animal() -> DlAxiom {
        DlAxiom::SubClassOf {
            sub: TermValue::iri(CAT),
            sup: TermValue::iri(ANIMAL),
        }
    }

    // ── The justification ───────────────────────────────────────────────────────

    /// BOTH HALVES, over the fixture that has something to shrink away.
    ///
    /// Sufficiency: the two-axiom chain still entails. Minimality: dropping either half of
    /// it loses the entailment. And the two irrelevant axioms are gone, which is the whole
    /// point of shrinking rather than returning the input.
    #[test]
    fn a_justification_is_sufficient_and_minimal() {
        let ds = chain();
        let justification = justify(&ds, &cat_is_an_animal()).expect("entailed");
        assert_eq!(justification.axioms(), 2);
        assert!(justification.is_sufficient().expect("well formed"));
        assert!(justification.is_minimal().expect("well formed"));

        // Named, not merely counted: the chain, and not the sibling or the instance.
        let lines: BTreeSet<String> = purrdf_core::canonicalize(justification.ontology())
            .nquads
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        assert_eq!(
            lines,
            [
                format!("<{CAT}> <{SUB}> <{MAMMAL}> ."),
                format!("<{MAMMAL}> <{SUB}> <{ANIMAL}> ."),
            ]
            .into_iter()
            .collect::<BTreeSet<String>>()
        );
    }

    /// MINIMALITY IS A CHECK, NOT A CLAIM — so a NON-minimal subset must fail it.
    ///
    /// Without this the `is_minimal` assertion above would be worth nothing: a method that
    /// returned `true` unconditionally would pass it. The whole ontology entails the axiom
    /// and is not minimal, and this is that stated as a failing check.
    #[test]
    fn a_non_minimal_subset_fails_the_minimality_check() {
        let ds = chain();
        let whole = Justification {
            axiom: cat_is_an_animal(),
            ontology: ds,
            axioms: 4,
            decisions: 0,
        };
        assert!(whole.is_sufficient().expect("well formed"));
        assert!(
            !whole.is_minimal().expect("well formed"),
            "the whole ontology carries two axioms the entailment does not need"
        );
    }

    /// SUFFICIENCY IS A CHECK TOO: a subset that does NOT entail fails it.
    #[test]
    fn an_insufficient_subset_fails_the_sufficiency_check() {
        let half = Justification {
            axiom: cat_is_an_animal(),
            ontology: dataset(&[(CAT, SUB, MAMMAL)]),
            axioms: 1,
            decisions: 0,
        };
        assert!(!half.is_sufficient().expect("well formed"));
        // …and it is vacuously minimal, which is the honest reading: there is nothing to
        // remove. Minimality alone is not evidence of anything, which is why both halves
        // are asserted everywhere and neither is allowed to stand for the other.
        assert!(half.is_minimal().expect("well formed"));
    }

    /// A justification of an ASSERTED axiom is the one triple that asserts it.
    #[test]
    fn an_asserted_axiom_justifies_itself() {
        let ds = chain();
        let axiom = DlAxiom::SubClassOf {
            sub: TermValue::iri(CAT),
            sup: TermValue::iri(MAMMAL),
        };
        let justification = justify(&ds, &axiom).expect("entailed");
        assert_eq!(justification.axioms(), 1);
        assert!(justification.is_sufficient().expect("well formed"));
        assert!(justification.is_minimal().expect("well formed"));
    }

    /// A CLASS ASSERTION shrinks to the instance triple plus the chain that types it.
    #[test]
    fn a_class_assertion_keeps_the_instance_and_the_chain() {
        let ds = chain();
        let axiom = DlAxiom::ClassAssertion {
            individual: TermValue::iri(TOM),
            class: TermValue::iri(ANIMAL),
        };
        let justification = justify(&ds, &axiom).expect("entailed");
        assert_eq!(justification.axioms(), 3);
        assert!(justification.is_sufficient().expect("well formed"));
        assert!(justification.is_minimal().expect("well formed"));
    }

    /// AN UNENTAILED AXIOM IS A HARD ERROR, never an empty justification.
    ///
    /// The empty set is a perfectly well-formed subset and it entails nothing, so returning
    /// it would read as "no axiom is needed for this" — the opposite of the truth.
    #[test]
    fn an_unentailed_axiom_is_refused_rather_than_justified_emptily() {
        let ds = chain();
        let axiom = DlAxiom::SubClassOf {
            sub: TermValue::iri(ANIMAL),
            sup: TermValue::iri(CAT),
        };
        assert!(matches!(
            justify(&ds, &axiom),
            Err(ExplainError::NotEntailed)
        ));
    }

    /// The digest is a CONTENT digest: equal for equal axiom sets, different otherwise, and
    /// stable across two runs.
    #[test]
    fn the_justification_digest_addresses_its_content() {
        let first = justify(&chain(), &cat_is_an_animal()).expect("entailed");
        let second = justify(&chain(), &cat_is_an_animal()).expect("entailed");
        assert_eq!(first.digest(), second.digest());
        assert_eq!(first.digest_hex().len(), 64);
        assert_eq!(hex(first.digest()), first.digest_hex());

        // The SAME two axioms reached from a different ontology digest identically: the
        // digest is over the justification, not over what it was carved out of.
        let bigger = dataset(&[
            (FISH, SUB, ANIMAL),
            (CAT, SUB, MAMMAL),
            (TOM, TYPE, FISH),
            (MAMMAL, SUB, ANIMAL),
        ]);
        let other = justify(&bigger, &cat_is_an_animal()).expect("entailed");
        assert_eq!(other.digest(), first.digest());

        // A DIFFERENT justification digests differently.
        let assertion = justify(
            &chain(),
            &DlAxiom::ClassAssertion {
                individual: TermValue::iri(TOM),
                class: TermValue::iri(ANIMAL),
            },
        )
        .expect("entailed");
        assert_ne!(assertion.digest(), first.digest());
    }

    /// The shrinking cost is REPORTED: one decision per candidate axiom, plus the one that
    /// established the entailment in the first place.
    #[test]
    fn the_decision_count_is_the_measured_cost() {
        let ds = chain();
        let justification = justify(&ds, &cat_is_an_animal()).expect("entailed");
        assert_eq!(
            justification.decisions(),
            1 + 4,
            "one bracketing decision plus one per axiom of the ontology"
        );
    }

    // ── The chase proof ─────────────────────────────────────────────────────────

    /// A DERIVED triple is explained by the rules that derived it, and the proof RE-DERIVES.
    #[test]
    fn a_derived_conclusion_carries_a_checkable_derivation() {
        let ds = chain();
        let proof = explain_conclusion(
            &ds,
            Regime::OwlRl,
            None,
            &TermValue::iri(TOM),
            &TermValue::iri(TYPE),
            &TermValue::iri(ANIMAL),
        )
        .expect("cax-sco derives it");
        assert!(!proof.is_asserted());
        assert_eq!(proof.regime(), Regime::OwlRl);
        assert!(proof.graph().is_none());
        assert!(
            proof.rules().contains(&RuleId::CaxSco),
            "{:?}",
            proof.rules()
        );
        assert!(proof.steps() > 1, "a derivation has premises");
        // The checker returns the fact IT derived, and it is the conclusion.
        let derived = proof.check().expect("the proof re-derives");
        assert_eq!(derived.subject, surface_of(&TermValue::iri(TOM)));
        assert_eq!(derived.object, surface_of(&TermValue::iri(ANIMAL)));
    }

    /// An ASSERTED triple is explained by being asserted — a checkable claim, since an
    /// axiom leaf checks against the SEEDED store and nothing else.
    #[test]
    fn an_asserted_conclusion_is_explained_as_given() {
        let ds = chain();
        let proof = explain_conclusion(
            &ds,
            Regime::OwlRl,
            None,
            &TermValue::iri(CAT),
            &TermValue::iri(SUB),
            &TermValue::iri(MAMMAL),
        )
        .expect("asserted");
        assert!(proof.is_asserted());
        assert_eq!(proof.steps(), 1);
        assert!(proof.rules().is_empty(), "a given fact cites no rule");
        assert!(proof.check().is_ok());
    }

    /// A TRIPLE NOTHING DERIVES IS A HARD ERROR — the unresolvable antecedent, refused by
    /// name rather than answered with an empty explanation.
    #[test]
    fn an_underivable_conclusion_is_refused() {
        let ds = chain();
        let error = explain_conclusion(
            &ds,
            Regime::OwlRl,
            None,
            &TermValue::iri(ANIMAL),
            &TermValue::iri(SUB),
            &TermValue::iri(CAT),
        )
        .expect_err("nothing derives it");
        let ExplainError::NotDerived { conclusion } = &error else {
            panic!("{error}");
        };
        assert!(conclusion.contains(ANIMAL), "{conclusion}");
        assert!(error.to_string().contains("no derivation"), "{error}");
    }

    /// AN EXISTENTIAL RULE IN THE TABLE DOES NOT REFUSE THE REGIME.
    ///
    /// Both branches, because the interesting failure is over-refusal rather than
    /// under-refusal. `Rdfs` carries four existential rules out of eighteen, and an earlier
    /// revision rejected the whole regime on their account — so `Cat ⊑ Animal`, derived purely
    /// by `rdfs11`, could not be explained under `rdfs` while the identical conclusion
    /// explained fine under `owl-rl`. The refusal now depends on the CONCLUSION, not the table.
    #[test]
    fn an_existential_rule_elsewhere_in_the_table_does_not_refuse_the_conclusion() {
        let ds = chain();

        // Derivable by an ordinary Datalog rule: explained, and the proof checks.
        let proof = explain_conclusion(
            &ds,
            Regime::Rdfs,
            None,
            &TermValue::iri(CAT),
            &TermValue::iri(SUB),
            &TermValue::iri(ANIMAL),
        )
        .expect("rdfs11 derives it and rdfs11 is a Datalog rule");
        assert!(proof.check().is_ok(), "the returned proof must re-derive");

        // Not reachable through the Datalog subset. The regime HAS existential rules, so one
        // of them may be what derives it and this checker cannot produce a term for such a
        // step — refused by name rather than reported as "not entailed", which would be a
        // different and false answer.
        let error = explain_conclusion(
            &ds,
            Regime::Rdfs,
            None,
            &TermValue::iri(FISH),
            &TermValue::iri(SUB),
            &TermValue::iri(CAT),
        )
        .expect_err("no Datalog rule derives it");
        assert!(
            matches!(error, ExplainError::Existential(Regime::Rdfs)),
            "{error}"
        );
    }

    /// A proof's digest is a content digest, and its encoding round-trips.
    #[test]
    fn a_chase_proof_digest_addresses_its_content() {
        let explain = || {
            explain_conclusion(
                &chain(),
                Regime::OwlRl,
                None,
                &TermValue::iri(TOM),
                &TermValue::iri(TYPE),
                &TermValue::iri(ANIMAL),
            )
            .expect("derived")
        };
        let first = explain();
        let second = explain();
        assert_eq!(first.digest(), second.digest());
        assert_eq!(first.encode(), second.encode());
        assert_eq!(first.digest_hex().len(), 64);
        // A DIFFERENT conclusion has a different proof.
        let other = explain_conclusion(
            &chain(),
            Regime::OwlRl,
            None,
            &TermValue::iri(CAT),
            &TermValue::iri(SUB),
            &TermValue::iri(ANIMAL),
        )
        .expect("scm-sco derives it");
        assert_ne!(other.digest(), first.digest());
    }

    /// A NAMED GRAPH's conclusion is explained in that graph's run, and the default
    /// graph's run cannot explain it.
    ///
    /// The dataset semantics and the explanation surface have to agree, or a caller could
    /// ask why a triple holds and be told it does not.
    #[test]
    fn a_named_graph_conclusion_is_explained_in_its_own_graph() {
        let mut b = RdfDatasetBuilder::new();
        let cat = b.intern_iri(CAT);
        let sub = b.intern_iri(SUB);
        let mammal = b.intern_iri(MAMMAL);
        let ty = b.intern_iri(TYPE);
        let tom = b.intern_iri(TOM);
        let g = b.intern_iri("http://example.org/g");
        b.push_quad(cat, sub, mammal, None);
        b.push_quad(tom, ty, cat, Some(g));
        let ds = b.freeze().expect("freeze");

        let graph = TermValue::iri("http://example.org/g");
        let proof = explain_conclusion(
            &ds,
            Regime::OwlRl,
            Some(&graph),
            &TermValue::iri(TOM),
            &TermValue::iri(TYPE),
            &TermValue::iri(MAMMAL),
        )
        .expect("the named graph's closure derives it");
        assert_eq!(proof.graph(), Some(&graph));
        assert!(proof.rules().contains(&RuleId::CaxSco));
        assert!(proof.check().is_ok());

        // The DEFAULT graph's run has no instance, so it has no explanation to give.
        assert!(matches!(
            explain_conclusion(
                &ds,
                Regime::OwlRl,
                None,
                &TermValue::iri(TOM),
                &TermValue::iri(TYPE),
                &TermValue::iri(MAMMAL),
            ),
            Err(ExplainError::NotDerived { .. })
        ));
    }

    /// THE CHECKER RE-DERIVES RATHER THAN RE-READS, and it closes circularity too.
    ///
    /// Without this, `check()` returning `Ok` would be evidence of nothing — a checker that
    /// compared the proof against itself would pass every proof ever written. Two forgeries
    /// are built from an HONEST proof and each is rejected for its own reason:
    ///
    /// * the same rule application restated against a conclusion it does not license is a
    ///   [`ProofError::GoalMismatch`] — the checker instantiated the head itself and got a
    ///   different fact;
    /// * a DERIVED fact asserted as a given is a [`ProofError::NotAsserted`] — an axiom leaf
    ///   may appeal to the SEEDED store and nothing else, which is what stops a proof
    ///   assuming its own conclusion.
    #[test]
    fn a_forged_proof_does_not_check() {
        let ds = chain();
        // `Cat ⊑ Animal` is derived by ONE step from two ASSERTED premises, which is what
        // makes the goal-mismatch forgery below reach the goal check rather than tripping
        // the axiom check first.
        let honest = explain_conclusion(
            &ds,
            Regime::OwlRl,
            None,
            &TermValue::iri(CAT),
            &TermValue::iri(SUB),
            &TermValue::iri(ANIMAL),
        )
        .expect("derived");
        assert!(honest.check().is_ok(), "the honest proof must check");

        let mut arena = ProofArena::new();
        let premises: Vec<ProofId> = honest
            .arena
            .premises(honest.root)
            .iter()
            .map(|&premise| arena.axiom(honest.arena.goal(premise).clone()))
            .collect();
        let rule = honest.arena.rule(honest.root).expect("a derived root");
        let mut lie = honest.arena.goal(honest.root).clone();
        lie.object = surface_of(&TermValue::iri(FISH));
        let root = arena.by_rule(lie, rule, &premises);
        let forged = ChaseProof {
            arena,
            root,
            ..honest.clone()
        };
        let error = forged.check().expect_err("the rule does not license it");
        assert!(
            matches!(
                error,
                ExplainError::Unchecked(ProofError::GoalMismatch { .. })
            ),
            "{error}"
        );

        // The circular forgery: claim the DERIVED conclusion as a given.
        let mut arena = ProofArena::new();
        let root = arena.axiom(honest.arena.goal(honest.root).clone());
        let circular = ChaseProof {
            arena,
            root,
            ..honest
        };
        let error = circular
            .check()
            .expect_err("a derived fact is not in the seeded store");
        assert!(
            matches!(
                error,
                ExplainError::Unchecked(ProofError::NotAsserted { .. })
            ),
            "{error}"
        );
    }
}
