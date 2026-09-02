// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **reflexivity** mechanism: `p` is reflexive, so `x p x`, for the conclusion's own
//! named terms and no others.
//!
//! # The gap this closes
//!
//! W3C's `new-feature-reflexiveproperty-001` asserts `knows rdf:type owl:ReflexiveProperty`
//! and concludes `Peter knows Peter`. That conclusion is an ordinary assertional triple over
//! named terms — exactly the shape [`homomorphism`](super::homomorphism) matches — and the
//! match still misses, because OWL 2 Profiles §4.3 states **no `prp-rfl` rule**:
//! `owl:ReflexiveProperty` is outside the OWL 2 RL syntax, so the profile's rule table has
//! nothing to say about it and the chase copies the premise's typing through without drawing
//! anything from it.
//!
//! The premise is also, for the same reason, outside the RL syntax — it is one of the six
//! such premises the corpus carries — so [`precondition`](super::precondition) will not let a
//! failed match refute either. Reaching the right answer therefore means establishing the
//! conclusion POSITIVELY. A found proof needs no completeness theorem; only a refutation
//! does.
//!
//! # The argument, in full
//!
//! OWL 2's RDF-Based Semantics fixes the extension of `owl:ReflexiveProperty` by
//!
//! > `p ∈ ICEXT(owl:ReflexiveProperty)` iff `p ∈ IP` and `<x,x> ∈ EXT(p)` for every `x ∈ IR`.
//!
//! Take any model `I` of the premise. The premise's closure holds
//! `p rdf:type owl:ReflexiveProperty`, and the chase is SOUND, so the premise entails that
//! typing and `I` satisfies it. Now let `x` be any IRI of the conclusion's vocabulary.
//! Deciding `premise ⊨ conclusion` quantifies over interpretations of
//! `vocab(premise) ∪ vocab(conclusion)`, and in any such interpretation `x` denotes an element
//! of `IR` — every IRI does; that is what an RDF interpretation IS. So `<x^I, x^I> ∈ EXT(p^I)`,
//! i.e. `I ⊨ x p x`. Since `I` was arbitrary, `premise ⊨ x p x`. ∎
//!
//! Two things that argument does NOT need are worth naming, because both are places a reader
//! might expect a side condition:
//!
//! * `x` need not occur in the premise. It occurs in the CONCLUSION, which is enough: the
//!   interpretations that decide the entailment interpret it.
//! * `x` need not be typed as anything. `IR` is every resource, not `owl:Thing`'s extension
//!   and not any named class's.
//!
//! What it does need is that `x` be an IRI. A LITERAL cannot occupy a subject position in RDF
//! 1.2, so `x p x` is not a triple for a literal `x` and there is nothing to establish; a
//! BLANK node in the conclusion is an existential, and binding one is
//! [`homomorphism`](super::homomorphism)'s job over whatever this mechanism has minted, not a
//! separate claim of this one.
//!
//! # Why there is NO `ext-prp-rfl` RULE, and why that is not squeamishness
//!
//! The obvious alternative is a clause `ReflexiveProperty(p) ∧ Term(x) → p(x,x)`, declared as
//! an extension beside `ext-eq-diff-sym`. It is sound. It is also the wrong shape for a lane
//! every consumer runs by default, in four separate ways:
//!
//! 1. **It ranges over all resources.** The body has no join to constrain `x`, so the clause
//!    fires once per (term × reflexive property) pair — an `O(|terms|)` addition to a closure
//!    a caller asked for other reasons. Every other rule of the table joins on something the
//!    premise states; this one would not.
//! 2. **It puts literals in subject position.** `Term(x)` ranges over the term table, literals
//!    included, and a conclusion with a literal subject is exactly what the engine's
//!    `admits_subject` filter drops while counting a
//!    [`Construct::GeneralizedRdf`](crate::Construct::GeneralizedRdf) boundary. The lane would
//!    start reporting that boundary on ordinary inputs, which is a disclosure about the INPUT
//!    turned into a standing property of the rule set.
//! 3. **It moves `contract_hash`.** The calculus's contract hash is the identity of the rule
//!    program, and 159 committed goldens carry it. Changing it is a byte-level change to every
//!    one of them.
//! 4. **It would need an out-of-band golden regeneration.** The closure goldens are checked
//!    against `owlrl==7.1.4`, which states no such rule either, so the regeneration would
//!    have to be performed and justified outside the rule set it is meant to be measured by.
//!
//! So the conclusion is reached through the ENTAILMENT PATH instead, where the question is
//! conclusion-directed and the work is one lookup per conclusion triple.
//! `strict_materialization_is_unchanged` and `the_reflexivity_lane_adds_no_rule` are the
//! falsifiable form of what that buys: `materialize(premise, Materialization::OwlRl)` produces
//! exactly what it produced before, and `extensions(Regime::OwlRl)` is still exactly
//! `[ExtEqDiffSym]`.
//!
//! # Applicability is a WHITELIST
//!
//! A conclusion triple is read only when its subject and object are the SAME IRI, its
//! predicate is an IRI, and the premise's closure holds that predicate's
//! `owl:ReflexiveProperty` typing. Everything else is an ordinary pattern and keeps its
//! ordinary obligation to map — this mechanism adds triples to the closure and removes no
//! obligation from anything.
//!
//! # Determinism
//!
//! Triples are read in the conclusion's own frozen order and each licence is the one closure
//! triple that establishes it. Two runs over one premise and one conclusion mint the same
//! triples, on `wasm32` as on native.

