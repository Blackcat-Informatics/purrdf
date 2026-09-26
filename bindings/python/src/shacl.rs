// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PyO3 Python bindings for `purrdf-shapes`.
//!
//! # Platform note
//!
//! This module belongs to the separate Python binding crate because pyo3 exposes
//! CPython C-API symbols and those are intentionally unavailable in the main
//! wasm-clean Rust crates. There are zero degraded fallbacks and zero feature
//! flags controlling this.
//!
//! # Engine core separation
//!
//! Only this file imports pyo3. All engine modules (`engine`, `shapes`,
//! `constraints`, `path`, `report`, `model`) are PyO3-free so the rlib links
//! into the future Rust compiler without any Python dependency.
//!
//! # The shapes graph's `owl:imports`
//!
//! Every function here that takes a Turtle shapes graph takes an `imports` keyword: the
//! shapes graph's `owl:imports` table, a sequence of `(ontology IRI, Turtle document)`
//! pairs — the same shape `purrdf.entail`'s `imports` takes — each document parsed with its
//! ontology IRI as its base. An `owl:imports` in the shapes graph is resolved by one of
//! these, by a document the shapes graph was read under (`shapes_base`, or its own
//! `@base`), by the closure declaring the ontology (`<X> a owl:Ontology`, or an
//! ontology whose `owl:versionIRI` is `<X>`), or by the closure describing `<X>` with
//! `sh:declare` — SHACL's prefix-declaration idiom. Anything else — or an entry no import names
//! — raises `ShapesImportError` rather than validating a smaller shapes graph than the one
//! named, exactly as the Rust API, the command line, WebAssembly and C refuse it. The
//! default `()` imports nothing and still enforces the rule. PurRDF fetches nothing.

use std::sync::Arc;

use ::purrdf::RdfDataset;
use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyCapsule, PyCapsuleMethods, PyDict, PyList, PyTuple};

use purrdf_shapes::engine;
use purrdf_shapes::report::ValidationReport;
use purrdf_validate::ShapesProductRefusal;

use crate::py_store::PyStore;

/// Validate a data graph (N-Triples) against a shapes graph (Turtle).
///
/// Returns a dict with keys:
/// - `"conforms"` — bool
/// - `"results"` — list of dicts, each with keys:
///   `"focus"`, `"path"`, `"value"`, `"severity"`, `"component"`,
///   `"source_shape"`, `"messages"` (every `sh:resultMessage`, each a dict with
///   `"text"` and its `"language"` / `"direction"` / `"datatype"` when present),
///   and, when the result carries SHACL-SPARQL result annotations
///   (`sh:resultAnnotation`), `"annotations"`: a list of `(property IRI, value)`
///   tuples, each value the RDF term in N-Triples syntax.
///
/// `shapes_base` is the base IRI the SHAPES document's relative IRI references resolve
/// against. This binding is handed a string and so has no retrieval IRI of its own;
/// PurRDF will not invent one, so a caller who read the shapes from a file or a URL and
/// wants `<PersonShape>` to mean something should pass that document's IRI. Left `None`,
/// a relative reference is a hard `ValueError` naming the remedy — never a validation
/// that quietly conforms because the constraint term was never resolved. `data_nt` needs
/// no counterpart: N-Triples admits no relative IRI by grammar.
///
/// `conformance_disallows` is the conformance-disallow set the report is judged
/// against: severity IRIs, a result whose severity is among them making the data
/// non-conforming (the report's `"conforms"` and every nested `sh:node` / `sh:not` /
/// `sh:and` / `sh:or` / `sh:xone` check alike). `None` is SHACL's default set,
/// `sh:Violation`, `sh:Warning` and `sh:Info`; an empty sequence or a value that is
/// not an absolute IRI raises `ValueError`. The dict's `"conformance_disallows"` key
/// lists the set the report was judged against. A result's `"severity"` is its IRI,
/// `sh:Debug` and `sh:Trace` included.
///
/// `imports` is the shapes graph's `owl:imports` table — see the [module documentation](self).
#[pyfunction]
#[pyo3(signature = (shapes_ttl, data_nt, *, shapes_base=None, conformance_disallows=None, imports=Vec::new()))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn validate(
    py: Python<'_>,
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<&str>,
    conformance_disallows: Option<Vec<String>>,
    imports: Vec<(String, String)>,
) -> PyResult<Py<PyAny>> {
    let options = match conformance_disallows {
        None => engine::ValidationOptions::default(),
        Some(iris) => engine::ValidationOptions::default().with_conformance_disallows(
            purrdf_shapes::report::ConformanceDisallows::from_iris(&iris)
                .map_err(pyo3::exceptions::PyValueError::new_err)?,
        ),
    };
    // Parse + validation run detached (GIL released); the result dicts are
    // built after the GIL is reacquired.
    let pairs = crate::py_entail::import_list(&imports);
    let report = py
        .detach(|| {
            let table = purrdf_shapes::ShapesImports::from_turtle(&pairs)?;
            engine::validate_graphs_with_options(data_nt, shapes_ttl, shapes_base, &options, &table)
        })
        .map_err(|error| shapes_error(py, error))?;

    let out = PyDict::new(py);
    out.set_item("conforms", report.conforms)?;
    out.set_item("conformance_disallows", report.conformance_disallows.iris())?;

    let results = PyList::empty(py);
    for r in &report.results {
        let d = PyDict::new(py);
        d.set_item("focus", r.focus_node.to_string())?;
        d.set_item("path", r.result_path.as_ref().map(ToString::to_string))?;
        d.set_item("value", r.value.as_ref().map(ToString::to_string))?;
        d.set_item("severity", r.severity.iri())?;
        d.set_item("component", r.source_constraint_component.as_str())?;
        d.set_item("source_shape", r.source_shape.to_string())?;
        d.set_item("messages", messages_list(py, &r.messages)?)?;
        if !r.annotations.is_empty() {
            d.set_item("annotations", annotations_list(&r.annotations))?;
        }
        if !r.source_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .source_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("source_box_roles", roles)?;
        }
        if !r.path_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .path_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("path_box_roles", roles)?;
        }
        if !r.result_box_roles.is_empty() {
            let roles: Vec<&str> = r
                .result_box_roles
                .iter()
                .map(purrdf_shapes::term::NamedNode::as_str)
                .collect();
            d.set_item("result_box_roles", roles)?;
        }
        results.append(d)?;
    }
    out.set_item("results", results)?;

    Ok(out.into_any().unbind())
}

/// A result's SHACL-SPARQL result annotations as `(property IRI, value)` pairs,
/// each value the RDF term in N-Triples syntax, in the report's canonical order.
fn annotations_list(
    annotations: &[(purrdf_shapes::term::NamedNode, purrdf_shapes::term::Term)],
) -> Vec<(String, String)> {
    annotations
        .iter()
        .map(|(property, value)| (property.as_str().to_owned(), value.to_string()))
        .collect()
}

