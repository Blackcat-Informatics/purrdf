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

use crate::{SarifOptions, report_to_sarif_string};

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
/// let sarif = validate_to_sarif_string(shapes, None, data, &SarifOptions::default())
///     .expect("sarif produced");
/// assert!(sarif.contains("\"version\": \"2.1.0\""));
/// ```
pub fn validate_to_sarif_string(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    options: &SarifOptions,
) -> Result<String, String> {
    let report = engine::validate_graphs(data_nt, shapes_ttl, shapes_base)?;
    Ok(report_to_sarif_string(&report, options))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use serde_json::{Value, json};

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
        let sarif = validate_to_sarif_string(SHAPES, None, DATA, &SarifOptions::default())
            .expect("sarif produced");
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("\"level\": \"error\""));
        assert!(sarif.contains("DatatypeConstraintComponent"));
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(
            validate_to_sarif_string("@@@ not turtle", None, DATA, &SarifOptions::default())
                .is_err()
        );
    }

    fn sarif_result(
        component: &str,
        source_shape: &str,
        path: Option<&str>,
        message: &str,
    ) -> Value {
        let mut locations = vec![json!({
            "name": "http://example.org/alice",
            "kind": "focusNode",
        })];
        if let Some(path) = path {
            locations.push(json!({ "name": path, "kind": "resultPath" }));
        }
        locations.push(json!({ "name": component, "kind": "constraintComponent" }));
        json!({
            "ruleId": component,
            "ruleIndex": 0,
            "level": "error",
            "message": { "text": message },
            "locations": [{ "logicalLocations": locations }],
            "relatedLocations": [{
                "logicalLocations": [{ "name": source_shape, "kind": "sourceShape" }],
                "message": { "text": "shape defined here" },
            }],
        })
    }

    #[test]
    fn repeated_custom_parameters_preserve_every_sarif_result() {
        const COMPONENT: &str = "http://example.org/RequiredPredicateConstraintComponent";
        const DECLARATION: &str = r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:RequiredPredicateConstraintComponent a sh:ConstraintComponent ;
                sh:parameter [ sh:path ex:required ] ;
                sh:validator [
                    a sh:SPARQLAskValidator ;
                    sh:message "Missing {$required}" ;
                    sh:ask "ASK { $value $required ?object }"
                ] .
        "#;

        for (shape_kind, path, value_node) in [
            ("sh:NodeShape", None, "alice"),
            (
                "sh:PropertyShape ; sh:path ex:item",
                Some("<http://example.org/item>"),
                "bob",
            ),
        ] {
            let shapes = format!(
                "{DECLARATION}\nex:Requirements a {shape_kind} ; \
                 sh:targetNode ex:alice ; ex:required ex:first, ex:second ."
            );
            let reversed = shapes.replace("ex:first, ex:second", "ex:second, ex:first");
            let mut data = String::from(
                "<http://example.org/alice> <http://example.org/item> \
                 <http://example.org/bob> .\n",
            );
            for present in 0..=2 {
                let sarif =
                    validate_to_sarif_string(&shapes, None, &data, &SarifOptions::default())
                        .expect("repeated parameters are independent conjunctive constraints");
                assert_eq!(
                    sarif,
                    validate_to_sarif_string(&reversed, None, &data, &SarifOptions::default())
                        .expect("reversing parameter values preserves validation"),
                );
                let document: Value = serde_json::from_str(&sarif).expect("SARIF JSON");
                let expected: Vec<Value> = ["first", "second"]
                    .into_iter()
                    .skip(present)
                    .map(|predicate| {
                        sarif_result(
                            COMPONENT,
                            "<http://example.org/Requirements>",
                            path,
                            &format!("Missing http://example.org/{predicate}"),
                        )
                    })
                    .collect();
                assert_eq!(
                    document["runs"][0]["results"],
                    if expected.is_empty() {
                        Value::Null
                    } else {
                        json!(expected)
                    },
                );

                if let Some(predicate) = ["first", "second"].get(present) {
                    writeln!(
                        data,
                        "<http://example.org/{value_node}> \
                         <http://example.org/{predicate}> <http://example.org/object> ."
                    )
                    .expect("write data to String");
                }
            }
        }
    }

    #[test]
    fn imported_property_component_preserves_all_fifteen_property_constraints() {
        const DECLARATION: &str = r"
            sh:PropertyConstraintComponent a sh:ConstraintComponent ;
                sh:parameter sh:PropertyConstraintComponent-property .
            sh:PropertyConstraintComponent-property a sh:Parameter ;
                sh:path sh:property ;
                sh:nodeKind sh:BlankNodeOrIRI .
        ";
        let mut shapes = String::from(
            "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
             @prefix ex: <http://example.org/> .\n\
             ex:Requirements a sh:NodeShape ; sh:targetNode ex:alice .\n",
        );
        let mut data = String::new();
        let mut expected = Vec::new();
        for index in 0..15 {
            writeln!(
                shapes,
                "ex:Requirements sh:property ex:property{index:02} .\n\
                 ex:property{index:02} sh:path ex:value{index:02} ; sh:minCount 1 ; \
                 sh:message \"Missing property {index:02}\" ."
            )
            .expect("write shapes to String");
            writeln!(
                data,
                "<http://example.org/alice> <http://example.org/value{index:02}> \
                 <http://example.org/object> ."
            )
            .expect("write data to String");
            expected.push(sarif_result(
                "http://www.w3.org/ns/shacl#MinCountConstraintComponent",
                &format!("<http://example.org/property{index:02}>"),
                Some(&format!("<http://example.org/value{index:02}>")),
                &format!("Missing property {index:02}"),
            ));
        }
        let imported = format!("{shapes}\n{DECLARATION}");
        for (input, results) in [("", expected), (data.as_str(), Vec::new())] {
            let options = SarifOptions::default();
            let sarif = validate_to_sarif_string(&imported, None, input, &options)
                .expect("standard component declarations permit repeated sh:property");
            assert_eq!(
                sarif,
                validate_to_sarif_string(&shapes, None, input, &options)
                    .expect("native property constraints validate"),
                "importing the vocabulary preserves the exact report",
            );
            let document: Value = serde_json::from_str(&sarif).expect("SARIF JSON");
            assert_eq!(
                document["runs"][0]["results"],
                if results.is_empty() {
                    Value::Null
                } else {
                    json!(results)
                },
            );
        }
    }

    #[test]
    fn malformed_singleton_parameter_propagates_the_engine_error() {
        let shapes = r"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:Requirements a sh:NodeShape ; sh:targetNode ex:alice ;
                sh:property ex:RequiredValue .
            ex:RequiredValue sh:path ex:value ; sh:minCount 1, 2 .
        ";
        let engine_error = engine::validate_graphs("", shapes, None)
            .expect_err("multiple sh:minCount values are malformed");
        let boundary_error = validate_to_sarif_string(shapes, None, "", &SarifOptions::default())
            .expect_err("malformed constraints must not produce a SARIF report");
        assert_eq!(boundary_error, engine_error);
        assert!(boundary_error.contains("minCount"), "{boundary_error}");
        assert!(boundary_error.contains("RequiredValue"), "{boundary_error}");
    }
}
