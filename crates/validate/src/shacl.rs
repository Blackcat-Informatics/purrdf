// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use std::sync::Arc;

use purrdf_core::DatasetMut;
use purrdf_core::ir::{MutableDataset, ViewLimits};
use purrdf_shapes::engine::{self, ChangeScope, PreparedShapes};

use purrdf_shapes::{ShapesError, ShapesImports};

use crate::{SarifOptions, ShapesImportList, report_to_sarif_string};

/// Validate `data_nt` (N-Triples) against `shapes_ttl` (Turtle) and render the
/// resulting SHACL report to a SARIF 2.1.0 JSON string.
///
/// This is the single entry point every language binding shares: it parses the
/// two graphs via the SHACL engine and serializes the report, returning the
/// engine's own [`ShapesError`] so callers can map it to whatever their platform
/// expects.
///
/// `imports` is the shapes graph's `owl:imports` table ([`ShapesImportList`]); the
/// empty list still refuses a shapes graph that imports a document it does not hold.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in
/// hand or `imports` cannot be used; [`ShapesError::Invalid`] if either the shapes
/// graph (Turtle) or the data graph (N-Triples) fails to parse or validate.
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
/// let sarif = validate_to_sarif_string(shapes, None, data, &SarifOptions::default(), &[])
///     .expect("sarif produced");
/// assert!(sarif.contains("\"version\": \"2.1.0\""));
/// ```
pub fn validate_to_sarif_string(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    options: &SarifOptions,
    imports: &ShapesImportList<'_>,
) -> Result<String, ShapesError> {
    let report = engine::validate_graphs_with_options(
        data_nt,
        shapes_ttl,
        shapes_base,
        &options.validation,
        &ShapesImports::from_turtle(imports)?,
    )?;
    Ok(report_to_sarif_string(&report, options))
}

/// Validate a CHANGE to `data_nt` against `shapes_ttl`, rendering the resulting
/// SHACL report to SARIF 2.1.0 and returning it beside the [`ChangeScope`] it
/// describes.
///
/// `added_nt` and `removed_nt` are the two halves of the delta — rows joining and
/// rows leaving the data graph — each an N-Triples document or `None`. Both
/// halves, because a verdict moves when a row leaves the graph as readily as when
/// one joins, and one flag would be half a delta. Additions are applied before
/// removals, so a change set naming the same row on both halves settles on
/// *removed*: the order a replayed insert-then-delete reaches. A removal naming a
/// row the data graph does not carry retracts nothing, which is the `remove`
/// contract everywhere else in PurRDF — a change set describes what moved, it does
/// not assert what the base contained.
///
/// # What the returned report describes
///
/// The scope is not decoration. [`ChangeScope::Bounded`] means the report covers
/// the affected focus nodes — for those nodes it is identical, results and
/// ordering alike, to a full validation of the mutated graph — and says nothing
/// about a pre-existing violation the change cannot reach, so `conforms` there
/// means *this change introduced no violation*. [`ChangeScope::Everything`] means
/// no bounded footprint exists for this shapes graph (its constraints read through
/// SPARQL query text), the run fell back to a full validation of the mutated
/// graph, and `conforms` means *the graph conforms*. A caller handed the report
/// alone cannot tell those apart, and the weaker reading is the dangerous one.
///
/// The loop itself is [`engine::validate_change`] — the same call the command
/// line and the Python bindings drive. This adds the string boundary: parsing the
/// three documents, branching the base into a copy-on-write mutation, and
/// rendering the report.
///
/// `imports` is the shapes graph's `owl:imports` table, exactly as
/// [`validate_to_sarif_string`] takes it.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in
/// hand or `imports` cannot be used; [`ShapesError::Invalid`] when the shapes graph
/// (Turtle) or any of the three N-Triples documents fails to parse, when a change row
/// cannot be admitted, or when constraint evaluation hard-fails.
///
/// # Examples
///
/// ```
/// use purrdf_shapes::engine::ChangeScope;
/// use purrdf_validate::{SarifOptions, validate_changes_to_sarif_string};
///
/// let shapes = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
///     @prefix ex: <http://example.org/> .\n\
///     @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
///     ex:PersonShape a sh:NodeShape ;\n\
///       sh:targetClass ex:Person ;\n\
///       sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";
/// let data = "<http://example.org/alice> \
///     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";
/// let added = "<http://example.org/alice> <http://example.org/age> \"nope\" .\n";
///
/// let (sarif, scope) = validate_changes_to_sarif_string(
///     shapes,
///     None,
///     data,
///     Some(added),
///     None,
///     &SarifOptions::default(),
///     &[],
/// )
/// .expect("the change validates");
/// assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
/// assert!(sarif.contains("DatatypeConstraintComponent"));
/// ```
pub fn validate_changes_to_sarif_string(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    added_nt: Option<&str>,
    removed_nt: Option<&str>,
    options: &SarifOptions,
    imports: &ShapesImportList<'_>,
) -> Result<(String, ChangeScope), ShapesError> {
    let table = ShapesImports::from_turtle(imports)?;
    let base = parse_ntriples(data_nt)?;
    let mut mutation = MutableDataset::new(base);
    if let Some(added) = added_nt {
        for row in parse_ntriples(added)?.flat_default_graph_quads() {
            mutation
                .insert(row)
                .map_err(|error| error.diagnostic_code().to_owned())?;
        }
    }
    if let Some(removed) = removed_nt {
        for row in parse_ntriples(removed)?.flat_default_graph_quads() {
            mutation.remove(&row);
        }
    }
    let snapshot = Arc::new(
        mutation
            .snapshot_view()
            .map_err(|error| error.to_string())?,
    );

    let mut shapes = engine::parse_shapes_with_config(shapes_ttl, shapes_base, None, &table)?;
    shapes.set_validation_options(options.validation.clone());
    let validator = PreparedShapes::new(Arc::new(shapes)).bind_delta_with_shapes_graph(
        Arc::clone(&snapshot),
        None,
        ViewLimits::default(),
    )?;
    let validation = engine::validate_change(&validator, &snapshot)?;
    Ok((
        report_to_sarif_string(&validation.report, options),
        validation.scope,
    ))
}

