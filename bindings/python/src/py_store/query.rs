// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL result model for the `purrdf` Python extension: the materialized
//! `QuerySolutions` / `QuerySolution` (SELECT), `QueryTriples` (CONSTRUCT), and
//! `QueryBoolean` (ASK) pyclasses, plus the `materialize_results` adapter the
//! store seam uses to turn a native [`SparqlResult`] into these objects.
//!
//! Native backing (EPIC #906): solution cells are `purrdf_core::TermValue`,
//! CONSTRUCT triples are `RdfTriple`. The engine is `NativeSparqlEngine`; the
//! oxigraph `QueryResults` type is gone from this surface.

use pyo3::exceptions::{PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

use super::io::{serialize_triples, PyRdfFormat};
use super::term::{term_to_py, PyTriple, PyVariable};
use crate::{RdfDataset, RdfTerm, RdfTriple, SparqlResult, TermValue};

/// SELECT results, materialized. Mirrors the oxigraph Python `QuerySolutions`.
#[pyclass(name = "QuerySolutions")]
#[derive(Debug)]
pub struct PyQuerySolutions {
    variables: Vec<String>,
    rows: Vec<Vec<Option<RdfTerm>>>,
    pos: usize,
}

#[pymethods]
impl PyQuerySolutions {
    /// The bound variables, in projection order.
    #[getter]
    fn variables(&self, py: Python<'_>) -> PyResult<Vec<Py<PyVariable>>> {
        self.variables
            .iter()
            .map(|v| Py::new(py, PyVariable { inner: v.clone() }))
            .collect()
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(
        mut slf: PyRefMut<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Option<Py<PyQuerySolution>>> {
        if slf.pos >= slf.rows.len() {
            return Ok(None);
        }
        let row = slf.rows[slf.pos].clone();
        let variables = slf.variables.clone();
        slf.pos += 1;
        Ok(Some(Py::new(py, PyQuerySolution { variables, row })?))
    }

    fn __len__(&self) -> usize {
        self.rows.len()
    }
}

/// A single SELECT solution row. Mirrors the oxigraph Python `QuerySolution`.
#[pyclass(name = "QuerySolution")]
#[derive(Debug)]
pub struct PyQuerySolution {
    variables: Vec<String>,
    row: Vec<Option<RdfTerm>>,
}

#[pymethods]
impl PyQuerySolution {
    /// Look a binding up by variable name (`str`), `Variable`, or position
    /// (`int`). An unbound variable yields `None`; an unknown name is a
    /// `KeyError`, matching the oxigraph Python API.
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
        let index = if let Ok(i) = key.extract::<usize>() {
            if i >= self.row.len() {
                return Err(PyKeyError::new_err(format!("no variable at position {i}")));
            }
            i
        } else {
            let name = if let Ok(var) = key.cast::<PyVariable>() {
                var.get().inner.clone()
            } else if let Ok(s) = key.cast::<PyString>() {
                s.to_str()?.to_owned()
            } else {
                return Err(PyTypeError::new_err(
                    "solution key must be a str, Variable, or int",
                ));
            };
            self.variables
                .iter()
                .position(|v| v.as_str() == name)
                .ok_or_else(|| PyKeyError::new_err(format!("no variable named `{name}`")))?
        };
        match &self.row[index] {
            Some(term) => Ok(Some(term_to_py(py, term)?)),
            None => Ok(None),
        }
    }
}

/// CONSTRUCT results, materialized. Mirrors the oxigraph Python `QueryTriples`.
#[pyclass(name = "QueryTriples")]
#[derive(Debug)]
pub struct PyQueryTriples {
    pub(crate) triples: Vec<RdfTriple>,
    pos: usize,
}

#[pymethods]
impl PyQueryTriples {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Py<PyTriple>>> {
        if slf.pos >= slf.triples.len() {
            return Ok(None);
        }
        let triple = slf.triples[slf.pos].clone();
        slf.pos += 1;
        Ok(Some(Py::new(py, PyTriple { inner: triple })?))
    }

    fn __len__(&self) -> usize {
        self.triples.len()
    }

    /// Serialize the constructed triples to bytes in `format` (the N-Triples
    /// fast path the `sparql` seam uses for its rdflib hand-off).
    fn serialize<'py>(
        &self,
        py: Python<'py>,
        format: PyRdfFormat,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = serialize_triples(&self.triples, format.to_native())
            .map_err(|e| PyValueError::new_err(format!("serialize error: {e}")))?;
        Ok(PyBytes::new(py, &bytes))
    }
}

/// An ASK result. Mirrors the oxigraph Python `QueryBoolean`.
#[pyclass(name = "QueryBoolean")]
#[derive(Debug)]
pub struct PyQueryBoolean {
    value: bool,
}

#[pymethods]
impl PyQueryBoolean {
    fn __bool__(&self) -> bool {
        self.value
    }

    fn __str__(&self) -> String {
        self.value.to_string()
    }

    fn __eq__(&self, other: bool) -> bool {
        self.value == other
    }

    fn __hash__(&self) -> u64 {
        u64::from(self.value)
    }
}

/// Convert a native [`SparqlResult`] into the materialized Python result object.
///
/// A SELECT becomes [`PyQuerySolutions`] (each cell a [`RdfTerm`]); a
/// CONSTRUCT/DESCRIBE [`SparqlResult::Graph`] is flattened into its triple stream
/// and becomes [`PyQueryTriples`]; an ASK becomes [`PyQueryBoolean`].
pub(crate) fn materialize_results(py: Python<'_>, result: SparqlResult) -> PyResult<Py<PyAny>> {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            let rows: Vec<Vec<Option<RdfTerm>>> = rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|cell| cell.map(term_value_to_rdf))
                        .collect()
                })
                .collect();
            Ok(Py::new(
                py,
                PyQuerySolutions {
                    variables,
                    rows,
                    pos: 0,
                },
            )?
            .into_any())
        }
        SparqlResult::Graph(graph) => {
            let triples = graph_to_triples(&graph);
            Ok(Py::new(py, PyQueryTriples { triples, pos: 0 })?.into_any())
        }
        SparqlResult::Boolean(value) => Ok(Py::new(py, PyQueryBoolean { value })?.into_any()),
    }
}

