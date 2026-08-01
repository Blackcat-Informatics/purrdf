// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **refutation** mechanism: assert the conclusion's negation, re-chase, and read
//! `false` as the proof.
//!
//! # The gap this closes, and why no rule could close it
//!
//! [`homomorphism`](super::homomorphism) matches a conclusion into the closure, and that is
//! complete for every conclusion the OWL 2 RL rule table can produce. It can produce a great
//! many, and it cannot produce a NEGATIVE FACT: every head in Profiles §4.3 Tables 4–9 is
//! either an assertional triple over named terms or `false`, and not one of the
//! seventy-eight concludes `owl:differentFrom`, or membership in an `owl:complementOf`
//! class. So a premise can ENTAIL such a conclusion — W3C publishes eight cases where it
//! does — while a forward chase over the table derives nothing to match against, forever,
//! however complete its coverage.
//!
//! The answer is not another rule. It is that **the seventeen `false`-concluding rules ARE
//! the profile's inconsistency calculus** — `eq-diff1..3`, `prp-irp`, `prp-asyp`,
//! `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`,
//! `cls-maxqc1`, `cls-maxqc2`, `cax-dw`, `cax-adc`, `dt-not-type` — and an inconsistency
//! calculus decides negative facts by refutation. Assert `α`'s negation into the premise,
//! re-run the SAME rule table, and [`EntailError::Inconsistent`] is the proof of `α`.
//!
//! Nothing is added to the rule table by this. `rules(Regime::OwlRl)` and
//! `implemented(Regime::OwlRl)` are still exactly the seventy-eight,
//! [`extensions`](crate::extensions) gains nothing, and
//! `the_refutation_lane_adds_no_rule` asserts all three: refutation is a proof STRATEGY over
//! the declared calculus, not a widening of it.
//!
//! # The soundness argument, in full, including its precondition
//!
//! The claim is:
//!
//! > if `KB` is CONSISTENT and `KB ∪ {¬α}` is inconsistent, then `KB ⊨ α`.
//!
//! Proof. Suppose `KB ⊭ α`. Then some model `I` of `KB` fails `α`, so `I ⊨ ¬α`, so `I` is a
//! model of `KB ∪ {¬α}` — contradicting its inconsistency. Hence `KB ⊨ α`. ∎
//!
//! The consistency of `KB` is not decoration on that argument, it is what makes the
//! conclusion worth anything: an INCONSISTENT `KB` makes `KB ∪ {¬α}` inconsistent for every
//! `α` whatsoever, so the same computation would "prove" every conclusion and this module
//! would report `Entailed` for all of them. The premise's consistency is therefore
//! established BEFORE any refutation runs, and it is established by the same chase —
//! `super::prepare` calls [`materialize`](crate::materialize), which refuses with
//! [`EntailError::Inconsistent`] the moment a
//! `false`-headed rule matches on the premise alone. `attempt` below is reachable only from
//! after that refusal has not happened, and
//! `an_inconsistent_premise_never_reaches_the_refutation_lane` is the falsifiable form of
//! that ordering.
//!
//! One corollary is worth stating because the shrinking search below depends on it: since
//! `KB` is consistent and the rule table is monotone in the facts, EVERY subset of `KB` is
//! consistent too. So a subset that refutes cannot be refuting on its own account, and the
//! search cannot drop an axiom because the remainder became inconsistent — the failure mode
//! [`justify`](crate::justify) has to guard against explicitly in the tableau lane.
//!
//! # What a FAILED refutation licenses, and what it does not
//!
//! If `KB ∪ {¬α}` produces no clash, that is a proof of `KB ⊭ α` only if the calculus is
//! COMPLETE for detecting the inconsistency of `KB ∪ {¬α}` — Profiles Theorem PR1's
//! hypothesis, which is that the ontology is inside the OWL 2 RL syntax. This module
//! therefore does not decide a failure at all: it reports "not established" and hands the
//! question back to [`super::precondition`], which already checks PR1's hypothesis against
//! the premise and answers [`Undecided`](super::EntailmentOutcome::Undecided) when it fails.
//! The negation itself cannot break that hypothesis — [`NegativeFact::negation`] mints only
//! `owl:sameAs` between two terms and `rdf:type` to a NAMED class, both of which are OWL 2
//! RL assertional axioms — and `a_negation_stays_inside_the_rl_syntax` asserts it.
//!
//! # Why `OWL-RL` and nothing else
//!
//! The argument above turns entirely on the premise's lane having an inconsistency calculus
//! that is complete for it. `Simple`, `RDF` and `RDFS` state NO rule whose head is `false`,
//! so a re-chase over them can never clash and a refutation lane there would be a
//! computation that always answers "not established" — silence dressed as a mechanism. `D`
//! states one (`dt-not-type`) and this crate states no completeness theorem for it at all.
//! So the lane is gated to `OWL-RL` by whitelist, and the four others fall out.
//!
//! # Cost, and the budget that bounds it
//!
//! Establishing an `owl:AllDifferent` over `n` members costs `n(n−1)/2` re-chases, and
//! shrinking each of them to a minimal entailing subset costs one more per premise axiom.
//! Both are bounded by [`REFUTATION_BUDGET`], and the two phases are ordered so the bound
//! can never change a VERDICT into a wrong one: every fact is established first, and only
//! then is whatever budget is left spent on evidence quality. Exhausting the budget during
//! establishment is [`super::UndecidedReason::RefutationBudget`],
//! never a refutation; exhausting it
//! during shrinking leaves a subset that still entails and reports
//! [`Refutation::is_irreducible`] as `false`, never a minimality claim the search did not
//! make.
//!
//! # Determinism
//!
//! The facts are lowered in the conclusion's own triple order, the premise subsets are
//! shrunk in source order through ordered sets, and the clash chosen is the FIRST in the
//! evaluator's own total derivation order. Two runs over one premise and one conclusion
//! produce the same warrant, on `wasm32` as on native.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{RdfDataset, TermValue};

