// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`purrdf::query_with_entailment_governed`], driven through the public facade.
//!
//! # What this file is here to pin
//!
//! An entailment-regime query is two phases, and the governed entry point governs both —
//! differently, and for reasons that are about semantics rather than effort. These tests
//! assert the split as a behaviour rather than as prose:
//!
//! * every ceiling the caller names is in force over the SPARQL evaluation of the closure,
//!   and trips THERE (not before the closure, not nowhere);
//! * a wall deadline or a cancellation additionally reaches the CLOSURE's own fixpoint, so a
//!   host's deadline is honest even when materializing is the expensive half — and a run it
//!   stops claims nothing at all, not an empty answer;
//! * the [`ReasoningReport`] travels with the answer on BOTH arms of the outcome, because a
//!   truncated answer over a closure is unreadable without knowing what closed it;
//! * an ungoverned entailment query is byte-for-byte the one it always was;
//! * and the OWL-Direct combined approach's chase-minted witnesses never reach a caller
//!   through a PARTIAL answer, which is a route a complete-answer-only filtration would have
//!   left open.
//!
//! Every fixture uses `example.org`.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use purrdf::entail::{Regime, materialize_combined};
use purrdf::sparql::{
    CancellationFlag, GovernedOutcome, NativeSparqlEngine, PartialAnswers, QueryGovernors,
    ResourceDimension, StopCause, StopSignal, TrippedGovernor, WallDeadline,
};
use purrdf::{BlankScope, RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf::{GovernedEntailment, QueryEntailment, query_with_entailment};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const NS: &str = "http://example.org/";

/// A three-instance class hierarchy: enough rows for an answer cap to cut, and enough
/// vocabulary for the RDFS closure to be substantially larger than the assertion.
fn hierarchy() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let subclass = b.intern_iri(RDFS_SUBCLASS);
    let cat = b.intern_iri(&format!("{NS}Cat"));
    let animal = b.intern_iri(&format!("{NS}Animal"));
    b.push_quad(cat, subclass, animal, None);
    for name in ["lillith", "tom", "mia"] {
        let individual = b.intern_iri(&format!("{NS}{name}"));
        b.push_quad(individual, ty, cat, None);
    }
    b.freeze().expect("the fixture freezes")
}

/// The query every closure test below answers: three rows under RDFS, zero without it.
const DERIVED_ANIMALS: &str = "SELECT ?x WHERE { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal> }";

/// Run `query` over `dataset` under `entailment` and `governors`.
fn governed(
    dataset: &Arc<RdfDataset>,
    query: &str,
    entailment: QueryEntailment<'_>,
    governors: &QueryGovernors,
) -> GovernedEntailment {
    query_with_entailment_governed_(dataset, query, entailment, governors).expect("the run decides")
}

/// [`governed`], without the unwrap, for the tests that assert on the `Result`.
fn query_with_entailment_governed_(
    dataset: &Arc<RdfDataset>,
    query: &str,
    entailment: QueryEntailment<'_>,
    governors: &QueryGovernors,
) -> Result<GovernedEntailment, purrdf::ReasoningError> {
    purrdf::query_with_entailment_governed(
        &NativeSparqlEngine::new(),
        dataset,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        entailment,
        governors,
    )
}

/// The rows of a solution result, as the projected variable's bindings.
fn bindings(result: &SparqlResult) -> Vec<TermValue> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected a solution sequence");
    };
    rows.iter()
        .map(|row| {
            row[0]
                .as_ref()
                .expect("the projected variable is bound")
                .clone()
        })
        .collect()
}

// ── The evaluation over the closure is governed, and trips there ──────────────────────

