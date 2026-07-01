// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! W3C `mf:` test-manifest parsing.
//!
//! A manifest `manifest.ttl` is itself an RDF graph: it is loaded with the native
//! Turtle codec and **queried with the native engine** (dog-fooding) to extract
//! its `mf:entries` list of test cases. File references in the manifest are
//! relative IRIs; they are parsed against a sentinel base and mapped back to
//! local paths under the manifest's directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::NativeSparqlEngine;

use crate::paths;

/// The sentinel base IRI the manifest is parsed against, so a relative file
/// reference `<agg01.rq>` resolves to `<BASE>agg01.rq` and the local name is
/// recoverable by stripping the prefix.
const BASE: &str = "http://purrdf.test/manifest/";

const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";

/// The kind of a discovered SPARQL test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    /// `mf:QueryEvaluationTest` (and result-format variants): run the query and
    /// diff the result.
    QueryEval,
    /// `mf:PositiveSyntaxTest(11)`: the query must parse.
    PositiveSyntax,
    /// `mf:NegativeSyntaxTest(11)`: the query must fail to parse.
    NegativeSyntax,
    /// A manifest entry whose `rdf:type` the harness does not model — recorded and
    /// surfaced (never silently skipped).
    Unknown,
}

/// What a [`TestKind::QueryEval`] case expects.
#[derive(Debug, Clone)]
pub enum ExpectedResult {
    /// SPARQL Results XML.
    Srx(PathBuf),
    /// SPARQL Results JSON.
    Srj(PathBuf),
    /// A graph (`CONSTRUCT`/`DESCRIBE`) — compared as canonical N-Quads.
    Graph(PathBuf),
    /// Syntax tests carry no result.
    None,
    /// A result file whose extension the harness does not model.
    Unsupported(PathBuf),
}

/// One discovered conformance test case.
#[derive(Debug, Clone)]
pub struct SparqlTestCase {
    /// The full test IRI (used for diagnostics and xfail matching).
    pub iri: String,
    /// The human-readable `mf:name`.
    pub name: String,
    /// The test kind.
    pub kind: TestKind,
    /// The query file (`.rq`).
    pub query: PathBuf,
    /// The default-graph data file(s) (`qt:data`).
    pub data: Vec<PathBuf>,
    /// Named-graph data files (`qt:graphData`); the graph IRI is the file IRI.
    pub graph_data: Vec<(String, PathBuf)>,
    /// `SERVICE` endpoint data: `(endpoint IRI, local file)` (`qt:serviceData`).
    pub service_data: Vec<(String, PathBuf)>,
    /// The expected result.
    pub expected: ExpectedResult,
}

/// Load and parse all cases declared by `manifest_path`.
///
/// # Errors
///
/// Returns a message on a read/parse failure or a malformed manifest.
pub fn load(manifest_path: &Path) -> Result<Vec<SparqlTestCase>, String> {
    let bytes = std::fs::read(manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let dataset = purrdf::parse_dataset(&bytes, "text/turtle", Some(BASE))
        .map_err(|e| format!("parse manifest {}: {e}", manifest_path.display()))?;

    let dir = paths::manifest_dir(manifest_path);

    // One row per (test × data × graphData × serviceData × result) combination;
    // grouped by ?test below. Property paths walk the rdf:List of entries.
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX qt: <http://www.w3.org/2001/sw/DataAccess/tests/test-query#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         SELECT ?test ?type ?name ?act ?query ?data ?graphData ?serviceEp ?serviceData ?result WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         ?test rdf:type ?type ; mf:name ?name ; mf:action ?act .\n\
         OPTIONAL {{ ?act qt:query ?query }}\n\
         OPTIONAL {{ ?act qt:data ?data }}\n\
         OPTIONAL {{ ?act qt:graphData ?graphData }}\n\
         OPTIONAL {{ ?act qt:serviceData ?sd . ?sd qt:endpoint ?serviceEp ; qt:data ?serviceData }}\n\
         OPTIONAL {{ ?test mf:result ?result }}\n\
         }}"
    );

    let rows = query_rows(&dataset, &query)?;

    // Group rows by ?test IRI, accumulating the multi-valued columns.
    let mut by_test: BTreeMap<String, SparqlTestCase> = BTreeMap::new();
    for row in &rows {
        let test_iri = iri_of(row, "test").ok_or("manifest row without ?test IRI")?;
        let kind = classify(row.get("type"));
        let entry = by_test
            .entry(test_iri.clone())
            .or_insert_with(|| SparqlTestCase {
                iri: test_iri.clone(),
                name: lexical_of(row, "name").unwrap_or_default(),
                kind,
                query: PathBuf::new(),
                data: Vec::new(),
                graph_data: Vec::new(),
                service_data: Vec::new(),
                expected: ExpectedResult::None,
            });
        // A test may carry several rdf:type values; prefer a recognized kind.
        if entry.kind == TestKind::Unknown && kind != TestKind::Unknown {
            entry.kind = kind;
        }

        // The query file: qt:query for eval tests, else mf:action itself (syntax).
        if let Some(q) = iri_of(row, "query") {
            entry.query = local_path(&dir, &q);
        } else if entry.query.as_os_str().is_empty() {
            if let Some(act) = iri_of(row, "act") {
                entry.query = local_path(&dir, &act);
            }
        }
        push_unique_path(
            &mut entry.data,
            iri_of(row, "data").map(|i| local_path(&dir, &i)),
        );
        if let Some(gd) = iri_of(row, "graphData") {
            let path = local_path(&dir, &gd);
            if !entry.graph_data.iter().any(|(_, p)| *p == path) {
                entry.graph_data.push((gd, path));
            }
        }
        if let (Some(ep), Some(sd)) = (iri_of(row, "serviceEp"), iri_of(row, "serviceData")) {
            let path = local_path(&dir, &sd);
            if !entry.service_data.iter().any(|(e, _)| *e == ep) {
                entry.service_data.push((ep, path));
            }
        }
        if let Some(result) = iri_of(row, "result") {
            entry.expected = classify_result(&local_path(&dir, &result));
        }
    }

    Ok(by_test.into_values().collect())
}

