// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential planner-correctness test for the cost-based BGP planner.
//!
//! Runs every vendored W3C SPARQL query-evaluation case twice — once with the
//! production cost-based planner and once with the retired structural heuristic
//! forced — and asserts that the result multiset is identical. This proves that
//! reordering BGP patterns never changes semantics across quoted triples,
//! reifiers, and GRAPH scopes.

use std::path::{Path, PathBuf};

use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser};
use purrdf_sparql_conformance::manifest::{SparqlTestCase, TestKind};
use purrdf_sparql_eval::{EvalOptions, NativeSparqlEngine, ParserOptions, StandpointPredicates};

const BASE: &str = "http://purrdf.test/manifest/";
const EXT_NS: &str = "https://example.org/ext/";

/// Build an engine with the requested planner mode. Both engines share the same
/// parse-time configuration the conformance harness uses.
fn engine(cost: bool) -> NativeSparqlEngine {
    let options = EvalOptions {
        exists_memo: true,
        force_structural_bgp_order: !cost,
    };
    NativeSparqlEngine::new()
        .with_parser_options(ParserOptions {
            extension_fn_namespaces: vec![EXT_NS.to_owned()],
        })
        .with_standpoint_predicates(StandpointPredicates::new(
            format!("{EXT_NS}accordingTo"),
            format!("{EXT_NS}sharpens"),
        ))
        .with_eval_options(options)
}

/// Evaluate `case` with `cost` planner (`true`) or forced structural order
/// (`false`). Mirrors the conformance harness's evaluation path, including the
/// in-memory SERVICE source when the case is federated.
fn eval_case(case: &SparqlTestCase, cost: bool) -> Result<SparqlResult, String> {
    let dataset = purrdf_sparql_conformance::run::load_dataset(case)?;
    let query_text = std::fs::read_to_string(&case.query)
        .map_err(|e| format!("read query {}: {e}", case.query.display()))?;
    let request = SparqlRequest {
        query: &query_text,
        base_iri: Some(BASE),
        substitutions: &[],
    };
    let remote = purrdf_sparql_conformance::service::build(case)?;
    let result = match remote {
        Some(source) => engine(cost).query_with_source(&dataset, request, &source),
        None => engine(cost).query(&dataset, request),
    }
    .map_err(|e| format!("evaluate {}: {e}", case.iri))?;
    Ok(result)
}

/// Whether `query_text` is a `SELECT` with a top-level `ORDER BY`, so row order
/// is observable and must be compared as an ordered sequence.
fn query_is_top_level_ordered(query_text: &str) -> bool {
    let Ok(Query::Select { pattern, .. }) = SparqlParser::new().parse_query(query_text) else {
        return false;
    };
    let mut node = &pattern;
    loop {
        match node {
            GraphPattern::OrderBy { .. } => return true,
            GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => node = inner,
            _ => return false,
        }
    }
}

/// Recursively list every `manifest.ttl` under `suite/`.
fn discover_manifests(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(discover_manifests(&path));
            } else if path.file_name().and_then(|n| n.to_str()) == Some("manifest.ttl") {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn cost_and_structural_planner_produce_identical_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite");
    let manifests = discover_manifests(&root);
    assert!(
        manifests.len() >= 10,
        "suite inventory shrank: found only {} manifests",
        manifests.len()
    );

    let mut cases = 0usize;
    let mut skipped = 0usize;
    let mut mismatches: Vec<(String, String)> = Vec::new();

    for manifest in &manifests {
        let loaded = purrdf_sparql_conformance::manifest::load(manifest)
            .unwrap_or_else(|e| panic!("load {}: {e}", manifest.display()));
        for case in loaded {
            if !matches!(case.kind, TestKind::QueryEval) {
                continue;
            }
            cases += 1;
            let query_text = std::fs::read_to_string(&case.query)
                .unwrap_or_else(|e| panic!("read query {}: {e}", case.query.display()));
            let ordered = query_is_top_level_ordered(&query_text);

            let cost_result = match eval_case(&case, true) {
                Ok(r) => r,
                Err(msg) => {
                    // If both planners error, the case is not a planner-differential
                    // failure (e.g. an unsupported feature). Record a skip.
                    match eval_case(&case, false) {
                        Ok(_) => mismatches.push((
                            case.iri.clone(),
                            format!("cost planner errored while structural succeeded: {msg}"),
                        )),
                        Err(_) => skipped += 1,
                    }
                    continue;
                }
            };
            let structural_result = match eval_case(&case, false) {
                Ok(r) => r,
                Err(msg) => {
                    mismatches.push((
                        case.iri.clone(),
                        format!("structural planner errored while cost succeeded: {msg}"),
                    ));
                    continue;
                }
            };

            if let Err(msg) = purrdf_sparql_conformance::compare::compare_results(
                &cost_result,
                &structural_result,
                ordered,
            ) {
                mismatches.push((case.iri.clone(), msg));
            }
        }
    }

    println!(
        "differential planner corpus: {cases} query-eval cases, {skipped} skipped (both errored), {} mismatches",
        mismatches.len()
    );

    assert!(
        mismatches.is_empty(),
        "{} case(s) produced different results under cost vs structural BGP order:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .map(|(iri, msg)| format!("  {iri}\n    -> {msg}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        cases >= 100,
        "differential corpus shrank: only {cases} query-eval cases"
    );
}
