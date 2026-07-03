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

/// The SPARQL-1.1 update-test vocabulary (`ut:`). Update tests describe their
/// pre-state (`ut:data`/`ut:graphData`), the update request (`ut:request`), and
/// their expected post-state (an `mf:result` node carrying `ut:data`/
/// `ut:graphData`). A named graph is a blank node `[ ut:graph <file> ;
/// rdfs:label "graph-iri" ]` — the graph IRI is the `rdfs:label`, not the file.
const UT: &str = "http://www.w3.org/2009/sparql/tests/test-update#";

/// The RDF Schema namespace; `rdfs:label` carries the graph IRI of a
/// `ut:graphData` entry in an update test.
const RDFS_LABEL_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";

/// The SPARQL service-description namespace; `sd:entailmentRegime` on a query
/// test's action lists the entailment regimes under which its expected result
/// holds (an RDF list of `http://www.w3.org/ns/entailment/*` IRIs).
const SD_NS: &str = "http://www.w3.org/ns/sparql-service-description#";

/// The kind of a discovered SPARQL test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    /// `mf:QueryEvaluationTest` (and result-format variants): run the query and
    /// diff the result.
    QueryEval,
    /// `mf:UpdateEvaluationTest`: apply the `ut:request` update to the pre-state
    /// dataset and diff the resulting dataset against the expected post-state.
    UpdateEval,
    /// `mf:PositiveSyntaxTest(11)`: the query must parse.
    PositiveSyntax,
    /// `mf:NegativeSyntaxTest(11)`: the query must fail to parse.
    NegativeSyntax,
    /// `mf:PositiveUpdateSyntaxTest`: the UPDATE request must parse.
    PositiveUpdateSyntax,
    /// `mf:NegativeUpdateSyntaxTest`: the UPDATE request must fail to parse.
    NegativeUpdateSyntax,
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
    /// An UPDATE post-state: the expected default-graph data (`ut:data`) and
    /// named graphs (`ut:graphData`), compared to the mutated dataset as
    /// canonical N-Quads. Empty vectors denote an empty expected dataset.
    DatasetState {
        /// Expected default-graph Turtle files.
        data: Vec<PathBuf>,
        /// Expected named graphs as `(graph IRI, file)`.
        graph_data: Vec<(String, PathBuf)>,
    },
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
    /// The best-supported entailment regime for this case (`sd:entailmentRegime`),
    /// if any: the dataset is materialized under it before the query runs. `None`
    /// for a plain (Simple-entailment) test or one whose only regimes the native
    /// reasoner cannot materialize (OWL-Direct / D / RIF).
    pub regime: Option<purrdf_entail::Regime>,
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
                regime: None,
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

    let mut cases: Vec<SparqlTestCase> = by_test.into_values().collect();
    // Update tests carry their pre-state, request, and post-state under the `ut:`
    // vocabulary, which the query SELECT above does not read. Fill those fields in
    // with a dedicated pass so the `ut:` shape (nested graphData blank nodes) is
    // read explicitly rather than shoe-horned into the query SELECT.
    if cases.iter().any(|c| c.kind == TestKind::UpdateEval) {
        load_update_details(&dataset, &dir, &mut cases)?;
    }
    // Entailment tests declare an `sd:entailmentRegime` list; select the regime
    // the native reasoner should materialize before the query runs.
    load_entailment_regimes(&dataset, &mut cases)?;
    Ok(cases)
}

/// Set `regime` for each test that declares an `sd:entailmentRegime` list,
/// choosing the regime the native reasoner should materialize under.
fn load_entailment_regimes(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    cases: &mut [SparqlTestCase],
) -> Result<(), String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX sd: <{SD_NS}>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         SELECT ?test ?regime WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         ?test mf:action ?act .\n\
         {{ ?act sd:entailmentRegime ?regime }}\n\
         UNION\n\
         {{ ?act sd:entailmentRegime/rdf:rest*/rdf:first ?regime }}\n\
         }}"
    );
    let rows = query_rows(dataset, &query)?;
    if rows.is_empty() {
        return Ok(()); // no entailment tests in this manifest
    }
    let mut by_test: BTreeMap<String, Vec<purrdf_entail::Regime>> = BTreeMap::new();
    for row in &rows {
        if let (Some(test), Some(reg)) = (iri_of(row, "test"), iri_of(row, "regime")) {
            if let Some(r) = purrdf_entail::Regime::from_iri(&reg) {
                by_test.entry(test).or_default().push(r);
            }
        }
    }
    for case in cases.iter_mut() {
        if let Some(regimes) = by_test.get(&case.iri) {
            case.regime = pick_regime(regimes);
        }
    }
    Ok(())
}