use crate::calculus::concludes_false;
use crate::engine::Refuter;
use crate::entails::graph::{Triple, default_graph_triples};
use crate::entails::homomorphism::{Binding, Closure, substitute};
use crate::entails::negation::{self, NegativeFact, Read};
use crate::entails::warrant::{EntailmentMechanism, EntailmentWarrant, Replay};
use crate::entails::{Attempt, Established, Question, UndecidedReason};
use crate::explain::shrink_to_irreducible;
use crate::report::InconsistencyWitness;
use crate::{EntailError, Regime};

/// The refutation budget, in CHASE RE-RUNS per [`entails`](super::entails) call.
///
/// A step count and never a clock reading, so the bound is reproducible on every target
/// including `wasm32`. It is small — three orders of magnitude below
/// [`MATCH_BUDGET`](super::MATCH_BUDGET) — because the unit is enormous by comparison: one
/// step here is a complete evaluation of the seventy-eight-rule program over the premise,
/// where one step there is a single candidate triple visited. Sized so that every conclusion
/// in the W3C entailment corpus, and any `owl:AllDifferent` over a couple of dozen members,
/// finishes with room for a full shrink of each fact.
pub const REFUTATION_BUDGET: u64 = 512;

/// WHY one negative fact holds: the clash, the premise subset that produces it, and the
/// closure its rule body instances lie in.
///
/// This is the refutation lane's unit of evidence. Its three parts answer three different
/// questions and none of them substitutes for another:
///
/// * [`Self::witness`] names the `false`-concluding rule that fired and the triples that
///   satisfied it — WHICH of the profile's seventeen inconsistency rules did the work;
/// * [`Self::premises`] is a subset of the caller's premise that still refutes, shrunk one
///   axiom at a time by exactly the search [`justify`](crate::justify) uses — WHICH of the
///   caller's axioms the entailment needs;
/// * [`Self::closure_size`]'s closure is `premises ∪ ¬fact` together with everything the
///   re-chase derived from it — where a body instance that is not asserted came from.
#[derive(Debug, Clone)]
pub struct Refutation {
    /// The negative fact this proves.
    fact: NegativeFact,
    /// The rule whose premises were all satisfied, and the triples that satisfied them.
    witness: InconsistencyWitness,
    /// A subset of the premise over which the negation still clashes.
    premises: Arc<RdfDataset>,
    /// Whether the shrink tried every axiom, so no axiom of [`Self::premises`] can be
    /// dropped.
    irreducible: bool,
    /// How many chase re-runs the shrink spent.
    decisions: u64,
    /// The closure of `premises ∪ ¬fact`: the seeded triples plus everything derived.
    closure: Closure,
}

impl Refutation {
    /// The negative fact this refutation proves.
    #[must_use]
    pub const fn fact(&self) -> &NegativeFact {
        &self.fact
    }

    /// The rule that fired and the triples that satisfied it.
    #[must_use]
    pub const fn witness(&self) -> &InconsistencyWitness {
        &self.witness
    }

    /// The premise subset the entailment needs, as a dataset holding exactly its axioms.
    ///
    /// Every axiom of it is one whose removal loses the refutation — provided
    /// [`Self::is_irreducible`], which is the honest qualifier and is why that is a separate
    /// question rather than a promise made by this method's name.
    #[must_use]
    pub const fn premises(&self) -> &Arc<RdfDataset> {
        &self.premises
    }

    /// Whether the shrinking search tried every axiom of the premise.
    ///
    /// `false` means [`REFUTATION_BUDGET`] ran out mid-pass, so [`Self::premises`] still
    /// refutes but may carry an axiom the refutation does not need. The subset is a correct
    /// answer either way; the difference is exactly whether "minimal" may be said about it,
    /// and a caller that would print that word reads this first.
    #[must_use]
    pub const fn is_irreducible(&self) -> bool {
        self.irreducible
    }

    /// How many chase re-runs the shrinking search spent.
    ///
    /// Reported rather than hidden, in the same spirit as
    /// [`Justification::decisions`](crate::Justification::decisions): shrinking is the
    /// expensive half of explaining a refutation, and this is the number a caller sizing the
    /// call needs.
    #[must_use]
    pub const fn decisions(&self) -> u64 {
        self.decisions
    }