use purrdf_core::{RdfDataset, TermValue};

use std::collections::BTreeSet;

use crate::entails::graph::{Triple, show};
use crate::entails::homomorphism::{Binding, Closure};
use crate::entails::warrant::{EntailmentMechanism, EntailmentWarrant, Replay};
use crate::entails::{Attempt, Established, Question, Recognized, UndecidedReason};
use crate::vocab::{OWL_REFLEXIVEPROPERTY, RDF_TYPE};
use crate::{EntailError, Regime};

/// The evidence that a premise entails a conclusion whose reflexive self-loops it states.
///
/// Two parts. [`Self::minted`] is what was added to the premise's closure — one `x p x` per
/// conclusion triple this mechanism read — and [`Self::licences`] is the closure triple that
/// licenses each, `p rdf:type owl:ReflexiveProperty`. [`Self::binding`] then maps the WHOLE
/// conclusion into the extended closure, so nothing is discharged by having been recognized.
#[derive(Debug, Clone)]
pub struct ReflexivityWarrant {
    /// The regime the closure was computed under.
    regime: Regime,
    /// What each existential of the conclusion was bound to.
    binding: Binding,
    /// The premise's own closure, unextended.
    closure: Closure,
    /// The self-loops the reflexive typings licensed, in reading order.
    minted: Vec<Triple>,
    /// The closure triple licensing each, in the same order.
    licences: Vec<Triple>,
}

impl ReflexivityWarrant {
    /// The regime whose closure carried the reflexive typing.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        self.regime
    }

    /// The mapping: what each existential of the conclusion was bound to.
    #[must_use]
    pub const fn binding(&self) -> &Binding {
        &self.binding
    }

    /// The self-loops this warrant established, in the conclusion's own triple order.
    #[must_use]
    pub fn minted(&self) -> &[Triple] {
        &self.minted
    }

    /// The `owl:ReflexiveProperty` typings that license them, in the same order.
    #[must_use]
    pub fn licences(&self) -> &[Triple] {
        &self.licences
    }

    /// How many distinct triples the PREMISE closure this warrant is against holds.
    ///
    /// The minted self-loops are not counted: they are not conclusions of the chase, and
    /// folding them in would misreport what the chase produced.
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
}

