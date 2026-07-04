// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL validation → SARIF 2.1.0 for the wasm/JS surface.
//!
//! A thin shim over the wasm-clean SHACL engine and its SARIF reporting
//! boundary: validate a data graph (N-Triples) against a shapes graph (Turtle)
//! and return a SARIF 2.1.0 JSON string that editors and CI dashboards consume.

use wasm_bindgen::prelude::*;

/// Validate `data_nt` against `shapes_ttl` and render the report to SARIF 2.1.0.
///
/// Returns a plain `String` error (NOT a `JsError`) so it is unit-testable on the
/// native build — constructing a `JsError` calls a wasm-only import that panics
/// off wasm. The `#[wasm_bindgen]` wrapper maps the `String` to a `JsError`.
pub(crate) fn validate_to_sarif_impl(shapes_ttl: &str, data_nt: &str) -> Result<String, String> {
    let report = purrdf_shapes::engine::validate_graphs(data_nt, shapes_ttl)?;
    Ok(purrdf_validate::report_to_sarif_string(
        &report,
        &purrdf_validate::SarifOptions::default(),
    ))
}

/// `shaclValidateToSarif(shapesTtl, dataNt)` → a SARIF 2.1.0 JSON string.
///
/// `shapesTtl` is a Turtle shapes graph; `dataNt` is an N-Triples data graph.
/// Throws (rejects) if either graph fails to parse.
#[wasm_bindgen(js_name = shaclValidateToSarif)]
pub fn shacl_validate_to_sarif(shapes_ttl: &str, data_nt: &str) -> Result<String, JsError> {
    validate_to_sarif_impl(shapes_ttl, data_nt).map_err(|e| JsError::new(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
        ex:PersonShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";

    const DATA: &str = "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
        <http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    #[test]
    fn validate_emits_sarif_2_1_0() {
        let sarif = validate_to_sarif_impl(SHAPES, DATA).expect("sarif produced");
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("\"level\": \"error\""));
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(validate_to_sarif_impl("@@@ not turtle", DATA).is_err());
    }
}
