// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! HTTP-shaped `SERVICE` federation tests.
//!
//! The core crate owns request construction and SPARQL Results decoding; the
//! actual HTTP exchange is injected by the host runtime so the evaluator remains
//! wasm-portable.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{RdfDatasetBuilder, SparqlRequest, SparqlResult, StopCause, TrippedGovernor};
use purrdf_sparql_algebra::Variable;
use purrdf_sparql_eval::{
    CancellationFlag, HttpRemoteQuerySource, HttpRequest, NativeSparqlEngine, PartialAnswers,
    QueryGovernors, RemoteError, RemoteQuerySource, StopSignal,
};

const ENDPOINT: &str = "https://query.example/sparql";
const RESULT_JSON: &[u8] = br#"{
  "head": { "vars": ["x"] },
  "results": {
    "bindings": [
      {
        "x": {
          "type": "literal",
          "value": "1",
          "datatype": "http://www.w3.org/2001/XMLSchema#integer"
        }
      }
    ]
  }
}"#;

fn fixture_transport(request: HttpRequest<'_>) -> Result<Vec<u8>, RemoteError> {
    assert_eq!(request.endpoint, ENDPOINT);
    assert!(request.query_text.contains("SELECT"));
    assert_eq!(request.content_type, "application/sparql-query");
    assert_eq!(request.accept, "application/sparql-results+json");
    assert!(request.user_agent.contains("purrdf-sparql-eval"));
    Ok(RESULT_JSON.to_vec())
}

#[test]
fn http_transport_decodes_remote_bindings() {
    let source = HttpRemoteQuerySource::new(fixture_transport);
    let resolved = source
        .query(ENDPOINT, "SELECT ?x WHERE { BIND(1 AS ?x) }", None, None)
        .expect("injected transport");
    assert_eq!(resolved.variables, vec![Variable::new("x")]);
    assert_eq!(resolved.rows.len(), 1, "expected exactly one binding row");
    assert!(resolved.rows[0][0].is_some(), "?x must be bound");
}

#[test]
fn http_transport_cell_bound_decodes_no_limit_plus_one_row() {
    let source = HttpRemoteQuerySource::new(fixture_transport);
    let resolved = source
        .query(ENDPOINT, "SELECT ?x WHERE { BIND(1 AS ?x) }", None, Some(0))
        .expect("the bounded response is a typed prefix, not a decode error");

    assert_eq!(resolved.variables, vec![Variable::new("x")]);
    assert!(resolved.rows.is_empty(), "zero cells admit no one-cell row");
    assert_eq!(
        resolved.cell_limit_exceeded_at,
        Some(1),
        "the decoder reports the first row it skipped without materializing it"
    );
}

#[test]
fn the_stop_signal_travels_with_the_request_and_gates_it() {
    // The transport counts the exchanges it is asked to perform, which is the only
    // observation that can tell "the request was prevented" apart from "the request was
    // made and its result discarded".
    let posts = AtomicUsize::new(0);
    let flag = CancellationFlag::new();
    let signal: Arc<dyn StopSignal> = Arc::new(flag.clone());
    let source = HttpRemoteQuerySource::new(|request: HttpRequest<'_>| {
        posts.fetch_add(1, Ordering::Relaxed);
        // A host transport can only abandon an in-flight exchange if the signal reached
        // it; the evaluator is blocked here for the whole duration of the call.
        assert!(
            request.stop.is_some(),
            "the executing query's stop signal must reach the transport"
        );
        assert!(request.timeout.as_secs() > 0, "the timeout is still data");
        Ok(RESULT_JSON.to_vec())
    });

    let query = "SELECT ?x WHERE { BIND(1 AS ?x) }";
    source
        .query(ENDPOINT, query, Some(&signal), None)
        .expect("an unfired signal does not gate the request");
    assert_eq!(posts.load(Ordering::Relaxed), 1);

    // Once the signal has fired the request is not issued at all — a governor that could
    // only be observed after the exchange returned would not bound the exchange.
    flag.cancel();
    let err = source
        .query(ENDPOINT, query, Some(&signal), None)
        .expect_err("a fired signal refuses the request");
    assert_eq!(
        err,
        RemoteError::Governed(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }),
        "a governor is reported as a governor, never as a transport failure that SILENT \
         would be entitled to swallow"
    );
    assert_eq!(
        posts.load(Ordering::Relaxed),
        1,
        "no second exchange was performed"
    );
}

