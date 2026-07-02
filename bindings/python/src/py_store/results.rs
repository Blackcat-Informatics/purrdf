// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL **Results** serialization / parsing for the `purrdf` Python extension
//! (rdflib drop-in Task 6).
//!
//! The native SPARQL surface materializes a SELECT/ASK/CONSTRUCT result into the
//! [`PyQuerySolutions`](super::query::PyQuerySolutions) family, but there was no
//! way to emit a SELECT/ASK result in the four W3C SPARQL **Results** formats
//! (JSON / XML / CSV / TSV) or to read one back. This module bridges the compat
//! `Result` object model to [`purrdf_sparql_results`](crate::sparql) so
//! `rdflib`-style `Result.serialize(format=...)` / `Result.parse(...)` work.
//!
//! Every emitter is byte-deterministic by construction (the crate's hand-rolled
//! writers), so goldens are stable. Reads support JSON and XML (the two formats
//! the native crate can parse); CSV/TSV reads stay deferred on the Python side.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyTuple};

use super::query::term_value_to_rdf;
use super::term::{extract_term, term_to_py};
use crate::sparql::{
    from_json, from_json_boolean, from_xml, from_xml_boolean, serialize as serialize_results,
    ParsedSolutions, ResultProvenance, SparqlResultsFormat,
};
use crate::{BlankScope, RdfDatasetBuilder, RdfTerm, SparqlResult, TermValue};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Map the short format id (`json`/`xml`/`csv`/`tsv`) to the crate's format enum.
fn parse_format(name: &str) -> PyResult<SparqlResultsFormat> {
    match name {
        "json" => Ok(SparqlResultsFormat::Json),
        "xml" => Ok(SparqlResultsFormat::Xml),
        "csv" => Ok(SparqlResultsFormat::Csv),
        "tsv" => Ok(SparqlResultsFormat::Tsv),
        other => Err(PyValueError::new_err(format!(
            "unknown SPARQL results format `{other}` (expected json/xml/csv/tsv)"
        ))),
    }
}

/// Lower an owned [`RdfTerm`] (a compat-term round-tripped through the native
/// term model) into the [`TermValue`] the result model carries. Blank-node labels
/// are kept verbatim (DEFAULT scope) so serialized output is stable.
fn rdf_term_to_value(term: &RdfTerm) -> TermValue {
    match term {
        RdfTerm::Iri(iri) => TermValue::Iri(iri.clone()),
        RdfTerm::BlankNode(label) => TermValue::Blank {
            label: label.clone(),
            scope: BlankScope::DEFAULT,
        },
        RdfTerm::Literal(lit) => TermValue::Literal {
            lexical_form: lit.lexical_form.clone(),
            datatype: match (&lit.datatype, &lit.language) {
                (Some(dt), _) => dt.clone(),
                (None, Some(_)) => RDF_LANG_STRING.to_owned(),
                (None, None) => XSD_STRING.to_owned(),
            },
            language: lit.language.clone(),
            direction: lit.direction,
        },
        RdfTerm::Triple(t) => TermValue::Triple {
            s: Box::new(rdf_term_to_value(&t.subject)),
            p: Box::new(TermValue::Iri(t.predicate.clone())),
            o: Box::new(rdf_term_to_value(&t.object)),
        },
    }
}

/// Serialize a SELECT result (variables + dense rows of native terms) to the
/// requested SPARQL Results `format`.
///
/// Each cell is either `None` (unbound) or a native term object
/// (`NamedNode`/`BlankNode`/`Literal`/`Triple`). Returns the encoded document
/// bytes; the emitter is byte-deterministic.
#[pyfunction]
#[pyo3(signature = (format, variables, rows))]
pub(crate) fn serialize_sparql_solutions<'py>(
    py: Python<'py>,
    format: &str,
    variables: Vec<String>,
    rows: Vec<Vec<Option<Py<PyAny>>>>,
) -> PyResult<Bound<'py, PyBytes>> {
    let fmt = parse_format(format)?;
    let mut native_rows: Vec<Vec<Option<TermValue>>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut cells: Vec<Option<TermValue>> = Vec::with_capacity(row.len());
        for cell in row {
            match cell {
                None => cells.push(None),
                Some(obj) => {
                    let term = extract_term(obj.bind(py))?;
                    cells.push(Some(rdf_term_to_value(&term)));
                }
            }
        }
        native_rows.push(cells);
    }
    let aux = RdfDatasetBuilder::new()
        .freeze()
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let result = SparqlResult::Solutions {
        variables,
        rows: native_rows,
        aux,
    };
    let prov = ResultProvenance::default();
    // The native serialization runs detached (GIL released).
    let outcome = py
        .detach(|| serialize_results(&result, fmt, &prov))
        .map_err(|e| PyValueError::new_err(format!("SPARQL results serialize error: {e}")))?;
    Ok(PyBytes::new(py, &outcome.bytes))
}