/// A result's messages as Python: one dict per `sh:resultMessage` literal, in the
/// report's canonical order, with `"text"` and — when the literal has them —
/// `"language"`, `"direction"` (`"ltr"`/`"rtl"`) and `"datatype"` (given only for
/// a datatype other than `xsd:string` and the language-string types). Every
/// message is kept: SHACL copies all of a shape's messages into each result.
fn messages_list<'py>(
    py: Python<'py>,
    messages: &[purrdf_shapes::term::Literal],
) -> PyResult<Bound<'py, PyList>> {
    let list = PyList::empty(py);
    for message in messages {
        let entry = PyDict::new(py);
        entry.set_item("text", message.value())?;
        if let Some(language) = message.language() {
            entry.set_item("language", language)?;
        }
        if let Some(direction) = message.direction() {
            entry.set_item(
                "direction",
                match direction {
                    ::purrdf::RdfTextDirection::Ltr => "ltr",
                    ::purrdf::RdfTextDirection::Rtl => "rtl",
                },
            )?;
        }
        if message.language().is_none()
            && message.datatype_str() != "http://www.w3.org/2001/XMLSchema#string"
        {
            entry.set_item("datatype", message.datatype_str())?;
        }
        list.append(entry)?;
    }
    Ok(list)
}

/// Entail a data graph (N-Triples) under a shapes graph (Turtle), returning the
/// materialized dataset as a canonical N-Triples string.
///
/// The entailment twin of [`validate`]: it runs the shapes graph's default rule
/// set (`sh:TripleRule` / `sh:SPARQLRule`) as SHACL 1.2 Inference Rules executes
/// it — layer by layer in `sh:layer` order, each layer's `sh:runOnce` rules once
/// and its iterating rules while an iteration infers a new triple, in `sh:order`
/// groups — and returns the base graph plus every inferred triple, serialized as
/// deterministic N-Triples.
///
/// Raises `ValueError` if either graph fails to parse or if rule application
/// fails (an illegal head term, an unresolvable `sh:condition`, an unregistered
/// `sh:ruleProcessor`, or a rule set that passes the engine's term-generating
/// round limit or another fixed ceiling).
///
/// # One boundary, three bindings
///
/// The parse→entail→serialize sequence is NOT reimplemented here: it is
/// [`purrdf_validate::entail_to_ntriples_string`], the same function the C-ABI
/// (`purrdf_shacl_entail_to_ntriples`) and WASM (`shacl_entail`) bindings call.
/// This binding used to inline the two-line body, which made that crate's
/// "the shared boundary the language bindings all route through" claim false and
/// left a third copy free to drift. Everything Python-specific — releasing the
/// GIL, mapping the error string to `ValueError` — stays here; the RDF work does
/// not.
///
/// `imports` is the shapes graph's `owl:imports` table — see the [module documentation](self).
#[pyfunction]
#[pyo3(signature = (shapes_ttl, data_nt, *, shapes_base=None, imports=Vec::new()))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn entail(
    py: Python<'_>,
    shapes_ttl: &str,
    data_nt: &str,
    shapes_base: Option<&str>,
    imports: Vec<(String, String)>,
) -> PyResult<String> {
    let pairs = crate::py_entail::import_list(&imports);
    // Parse + entailment + serialization run detached (GIL released).
    py.detach(|| {
        purrdf_validate::entail_to_ntriples_string(shapes_ttl, shapes_base, data_nt, &pairs)
    })
    .map_err(|error| shapes_error(py, error))
}

/// Run a rule set over a data graph (N-Triples) and return the INFERENCE GRAPH — the
/// inferred triples only, never the data graph — as a dict:
///
/// - `"inferred"` — N-Triples 1.2, one triple per line, in canonical order;
/// - `"proof"` — when `explain` is true, the proof of every inferred triple
///   (`derived S P O .`, then `  rule R` and one `  premise S P O .` per matched fact,
///   or `  data-block` for a SPARQL 1.2 RL data-block triple); otherwise `None`.
///
/// The rule source is exactly one of `shapes_ttl` — a SHACL shapes graph (Turtle), whose
/// default rule set runs — and `srl`, a SPARQL 1.2 RL rule set; naming neither or both
/// raises `ValueError`. `shapes_base` / `srl_base` are the documents' base IRIs.
///
/// `max_term_generating_rounds` bounds the evaluation rounds that infer a term the graph
/// did not hold; one more raises `ValueError` naming the limit. `None` keeps the engine
/// default (65,536). A host running UNTRUSTED rule sets should lower it: an exponential
/// rule set reaches the engine's fixed arena and join ceilings only slowly under the
/// default, and the limit is what bounds the time it can take.
///
/// `imports` is the SHACL shapes graph's `owl:imports` table — see the [module documentation](self); an
/// imported document's rules run. A SPARQL 1.2 RL rule set reads no table, so passing one
/// beside `srl` raises `ValueError`.
///
/// The work is [`purrdf_validate::apply_rules_to_ntriples`], the function the WASM and
/// C-ABI bindings call.
#[pyfunction]
#[pyo3(signature = (
    data_nt,
    shapes_ttl=None,
    *,
    srl=None,
    shapes_base=None,
    srl_base=None,
    explain=false,
    max_term_generating_rounds=None,
    imports=Vec::new(),
))]
#[allow(clippy::too_many_arguments)] // mirrors the Python keyword surface one-to-one
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn apply_rules(
    py: Python<'_>,
    data_nt: &str,
    shapes_ttl: Option<&str>,
    srl: Option<&str>,
    shapes_base: Option<&str>,
    srl_base: Option<&str>,
    explain: bool,
    max_term_generating_rounds: Option<u64>,
    imports: Vec<(String, String)>,
) -> PyResult<Py<PyAny>> {
    let pairs = crate::py_entail::import_list(&imports);
    let outcome = py
        .detach(|| {
            purrdf_validate::apply_rules_to_ntriples(&purrdf_validate::RulesRequest {
                data_nt,
                shapes_ttl,
                shapes_base,
                shapes_imports: &pairs,
                srl,
                srl_base,
                explain,
                max_term_generating_rounds,
            })
        })
        .map_err(|error| shapes_error(py, error))?;
    let out = PyDict::new(py);
    out.set_item("inferred", outcome.inferred_ntriples)?;
    out.set_item("proof", outcome.proof)?;
    Ok(out.into_any().unbind())
}

