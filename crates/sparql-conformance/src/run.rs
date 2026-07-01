// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Running a discovered case: load data, parse + evaluate the query.

use std::sync::Arc;

use purrdf::{serialize_dataset, SerializeGraph};
use purrdf_core::{RdfDataset, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{NativeSparqlEngine, RemoteQuerySource};

use crate::manifest::{SparqlTestCase, TestKind};

const BASE: &str = "http://purrdf.test/manifest/";

/// The outcome of running a case (before comparison against the expected result).
pub enum RunOutcome {
    /// A `QueryEvaluationTest` result.
    Eval(SparqlResult),
    /// A syntax test: did the query parse?
    Syntax { parsed_ok: bool },
}

/// Load the case's `qt:data` and `qt:graphData` files into a combined dataset.
///
/// Default-graph data (`qt:data` Turtle files) is merged into the default graph.
/// Named-graph data (`qt:graphData`) is placed in the named graph identified by
/// its file IRI: each triple from the file is tagged with the graph IRI so it
/// appears in the named graph when queried with `GRAPH <iri> { … }`.
///
/// Both scoping axes are supported: named-graph worlds (queried via `GRAPH ?world
/// { … }`) and the standpoint poset (queried via `purrdf:heldIn` over the default-
/// graph reification layer). The combined-world case proves both axes with a JOIN:
/// a named-graph world triple joined against a default-graph standpoint-held
/// reifier.
///
/// # Errors
///
/// Returns a message on any read, parse, or serialize failure (never silent).
pub fn load_dataset(case: &SparqlTestCase) -> Result<Arc<RdfDataset>, String> {
    // Serialize each qt:data Turtle file to N-Quads (default graph — no graph tag).
    let mut combined_nq: Vec<u8> = Vec::new();
    for data in &case.data {
        let chunk = std::fs::read(data).map_err(|e| format!("read {}: {e}", data.display()))?;
        let ds = purrdf::parse_dataset(&chunk, "text/turtle", Some(BASE))
            .map_err(|e| format!("parse data {}: {e}", data.display()))?;
        let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
            .map_err(|e| format!("serialize {}: {e}", data.display()))?;
        combined_nq.extend_from_slice(&nq);
        if combined_nq.last() != Some(&b'\n') {
            combined_nq.push(b'\n');
        }
    }

    // Serialize each qt:graphData Turtle file to N-Quads, then tag every triple line
    // with the named-graph IRI so it is placed in that named graph.
    for (graph_iri, path) in &case.graph_data {
        let chunk = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let ds = purrdf::parse_dataset(&chunk, "text/turtle", Some(BASE))
            .map_err(|e| format!("parse graph data {}: {e}", path.display()))?;
        let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
            .map_err(|e| format!("serialize graph data {}: {e}", path.display()))?;
        let nq_text = std::str::from_utf8(&nq)
            .map_err(|e| format!("utf-8 in serialized nquads for {}: {e}", path.display()))?;

        // Tag each triple line (lines ending with ` .`) with the named-graph IRI.
        // Comment lines and blank lines are passed through unchanged.
        // Lines that already carry a graph term (four-element quads) are also passed through.
        for line in nq_text.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                combined_nq.extend_from_slice(trimmed.as_bytes());
            } else if let Some(body) = trimmed.strip_suffix(" .") {
                // Strip the trailing ` .`, insert the graph IRI, re-append ` .`
                combined_nq.extend_from_slice(body.as_bytes());
                combined_nq.extend_from_slice(b" <");
                combined_nq.extend_from_slice(graph_iri.as_bytes());
                combined_nq.extend_from_slice(b"> .");
            } else {
                combined_nq.extend_from_slice(trimmed.as_bytes());
            }
            combined_nq.push(b'\n');
        }
    }

    purrdf::parse_dataset(&combined_nq, "application/n-quads", Some(BASE))
        .map_err(|e| format!("parse combined n-quads for {}: {e}", case.iri))
}

/// Run `case`, optionally resolving `SERVICE` clauses through `remote`.
///
/// # Errors
///
/// Returns a message on a read/parse/evaluation failure (the harness decides
/// whether that is an expected failure).
pub fn run(
    case: &SparqlTestCase,
    remote: Option<&dyn RemoteQuerySource>,
) -> Result<RunOutcome, String> {
    let query_text = std::fs::read_to_string(&case.query)
        .map_err(|e| format!("read query {}: {e}", case.query.display()))?;

    match case.kind {
        TestKind::PositiveSyntax | TestKind::NegativeSyntax => {
            let parsed_ok = purrdf_sparql_algebra::SparqlParser::new()
                .parse_query(&query_text)
                .is_ok();
            Ok(RunOutcome::Syntax { parsed_ok })
        }
        TestKind::QueryEval => {
            let dataset = load_dataset(case)?;
            let engine = NativeSparqlEngine::new();
            let request = SparqlRequest {
                query: &query_text,
                base_iri: Some(BASE),
                substitutions: &[],
            };
            let result = match remote {
                Some(source) => engine.query_with_source(&dataset, request, source),
                None => engine.query(&dataset, request),
            }
            .map_err(|e| format!("evaluate {}: {e}", case.iri))?;
            Ok(RunOutcome::Eval(result))
        }
        TestKind::Unknown => Err(format!("unmodeled test type for {}", case.iri)),
    }
}