/// Flatten a CONSTRUCT result dataset into its triple stream (source-faithful: the
/// RDF 1.2 statement layer is re-materialized as `rdf:reifies`/annotation triples),
/// dropping the graph name (a CONSTRUCT yields a triple graph, matching the oxigraph
/// `QueryResults::Graph` egress).
fn graph_to_triples(graph: &RdfDataset) -> Vec<RdfTriple> {
    crate::flat_rdf_quads_from_dataset(graph)
        .into_iter()
        .map(|q| RdfTriple::new(q.subject, q.predicate, q.object))
        .collect()
}

/// Lower a dataset-independent [`TermValue`] (the SPARQL egress cell type) into the
/// owned [`RdfTerm`] the Python term layer wraps.
pub(crate) fn term_value_to_rdf(value: TermValue) -> RdfTerm {
    match value {
        TermValue::Iri(iri) => RdfTerm::Iri(iri),
        TermValue::Blank { label, scope } => {
            RdfTerm::BlankNode(scope.qualify_label(&label).into_owned())
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => RdfTerm::Literal(crate::RdfLiteral {
            // The native IR carries the datatype IRI by value (always present); the
            // owned model keeps a plain `xsd:string` / lang `rdf:langString` literal
            // datatype-less, so collapse those back to `None` for term parity.
            datatype: collapse_synthetic_datatype(&datatype, language.as_ref()),
            lexical_form,
            language,
            direction,
        }),
        TermValue::Triple { s, p, o } => RdfTerm::triple(RdfTriple::new(
            term_value_to_rdf(*s),
            term_value_predicate(*p),
            term_value_to_rdf(*o),
        )),
    }
}

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Drop the `TermValue` synthetic datatype IRI when it is the one the owned model
/// leaves implicit: `xsd:string` for a plain literal, `rdf:langString` for a
/// language-tagged one. Any other datatype is kept verbatim.
fn collapse_synthetic_datatype(datatype: &str, language: Option<&String>) -> Option<String> {
    if language.is_some() {
        return (datatype != RDF_LANG_STRING).then(|| datatype.to_owned());
    }
    (datatype != XSD_STRING).then(|| datatype.to_owned())
}

/// A triple-term predicate `TermValue` must be an IRI; fall back to its lexical form
/// for any other (ill-formed) shape so the conversion is total.
fn term_value_predicate(value: TermValue) -> String {
    match value {
        TermValue::Iri(iri) => iri,
        other => term_value_to_rdf(other).to_string(),
    }
}