/// Evaluate ONE node expression of a shapes graph (Turtle) against a focus node of a data
/// graph (N-Triples) — SHACL 1.2 Node Expressions' `evalExpr(expr, focusGraph, focusNode,
/// scope)` — returning its output nodes as N-Triples 1.2 terms, in the order the
/// expression's sequence semantics define.
///
/// `expr` is the expression node: an absolute IRI, or `"_:label"` for a blank node the
/// shapes document labels so. `focus` is an absolute IRI or any N-Triples term. `scope`
/// maps each `shnex:var` name to a term spelled as `focus` is; the name `focusNode`
/// (resolved to the focus node before the scope is searched) raises `ValueError`, as do a
/// label the shapes document never wrote and any parse or evaluation failure.
///
/// `imports` is the shapes graph's `owl:imports` table — see the [module documentation](self); an
/// imported document's functions and shapes are in scope.
#[pyfunction]
#[pyo3(signature = (shapes_ttl, data_nt, expr, focus, *, scope=None, shapes_base=None, imports=Vec::new()))]
#[allow(clippy::too_many_arguments)] // mirrors the Python keyword surface one-to-one
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn eval_node_expr(
    py: Python<'_>,
    shapes_ttl: &str,
    data_nt: &str,
    expr: &str,
    focus: &str,
    scope: Option<std::collections::BTreeMap<String, String>>,
    shapes_base: Option<&str>,
    imports: Vec<(String, String)>,
) -> PyResult<Vec<String>> {
    let pairs = crate::py_entail::import_list(&imports);
    let scope = scope.unwrap_or_default();
    let bindings: Vec<(&str, &str)> = scope
        .iter()
        .map(|(name, term)| (name.as_str(), term.as_str()))
        .collect();
    py.detach(|| {
        purrdf_validate::eval_node_expr_to_terms(&purrdf_validate::NodeExprRequest {
            shapes_ttl,
            shapes_base,
            data_nt,
            expr,
            focus,
            scope: &bindings,
            imports: &pairs,
        })
    })
    .map_err(|error| shapes_error(py, error))
}

/// Certify a shapes graph (Turtle), COLD: the loader's verdict, every result of validating
/// it against the W3C `shacl-shacl.ttl`, and which implementation every node-expression
/// function call binds to. Returns a dict:
///
/// - `"clean"` — no finding: the loader accepted the graph and every `shacl-shacl`
///   result is superseded (flagged by `shacl-shacl.ttl` but well-formed SHACL 1.2 Core);
/// - `"findings"` — the finding count;
/// - `"load_error"` — the loader's refusal, or `None`;
/// - `"shacl_shacl"` — one dict per result: `"focus"`, `"path"`, `"value"`,
///   `"component"`, `"source_shape"`, `"severity"`, `"messages"`, `"superseded"` (the
///   supersession rule's name, or `None`);
/// - `"calls"` — one dict per function call site, `"binding"` (`native`, `custom`,
///   `sparql-registered`, `host-extension`), `"function"`, `"owner"`; `None` when the
///   loader refused the graph;
/// - `"report"` — the deterministic text every PurRDF host prints.
///
/// The report certifies the shapes graph's whole `owl:imports` closure, resolved against
/// `imports` (see the [module documentation](self)). A closure that is not in hand raises
/// `ShapesImportError` — never a report about the importing document alone, which would
/// call a shapes graph `clean` that validation refuses.
///
/// Otherwise raises `ValueError` only when the document is not Turtle; a malformed shapes
/// graph is a report, not an exception.
#[pyfunction]
#[pyo3(signature = (shapes_ttl, *, shapes_base=None, imports=Vec::new()))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn lint_shapes(
    py: Python<'_>,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    imports: Vec<(String, String)>,
) -> PyResult<Py<PyAny>> {
    let pairs = crate::py_entail::import_list(&imports);
    let report = py
        .detach(|| purrdf_validate::lint_shapes_ttl(shapes_ttl, shapes_base, &pairs))
        .map_err(|error| shapes_error(py, error))?;
    let out = PyDict::new(py);
    out.set_item("clean", report.is_clean())?;
    out.set_item("findings", report.findings())?;
    out.set_item("load_error", report.load_error())?;
    let results = PyList::empty(py);
    for result in report.shacl_shacl() {
        let d = PyDict::new(py);
        d.set_item("focus", result.focus.to_string())?;
        d.set_item("path", result.path.as_ref().map(ToString::to_string))?;
        d.set_item("value", result.value.as_ref().map(ToString::to_string))?;
        d.set_item("component", &result.component)?;
        d.set_item("source_shape", result.source_shape.to_string())?;
        d.set_item("severity", &result.severity)?;
        d.set_item("messages", &result.messages)?;
        d.set_item("superseded", result.superseded.map(|rule| rule.name))?;
        results.append(d)?;
    }
    out.set_item("shacl_shacl", results)?;
    match report.function_resolution() {
        None => out.set_item("calls", py.None())?,
        Some(functions) => {
            let calls = PyList::empty(py);
            for site in functions.sites() {
                let d = PyDict::new(py);
                d.set_item("binding", site.binding.label())?;
                d.set_item("function", &site.function)?;
                d.set_item("owner", &site.owner)?;
                calls.append(d)?;
            }
            out.set_item("calls", calls)?;
        }
    }
    out.set_item("report", report.render())?;
    Ok(out.into_any().unbind())
}

/// Parsed SHACL shapes that can be reused to validate multiple data graphs.
///
/// Construct from a Turtle shapes graph with `PyShapes(shapes_ttl)`, then call
/// `validate_nt(data_nt)` for each data graph. The Rust orchestration path in
/// `purrdf-validate` borrows the parsed shapes via [`Self::validate_against_dataset`].
#[pyclass(name = "Shapes")]
#[derive(Debug)]
pub struct PyShapes {
    inner: purrdf_shapes::shapes::Shapes,
}

impl PyShapes {
    /// Validate a borrowed native [`RdfDataset`] against these parsed shapes.
    ///
    /// This is the Rust-side primitive used by `purrdf-validate::PyValidationStore`
    /// so the data store does not have to be re-serialized to N-Triples.
    pub fn validate_against_dataset(&self, data: &RdfDataset) -> ValidationReport {
        engine::validate_dataset(data, &self.inner)
            .expect("validation over a frozen dataset is infallible")
    }
}

#[pymethods]
impl PyShapes {
    /// `Shapes(shapes_ttl, *, base=None, imports=())`.
    ///
    /// `base` is the shapes document's own base IRI, used to resolve its relative IRI
    /// references (RFC-3986 §5.1.2). Omitted, only an in-document `@base` can establish
    /// one and a relative reference otherwise raises `ValueError`.
    ///
    /// `imports` is the shapes graph's `owl:imports` table (see the [module documentation](self)): the
    /// parsed shapes are the whole closure, and a closure that is not in hand raises
    /// `ShapesImportError`.
    #[new]
    #[pyo3(signature = (shapes_ttl, *, base=None, imports=Vec::new()))]
    #[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
    fn new(
        py: Python<'_>,
        shapes_ttl: &str,
        base: Option<&str>,
        imports: Vec<(String, String)>,
    ) -> PyResult<Self> {
        let pairs = crate::py_entail::import_list(&imports);
        // Shapes-graph parsing runs detached (GIL released).
        let inner = py
            .detach(|| {
                let table = purrdf_shapes::ShapesImports::from_turtle(&pairs)?;
                engine::parse_shapes_with_config(shapes_ttl, base, None, &table)
            })
            .map_err(|error| shapes_error(py, error))?;
        Ok(Self { inner })
    }