#[test]
fn service_clause_federates_through_injected_http_transport() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("http://ex/p");
    let s = b.intern_iri("http://ex/s");
    let o = b.intern_iri("http://ex/o");
    b.push_quad(s, p, o, None);
    let dataset = b.freeze().expect("freeze");

    let query = "SELECT ?x WHERE { \
                 <http://ex/s> <http://ex/p> ?o \
                 SERVICE <https://query.example/sparql> { BIND(1 AS ?x) } }";
    let engine = NativeSparqlEngine::new();
    let source = HttpRemoteQuerySource::new(fixture_transport);
    let result = engine
        .query_with_source(
            &dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            &source,
        )
        .expect("federated query");
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            assert!(variables.contains(&"x".to_owned()));
            assert_eq!(rows.len(), 1, "the SERVICE bag joins the single local row");
        }
        other => panic!("expected solutions, got {other:?}"),
    }
}

#[test]
fn missing_named_graph_does_not_execute_its_inner_service() {
    let posts = AtomicUsize::new(0);
    let source = HttpRemoteQuerySource::new(|_request: HttpRequest<'_>| {
        posts.fetch_add(1, Ordering::Relaxed);
        Err(RemoteError::Transport(
            "a known-empty GRAPH branch must not issue this request".to_owned(),
        ))
    });
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("freeze empty dataset");
    let result = NativeSparqlEngine::new()
        .query_with_source(
            &dataset,
            SparqlRequest {
                query: "SELECT ?x WHERE { GRAPH <https://example.org/missing> { \
                        SERVICE <https://query.example/sparql> { BIND(1 AS ?x) } } }",
                base_iri: None,
                substitutions: &[],
            },
            &source,
        )
        .expect("a missing named graph is an empty result, not a SERVICE evaluation");
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("SELECT must return a solution sequence");
    };
    assert_eq!(variables, ["x"]);
    assert!(rows.is_empty());
    assert_eq!(posts.load(Ordering::Relaxed), 0);
}

fn terminal_service_cancelled_during_post(silent: bool) {
    let posts = Arc::new(AtomicUsize::new(0));
    let flag = CancellationFlag::new();
    let transport_flag = flag.clone();
    let transport_posts = Arc::clone(&posts);
    let source = HttpRemoteQuerySource::new(move |request: HttpRequest<'_>| {
        transport_posts.fetch_add(1, Ordering::Relaxed);
        assert!(
            request.stop.is_some(),
            "the stop signal reached the transport"
        );
        // Deliberately deaf for the remainder of this exchange: the request completes
        // after firing the signal instead of returning `RemoteError::Governed` itself.
        transport_flag.cancel();
        Ok(RESULT_JSON.to_vec())
    });
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("freeze empty dataset");
    let silent = if silent { "SILENT " } else { "" };
    let query = format!(
        "SELECT ?x WHERE {{ SERVICE {silent}<https://query.example/sparql> {{ BIND(1 AS ?x) }} }}"
    );
    let outcome = NativeSparqlEngine::new()
        .query_governed_with_source(
            &dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            &source,
            &QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(flag)),
        )
        .expect("a stop is a typed outcome");

    assert_eq!(posts.load(Ordering::Relaxed), 1);
    let exhausted = outcome
        .exhausted()
        .expect("the terminal call cannot complete");
    assert_eq!(
        exhausted.tripped,
        TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }
    );
    let PartialAnswers::Certain(partial) = &exhausted.partial else {
        panic!("a discarded terminal response remains a lower bound")
    };
    assert!(
        !partial.is_positional_prefix(),
        "a completed-and-discarded response must withdraw resumption"
    );
}

#[test]
fn terminal_service_cannot_launder_a_stop_fired_during_a_deaf_exchange() {
    terminal_service_cancelled_during_post(false);
}

#[test]
fn terminal_service_silent_cannot_launder_a_stop_fired_during_a_deaf_exchange() {
    terminal_service_cancelled_during_post(true);
}
