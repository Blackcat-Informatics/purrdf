// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A generic consumer reads real fallible-query, graph-publication and protocol
//! refusals field by field, without matching their enums or formatting them.

mod support;

use std::sync::Arc;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, TrippedGovernor};
use purrdf_hash::error::ErrorValue;
use purrdf_sparql_eval::protocol::ProtocolRequest;
use purrdf_sparql_eval::{
    FallibleSparqlError, GovernorState, NativeSparqlEngine, QueryGovernors, QueryOptions,
};
use purrdf_testkit::error_capture::{CallerRefusal, Capture, Event};
use support::local_dataset;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ALL_ROWS: &str = "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o }";

fn edges() -> Arc<RdfDataset> {
    local_dataset([
        ("s0", "p", "o0"),
        ("s1", "p", "o1"),
        ("s2", "p", "o2"),
        ("s3", "p", "o3"),
    ])
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

/// INVARIANT: a query refused over a view that stayed ready reports its
/// diagnostic as a nested cause with a typed code, beside the view's receipt.
#[test]
fn a_refused_query_keeps_its_diagnostic_code_and_view_receipt() {
    let data = edges();
    let error = NativeSparqlEngine::new()
        .query_fallible_view(
            data.as_ref(),
            request("SELECT WHERE {"),
            &QueryOptions::EMPTY,
        )
        .expect_err("invalid SPARQL must fail parsing");
    let capture = Capture::read(&error);
    assert_eq!(capture.code, "sparql-fallible-query");
    let retained = capture.child("cause");
    assert_eq!(retained.code, "sparql-retained-diagnostic");
    let diagnostic = retained.child("cause");
    assert_eq!(diagnostic.code, "rdf-diagnostic");
    assert!(matches!(
        diagnostic.field("code"),
        ErrorValue::Code("native-sparql-query-parse")
    ));
    assert_eq!(capture.child("evidence").code, "unit");
}

/// INVARIANT: a governor trip carries the exact ceiling and consumption, the
/// certified partial rows with their positional verdict, and both receipts;
/// none of them is flattened into a message.
#[test]
fn an_exhausted_budget_keeps_its_trip_certified_rows_and_both_receipts() {
    let data = edges();
    let error = NativeSparqlEngine::new()
        .query_governed_fallible_view(
            data.as_ref(),
            request(ALL_ROWS),
            &QueryOptions::EMPTY,
            &QueryGovernors::UNBOUNDED.with_max_answers(2),
        )
        .expect_err("a trip is not a complete result");
    let Some(TrippedGovernor::Budget {
        limit, consumed, ..
    }) = error.tripped()
    else {
        panic!("the answer cap is a budget trip: {error:?}")
    };
    let certified = error
        .partial_answers()
        .and_then(|partial| partial.result())
        .expect("the admitted prefix is certified");

    let capture = Capture::read(&error);
    assert_eq!(capture.code, "sparql-fallible-budget-exhausted");
    let tripped = capture.child("tripped");
    assert_eq!(tripped.code, "governor-budget");
    assert!(matches!(
        tripped.field("limit"),
        ErrorValue::Unsigned(value) if value == u128::from(limit)
    ));
    assert!(matches!(
        tripped.field("consumed"),
        ErrorValue::Unsigned(value) if value == u128::from(consumed)
    ));

    let partial = capture.child("partial");
    assert_eq!(partial.code, "sparql-partial-certain");
    let rows = partial.child("result");
    assert_eq!(rows.code, "sparql-retained-partial-result");
    assert!(matches!(
        rows.field("positional_prefix"),
        ErrorValue::Boolean(value) if value == certified.is_positional_prefix()
    ));
    let retained = rows.child("result");
    assert_eq!(retained.code, "sparql-retained-result");
    let solutions = retained.child("result");
    assert_eq!(solutions.code, "sparql-solutions");
    assert!(
        solutions
            .events
            .iter()
            .any(|event| matches!(event, Event::BeginList(Some("rows"), 2))),
        "the two admitted rows are a list of two, not a count in prose"
    );

    let evidence = capture.child("evidence");
    assert_eq!(evidence.code, "sparql-governed-evidence");
    assert_eq!(
        evidence.child("coordinates").code,
        "query-coordinates-started"
    );
    assert_eq!(evidence.child("view").code, "unit");
    let governors = evidence.child("governors");
    assert_eq!(governors.code, "governor-evidence");
    assert_eq!(governors.child("tripped").code, "governor-budget");
}

/// INVARIANT: inspecting an operational failure borrows the view's own error.
/// A caller-defined cause whose `Display` and `Debug` panic is never formatted,
/// keeps its full-width field, and inspection allocates nothing.
#[test]
fn a_caller_operational_cause_is_borrowed_and_never_formatted() {
    let error = FallibleSparqlError::Operational {
        error: CallerRefusal { limit: u128::MAX },
        evidence: (),
    };
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let capture = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(capture.code, "sparql-fallible-operational");
    let cause = capture.child("cause");
    assert_eq!(cause.code, "caller-refused");
    assert!(matches!(
        cause.field("limit"),
        ErrorValue::Unsigned(u128::MAX)
    ));
    assert_eq!(capture.child("evidence").code, "unit");
    let source = std::error::Error::source(&error).expect("the view error is the source");
    assert_eq!(
        source
            .downcast_ref::<CallerRefusal>()
            .map(|cause| cause.limit),
        Some(u128::MAX)
    );
}

/// INVARIANT: a graph publication stopped by its operation governor reports the
/// trip and the operation's accumulated evidence as typed records.
#[test]
fn a_stopped_graph_publication_keeps_its_trip_and_operation_evidence() {
    let engine = NativeSparqlEngine::new();
    let data = edges();
    let plan = engine
        .prepare_query(
            "CONSTRUCT { ?s <http://example.org/p> ?o } WHERE { ?s <http://example.org/p> ?o }",
            None,
        )
        .expect("plan");
    let state = Arc::new(GovernorState::new(
        &QueryGovernors::UNBOUNDED.with_max_answers(0),
    ));
    let mut target = RdfDatasetBuilder::new();
    let error = engine
        .construct_prepared_in_operation_into_view(
            data.as_ref(),
            &plan,
            &[],
            &QueryOptions::EMPTY,
            &state,
            &mut target,
        )
        .expect_err("an answer cap of zero publishes nothing");
    let capture = Capture::read(&error);
    assert_eq!(capture.code, "sparql-graph-build-budget-exhausted");
    let tripped = capture.child("tripped");
    assert_eq!(tripped.code, "governor-budget");
    assert!(matches!(tripped.field("limit"), ErrorValue::Unsigned(0)));
    let evidence = capture.child("evidence");
    assert_eq!(evidence.code, "governor-evidence");
    assert_eq!(evidence.child("tripped").code, "governor-budget");
}

/// INVARIANT: a protocol refusal reports byte offsets as byte counts, parameter
/// and clause names as codes, the request's own text as text and the operation
/// index as an integer, each from a request the parser really refused.
#[test]
fn protocol_refusals_keep_offsets_parameters_and_operation_positions() {
    let body = ProtocolRequest::parse(
        "POST",
        Some("application/sparql-query"),
        None,
        b"ASK { ?s ?p \"\xff\" }",
    )
    .expect_err("the body is not UTF-8");
    let capture = Capture::read(&body);
    assert_eq!(capture.code, "sparql-protocol-non-utf8-body");
    assert!(matches!(
        capture.field("valid_up_to"),
        ErrorValue::ByteCount(13)
    ));

    let relative = ProtocolRequest::parse(
        "GET",
        None,
        Some("query=ASK+%7B%7D&default-graph-uri=g1"),
        b"",
    )
    .expect_err("a dataset parameter must be an absolute IRI");
    let capture = Capture::read(&relative);
    assert_eq!(capture.code, "sparql-protocol-invalid-graph-iri");
    assert!(matches!(
        capture.field("parameter"),
        ErrorValue::Code("default-graph-uri")
    ));
    assert!(matches!(capture.field("value"), ErrorValue::Text("g1")));
    assert!(matches!(capture.field("reason"), ErrorValue::Text(_)));

    let conflict = ProtocolRequest::parse(
        "POST",
        Some("application/x-www-form-urlencoded"),
        None,
        b"update=WITH+%3Chttp%3A%2F%2Fexample.org%2Fg2%3E+DELETE+%7B+%3Fs+%3Fp+%3Fo+%7D+\
          WHERE+%7B+%3Fs+%3Fp+%3Fo+%7D&using-graph-uri=http%3A%2F%2Fexample.org%2Fg1",
    )
    .expect("the request itself is well formed")
    .effective_text()
    .expect_err("the protocol forbids the combination");
    let capture = Capture::read(&conflict);
    assert_eq!(capture.code, "sparql-protocol-using-conflicts-with-clause");
    assert!(matches!(
        capture.field("parameter"),
        ErrorValue::Code("using-graph-uri")
    ));
    assert!(matches!(
        capture.field("operation"),
        ErrorValue::Unsigned(0)
    ));
    assert!(matches!(capture.field("clause"), ErrorValue::Code("WITH")));

    let unparsed = ProtocolRequest::parse(
        "GET",
        None,
        Some("query=SELECT+WHERE&default-graph-uri=http%3A%2F%2Fexample.org%2Fg1"),
        b"",
    )
    .expect("the request itself is well formed")
    .effective_text()
    .expect_err("there is no position to write the dataset at");
    let capture = Capture::read(&unparsed);
    assert_eq!(capture.code, "sparql-protocol-malformed-operation");
    assert_eq!(capture.child("kind").code, "sparql-operation-query");
    assert!(matches!(capture.field("message"), ErrorValue::Text(_)));
}
