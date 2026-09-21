// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 Python bindings for `purrdf-shex` — the ShEx 2.1 schema layer and
//! fixed-shape-map validator, exposed as the `purrdf_native.shex` submodule
//! (mirroring [`crate::shacl`]).
//!
//! # Surface
//!
//! * [`validate`] — parse a schema (ShExC or ShExJ) and an RDF document
//!   (Turtle / N-Triples / N-Quads via the native `purrdf-rdf` codecs), run
//!   fixed-shape-map validation over the `(node, shape)` associations, and
//!   return one result dict per association.
//! * [`parse`] — parse a schema and return its canonical ShExJ (via
//!   [`purrdf_shex::to_shexj`]) for schema tooling.
//!
//! ```python
//! from purrdf_native import shex
//!
//! schema = "PREFIX ex: <https://ex.example/> ex:S { ex:p . }"
//! data = "<https://ex.example/n> <https://ex.example/p> 1 ."
//! results = shex.validate(schema, data, [("https://ex.example/n", "https://ex.example/S")])
//! assert results[0]["conformant"]
//!
//! shexj = shex.parse(schema)  # canonical ShExJ JSON text
//! ```
//!
//! # Hard-fail
//!
//! Every typed engine error ([`purrdf_shex::ShexError`], a codec
//! `RdfDiagnostic`, a malformed node string) is a Python `ValueError` carrying
//! the engine's message; the pure-Rust cores below are panic-free so nothing
//! unwinds across the FFI boundary.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use purrdf_shex::{ConformanceStatus, Schema, ShapeSelector, parse_shexc, parse_shexj, to_shexj};

use crate::{DatasetView, GraphMatch, NativeRdfFormat, TermValue, parse_dataset};

/// The shape-map spelling that selects the schema's `start` shape expression.
const START_SELECTOR: &str = "START";

// ── pure-Rust cores (PyO3-free, exercised through the pytest suite) ─────────────

/// Parse `schema` under `format` (`"shexc"` or `"shexj"`); `base` resolves the
/// relative IRIs of EITHER syntax — ShExJ is a JSON-LD dialect whose IRI-valued
/// members are document-relative just as ShExC's IRIREFs are.
fn parse_schema(schema: &str, format: &str, base: Option<&str>) -> Result<Schema, String> {
    match format {
        "shexc" => parse_shexc(schema, base).map_err(|e| e.to_string()),
        "shexj" => parse_shexj(schema, base).map_err(|e| e.to_string()),
        other => Err(format!(
            "unknown schema format `{other}` (expected \"shexc\" or \"shexj\")"
        )),
    }
}

/// Map the Python-surface data format name onto the native codec's media type.
fn data_media_type(format: &str) -> Result<&'static str, String> {
    match format {
        "turtle" => Ok(NativeRdfFormat::Turtle.media_type()),
        "ntriples" => Ok(NativeRdfFormat::NTriples.media_type()),
        "nquads" => Ok(NativeRdfFormat::NQuads.media_type()),
        other => Err(format!(
            "unknown data format `{other}` (expected \"turtle\", \"ntriples\", or \"nquads\")"
        )),
    }
}

/// Decode a shape-map focus-node string into a [`TermValue`]:
///
/// * `_:label` — a blank node;
/// * `<iri>` or a bare IRI — an IRI;
/// * `"…"`, `"…"@lang`, `"…"^^<dt>`, and the other Turtle literal spellings —
///   a literal (parsed through the native Turtle codec, so escapes and
///   datatypes behave exactly as in data).
fn node_to_term_value(node: &str) -> Result<TermValue, String> {
    if let Some(label) = node.strip_prefix("_:") {
        return Ok(TermValue::blank(label));
    }
    if let Some(inner) = node.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return Ok(TermValue::iri(inner));
    }
    if node.starts_with('"') || node.starts_with('\'') {
        return literal_term_value(node);
    }
    Ok(TermValue::iri(node))
}

