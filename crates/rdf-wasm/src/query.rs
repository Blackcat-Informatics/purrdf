// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The offline, in-browser SPARQL query surface over the wasm [`Dataset`].
//!
//! Binds the native multiset SPARQL evaluator
//! ([`NativeSparqlEngine`](purrdf_sparql_eval::NativeSparqlEngine)) to JavaScript so a
//! page can run SELECT / ASK / CONSTRUCT / DESCRIBE entirely client-side, with no
//! server and no network. The engine is the same one the native query gate uses,
//! with no baked-in HTTP client.
//!
//! ## Federation is intentionally absent
//!
//! This binds the plain [`SparqlEngine::query`](purrdf_core::SparqlEngine::query)
//! entry — the one with **no** [`RemoteQuerySource`](purrdf_sparql_eval::remote)
//! installed. A `SERVICE` or `LOAD` clause therefore **hard-fails** with a JsError
//! rather than silently returning an empty or partial result: in a browser there is
//! no resolver to fetch a remote graph, and a false answer is worse than an error.
//!
//! ## Result encoding
//!
//! - SELECT / ASK → **SPARQL Results JSON** (the W3C SRJ format) via
//!   [`purrdf_sparql_results`].
//! - CONSTRUCT / DESCRIBE → **Turtle** via the `native_codecs` serializer (the one
//!   serialization seam; never `oxigraph::io`, never the `purrdf-gts` crate).

use purrdf::ir::MutableDataset;
use purrdf::{SerializeGraph, serialize_dataset};
use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;
use purrdf_sparql_results::{
    ResultProvenance, SparqlResultsFormat, serialize as serialize_results,
};
use wasm_bindgen::prelude::*;

use crate::codec::resolve_media_type;
use crate::convert::term_value_to_rdf_term;
use crate::dataset::{Dataset, diag_to_err};
use crate::term::Term;

/// The typed result kind exposed to the package-root JavaScript wrapper.
#[derive(Debug, Clone, Copy)]
enum QueryResultKind {
    Select,
    Ask,
    Graph,
}

impl QueryResultKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Ask => "ask",
            Self::Graph => "graph",
        }
    }
}

/// One SELECT binding row.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct SelectRow {
    variables: Vec<String>,
    values: Vec<Option<Term>>,
}

#[wasm_bindgen]
impl SelectRow {
    /// Variables projected by this row, in SELECT projection order.
    #[wasm_bindgen(getter)]
    pub fn variables(&self) -> Vec<String> {
        self.variables.clone()
    }

    /// Return the bound term for a variable name, or `undefined` for unbound/absent.
    pub fn get(&self, variable: &str) -> Option<Term> {
        self.variables
            .iter()
            .position(|v| v == variable)
            .and_then(|i| self.values.get(i))
            .cloned()
            .flatten()
    }
}

/// A typed SELECT result returned by the raw wasm binding.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct SelectResult {
    variables: Vec<String>,
    rows: Vec<SelectRow>,
}

#[wasm_bindgen]
impl SelectResult {
    /// The result discriminator.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        QueryResultKind::Select.as_str().to_owned()
    }

    /// Projected variables, in SELECT projection order.
    #[wasm_bindgen(getter)]
    pub fn variables(&self) -> Vec<String> {
        self.variables.clone()
    }

    /// SELECT rows, one object-like row per solution.
    #[wasm_bindgen(getter)]
    pub fn rows(&self) -> Vec<SelectRow> {
        self.rows.clone()
    }
}

#[derive(Debug)]
enum QueryResultValue {
    Select(SelectResult),
    Ask(bool),
    Graph(Dataset),
}

/// A typed SPARQL result returned by the raw wasm binding.
#[wasm_bindgen]
#[derive(Debug)]
pub struct QueryResult {
    kind: QueryResultKind,
    value: Option<QueryResultValue>,
}

#[wasm_bindgen]
impl QueryResult {
    /// The result discriminator: `"select"`, `"ask"`, or `"graph"`.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.as_str().to_owned()
    }

    /// The ASK boolean when `kind === "ask"`, otherwise `undefined`.
    #[wasm_bindgen(getter)]
    pub fn boolean(&self) -> Option<bool> {
        match self.value {
            Some(QueryResultValue::Ask(value)) => Some(value),
            _ => None,
        }
    }

    /// Move the SELECT result out of this wrapper.
    #[wasm_bindgen(js_name = takeSelect)]
    pub fn take_select(&mut self) -> Option<SelectResult> {
        let value = self.value.take()?;
        match value {
            QueryResultValue::Select(result) => Some(result),
            other => {
                self.value = Some(other);
                None
            }
        }
    }

    /// Move the graph dataset out of this wrapper.
    #[wasm_bindgen(js_name = takeDataset)]
    pub fn take_dataset(&mut self) -> Option<Dataset> {
        let value = self.value.take()?;
        match value {
            QueryResultValue::Graph(dataset) => Some(dataset),
            other => {
                self.value = Some(other);
                None
            }
        }
    }
}

