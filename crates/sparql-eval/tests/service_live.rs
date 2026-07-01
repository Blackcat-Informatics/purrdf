// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! HTTP-shaped `SERVICE` federation tests.
//!
//! The core crate owns request construction and SPARQL Results decoding; the
//! actual HTTP exchange is injected by the host runtime so the evaluator remains
//! wasm-portable.

use purrdf_core::{RdfDatasetBuilder, SparqlRequest, SparqlResult};
use purrdf_sparql_algebra::Variable;
use purrdf_sparql_eval::{
    HttpRemoteQuerySource, HttpRequest, NativeSparqlEngine, RemoteError, RemoteQuerySource,
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
        .query(ENDPOINT, "SELECT ?x WHERE { BIND(1 AS ?x) }")
        .expect("injected transport");
    assert_eq!(resolved.variables, vec![Variable::new("x")]);
    assert_eq!(resolved.rows.len(), 1, "expected exactly one binding row");
    assert!(resolved.rows[0][0].is_some(), "?x must be bound");
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