    /// How many distinct triples the closure of `premises ∪ ¬fact` holds.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// RE-CHECK this refutation against `premise`, WITHOUT running a reasoner.
    ///
    /// Four things, all of them lookups:
    ///
    /// 1. the rule the witness names is one of the seventeen whose conclusion is `false`. A
    ///    witness citing a rule that concludes a TRIPLE would be evidence of a derivation,
    ///    not of a contradiction, and the two are not interchangeable;
    /// 2. every triple of [`Self::premises`] is a default-graph triple of `premise`. This is
    ///    what BINDS the refutation to the premise it was issued for — a subset of some other
    ///    ontology fails here instead of being replayed against this one;
    /// 3. `premises ∪ ¬fact` lies in the refutation closure, so the closure really is a
    ///    closure OF the cited subset and the asserted negation rather than of something
    ///    else;
    /// 4. every triple the witness's rule body matched lies in that same closure — the rule
    ///    body instances are accounted for by the cited premise subset, the asserted
    ///    negation, and what the chase drew from them.
    ///
    /// # Why (4) is stated against the closure and not against `premises ∪ ¬fact` alone
    ///
    /// Because it would be FALSE, and quietly so. A `false`-headed rule matches whatever is
    /// in the store, asserted or derived: `prp-pdw` fires on
    /// `hasFather owl:propertyDisjointWith hasMother`, `Stewie hasFather Peter` and
    /// `Stewie hasMother Peter` — and the third of those is not asserted anywhere. It is
    /// what `eq-rep-o` derives from the asserted `Stewie hasMother Lois` once the negation
    /// `Peter owl:sameAs Lois` goes in. A check that demanded every body instance be
    /// asserted would reject exactly the refutations that do the interesting work, so it
    /// demands they be in the closure of the cited premises and the negation, which is the
    /// strongest containment that is true of all of them.
    ///
    /// # What this does NOT check
    ///
    /// That the closure really follows from `premises ∪ ¬fact`. That is the CHASE's claim,
    /// with its own checker — [`explain_conclusion`](crate::explain_conclusion) re-derives a
    /// closure triple from the clause program step by step — and the split is the same one
    /// [`verify`](super::verify) makes for the homomorphism arm, for the same reason: a
    /// checker that re-ran the chase would cost what the original call cost and give the
    /// caller no independent check at all.
    #[must_use]
    pub fn check(&self, premise: &RdfDataset) -> bool {
        if !concludes_false(self.witness.rule()) {
            return false;
        }
        let held: BTreeSet<Triple> = default_graph_triples(premise).into_iter().collect();
        let cited = default_graph_triples(&self.premises);
        if !cited.iter().all(|triple| held.contains(triple)) {
            return false;
        }
        if !cited
            .iter()
            .chain(&self.fact.negation())
            .all(|triple| self.closure.contains(triple))
        {
            return false;
        }
        self.witness.premises().iter().all(|body| {
            self.closure.contains(&[
                body.subject().clone(),
                body.predicate().clone(),
                body.object().clone(),
            ])
        })
    }
}

/// The evidence that a premise entails a conclusion whose negative facts had to be refuted.
///
/// A conclusion of this shape is generally MIXED — W3C's `disjointclasses-001` concludes
/// `Stewie rdf:type [ owl:complementOf Girl ]` and, in the same graph, `Girl rdf:type
/// owl:Class` — so this warrant carries both halves and neither is allowed to stand for the
/// other: [`Self::binding`] is the ordinary homomorphism that discharged the residual
/// triples, and [`Self::refutations`] is one refutation per negative fact. Every triple of
/// the conclusion is discharged by exactly one of the two, which is what
/// [`verify`](super::verify) re-establishes by lowering the conclusion again on the spot.
#[derive(Debug, Clone)]
pub struct RefutationWarrant {
    /// The regime the closures were computed under.
    regime: Regime,
    /// What each existential of the RESIDUAL conclusion triples was bound to.
    binding: Binding,
    /// The premise's own closure, which the residual triples map into.
    closure: Closure,
    /// One refutation per negative fact, in the conclusion's lowering order.
    refutations: Vec<Refutation>,
}

impl RefutationWarrant {
    /// The regime whose calculus refuted.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping that discharged the conclusion's residual (non-negative) triples.
    ///
    /// Empty when the conclusion is entirely negative — `AllDifferent` collections are,
    /// because their whole scaffold is consumed by the pairs it lowers to.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// One refutation per negative fact of the conclusion, in lowering order.
    #[must_use]
    pub fn refutations(&self) -> &[Refutation] {
        &self.refutations
    }

    /// How many distinct triples the PREMISE closure this warrant is against holds.
    ///
    /// The closure of `premises ∪ ¬fact` for each refutation is
    /// [`Refutation::closure_size`]; they are different closures and are counted separately.
    #[must_use]
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// The premise closure this warrant is against.
    pub(crate) const fn closure(&self) -> &Closure {
        &self.closure
    }

    /// This warrant with the fold's residual `binding` attached.
    pub(crate) fn with_binding(mut self, binding: Binding) -> Self {
        self.binding = binding;
        self
    }

    /// The closure triples that witness the conclusion's NON-NEGATIVE half, or `None` if the
    /// mapping leaves a position of it open.
    ///
    /// Deliberately not called `witnesses`: the negative half of the conclusion has no image
    /// in any closure — that is the whole reason it needed refuting — so a method that
    /// claimed to render "the conclusion's witnesses" would be rendering half of one. What
    /// witnesses the other half is [`Refutation::witness`], one per fact.
    #[must_use]
    pub fn residual_witnesses(&self, conclusion: &RdfDataset) -> Option<Vec<[TermValue; 3]>> {
        let lowering = negation::lowering(conclusion)?;
        lowering
            .residual(&default_graph_triples(conclusion))
            .iter()
            .map(|pat| {
                Some([
                    substitute(&pat[0], &self.binding)?,
                    substitute(&pat[1], &self.binding)?,
                    substitute(&pat[2], &self.binding)?,
                ])
            })
            .collect()
    }
}