/// EVERY CEILING REACHES THE EVALUATION OVER THE ENTAILED CLOSURE.
///
/// Each of the four chargeable ceilings is set to a value the query over the RDFS closure
/// passes, and each one trips — reported as its OWN governor, so a ceiling cannot pass this
/// test by being enforced under a neighbour's name. The trip is on the `Answered` arm,
/// which is the point: the closure was computed, and the ceiling bounded the query over it.
#[test]
fn each_ceiling_trips_over_the_entailed_closure() {
    let dataset = hierarchy();
    // Scratch bytes are minted only by a value-constructing expression, so that dimension is
    // charged by a query that constructs one; every other ceiling is charged by the plain
    // scan over the closure.
    for (governors, dimension, query) in [
        (
            QueryGovernors::UNBOUNDED.with_fuel(1),
            ResourceDimension::Fuel,
            DERIVED_ANIMALS,
        ),
        (
            QueryGovernors::UNBOUNDED.with_max_answers(1),
            ResourceDimension::AnswerRows,
            DERIVED_ANIMALS,
        ),
        (
            QueryGovernors::UNBOUNDED.with_max_intermediate_cells(1),
            ResourceDimension::IntermediateCells,
            DERIVED_ANIMALS,
        ),
        (
            QueryGovernors::UNBOUNDED.with_max_scratch_bytes(0),
            ResourceDimension::ScratchBytes,
            CONCAT_QUERY,
        ),
    ] {
        let answered = governed(&dataset, query, QueryEntailment::Rdfs, &governors);
        let tripped = answered
            .tripped()
            .unwrap_or_else(|| panic!("{dimension:?} must trip over the closure"));
        match tripped {
            TrippedGovernor::Budget { dimension: d, .. }
            | TrippedGovernor::Refused { dimension: d, .. } => assert_eq!(
                d, dimension,
                "a ceiling must trip under its OWN governor, not a neighbour's"
            ),
            // `TrippedGovernor` is `#[non_exhaustive]`; a governor this match has never seen
            // is not the one the ceiling named, which is the whole assertion.
            other => panic!("{dimension:?} is a ceiling, and {other:?} is not it"),
        }
        assert!(
            answered.report().is_some(),
            "{dimension:?}: the closure WAS computed, so its certificate must travel with \
             the truncated answer"
        );
    }
}

/// A query whose `CONCAT` mints a term into the per-query scratch arena.
const CONCAT_QUERY: &str = "SELECT (CONCAT(STR(?x), \"!\") AS ?label) WHERE { ?x <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal> }";

/// THE REMOTE-REQUEST CEILING REACHES THE ENGINE over a closure too.
///
/// Nothing in this test issues a `SERVICE` request, so the honest evidence that the flag
/// arrived is that the engine is enforcing the number: the evidence vector a governed run
/// over the closure hands back reports the ceiling as bounded, at the value the caller set.
#[test]
fn the_remote_request_ceiling_is_in_force_over_the_closure() {
    let answered = governed(
        &hierarchy(),
        DERIVED_ANIMALS,
        QueryEntailment::Rdfs,
        &QueryGovernors::UNBOUNDED.with_max_remote_requests(7),
    );
    let GovernedEntailment::Answered { outcome, .. } = &answered else {
        panic!("an unreachable ceiling must not stop the closure");
    };
    let limits = outcome.evidence().limits();
    assert!(limits.is_bounded(ResourceDimension::RemoteRequests));
    assert_eq!(limits.get(ResourceDimension::RemoteRequests), 7);
}

/// A CEILING NOTHING REACHES CHANGES NOTHING: the governed answer over the closure is the
/// ungoverned one, row for row, and the report is the same regime's.
#[test]
fn an_unreached_ceiling_answers_exactly_as_the_ungoverned_lane_does() {
    let dataset = hierarchy();
    let (ungoverned, ungoverned_report) = query_with_entailment(
        &NativeSparqlEngine::new(),
        &dataset,
        SparqlRequest {
            query: DERIVED_ANIMALS,
            base_iri: None,
            substitutions: &[],
        },
        QueryEntailment::Rdfs,
    )
    .expect("the ungoverned lane answers");

    let answered = governed(
        &dataset,
        DERIVED_ANIMALS,
        QueryEntailment::Rdfs,
        &QueryGovernors::UNBOUNDED.with_max_answers(1_000),
    );
    let GovernedEntailment::Answered { outcome, report } = answered else {
        panic!("nothing was reached, so nothing may stop");
    };
    let GovernedOutcome::Complete { result, .. } = outcome else {
        panic!("nothing was reached, so the outcome is complete");
    };
    assert_eq!(bindings(&result), bindings(&ungoverned));
    assert_eq!(
        format!("{report:?}"),
        format!("{ungoverned_report:?}"),
        "the same closure was computed, so the same certificate describes it"
    );
}