impl std::fmt::Display for ReflexivityWarrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} reflexive self-loop{}",
            self.minted.len(),
            if self.minted.len() == 1 { "" } else { "s" }
        )?;
        for triple in &self.minted {
            write!(
                f,
                "\n  {} {} {}",
                show(&triple[0]),
                show(&triple[1]),
                show(&triple[2])
            )?;
        }
        Ok(())
    }
}

/// What one conclusion licensed: the self-loops, the typings that license them, and the
/// self-loops this lane RECOGNIZED and declined.
struct Licensed {
    /// The self-loops.
    minted: Vec<Triple>,
    /// Which conclusion triples the self-loops came off, by index into the conclusion's own
    /// frozen triple order. This lane DISCHARGES none of them — it widens the closure and
    /// leaves each its full obligation to map — so the set is not a discharge; it is what
    /// [`recognizes`] answers with.
    read: BTreeSet<usize>,
    /// The closure triple licensing each.
    licences: Vec<Triple>,
    /// Self-loops over an EXISTENTIAL — `_:b p _:b` for a `p` the closure declares reflexive.
    /// This lane establishes a loop at a term the conclusion NAMES and does not choose a
    /// witness for one it does not, so such a triple is an admission rather than a residual
    /// the closure would be asked to refute.
    declined: Vec<String>,
}

/// Every self-loop of `conclusion` the closure's reflexive typings license, with the whole
/// conclusion as match patterns.
///
/// A pure function of the conclusion and the closure — which is what lets
/// [`verify`](super::verify) recompute it and compare rather than trust the warrant's list.
fn license(triples: &[Triple], closure: &Closure) -> Licensed {
    let mut minted = Vec::new();
    let mut read = BTreeSet::new();
    let mut licences = Vec::new();
    let mut declined = Vec::new();
    for (index, triple) in triples.iter().enumerate() {
        let [subject, predicate, object] = triple;
        // THE WHITELIST: the same NAMED term either side of a NAMED predicate. A literal
        // cannot be a subject at all, and a triple whose two sides differ is not a self-loop.
        if !matches!(predicate, TermValue::Iri(_)) || subject != object {
            continue;
        }
        let typing = [
            predicate.clone(),
            TermValue::iri(RDF_TYPE),
            TermValue::iri(OWL_REFLEXIVEPROPERTY),
        ];
        if !closure.contains(&typing) {
            continue;
        }
        // A BLANK node either side is an existential — "is there something that `p`s itself?"
        // — and this lane establishes a loop at a term the CONCLUSION names rather than
        // choosing a witness for one it does not. Recognized and declined, never dropped: a
        // dropped one would fall through to a failed match and be reported as a proof.
        if !matches!(subject, TermValue::Iri(_)) {
            declined.push(format!(
                "{} {} {}: a self-loop over an existential names no term to establish it at",
                show(subject),
                show(predicate),
                show(object)
            ));
            continue;
        }
        minted.push(triple.clone());
        read.insert(index);
        licences.push(typing);
    }
    declined.sort_unstable();
    declined.dedup();
    Licensed {
        minted,
        read,
        licences,
        declined,
    }
}

/// What this lane READS of a question, with nothing minted.
///
/// The same [`license`] the decision below opens with, run for its reading alone: a self-loop
/// the closure's reflexive typings license is one no rule of the table concludes, and a
/// declined one is a self-loop over an existential this lane names and will not choose a
/// witness for. Either way a service that does not run this lane has left something untested.
pub(crate) fn recognizes(q: &Question<'_>) -> Recognized {
    if !matches!(q.regime, Regime::OwlRl) {
        return Recognized::default();
    }
    let licensed = license(q.triples, q.closure);
    Recognized {
        read: licensed.read,
        declined: licensed.declined,
    }
}