/// A reusable SPARQL engine that keeps the native plan cache alive across calls.
#[wasm_bindgen]
pub struct QueryEngine {
    inner: NativeSparqlEngine,
}

impl std::fmt::Debug for QueryEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryEngine").finish_non_exhaustive()
    }
}

#[wasm_bindgen]
impl QueryEngine {
    /// Create a reusable offline SPARQL engine.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: NativeSparqlEngine::new(),
        }
    }

    /// Run any SPARQL query and return a typed raw wasm result wrapper.
    #[wasm_bindgen(js_name = query)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<QueryResult, JsError> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        query_result_from_sparql(result)
    }

    /// Run a SELECT query and return typed rows.
    #[wasm_bindgen(js_name = select)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn select(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<SelectResult, JsError> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        select_result_from_sparql(result)
    }

    /// Run an ASK query and return the boolean result.
    #[wasm_bindgen(js_name = ask)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn ask(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<bool, JsError> {
        match self.run_query(dataset, sparql, base.as_deref())? {
            SparqlResult::Boolean(value) => Ok(value),
            other => Err(kind_mismatch("ASK boolean", &other)),
        }
    }

    /// Run a CONSTRUCT query and return its result dataset.
    #[wasm_bindgen(js_name = construct)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn construct(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<Dataset, JsError> {
        graph_result_from_sparql(self.run_query(dataset, sparql, base.as_deref())?)
    }

    /// Run a DESCRIBE query and return its result dataset.
    #[wasm_bindgen(js_name = describe)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn describe(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<Dataset, JsError> {
        graph_result_from_sparql(self.run_query(dataset, sparql, base.as_deref())?)
    }

    /// Apply a SPARQL UPDATE atomically to the supplied dataset.
    #[wasm_bindgen(js_name = update)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn update(
        &self,
        dataset: &mut Dataset,
        sparql: &str,
        base: Option<String>,
    ) -> Result<(), JsError> {
        let mut frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        self.inner
            .update(&mut frozen, sparql_request(sparql, base.as_deref()))
            .map_err(|e| diag_to_err(&e))?;
        dataset.inner = MutableDataset::new(frozen);
        Ok(())
    }

    /// Run any SPARQL query and serialize its raw result.
    #[wasm_bindgen(js_name = queryRaw)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query_raw(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<String>,
        format: Option<String>,
    ) -> Result<String, JsError> {
        let result = self.run_query(dataset, sparql, base.as_deref())?;
        serialize_query_result(&result, format.as_deref())
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryEngine {
    fn run_query(
        &self,
        dataset: &Dataset,
        sparql: &str,
        base: Option<&str>,
    ) -> Result<SparqlResult, JsError> {
        let frozen = dataset.inner.freeze().map_err(|e| diag_to_err(&e))?;
        self.inner
            .query(&frozen, sparql_request(sparql, base))
            .map_err(|e| diag_to_err(&e))
    }
}

#[wasm_bindgen]
impl Dataset {
    /// `query(sparql, base?)` → run a SPARQL query against this dataset, offline.
    ///
    /// Returns **SPARQL Results JSON** for SELECT / ASK and **Turtle** for
    /// CONSTRUCT / DESCRIBE. A parse error, an evaluation error, or a `SERVICE` /
    /// `LOAD` clause (unresolvable in-browser) throws a JsError — never a silent
    /// empty result.
    #[wasm_bindgen(js_name = query)]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    pub fn query(&self, sparql: &str, base: Option<String>) -> Result<String, JsError> {
        QueryEngine::new().query_raw(self, sparql, base, None)
    }
}

fn sparql_request<'a>(sparql: &'a str, base: Option<&'a str>) -> SparqlRequest<'a> {
    SparqlRequest {
        query: sparql,
        base_iri: base,
        substitutions: &[],
    }
}

fn query_result_from_sparql(result: SparqlResult) -> Result<QueryResult, JsError> {
    Ok(match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => QueryResult {
            kind: QueryResultKind::Select,
            value: Some(QueryResultValue::Select(select_result(variables, rows)?)),
        },
        SparqlResult::Boolean(value) => QueryResult {
            kind: QueryResultKind::Ask,
            value: Some(QueryResultValue::Ask(value)),
        },
        SparqlResult::Graph(graph) => QueryResult {
            kind: QueryResultKind::Graph,
            value: Some(QueryResultValue::Graph(Dataset {
                inner: MutableDataset::new(graph),
            })),
        },
    })
}

