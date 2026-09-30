// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The RDF term object model for the `purrdf` Python extension: the
//! `NamedNode` / `BlankNode` / `Literal` / `Triple` / `Quad` / `DefaultGraph` /
//! `Variable` pyclasses, plus the Python ⇄ native-IR term converters and
//! extractors the store, query, and io seams share.
//!
//! # Native backing
//!
//! Every pyclass is backed by the `purrdf_core` owned model
//! (`RdfTerm` / `RdfLiteral` / `RdfTriple` / `RdfQuad`) plus `String` for IRI
//! predicates and variable names. The Python-facing class names, attributes
//! (`value` / `datatype` / `language` / `subject` …), and semantics form the
//! rdflib drop-in: in particular `Literal.datatype` always returns an IRI
//! (`xsd:string` for a plain literal, `rdf:langString` for a language-tagged one).

use std::fmt::Write as _;
use std::hash::BuildHasher;

use purrdf_core::langtag::identity_fold;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;

use crate::{
    BlankScope, QuadValues, RdfLiteral, RdfQuad, RdfTerm, RdfTextDirection, RdfTriple, TermValue,
};

// ── Term model ──────────────────────────────────────────────────────────────────

fn hash_str(value: &str) -> u64 {
    purrdf_core::FastHasher::default().hash_one(value)
}

/// The native RDF 1.2 expanded datatype, shared by lookup, hashing and accessors.
fn literal_datatype_iri(lit: &RdfLiteral) -> &str {
    lit.datatype_iri()
}

/// Parse the optional RDF 1.2 base-direction argument (`"ltr"`/`"rtl"`) into the
/// native [`RdfTextDirection`]. A `None` argument yields `None`; any other string is
/// rejected, mirroring the closed direction vocabulary of RDF 1.2.
fn parse_direction(direction: Option<&str>) -> PyResult<Option<RdfTextDirection>> {
    direction
        .map(|token| {
            RdfTextDirection::from_str_token(token).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "invalid base direction `{token}`: expected \"ltr\" or \"rtl\""
                ))
            })
        })
        .transpose()
}

/// An IRI node (`NamedNode`).
#[pyclass(name = "NamedNode", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyNamedNode {
    pub(crate) inner: String,
}

#[pymethods]
impl PyNamedNode {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        Ok(Self {
            inner: non_empty(value, "invalid IRI: an IRI must not be empty")?,
        })
    }

    /// The IRI string (no angle brackets).
    #[getter]
    fn value(&self) -> &str {
        &self.inner
    }

    fn __str__(&self) -> String {
        format!("<{}>", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("<NamedNode value={}>", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_str(&self.inner)
    }
}

/// A blank node (`BlankNode`).
#[pyclass(name = "BlankNode", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyBlankNode {
    pub(crate) inner: String,
}

#[pymethods]
impl PyBlankNode {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        Ok(Self {
            inner: non_empty(
                value,
                "invalid blank node: a blank-node label must not be empty",
            )?,
        })
    }

    /// The blank-node id (no `_:` prefix).
    #[getter]
    fn value(&self) -> &str {
        &self.inner
    }

    fn __str__(&self) -> String {
        format!("_:{}", self.inner)
    }

    fn __repr__(&self) -> String {
        format!("<BlankNode value={}>", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_str(&self.inner)
    }
}

/// An RDF literal (`Literal`).
#[pyclass(name = "Literal", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyLiteral {
    pub(crate) inner: RdfLiteral,
}