    /// Validate an N-Triples data graph against these parsed shapes.
    fn validate_nt(&self, py: Python<'_>, data_nt: &str) -> PyResult<PyValidationReport> {
        // Native codec ingest: lenient on private-use language tags, every
        // malformed line reported in one pass. The engine runs over the frozen IR.
        // Both the ingest and the validation run detached (GIL released).
        let shapes = &self.inner;
        let report = py.detach(|| {
            let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
                .map_err(|errors| pyo3::exceptions::PyValueError::new_err(errors.join("\n")))?;
            engine::validate_dataset(data.as_ref(), shapes)
                .map_err(pyo3::exceptions::PyValueError::new_err)
        })?;
        Ok(PyValidationReport::new(report))
    }

    /// Analyze this shape tree once, without inspecting any data, returning a
    /// reusable `PreparedShapes`.
    ///
    /// The step that makes a prepared PRODUCT possible: a `PreparedShapes` is what
    /// `to_product()` writes out, and what admitting a product hands back.
    fn prepare(&self, py: Python<'_>) -> PyPreparedShapes {
        let shapes = self.inner.clone();
        PyPreparedShapes {
            inner: py.detach(|| engine::PreparedShapes::new(Arc::new(shapes))),
        }
    }

    /// `extension_usage(*, relation_iris=(), relation_namespaces=(), extension_namespaces=())`.
    ///
    /// What the environment those declarations describe would make of every SPARQL
    /// text this shapes graph carries — asked and answered BEFORE any validation runs.
    ///
    /// # The question this answers
    ///
    /// A host wires a relation, writes a shapes graph that names it, validates, and
    /// gets `conforms: True`. Did the relation run? Running the validation cannot say:
    /// an IRI the environment does not recognize becomes an ordinary triple pattern,
    /// matches whatever the data holds for that predicate — usually nothing — and
    /// answers, which is also exactly what a correctly-resolved relation over no
    /// matching rows returns. The two are indistinguishable from the report.
    ///
    /// # Why this takes declarations rather than a registry
    ///
    /// Whether a predicate is a call is decided at PARSE time, by the IRI set and the
    /// declared namespaces alone — never by what a relation would return. So this
    /// needs no implementations, no registry and no data graph, which is what lets it
    /// be asked before anything is wired: pass the IRIs and namespaces you INTEND to
    /// register and find out what the parse will make of them.
    ///
    /// Returns `{"sites": {site: {"calls": [iri], "data": [iri]}}, "unreadable":
    /// {site: reason}, "complete": bool}`. An IRI you believe you registered appearing
    /// under `"data"` is the answer to "why did my relation never run?". A non-empty
    /// `"unreadable"` means the graph and these declarations genuinely disagree and
    /// validating under them will fail at those sites.
    ///
    /// # Errors
    ///
    /// `ValueError` if the declarations cannot form an environment.
    #[pyo3(signature = (*, relation_iris=None, relation_namespaces=None, extension_namespaces=None))]
    fn extension_usage<'py>(
        &self,
        py: Python<'py>,
        relation_iris: Option<Vec<String>>,
        relation_namespaces: Option<Vec<String>>,
        extension_namespaces: Option<Vec<String>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let env =
            purrdf_sparql_eval::ExtensionEnv::over_options(purrdf_sparql_eval::ParserOptions {
                extension_fn_namespaces: extension_namespaces.unwrap_or_default(),
                property_fn_namespaces: relation_namespaces.unwrap_or_default(),
                property_fn_iris: relation_iris.unwrap_or_default(),
            })
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("extension usage: {e}"))
            })?;

        let usage = py.detach(|| self.inner.extension_usage(&env));

        let sites = PyDict::new(py);
        for (site, used) in usage.sites() {
            let entry = PyDict::new(py);
            entry.set_item("calls", used.calls.iter().cloned().collect::<Vec<_>>())?;
            entry.set_item("data", used.data.iter().cloned().collect::<Vec<_>>())?;
            sites.set_item(site.as_str(), entry)?;
        }
        let unreadable = PyDict::new(py);
        for (site, why) in usage.unreadable() {
            unreadable.set_item(site.as_str(), why)?;
        }

        let out = PyDict::new(py);
        out.set_item("sites", sites)?;
        out.set_item("unreadable", unreadable)?;
        out.set_item("complete", usage.is_complete())?;
        Ok(out)
    }

    /// Validate a borrowed native dataset against these parsed shapes.
    ///
    /// `data` is any object exposing the internal snapshot protocol — a
    /// `_store_capsule()` method returning a capsule that carries a frozen
    /// `Arc<RdfDataset>` — which on the Python surface is `purrdf.Store`,
    /// `purrdf.MutableDataset` and `purrdf_validate.ValidationStore`. Validating
    /// through the capsule is what avoids serialising the data to N-Triples and
    /// parsing it back for each validation phase.
    ///
    /// # Errors
    ///
    /// Returns `TypeError` naming the argument's type if it does not implement that
    /// protocol: a missing private attribute is not a diagnosis, so the refusal says
    /// what the protocol is and which types satisfy it rather than letting an
    /// `AttributeError` about `_store_capsule` escape to a caller who never wrote
    /// that name. Returns `ValueError` if the capsule is present but cannot be read.
    fn validate_store(&self, data: &Bound<'_, PyAny>) -> PyResult<PyValidationReport> {
        if !data.hasattr("_store_capsule")? {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "validate_store: a {} exposes no `_store_capsule()`, the internal protocol this \
                 call reads a frozen dataset snapshot through, so there is no data graph here to \
                 validate. Pass a purrdf.Store or a purrdf.MutableDataset — both hand the snapshot \
                 over directly, with no serialization — or, for a document you hold as text, call \
                 validate_nt with its N-Triples",
                data.get_type().name()?
            )));
        }
        let capsule = data.call_method0("_store_capsule")?;
        let capsule = capsule.cast::<PyCapsule>().map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(
                "validate_store: `_store_capsule()` returned something other than a capsule, so \
                 the snapshot protocol was not honoured and no dataset can be read from it",
            )
        })?;
        let ptr = capsule
            .pointer_checked(Some(c"purrdf-validation-dataset"))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let addr = unsafe { *ptr.cast::<usize>().as_ptr() };
        // SAFETY: the capsule's value is the address of an `Arc<RdfDataset>` the
        // producer keeps alive (and at a stable address) for the capsule's lifetime.
        // We clone the `Arc` (extending the dataset's lifetime past the capsule
        // borrow) so validation can run detached (GIL released) without touching
        // any py-bound value.
        let dataset = Arc::clone(unsafe { &*(addr as *const Arc<RdfDataset>) });
        let report = data
            .py()
            .detach(|| self.validate_against_dataset(dataset.as_ref()));
        Ok(PyValidationReport::new(report))
    }
}

/// A SHACL validation report.
///
/// Wraps the Rust [`crate::report::ValidationReport`] and exposes `conforms`,
/// the list of result dicts, and a canonical N-Triples serialization.
#[pyclass(name = "ValidationReport")]
#[derive(Debug)]
pub struct PyValidationReport {
    inner: ValidationReport,
}