/// Run `query` against `dataset` and return its solution rows as variable→value
/// maps (unbound cells omitted).
fn query_rows(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    query: &str,
) -> Result<Vec<BTreeMap<String, TermValue>>, String> {
    let engine = NativeSparqlEngine::new();
    let result = engine
        .query(
            dataset,
            SparqlRequest {
                query,
                base_iri: Some(BASE),
                substitutions: &[],
            },
        )
        .map_err(|e| format!("manifest query failed: {e}"))?;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => Ok(rows
            .into_iter()
            .map(|row| {
                variables
                    .iter()
                    .zip(row)
                    .filter_map(|(v, cell)| cell.map(|t| (v.clone(), t)))
                    .collect()
            })
            .collect()),
        other => Err(format!(
            "manifest query did not return solutions: {other:?}"
        )),
    }
}

/// The IRI string of a bound variable, if it is an IRI.
fn iri_of(row: &BTreeMap<String, TermValue>, var: &str) -> Option<String> {
    match row.get(var) {
        Some(TermValue::Iri(i)) => Some(i.clone()),
        _ => None,
    }
}

/// The lexical form of a bound literal variable.
fn lexical_of(row: &BTreeMap<String, TermValue>, var: &str) -> Option<String> {
    match row.get(var) {
        Some(TermValue::Literal { lexical_form, .. }) => Some(lexical_form.clone()),
        _ => None,
    }
}

/// Map a sentinel-based file IRI back to a local path under the manifest dir.
fn local_path(dir: &Path, iri: &str) -> PathBuf {
    let relative = iri.strip_prefix(BASE).unwrap_or(iri);
    paths::resolve(dir, relative)
}

/// Push `path` into `dst` if present and not already there.
fn push_unique_path(dst: &mut Vec<PathBuf>, path: Option<PathBuf>) {
    if let Some(p) = path {
        if !dst.contains(&p) {
            dst.push(p);
        }
    }
}

/// Classify a manifest entry's `rdf:type` IRI into a [`TestKind`].
fn classify(type_term: Option<&TermValue>) -> TestKind {
    let Some(TermValue::Iri(t)) = type_term else {
        return TestKind::Unknown;
    };
    let local = t.strip_prefix(MF).unwrap_or(t);
    match local {
        "QueryEvaluationTest" | "CSVResultFormatTest" => TestKind::QueryEval,
        "PositiveSyntaxTest" | "PositiveSyntaxTest11" => TestKind::PositiveSyntax,
        "NegativeSyntaxTest" | "NegativeSyntaxTest11" => TestKind::NegativeSyntax,
        _ => TestKind::Unknown,
    }
}

/// Classify a result file by extension.
fn classify_result(path: &Path) -> ExpectedResult {
    match path.extension().and_then(|e| e.to_str()) {
        Some("srx") => ExpectedResult::Srx(path.to_path_buf()),
        Some("srj") => ExpectedResult::Srj(path.to_path_buf()),
        Some("ttl" | "nt" | "nq" | "rdf") => ExpectedResult::Graph(path.to_path_buf()),
        _ => ExpectedResult::Unsupported(path.to_path_buf()),
    }
}