#[pymethods]
impl PyLiteral {
    #[new]
    #[pyo3(signature = (value, *, datatype=None, language=None, direction=None))]
    fn new(
        value: String,
        datatype: Option<&PyNamedNode>,
        language: Option<String>,
        direction: Option<&str>,
    ) -> PyResult<Self> {
        let direction = parse_direction(direction)?;
        let inner = if let Some(language) = language {
            if datatype.is_some() {
                return Err(PyValueError::new_err(
                    "a language-tagged literal cannot also carry an explicit datatype",
                ));
            }
            if language.is_empty() {
                return Err(PyValueError::new_err(
                    "invalid language tag: a language tag must not be empty",
                ));
            }
            RdfLiteral {
                lexical_form: value,
                datatype: None,
                language: Some(language),
                direction,
            }
        } else {
            // RDF 1.2: base direction is only meaningful on a language-tagged
            // literal (a `dirLangString`). Reject a bare/typed literal carrying one.
            if direction.is_some() {
                return Err(PyValueError::new_err(
                    "a base direction requires a language tag (RDF 1.2 dirLangString)",
                ));
            }
            if let Some(datatype) = datatype {
                RdfLiteral {
                    lexical_form: value,
                    datatype: Some(datatype.inner.clone()),
                    language: None,
                    direction: None,
                }
            } else {
                // A plain literal: datatype-less in the native model, surfaced as
                // `xsd:string` by the `datatype` getter.
                RdfLiteral {
                    lexical_form: value,
                    datatype: None,
                    language: None,
                    direction: None,
                }
            }
        };
        RdfLiteral::validate_components(
            inner.datatype_iri(),
            inner.language.as_deref(),
            inner.direction,
        )
        .map_err(PyValueError::new_err)?;
        Ok(Self { inner })
    }

    /// The lexical form (no datatype/language decoration).
    #[getter]
    fn value(&self) -> &str {
        &self.inner.lexical_form
    }

    /// The language tag, or `None` for a non-language-tagged literal.
    #[getter]
    fn language(&self) -> Option<&str> {
        self.inner.language.as_deref()
    }

    /// The RDF 1.2 base direction (`"ltr"`/`"rtl"`), or `None` when absent.
    #[getter]
    fn direction(&self) -> Option<&'static str> {
        self.inner.direction.map(RdfTextDirection::as_str)
    }

    /// The datatype IRI (always present — `xsd:string` for a plain literal,
    /// `rdf:langString` for a language tag, `rdf:dirLangString` with base direction).
    #[getter]
    fn datatype(&self) -> PyNamedNode {
        PyNamedNode {
            inner: literal_datatype_iri(&self.inner).to_owned(),
        }
    }

    fn __str__(&self) -> String {
        RdfTerm::Literal(self.inner.clone()).to_string()
    }

    fn __repr__(&self) -> String {
        format!("<Literal {}>", RdfTerm::Literal(self.inner.clone()))
    }

    fn __eq__(&self, other: &Self) -> bool {
        // RDF term equality over the value-space-equivalent representation: a plain
        // literal and an explicit `xsd:string` literal of the same lexical form are
        // the SAME term (a plain literal's datatype IS `xsd:string`). The native model keeps a plain
        // literal datatype-less, so normalize both sides through the datatype IRI.
        let (lex, dt, lang, direction) = literal_key(&self.inner);
        let (other_lex, other_dt, other_lang, other_direction) = literal_key(&other.inner);
        lex == other_lex
            && dt == other_dt
            && direction == other_direction
            && match (lang, other_lang) {
                (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                (None, None) => true,
                _ => false,
            }
    }

    fn __hash__(&self) -> u64 {
        hash_str(&literal_key_string(&self.inner))
    }
}

/// The RDF-term-equality key: lexical form, datatype IRI, language and direction,
/// with a plain literal's datatype normalized to `xsd:string`, so a plain literal and
/// an explicit `xsd:string` literal compare equal.
fn literal_key(lit: &RdfLiteral) -> (&str, &str, Option<&str>, Option<RdfTextDirection>) {
    (
        &lit.lexical_form,
        literal_datatype_iri(lit),
        lit.language.as_deref(),
        lit.direction,
    )
}

fn literal_key_string(lit: &RdfLiteral) -> String {
    let (lex, dt, lang, direction) = literal_key(lit);
    framed_key(&[
        lex,
        dt,
        &identity_fold(lang.unwrap_or("")),
        direction.map_or("", RdfTextDirection::as_str),
    ])
}

/// Length framing keeps embedded separator characters out of identity decisions.
fn framed_key(parts: &[&str]) -> String {
    let mut key = String::new();
    for part in parts {
        write!(key, "{}:{part}", part.len()).expect("writing to a String cannot fail");
    }
    key
}