impl PyValidationReport {
    /// Construct from a Rust [`crate::report::ValidationReport`].
    pub fn new(inner: ValidationReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValidationReport {
    #[getter]
    fn conforms(&self) -> bool {
        self.inner.conforms
    }

    #[getter]
    fn results(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for r in &self.inner.results {
            let d = PyDict::new(py);
            d.set_item("focus", r.focus_node.to_string())?;
            d.set_item("path", r.result_path.as_ref().map(ToString::to_string))?;
            d.set_item("value", r.value.as_ref().map(ToString::to_string))?;
            d.set_item("severity", r.severity.iri())?;
            d.set_item("component", r.source_constraint_component.as_str())?;
            d.set_item("source_shape", r.source_shape.to_string())?;
            d.set_item("messages", messages_list(py, &r.messages)?)?;
            if !r.annotations.is_empty() {
                d.set_item("annotations", annotations_list(&r.annotations))?;
            }
            if !r.source_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .source_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("source_box_roles", roles)?;
            }
            if !r.path_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .path_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("path_box_roles", roles)?;
            }
            if !r.result_box_roles.is_empty() {
                let roles: Vec<&str> = r
                    .result_box_roles
                    .iter()
                    .map(purrdf_shapes::term::NamedNode::as_str)
                    .collect();
                d.set_item("result_box_roles", roles)?;
            }
            list.append(d)?;
        }
        Ok(list.into_any().unbind())
    }

    /// Serialize the report to canonical N-Triples (runs detached — GIL released).
    fn to_ntriples(&self, py: Python<'_>) -> String {
        let report = &self.inner;
        py.detach(|| report.to_ntriples())
    }

    /// Serialize the report to a SARIF 2.1.0 JSON string (runs detached — GIL
    /// released). The sibling of [`to_ntriples`](Self::to_ntriples): the RDF
    /// report graph stays canonical N-Triples, while SARIF is the source-traced,
    /// actionable surface for editors and CI. Logical locations are emitted here
    /// (focus node, result path, constraint component, source shape); physical
    /// source spans require a span-tracked parse.
    fn to_sarif(&self, py: Python<'_>) -> String {
        let report = &self.inner;
        py.detach(|| {
            purrdf_validate::report_to_sarif_string(
                report,
                &purrdf_validate::SarifOptions::default(),
            )
        })
    }
}

// ── Prepared shapes products ────────────────────────────────────────────────

create_exception!(
    shacl,
    ShapesProductError,
    pyo3::exceptions::PyValueError,
    "A refusal from the prepared-shapes-product admission boundary.\n\
     \n\
     Carries a `.dimension` attribute: one of the codec's pinned kebab-case labels \
     (`magic`, `format-version`, `stage-id`, `profile`, `truncated`, `trailer`, \
     `section-digest`, `container-digest`, `dataset-identity`, `shapes-graph`, \
     `prefixes`, `base`, `vocabulary`, `function-registry`, `aggregate-registry`, \
     `property-function-registry`, `class-catalog`, `unsupported-capability`, \
     `depth-limit`, `malformed`), or `None` when the failure happened before any \
     product existed — a shapes or data document that did not parse was never \
     admitted, and naming a dimension for it would claim a product was inspected \
     when none was.\n\
     \n\
     Branch on `.dimension`, never on `str(exc)`: the label is a pinned contract and \
     the message is prose that may be reworded. A `stage-id` refusal means re-pack \
     or restore with `rebuild()`; `container-digest` means the bytes are corrupt in \
     place; `function-registry` means the caller's own configuration differs from \
     the one the product was prepared against, which re-packing will not fix.\n\
     \n\
     Subclasses `ValueError`, so code that already catches the SHACL surface's \
     `ValueError` keeps working."
);

/// Raise a product refusal as [`ShapesProductError`], with `.dimension` always
/// present.
///
/// Always present — `None` rather than absent when there is no dimension — because an
/// attribute that sometimes exists forces every caller to write `getattr(exc,
/// "dimension", None)`, and the one who forgets gets an `AttributeError` from their
/// own error handler.
fn product_error(py: Python<'_>, refusal: &ShapesProductRefusal) -> PyErr {
    let error = ShapesProductError::new_err(refusal.to_string());
    let dimension = refusal.dimension_label().map_or_else(
        || py.None(),
        |label| {
            label
                .into_pyobject(py)
                .map_or_else(|_| py.None(), |bound| bound.into_any().unbind())
        },
    );
    // A failure to set the attribute would mean the exception object refused an
    // ordinary `setattr`, which cannot happen for a Python-level exception class;
    // it is ignored rather than replacing a precise refusal with a vaguer one.
    let _ = error.value(py).setattr("dimension", dimension);
    error
}

/// An immutable shape preparation: the parse-and-analyze work done once, reusable
/// across independent data graphs and writable as a prepared PRODUCT.
///
/// Obtain one with `Shapes(...).prepare()`, or by admitting a product with
/// `ShapesProduct.open(data).admit()`.
#[pyclass(name = "PreparedShapes")]
#[derive(Debug)]
pub struct PyPreparedShapes {
    inner: engine::PreparedShapes,
}

#[pymethods]
impl PyPreparedShapes {
    /// Write this preparation out as a prepared product, returning its bytes.
    ///
    /// Byte-deterministic: no clock, no randomness and no hash-iteration order reach
    /// the writer, so two calls over equal preparations return identical bytes and a
    /// content-addressed cache key over them is stable.
    ///
    /// The product carries the compiled model AND the shapes dataset it came from,
    /// under a per-section SHA-256 and a whole-container digest, plus the binding of
    /// every input it was compiled against.
    ///
    /// Raises `ShapesProductError` when the shapes graph declares something the
    /// product format cannot carry.
    fn to_product<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let prepared = &self.inner;
        let bytes = py
            .detach(|| purrdf_validate::prepared_to_product(prepared))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Where this preparation came from, as one deterministic token — `parsed`,
    /// `restored-admitted <identity_digest>` or `restored-rebuilt <identity_digest>`.
    ///
    /// TOTAL: every preparation has an answer and none of them means "we forgot to
    /// record it". An authenticated artifact whose output cannot be attributed to it
    /// answers only half the question — `admit()` establishes that this process MAY
    /// execute a product, and this establishes WHICH product a report came out of,
    /// which is the half a caller needs after the fact.
    ///
    /// The digest is the same 64 hexadecimal digits `identity_digest()` returns for
    /// the product, so a value read off a validation log can be handed straight back
    /// to `admit_expecting()` without editing. The two restore tokens are distinct
    /// because the identity means different things on each: admitted means the
    /// product's binding was checked against this process, rebuilt means it was
    /// recorded from the artifact and deliberately not checked (see `rebuild()`).
    fn provenance(&self) -> String {
        self.inner.provenance().to_string()
    }

