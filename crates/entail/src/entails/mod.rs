// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Conclusion-directed** entailment: does this premise entail this conclusion?
//!
//! [`materialize`] answers a different question. It computes a CLOSURE
//! — everything the premise entails, as a dataset — and hands it over. That is the right
//! shape for a caller that will go on asking many questions of one premise, and the wrong
//! shape for a caller with one question, because turning a closure into a verdict is not
//! the obvious membership test it looks like: a conclusion's blank nodes are existentials
//! that have to be MAPPED, an inconsistent premise entails everything, and a failure to
//! find a mapping means nothing at all unless the rule set is complete for the premise it
//! ran on. Every one of those steps has been got wrong somewhere, so this module does them
//! once, in the library, with the evidence attached.
//!
//! # The three answers, and why there are three
//!
//! [`EntailmentOutcome`] has no boolean anywhere in it:
//!
//! * [`Entailed`](EntailmentOutcome::Entailed) carries an [`EntailmentWarrant`] — the
//!   mapping that made it true — which [`verify`] re-decides without running a reasoner.
//! * [`NotEntailed`](EntailmentOutcome::NotEntailed) carries a [`MissReason`] and is a
//!   PROOF: it is returned only when the procedure is complete for this premise, so the
//!   absence of a mapping is the absence of an entailment.
//! * [`Undecided`](EntailmentOutcome::Undecided) carries an [`UndecidedReason`] and is
//!   returned when no mapping was found and the procedure is NOT complete here. Collapsing
//!   it into `NotEntailed` would turn a limitation of this library into a false statement
//!   about the caller's ontology, and it is the single most consequential distinction in
//!   this module.
//!
//! # Consistency is established FIRST, and it hard-fails
//!
//! An inconsistent knowledge base entails every triple. So a service that tested membership
//! in the closure of an inconsistent premise would answer `Entailed` for literally
//! everything, correctly and uselessly, and a caller reading `Entailed` would have no way
//! to tell that answer apart from a real one.
//!
//! The check is not a separate pass that could be skipped or reordered: it is the chase
//! itself, and it happens before any conclusion of that chase is readable. Seventeen of OWL
//! 2 RL's seventy-eight rules conclude `false` — `eq-diff1..3`, `prp-irp`, `prp-asyp`,
//! `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`,
//! `cls-maxqc1`, `cls-maxqc2`, `cax-dw`, `cax-adc` and `dt-not-type`, the last of which is
//! also the `D` lane's — and a body match on any of them makes [`materialize`] return
//! [`EntailError::Inconsistent`] instead of a closure. So there is no closure for this
//! module to match against, and the refusal propagates to the caller carrying the
//! [`InconsistentRun`](crate::InconsistentRun) witness that says which rule fired on which
//! asserted triples. `Simple`, `RDF` and `RDFS` state no rule whose head is `false`, so for
//! those three the check is VACUOUS rather than skipped: there is no rule that could have
//! detected an inconsistency, and this module does not pretend one ran.
//!
//! # Which regimes, and why not all seven
//!
//! The parameter is a [`Regime`] and not a [`Materialization`],
//! because a `Materialization` has seven inhabitants and two of them are defined by an
//! input this signature does not carry: `OWL-Direct` is query-directed and `RIF` entails
//! under a rule set the caller wrote. Accepting them here and quietly doing something else
//! would be worse than refusing, so they are refused —
//! [`EntailError::UnsupportedRegime`], a caller-visible error naming the regime, never a
//! fallback to a weaker one. The five rule-table regimes (`Simple`, `RDF`, `RDFS`,
//! `OWL-RL`, `D`) are served, each with its own completeness condition; see
//! [`precondition`] for which theorem each condition is the hypothesis of.
//!
//! # `owl:imports` is resolved or refused
//!
//! OWL 2 defines an ontology's imports closure to BE the ontology, so a premise that imports
//! a document this call was not handed is a DIFFERENT premise from the one the caller asked
//! about. Every answer over it would be about that other premise, so an unresolvable import
//! is [`EntailError::UnresolvedImport`] naming the document — never a silently truncated
//! premise. See [`imports`].
//!
//! # Two mechanisms, and both are named
//!
//! * [`homomorphism`] — the chase-and-graph-match procedure OWL 2 Profiles §4.3 states the
//!   RL entailment relation in terms of. It is complete for every conclusion the rule table
//!   can produce.
//! * [`refutation`] — assert the conclusion's negation into the premise, re-chase, and read
//!   the profile's own seventeen `false`-concluding rules as the proof. It exists because
//!   the rule table produces no NEGATIVE FACT at all: no head in Tables 4–9 is an
//!   `owl:differentFrom` or a membership in an `owl:complementOf` class, so a premise can
//!   entail one while a forward chase derives nothing to match against. It adds no rule —
//!   `rules`, `implemented` and `extensions` are untouched — and it runs only after the
//!   premise's consistency has been established, which is the hypothesis its whole soundness
//!   argument rests on. See its module docs for that argument, written out.
//!
//! [`EntailmentWarrant`] therefore has exactly two arms, one per mechanism, each minted by
//! the mechanism it names. A third arrives with its own producer, together — this crate does
//! not pre-declare states that nothing constructs.
//!
//! The refutation lane is [`entails`]-only, and deliberately: it decides a ground negative
//! fact, and a projected variable ranging over one is a different question — "which
//! individuals is `a` entailed to differ from?" would need a refutation per candidate over
//! the whole domain, which is not what [`certain_answers`] computes and not what it would be
//! honest to let it claim.
//!
//! # Determinism
//!
//! Everything below is a function of the inputs alone: the closure's frozen quad order, a
//! stable most-constrained-first pattern sort, `BTreeMap`/`BTreeSet` iteration, and a STEP
//! budget rather than a clock. Two runs over one premise and one question return the same
//! verdict, the same binding, and the same diagnosis, on `wasm32` as on native.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{RdfDataset, TermValue};

