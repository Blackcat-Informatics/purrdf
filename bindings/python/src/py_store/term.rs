// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF term object model for the `purrdf` Python extension: the
//! `NamedNode` / `BlankNode` / `Literal` / `Triple` / `Quad` / `DefaultGraph` /
//! `Variable` pyclasses, plus the Python ⇄ native-IR term converters and
//! extractors the store, query, and io seams share.
//!
//! # Native backing (EPIC #906)
//!
//! Every pyclass is backed by the oxigraph-free `purrdf_core` owned model
//! (`RdfTerm` / `RdfLiteral` / `RdfTriple` / `RdfQuad`) plus `String` for IRI
//! predicates and variable names — never `oxigraph::model::*`. The Python-facing
//! class names, attributes (`value` / `datatype` / `language` / `subject` …), and
//! semantics are IDENTICAL to the prior oxigraph-backed surface (this is the
//! rdflib drop-in): in particular `Literal.datatype` always returns an IRI
//! (`xsd:string` for a plain literal, `rdf:langString` for a language-tagged one),
//! matching the oxigraph Python `Literal` API the codebase relies on.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;

use crate::{RdfLiteral, RdfQuad, RdfTerm, RdfTextDirection, RdfTriple};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

// ── Term model ──────────────────────────────────────────────────────────────────

fn hash_str(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// The datatype IRI of a native literal, expanded per the oxigraph Python `Literal`
/// API: a plain (datatype-less) literal reports `xsd:string`, a language-tagged one
/// reports `rdf:langString`, and a typed one reports its explicit datatype.
fn literal_datatype_iri(lit: &RdfLiteral) -> &str {
    match (&lit.datatype, &lit.language) {
        (Some(dt), _) => dt.as_str(),
        (None, Some(_)) => RDF_LANG_STRING,
        (None, None) => XSD_STRING,
    }
}

/// Parse the optional RDF 1.2 base-direction argument (`"ltr"`/`"rtl"`) into the
/// native [`RdfTextDirection`]. A `None` argument yields `None`; any other string is
/// rejected, mirroring the closed direction vocabulary of RDF 1.2.
fn parse_direction(direction: Option<&str>) -> PyResult<Option<RdfTextDirection>> {
    match direction {
        None => Ok(None),
        Some("ltr") => Ok(Some(RdfTextDirection::Ltr)),
        Some("rtl") => Ok(Some(RdfTextDirection::Rtl)),
        Some(other) => Err(PyValueError::new_err(format!(
            "invalid base direction `{other}`: expected \"ltr\" or \"rtl\""
        ))),
    }
}

/// An IRI node. Mirrors the oxigraph Python `NamedNode`.
#[pyclass(name = "NamedNode", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyNamedNode {
    pub(crate) inner: String,
}