    /// **The incremental lane.** Validate only what `store`'s PENDING CHANGE can
    /// move, rather than the whole graph.
    ///
    /// A `Store` records its mutations as a copy-on-write delta over a frozen base
    /// (`add` / `remove` / `load` edit that delta), so a store that has been mutated
    /// already holds the one thing incremental validation needs: a description of
    /// what moved. This reads that delta, asks the engine which focus nodes the
    /// change can move, and re-validates exactly those.
    ///
    /// ```python
    /// store = purrdf.Store()
    /// store.load(base_ttl, "turtle")
    /// store.checkpoint()          # everything loaded so far is now the BASE
    /// store.add(quad)             # ... and this is the change
    ///
    /// outcome = shapes.prepare().validate_store_changes(store)
    /// if not outcome.report.conforms:
    ///     ...                     # what THIS change broke
    /// ```
    ///
    /// Call `Store.checkpoint()` first or the "change" is the whole store: a fresh
    /// `Store` has an empty base, so everything ever loaded into it is in the delta.
    /// `Store.change_size()` reports how large the pending change is.
    ///
    /// # What the returned report describes
    ///
    /// `ChangeValidation.bounded` is `True` when the engine could bound the change's
    /// footprint. The report then covers the AFFECTED focus nodes — for those nodes
    /// it is identical, results and order alike, to what a full validation of the
    /// mutated graph reports about them — and says nothing about a pre-existing
    /// violation the change cannot reach. `conforms` therefore means "this change
    /// introduced no violation", not "the graph conforms".
    ///
    /// `bounded` is `False` when the shapes graph reads through SPARQL query text
    /// (`sh:sparql`, a SPARQL target, a component's `sh:ask`/`sh:select` validator, a
    /// `sh:SPARQLFunction` call, a SPARQL node expression). No bounded footprint
    /// exists for such a graph, so this falls back to a FULL validation of the
    /// mutated graph and `ChangeValidation.reason` names the construct responsible.
    /// The fallback is not optional: an under-approximated change set is
    /// indistinguishable from a clean bill of health.
    ///
    /// A `sh:shapesGraph` the shapes document declares is honoured, exactly as it is
    /// on every other validation route here.
    ///
    /// # Errors
    ///
    /// `ValueError` when the store cannot be snapshotted, when the snapshot exceeds
    /// the view's retention limits, or when constraint evaluation hard-fails.
    fn validate_store_changes(
        &self,
        py: Python<'_>,
        store: &Bound<'_, PyStore>,
    ) -> PyResult<PyChangeValidation> {
        // Taken under the GIL (it borrows the store), then owned — so the expansion
        // and the validation below run detached with nothing py-bound in hand.
        let snapshot = Arc::new(store.borrow().change_snapshot()?);
        let prepared = &self.inner;
        let validation = py.detach(|| {
            let validator = prepared
                .bind_delta_with_shapes_graph(
                    Arc::clone(&snapshot),
                    None,
                    ::purrdf::ir::ViewLimits::default(),
                )
                .map_err(pyo3::exceptions::PyValueError::new_err)?;
            // The engine's own expand-then-validate entry point, which is what the
            // command line, the C ABI and the WebAssembly guest all drive: one
            // implementation of the loop, so no surface can answer a question the
            // others would not.
            engine::validate_change(&validator, &snapshot)
                .map_err(pyo3::exceptions::PyValueError::new_err)
        })?;
        Ok(PyChangeValidation {
            report: Py::new(py, PyValidationReport::new(validation.report))?,
            scope: validation.scope,
        })
    }

    /// Validate an N-Triples data graph against this preparation.
    ///
    /// The same verdict `Shapes.validate_nt` reaches, through the same engine entry
    /// point — which is the property a restored product is only useful if it has.
    fn validate_nt(&self, py: Python<'_>, data_nt: &str) -> PyResult<PyValidationReport> {
        let prepared = &self.inner;
        let report = py.detach(|| {
            let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
                .map_err(|errors| pyo3::exceptions::PyValueError::new_err(errors.join("\n")))?;
            engine::validate_dataset(data.as_ref(), prepared.shapes())
                .map_err(pyo3::exceptions::PyValueError::new_err)
        })?;
        Ok(PyValidationReport::new(report))
    }
}

/// The outcome of `PreparedShapes.validate_store_changes`: the report, plus the
/// SCOPE the report describes.
///
/// Two facts rather than one, because a `ValidationReport` alone cannot say which
/// question it answered. An incremental run reports about the focus nodes the change
/// could move; a run whose change footprint could not be bounded reports about the
/// whole graph. Both are honest answers and they are not the same answer, so a caller
/// reading `conforms` is told which one they have rather than left to assume.
///
/// Returning the scope beside the report is the same choice `UpdateOutcome` makes for
/// a governed update: the outcome carries the evidence of how it was reached, and an
/// attribute that is sometimes absent would force every caller to `getattr`.
#[pyclass(name = "ChangeValidation")]
#[derive(Debug)]
pub struct PyChangeValidation {
    /// The report, built once here rather than on each `report` read, so two reads
    /// cannot hand back two independently-constructed objects.
    report: Py<PyValidationReport>,
    /// Which question the report answered, carried as the engine's own two-armed
    /// answer rather than re-spelled as a pair of `Option`s here. A pair admits a
    /// fourth state — neither set — that the engine cannot produce, and this class
    /// would then have to render something for it.
    scope: engine::ChangeScope,
}

#[pymethods]
impl PyChangeValidation {
    /// The SHACL validation report. See `bounded` for what it describes.
    #[getter]
    fn report(&self, py: Python<'_>) -> Py<PyValidationReport> {
        self.report.clone_ref(py)
    }

    /// Whether the change's footprint could be bounded.
    ///
    /// `True`: the report describes the AFFECTED focus nodes only, and `conforms`
    /// means this change introduced no violation. `False`: no bounded footprint
    /// exists for this shapes graph, the run fell back to a FULL validation of the
    /// mutated graph, and `conforms` means the whole graph conforms.
    #[getter]
    const fn bounded(&self) -> bool {
        self.scope.is_bounded()
    }

    /// How many focus nodes the change was expanded into, or `None` when the
    /// footprint could not be bounded and the whole graph was validated.
    ///
    /// `None` rather than the graph's node count on the fallback path: "every focus
    /// node in the graph" and a number are different statements, and collapsing them
    /// would make a fallback indistinguishable from a large bounded expansion.
    #[getter]
    const fn focus_nodes(&self) -> Option<usize> {
        self.scope.focus_nodes()
    }

    /// Which construct made this shapes graph's change footprint unbounded, or
    /// `None` when it was bounded.
    ///
    /// Actionable rather than decorative: it names what to change to get incremental
    /// validation back.
    #[getter]
    const fn reason(&self) -> Option<&'static str> {
        self.scope.reason()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let conforms = self.report.borrow(py).inner.conforms;
        match self.scope {
            engine::ChangeScope::Bounded { focus_nodes } => {
                format!("<ChangeValidation bounded focus_nodes={focus_nodes} conforms={conforms}>")
            }
            engine::ChangeScope::Everything { reason } => {
                format!("<ChangeValidation everything reason={reason} conforms={conforms}>")
            }
        }
    }
}