use crate::owl_dl::query::QTriple;
use crate::report::ReasoningReport;
use crate::{EntailError, Materialization, Regime, materialize};

pub mod answers;
pub mod homomorphism;
pub mod imports;
pub mod negation;
pub mod precondition;
pub mod refutation;
pub mod warrant;

// Two support modules with no public items of their own: the owned triple view both sides
// of a match are read through, and the pattern the question is compiled to. `VarKey` is the
// one thing a caller sees out of either, and it is re-exported below.
mod graph;
mod pattern;

pub use answers::CertainAnswers;
pub use homomorphism::{Binding, MATCH_BUDGET, MissReason};
pub use imports::ImportMap;
pub use negation::NegativeFact;
pub use pattern::VarKey;
pub use precondition::UndecidedReason;
pub use refutation::{REFUTATION_BUDGET, Refutation, RefutationWarrant};
pub use warrant::{EntailmentWarrant, HomomorphismWarrant, verify};

use graph::default_graph_triples;
use homomorphism::Closure;
use pattern::{PatTriple, bgp_patterns, conclusion_patterns, projected_vars};

/// What a conclusion-directed question answered.
///
/// Three answers, never two. See the [module docs](self) for why `Undecided` cannot be
/// folded into `NotEntailed`.
#[derive(Debug, Clone)]
pub enum EntailmentOutcome {
    /// The premise entails the conclusion, and here is the evidence.
    Entailed(EntailmentWarrant),
    /// The premise does NOT entail the conclusion. A proof: the procedure was complete for
    /// this premise, so the absence of a mapping is the absence of an entailment.
    NotEntailed(MissReason),
    /// No mapping was found AND the procedure is not complete for this premise, so nothing
    /// is proven in either direction.
    Undecided(UndecidedReason),
}

