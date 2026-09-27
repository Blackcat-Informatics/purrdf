// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The execution scope: the one door through which a validation's SPARQL reaches a
//! `SERVICE` source, and through which a stop signal reaches the validation itself.
//!
//! Three claims, each driven through the public surface and each beside the neighbour
//! that must NOT change:
//!
//! 1. A SHACL-SPARQL constraint whose query uses `SERVICE` answers through the sources
//!    the scope installs — and its violations are the REMOTE answer's, so two remote
//!    answers give two different, correct reports. Without the scope the same shapes
//!    graph fails by name, exactly as it always has.
//! 2. The scope's stop signal ends a validation that runs no SPARQL at all: it is
//!    polled between focus nodes, not only inside queries. The neighbour — the same
//!    validation under a signal that never fires — reaches the ungoverned report.
//! 3. [`replace_ambient_context`] takes every installed scope off the thread and puts it
//!    back, which is what lets a stack-switching host park one validation and run
//!    another.

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf::RdfDataset;
use purrdf_shapes::engine::validate_graphs;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::sparql::{
    AmbientContext, QuerySources, current_governors, enter_execution_scope, replace_ambient_context,
};
use purrdf_sparql_eval::{
    CancellationFlag, GovernorState, InProcessServiceResolver, QueryGovernors, StopCause,
    StopSignal, TrippedGovernor,
};

/// Test fixtures use `example.org`; PurRDF mints no vocabulary IRIs.
const EX: &str = "http://example.org/";
const ENDPOINT: &str = "http://example.org/sparql";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// A shapes graph whose TARGET asks a remote service which people are banned, and
/// whose one constraint every person here violates (none has a clearance): every
/// violation it can find is a focus node the service's answer selected.
///
/// The `SERVICE` is in the target, not in a constraint, because that is where SHACL
/// admits it: a constraint's query runs with `$this` pre-bound, and SHACL 1.2 SPARQL
/// Extensions Appendix A forbids `SERVICE` in such a query — this crate refuses one at
/// shape load. A `sh:SPARQLTarget` query pre-binds nothing, so it may federate.
fn service_shapes(silent: bool) -> String {
    let silent = if silent { "SILENT " } else { "" };
    format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:BannedShape
    a sh:NodeShape ;
    sh:target [
        a sh:SPARQLTarget ;
        sh:select """
            SELECT ?this
            WHERE {{
                ?this a <{EX}Person> .
                SERVICE {silent}<{ENDPOINT}> {{ ?this <{EX}status> "banned" }}
            }}
        """ ;
    ] ;
    sh:property [ sh:path ex:clearance ; sh:minCount 1 ] .
"#
    )
}

/// A shapes graph that puts the same `SERVICE` in a constraint's query instead, where
/// `$this` is pre-bound.
fn service_constraint_shapes() -> String {
    format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .
ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
    sh:sparql [ sh:select """SELECT $this WHERE {{ SERVICE <{ENDPOINT}> {{ $this <{EX}status> "banned" }} }}""" ] .
"#
    )
}

/// Alice and Bob, both people.
fn people_nt() -> String {
    format!("<{EX}alice> <{RDF_TYPE}> <{EX}Person> .\n<{EX}bob> <{RDF_TYPE}> <{EX}Person> .\n")
}

/// The remote registry, banning `who`.
fn registry(who: &str) -> Arc<RdfDataset> {
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&format!(
        "<{EX}{who}> <{EX}status> \"banned\" .\n<{EX}carol> <{EX}status> \"cleared\" .\n"
    ))
    .unwrap_or_else(|errors| panic!("registry fixture: {}", errors.join("\n")))
}

/// Sources that answer `ENDPOINT` in process from `remote`.
fn sources_over(remote: Arc<RdfDataset>) -> QuerySources {
    QuerySources {
        remote: Some(Arc::new(
            InProcessServiceResolver::new().with_endpoint(ENDPOINT, remote),
        )),
        load: None,
    }
}

/// The focus nodes a report names, sorted.
fn focus_nodes(report: &ValidationReport) -> Vec<String> {
    let mut nodes: Vec<String> = report
        .results
        .iter()
        .map(|result| result.focus_node.to_string())
        .collect();
    nodes.sort();
    nodes
}

/// Validate under an execution scope over `governors` and `sources`, returning the
/// validation's own result and whatever the state latched.
fn validate_in_scope(
    shapes: &str,
    data: &str,
    governors: &QueryGovernors,
    sources: QuerySources,
) -> (Result<ValidationReport, String>, Option<TrippedGovernor>) {
    let state = Arc::new(GovernorState::new(governors));
    let result = {
        let _scope = enter_execution_scope(Arc::clone(&state), sources);
        validate_graphs(data, shapes, None)
    };
    (result, state.tripped())
}