/// Try to establish `conclusion` from the premise through its reflexive properties.
///
/// The premise itself is not read: everything this mechanism needs — the reflexive typings —
/// is in `closure`, which holds the premise plus what the chase drew from it. The parameter
/// is present because every mechanism presents one signature, and a mechanism that took a
/// different one could not be a member of the list [`entails`](super::entails) walks.
///
/// # Errors
///
/// [`EntailError::MatchBudget`] if the final match exhausts its budget.
pub(crate) fn attempt(q: &Question<'_>) -> Result<Attempt, EntailError> {
    let Question {
        regime,
        closure,
        triples,
        ..
    } = *q;
    // WHITELIST, not blacklist. `owl:ReflexiveProperty` is an OWL 2 term, and the three
    // lanes below `OWL-RL` interpret no OWL vocabulary at all: reading a typing they do not
    // interpret as a licence would be this service drawing an OWL conclusion under a regime
    // the caller asked to be weaker. `D` adds only the five `dt-*` rules and the same holds.
    if !matches!(regime, Regime::OwlRl) {
        return Ok(Attempt::NotApplicable);
    }
    let licensed = license(triples, closure);
    if licensed.minted.is_empty() {
        return Ok(if licensed.declined.is_empty() {
            Attempt::NotApplicable
        } else {
            Attempt::Disqualified(UndecidedReason::ConstructNotRead {
                lane: EntailmentMechanism::Reflexivity,
                constructs: licensed.declined,
            })
        });
    }

    // This lane DISCHARGES nothing: it adds self-loops to the closure and every conclusion
    // triple, self-loop included, keeps its full obligation to map into the widened closure.
    Ok(Attempt::Entailed(Box::new(Established {
        warrant: EntailmentWarrant::Reflexivity(ReflexivityWarrant {
            regime,
            binding: Binding::new(),
            closure: closure.clone(),
            minted: licensed.minted.clone(),
            licences: licensed.licences,
        }),
        discharged: BTreeSet::new(),
        minted: licensed.minted,
        // One pass licenses both: a self-loop at a name is minted and one at an existential is
        // declined, and the second travels WITH the first rather than being lost to it.
        declined: licensed.declined,
    })))
}