/// The plan for a regime, or a refusal.
///
/// Written as a total match with no wildcard so an eighth regime cannot be added without
/// deciding, here, whether this service can serve it.
const fn plan_for(regime: Regime) -> Result<Materialization<'static>, EntailError> {
    match regime {
        Regime::Simple => Ok(Materialization::Simple),
        Regime::Rdf => Ok(Materialization::Rdf),
        Regime::Rdfs => Ok(Materialization::Rdfs),
        Regime::OwlRl => Ok(Materialization::OwlRl),
        Regime::D => Ok(Materialization::D),
        // Defined by an input this signature does not carry: the query's class expressions,
        // and the caller's rule document. Refused rather than approximated by a weaker lane.
        Regime::OwlDirect | Regime::Rif => Err(EntailError::UnsupportedRegime(regime)),
    }
}

/// One prepared run: the premise's imports resolved, its consistency established, and its
/// closure indexed.
struct Prepared {
    /// The imports closure, when the premise imported anything. `None` is the common case
    /// and costs nothing: a premise that imports nothing is its own effective premise, and
    /// copying it to say so would be a full dataset copy per call.
    merged: Option<Arc<RdfDataset>>,
    /// The indexed closure a question is matched against.
    closure: Closure,
    /// What the run did, which two of the completeness conditions are read from.
    report: ReasoningReport,
}

impl Prepared {
    /// The premise the run was actually over.
    fn effective<'a>(&'a self, premise: &'a RdfDataset) -> &'a RdfDataset {
        self.merged.as_deref().unwrap_or(premise)
    }
}

/// Resolve imports, run the chase, and index the closure.
///
/// Everything that can refuse happens here, in the order the refusals have to happen in: an
/// unresolvable import before the chase (because it changes what the premise IS), and the
/// chase's own inconsistency refusal before any conclusion of it is readable.
fn prepare(
    premise: &RdfDataset,
    regime: Regime,
    imports: &ImportMap,
) -> Result<Prepared, EntailError> {
    let plan = plan_for(regime)?;
    let merged = imports::resolve(premise, imports)?;
    let (closure, report) = materialize(merged.as_deref().unwrap_or(premise), plan)?;
    Ok(Prepared {
        merged,
        closure: Closure::of(default_graph_triples(&closure)),
        report,
    })
}

/// The certain answers of `bgp` over `premise` under `regime`.
///
/// A row is a substitution the knowledge base ENTAILS the pattern under — true in every
/// model, not merely present in one closure — over the premise's own terms, as SPARQL's
/// entailment regimes require. Every row is sound unconditionally; whether the row set is
/// exhaustive is [`CertainAnswers::is_complete`], derived from the same completeness
/// conditions [`entails`] uses.
///
/// A `?v` of the pattern is projected and appears in [`CertainAnswers::vars`]; a blank node
/// of the pattern is a non-distinguished variable, constrained by the match and not
/// projected, which is what SPARQL says a query blank node is.
///
/// # Errors
///
/// [`EntailError::UnsupportedRegime`] for a regime defined by an input this signature does
/// not carry; [`EntailError::UnresolvedImport`] for an `owl:imports` the map does not
/// resolve; [`EntailError::Inconsistent`] for a premise with no model, carrying the witness
/// and the run's report; [`EntailError::MatchBudget`] if the match exhausts
/// [`MATCH_BUDGET`]; and whatever [`materialize`] refuses with.
///
/// ```
/// use purrdf_core::{RdfDatasetBuilder, TermValue};
/// use purrdf_entail::{ImportMap, QNode, QTriple, Regime, certain_answers};
///
/// let mut b = RdfDatasetBuilder::new();
/// let cat = b.intern_iri("http://example.org/Cat");
/// let animal = b.intern_iri("http://example.org/Animal");
/// let tom = b.intern_iri("http://example.org/tom");
/// let sub = b.intern_iri("http://www.w3.org/2000/01/rdf-schema#subClassOf");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// b.push_quad(cat, sub, animal, None);
/// b.push_quad(tom, ty, cat, None);
/// let premise = b.freeze().expect("freeze");
///
/// // `?c` ranges over the ENTAILED types of `tom`, not the asserted one.
/// let bgp = [QTriple {
///     s: QNode::Term(TermValue::iri("http://example.org/tom")),
///     p: QNode::Term(TermValue::iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")),
///     o: QNode::Var("c".to_owned()),
/// }];
/// let answers = certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new())
///     .expect("a consistent premise");
/// assert_eq!(answers.vars(), ["c"]);
/// assert!(answers.rows().iter().any(|row| row == &[TermValue::iri("http://example.org/Animal")]));
/// ```
pub fn certain_answers(
    premise: &RdfDataset,
    bgp: &[QTriple],
    regime: Regime,
    imports: &ImportMap,
) -> Result<CertainAnswers, EntailError> {
    let prepared = prepare(premise, regime, imports)?;
    let pats = bgp_patterns(bgp);
    let limits = precondition::limits(regime, prepared.effective(premise), &prepared.report, &pats);
    let names = projected_vars(&pats);
    let vars: Vec<VarKey> = names.iter().cloned().map(VarKey::Projected).collect();
    let rows: BTreeSet<Vec<TermValue>> = homomorphism::find_all(pats, &prepared.closure, &vars)?;
    Ok(CertainAnswers::new(
        regime,
        names,
        rows.into_iter().collect(),
        limits,
    ))
}