/// A quoted triple term (RDF 1.2 / RDF-star) (`Triple`).
#[pyclass(name = "Triple", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyTriple {
    pub(crate) inner: RdfTriple,
}

#[pymethods]
impl PyTriple {
    #[new]
    fn new(
        py: Python<'_>,
        subject: &Bound<'_, PyAny>,
        predicate: &Bound<'_, PyAny>,
        object: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let _ = py;
        Ok(Self {
            inner: RdfTriple::new(
                extract_subject(subject)?,
                extract_named_node(predicate)?,
                extract_term(object)?,
            ),
        })
    }

    #[getter]
    fn subject(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        subject_to_py(py, &self.inner.subject)
    }

    #[getter]
    fn predicate(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        named_node_to_py(py, &self.inner.predicate)
    }

    #[getter]
    fn object(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        term_to_py(py, &self.inner.object)
    }

    fn __str__(&self) -> String {
        triple_term_to_string(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!("<Triple {}>", triple_term_to_string(&self.inner))
    }

    fn __eq__(&self, other: &Self) -> bool {
        triple_key(&self.inner) == triple_key(&other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_str(&triple_key(&self.inner))
    }
}

/// An RDF quad (`Quad`).
#[pyclass(name = "Quad", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyQuad {
    pub(crate) inner: RdfQuad,
}

#[pymethods]
impl PyQuad {
    #[new]
    #[pyo3(signature = (subject, predicate, object, graph_name=None))]
    fn new(
        subject: &Bound<'_, PyAny>,
        predicate: &Bound<'_, PyAny>,
        object: &Bound<'_, PyAny>,
        graph_name: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut quad = RdfQuad::new(
            extract_subject(subject)?,
            extract_named_node(predicate)?,
            extract_term(object)?,
        );
        quad.graph_name = extract_graph_name(graph_name)?;
        Ok(Self { inner: quad })
    }

    #[getter]
    fn subject(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        subject_to_py(py, &self.inner.subject)
    }

    #[getter]
    fn predicate(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        named_node_to_py(py, &self.inner.predicate)
    }

    #[getter]
    fn object(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        term_to_py(py, &self.inner.object)
    }

    #[getter]
    fn graph_name(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        graph_name_to_py(py, self.inner.graph_name.as_ref())
    }

    fn __str__(&self) -> String {
        quad_to_string(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!("<Quad {}>", quad_to_string(&self.inner))
    }

    fn __eq__(&self, other: &Self) -> bool {
        quad_key(&self.inner) == quad_key(&other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_str(&quad_key(&self.inner))
    }
}

/// A default-graph marker term (`DefaultGraph`).
#[pyclass(name = "DefaultGraph", frozen, skip_from_py_object)]
#[derive(Clone, Debug, Default)]
pub struct PyDefaultGraph;

#[pymethods]
impl PyDefaultGraph {
    #[new]
    fn new() -> Self {
        Self
    }

    fn __str__(&self) -> &'static str {
        "DEFAULT"
    }

    fn __eq__(&self, _other: &Self) -> bool {
        true
    }

    fn __hash__(&self) -> u64 {
        0
    }
}

/// A SPARQL variable, used to key query substitutions (`Variable`).
#[pyclass(name = "Variable", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyVariable {
    pub(crate) inner: String,
}

#[pymethods]
impl PyVariable {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        // The bare variable name, without the leading `?`/`$` sigil. Reject an
        // empty name and a name still carrying a sigil.
        if value.is_empty() {
            return Err(PyValueError::new_err(
                "invalid variable ``: a variable name must not be empty",
            ));
        }
        if value.starts_with('?') || value.starts_with('$') {
            return Err(PyValueError::new_err(format!(
                "invalid variable `{value}`: pass the bare name without a ?/$ sigil"
            )));
        }
        Ok(Self {
            inner: value.to_owned(),
        })
    }

    #[getter]
    fn value(&self) -> &str {
        &self.inner
    }