/// Parse one N-Triples document, joining its per-line diagnostics the way every
/// other entry point in this module reports a failed parse.
///
/// The whole RDF 1.2 surface of the parsed document is read back out with
/// `flat_default_graph_quads` at the call sites above, so a change document
/// carrying a reifier declaration or an annotation contributes those rows too. A
/// change set short by a row cannot be told from a graph that did not change.
/// Nothing is dropped by flattening: N-Triples has no named graph to drop.
fn parse_ntriples(document: &str) -> Result<Arc<purrdf_core::RdfDataset>, String> {
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(document)
        .map_err(|errors| errors.join("\n"))
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
        let sarif = validate_to_sarif_string(SHAPES, None, DATA, &SarifOptions::default(), &[])
            .expect("sarif produced");
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("\"level\": \"error\""));
        assert!(sarif.contains("DatatypeConstraintComponent"));
    }

    /// The request's conformance-disallow set reaches the validation: a
    /// Warning-only report does not conform under the default set and conforms
    /// under {Violation}, and the log says which set it was judged against.
    #[test]
    fn validate_to_sarif_string_honours_conformance_disallows() {
        let shapes = SHAPES.replace(
            "sh:path ex:age ;",
            "sh:path ex:age ; sh:severity sh:Warning ;",
        );
        let conforms = |options: &SarifOptions| -> (Value, Value) {
            let sarif = validate_to_sarif_string(&shapes, None, DATA, options, &[]).expect("sarif");
            let log: Value = serde_json::from_str(&sarif).expect("json");
            let properties = log["runs"][0]["properties"].clone();
            (
                properties["shaclConforms"].clone(),
                properties["shaclConformanceDisallows"].clone(),
            )
        };
        assert_eq!(
            conforms(&SarifOptions::default()),
            (
                json!(false),
                json!([
                    "http://www.w3.org/ns/shacl#Violation",
                    "http://www.w3.org/ns/shacl#Warning",
                    "http://www.w3.org/ns/shacl#Info"
                ])
            )
        );
        let relaxed = SarifOptions {
            validation: engine::ValidationOptions::default().with_conformance_disallows(
                purrdf_shapes::report::ConformanceDisallows::new([
                    purrdf_shapes::report::Severity::Violation,
                ])
                .expect("non-empty"),
            ),
            ..SarifOptions::default()
        };
        assert_eq!(
            conforms(&relaxed),
            (json!(true), json!(["http://www.w3.org/ns/shacl#Violation"]))
        );
    }

    #[test]
    fn malformed_shapes_is_an_error() {
        assert!(
            validate_to_sarif_string("@@@ not turtle", None, DATA, &SarifOptions::default(), &[])
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
                    validate_to_sarif_string(&shapes, None, &data, &SarifOptions::default(), &[])
                        .expect("repeated parameters are independent conjunctive constraints");
                assert_eq!(
                    sarif,
                    validate_to_sarif_string(&reversed, None, &data, &SarifOptions::default(), &[])
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
            let sarif = validate_to_sarif_string(&imported, None, input, &options, &[])
                .expect("standard component declarations permit repeated sh:property");
            assert_eq!(
                sarif,
                validate_to_sarif_string(&shapes, None, input, &options, &[])
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

    /// The base graph both change cases start from: Alice is a person with a
    /// well-typed age, so the base CONFORMS and every violation below is the
    /// change's doing.
    const CHANGE_BASE: &str = "<http://example.org/alice> \
        <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
        <http://example.org/alice> <http://example.org/age> \"41\"\
        ^^<http://www.w3.org/2001/XMLSchema#integer> .\n";

    /// A shapes graph whose constraint reads through SPARQL query text, so no
    /// bounded footprint exists for it and the change path must fall back.
    const SPARQL_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:PersonShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:sparql [ a sh:SPARQLConstraint ;\n\
            sh:message \"every person needs a name\" ;\n\
            sh:select \"\"\"SELECT $this WHERE { FILTER NOT EXISTS \
              { $this <http://example.org/name> ?n } }\"\"\" ] .\n";

    /// An ADDED row that introduces a violation is reported, the scope says the
    /// report is about the focus nodes the change reached, and the SARIF is the
    /// report a full validation of the merged graph produces.
    #[test]
    fn an_added_row_is_validated_against_the_graph_it_joins() {
        let added = "<http://example.org/bob> \
            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
            <http://example.org/bob> <http://example.org/age> \"nope\" .\n";
        let options = SarifOptions::default();
        let (sarif, scope) = validate_changes_to_sarif_string(
            SHAPES,
            None,
            CHANGE_BASE,
            Some(added),
            None,
            &options,
            &[],
        )
        .expect("the change validates");

        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
        // The merged graph conforms about Alice and violates about Bob, so a full
        // validation of it is the same log the bounded run produced: the change
        // path is a cheaper route to one answer, never a second answer.
        let merged = format!("{CHANGE_BASE}{added}");
        assert_eq!(
            sarif,
            validate_to_sarif_string(SHAPES, None, &merged, &options, &[])
                .expect("full validation"),
        );
        assert!(sarif.contains("DatatypeConstraintComponent"));
    }

    /// The retract half is a real half: a removal that takes a required value away
    /// moves the verdict exactly as an addition does, and matches a full
    /// validation of the REDUCED graph.
    #[test]
    fn a_removed_row_moves_the_verdict_too() {
        const MIN_COUNT_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
            @prefix ex: <http://example.org/> .\n\
            ex:PersonShape a sh:NodeShape ;\n\
              sh:targetClass ex:Person ;\n\
              sh:property [ sh:path ex:age ; sh:minCount 1 ] .\n";
        let removed = "<http://example.org/alice> <http://example.org/age> \"41\"\
            ^^<http://www.w3.org/2001/XMLSchema#integer> .\n";
        let options = SarifOptions::default();

        let (sarif, scope) = validate_changes_to_sarif_string(
            MIN_COUNT_SHAPES,
            None,
            CHANGE_BASE,
            None,
            Some(removed),
            &options,
            &[],
        )
        .expect("the retraction validates");

        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 1 });
        assert!(sarif.contains("MinCountConstraintComponent"), "{sarif}");
        let reduced = CHANGE_BASE.replace(removed, "");
        assert_eq!(
            sarif,
            validate_to_sarif_string(MIN_COUNT_SHAPES, None, &reduced, &options, &[])
                .expect("full validation of the reduced graph"),
        );
    }

    /// The fallback is not optional. A shapes graph reading through query text has
    /// no bounded footprint, so the run validates the WHOLE mutated graph, names
    /// the construct responsible, and reaches the full validation's own log.
    #[test]
    fn an_unbounded_footprint_falls_back_to_a_full_validation_and_says_so() {
        let added = "<http://example.org/bob> \
            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";
        let options = SarifOptions::default();
        let (sarif, scope) = validate_changes_to_sarif_string(
            SPARQL_SHAPES,
            None,
            CHANGE_BASE,
            Some(added),
            None,
            &options,
            &[],
        )
        .expect("the change validates");

        assert!(!scope.is_bounded(), "{scope:?}");
        assert_eq!(scope.focus_nodes(), None, "a fallback covers no COUNT");
        assert!(scope.reason().is_some_and(|reason| !reason.is_empty()));

        let merged = format!("{CHANGE_BASE}{added}");
        assert_eq!(
            sarif,
            validate_to_sarif_string(SPARQL_SHAPES, None, &merged, &options, &[])
                .expect("full validation"),
            "the fallback report must BE the full validation's report",
        );
        // Both Alice and Bob lack a name, so the fallback genuinely reported on
        // the untouched node too — which is what makes it a full validation.
        assert!(sarif.contains("alice") && sarif.contains("bob"), "{sarif}");
    }

    /// An empty change expands to nothing rather than quietly re-validating
    /// everything, and the neighbouring case — a change that does move a node —
    /// still reports one. Zero and "everything" are opposite instructions.
    #[test]
    fn an_empty_change_expands_to_nothing() {
        let options = SarifOptions::default();
        let (sarif, scope) =
            validate_changes_to_sarif_string(SHAPES, None, CHANGE_BASE, None, None, &options, &[])
                .expect("an empty change validates");
        assert_eq!(scope, ChangeScope::Bounded { focus_nodes: 0 });
        assert!(!sarif.contains("\"level\": \"error\""), "{sarif}");

        let added = "<http://example.org/bob> \
            <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n";
        let (_, moved) = validate_changes_to_sarif_string(
            SHAPES,
            None,
            CHANGE_BASE,
            Some(added),
            None,
            &options,
            &[],
        )
        .expect("a real change validates");
        assert_eq!(moved, ChangeScope::Bounded { focus_nodes: 1 });
    }

    /// Every document on this route is parsed, and a failure in any of the three
    /// is reported rather than silently treated as an empty change — while the
    /// neighbouring well-formed call still succeeds.
    #[test]
    fn a_malformed_document_on_any_leg_is_an_error() {
        let options = SarifOptions::default();
        for (shapes, data, added, removed) in [
            ("@@@ not turtle", CHANGE_BASE, None, None),
            (SHAPES, "@@@ not n-triples", None, None),
            (SHAPES, CHANGE_BASE, Some("@@@ not n-triples"), None),
            (SHAPES, CHANGE_BASE, None, Some("@@@ not n-triples")),
        ] {
            assert!(
                validate_changes_to_sarif_string(shapes, None, data, added, removed, &options, &[])
                    .is_err(),
                "a malformed document must not validate",
            );
        }
        validate_changes_to_sarif_string(
            SHAPES,
            None,
            CHANGE_BASE,
            Some("<http://example.org/bob> <http://example.org/age> \"7\" .\n"),
            Some(
                "<http://example.org/alice> <http://example.org/age> \"41\"\
                ^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
            ),
            &options,
            &[],
        )
        .expect("well-formed documents on every leg still validate");
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
            .expect_err("multiple sh:minCount values are malformed")
            .to_string();
        let boundary_error =
            validate_to_sarif_string(shapes, None, "", &SarifOptions::default(), &[])
                .expect_err("malformed constraints must not produce a SARIF report")
                .to_string();
        assert_eq!(boundary_error, engine_error);
        assert!(boundary_error.contains("minCount"), "{boundary_error}");
        assert!(boundary_error.contains("RequiredValue"), "{boundary_error}");
    }
}