/// Serialize an ASK result (a boolean) to the requested SPARQL Results `format`.
///
/// Only JSON and XML carry a boolean; CSV/TSV reject it (the crate enforces the
/// support matrix), surfacing a `ValueError`.
#[pyfunction]
#[pyo3(signature = (format, value))]
pub(crate) fn serialize_sparql_boolean<'py>(
    py: Python<'py>,
    format: &str,
    value: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let fmt = parse_format(format)?;
    let result = SparqlResult::Boolean(value);
    let prov = ResultProvenance::default();
    let outcome = py
        .detach(|| serialize_results(&result, fmt, &prov))
        .map_err(|e| PyValueError::new_err(format!("SPARQL results serialize error: {e}")))?;
    Ok(PyBytes::new(py, &outcome.bytes))
}

/// Parse a SPARQL Results document (`json` or `xml`) into a Python tuple the
/// compat layer turns into a `Result`.
///
/// The result is one of:
/// * `("SELECT", variables: list[str], rows: list[list[term | None]])`
/// * `("ASK", boolean: bool)`
///
/// where each `term` is a native term object. Only JSON and XML are parseable
/// (the native crate has no CSV/TSV reader); CSV/TSV raise a `ValueError`.
#[pyfunction]
#[pyo3(signature = (format, data))]
pub(crate) fn parse_sparql_results<'py>(
    py: Python<'py>,
    format: &str,
    data: &[u8],
) -> PyResult<Bound<'py, PyAny>> {
    let fmt = parse_format(format)?;
    match fmt {
        SparqlResultsFormat::Json => match from_json(data) {
            Ok(sol) => build_select_py(py, &sol),
            Err(select_err) => match from_json_boolean(data) {
                Ok(value) => build_ask_py(py, value),
                Err(_) => Err(PyValueError::new_err(format!(
                    "SPARQL results parse error: {select_err}"
                ))),
            },
        },
        SparqlResultsFormat::Xml => match from_xml(data) {
            Ok(sol) => build_select_py(py, &sol),
            Err(select_err) => match from_xml_boolean(data) {
                Ok(value) => build_ask_py(py, value),
                Err(_) => Err(PyValueError::new_err(format!(
                    "SPARQL results parse error: {select_err}"
                ))),
            },
        },
        SparqlResultsFormat::Csv | SparqlResultsFormat::Tsv => Err(PyValueError::new_err(
            format!("parsing SPARQL results `{format}` is not supported (JSON/XML only)"),
        )),
    }
}

/// Build the `("SELECT", variables, rows)` Python tuple from parsed solutions.
fn build_select_py<'py>(py: Python<'py>, sol: &ParsedSolutions) -> PyResult<Bound<'py, PyAny>> {
    let variables = PyList::new(py, &sol.variables)?;
    let rows = PyList::empty(py);
    for row in &sol.rows {
        let py_row = PyList::empty(py);
        for cell in row {
            match cell {
                None => py_row.append(py.None())?,
                Some(value) => {
                    let term = term_value_to_rdf(value.clone());
                    py_row.append(term_to_py(py, &term)?)?;
                }
            }
        }
        rows.append(py_row)?;
    }
    let tuple = PyTuple::new(
        py,
        [
            "SELECT".into_pyobject(py)?.into_any(),
            variables.into_any(),
            rows.into_any(),
        ],
    )?;
    Ok(tuple.into_any())
}

/// Build the `("ASK", boolean)` Python tuple.
fn build_ask_py(py: Python<'_>, value: bool) -> PyResult<Bound<'_, PyAny>> {
    let tuple = PyTuple::new(
        py,
        [
            "ASK".into_pyobject(py)?.into_any(),
            value.into_pyobject(py)?.to_owned().into_any(),
        ],
    )?;
    Ok(tuple.into_any())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_ids_map_to_enum() {
        assert_eq!(parse_format("json").unwrap(), SparqlResultsFormat::Json);
        assert_eq!(parse_format("xml").unwrap(), SparqlResultsFormat::Xml);
        assert_eq!(parse_format("csv").unwrap(), SparqlResultsFormat::Csv);
        assert_eq!(parse_format("tsv").unwrap(), SparqlResultsFormat::Tsv);
        assert!(parse_format("txt").is_err());
    }

    #[test]
    fn plain_literal_lowers_to_xsd_string_value() {
        let term = RdfTerm::literal(crate::RdfLiteral::simple("hi"));
        match rdf_term_to_value(&term) {
            TermValue::Literal {
                lexical_form,
                datatype,
                ..
            } => {
                assert_eq!(lexical_form, "hi");
                assert_eq!(datatype, XSD_STRING);
            }
            other => panic!("expected literal, got {other:?}"),
        }
    }
}
