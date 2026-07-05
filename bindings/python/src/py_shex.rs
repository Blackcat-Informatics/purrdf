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

// ── pure-Rust cores (unit-tested without a Python interpreter) ───────────────────

/// Parse `schema` under `format` (`"shexc"` or `"shexj"`); `base` resolves
/// relative IRIs in a ShExC document (ShExJ is always absolute).
fn parse_schema(schema: &str, format: &str, base: Option<&str>) -> Result<Schema, String> {
    match format {
        "shexc" => parse_shexc(schema, base).map_err(|e| e.to_string()),
        "shexj" => parse_shexj(schema).map_err(|e| e.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = "PREFIX ex: <https://ex.example/> ex:S { ex:p . }";
    const DATA: &str = "<https://ex.example/n> <https://ex.example/p> 1 .";

    /// The pure-Rust core the pyfunction wraps: parse both inputs, validate the
    /// fixed map, and return the entries (mirrors `validate` sans the dict layer).
    fn run(map: &[(&str, &str)]) -> Vec<(bool, Option<String>)> {
        let schema = parse_schema(SCHEMA, "shexc", None).expect("schema");
        let dataset = parse_dataset(
            DATA.as_bytes(),
            data_media_type("turtle").expect("format"),
            None,
        )
        .expect("data");
        let associations: Vec<(TermValue, ShapeSelector)> = map
            .iter()
            .map(|(node, shape)| {
                (
                    node_to_term_value(node).expect("node"),
                    shape_selector(shape),
                )
            })
            .collect();
        purrdf_shex::validate(&schema, &dataset, &associations)
            .entries
            .into_iter()
            .map(|e| (e.status == ConformanceStatus::Conformant, e.reason))
            .collect()
    }

    #[test]
    fn fixed_map_validation_reports_per_association_verdicts() {
        let entries = run(&[
            ("https://ex.example/n", "https://ex.example/S"),
            ("<https://ex.example/absent>", "https://ex.example/S"),
        ]);
        assert!(entries[0].0, "node with ex:p conforms");
        assert!(entries[0].1.is_none(), "conformant entry has no reason");
        assert!(!entries[1].0, "node without ex:p does not conform");
        assert!(
            entries[1].1.is_some(),
            "nonconformant entry carries a reason"
        );
    }

    #[test]
    fn start_selector_without_start_shape_is_nonconformant_with_reason() {
        let entries = run(&[("https://ex.example/n", "START")]);
        assert!(!entries[0].0);
        assert!(
            entries[0].1.as_deref().unwrap_or("").contains("start"),
            "reason names the missing start shape: {:?}",
            entries[0].1
        );
    }

    #[test]
    fn node_strings_decode_to_terms() {
        assert_eq!(
            node_to_term_value("_:b0").expect("blank"),
            TermValue::blank("b0")
        );
        assert_eq!(
            node_to_term_value("<https://e/x>").expect("wrapped iri"),
            TermValue::iri("https://e/x")
        );
        assert_eq!(
            node_to_term_value("https://e/x").expect("bare iri"),
            TermValue::iri("https://e/x")
        );
        let lang = node_to_term_value("\"chat\"@fr").expect("lang literal");
        assert!(
            matches!(&lang, TermValue::Literal { lexical_form, language, .. }
                if lexical_form == "chat" && language.as_deref() == Some("fr")),
            "got {lang:?}"
        );
        let typed = node_to_term_value("\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>")
            .expect("typed literal");
        assert!(
            matches!(&typed, TermValue::Literal { lexical_form, datatype, .. }
                if lexical_form == "1" && datatype.ends_with("integer")),
            "got {typed:?}"
        );
        assert!(
            node_to_term_value("\"unterminated").is_err(),
            "a malformed literal is a typed error, not a panic"
        );
    }

    #[test]
    fn schema_format_routing_and_shexj_round_trip() {
        let shexc = parse_schema(SCHEMA, "shexc", None).expect("shexc");
        let shexj = to_shexj(&shexc);
        let reparsed = parse_schema(&shexj, "shexj", None).expect("shexj");
        assert_eq!(to_shexj(&reparsed), shexj, "ShExJ output is canonical");
        assert!(
            parse_schema(SCHEMA, "shexk", None).is_err(),
            "unknown format"
        );
        assert!(
            parse_schema("ex:S {", "shexc", None).is_err(),
            "malformed ShExC is a typed error"
        );
    }

    #[test]
    fn data_format_names_route_to_media_types() {
        assert_eq!(data_media_type("turtle").expect("turtle"), "text/turtle");
        assert_eq!(
            data_media_type("ntriples").expect("ntriples"),
            "application/n-triples"
        );
        assert_eq!(
            data_media_type("nquads").expect("nquads"),
            "application/n-quads"
        );
        assert!(data_media_type("trix").is_err());
    }
}