#[pymethods]
impl PyNamedNode {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        if value.is_empty() {
            return Err(PyValueError::new_err(
                "invalid IRI: an IRI must not be empty",
            ));
        }
        Ok(Self {
            inner: value.to_owned(),
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

/// A blank node. Mirrors the oxigraph Python `BlankNode`.
#[pyclass(name = "BlankNode", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyBlankNode {
    pub(crate) inner: String,
}

#[pymethods]
impl PyBlankNode {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        if value.is_empty() {
            return Err(PyValueError::new_err(
                "invalid blank node: a blank-node label must not be empty",
            ));
        }
        Ok(Self {
            inner: value.to_owned(),
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

/// An RDF literal. Mirrors the oxigraph Python `Literal`.
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
                // `xsd:string` by the `datatype` getter (oxigraph Python parity).
                RdfLiteral {
                    lexical_form: value,
                    datatype: None,
                    language: None,
                    direction: None,
                }
            }
        };
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
    /// `rdf:langString` for a language-tagged one), matching the oxigraph Python API.
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
        // the SAME term (matching the prior oxigraph `Literal` equality, where a
        // plain literal's datatype IS `xsd:string`). The native model keeps a plain
        // literal datatype-less, so normalize both sides through the datatype IRI.
        literal_key(&self.inner) == literal_key(&other.inner)
    }

    fn __hash__(&self) -> u64 {
        hash_str(&literal_key_string(&self.inner))
    }
}

/// The RDF-term-equality key of a native literal: `(lexical, datatype-IRI, language)`
/// with a plain literal's datatype normalized to `xsd:string`, so a plain literal and
/// an explicit `xsd:string` literal compare equal (oxigraph `Literal` parity).
fn literal_key(lit: &RdfLiteral) -> (&str, &str, Option<&str>) {
    (
        &lit.lexical_form,
        literal_datatype_iri(lit),
        lit.language.as_deref(),
    )
}

fn literal_key_string(lit: &RdfLiteral) -> String {
    let (lex, dt, lang) = literal_key(lit);
    format!("{lex}\u{1}{dt}\u{1}{}", lang.unwrap_or(""))
}

/// A quoted triple term (RDF 1.2 / RDF-star). Mirrors the oxigraph Python `Triple`.
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
        Py::new(
            py,
            PyNamedNode {
                inner: self.inner.predicate.clone(),
            },
        )
        .map(Py::into_any)
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
        hash_str(&triple_term_to_string(&self.inner))
    }
}

/// An RDF quad. Mirrors the oxigraph Python `Quad`.
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
        Py::new(
            py,
            PyNamedNode {
                inner: self.inner.predicate.clone(),
            },
        )
        .map(Py::into_any)
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
        hash_str(&quad_to_string(&self.inner))
    }
}

/// A default-graph marker term. Mirrors the oxigraph Python `DefaultGraph`.
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

/// A SPARQL variable, used to key query substitutions. Mirrors
/// the oxigraph Python `Variable`.
#[pyclass(name = "Variable", frozen, skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyVariable {
    pub(crate) inner: String,
}