fn select_result_from_sparql(result: SparqlResult) -> Result<SelectResult, JsError> {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => select_result(variables, rows),
        other => Err(kind_mismatch("SELECT solutions", &other)),
    }
}

fn graph_result_from_sparql(result: SparqlResult) -> Result<Dataset, JsError> {
    match result {
        SparqlResult::Graph(graph) => Ok(Dataset {
            inner: MutableDataset::new(graph),
        }),
        other => Err(kind_mismatch("CONSTRUCT/DESCRIBE graph", &other)),
    }
}

fn select_result(
    variables: Vec<String>,
    rows: Vec<Vec<Option<purrdf::TermValue>>>,
) -> Result<SelectResult, JsError> {
    let rows = rows
        .into_iter()
        .map(|row| select_row(&variables, row))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SelectResult { variables, rows })
}

fn select_row(
    variables: &[String],
    row: Vec<Option<purrdf::TermValue>>,
) -> Result<SelectRow, JsError> {
    let values = row
        .into_iter()
        .map(|value| {
            value
                .as_ref()
                .map(term_from_value)
                .transpose()
                .map_err(|e| JsError::new(&e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SelectRow {
        variables: variables.to_vec(),
        values,
    })
}

fn term_from_value(value: &purrdf::TermValue) -> Result<Term, String> {
    let term = term_value_to_rdf_term(value)?;
    Ok(Term::from_rdf_term(&term))
}

fn serialize_query_result(result: &SparqlResult, format: Option<&str>) -> Result<String, JsError> {
    match result {
        SparqlResult::Graph(graph) => serialize_graph_result(graph, format.unwrap_or("turtle")),
        SparqlResult::Solutions { .. } | SparqlResult::Boolean(_) => {
            let results_format = match format {
                None => SparqlResultsFormat::Json,
                Some(format) => resolve_results_format(format)?,
            };
            serialize_tabular_result(result, results_format)
        }
    }
}

fn resolve_results_format(format: &str) -> Result<SparqlResultsFormat, JsError> {
    let normalized = format.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "json" | "srj" | "sparql-json" | "application/sparql-results+json" => {
            Ok(SparqlResultsFormat::Json)
        }
        "xml" | "sparql-xml" | "application/sparql-results+xml" => Ok(SparqlResultsFormat::Xml),
        "csv" | "text/csv" => Ok(SparqlResultsFormat::Csv),
        "tsv" | "text/tab-separated-values" => Ok(SparqlResultsFormat::Tsv),
        other => Err(JsError::new(&format!(
            "unsupported SPARQL results format {other:?} \
             (use json/xml/csv/tsv or graph formats for CONSTRUCT/DESCRIBE)"
        ))),
    }
}

fn serialize_tabular_result(
    result: &SparqlResult,
    format: SparqlResultsFormat,
) -> Result<String, JsError> {
    let outcome = serialize_results(result, format, &ResultProvenance::default())
        .map_err(|e| JsError::new(&e.to_string()))?;
    String::from_utf8(outcome.bytes)
        .map_err(|e| JsError::new(&format!("SPARQL result is not valid UTF-8: {e}")))
}

fn serialize_graph_result(
    graph: &std::sync::Arc<purrdf::RdfDataset>,
    format: &str,
) -> Result<String, JsError> {
    let normalized = format.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "jsonld" | "json-ld" | "application/ld+json"
    ) {
        return purrdf::native_codecs::jsonld::serialize_dataset_to_jsonld(graph)
            .map_err(|e| diag_to_err(&e));
    }
    let media_type = resolve_media_type(format).map_err(|e| JsError::new(&e))?;
    let bytes = serialize_dataset(graph, media_type, SerializeGraph::Dataset)
        .map_err(|e| diag_to_err(&e))?;
    String::from_utf8(bytes)
        .map_err(|e| JsError::new(&format!("SPARQL graph result is not valid UTF-8: {e}")))
}

fn kind_mismatch(expected: &str, actual: &SparqlResult) -> JsError {
    JsError::new(&format!(
        "expected {expected}, got {}",
        sparql_result_kind(actual)
    ))
}

fn sparql_result_kind(result: &SparqlResult) -> &'static str {
    match result {
        SparqlResult::Solutions { .. } => "SELECT solutions",
        SparqlResult::Boolean(_) => "ASK boolean",
        SparqlResult::Graph(_) => "CONSTRUCT/DESCRIBE graph",
    }
}