/// THE UNGOVERNED ENTRY POINT IS UNTOUCHED, in every regime it serves.
///
/// The governed lane is a sibling rather than a rewrite, and this is the assertion that says
/// so: each regime's ungoverned answer and certificate are what they were, and each equals
/// what the governed lane produces under `UNBOUNDED`.
#[test]
fn the_ungoverned_lane_is_byte_for_byte_unchanged() {
    let dataset = hierarchy();
    let rules = purrdf::entail::RuleSet::new();
    for (mode, regime) in [
        (QueryEntailment::Simple, Regime::Simple),
        (QueryEntailment::Rdf, Regime::Rdf),
        (QueryEntailment::Rdfs, Regime::Rdfs),
        (QueryEntailment::OwlRl, Regime::OwlRl),
        (QueryEntailment::D, Regime::D),
        (QueryEntailment::OwlDirect, Regime::OwlDirect),
        (QueryEntailment::Rif(&rules), Regime::Rif),
    ] {
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &dataset,
            SparqlRequest {
                query: DERIVED_ANIMALS,
                base_iri: None,
                substitutions: &[],
            },
            mode,
        )
        .expect("the ungoverned lane answers");
        assert_eq!(report.regime(), regime, "{regime:?}");

        let answered = governed(&dataset, DERIVED_ANIMALS, mode, &QueryGovernors::UNBOUNDED);
        let GovernedEntailment::Answered {
            outcome,
            report: governed_report,
        } = answered
        else {
            panic!("{regime:?}: UNBOUNDED names no signal, so nothing can stop");
        };
        let GovernedOutcome::Complete {
            result: governed_result,
            ..
        } = outcome
        else {
            panic!("{regime:?}: UNBOUNDED sets no ceiling, so nothing can trip");
        };
        assert_eq!(bindings(&governed_result), bindings(&result), "{regime:?}");
        assert_eq!(
            format!("{governed_report:?}"),
            format!("{report:?}"),
            "{regime:?}"
        );
    }
}

/// THE CERTIFICATE TRAVELS WITH A TRUNCATED ANSWER, and it is the closure's own.
///
/// A partial answer over an OWL 2 RL closure is exactly the answer that most needs its
/// certificate: the rows came from somewhere the assertions do not state, and the caller has
/// fewer of them than the query has.
#[test]
fn the_reasoning_report_travels_with_a_tripped_answer() {
    let answered = governed(
        &hierarchy(),
        DERIVED_ANIMALS,
        QueryEntailment::OwlRl,
        &QueryGovernors::UNBOUNDED.with_max_answers(1),
    );
    let GovernedEntailment::Answered { outcome, report } = answered else {
        panic!("a ceiling is not a stop signal");
    };
    assert_eq!(report.regime(), Regime::OwlRl);
    let GovernedOutcome::BudgetExhausted(exhausted) = outcome else {
        panic!("one answer of three must trip the cap");
    };
    assert!(matches!(
        exhausted.tripped,
        TrippedGovernor::Budget {
            dimension: ResourceDimension::AnswerRows,
            ..
        }
    ));
    let partial = exhausted
        .partial
        .result()
        .expect("a projected SELECT is monotone, so its rows cross");
    assert!(exhausted.partial.is_certain());
    assert_eq!(bindings(partial.result()).len(), 1);
}

// ── The stop signal reaches the closure itself ────────────────────────────────────────

/// AN ALREADY-EXPIRED DEADLINE STOPS THE CLOSURE, AND CLAIMS NOTHING.
///
/// This is the half a charge schedule could not buy. The deadline is expired before the
/// first fixpoint round, so the run ends in phase one — where there is no query result to
/// truncate and therefore nothing that could be mistaken for one. The outcome arm carries no
/// rows and no report by construction.
#[test]
fn an_expired_deadline_stops_the_closure_with_nothing_claimed() {
    let signal: Arc<dyn StopSignal> = Arc::new(WallDeadline::after(Duration::ZERO));
    let answered = governed(
        &hierarchy(),
        DERIVED_ANIMALS,
        QueryEntailment::Rdfs,
        &QueryGovernors::UNBOUNDED.with_stop_signal(signal),
    );
    assert!(matches!(
        answered,
        GovernedEntailment::ClosureStopped {
            tripped: TrippedGovernor::Stopped {
                cause: StopCause::Deadline
            }
        }
    ));
    assert!(
        answered.outcome().is_none() && answered.report().is_none(),
        "a stopped closure has no answer and no certificate, and the type says so"
    );
    assert!(!answered.is_complete());
}

