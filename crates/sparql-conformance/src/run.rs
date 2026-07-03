// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Running a discovered case: load data, parse + evaluate the query.

use std::sync::Arc;

use purrdf::{serialize_dataset, SerializeGraph};
use purrdf_core::{RdfDataset, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::{
    NativeSparqlEngine, ParserOptions, RemoteQuerySource, StandpointPredicates,
};

use crate::manifest::{SparqlTestCase, TestKind};

const BASE: &str = "http://purrdf.test/manifest/";

/// The extension-function namespace the first-party suite fixtures spell their
/// calls under. PurRDF itself mints no vocabulary — the namespace is HARNESS
/// configuration (a neutral example.org name), exactly as a real deployment
/// supplies its own ontology namespace.
const EXT_NS: &str = "https://example.org/ext/";

/// The outcome of running a case (before comparison against the expected result).
#[derive(Debug)]
pub enum RunOutcome {
    /// A `QueryEvaluationTest` result.
    Eval(SparqlResult),
    /// An `UpdateEvaluationTest` post-state: the dataset after applying the update.
    Update(Arc<RdfDataset>),
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
    let ds = build_dataset(&case.data, &case.graph_data)?;
    // For an entailment test, materialize the regime's closure into the dataset
    // before it is frozen and queried (forward-materialization; the eval loop is
    // untouched — it queries an already-reasoned dataset).
    match case.regime {
        Some(regime) => purrdf_entail::materialize(&ds, regime)
            .map_err(|e| format!("entailment ({regime:?}) for {}: {e}", case.iri)),
        None => Ok(ds),
    }
}

/// Build a dataset from default-graph Turtle files (`data`) and named-graph files
/// (`graph_data`, each `(graph IRI, file)`). Shared by the query pre-state loader
/// and the UPDATE pre-/post-state builders.
///
/// # Errors
///
/// Returns a message on any read, parse, or serialize failure (never silent).
pub fn build_dataset(
    data: &[std::path::PathBuf],
    graph_data: &[(String, std::path::PathBuf)],
) -> Result<Arc<RdfDataset>, String> {
    // Serialize each qt:data Turtle file to N-Quads (default graph — no graph tag).
    let mut combined_nq: Vec<u8> = Vec::new();
    for data in data {
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
    for (graph_iri, path) in graph_data {
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
        .map_err(|e| format!("parse combined n-quads: {e}"))
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
            // Both the extension-function namespace and the standpoint predicate
            // table are CALLER configuration (the engine has no defaults): the
            // purrdf-extend suite's standpoint cases exercise `ext:heldIn` and the
            // purrdf-list-functions suite the `ext:list*` functions, all spelled
            // under the harness-configured example.org/ext/ namespace, against
            // fixture data written in the same namespace — so the harness supplies
            // that namespace plus its accordingTo/sharpens table here. (A gmeow
            // deployment would supply its own gmeow IRIs instead — everything
            // flows through configuration, not constants.) Harmless for the W3C
            // suites, which never call the extension functions.
            let engine = NativeSparqlEngine::new()
                .with_parser_options(ParserOptions {
                    extension_fn_namespaces: vec![EXT_NS.to_owned()],
                })
                .with_standpoint_predicates(StandpointPredicates::new(
                    format!("{EXT_NS}accordingTo"),
                    format!("{EXT_NS}sharpens"),
                ));
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
        TestKind::UpdateEval => {
            // Apply the `ut:request` update to the pre-state dataset; the mutated
            // dataset is diffed against the expected post-state in `compare`.
            let mut dataset = build_dataset(&case.data, &case.graph_data)?;
            let engine = NativeSparqlEngine::new().with_parser_options(ParserOptions {
                extension_fn_namespaces: vec![EXT_NS.to_owned()],
            });
            let request = SparqlRequest {
                query: &query_text,
                base_iri: Some(BASE),
                substitutions: &[],
            };
            engine
                .update(&mut dataset, request)
                .map_err(|e| format!("apply update {}: {e}", case.iri))?;
            Ok(RunOutcome::Update(dataset))
        }
        TestKind::Unknown => Err(format!("unmodeled test type for {}", case.iri)),
    }
}