/// A prepared product whose envelope has been verified and whose self-description has
/// been decoded — but which has NOT been admitted.
///
/// Holding one is a statement about framing and integrity, never about fitness: the
/// bytes are a well-formed product of this format, and nothing has yet claimed they
/// are a product this build may execute. That separation is what makes
/// `identity_components()` useful — read what a product says it was compiled from in
/// order to decide what to do about it, without any of it reaching a validator.
///
/// This value OWNS the bytes it was opened from. The Rust view borrows its input, and
/// a Python object cannot hold a borrow of a `bytes` it does not own, so each method
/// re-opens the owned buffer. Re-opening costs the container's cheap integrity tier —
/// the framing and the section digests — never the canonicalization `certify()`
/// performs.
#[pyclass(name = "ShapesProduct")]
#[derive(Debug)]
pub struct PyShapesProduct {
    bytes: Vec<u8>,
}

#[pymethods]
impl PyShapesProduct {
    /// Open `data` as a prepared product: verify the envelope and decode the
    /// product's self-description, admitting nothing.
    ///
    /// Raises `ShapesProductError` with a structural `.dimension` when the bytes are
    /// not a well-formed product of this format.
    #[staticmethod]
    fn open(py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        // Opened here and discarded: this is the eager check that makes holding a
        // `ShapesProduct` mean something. Every later method re-opens the owned copy.
        py.detach(|| purrdf_validate::explain_shapes_product(data))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(Self {
            bytes: data.to_vec(),
        })
    }

    /// The product's own bytes, exactly as opened.
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.bytes)
    }

    /// Everything the product says about itself, as deterministic `key value` lines:
    /// the container format version, the preparation stage id and whether this build
    /// knows it, the identity digest and every labelled identity component, then the
    /// recorded base, `sh:shapesGraph` IRI and prefix map.
    ///
    /// This is what makes a named refusal actionable: a restore refused on
    /// `prefixes` is answered by reading which prefix map the product actually
    /// carries, not by guessing.
    fn explain(&self, py: Python<'_>) -> PyResult<String> {
        let bytes = &self.bytes;
        py.detach(|| purrdf_validate::explain_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))
    }

    /// The container format version these bytes were written under.
    fn format_version(&self, py: Python<'_>) -> PyResult<u32> {
        self.explain_field(py, "format-version")?
            .parse()
            .map_err(|_| {
                pyo3::exceptions::PyValueError::new_err(
                    "this build rendered a non-numeric product format version",
                )
            })
    }

    /// The 32-byte preparation stage id the product declares, as lowercase hex.
    fn stage_id(&self, py: Python<'_>) -> PyResult<String> {
        self.explain_field(py, "stage-id")
    }

    /// Whether this build knows the product's preparation stage.
    ///
    /// `False` says `admit()` will refuse these bytes and `rebuild()` is the path
    /// that still restores them — the memo was written against a model this build no
    /// longer has, and the shapes dataset the product carries is what rescues it.
    fn stage_known(&self, py: Python<'_>) -> PyResult<bool> {
        Ok(self.explain_field(py, "stage-known")? == "true")
    }

    /// The SHA-256 digest of the product's input binding, as lowercase hex.
    fn identity_digest(&self, py: Python<'_>) -> PyResult<String> {
        self.explain_field(py, "identity-digest")
    }

    /// The ordered, labelled components of the product's input binding as
    /// `(label, rendered_value)` pairs — which inputs it was compiled from.
    ///
    /// The ORDER is the identity, not the labels: two components may share a label
    /// and still be different components, so this is a list of pairs rather than a
    /// dict.
    fn identity_components<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let rendered = self.explain(py)?;
        let out = PyList::empty(py);
        for line in rendered.lines() {
            let Some(rest) = line.strip_prefix("identity ") else {
                continue;
            };
            // `splitn(2, …)` rather than a split on every space: an identity label is a
            // kebab-case constant and never contains one, but a rendered VALUE may, so
            // everything after the first space is the value.
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }
            out.append(PyTuple::new(py, parts)?)?;
        }
        Ok(out)
    }

    /// **The common path.** Restore the preparation from the product's memo,
    /// re-deriving nothing expensive.
    ///
    /// The product's stage id, profile and complete input binding are checked before
    /// any of it reaches a validator. Raises `ShapesProductError` — read
    /// `.dimension` to decide what to do: `stage-id` means try `rebuild()`, an
    /// identity dimension means this process's configuration is not the one the
    /// product was prepared against.
    fn admit(&self, py: Python<'_>) -> PyResult<PyPreparedShapes> {
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::admit_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The common path, bound to the product you MEANT.** Restore exactly as
    /// `admit()` does, but only after confirming the product's input binding is
    /// `expected_identity`.
    ///
    /// Everything `admit()` checks is a question about this PROCESS — its build, its
    /// registries, its class analysis. None of them asks whether these are the bytes
    /// the caller wanted, because nothing in a product states which product was
    /// meant. Admitting a cache entry, a downloaded artifact or a path built from a
    /// configuration string without saying which one it must be is how a validator
    /// returns a well-formed report about a shapes graph nobody asked about.
    ///
    /// `expected_identity` is the 64 hexadecimal digits `identity_digest()` returns
    /// for the product you intend — one spelling, readable off the artifact itself,
    /// so the selector can be pinned in a test or a deployment manifest.
    ///
    /// Raises `ShapesProductError` with `.dimension == "shapes-graph"` when the
    /// product carries a different binding, and `ValueError` when
    /// `expected_identity` is not 64 hexadecimal digits — no product was inspected in
    /// that case, so no dimension names it.
    fn admit_expecting(
        &self,
        py: Python<'_>,
        expected_identity: &str,
    ) -> PyResult<PyPreparedShapes> {
        let expected = purrdf_validate::parse_identity_digest(expected_identity)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::admit_shapes_product_expecting(bytes, &expected))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The forward-compatibility path.** Ignore the memo and re-derive the
    /// preparation from the shapes dataset the product carries.
    ///
    /// Not a Turtle fallback: no RDF text is parsed and no file is read. The dataset
    /// travels inside the product under the envelope's own digests, and this
    /// re-derives the shapes from it under the product's recorded parse inputs.
    fn rebuild(&self, py: Python<'_>) -> PyResult<PyPreparedShapes> {
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::rebuild_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The forward-compatibility path, bound to the product you MEANT.**
    /// Re-derive the preparation exactly as `rebuild()` does, but only after
    /// confirming the product's input binding is `expected_identity`.
    ///
    /// The rescue `rebuild()` performs is not a reason to stop asking whether
    /// this is the artifact the caller wanted: a cache entry from another
    /// build, or a product a deployment placed on disk under a stage id this
    /// build does not recognize, is still just a file that could be the wrong
    /// one. The same 32-byte comparison `admit_expecting()` runs FIRST also runs
    /// first here, ahead of the re-derivation, for the identical reason.
    ///
    /// `expected_identity` carries the same meaning it does on
    /// `admit_expecting()` — the 64 hexadecimal digits `identity_digest()`
    /// returns for the product you intend.
    ///
    /// Raises `ShapesProductError` with `.dimension == "shapes-graph"` when the
    /// product carries a different binding, and `ValueError` when
    /// `expected_identity` is not 64 hexadecimal digits — no product was
    /// inspected in that case, so no dimension names it.
    fn rebuild_expecting(
        &self,
        py: Python<'_>,
        expected_identity: &str,
    ) -> PyResult<PyPreparedShapes> {
        let expected = purrdf_validate::parse_identity_digest(expected_identity)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        let bytes = &self.bytes;
        let inner = py
            .detach(|| purrdf_validate::rebuild_shapes_product_expecting(bytes, &expected))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))?;
        Ok(PyPreparedShapes { inner })
    }

    /// **The cold path.** Independently corroborate the shapes dataset's canonical
    /// identity against the one this product's binding claims.
    ///
    /// Canonicalization is a graph-isomorphism computation over the shapes graph's
    /// blank nodes and can cost more than the shapes parse a product exists to
    /// eliminate, so it is never on a restore path. Call it from a build step, a
    /// conformance harness or a test — not before every validation.
    fn certify(&self, py: Python<'_>) -> PyResult<()> {
        let bytes = &self.bytes;
        py.detach(|| purrdf_validate::certify_shapes_product(bytes))
            .map_err(|error| product_error(py, &ShapesProductRefusal::Admission(error)))
    }
}