/// Re-decide a reflexivity warrant against the caller's own premise and conclusion.
///
/// Called by [`verify`](super::verify), which owns the doc comment a caller reads. It runs no
/// reasoner: the conclusion is READ again on the spot, the licensing is RECOMPUTED and
/// compared, every licence is re-looked-up in the closure, and the binding is replayed.
pub(crate) fn verify_reflexivity(
    w: &ReflexivityWarrant,
    _conclusion: &RdfDataset,
    triples: &[Triple],
    _pending: &BTreeSet<usize>,
) -> Option<Replay> {
    let licensed = license(triples, &w.closure);
    if licensed.minted != w.minted || licensed.licences != w.licences || w.minted.is_empty() {
        return None;
    }
    if !w.licences.iter().all(|triple| w.closure.contains(triple)) {
        return None;
    }
    Some(Replay {
        discharged: BTreeSet::new(),
        minted: w.minted.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    use crate::entails::graph::default_graph_triples;
    use crate::entails::{EntailmentOutcome, EntailmentWarrant, ImportMap, entails, verify};
    use crate::reasoner::{OwlProfile, profile};
    use crate::vocab::{
        OWL_NAMEDINDIVIDUAL, OWL_OBJECTPROPERTY, OWL_ONTOLOGY, OWL_REFLEXIVEPROPERTY, RDF_TYPE,
    };
    use crate::{Materialization, Regime, RuleId, extensions, implemented, materialize, rules};

    const KNOWS: &str = "http://example.org/knows";
    const LIKES: &str = "http://example.org/likes";
    const PETER: &str = "http://example.org/Peter";
    const LOIS: &str = "http://example.org/Lois";

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

    /// W3C `new-feature-reflexiveproperty-001`'s premise.
    fn reflexive_premise() -> Arc<RdfDataset> {
        graph(&[
            ("_o", RDF_TYPE, OWL_ONTOLOGY),
            (KNOWS, RDF_TYPE, OWL_OBJECTPROPERTY),
            (PETER, RDF_TYPE, OWL_NAMEDINDIVIDUAL),
            (KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY),
        ])
    }

    /// …and its conclusion.
    fn self_loop_conclusion() -> Arc<RdfDataset> {
        graph(&[
            ("_o", RDF_TYPE, OWL_ONTOLOGY),
            (KNOWS, RDF_TYPE, OWL_OBJECTPROPERTY),
            (PETER, KNOWS, PETER),
        ])
    }

    fn decide(premise: &RdfDataset, conclusion: &RdfDataset) -> EntailmentOutcome {
        entails(premise, conclusion, Regime::OwlRl, &ImportMap::new())
            .expect("a consistent premise")
            .into_parts()
            .0
    }

    // ── The mechanism reaches what the rule table cannot ───────────────────────────────

    /// A REFLEXIVE PROPERTY SELF-LOOPS, and the warrant re-checks.
    #[test]
    fn a_reflexive_property_entails_the_self_loop_and_the_warrant_verifies() {
        let premise = reflexive_premise();
        let conclusion = self_loop_conclusion();
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("a reflexive property holds of every resource");
        };
        let EntailmentWarrant::Reflexivity(reflexive) = &warrant else {
            panic!("Profiles §4.3 states no prp-rfl rule");
        };
        assert_eq!(reflexive.regime(), Regime::OwlRl);
        assert_eq!(
            reflexive.minted(),
            [[
                TermValue::iri(PETER),
                TermValue::iri(KNOWS),
                TermValue::iri(PETER)
            ]]
        );
        assert_eq!(
            reflexive.licences(),
            [[
                TermValue::iri(KNOWS),
                TermValue::iri(RDF_TYPE),
                TermValue::iri(OWL_REFLEXIVEPROPERTY)
            ]]
        );
        assert_ne!(reflexive.to_string(), "");
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// THE PREMISE IS OUTSIDE THE OWL 2 RL SYNTAX, and it is established anyway.
    ///
    /// This is the whole point of reaching it through the entailment path: a found proof needs
    /// no completeness theorem, so the precondition that would have made a failed match
    /// `Undecided` never comes into it.
    #[test]
    fn the_premise_is_outside_rl_and_the_conclusion_is_still_established() {
        let premise = reflexive_premise();
        assert!(
            !profile(&premise).certifies(OwlProfile::Rl),
            "owl:ReflexiveProperty is outside the OWL 2 RL syntax"
        );
        assert!(matches!(
            decide(&premise, &self_loop_conclusion()),
            EntailmentOutcome::Entailed(_)
        ));
    }

    /// A term the PREMISE never mentions still self-loops: `IR` is every resource, and the
    /// interpretations that decide the entailment interpret the conclusion's vocabulary too.
    #[test]
    fn a_term_only_the_conclusion_names_still_self_loops() {
        let premise = graph(&[(KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY)]);
        let conclusion = graph(&[(LOIS, KNOWS, LOIS)]);
        assert!(matches!(
            decide(&premise, &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
    }

    // ── ADVERSARIAL: the mechanism must be able to say NO ──────────────────────────────

    /// A PROPERTY NOTHING DECLARES REFLEXIVE DOES NOT SELF-LOOP.
    ///
    /// Falsifiable against the failure mode this mechanism invites: one that minted every
    /// `x p x` the conclusion asked for would pass the corpus case and be worthless.
    #[test]
    fn an_undeclared_property_does_not_self_loop() {
        let premise = graph(&[(KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY)]);
        let conclusion = graph(&[(PETER, LIKES, PETER)]);
        assert!(
            !matches!(
                decide(&premise, &conclusion),
                EntailmentOutcome::Entailed(_)
            ),
            "nothing declares `likes` reflexive"
        );
    }

    /// A SELF-LOOP OVER AN EXISTENTIAL IS AN ADMISSION, NOT A REFUTATION.
    ///
    /// `_:b knows _:b` asks whether SOMETHING knows itself. The argument this lane rests on
    /// establishes the loop at a term the CONCLUSION names — "let `x` be any IRI of the
    /// conclusion's vocabulary" — and an existential names none, so the lane recognizes the
    /// reflexive typing and declines. Falsifiable against the failure the whitelist used to
    /// have: dropping the triple silently sent it to a match that could not find it, and the
    /// fall-through reported that as a proof of non-entailment.
    #[test]
    fn a_self_loop_over_an_existential_is_declined_by_name() {
        let premise = graph(&[(KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY)]);
        let conclusion = graph(&[("_b", KNOWS, "_b")]);
        let EntailmentOutcome::Undecided(crate::UndecidedReason::ConstructNotRead {
            lane,
            constructs,
        }) = decide(&premise, &conclusion)
        else {
            panic!("declining to choose a witness is an admission, not a refutation");
        };
        assert_eq!(lane, crate::EntailmentMechanism::Reflexivity);
        assert!(
            constructs.iter().any(|why| why.contains("existential")),
            "{constructs:?}"
        );
    }

    /// …AND A MINT DOES NOT CANCEL AN ADMISSION MADE IN THE SAME PASS.
    ///
    /// Two reflexive properties, one self-loop at a name and one at an existential: the lane
    /// mints the first and declines the second, in ONE call. The declined one is what a caller
    /// must be told about, and the shape of the bug this test pins is that it was told only
    /// when the lane minted NOTHING — add a mintable loop beside it and the sentence naming
    /// the refusal disappeared, leaving the failed match to speak for a triple nothing tested.
    #[test]
    fn a_declined_self_loop_survives_a_mint_made_in_the_same_pass() {
        let premise = graph(&[
            (KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY),
            (LIKES, RDF_TYPE, OWL_REFLEXIVEPROPERTY),
        ]);
        // `_:b likes _:b` cannot borrow the minted `Peter knows Peter`: nothing in the closure
        // or the mint states `likes` of anything, so the residual really does fail and the
        // answer really does turn on what the lane says about it.
        let conclusion = graph(&[(PETER, KNOWS, PETER), ("_b", LIKES, "_b")]);
        let EntailmentOutcome::Undecided(crate::UndecidedReason::ConstructNotRead {
            lane,
            constructs,
        }) = decide(&premise, &conclusion)
        else {
            panic!("a recognized-and-declined self-loop is an ADMISSION, never a shrug");
        };
        assert_eq!(lane, crate::EntailmentMechanism::Reflexivity);
        assert_eq!(constructs.len(), 1, "{constructs:?}");
        assert!(
            constructs[0].contains("existential") && constructs[0].contains(LIKES),
            "{constructs:?}"
        );
    }

    /// …and a self-loop is not a loop between two DIFFERENT terms.
    #[test]
    fn a_reflexive_property_does_not_relate_two_terms() {
        let premise = graph(&[(KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY)]);
        let conclusion = graph(&[(PETER, KNOWS, LOIS)]);
        assert!(!matches!(
            decide(&premise, &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
    }

    /// A conclusion whose OTHER triples do not map is not established either — the minted
    /// self-loop discharges itself and nothing else.
    #[test]
    fn an_unmatched_residual_triple_sinks_the_conclusion() {
        let premise = graph(&[(KNOWS, RDF_TYPE, OWL_REFLEXIVEPROPERTY)]);
        let conclusion = graph(&[
            (PETER, KNOWS, PETER),
            (PETER, RDF_TYPE, "http://example.org/Never"),
        ]);
        assert!(!matches!(
            decide(&premise, &conclusion),
            EntailmentOutcome::Entailed(_)
        ));
    }

    // ── The rule table is UNTOUCHED, and that is the point ─────────────────────────────

    /// STRICT MATERIALIZATION IS UNCHANGED. An `ext-prp-rfl` clause would have added a self
    /// loop per (term × reflexive property) pair to a lane every consumer runs by default;
    /// this mechanism adds nothing to it at all.
    #[test]
    fn strict_materialization_is_unchanged() {
        let premise = reflexive_premise();
        let (closure, _) = materialize(&premise, Materialization::OwlRl).expect("consistent");
        let produced = default_graph_triples(&closure);
        assert!(
            !produced.contains(&[
                TermValue::iri(PETER),
                TermValue::iri(KNOWS),
                TermValue::iri(PETER)
            ]),
            "no rule of Tables 4-9 concludes a reflexive self-loop"
        );
        // The closure is the premise and nothing else: the OWL 2 RL table draws no conclusion
        // from a reflexive typing, so the strict lane's output is byte-for-byte the input.
        let asserted = default_graph_triples(&premise);
        for triple in &asserted {
            assert!(produced.contains(triple));
        }
        assert!(
            !produced.iter().any(|[_, p, _]| p == &TermValue::iri(KNOWS)),
            "the strict lane derived no `knows` triple whatsoever"
        );
    }

    /// THE NORMATIVE INVENTORY IS UNTOUCHED, extension list included.
    #[test]
    fn the_reflexivity_lane_adds_no_rule() {
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl), rules(Regime::OwlRl));
        assert_eq!(
            extensions(Regime::OwlRl),
            [RuleId::ExtEqDiffSym],
            "the one declared extension, and no `ext-prp-rfl` beside it"
        );
    }

    /// The lane is gated to `OWL-RL` by WHITELIST: the four lanes that interpret no OWL
    /// vocabulary fall out rather than drawing an OWL conclusion under a weaker regime.
    #[test]
    fn only_the_owl_rl_lane_self_loops() {
        let premise = reflexive_premise();
        let conclusion = self_loop_conclusion();
        for regime in [Regime::Simple, Regime::Rdf, Regime::Rdfs, Regime::D] {
            assert!(
                !matches!(
                    entails(&premise, &conclusion, regime, &ImportMap::new())
                        .expect("consistent")
                        .outcome(),
                    EntailmentOutcome::Entailed(_)
                ),
                "{regime:?} interprets no OWL vocabulary"
            );
        }
    }

    // ── `verify` is a CHECK, not a claim ───────────────────────────────────────────────

    /// A reflexivity warrant does not replay against another premise or conclusion.
    #[test]
    fn a_reflexivity_warrant_does_not_replay() {
        let premise = reflexive_premise();
        let conclusion = self_loop_conclusion();
        let EntailmentOutcome::Entailed(warrant) = decide(&premise, &conclusion) else {
            panic!("entailed");
        };
        assert!(verify(&warrant, &premise, &conclusion));
        // Another PREMISE: this warrant's closure is not a closure of it.
        assert!(!verify(
            &warrant,
            &graph(&[(LOIS, RDF_TYPE, OWL_NAMEDINDIVIDUAL)]),
            &conclusion
        ));
        // Another CONCLUSION: the self-loops it states are different ones.
        assert!(!verify(&warrant, &premise, &graph(&[(LOIS, KNOWS, LOIS)])));
        // A conclusion stating no self-loop at all licenses nothing to compare against.
        assert!(!verify(
            &warrant,
            &premise,
            &graph(&[(KNOWS, RDF_TYPE, OWL_OBJECTPROPERTY)])
        ));
    }

    /// The whole answer is a function of the inputs: two runs mint the same triples.
    #[test]
    fn the_reflexivity_lane_is_deterministic() {
        let run = || {
            let EntailmentOutcome::Entailed(EntailmentWarrant::Reflexivity(w)) =
                decide(&reflexive_premise(), &self_loop_conclusion())
            else {
                panic!("entailed");
            };
            (w.minted().to_vec(), w.licences().to_vec())
        };
        assert_eq!(run(), run());
    }
}
