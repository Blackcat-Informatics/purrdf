// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL validation → SARIF 2.1.0 in one call — the shared boundary the language
//! bindings (C-ABI, WASM, and a future Python caller) all route through.
//!
//! Each binding used to open-code the same two steps: run the SHACL
//! [`engine::validate_graphs`] over the shapes + data graphs, then hand the
//! resulting [`ValidationReport`] to [`report_to_sarif_string`]. Hoisting that
//! sequence here keeps the bindings to their platform-specific wrapping (buffer,
//! `JsValue`, `PyBytes`) and keeps the validate→SARIF semantics in one place.
//!
//! Wasm-clean: pure in-memory string work over the wasm-clean SHACL engine and
//! the SARIF writer — no new dependencies and no ambient I/O.
//!
//! [`engine::validate_graphs`]: purrdf_shapes::engine::validate_graphs
//! [`ValidationReport`]: purrdf_shapes::report::ValidationReport

use purrdf_shapes::engine;

use crate::{report_to_sarif_string, SarifOptions};

/// Validate `data_nt` (N-Triples) against `shapes_ttl` (Turtle) and render the
/// resulting SHACL report to a SARIF 2.1.0 JSON string.
///
/// This is the single entry point every language binding shares: it parses the
/// two graphs via the SHACL engine and serializes the report, returning a
/// `String` error (the engine's own parse/validation error) so callers can map
/// it to whatever their platform expects.
///
/// # Errors
///
/// Returns the SHACL engine's error string if either the shapes graph (Turtle)
/// or the data graph (N-Triples) fails to parse or validate.
///
/// # Examples
///
/// ```
/// use purrdf_validate::{validate_to_sarif_string, SarifOptions};
///
/// let shapes = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
///     @prefix ex: <http://example.org/> .\n\
///     @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
///     ex:PersonShape a sh:NodeShape ;\n\
///       sh:targetClass ex:Person ;\n\
///       sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";
/// let data = "<http://example.org/alice> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
///     <http://example.org/alice> <http://example.org/age> \"nope\" .\n";
///
/// let sarif = validate_to_sarif_string(shapes, data, &SarifOptions::default())
///     .expect("sarif produced");
/// assert!(sarif.contains("\"version\": \"2.1.0\""));
/// ```
pub fn validate_to_sarif_string(
    shapes_ttl: &str,
    data_nt: &str,
    options: &SarifOptions,
) -> Result<String, String> {
    let report = engine::validate_graphs(data_nt, shapes_ttl)?;
    Ok(report_to_sarif_string(&report, options))
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
    fn validate_to_sarif_string_emits_2_1_0_error() {
        let sarif = validate_to_sarif_string(SHAPES, DATA, &SarifOptions::default())
            .expect("sarif produced");
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("\"level\": \"error\""));
        assert!(sarif.contains("DatatypeConstraintComponent"));
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(
            validate_to_sarif_string("@@@ not turtle", DATA, &SarifOptions::default()).is_err()
        );
    }
}