/// A CANCELLATION STOPS THE CLOSURE TOO, and reports itself as one.
///
/// The cause is carried out rather than collapsed into a generic stop: an operator who
/// cancelled a run and an operator whose deadline elapsed need to tell those apart.
#[test]
fn a_cancellation_stops_the_closure_and_names_itself() {
    let flag = CancellationFlag::new();
    flag.cancel();
    let signal: Arc<dyn StopSignal> = Arc::new(flag);
    let answered = governed(
        &hierarchy(),
        DERIVED_ANIMALS,
        QueryEntailment::OwlRl,
        &QueryGovernors::UNBOUNDED.with_stop_signal(signal),
    );
    assert_eq!(
        answered.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
}

/// EVERY REGIME HONOURS A SIGNAL THAT IS ALREADY FIRING, and none of them answers.
///
/// The lanes reach different evaluators (`Rdf`/`Rdfs` the restricted chase, `OwlRl`/`D` the
/// semi-naive fixpoint, `OwlDirect` the hypertableau and the consequence-based classifier,
/// `Rif` its own fixpoint), so a signal honoured in one says nothing about the others.
///
/// `Simple` stops in the OTHER phase, and that is the correct place rather than a gap: the
/// identity closure is an `Arc` clone, so there is no closure computation to stop, and the
/// same signal is the one the SPARQL evaluator then observes. Both phases are asserted here
/// through the shared `tripped()` accessor — the fact a caller acts on is "this run was
/// stopped", which is one fact and reads the same either way — and the phase is asserted
/// separately, so a lane that quietly stopped answering would still fail.
#[test]
fn every_regime_honours_a_signal_that_is_already_firing() {
    let dataset = hierarchy();
    let rules = purrdf::entail::RuleSet::new();
    for (mode, regime) in [
        (QueryEntailment::Simple, Regime::Simple),
        (QueryEntailment::Rdf, Regime::Rdf),
        (QueryEntailment::Rdfs, Regime::Rdfs),
        (QueryEntailment::OwlRl, Regime::OwlRl),
        (QueryEntailment::D, Regime::D),
        (QueryEntailment::OwlDirect, Regime::OwlDirect),
        (QueryEntailment::Rif(&rules), Regime::Rif),
    ] {
        let flag = CancellationFlag::new();
        flag.cancel();
        let signal: Arc<dyn StopSignal> = Arc::new(flag);
        let answered = governed(
            &dataset,
            DERIVED_ANIMALS,
            mode,
            &QueryGovernors::UNBOUNDED.with_stop_signal(signal),
        );
        assert_eq!(
            answered.tripped(),
            Some(TrippedGovernor::Stopped {
                cause: StopCause::Cancelled
            }),
            "{regime:?} must observe a signal that is already firing"
        );
        assert!(!answered.is_complete(), "{regime:?}");
        match regime {
            // The identity closure runs no fixpoint, so the signal is observed by the
            // evaluator: a stopped SPARQL execution, carrying whatever it certified (which,
            // having been stopped before its first charge, is nothing).
            Regime::Simple => {
                let GovernedEntailment::Answered { outcome, .. } = &answered else {
                    panic!("Simple materializes nothing, so it cannot stop in phase one");
                };
                assert!(matches!(outcome, GovernedOutcome::BudgetExhausted(_)));
            }
            // Every other lane runs a fixpoint, and the signal ends it before the query.
            _ => assert!(
                matches!(answered, GovernedEntailment::ClosureStopped { .. }),
                "{regime:?} must stop while the closure is being computed"
            ),
        }
    }
}

/// A SIGNAL THAT NEVER FIRES CHANGES NO ANSWER, in every regime.
///
/// The other half of the stop-signal argument, and the one that makes it admissible beside a
/// crate whose budgets are constants: attaching a signal is observationally nothing until it
/// fires. Asserted against the ungoverned lane rather than against itself.
#[test]
fn a_signal_that_never_fires_changes_no_closure() {
    let dataset = hierarchy();
    let rules = purrdf::entail::RuleSet::new();
    for mode in [
        QueryEntailment::Simple,
        QueryEntailment::Rdf,
        QueryEntailment::Rdfs,
        QueryEntailment::OwlRl,
        QueryEntailment::D,
        QueryEntailment::OwlDirect,
        QueryEntailment::Rif(&rules),
    ] {
        let (result, report) = query_with_entailment(
            &NativeSparqlEngine::new(),
            &dataset,
            SparqlRequest {
                query: DERIVED_ANIMALS,
                base_iri: None,
                substitutions: &[],
            },
            mode,
        )
        .expect("the ungoverned lane answers");

        // An uncancelled flag: present, polled at every round, and never firing.
        let signal: Arc<dyn StopSignal> = Arc::new(CancellationFlag::new());
        let answered = governed(
            &dataset,
            DERIVED_ANIMALS,
            mode,
            &QueryGovernors::UNBOUNDED.with_stop_signal(signal),
        );
        let GovernedEntailment::Answered {
            outcome,
            report: governed_report,
        } = answered
        else {
            panic!("{:?}: an unfired signal must stop nothing", report.regime());
        };
        let GovernedOutcome::Complete {
            result: governed_result,
            ..
        } = outcome
        else {
            panic!("{:?}: no ceiling was set", report.regime());
        };
        assert_eq!(
            bindings(&governed_result),
            bindings(&result),
            "{:?}",
            report.regime()
        );
        assert_eq!(
            format!("{governed_report:?}"),
            format!("{report:?}"),
            "{:?}",
            report.regime()
        );
    }
}

// ── The combined approach's witnesses cannot escape through a PARTIAL answer ───────────

const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";

/// `A ⊑ ∃r.B` with three `A` instances — the shape the combined approach answers by minting
/// an existential witness, and enough instances for an answer cap to cut the result in half.
fn some_values_from_ontology() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let ty = b.intern_iri(RDF_TYPE);
    let class = b.intern_iri(OWL_CLASS);
    let subclass_of = b.intern_iri(RDFS_SUBCLASS);
    let a = b.intern_iri(&format!("{NS}A"));
    let big_b = b.intern_iri(&format!("{NS}B"));
    let r = b.intern_iri(&format!("{NS}r"));
    let restriction = b.intern_blank("restriction", BlankScope::DEFAULT);
    let restriction_class = b.intern_iri(OWL_RESTRICTION);
    let on_property = b.intern_iri(OWL_ON_PROPERTY);
    let some_values_from = b.intern_iri(OWL_SOME_VALUES_FROM);
    b.push_quad(a, ty, class, None);
    b.push_quad(big_b, ty, class, None);
    b.push_quad(restriction, ty, restriction_class, None);
    b.push_quad(restriction, on_property, r, None);
    b.push_quad(restriction, some_values_from, big_b, None);
    b.push_quad(a, subclass_of, restriction, None);
    for name in ["a1", "a2", "a3"] {
        let individual = b.intern_iri(&format!("{NS}{name}"));
        b.push_quad(individual, ty, a, None);
    }
    b.freeze().expect("the fixture freezes")
}