    fn __str__(&self) -> String {
        format!("?{}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        hash_str(&self.inner)
    }
}

// ── string forms (single source via RdfTerm Display) ────────────────────────────

fn triple_term_to_string(triple: &RdfTriple) -> String {
    RdfTerm::triple(triple.clone()).to_string()
}

/// A quad's N-Quads-style string form: `subject <predicate> object [graph]`. It is
/// the Python `Quad`'s `str()` and the deterministic sort key canonicalization
/// orders its output by, one form for both.
pub(crate) fn quad_to_string(quad: &RdfQuad) -> String {
    let triple = format!("{} <{}> {}", quad.subject, quad.predicate, quad.object);
    match &quad.graph_name {
        None => triple,
        Some(g) => format!("{triple} {g}"),
    }
}

// ── content-equality keys ─────────────────────────────────────────────────────────

/// The RDF-term-equality key of a native term. A literal is normalized through
/// [`literal_key_string`] so a plain literal and an explicit `xsd:string` literal of
/// the same lexical form compare equal; every other term keys on its canonical
/// string form.
fn term_key(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Literal(lit) => format!("L\u{1}{}", literal_key_string(lit)),
        RdfTerm::Triple(t) => format!("T\u{1}{}", triple_key(t)),
        other => other.to_string(),
    }
}

fn triple_key(triple: &RdfTriple) -> String {
    framed_key(&[
        &term_key(&triple.subject),
        &triple.predicate,
        &term_key(&triple.object),
    ])
}

fn quad_key(quad: &RdfQuad) -> String {
    framed_key(&[
        &term_key(&quad.subject),
        &quad.predicate,
        &term_key(&quad.object),
        &quad.graph_name.as_ref().map_or(String::new(), term_key),
    ])
}

// ── cross-crate constructors ──────────────────────────────────────────────────────

/// Build a Python `Quad` object from a native [`RdfQuad`].
///
/// Cross-crate constructor for the engine crates that produce quads natively (the
/// RL closure in `purrdf-logic`): they assemble a native `RdfQuad` and
/// hand Python a live `purrdf.Quad` directly, so the closure result never makes
/// a round-trip through an intermediate N-Triples string the Python side has to
/// re-parse. The returned object is the same `PyQuad` the parser/SPARQL surface
/// yields, so downstream code (rdflib adapters, comparators) treats it uniformly.
#[allow(dead_code)]
pub(super) fn quad_to_py(py: Python<'_>, quad: &RdfQuad) -> PyResult<Py<PyAny>> {
    Ok(Py::new(
        py,
        PyQuad {
            inner: quad.clone(),
        },
    )?
    .into_any())
}

/// Build the live `purrdf.Quad` list for every (flattened) quad of a native
/// [`RdfDataset`](crate::RdfDataset) — the cross-crate entry point for
/// engine crates (e.g. `purrdf-logic`'s RL closure) that produce a
/// frozen IR dataset and must hand Python live quad objects without naming any
/// binding type themselves.
///
/// The dataset is flattened to the source-faithful flat quad stream (base quads plus
/// the re-materialized RDF 1.2 statement layer), then each quad becomes a `PyQuad`.
///
/// # Errors
///
/// Returns a Python error if the dataset cannot be flattened into quads.
#[allow(dead_code)]
pub(super) fn dataset_quads_to_py(
    py: Python<'_>,
    dataset: &crate::RdfDataset,
) -> PyResult<Vec<Py<PyAny>>> {
    let quads = crate::flat_rdf_quads_from_dataset(dataset);
    let mut out: Vec<Py<PyAny>> = Vec::with_capacity(quads.len());
    for quad in &quads {
        out.push(quad_to_py(py, quad)?);
    }
    Ok(out)
}

// ── Term ⇄ Python conversions ────────────────────────────────────────────────────