/// Parse a Turtle literal token into its [`TermValue`] by embedding it as the
/// object of a one-triple document (the native codec owns escape/datatype
/// semantics, so this stays byte-faithful to data parsing).
fn literal_term_value(node: &str) -> Result<TermValue, String> {
    let doc = format!("<urn:x-purrdf:s> <urn:x-purrdf:p> {node} .\n");
    let dataset = parse_dataset(doc.as_bytes(), NativeRdfFormat::Turtle.media_type(), None)
        .map_err(|e| format!("invalid literal node `{node}`: {e}"))?;
    let mut quads = dataset.quads_for_pattern(None, None, None, GraphMatch::Any);
    let (Some(quad), None) = (quads.next(), quads.next()) else {
        return Err(format!("invalid literal node `{node}`"));
    };
    Ok(dataset.term_value(quad.o))
}

/// Decode a shape-map shape string: the literal `"START"` selects the schema's
/// start shape; anything else is a shape label (IRI or `_:`-prefixed blank).
fn shape_selector(shape: &str) -> ShapeSelector {
    if shape == START_SELECTOR {
        ShapeSelector::Start
    } else {
        ShapeSelector::Label(shape.to_owned())
    }
}

// ── the PyO3 surface ─────────────────────────────────────────────────────────────

/// Validate a fixed shape map against an RDF document.
///
/// `map` is a list of `(node, shape)` associations: `node` is an IRI (bare or
/// `<…>`-wrapped), a `_:`-prefixed blank node, or a Turtle literal token;
/// `shape` is a shape label, or the literal string `"START"` for the schema's
/// start shape. Returns one dict per association, in input order, with keys
/// `"node"` / `"shape"` (echoed verbatim), `"conformant"` (bool), and
/// `"reason"` (`None`, or the deepest failure for a nonconformant entry).
#[pyfunction]
#[pyo3(signature = (schema, data, map, *, schema_format="shexc", data_format="turtle", base=None))]
#[allow(
    clippy::needless_pass_by_value,
    reason = "the binding ABI receives owned values"
)]
fn validate(
    py: Python<'_>,
    schema: &str,
    data: &str,
    map: Vec<(String, String)>,
    schema_format: &str,
    data_format: &str,
    base: Option<&str>,
) -> PyResult<Py<PyAny>> {
    // Schema parse, data parse, and validation run detached (GIL released);
    // the per-association result dicts are built after the GIL is reacquired.
    let map_ref = &map;
    let result = py.detach(|| {
        let schema = parse_schema(schema, schema_format, base).map_err(PyValueError::new_err)?;
        let media_type = data_media_type(data_format).map_err(PyValueError::new_err)?;
        let dataset = parse_dataset(data.as_bytes(), media_type, base)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let mut associations = Vec::with_capacity(map_ref.len());
        for (node, shape) in map_ref {
            associations.push((
                node_to_term_value(node).map_err(PyValueError::new_err)?,
                shape_selector(shape),
            ));
        }
        Ok::<_, PyErr>(purrdf_shex::validate(&schema, &dataset, &associations))
    })?;

    let out = PyList::empty(py);
    for ((node, shape), entry) in map.iter().zip(&result.entries) {
        let d = PyDict::new(py);
        d.set_item("node", node)?;
        d.set_item("shape", shape)?;
        d.set_item("conformant", entry.status == ConformanceStatus::Conformant)?;
        d.set_item("reason", entry.reason.clone())?;
        out.append(d)?;
    }
    Ok(out.into_any().unbind())
}

/// Parse a ShEx schema (`format` is `"shexc"` or `"shexj"`) and return its
/// canonical ShExJ JSON text (via [`purrdf_shex::to_shexj`]), for schema
/// tooling and cross-syntax round-trips.
#[pyfunction]
#[pyo3(signature = (schema, *, format="shexc", base=None))]
fn parse(py: Python<'_>, schema: &str, format: &str, base: Option<&str>) -> PyResult<String> {
    // Parse + canonical ShExJ emission run detached (GIL released).
    py.detach(|| {
        let schema = parse_schema(schema, format, base).map_err(PyValueError::new_err)?;
        Ok(to_shexj(&schema))
    })
}

/// Register the `purrdf-shex` surface on a Python module. Called by the
/// unified `purrdf_native` cdylib to populate the `purrdf_native.shex`
/// submodule (mirroring [`crate::shacl::register`]).
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