/// The witnesses the combined approach mints over that ontology, taken from the library's
/// own materialization rather than reconstructed from the label format.
fn minted_witnesses() -> BTreeSet<String> {
    let combined = materialize_combined(&some_values_from_ontology(), &[])
        .expect("the TBox is in the certified Horn fragment")
        .expect("…so the combined approach answers");
    assert!(
        !combined.surrogates.is_empty(),
        "the fixture exists to mint a witness; if it stopped minting one, every assertion \
         below would pass vacuously"
    );
    combined.surrogates
}

/// Whether `term`, or a term quoted at any depth inside it, is one of `witnesses`.
fn term_mentions(term: &TermValue, witnesses: &BTreeSet<String>) -> bool {
    match term {
        TermValue::Blank { label, .. } => witnesses.contains(label),
        TermValue::Iri(_) | TermValue::Literal { .. } => false,
        TermValue::Triple { s, p, o } => {
            term_mentions(s, witnesses)
                || term_mentions(p, witnesses)
                || term_mentions(o, witnesses)
        }
    }
}

/// Whether any term of `result` mentions one of `witnesses`, at any depth.
fn mentions_a_witness(result: &SparqlResult, witnesses: &BTreeSet<String>) -> bool {
    match result {
        SparqlResult::Boolean(_) => false,
        SparqlResult::Solutions { rows, .. } => rows.iter().any(|row| {
            row.iter().any(|cell| {
                cell.as_ref()
                    .is_some_and(|term| term_mentions(term, witnesses))
            })
        }),
        SparqlResult::Graph(graph) => graph.quads().any(|quad| {
            [quad.s, quad.p, quad.o]
                .into_iter()
                .chain(quad.g)
                .any(|id| term_mentions(&graph.term_value(id), witnesses))
        }),
    }
}