/// Does `premise` entail `conclusion` under `regime`?
///
/// The zero-projected-variable specialisation of [`certain_answers`]: an RDF graph is a
/// conjunction of triples whose blank nodes are existentially quantified, so a conclusion
/// GRAPH is a basic graph pattern with nothing to project, and its answer is a verdict
/// rather than a relation. It runs the same mechanism through the same completeness
/// conditions; what differs is that the binding is read as the WARRANT for a yes rather
/// than as an answer.
///
/// # Errors
///
/// As [`certain_answers`].
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{EntailmentOutcome, ImportMap, Regime, entails};
///
/// let mut b = RdfDatasetBuilder::new();
/// let p = b.intern_iri("http://example.org/p");
/// let x = b.intern_iri("http://example.org/x");
/// let y = b.intern_iri("http://example.org/y");
/// let z = b.intern_iri("http://example.org/z");
/// let ty = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// let transitive = b.intern_iri("http://www.w3.org/2002/07/owl#TransitiveProperty");
/// b.push_quad(p, ty, transitive, None);
/// b.push_quad(x, p, y, None);
/// b.push_quad(y, p, z, None);
/// let premise = b.freeze().expect("freeze");
///
/// // `x p z` follows by `prp-trp` and is not asserted.
/// let mut c = RdfDatasetBuilder::new();
/// let x = c.intern_iri("http://example.org/x");
/// let p = c.intern_iri("http://example.org/p");
/// let z = c.intern_iri("http://example.org/z");
/// c.push_quad(x, p, z, None);
/// let conclusion = c.freeze().expect("freeze");
///
/// let outcome = entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
///     .expect("a consistent premise");
/// assert!(matches!(outcome, EntailmentOutcome::Entailed(_)));
/// ```
pub fn entails(
    premise: &RdfDataset,
    conclusion: &RdfDataset,
    regime: Regime,
    imports: &ImportMap,
) -> Result<EntailmentOutcome, EntailError> {
    let prepared = prepare(premise, regime, imports)?;
    let pats: Vec<PatTriple> = conclusion_patterns(conclusion);
    let limits = precondition::limits(regime, prepared.effective(premise), &prepared.report, &pats);
    match homomorphism::find_one(pats, &prepared.closure)? {
        // A found mapping is a proof, and it needs no precondition: the rule set is sound,
        // so a conclusion mapped into the closure is entailed whatever the premise's syntax.
        Ok(binding) => Ok(EntailmentOutcome::Entailed(
            EntailmentWarrant::Homomorphism(HomomorphismWarrant::new(
                regime,
                binding,
                prepared.closure,
            )),
        )),
        // No mapping. Before that is read as anything, the second mechanism gets its turn:
        // a conclusion the rule table has no head for is exactly the case a match cannot
        // reach and a refutation can. It runs HERE and not earlier because it is strictly
        // more expensive — one full re-chase per negative fact — and because the premise's
        // consistency, which its soundness argument requires, is what `prepare` above has
        // just established.
        Err(miss) => match refutation::attempt(
            prepared.effective(premise),
            conclusion,
            regime,
            &prepared.closure,
        )? {
            refutation::Attempt::Entailed(warrant) => Ok(EntailmentOutcome::Entailed(
                EntailmentWarrant::Refutation(*warrant),
            )),
            // The lane ran and stopped early. "I stopped looking" is not "there is nothing
            // to find", so it is never allowed to become a refutation.
            refutation::Attempt::Exhausted { needed } => Ok(EntailmentOutcome::Undecided(
                UndecidedReason::RefutationBudget(needed),
            )),
            // Neither mechanism reached it. What that MEANS is the precondition's answer,
            // not either search's.
            refutation::Attempt::NotApplicable | refutation::Attempt::NotEstablished => {
                Ok(match limits.into_iter().next() {
                    Some(reason) => EntailmentOutcome::Undecided(reason),
                    None => EntailmentOutcome::NotEntailed(miss),
                })
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

    use super::{
        CertainAnswers, EntailmentOutcome, ImportMap, MissReason, UndecidedReason, certain_answers,
        entails, verify,
    };
    use crate::owl_dl::query::{QNode, QTriple};
    use crate::{EntailError, Regime};

    const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    const DISJOINT: &str = "http://www.w3.org/2002/07/owl#disjointWith";
    const SOMEVALUES: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
    const ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    const RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";

    fn graph(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = b.intern_iri(s);
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// The premise `A ⊑ B`, `x a A` — enough for `cax-sco` to type `x` a `B`.
    fn subclass_premise() -> Arc<RdfDataset> {
        graph(&[
            ("http://example.org/A", SUBCLASS, "http://example.org/B"),
            ("http://example.org/x", TYPE, "http://example.org/A"),
        ])
    }

    /// A DERIVED conclusion is entailed, and the warrant re-checks.
    #[test]
    fn a_derived_conclusion_is_entailed_and_the_warrant_verifies() {
        let premise = subclass_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        let EntailmentOutcome::Entailed(warrant) =
            entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).expect("consistent")
        else {
            panic!("cax-sco derives it");
        };
        assert_eq!(warrant.regime(), Regime::OwlRl);
        assert!(verify(&warrant, &premise, &conclusion));
        // The warrant is against THIS premise: a closure that does not hold the premise's
        // own triples is not a warrant for it.
        let other = graph(&[("http://example.org/q", TYPE, "http://example.org/Q")]);
        assert!(!verify(&warrant, &other, &conclusion));
        // …and it is against THIS conclusion.
        let unrelated = graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]);
        assert!(!verify(&warrant, &premise, &unrelated));
    }

    /// A conclusion nothing derives, over a premise inside OWL 2 RL, is a PROOF of
    /// non-entailment — and it says which triple was missing.
    #[test]
    fn an_rl_premise_refutes_and_names_the_missing_triple() {
        let premise = subclass_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]);
        let EntailmentOutcome::NotEntailed(MissReason::NoCandidate(missing)) =
            entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).expect("consistent")
        else {
            panic!("the closure of an RL premise refutes");
        };
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("Never"), "{missing:?}");
    }

    /// THE CENTRAL DISTINCTION: a premise OUTSIDE OWL 2 RL cannot refute.
    ///
    /// `owl:someValuesFrom` in SUPERCLASS position is outside the RL syntax, so Theorem PR1's
    /// completeness half does not apply and a failed match proves nothing. The same
    /// conclusion over an RL premise refutes (above), so this is a fact about the
    /// PRECONDITION rather than about the conclusion.
    #[test]
    fn a_non_rl_premise_is_undecided_rather_than_refuted() {
        // `A ⊑ ∃p.B` — an existential on the SUPERCLASS side.
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/A");
        let sub = b.intern_iri(SUBCLASS);
        let restriction = b.intern_blank("r", purrdf_core::BlankScope::DEFAULT);
        let ty = b.intern_iri(TYPE);
        let class = b.intern_iri(RESTRICTION);
        let on = b.intern_iri(ONPROPERTY);
        let p = b.intern_iri("http://example.org/p");
        let some = b.intern_iri(SOMEVALUES);
        let bb = b.intern_iri("http://example.org/B");
        b.push_quad(a, sub, restriction, None);
        b.push_quad(restriction, ty, class, None);
        b.push_quad(restriction, on, p, None);
        b.push_quad(restriction, some, bb, None);
        let x = b.intern_iri("http://example.org/x");
        b.push_quad(x, ty, a, None);
        let premise = b.freeze().expect("freeze");

        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/Never")]);
        let EntailmentOutcome::Undecided(UndecidedReason::PremiseOutsideRl(violations)) =
            entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).expect("consistent")
        else {
            panic!("an existential in superclass position is outside OWL 2 RL");
        };
        assert!(!violations.is_empty());
    }

    /// AN INCONSISTENT PREMISE ENTAILS EVERYTHING, SO IT IS REFUSED.
    ///
    /// Falsifiable against the failure mode this service is arranged to prevent: without the
    /// consistency check the closure would be matched anyway, and the answer for this
    /// conclusion — for EVERY conclusion — would be `Entailed`.
    #[test]
    fn an_inconsistent_premise_refuses_rather_than_entailing_everything() {
        let premise = graph(&[
            ("http://example.org/A", DISJOINT, "http://example.org/B"),
            ("http://example.org/x", TYPE, "http://example.org/A"),
            ("http://example.org/x", TYPE, "http://example.org/B"),
        ]);
        let conclusion = graph(&[("http://example.org/anything", TYPE, "http://example.org/At")]);
        let Err(EntailError::Inconsistent(run)) =
            entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
        else {
            panic!("two disjoint classes with a shared instance is `cax-dw`");
        };
        assert_eq!(run.report().regime(), Regime::OwlRl);
        assert!(run.report().inconsistency().is_some());
    }

    /// A blank node of the conclusion is an EXISTENTIAL, and the warrant says what it was.
    #[test]
    fn a_conclusion_blank_node_is_bound_and_the_binding_is_the_warrant() {
        let premise = subclass_premise();
        let mut c = RdfDatasetBuilder::new();
        let some = c.intern_blank("who", purrdf_core::BlankScope::DEFAULT);
        let ty = c.intern_iri(TYPE);
        let bb = c.intern_iri("http://example.org/B");
        c.push_quad(some, ty, bb, None);
        let conclusion = c.freeze().expect("freeze");

        let EntailmentOutcome::Entailed(warrant) =
            entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).expect("consistent")
        else {
            panic!("`_:who a B` holds of `x`");
        };
        assert_eq!(warrant.binding().len(), 1);
        assert_eq!(
            warrant.binding().values().next(),
            Some(&TermValue::iri("http://example.org/x"))
        );
        assert!(verify(&warrant, &premise, &conclusion));
    }

    /// The two regimes defined by an input this signature does not carry are REFUSED, by
    /// name, rather than served by a weaker lane.
    #[test]
    fn a_regime_this_service_cannot_serve_is_named() {
        let premise = subclass_premise();
        let conclusion = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        for regime in [Regime::OwlDirect, Regime::Rif] {
            let Err(EntailError::UnsupportedRegime(refused)) =
                entails(&premise, &conclusion, regime, &ImportMap::new())
            else {
                panic!("{regime:?} is defined by an input this signature does not carry");
            };
            assert_eq!(refused, regime);
        }
        // …and the five that ARE served all answer.
        for regime in [
            Regime::Simple,
            Regime::Rdf,
            Regime::Rdfs,
            Regime::OwlRl,
            Regime::D,
        ] {
            entails(&premise, &conclusion, regime, &ImportMap::new())
                .unwrap_or_else(|e| panic!("{regime:?}: {e}"));
        }
    }

    /// `Simple` entailment is the identity closure plus the match, so it entails what is
    /// ASSERTED and refutes what is not — and its refutation is a proof, because the
    /// interpolation lemma leaves nothing to be incomplete about.
    #[test]
    fn simple_entailment_refutes_without_a_precondition() {
        let premise = subclass_premise();
        let asserted = graph(&[("http://example.org/x", TYPE, "http://example.org/A")]);
        assert!(matches!(
            entails(&premise, &asserted, Regime::Simple, &ImportMap::new()).expect("consistent"),
            EntailmentOutcome::Entailed(_)
        ));
        let derived = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        assert!(
            matches!(
                entails(&premise, &derived, Regime::Simple, &ImportMap::new()).expect("consistent"),
                EntailmentOutcome::NotEntailed(_)
            ),
            "Simple entailment draws no conclusion, and says so as a PROOF"
        );
    }

    /// `D` can prove an entailment and never refutes one, and it says which limit that is.
    #[test]
    fn the_d_lane_proves_but_does_not_refute() {
        let premise = subclass_premise();
        let derived = graph(&[("http://example.org/x", TYPE, "http://example.org/B")]);
        assert!(matches!(
            entails(&premise, &derived, Regime::D, &ImportMap::new()).expect("consistent"),
            EntailmentOutcome::Undecided(UndecidedReason::DatatypeValueSpace)
        ));
    }

    /// A projected variable enumerates the ENTAILED bindings, and the answer set says
    /// whether it is exhaustive.
    #[test]
    fn certain_answers_enumerate_entailed_bindings_and_disclose_completeness() {
        let premise = subclass_premise();
        let bgp = [QTriple {
            s: QNode::Term(TermValue::iri("http://example.org/x")),
            p: QNode::Term(TermValue::iri(TYPE)),
            o: QNode::Var("c".to_owned()),
        }];
        let answers: CertainAnswers =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert_eq!(answers.vars(), ["c"]);
        assert!(answers.is_complete(), "{:?}", answers.limits());
        for class in ["http://example.org/A", "http://example.org/B"] {
            assert!(
                answers
                    .rows()
                    .iter()
                    .any(|row| row == &[TermValue::iri(class)]),
                "{class} is an entailed type of x: {:?}",
                answers.rows()
            );
        }
        // The rows are deduplicated and ordered by the row itself, so two runs agree.
        let again =
            certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new()).expect("consistent");
        assert_eq!(answers.rows(), again.rows());
    }

    /// `entails` IS `certain_answers` with nothing to project: over the same premise and the
    /// same question they agree about whether an answer exists.
    #[test]
    fn entails_is_the_zero_projected_variable_case() {
        let premise = subclass_premise();
        for (object, expected) in [
            ("http://example.org/B", true),
            ("http://example.org/Never", false),
        ] {
            let conclusion = graph(&[("http://example.org/x", TYPE, object)]);
            let bgp = [QTriple {
                s: QNode::Term(TermValue::iri("http://example.org/x")),
                p: QNode::Term(TermValue::iri(TYPE)),
                o: QNode::Term(TermValue::iri(object)),
            }];
            let verdict = matches!(
                entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
                    .expect("consistent"),
                EntailmentOutcome::Entailed(_)
            );
            let answers = certain_answers(&premise, &bgp, Regime::OwlRl, &ImportMap::new())
                .expect("consistent");
            assert_eq!(verdict, expected, "{object}");
            assert_eq!(verdict, !answers.is_empty(), "{object}");
        }
    }
}