#[pymethods]
impl PyVariable {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        // The bare variable name, without the leading `?`/`$` sigil (oxigraph
        // `Variable::new` parity). Reject an empty name and a name still carrying a
        // sigil, the two cases the oxigraph constructor rejected.
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

// ── string forms (oxigraph Display parity, single source via RdfTerm Display) ─────

fn triple_term_to_string(triple: &RdfTriple) -> String {
    RdfTerm::triple(triple.clone()).to_string()
}

fn quad_to_string(quad: &RdfQuad) -> String {
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
    format!(
        "{}\u{2}{}\u{2}{}",
        term_key(&triple.subject),
        triple.predicate,
        term_key(&triple.object)
    )
}

fn quad_key(quad: &RdfQuad) -> String {
    format!(
        "{}\u{3}{}",
        triple_key(&RdfTriple::new(
            quad.subject.clone(),
            quad.predicate.clone(),
            quad.object.clone(),
        )),
        quad.graph_name.as_ref().map_or(String::new(), term_key)
    )
}

// ── cross-crate constructors ──────────────────────────────────────────────────────

/// Build a Python `Quad` object from a native [`RdfQuad`].
///
/// Cross-crate constructor for the engine crates that produce quads natively (the
/// RL closure in `purrdf-logic`, issue #630): they assemble a native `RdfQuad` and
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
/// [`RdfDataset`](crate::RdfDataset) — the oxigraph-free cross-crate entry point for
/// engine crates (`purrdf-logic`'s RL closure, #630 / EPIC #906) that produce a
/// frozen IR dataset and must hand Python live quad objects without naming any
/// oxigraph type themselves.
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
        RdfTerm::Iri(n) => Py::new(py, PyNamedNode { inner: n.clone() })?.into_any(),
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

fn subject_to_py(py: Python<'_>, subject: &RdfTerm) -> PyResult<Py<PyAny>> {
    match subject {
        RdfTerm::Iri(n) => Ok(Py::new(py, PyNamedNode { inner: n.clone() })?.into_any()),
        RdfTerm::BlankNode(b) => Ok(Py::new(py, PyBlankNode { inner: b.clone() })?.into_any()),
        _ => Err(PyTypeError::new_err(
            "a subject must be a NamedNode or BlankNode",
        )),
    }
}

fn graph_name_to_py(py: Python<'_>, graph_name: Option<&RdfTerm>) -> PyResult<Py<PyAny>> {
    match graph_name {
        None => Ok(Py::new(py, PyDefaultGraph)?.into_any()),
        Some(RdfTerm::Iri(n)) => Ok(Py::new(py, PyNamedNode { inner: n.clone() })?.into_any()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_explicit_xsd_string_literals_are_equal_terms() {
        // RDF term equality: a plain literal and an explicit `xsd:string` literal of
        // the same lexical form are the SAME term (oxigraph `Literal` parity — a
        // plain literal's datatype IS `xsd:string`).
        let plain = PyLiteral {
            inner: RdfLiteral::simple("Alice"),
        };
        let explicit = PyLiteral {
            inner: RdfLiteral::typed("Alice", XSD_STRING),
        };
        assert!(plain.__eq__(&explicit));
        assert_eq!(plain.__hash__(), explicit.__hash__());
        // The datatype getter expands a plain literal to `xsd:string`.
        assert_eq!(plain.datatype().inner, XSD_STRING);
    }

    #[test]
    fn lang_literal_reports_rdf_langstring_datatype() {
        let lit = PyLiteral {
            inner: RdfLiteral::language_tagged("hi", "en"),
        };
        assert_eq!(lit.language(), Some("en"));
        assert_eq!(lit.datatype().inner, RDF_LANG_STRING);
    }

    #[test]
    fn typed_literal_keeps_its_datatype_and_differs_from_plain() {
        let int_dt = "http://www.w3.org/2001/XMLSchema#integer";
        let typed = PyLiteral {
            inner: RdfLiteral::typed("1", int_dt),
        };
        let plain = PyLiteral {
            inner: RdfLiteral::simple("1"),
        };
        assert_eq!(typed.datatype().inner, int_dt);
        assert!(!typed.__eq__(&plain));
    }

    #[test]
    fn direction_parses_and_round_trips_on_lang_literal() {
        // RDF 1.2 base direction: a `dirLangString` carries a language tag AND a
        // direction; the getter surfaces the closed `ltr`/`rtl` vocabulary.
        let lit = PyLiteral::new("مرحبا".to_owned(), None, Some("ar".to_owned()), Some("rtl"))
            .expect("dirLangString constructs");
        assert_eq!(lit.language(), Some("ar"));
        assert_eq!(lit.direction(), Some("rtl"));

        // A direction without a language tag is rejected (not a dirLangString).
        assert!(PyLiteral::new("x".to_owned(), None, None, Some("ltr")).is_err());
        // An unknown direction token is rejected (closed vocabulary).
        assert!(
            PyLiteral::new("x".to_owned(), None, Some("en".to_owned()), Some("up")).is_err()
        );
        // A plain/typed literal reports no direction.
        let plain = PyLiteral::new("x".to_owned(), None, None, None).expect("plain constructs");
        assert_eq!(plain.direction(), None);
    }

    #[test]
    fn named_node_str_and_value() {
        let n = PyNamedNode::new("https://example.org/s").unwrap();
        assert_eq!(n.value(), "https://example.org/s");
        assert_eq!(n.__str__(), "<https://example.org/s>");
    }

    #[test]
    fn rdf12_triple_terms_are_object_position_only() {
        // RDF 1.2 (unlike obsolete RDF-star) permits quoted triples in the OBJECT
        // slot only; a subject is always an IRI or blank node. A quoted-triple
        // object round-trips through the native model.
        let inner = RdfTriple::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        );
        let quad = RdfQuad::new(
            RdfTerm::iri("https://example.org/r"),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies",
            RdfTerm::triple(inner),
        );
        assert!(matches!(quad.object, RdfTerm::Triple(_)));
        assert!(matches!(quad.subject, RdfTerm::Iri(_)));
    }
}