pub(crate) fn term_to_py(py: Python<'_>, term: &RdfTerm) -> PyResult<Py<PyAny>> {
    Ok(match term {
        RdfTerm::Iri(n) => named_node_to_py(py, n)?,
        RdfTerm::BlankNode(b) => Py::new(py, PyBlankNode { inner: b.clone() })?.into_any(),
        RdfTerm::Literal(l) => Py::new(py, PyLiteral { inner: l.clone() })?.into_any(),
        RdfTerm::Triple(t) => Py::new(
            py,
            PyTriple {
                inner: (**t).clone(),
            },
        )?
        .into_any(),
    })
}

/// `value` owned, or `ValueError(message)` when it is empty: the one emptiness
/// refusal the `NamedNode` and `BlankNode` constructors share (an IRI and a
/// blank-node label are both non-empty by grammar).
fn non_empty(value: &str, message: &'static str) -> PyResult<String> {
    if value.is_empty() {
        return Err(PyValueError::new_err(message));
    }
    Ok(value.to_owned())
}

/// A fresh Python `NamedNode` for `iri`: the one constructor every quad/triple
/// accessor that hands out an IRI position uses.
fn named_node_to_py(py: Python<'_>, iri: &str) -> PyResult<Py<PyAny>> {
    Ok(Py::new(
        py,
        PyNamedNode {
            inner: iri.to_owned(),
        },
    )?
    .into_any())
}

fn subject_to_py(py: Python<'_>, subject: &RdfTerm) -> PyResult<Py<PyAny>> {
    match subject {
        RdfTerm::Iri(n) => named_node_to_py(py, n),
        RdfTerm::BlankNode(b) => Ok(Py::new(py, PyBlankNode { inner: b.clone() })?.into_any()),
        _ => Err(PyTypeError::new_err(
            "a subject must be a NamedNode or BlankNode",
        )),
    }
}

fn graph_name_to_py(py: Python<'_>, graph_name: Option<&RdfTerm>) -> PyResult<Py<PyAny>> {
    match graph_name {
        None => Ok(Py::new(py, PyDefaultGraph)?.into_any()),
        Some(RdfTerm::Iri(n)) => named_node_to_py(py, n),
        Some(RdfTerm::BlankNode(b)) => {
            Ok(Py::new(py, PyBlankNode { inner: b.clone() })?.into_any())
        }
        Some(_) => Err(PyTypeError::new_err(
            "a graph name must be a NamedNode, BlankNode, or DefaultGraph",
        )),
    }
}

pub(crate) fn extract_term(obj: &Bound<'_, PyAny>) -> PyResult<RdfTerm> {
    if let Ok(n) = obj.cast::<PyNamedNode>() {
        return Ok(RdfTerm::Iri(n.get().inner.clone()));
    }
    if let Ok(b) = obj.cast::<PyBlankNode>() {
        return Ok(RdfTerm::BlankNode(b.get().inner.clone()));
    }
    if let Ok(l) = obj.cast::<PyLiteral>() {
        return Ok(RdfTerm::Literal(l.get().inner.clone()));
    }
    if let Ok(t) = obj.cast::<PyTriple>() {
        return Ok(RdfTerm::triple(t.get().inner.clone()));
    }
    Err(PyTypeError::new_err(
        "expected an RDF term (NamedNode, BlankNode, Literal, or Triple)",
    ))
}

/// Convert a Python term object straight to the value model, under the default
/// blank scope: [`extract_term`] followed by [`rdf_term_to_value`].
///
/// The one converter every seam that hands Python-authored terms to the engine goes
/// through — a substitution value, a property-function relation cell, a relation
/// table head — so a literal, an IRI, a blank node, and a triple term all reach the
/// engine as the same [`TermValue`] whichever keyword carried them.
pub(super) fn extract_term_value(obj: &Bound<'_, PyAny>) -> PyResult<TermValue> {
    Ok(rdf_term_to_value(&extract_term(obj)?))
}

/// Convert a native owned [`RdfTerm`] into the `MutableDataset` [`TermValue`] model:
/// [`TermValue::from_rdf_term`], which decodes a surfaced `purrdfesc{n}_{body}` scope
/// envelope back into its `(label, scope)` pair, so a blank node round-tripped through
/// Python matches the stored node.
pub(super) fn rdf_term_to_value(term: &RdfTerm) -> TermValue {
    TermValue::from_rdf_term(term)
}