/// Try to establish `conclusion` from `premise` by refutation.
///
/// `closure` is the premise's own closure, already computed and indexed by
/// [`prepare`](super::prepare) — which is also where the premise's CONSISTENCY was
/// established. See the [module docs](self) for why that ordering is the whole soundness
/// argument and not a detail of it.
///
/// # Errors
///
/// Whatever the re-chase refuses with: [`EntailError::Evaluate`] for an evaluation ceiling,
/// [`EntailError::MalformedList`] for a premise whose OWL collections are not well formed,
/// [`EntailError::Build`] for a subset that cannot be frozen, and
/// [`EntailError::MatchBudget`] from the residual match.
pub(crate) fn attempt(q: &Question<'_>) -> Result<Attempt, EntailError> {
    let Question {
        premise,
        conclusion,
        regime,
        closure,
        pending,
        ..
    } = *q;
    // WHITELIST, not blacklist: the four other regimes fall out rather than being served by
    // a calculus whose completeness for inconsistency this crate does not claim.
    if !matches!(regime, Regime::OwlRl) {
        return Ok(Attempt::NotApplicable);
    }
    let lowering = match negation::lower(conclusion) {
        Read::Lowered(lowering) => lowering,
        Read::NotApplicable => return Ok(Attempt::NotApplicable),
        // RECOGNIZED AND DECLINED. An admission of incapacity, which must never come out of
        // the service as a refutation — see [`super::Attempt::Disqualified`].
        Read::Declined(constructs) => {
            return Ok(Attempt::Disqualified(UndecidedReason::ConstructNotRead {
                lane: EntailmentMechanism::Refutation,
                constructs,
            }));
        }
    };
    // Every triple this lane reads must still be outstanding: a triple an earlier lane already
    // discharged is not this one's to decide a second time.
    if !lowering
        .consumed
        .iter()
        .all(|index| pending.contains(index))
    {
        return Ok(Attempt::NotApplicable);
    }

    let discharged = lowering.consumed;
    let facts = lowering.facts;
    let mut budget = REFUTATION_BUDGET;
    // ESTABLISHMENT BEFORE EVIDENCE. Every fact is refuted first, so a budget spent on
    // shrinking can never be the reason a verdict came out differently.
    let needed = facts.len() as u64;
    if needed > budget {
        return Ok(Attempt::Undecided(UndecidedReason::RefutationBudget(
            needed,
        )));
    }

    let mut refuter = Refuter::new(regime);
    let mut seeded = refuter.seed(premise)?;
    let mut clashes = Vec::with_capacity(facts.len());
    for fact in &facts {
        budget -= 1;
        let Some(clash) = refuter.refute(&mut seeded, &fact.negation())? else {
            return Ok(Attempt::NotEstablished);
        };
        clashes.push(clash);
    }
    drop(seeded);

    // Whatever is left, split evenly: one fact's evidence must not be able to starve
    // another's, and an even split is a function of the question rather than of the order
    // the facts happen to be in.
    let share = budget / needed;
    let mut refutations = Vec::with_capacity(facts.len());
    for (fact, established) in facts.into_iter().zip(clashes) {
        let negation = fact.negation();
        let mut best = established;
        let mut allowance = share;
        // The one search, shared with the tableau lane's `justify`: drop each axiom in
        // source order and put back the ones the refutation turns out to need. The subset
        // that survives is irreducible with respect to THIS decision procedure, and the
        // decision procedure is "does the negation still clash?".
        let mut holds = |subset: &RdfDataset| -> Result<bool, EntailError> {
            // A subset this crate cannot even seed — a truncated OWL collection, which
            // `Axioms` avoids for a blank-node collection and cannot for a named one — is
            // not a subset that refutes, so the candidate axiom stays.
            let Ok(mut seeded) = refuter.seed(subset) else {
                return Ok(false);
            };
            match refuter.refute(&mut seeded, &negation)? {
                Some(clash) => {
                    best = clash;
                    Ok(true)
                }
                None => Ok(false),
            }
        };
        let shrunk = shrink_to_irreducible(premise, &mut allowance, &mut holds)?;

        // `best` is the clash of the LAST subset the search accepted, which is the subset it
        // returned: a rejected drop restores `kept`, so the final kept set is the last
        // accepted one — or, if no drop was ever accepted, the whole premise, which is what
        // `established` already refuted over.
        let mut triples = default_graph_triples(&shrunk.subset);
        triples.extend(negation);
        triples.extend(best.derived);
        refutations.push(Refutation {
            fact,
            witness: best.witness,
            premises: shrunk.subset,
            irreducible: shrunk.irreducible,
            decisions: shrunk.decisions,
            closure: Closure::of(triples),
        });
    }

    Ok(Attempt::Entailed(Box::new(Established {
        warrant: EntailmentWarrant::Refutation(RefutationWarrant {
            regime,
            // The residual is the FOLD's, not this lane's: a triple refutation left behind may
            // be one a later mechanism discharges. `entails` fills this in once every lane has
            // had its turn.
            binding: Binding::new(),
            closure: closure.clone(),
            refutations,
        }),
        discharged,
        minted: Vec::new(),
    })))
}