impl PyShapesProduct {
    /// The value of the first `key value` line in the shared rendering whose key is
    /// `key`.
    ///
    /// Every scalar accessor reads the ONE rendering rather than opening the product
    /// a second way, so this class cannot report two independently-derived answers
    /// about one product.
    fn explain_field(&self, py: Python<'_>, key: &str) -> PyResult<String> {
        let rendered = self.explain(py)?;
        rendered
            .lines()
            .find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|rest| rest.strip_prefix(' '))
                    .map(ToOwned::to_owned)
            })
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "this product's rendering carries no `{key}` line; that is a defect in \
                     this build rather than in the product"
                ))
            })
    }
}

/// Compile a Turtle shapes graph into a prepared product in one call.
///
/// The composition of `Shapes(shapes_ttl, base=shapes_base).prepare().to_product()`,
/// offered because packing a document is the common case and the three-step spelling
/// is a preparation nobody keeps.
///
/// `shapes_base` is the base IRI the shapes document's relative IRI references
/// resolve against, RECORDED in the product so a restore resolves them identically
/// without the document.
///
/// `imports` is the shapes graph's `owl:imports` table (see the [module documentation](self)); the
/// product carries the merged closure, so a restore needs no documents.
///
/// Raises `ShapesImportError` — the same refusal `validate` raises — when the shapes
/// graph's `owl:imports` closure is not in hand; otherwise `ShapesProductError`, whose
/// `.dimension` is `None` when the shapes document itself did not parse.
#[pyfunction]
#[pyo3(signature = (shapes_ttl, *, shapes_base=None, imports=Vec::new()))]
#[allow(clippy::needless_pass_by_value)] // binding ABI receives owned values
fn pack_product<'py>(
    py: Python<'py>,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    imports: Vec<(String, String)>,
) -> PyResult<Bound<'py, PyBytes>> {
    let pairs = crate::py_entail::import_list(&imports);
    let bytes = py
        .detach(|| purrdf_validate::pack_shapes_product(shapes_ttl, shapes_base, &pairs))
        .map_err(|refusal| match refusal.import_error() {
            Some(error) => import_error(py, error),
            None => product_error(py, &refusal),
        })?;
    Ok(PyBytes::new(py, &bytes))
}

// ── The shapes graph's owl:imports ──────────────────────────────────────────

create_exception!(
    shacl,
    ShapesImportError,
    pyo3::exceptions::PyValueError,
    "A shapes graph's `owl:imports` closure is not in hand, or the `imports` table \
     cannot be used — the one refusal every shapes-graph entry point raises, on every \
     PurRDF host alike.\n\
     \n\
     Carries `.kind`: `unresolved-import` (the closure imports ontologies nothing in \
     hand resolves — pass their documents in `imports`), `unreached-import` (the table \
     supplies documents no import names, which would be read and never used) or \
     `invalid-import` (a key that is not an absolute IRI, a key named twice, or a \
     document that is not Turtle); and `.iris`, the IRIs it names. PurRDF fetches \
     nothing. Branch on `.kind`, never on `str(exc)`.\n\
     \n\
     Subclasses `ValueError`, so code that already catches the SHACL surface's \
     `ValueError` keeps working."
);

/// Raise a shapes-graph refusal: [`ShapesImportError`] for the import refusal, with
/// `.kind` and `.iris` always present, and `ValueError` for anything else.
fn shapes_error(py: Python<'_>, error: purrdf_validate::ShapesError) -> PyErr {
    match error {
        purrdf_validate::ShapesError::Imports(error) => import_error(py, &error),
        purrdf_validate::ShapesError::Invalid(message) => {
            pyo3::exceptions::PyValueError::new_err(message)
        }
    }
}

/// Raise `error` as [`ShapesImportError`].
fn import_error(py: Python<'_>, error: &purrdf_validate::ShapesImportError) -> PyErr {
    let raised = ShapesImportError::new_err(error.to_string());
    // Setting an attribute on a Python-level exception instance cannot fail; a failure
    // is ignored rather than replacing a precise refusal with a vaguer one.
    let _ = raised.value(py).setattr("kind", error.kind());
    let _ = raised.value(py).setattr("iris", error.iris());
    raised
}

/// Register the `purrdf-shapes` surface on a Python module.
///
/// Exposes the legacy `validate(shapes_ttl, data_nt)` function, the SHACL-AF
/// `entail(shapes_ttl, data_nt)` rule-entailment function, the shapes-graph tools
/// `apply_rules`, `eval_node_expr` and `lint_shapes`, and the reusable
/// `Shapes` / `ValidationReport` wrappers used by the Rust-native orchestration
/// in `purrdf-validate`. Called by the unified `purrdf_native` cdylib to
/// populate the `purrdf_native.shacl` submodule.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(entail, m)?)?;
    m.add_function(wrap_pyfunction!(pack_product, m)?)?;
    m.add_function(wrap_pyfunction!(apply_rules, m)?)?;
    m.add_function(wrap_pyfunction!(eval_node_expr, m)?)?;
    m.add_function(wrap_pyfunction!(lint_shapes, m)?)?;
    m.add_class::<PyShapes>()?;
    m.add_class::<PyValidationReport>()?;
    m.add_class::<PyPreparedShapes>()?;
    m.add_class::<PyChangeValidation>()?;
    m.add_class::<PyShapesProduct>()?;
    m.add(
        "ShapesProductError",
        m.py().get_type::<ShapesProductError>(),
    )?;
    m.add("ShapesImportError", m.py().get_type::<ShapesImportError>())?;
    Ok(())
}