/// Convert a native owned [`RdfQuad`] into [`QuadValues`], tagging every blank node
/// with `scope` (the per-load isolation scope) through
/// [`TermValue::from_rdf_term_in_scope`].
pub(super) fn rdf_quad_to_values_scoped(quad: &RdfQuad, scope: BlankScope) -> QuadValues {
    QuadValues {
        s: TermValue::from_rdf_term_in_scope(&quad.subject, scope),
        p: TermValue::Iri(quad.predicate.clone()),
        o: TermValue::from_rdf_term_in_scope(&quad.object, scope),
        g: quad
            .graph_name
            .as_ref()
            .map(|g| TermValue::from_rdf_term_in_scope(g, scope)),
    }
}

/// Convert a native owned [`RdfQuad`] into the `MutableDataset` [`QuadValues`] model
/// under the default blank scope.
pub(super) fn rdf_quad_to_values(quad: &RdfQuad) -> QuadValues {
    rdf_quad_to_values_scoped(quad, BlankScope::DEFAULT)
}

/// Convert a stored [`QuadValues`] back into the native owned [`RdfQuad`] model through
/// [`TermValue::to_rdf_term`]: a scoped blank label is qualified, so a per-load scope
/// is reflected in the surfaced label.
///
/// # Panics
///
/// On a quad no dataset stores: a predicate that is not an IRI, or a triple term
/// whose predicate is not one. Every stored quad came in as an [`RdfQuad`], whose
/// predicates are IRIs by construction.
pub(super) fn values_to_rdf_quad(values: &QuadValues) -> RdfQuad {
    let owned = |value: &TermValue| {
        value
            .to_rdf_term()
            .expect("a stored term has an owned form")
    };
    let predicate = values
        .p
        .as_iri()
        .expect("a stored quad's predicate is an IRI");
    let mut quad = RdfQuad::new(owned(&values.s), predicate, owned(&values.o));
    quad.graph_name = values.g.as_ref().map(owned);
    quad
}

/// Coerce a Python term to an RDF 1.2 subject. RDF 1.2 (unlike the obsolete
/// RDF-star) allows triple terms in the OBJECT position only — a subject is an
/// IRI or blank node, never a quoted triple. A `Triple` therefore reaches
/// `extract_term`, not here.
fn extract_subject(obj: &Bound<'_, PyAny>) -> PyResult<RdfTerm> {
    if let Ok(n) = obj.cast::<PyNamedNode>() {
        return Ok(RdfTerm::Iri(n.get().inner.clone()));
    }
    if let Ok(b) = obj.cast::<PyBlankNode>() {
        return Ok(RdfTerm::BlankNode(b.get().inner.clone()));
    }
    Err(PyTypeError::new_err(
        "a subject must be a NamedNode or BlankNode \
         (RDF 1.2 triple terms are object-position only)",
    ))
}

fn extract_named_node(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    obj.cast::<PyNamedNode>()
        .map(|n| n.get().inner.clone())
        .map_err(|_| PyTypeError::new_err("a predicate must be a NamedNode"))
}

/// Coerce a Python graph-name slot to the native optional graph term: `None` /
/// `DefaultGraph` → the default graph (`None`), a `NamedNode`/`BlankNode` → that term.
pub(crate) fn extract_graph_name(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<RdfTerm>> {
    let Some(obj) = obj else {
        return Ok(None);
    };
    if obj.is_none() || obj.cast::<PyDefaultGraph>().is_ok() {
        return Ok(None);
    }
    if let Ok(n) = obj.cast::<PyNamedNode>() {
        return Ok(Some(RdfTerm::Iri(n.get().inner.clone())));
    }
    if let Ok(b) = obj.cast::<PyBlankNode>() {
        return Ok(Some(RdfTerm::BlankNode(b.get().inner.clone())));
    }
    Err(PyTypeError::new_err(
        "a graph name must be a NamedNode, BlankNode, or DefaultGraph",
    ))
}