/// Re-check a refutation warrant's own evidence and recompute what it discharged.
///
/// Called by [`verify`](super::verify), which owns the doc comment a caller reads and which
/// checks the residual mapping once for the whole answer. It runs no reasoner: the conclusion
/// is lowered again on the spot — so a warrant cannot be replayed against a different
/// conclusion, or against the same conclusion read a different way — and each refutation is
/// re-checked by [`Refutation::check`].
pub(crate) fn verify_refutation(
    w: &RefutationWarrant,
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    _triples: &[Triple],
    pending: &BTreeSet<usize>,
) -> Option<Replay> {
    let lowering = negation::lowering(conclusion)?;
    // The facts this warrant claims must be EXACTLY the facts the conclusion states, in the
    // same order: a warrant for a subset of them would leave part of the conclusion
    // unaccounted for, and one for a superset would be evidence about a different question.
    if lowering.facts.len() != w.refutations.len() {
        return None;
    }
    if !lowering
        .facts
        .iter()
        .zip(&w.refutations)
        .all(|(fact, refutation)| fact == &refutation.fact)
    {
        return None;
    }
    if !lowering
        .consumed
        .iter()
        .all(|index| pending.contains(index))
    {
        return None;
    }
    if !w
        .refutations
        .iter()
        .all(|refutation| refutation.check(premise))
    {
        return None;
    }
    Some(Replay {
        discharged: lowering.consumed,
        minted: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use purrdf_core::{BlankScope, RdfDatasetBuilder};

    use super::{Attempt, REFUTATION_BUDGET, Refutation, attempt};
    use crate::entails::graph::default_graph_triples;
    use crate::entails::homomorphism::Closure;
    use crate::entails::negation::{NegativeFact, lower};
    use crate::entails::{EntailmentOutcome, EntailmentWarrant, ImportMap, entails, verify};
    use crate::lists::LIST_VALUED;
    use crate::reasoner::{OwlProfile, profile};
    use crate::vocab::{
        OWL_ALLDIFFERENT, OWL_ALLDISJOINTPROPERTIES, OWL_CLASS, OWL_COMPLEMENTOF,
        OWL_DIFFERENTFROM, OWL_DISJOINTWITH, OWL_MEMBERS, OWL_OBJECTPROPERTY,
        OWL_PROPERTYDISJOINTWITH, RDF_FIRST, RDF_NIL, RDF_REST, RDF_TYPE,
    };
    use crate::{Materialization, Regime, RuleId, extensions, implemented, materialize, rules};
    use purrdf_core::{RdfDataset, TermValue};
    use std::sync::Arc;

    const BOY: &str = "http://example.org/Boy";
    const GIRL: &str = "http://example.org/Girl";
    const STEWIE: &str = "http://example.org/Stewie";
    const PETER: &str = "http://example.org/Peter";
    const LOIS: &str = "http://example.org/Lois";
    const JR: &str = "http://example.org/StewieJr";
    const SOMEONE: &str = "http://example.org/someone";
    const OTHER: &str = "http://example.org/other";
    const HAS_FATHER: &str = "http://example.org/hasFather";
    const HAS_MOTHER: &str = "http://example.org/hasMother";
    const HAS_CHILD: &str = "http://example.org/hasChild";

    /// A default-graph dataset; a leading `_` names a blank node, anything else an IRI.
    fn graph(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let term = |b: &mut RdfDatasetBuilder, value: &str| match value.strip_prefix('_') {
                Some(label) => b.intern_blank(label, BlankScope::DEFAULT),
                None => b.intern_iri(value),
            };
            let s = term(&mut b, s);
            let p = term(&mut b, p);
            let o = term(&mut b, o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// W3C `disjointclasses-001`'s premise: `Boy ⊓ Girl = ⊥`, `Stewie : Boy`.
    fn disjoint_classes() -> Arc<RdfDataset> {
        graph(&[
            (BOY, RDF_TYPE, OWL_CLASS),
            (GIRL, RDF_TYPE, OWL_CLASS),
            (BOY, OWL_DISJOINTWITH, GIRL),
            (STEWIE, RDF_TYPE, BOY),
        ])
    }

    /// W3C `disjointclasses-001`'s conclusion: `Stewie : ¬Girl`.
    fn not_a_girl() -> Arc<RdfDataset> {
        graph(&[
            (GIRL, RDF_TYPE, OWL_CLASS),
            ("_c", RDF_TYPE, OWL_CLASS),
            ("_c", OWL_COMPLEMENTOF, GIRL),
            (STEWIE, RDF_TYPE, "_c"),
        ])
    }

    /// W3C `new-feature-disjointobjectproperties-001`'s premise.
    fn disjoint_properties() -> Arc<RdfDataset> {
        graph(&[
            (HAS_FATHER, RDF_TYPE, OWL_OBJECTPROPERTY),
            (HAS_MOTHER, RDF_TYPE, OWL_OBJECTPROPERTY),
            (HAS_FATHER, OWL_PROPERTYDISJOINTWITH, HAS_MOTHER),
            (STEWIE, HAS_FATHER, PETER),
            (STEWIE, HAS_MOTHER, LOIS),
        ])
    }

    /// W3C `new-feature-disjointobjectproperties-002`'s premise: three pairwise-disjoint
    /// properties, all three asserted of `Stewie`.
    fn all_disjoint_properties() -> Arc<RdfDataset> {
        graph(&[
            (HAS_FATHER, RDF_TYPE, OWL_OBJECTPROPERTY),
            (HAS_MOTHER, RDF_TYPE, OWL_OBJECTPROPERTY),
            (HAS_CHILD, RDF_TYPE, OWL_OBJECTPROPERTY),
            ("_d", RDF_TYPE, OWL_ALLDISJOINTPROPERTIES),
            ("_d", OWL_MEMBERS, "_p1"),
            ("_p1", RDF_FIRST, HAS_FATHER),
            ("_p1", RDF_REST, "_p2"),
            ("_p2", RDF_FIRST, HAS_MOTHER),
            ("_p2", RDF_REST, "_p3"),
            ("_p3", RDF_FIRST, HAS_CHILD),
            ("_p3", RDF_REST, RDF_NIL),
            (STEWIE, HAS_FATHER, PETER),
            (STEWIE, HAS_MOTHER, LOIS),
            (STEWIE, HAS_CHILD, JR),
        ])
    }

    /// `owl:AllDifferent(a, b, c)` as a conclusion graph.
    fn all_different(a: &str, b: &str, c: &str) -> Arc<RdfDataset> {
        graph(&[
            ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
            ("_x", OWL_MEMBERS, "_l1"),
            ("_l1", RDF_FIRST, a),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, b),
            ("_l2", RDF_REST, "_l3"),
            ("_l3", RDF_FIRST, c),
            ("_l3", RDF_REST, RDF_NIL),
        ])
    }

    fn decide(premise: &RdfDataset, conclusion: &RdfDataset) -> EntailmentOutcome {
        entails(premise, conclusion, Regime::OwlRl, &ImportMap::new())
            .expect("a consistent premise")
            .into_parts()
            .0
    }

    // ── The mechanism reaches what the rule table cannot ───────────────────────────────

    /// `Stewie : ¬Girl` follows from two disjoint classes, and NOTHING in the rule table
    /// concludes it: the closure holds no `owl:complementOf` triple at all, and the warrant
    /// that says so re-checks without a reasoner.
    #[test]
    fn a_complement_membership_is_reached_by_refutation_and_the_warrant_verifies() {
        let premise = disjoint_classes();
        let conclusion = not_a_girl();
        let EntailmentOutcome::Entailed(EntailmentWarrant::Refutation(warrant)) =
            decide(&premise, &conclusion)
        else {
            panic!("cax-dw refutes `Stewie a Girl`");
        };
        assert_eq!(warrant.regime(), Regime::OwlRl);
        assert_eq!(warrant.refutations().len(), 1);
        assert_eq!(
            warrant.refutations()[0].fact(),
            &NegativeFact::NotAnInstanceOf {
                individual: TermValue::iri(STEWIE),
                class: TermValue::iri(GIRL),
            }
        );
        assert_eq!(warrant.refutations()[0].witness().rule(), RuleId::CaxDw);
        let whole = EntailmentWarrant::Refutation(warrant);
        assert!(verify(&whole, &premise, &conclusion));

        // …and the residual half — `Girl a owl:Class` — really was matched rather than
        // waved through: it is the only conclusion triple no refutation consumed.
        let EntailmentWarrant::Refutation(warrant) = &whole else {
            unreachable!("just constructed")
        };
        assert_eq!(
            warrant
                .residual_witnesses(&conclusion)
                .expect("covered")
                .len(),
            1
        );
    }

    /// `Peter ≠ Lois` follows from two disjoint properties — the case whose clash body
    /// instance is DERIVED (`eq-rep-o` supplies `Stewie hasMother Peter` once the negation
    /// goes in), which is why `Refutation::check` looks in the refutation closure.
    #[test]
    fn a_different_from_is_reached_through_a_derived_body_instance() {
        let premise = disjoint_properties();
        let conclusion = graph(&[(PETER, OWL_DIFFERENTFROM, LOIS)]);
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("prp-pdw refutes `Peter owl:sameAs Lois`");
        };
        let EntailmentWarrant::Refutation(refuted) = &warrant else {
            panic!("no rule of the table has an owl:differentFrom head");
        };
        assert_eq!(refuted.refutations()[0].witness().rule(), RuleId::PrpPdw);
        assert!(verify(&warrant, &premise, &conclusion));

        // THE POINT: at least one triple the rule body matched is NOT asserted anywhere in
        // the premise or in the negation. A checker that demanded every body instance be
        // asserted would reject this refutation.
        let refutation = &refuted.refutations()[0];
        let asserted: Vec<[TermValue; 3]> = default_graph_triples(refutation.premises())
            .into_iter()
            .chain(refutation.fact().negation())
            .collect();
        assert!(
            refutation.witness().premises().iter().any(|body| {
                !asserted.contains(&[
                    body.subject().clone(),
                    body.predicate().clone(),
                    body.object().clone(),
                ])
            }),
            "{:?}",
            refutation.witness().premises()
        );
    }

    /// An `owl:AllDifferent` over three members is THREE refutations, and every one of them
    /// is required.
    #[test]
    fn an_all_different_collection_needs_every_pair_to_refute() {
        let premise = all_disjoint_properties();
        let EntailmentOutcome::Entailed(warrant) =
            decide(&premise, &all_different(PETER, LOIS, JR))
        else {
            panic!("prp-adp refutes each of the three pairs");
        };
        let EntailmentWarrant::Refutation(refuted) = &warrant else {
            panic!("no rule of the table has an owl:AllDifferent head");
        };
        assert_eq!(refuted.refutations().len(), 3, "n(n-1)/2 for n = 3");
        for refutation in refuted.refutations() {
            assert_eq!(refutation.witness().rule(), RuleId::PrpAdp);
            assert!(refutation.is_irreducible(), "{:?}", refutation.premises());
        }
        assert!(verify(&warrant, &premise, &all_different(PETER, LOIS, JR)));
    }

    // ── ADVERSARIAL: the mechanism must be able to say NO ──────────────────────────────

    /// THERE IS NO UNIQUE-NAME ASSUMPTION. Two individuals nothing forces apart are not
    /// entailed to be different, and the answer is a REFUTATION rather than a shrug.
    ///
    /// Falsifiable against the failure mode a refutation lane invites: a mechanism that
    /// always reported `Entailed` would pass every positive case of the W3C corpus, because
    /// all sixteen ledgered cases were positives.
    #[test]
    fn an_unforced_difference_is_not_entailed() {
        let premise = graph(&[
            (SOMEONE, RDF_TYPE, BOY),
            (OTHER, RDF_TYPE, BOY),
            (BOY, RDF_TYPE, OWL_CLASS),
        ]);
        let conclusion = graph(&[(SOMEONE, OWL_DIFFERENTFROM, OTHER)]);
        assert!(
            matches!(
                decide(&premise, &conclusion),
                EntailmentOutcome::NotEntailed(_)
            ),
            "nothing in the premise separates two individuals, and OWL makes no \
             unique-name assumption"
        );
    }

    /// ONE UNREFUTED PAIR SINKS THE COLLECTION. `Peter` and `Lois` are entailed different
    /// and neither is entailed different from an individual the premise says nothing about,
    /// so the three-member collection is NOT entailed.
    #[test]
    fn an_all_different_with_one_unforced_pair_is_not_entailed() {
        let premise = all_disjoint_properties();
        // `Peter` / `Lois` refutes; both pairs involving `other` do not.
        assert!(matches!(
            decide(&premise, &all_different(PETER, LOIS, OTHER)),
            EntailmentOutcome::NotEntailed(_)
        ));
        // …and the two-member collection over the pair that DOES refute is entailed, so the
        // negative above is about the third member and not about the mechanism.
        let pair = graph(&[
            ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
            ("_x", OWL_MEMBERS, "_l1"),
            ("_l1", RDF_FIRST, PETER),
            ("_l1", RDF_REST, "_l2"),
            ("_l2", RDF_FIRST, LOIS),
            ("_l2", RDF_REST, RDF_NIL),
        ]);
        assert!(matches!(
            decide(&premise, &pair),
            EntailmentOutcome::Entailed(_)
        ));
    }

    // ── The soundness precondition, and the inventory ──────────────────────────────────

    /// AN INCONSISTENT PREMISE NEVER REACHES THIS LANE.
    ///
    /// `KB ∪ {¬α}` inconsistent proves `KB ⊨ α` only for consistent `KB`, so this is the
    /// hypothesis of the whole argument stated as a test: over a premise with no model the
    /// call refuses instead of reporting a refutation it could have produced for literally
    /// any conclusion.
    #[test]
    fn an_inconsistent_premise_never_reaches_the_refutation_lane() {
        let premise = graph(&[
            (BOY, OWL_DISJOINTWITH, GIRL),
            (STEWIE, RDF_TYPE, BOY),
            (STEWIE, RDF_TYPE, GIRL),
        ]);
        let Err(crate::EntailError::Inconsistent(run)) = entails(
            &premise,
            &graph(&[(PETER, OWL_DIFFERENTFROM, LOIS)]),
            Regime::OwlRl,
            &ImportMap::new(),
        ) else {
            panic!("cax-dw already refuses this premise on its own");
        };
        assert_eq!(run.witness().rule(), RuleId::CaxDw);
    }

    /// STRICT MATERIALIZATION GAINS NOTHING. The closure of each premise still does not hold
    /// the conclusion; only the conclusion-directed service reaches it.
    #[test]
    fn materialization_still_does_not_produce_these_conclusions() {
        let (closure, _) =
            materialize(&disjoint_properties(), Materialization::OwlRl).expect("consistent");
        assert!(
            !default_graph_triples(&closure).contains(&[
                TermValue::iri(PETER),
                TermValue::iri(OWL_DIFFERENTFROM),
                TermValue::iri(LOIS),
            ]),
            "no rule of Tables 4-9 has an owl:differentFrom head"
        );

        let (closure, _) =
            materialize(&disjoint_classes(), Materialization::OwlRl).expect("consistent");
        assert!(
            !default_graph_triples(&closure)
                .iter()
                .any(|[_, p, _]| p == &TermValue::iri(OWL_COMPLEMENTOF)),
            "no rule of Tables 4-9 concludes membership in a complement class"
        );

        let (closure, _) =
            materialize(&all_disjoint_properties(), Materialization::OwlRl).expect("consistent");
        assert!(
            !default_graph_triples(&closure)
                .iter()
                .any(|[_, _, o]| o == &TermValue::iri(OWL_ALLDIFFERENT)),
            "no rule of Tables 4-9 concludes an owl:AllDifferent collection"
        );
    }

    /// THE NORMATIVE INVENTORY IS UNTOUCHED. Refutation is a proof strategy over the
    /// declared calculus, not a widening of it, and this is that claim as an assertion
    /// rather than as a sentence in a doc comment.
    #[test]
    fn the_refutation_lane_adds_no_rule() {
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl), rules(Regime::OwlRl));
        assert_eq!(extensions(Regime::OwlRl), [RuleId::ExtEqDiffSym]);
    }

    /// The negation this lane asserts is itself inside the OWL 2 RL syntax, so it cannot
    /// break Theorem PR1's hypothesis for the premise it is added to — which is what
    /// entitles a failed refutation over an RL premise to be read as a refutation at all.
    #[test]
    fn a_negation_stays_inside_the_rl_syntax() {
        let premise = disjoint_properties();
        assert!(profile(&premise).certifies(OwlProfile::Rl));
        for extended in [
            graph(&[
                (HAS_FATHER, RDF_TYPE, OWL_OBJECTPROPERTY),
                (HAS_MOTHER, RDF_TYPE, OWL_OBJECTPROPERTY),
                (HAS_FATHER, OWL_PROPERTYDISJOINTWITH, HAS_MOTHER),
                (STEWIE, HAS_FATHER, PETER),
                (STEWIE, HAS_MOTHER, LOIS),
                (PETER, "http://www.w3.org/2002/07/owl#sameAs", LOIS),
            ]),
            graph(&[
                (BOY, RDF_TYPE, OWL_CLASS),
                (GIRL, RDF_TYPE, OWL_CLASS),
                (BOY, OWL_DISJOINTWITH, GIRL),
                (STEWIE, RDF_TYPE, BOY),
                (STEWIE, RDF_TYPE, GIRL),
            ]),
        ] {
            assert!(
                profile(&extended).certifies(OwlProfile::Rl),
                "an owl:sameAs assertion and a class assertion are both OWL 2 RL axioms"
            );
        }
    }

    /// The seed's three pre-passes are reused across a refutation run, and that is only
    /// sound because the assertion cannot disturb any of them. This is the whitelist that
    /// makes it true, checked rather than recited.
    #[test]
    fn an_added_assertion_never_disturbs_a_pre_pass() {
        let facts = [
            NegativeFact::Distinct {
                left: TermValue::iri(PETER),
                right: TermValue::iri(LOIS),
            },
            NegativeFact::NotAnInstanceOf {
                individual: TermValue::iri(STEWIE),
                class: TermValue::iri(GIRL),
            },
        ];
        for fact in facts {
            for triple in fact.negation() {
                let TermValue::Iri(predicate) = &triple[1] else {
                    panic!("a negation's predicate is always an IRI");
                };
                // The collection pre-pass observes exactly these nine predicates, and none
                // of them is one a negation writes — so the walk the seed already did is
                // still the walk this run needs.
                assert_ne!(predicate.as_str(), RDF_FIRST);
                assert_ne!(predicate.as_str(), RDF_REST);
                assert!(!LIST_VALUED.contains(&predicate.as_str()));
                // …and the datatype pre-pass observes only literals, of which a negation
                // carries none in any position.
                for term in &triple {
                    assert!(
                        !matches!(term, TermValue::Literal { .. }),
                        "a negation carries no literal"
                    );
                }
            }
        }
    }

    // ── `verify` is a CHECK, not a claim ───────────────────────────────────────────────

    /// A refutation warrant does not replay against another premise, another conclusion, or
    /// a doctored subset — one forgery per check `Refutation::check` makes.
    #[test]
    fn a_refutation_warrant_does_not_replay() {
        let premise = disjoint_properties();
        let conclusion = graph(&[(PETER, OWL_DIFFERENTFROM, LOIS)]);
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("prp-pdw refutes it");
        };
        assert!(verify(&warrant, &premise, &conclusion));

        // Another PREMISE: the cited subset is not a subset of it.
        let other = graph(&[(STEWIE, RDF_TYPE, BOY)]);
        assert!(!verify(&warrant, &other, &conclusion));
        // Another CONCLUSION: the facts it lowers to are different ones.
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(PETER, OWL_DIFFERENTFROM, JR)])
        ));
        // A conclusion this lane does not read at all lowers to nothing, so there is no
        // fact list to agree with.
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(STEWIE, RDF_TYPE, BOY)])
        ));

        // A witness citing a rule that concludes a TRIPLE is evidence of a derivation, not
        // of a contradiction.
        let EntailmentWarrant::Refutation(refuted) = &warrant else {
            unreachable!("checked above")
        };
        let honest = &refuted.refutations()[0];
        let forged = Refutation {
            witness: crate::report::InconsistencyWitness::new(
                RuleId::CaxSco,
                honest.witness().premises().to_vec(),
                None,
            ),
            ..honest.clone()
        };
        assert!(!forged.check(&premise));

        // …and a cited subset holding a triple the premise does not is not about it.
        let forged = Refutation {
            premises: graph(&[(STEWIE, HAS_FATHER, JR)]),
            ..honest.clone()
        };
        assert!(!forged.check(&premise));
    }

    // ── Applicability, and the budget ──────────────────────────────────────────────────

    /// The lane is gated to `OWL-RL` by WHITELIST: the three lanes with no `false`-headed
    /// rule at all, and the one whose inconsistency calculus this crate claims no
    /// completeness for, fall out rather than running a computation that can only answer
    /// "not established".
    #[test]
    fn only_the_owl_rl_lane_refutes() {
        let premise = disjoint_properties();
        let conclusion = graph(&[(PETER, OWL_DIFFERENTFROM, LOIS)]);
        let closure = Closure::of(Vec::new());
        let triples = default_graph_triples(&conclusion);
        let pending: std::collections::BTreeSet<usize> = (0..triples.len()).collect();
        for regime in [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::D] {
            let question = crate::entails::Question {
                premise: &premise,
                conclusion: &conclusion,
                regime,
                closure: &closure,
                triples: &triples,
                pending: &pending,
            };
            assert!(
                matches!(
                    attempt(&question).expect("no chase"),
                    Attempt::NotApplicable
                ),
                "{regime:?} states no rule this lane could read as a refutation"
            );
        }
    }

    /// A conclusion needing more re-chases than the budget allows is UNDECIDED, never
    /// refuted: "I stopped" and "there is nothing to find" are different claims.
    #[test]
    fn a_conclusion_past_the_budget_is_undecided() {
        // `n(n-1)/2 > REFUTATION_BUDGET` needs `n` above 32 for a budget of 512.
        let members: Vec<String> = (0..64)
            .map(|i| format!("http://example.org/m{i}"))
            .collect();
        let mut triples: Vec<(&str, &str, &str)> = vec![
            ("_x", RDF_TYPE, OWL_ALLDIFFERENT),
            ("_x", OWL_MEMBERS, "_l0"),
        ];
        let cells: Vec<String> = (0..members.len()).map(|i| format!("_l{i}")).collect();
        for (i, member) in members.iter().enumerate() {
            triples.push((&cells[i], RDF_FIRST, member));
            triples.push((
                &cells[i],
                RDF_REST,
                if i + 1 == members.len() {
                    RDF_NIL
                } else {
                    &cells[i + 1]
                },
            ));
        }
        let conclusion = graph(&triples);
        let needed = (members.len() * (members.len() - 1) / 2) as u64;
        assert!(needed > REFUTATION_BUDGET);
        assert!(
            matches!(
                lower(&conclusion),
                crate::entails::negation::Read::Lowered(_)
            ),
            "the conclusion IS one this lane reads; what it cannot do is afford it"
        );
        assert!(matches!(
            decide(&disjoint_properties(), &conclusion),
            EntailmentOutcome::Undecided(crate::UndecidedReason::RefutationBudget(_))
        ));
    }

    /// The whole answer is a function of the inputs: two runs produce the same facts, the
    /// same witnesses and the same shrunk premise subsets.
    #[test]
    fn the_refutation_lane_is_deterministic() {
        let run = || {
            let EntailmentOutcome::Entailed(EntailmentWarrant::Refutation(w)) =
                decide(&all_disjoint_properties(), &all_different(PETER, LOIS, JR))
            else {
                panic!("every pair refutes");
            };
            w.refutations()
                .iter()
                .map(|r| {
                    (
                        r.fact().to_string(),
                        r.witness().clone(),
                        purrdf_core::canonicalize(r.premises()).nquads,
                        r.decisions(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