/// Choose the regime to materialize: prefer the weakest that still entails (RDFS),
/// then OWL-RL, then the identity regimes. Boundaries the native reasoner cannot
/// materialize (OWL-Direct / D) yield `None` — the case runs unmaterialized and,
/// if it needs those entailments, is recorded as a typed `Entailment` xfail.
fn pick_regime(regimes: &[purrdf_entail::Regime]) -> Option<purrdf_entail::Regime> {
    use purrdf_entail::Regime::{OwlRl, Rdf, Rdfs, Simple};
    [Rdfs, OwlRl, Rdf, Simple]
        .into_iter()
        .find(|pref| regimes.contains(pref))
}

/// An accumulated expected post-state: `(default-graph files, named graphs)`.
type ExpectedState = (Vec<PathBuf>, Vec<(String, PathBuf)>);

/// Fill in the `ut:`-vocabulary fields for every [`TestKind::UpdateEval`] case:
/// the `ut:request` update file, the pre-state (`ut:data`/`ut:graphData`), and
/// the expected post-state (`mf:result` → `ut:data`/`ut:graphData`).
fn load_update_details(
    dataset: &std::sync::Arc<purrdf_core::RdfDataset>,
    dir: &Path,
    cases: &mut [SparqlTestCase],
) -> Result<(), String> {
    let query = format!(
        "PREFIX mf: <{MF}>\n\
         PREFIX ut: <{UT}>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         PREFIX rdfs: <{RDFS_LABEL_NS}>\n\
         SELECT ?test ?request ?inData ?inGraph ?inLabel ?resData ?resGraph ?resLabel WHERE {{\n\
         ?mani mf:entries/rdf:rest*/rdf:first ?test .\n\
         ?test mf:action ?act .\n\
         OPTIONAL {{ ?act ut:request ?request }}\n\
         OPTIONAL {{ ?act ut:data ?inData }}\n\
         OPTIONAL {{ ?act ut:graphData ?ig . ?ig ut:graph ?inGraph . OPTIONAL {{ ?ig rdfs:label ?inLabel }} }}\n\
         OPTIONAL {{ ?test mf:result ?res .\n\
           OPTIONAL {{ ?res ut:data ?resData }}\n\
           OPTIONAL {{ ?res ut:graphData ?rg . ?rg ut:graph ?resGraph . OPTIONAL {{ ?rg rdfs:label ?resLabel }} }}\n\
         }}\n\
         }}"
    );
    let rows = query_rows(dataset, &query)?;

    // Accumulate the expected post-state per test IRI (built as we scan rows).
    let mut expected: BTreeMap<String, ExpectedState> = BTreeMap::new();

    let by_iri: BTreeMap<String, usize> = cases
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == TestKind::UpdateEval)
        .map(|(i, c)| (c.iri.clone(), i))
        .collect();

    for row in &rows {
        let Some(test_iri) = iri_of(row, "test") else {
            continue;
        };
        let Some(&idx) = by_iri.get(test_iri.as_str()) else {
            continue; // not an update test (or not modeled) — leave untouched
        };
        let case = &mut cases[idx];

        if let Some(req) = iri_of(row, "request") {
            case.query = local_path(dir, &req);
        }
        push_unique_path(
            &mut case.data,
            iri_of(row, "inData").map(|i| local_path(dir, &i)),
        );
        if let Some(g) = iri_of(row, "inGraph") {
            let name = lexical_of(row, "inLabel").unwrap_or_else(|| g.clone());
            let path = local_path(dir, &g);
            if !case.graph_data.iter().any(|(n, _)| *n == name) {
                case.graph_data.push((name, path));
            }
        }

        let acc = expected.entry(test_iri.clone()).or_default();
        push_unique_path(
            &mut acc.0,
            iri_of(row, "resData").map(|i| local_path(dir, &i)),
        );
        if let Some(g) = iri_of(row, "resGraph") {
            let name = lexical_of(row, "resLabel").unwrap_or_else(|| g.clone());
            let path = local_path(dir, &g);
            if !acc.1.iter().any(|(n, _)| *n == name) {
                acc.1.push((name, path));
            }
        }
    }

    for (iri, idx) in by_iri {
        let (data, graph_data) = expected.remove(&iri).unwrap_or_default();
        cases[idx].expected = ExpectedResult::DatasetState { data, graph_data };
    }
    Ok(())
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
        "UpdateEvaluationTest" => TestKind::UpdateEval,
        "PositiveSyntaxTest" | "PositiveSyntaxTest11" => TestKind::PositiveSyntax,
        "NegativeSyntaxTest" | "NegativeSyntaxTest11" => TestKind::NegativeSyntax,
        "PositiveUpdateSyntaxTest" | "PositiveUpdateSyntaxTest11" => TestKind::PositiveUpdateSyntax,
        "NegativeUpdateSyntaxTest" | "NegativeUpdateSyntaxTest11" => TestKind::NegativeUpdateSyntax,
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