#[test]
fn a_service_in_a_constraint_query_is_refused_at_load_with_or_without_a_scope() {
    // Where SERVICE may NOT go: a constraint's query, where `$this` is pre-bound. The
    // refusal is the specification's and precedes evaluation, so installing a source
    // changes nothing about it.
    let shapes = service_constraint_shapes();
    for (label, sources) in [
        ("no source", QuerySources::default()),
        ("a source", sources_over(registry("alice"))),
    ] {
        let (refused, _) =
            validate_in_scope(&shapes, &people_nt(), &QueryGovernors::METERED, sources);
        let refused = refused.expect_err("SERVICE under pre-binding is refused");
        assert!(
            refused.contains("federated queries (SERVICE) are not allowed"),
            "{label}: {refused}"
        );
    }
}

#[test]
fn a_service_target_answers_through_the_scope_and_its_violations_are_the_remote_answer() {
    let shapes = service_shapes(false);
    let data = people_nt();

    // The neighbour of the refusal: with no scope, the SERVICE has no source and the
    // validation fails by name — the synchronous lane's behaviour, unchanged.
    let refused = validate_graphs(&data, &shapes, None).expect_err("no SERVICE source");
    assert!(
        refused.contains("no remote query source configured"),
        "the refusal names the missing source: {refused}"
    );

    // A scope that installs governors but no source refuses identically: installing a
    // scope is not what makes a SERVICE answerable, a source is.
    let (sourceless, tripped) = validate_in_scope(
        &shapes,
        &data,
        &QueryGovernors::METERED,
        QuerySources::default(),
    );
    assert_eq!(tripped, None);
    let sourceless = sourceless.expect_err("a scope with no source has no SERVICE source");
    assert!(
        sourceless.contains("no remote query source configured"),
        "{sourceless}"
    );

    // Two registries, two answers: the report names exactly the person each bans.
    let (alice, tripped) = validate_in_scope(
        &shapes,
        &data,
        &QueryGovernors::METERED,
        sources_over(registry("alice")),
    );
    assert_eq!(tripped, None);
    let alice = alice.expect("the registry answers");
    assert!(!alice.conforms);
    assert_eq!(focus_nodes(&alice), vec![format!("<{EX}alice>")]);

    let (bob, _) = validate_in_scope(
        &shapes,
        &data,
        &QueryGovernors::METERED,
        sources_over(registry("bob")),
    );
    assert_eq!(
        focus_nodes(&bob.expect("the registry answers")),
        vec![format!("<{EX}bob>")]
    );

    // A registry that bans nobody here: the data conforms.
    let (nobody, _) = validate_in_scope(
        &shapes,
        &data,
        &QueryGovernors::METERED,
        sources_over(registry("dave")),
    );
    assert!(nobody.expect("the registry answers").conforms);

    // The scope is gone once dropped: the next validation on this thread has no source.
    assert!(validate_graphs(&data, &shapes, None).is_err());
    assert!(current_governors().is_none());
}

#[test]
fn service_silent_without_a_source_is_the_join_identity_inside_and_outside_the_scope() {
    // SILENT swallows the missing source to the join identity — one empty solution, which
    // joined with the pre-bound `$this` selects every focus node. The scope changes
    // nothing about that: SILENT's contract is the evaluator's, not the scope's.
    let shapes = service_shapes(true);
    let data = people_nt();
    let outside = validate_graphs(&data, &shapes, None).expect("SILENT swallows the failure");
    let (inside, _) = validate_in_scope(
        &shapes,
        &data,
        &QueryGovernors::METERED,
        QuerySources::default(),
    );
    let inside = inside.expect("SILENT swallows the failure");
    assert_eq!(focus_nodes(&inside), focus_nodes(&outside));
    assert_eq!(
        focus_nodes(&outside),
        vec![format!("<{EX}alice>"), format!("<{EX}bob>")]
    );
    // The answered neighbour differs: a registry that bans Bob selects Bob alone.
    let (answered, _) = validate_in_scope(
        &shapes,
        &data,
        &QueryGovernors::METERED,
        sources_over(registry("bob")),
    );
    assert_eq!(
        focus_nodes(&answered.expect("answered")),
        vec![format!("<{EX}bob>")]
    );
}

/// Core shapes only: a datatype constraint, evaluated from the IR with no SPARQL.
const CORE_SHAPES: &str = r"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
";

/// `count` people, every third with a malformed age.
fn aged_people(count: usize) -> String {
    let mut data = String::new();
    for index in 0..count {
        let age = if index % 3 == 0 {
            "\"old\"".to_owned()
        } else {
            format!("\"{index}\"^^<http://www.w3.org/2001/XMLSchema#integer>")
        };
        writeln!(
            data,
            "<{EX}p{index}> <{RDF_TYPE}> <{EX}Person> .\n<{EX}p{index}> <{EX}age> {age} ."
        )
        .expect("writing to a String cannot fail");
    }
    data
}

/// A stop signal that counts its polls and fires once it has been polled `fire_at`
/// times — latching, as the trait requires.
#[derive(Debug)]
struct CountingSignal {
    polls: AtomicU64,
    fire_at: u64,
}

impl StopSignal for CountingSignal {
    fn poll(&self) -> Option<StopCause> {
        let polled = self.polls.fetch_add(1, Ordering::Relaxed) + 1;
        (polled >= self.fire_at).then_some(StopCause::Cancelled)
    }
}