/// NO CHASE-MINTED WITNESS REACHES A PARTIAL SOLUTION SEQUENCE.
///
/// The restriction is in the ALGEBRA, upstream of the truncation, so a governed run over the
/// combined closure evaluates the already-restricted pattern and the governor truncates the
/// restricted sequence. This asserts the consequence rather than the mechanism: the trip
/// happens, rows cross, and none of them binds a witness.
#[test]
fn no_witness_reaches_a_partial_solution_sequence() {
    let witnesses = minted_witnesses();
    let query = format!("SELECT ?y WHERE {{ ?x <{NS}r> ?y }}");
    for cap in [0_u64, 1, 2] {
        let answered = governed(
            &some_values_from_ontology(),
            &query,
            QueryEntailment::OwlDirect,
            &QueryGovernors::UNBOUNDED
                .with_max_answers(cap)
                .with_fuel(64),
        );
        let GovernedEntailment::Answered { outcome, .. } = &answered else {
            panic!("no signal was named, so nothing may stop the closure");
        };
        let result = match outcome {
            GovernedOutcome::Complete { result, .. } => result.clone(),
            GovernedOutcome::BudgetExhausted(exhausted) => match &exhausted.partial {
                PartialAnswers::Certain(partial) | PartialAnswers::AtMost(partial) => {
                    partial.result().clone()
                }
                PartialAnswers::Unknown(_) => continue,
            },
        };
        assert!(
            !mentions_a_witness(&result, &witnesses),
            "cap {cap}: a chase-minted witness must never bind an observable variable, in a \
             partial answer either: {result:?}"
        );
    }
}

/// NO CHASE-MINTED WITNESS REACHES A PARTIAL CONSTRUCTED GRAPH.
///
/// A `DESCRIBE` is the case the post-evaluation scrub exists for: it reaches triples no
/// variable of the query names, so the algebra restriction cannot touch them. Under a
/// governor the graph that reaches the caller is a PARTIAL one, which is a second place the
/// scrub has to run — and the assertion here is that it does, across every cap from "nothing
/// crossed" to "everything crossed".
///
/// The closure genuinely holds the offending triples: `witness_triples_are_in_the_closure`
/// below is what stops this test passing because there was nothing to withhold.
#[test]
fn no_witness_reaches_a_partial_constructed_graph() {
    let witnesses = minted_witnesses();
    let query = format!("DESCRIBE <{NS}a1>");
    // `true` once some cap admitted a witness-mentioning statement and the scrub took it back
    // out — i.e. once the PARTIAL path actually did the withholding rather than the cap
    // having cut before there was anything to withhold.
    let mut withheld_from_a_partial = false;
    for cap in [0_u64, 1, 2, 3, 100] {
        let answered = governed(
            &some_values_from_ontology(),
            &query,
            QueryEntailment::OwlDirect,
            &QueryGovernors::UNBOUNDED.with_max_answers(cap),
        );
        let GovernedEntailment::Answered { outcome, .. } = &answered else {
            panic!("no signal was named, so nothing may stop the closure");
        };
        let result = match outcome {
            GovernedOutcome::Complete { result, .. } => result.clone(),
            GovernedOutcome::BudgetExhausted(exhausted) => {
                match &exhausted.partial {
                    PartialAnswers::Certain(partial) | PartialAnswers::AtMost(partial) => {
                        // The positional claim is dropped exactly when the scrub removed a
                        // statement, so it is the observable that says the withholding ran.
                        withheld_from_a_partial |= !partial.is_positional_prefix();
                        partial.result().clone()
                    }
                    PartialAnswers::Unknown(_) => continue,
                }
            }
        };
        assert!(
            !mentions_a_witness(&result, &witnesses),
            "cap {cap}: a partial DESCRIBE must be scrubbed exactly as a complete one is: \
             {result:?}"
        );
    }
    assert!(
        withheld_from_a_partial,
        "one of the caps must admit a witness-mentioning statement and have it withheld — \
         otherwise this test asserts only that a cap can cut before the interesting triple"
    );
}

/// The premise the two tests above rest on: the combined closure really does state triples
/// that mention a witness, and `a1` really is the subject of one.
///
/// Without this, "no witness reached the caller" would be true of a closure that never had
/// one — which is a test that cannot fail.
#[test]
fn witness_triples_are_in_the_closure_the_scrub_runs_over() {
    let combined = materialize_combined(&some_values_from_ontology(), &[])
        .expect("the TBox is in the certified Horn fragment")
        .expect("…so the combined approach answers");
    let witnesses = &combined.surrogates;
    let a1 = format!("{NS}a1");
    assert!(
        combined.dataset.quads().any(|quad| {
            let subject = combined.dataset.term_value(quad.s);
            let object = combined.dataset.term_value(quad.o);
            matches!(&subject, TermValue::Iri(iri) if *iri == a1)
                && matches!(&object, TermValue::Blank { label, .. } if witnesses.contains(label))
        }),
        "the closure must state `a1 r <witness>` for the scrub to have work to do"
    );
}