#[test]
fn the_stop_signal_is_polled_between_focus_nodes_of_a_validation_with_no_sparql() {
    const PEOPLE: usize = 60;
    let data = aged_people(PEOPLE);
    let baseline = validate_graphs(&data, CORE_SHAPES, None).expect("the ungoverned run");
    assert_eq!(
        baseline.results.len(),
        PEOPLE / 3,
        "one violation per malformed age"
    );

    // A signal that never fires: the governed run reaches the ungoverned report, and the
    // signal was polled at least once per focus node although no query ran.
    let quiet = Arc::new(CountingSignal {
        polls: AtomicU64::new(0),
        fire_at: u64::MAX,
    });
    let (report, tripped) = validate_in_scope(
        CORE_SHAPES,
        &data,
        &QueryGovernors::METERED.with_stop_signal(Arc::clone(&quiet) as Arc<dyn StopSignal>),
        QuerySources::default(),
    );
    assert_eq!(tripped, None);
    assert_eq!(
        format!("{:?}", report.expect("completes")),
        format!("{baseline:?}"),
        "a quiet signal changes nothing reported"
    );
    let polls = quiet.polls.load(Ordering::Relaxed);
    assert!(
        polls >= PEOPLE as u64,
        "the signal is polled between focus nodes: {polls} polls for {PEOPLE} focus nodes"
    );

    // A signal that fires partway: the validation stops with the typed trip, and no
    // report comes back.
    let firing = Arc::new(CountingSignal {
        polls: AtomicU64::new(0),
        fire_at: 10,
    });
    let (stopped, tripped) = validate_in_scope(
        CORE_SHAPES,
        &data,
        &QueryGovernors::METERED.with_stop_signal(Arc::clone(&firing) as Arc<dyn StopSignal>),
        QuerySources::default(),
    );
    assert_eq!(
        tripped,
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
    assert!(stopped.is_err(), "a stopped validation has no report");
    assert!(
        firing.polls.load(Ordering::Relaxed) < polls,
        "it stopped rather than running to the end"
    );

    // An already-cancelled flag stops it before the first focus node is validated.
    let cancelled = CancellationFlag::new();
    cancelled.cancel();
    let (_, tripped) = validate_in_scope(
        CORE_SHAPES,
        &data,
        &QueryGovernors::METERED.with_stop_signal(Arc::new(cancelled)),
        QuerySources::default(),
    );
    assert_eq!(
        tripped,
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
}

#[test]
fn rule_application_polls_the_stop_signal_between_focus_nodes() {
    let shapes = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:PersonRule a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .
";
    let data = aged_people(30);
    let baseline = purrdf_shapes::engine::entail_graphs(&data, shapes, None).expect("entails");
    let state = |signal: Arc<dyn StopSignal>| {
        Arc::new(GovernorState::new(
            &QueryGovernors::METERED.with_stop_signal(signal),
        ))
    };

    let quiet = state(Arc::new(CancellationFlag::new()));
    let entailed = {
        let _scope = enter_execution_scope(Arc::clone(&quiet), QuerySources::default());
        purrdf_shapes::engine::entail_graphs(&data, shapes, None).expect("entails")
    };
    assert_eq!(quiet.tripped(), None);
    assert_eq!(
        purrdf::canonical_flat_nquads(entailed.as_ref()).expect("canonical"),
        purrdf::canonical_flat_nquads(baseline.as_ref()).expect("canonical"),
        "a quiet signal changes nothing entailed"
    );

    let cancelled = CancellationFlag::new();
    cancelled.cancel();
    let stopped = state(Arc::new(cancelled));
    let result = {
        let _scope = enter_execution_scope(Arc::clone(&stopped), QuerySources::default());
        purrdf_shapes::engine::entail_graphs(&data, shapes, None)
    };
    assert!(result.is_err(), "a stopped rule application has no result");
    assert_eq!(
        stopped.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        })
    );
}

#[test]
fn replacing_the_ambient_context_takes_every_scope_off_the_thread_and_puts_it_back() {
    assert!(current_governors().is_none());
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let shapes = service_shapes(false);
    let data = people_nt();
    let _scope = enter_execution_scope(Arc::clone(&state), sources_over(registry("alice")));

    // Parked: the thread is idle, and a validation run now is the ungoverned one with no
    // source — it cannot see the parked scope's governors or its registry.
    let parked = replace_ambient_context(AmbientContext::default());
    assert!(
        !parked.is_idle(),
        "the parked context carries the installed scope"
    );
    assert!(current_governors().is_none());
    let meanwhile = validate_graphs(&data, &shapes, None).expect_err("no source while parked");
    assert!(
        meanwhile.contains("no remote query source configured"),
        "{meanwhile}"
    );

    // Restored: the scope's governors and registry are back, and answer.
    let idle = replace_ambient_context(parked);
    assert!(idle.is_idle(), "what ran while parked left nothing behind");
    assert!(
        current_governors().is_some_and(|installed| Arc::ptr_eq(&installed, &state)),
        "the restored governors are the scope's own"
    );
    let report = validate_graphs(&data, &shapes, None).expect("the registry answers again");
    assert_eq!(focus_nodes(&report), vec![format!("<{EX}alice>")]);
}
