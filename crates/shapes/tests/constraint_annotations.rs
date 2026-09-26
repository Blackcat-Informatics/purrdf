// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! PER-CONSTRAINT REIFIER ANNOTATIONS, THE `sh:Debug` / `sh:Trace` SEVERITIES,
//! AND THE CONFORMANCE-DISALLOW SET — each observed by validating.
//!
//! SHACL 1.2 Core lets an RDF 1.2 reifier of a `(shape, parameter, value)`
//! statement carry `sh:deactivated`, `sh:severity` and `sh:message` for the one
//! constraint that statement represents; names five built-in severities, two of
//! which are "not a constraint violation"; and defines conformance against "the
//! set of disallowed severity levels", defaulting to `sh:Violation`, `sh:Warning`
//! and `sh:Info`.
//!
//! Every treatment row here has a control row that answers DIFFERENTLY, so an
//! annotation that was silently ignored, or a set that was silently defaulted,
//! fails rather than passing by coincidence. Every refusal sits beside the valid
//! neighbour it must not catch.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{
    PreparedShapes, ValidationOptions, parse_shapes, validate_dataset_with_shapes_graph,
};
use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
use purrdf_shapes::report::{
    ConformanceDisallows, Severity, ValidationReport, conformance_disallows_from_dataset,
};
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:   <http://example.org/ns#> .
@prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
";

fn load(shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).map_err(String::from)
}

#[track_caller]
fn loads(shapes_ttl: &str) -> Shapes {
    load(shapes_ttl).unwrap_or_else(|error| panic!("the shapes graph must load: {error}"))
}

#[track_caller]
fn refused(shapes_ttl: &str, needle: &str) {
    let error = load(shapes_ttl).expect_err("the shapes graph must be refused at load");
    assert!(
        error.contains(needle),
        "refusal must mention {needle:?}: {error}"
    );
}

fn data(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parses")
}

#[track_caller]
fn validate_with(shapes: &Shapes, data_ttl: &str) -> ValidationReport {
    validate_dataset_with_shapes_graph(&data(data_ttl), shapes, None).expect("validation runs")
}

#[track_caller]
fn validate(shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    validate_with(&loads(shapes_ttl), data_ttl)
}

fn sh(local: &str) -> String {
    format!("http://www.w3.org/ns/shacl#{local}")
}

/// Every message of a result, rendered as N-Triples literals (so a language tag
/// shows), in the report's canonical order.
fn rendered(messages: &[Literal]) -> Vec<String> {
    messages
        .iter()
        .map(|m| Term::Literal(m.clone()).to_string())
        .collect()
}

/// `(component local name, severity IRI, messages)` of every result, sorted.
fn rows(report: &ValidationReport) -> Vec<(String, String, Vec<String>)> {
    let mut out: Vec<(String, String, Vec<String>)> = report
        .results
        .iter()
        .map(|r| {
            (
                r.source_constraint_component
                    .as_str()
                    .trim_start_matches("http://www.w3.org/ns/shacl#")
                    .to_owned(),
                r.severity.iri().to_owned(),
                rendered(&r.messages),
            )
        })
        .collect();
    out.sort();
    out
}

fn row(component: &str, severity: &str, messages: &[&str]) -> (String, String, Vec<String>) {
    (
        component.to_owned(),
        sh(severity),
        messages.iter().map(|m| (*m).to_owned()).collect(),
    )
}

fn disallowing(levels: &[Severity]) -> ValidationOptions {
    ValidationOptions::default().with_conformance_disallows(
        ConformanceDisallows::new(levels.iter().cloned()).expect("a non-empty set"),
    )
}

// ── sh:deactivated on a reifier ──────────────────────────────────────────────

/// The specification's own example: `sh:minCount 1 {| sh:deactivated true |}`
/// switches off the minCount constraint and leaves `sh:maxCount` on the same
/// property shape active.
#[test]
fn a_reifier_deactivates_one_constraint_and_leaves_its_sibling_active() {
    let shapes = |annotation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ;
               sh:property [ sh:path ex:name ; sh:minCount 1 {annotation} ; sh:maxCount 1 ] ."
        )
    };
    let data = "ex:b ex:name \"x\", \"y\" .";
    assert_eq!(
        rows(&validate(&shapes("{| sh:deactivated true |}"), data)),
        vec![row("MaxCountConstraintComponent", "Violation", &[])]
    );
    // Control: the same statement annotated `false`, and not annotated at all,
    // keep minCount — the missing `ex:a` value is reported.
    for control in ["{| sh:deactivated false |}", ""] {
        assert_eq!(
            rows(&validate(&shapes(control), data)),
            vec![
                row("MaxCountConstraintComponent", "Violation", &[]),
                row("MinCountConstraintComponent", "Violation", &[]),
            ],
            "{control:?}"
        );
    }
}

/// A reifier annotates one STATEMENT, so on a multi-valued parameter it reaches
/// only the constraint that statement represents: `ex:a` is an `ex:A` and not an
/// `ex:B`, so only the `sh:class ex:B` constraint reports, graded by the
/// annotation on ITS statement and not by one on its sibling's.
#[test]
fn a_reifier_on_one_value_of_a_multi_valued_parameter_reaches_that_value_only() {
    let data = "ex:a a ex:A .";
    assert_eq!(
        rows(&validate(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:class ex:A , ex:B {| sh:severity sh:Warning |} .",
            data,
        )),
        vec![row("ClassConstraintComponent", "Warning", &[])]
    );
    assert_eq!(
        rows(&validate(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:class ex:A {| sh:severity sh:Warning |} , ex:B .",
            data,
        )),
        vec![row("ClassConstraintComponent", "Violation", &[])]
    );
}

/// `sh:property` is a constraint parameter: deactivating its statement drops the
/// property shape from THAT reach only — the same property shape reached through
/// an unannotated statement still validates.
#[test]
fn deactivating_an_sh_property_statement_drops_that_reach_only() {
    let shapes = "
        ex:P a sh:PropertyShape ; sh:path ex:name ; sh:minCount 1 .
        ex:S1 a sh:NodeShape ; sh:targetNode ex:a ; sh:property ex:P {| sh:deactivated true |} .
        ex:S2 a sh:NodeShape ; sh:targetNode ex:b ; sh:property ex:P .";
    let report = validate(shapes, "ex:c ex:name \"x\" .");
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect();
    assert_eq!(focus, vec!["<http://example.org/ns#b>".to_owned()]);
}

/// A severity or message on the `sh:property` statement grades the results that
/// reach through it — below an annotation on the property shape's own constraint
/// statement, which names the constraint that caused the result more narrowly.
#[test]
fn an_sh_property_statement_grades_its_reach_below_the_constraints_own_annotation() {
    let shapes = "
        ex:P a sh:PropertyShape ; sh:path ex:name ;
          sh:minCount 1 ;
          sh:datatype xsd:integer {| sh:severity sh:Info |} .
        ex:S1 a sh:NodeShape ; sh:targetNode ex:a ;
          sh:property ex:P {| sh:severity sh:Warning ; sh:message \"through S1\" |} .
        ex:S2 a sh:NodeShape ; sh:targetNode ex:b ; sh:property ex:P .";
    let report = validate(shapes, "ex:a ex:q 1 . ex:b ex:name \"x\" .");
    let mut by_focus: Vec<(String, String, String, Vec<String>)> = report
        .results
        .iter()
        .map(|r| {
            (
                r.focus_node.to_string(),
                r.source_constraint_component
                    .as_str()
                    .trim_start_matches("http://www.w3.org/ns/shacl#")
                    .to_owned(),
                r.severity.iri().to_owned(),
                rendered(&r.messages),
            )
        })
        .collect();
    by_focus.sort();
    assert_eq!(
        by_focus,
        vec![
            (
                "<http://example.org/ns#a>".to_owned(),
                "MinCountConstraintComponent".to_owned(),
                sh("Warning"),
                vec!["\"through S1\"".to_owned()]
            ),
            (
                "<http://example.org/ns#b>".to_owned(),
                "DatatypeConstraintComponent".to_owned(),
                sh("Info"),
                vec![]
            ),
        ]
    );
}

/// The `sh:reifierShape` / `sh:reificationRequired` constraint takes the
/// annotation on its statements: a severity grades it, a deactivation removes it.
#[test]
fn the_reifier_shape_constraint_takes_its_annotations() {
    let shapes = |annotation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:property [ sh:path ex:p ; sh:reificationRequired true {annotation} ] ."
        )
    };
    let data = "ex:a ex:p 1 .";
    assert_eq!(
        rows(&validate(&shapes("{| sh:severity sh:Warning |}"), data)),
        vec![row("ReifierShapeConstraintComponent", "Warning", &[])]
    );
    assert_eq!(
        rows(&validate(&shapes(""), data)),
        vec![row("ReifierShapeConstraintComponent", "Violation", &[])]
    );
    assert!(
        validate(&shapes("{| sh:deactivated true |}"), data)
            .results
            .is_empty()
    );
}

/// A `sh:sparql` constraint whose own node says `sh:deactivated true` produces no
/// results (SHACL 1.2 SPARQL Extensions); `false` is the control.
#[test]
fn a_deactivated_sparql_constraint_produces_nothing() {
    let shapes = |flag: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:sparql [ sh:select \"SELECT $this WHERE {{ }}\" ; sh:deactivated {flag} ] ."
        )
    };
    assert!(validate(&shapes("true"), "").results.is_empty());
    assert_eq!(validate(&shapes("false"), "").results.len(), 1);
}

// ── sh:severity and sh:message on a reifier ──────────────────────────────────

/// The reifier's severity and message beat the shape's for their constraint, and
/// a sibling constraint keeps the shape's.
#[test]
fn a_reifier_severity_and_message_beat_the_shapes() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity sh:Info ;
           sh:message \"shape message\" ;
           sh:nodeKind sh:Literal {| sh:severity sh:Warning ; sh:message \"reifier message\" |} ;
           sh:datatype xsd:string .",
        "",
    );
    assert_eq!(
        rows(&report),
        vec![
            row(
                "DatatypeConstraintComponent",
                "Info",
                &["\"shape message\""]
            ),
            row(
                "NodeKindConstraintComponent",
                "Warning",
                &["\"reifier message\""]
            ),
        ]
    );
}

/// A severity on the `sh:qualifiedMaxCount` statement grades the max bound only:
/// the qualified min and max are two constraints sharing the
/// `sh:qualifiedValueShape` statement, and each has its own T.
#[test]
fn a_qualified_bound_takes_its_own_annotation() {
    let shapes = |max_annotation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a, ex:b ;
               sh:property [ sh:path ex:p ;
                 sh:qualifiedValueShape [ sh:class ex:C ] ;
                 sh:qualifiedMinCount 1 ;
                 sh:qualifiedMaxCount 1 {max_annotation} ] ."
        )
    };
    // ex:a has no qualifying value (min fails); ex:b has two (max fails).
    let data = "ex:b ex:p ex:x, ex:y . ex:x a ex:C . ex:y a ex:C .";
    assert_eq!(
        rows(&validate(&shapes("{| sh:severity sh:Warning |}"), data)),
        vec![
            row("QualifiedMaxCountConstraintComponent", "Warning", &[]),
            row("QualifiedMinCountConstraintComponent", "Violation", &[]),
        ]
    );
    assert_eq!(
        rows(&validate(&shapes(""), data)),
        vec![
            row("QualifiedMaxCountConstraintComponent", "Violation", &[]),
            row("QualifiedMinCountConstraintComponent", "Violation", &[]),
        ]
    );
    // Deactivating the max bound leaves the min bound active.
    assert_eq!(
        rows(&validate(&shapes("{| sh:deactivated true |}"), data)),
        vec![row(
            "QualifiedMinCountConstraintComponent",
            "Violation",
            &[]
        )]
    );
}

/// `sh:flags` belongs to every pattern constraint on its shape, so a severity on
/// the `sh:flags` statement grades them.
#[test]
fn an_annotation_on_sh_flags_reaches_the_pattern_constraint() {
    let shapes = |annotation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode \"B\" ;
               sh:pattern \"^a\" ; sh:flags \"i\" {annotation} ."
        )
    };
    assert_eq!(
        rows(&validate(&shapes("{| sh:severity sh:Info |}"), "")),
        vec![row("PatternConstraintComponent", "Info", &[])]
    );
    assert_eq!(
        rows(&validate(&shapes(""), "")),
        vec![row("PatternConstraintComponent", "Violation", &[])]
    );
}

/// Two reifiers of one statement that AGREE load and apply; two that disagree
/// are refused, as are two statements of one constraint that disagree.
#[test]
fn conflicting_reifier_annotations_are_refused_and_agreeing_ones_load() {
    let two_reifiers = |a: &str, b: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .
             _:r1 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:severity {a} .
             _:r2 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:severity {b} ."
        )
    };
    refused(&two_reifiers("sh:Warning", "sh:Info"), "two severities");
    assert_eq!(
        rows(&validate(&two_reifiers("sh:Warning", "sh:Warning"), "")),
        vec![row("NodeKindConstraintComponent", "Warning", &[])]
    );

    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .
         _:r1 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:deactivated true .
         _:r2 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:deactivated false .",
        "both sh:deactivated true and false",
    );
    assert!(
        validate(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .
             _:r1 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:deactivated true .
             _:r2 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:deactivated true .",
            "",
        )
        .results
        .is_empty()
    );

    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .
         _:r1 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:message \"one\" .
         _:r2 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ; sh:message \"two\" .",
        "two different sh:message sets",
    );
    // One reifier with two language-tagged messages is ONE message set (the
    // specification's own example), and two reifiers stating that same set agree.
    assert_eq!(
        rows(&validate(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal .
             _:r1 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ;
               sh:message \"Too many\"@en , \"Zu viele\"@de .
             _:r2 rdf:reifies <<( ex:S sh:nodeKind sh:Literal )>> ;
               sh:message \"Zu viele\"@de , \"Too many\"@en .",
            "",
        )),
        vec![row(
            "NodeKindConstraintComponent",
            "Violation",
            &["\"Too many\"@en", "\"Zu viele\"@de"]
        )]
    );

    // T is every statement of one constraint: the shared sh:qualifiedValueShape
    // and the count may not state two severities for the min constraint …
    let qualified = |shape_annotation: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:property [ sh:path ex:p ;
                 sh:qualifiedValueShape [ sh:class ex:C ] {shape_annotation} ;
                 sh:qualifiedMinCount 1 {{| sh:severity sh:Info |}} ] ."
        )
    };
    refused(&qualified("{| sh:severity sh:Warning |}"), "two severities");
    // … and when they agree, the constraint takes the one severity.
    assert_eq!(
        rows(&validate(&qualified("{| sh:severity sh:Info |}"), "")),
        vec![row("QualifiedMinCountConstraintComponent", "Info", &[])]
    );
}

/// A value of the wrong kind is refused, and the right kind beside it loads.
#[test]
fn an_ill_typed_annotation_value_is_refused_and_a_well_typed_one_applies() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal {| sh:deactivated \"yes\" |} .",
        "must be an xsd:boolean literal",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal {| sh:severity \"Warning\" |} .",
        "must be an IRI",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal {| sh:message ex:m |} .",
        "must be an xsd:string",
    );
    assert_eq!(
        rows(&validate(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:nodeKind sh:Literal {| sh:severity ex:Advisory ; sh:message \"m\" |} .",
            "",
        )),
        vec![(
            "NodeKindConstraintComponent".to_owned(),
            "http://example.org/ns#Advisory".to_owned(),
            vec!["\"m\"".to_owned()]
        )]
    );
}

/// The three annotations exist only on a constraint's statements: on a target, a
/// shape characteristic or a non-SHACL statement they would apply to nothing, so
/// they are refused; on a parameter statement they apply.
#[test]
fn an_annotation_on_a_non_parameter_statement_is_refused_and_on_a_parameter_applies() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a {| sh:deactivated true |} ; sh:nodeKind sh:Literal .",
        "is not a constraint parameter",
    );
    refused(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ;
           rdfs:label \"S\" {| sh:severity sh:Warning |} .",
        "is not a constraint parameter",
    );
    assert!(
        validate(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal {| sh:deactivated true |} .",
            "",
        )
        .results
        .is_empty()
    );
}

/// An annotation on a triple term the shapes graph does NOT assert annotates no
/// constraint: it deactivates nothing and creates nothing.
#[test]
fn an_annotation_on_an_unasserted_triple_term_changes_nothing() {
    // Treatment: the reifier names `sh:minCount 2`, which is not asserted; the
    // asserted `sh:minCount 1` stays active, and no `sh:maxCount 0` appears.
    let treatment = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property ex:P .
         ex:P sh:path ex:p ; sh:minCount 1 .
         _:r1 rdf:reifies <<( ex:P sh:minCount 2 )>> ; sh:deactivated true .
         _:r2 rdf:reifies <<( ex:P sh:maxCount 0 )>> ; sh:severity sh:Warning .",
        "",
    );
    assert_eq!(
        rows(&treatment),
        vec![row("MinCountConstraintComponent", "Violation", &[])]
    );
    // Control: the same annotation on the ASSERTED statement deactivates it.
    let control = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property ex:P .
         ex:P sh:path ex:p ; sh:minCount 1 .
         _:r1 rdf:reifies <<( ex:P sh:minCount 1 )>> ; sh:deactivated true .",
        "",
    );
    assert!(control.results.is_empty());
}

/// The annotations travel in the prepared product: a restored preparation answers
/// exactly as the fresh parse does.
#[test]
fn annotations_survive_the_prepared_product() {
    let shapes = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:nodeKind sh:Literal {| sh:severity sh:Debug ; sh:message \"debug\" |} ;
           sh:property [ sh:path ex:p ; sh:minCount 1 {| sh:severity sh:Trace |} ;
                         sh:maxCount 0 {| sh:deactivated true |} ] .",
    );
    let fresh = validate_with(&shapes, "ex:a ex:p 1 .");
    let product = PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("the annotated shapes pack");
    let restored = ShapesProduct::open(&product)
        .expect("the product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product admits");
    let again = validate_with(restored.shapes(), "ex:a ex:p 1 .");
    assert_eq!(fresh.to_ntriples(), again.to_ntriples());
    assert_eq!(
        rows(&again),
        vec![row("NodeKindConstraintComponent", "Debug", &["\"debug\""])]
    );
}

// ── sh:Debug / sh:Trace and the conformance-disallow set ─────────────────────

/// `sh:Debug` and `sh:Trace` results are reported and, under the default set, do
/// not block conformance; a `sh:Warning` result does.
#[test]
fn debug_and_trace_results_conform_by_default_and_warning_does_not() {
    let shapes = |severity: &str| {
        format!(
            "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity {severity} ; sh:nodeKind sh:Literal ."
        )
    };
    for (severity, conforms) in [
        ("sh:Debug", true),
        ("sh:Trace", true),
        ("sh:Info", false),
        ("sh:Warning", false),
        ("sh:Violation", false),
    ] {
        let report = validate(&shapes(severity), "");
        assert_eq!(report.results.len(), 1, "{severity}");
        assert_eq!(report.conforms, conforms, "{severity}");
    }
}

/// A Warning-only report does not conform under the default set, conforms when
/// the set is {Violation}, and echoes exactly the set it was judged against —
/// nothing for the default (which is what "no triples" means), the set otherwise.
#[test]
fn a_warning_only_report_follows_the_set_and_echoes_it() {
    let mut shapes = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .",
    );
    let default_report = validate_with(&shapes, "");
    assert!(!default_report.conforms);
    assert!(
        !default_report
            .to_ntriples()
            .contains("conformanceDisallows")
    );
    assert_eq!(
        conformance_disallows_from_dataset(&default_report.to_dataset()).expect("readable"),
        ConformanceDisallows::default()
    );

    shapes.set_validation_options(disallowing(&[Severity::Violation]));
    let report = validate_with(&shapes, "");
    assert!(
        report.conforms,
        "a Warning is not disallowed by {{Violation}}"
    );
    assert_eq!(report.results.len(), 1, "the result is still reported");
    assert_eq!(
        conformance_disallows_from_dataset(&report.to_dataset()).expect("readable"),
        ConformanceDisallows::new([Severity::Violation]).expect("non-empty")
    );
    assert!(report.to_ntriples().contains(
        "<http://www.w3.org/ns/shacl#conformanceDisallows> <http://www.w3.org/ns/shacl#Violation>"
    ));
}

/// Debug-only results conform by default and do not once `sh:Debug` is in the
/// set; a custom severity blocks conformance only when the set names it.
#[test]
fn debug_and_custom_severities_block_only_when_listed() {
    let mut debug = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity sh:Debug ; sh:nodeKind sh:Literal .",
    );
    assert!(validate_with(&debug, "").conforms);
    debug.set_validation_options(disallowing(&[Severity::Debug]));
    assert!(!validate_with(&debug, "").conforms);

    let advisory = Severity::Other(NamedNode::from("http://example.org/ns#Advisory"));
    let mut custom = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity ex:Advisory ; sh:nodeKind sh:Literal .",
    );
    assert!(
        validate_with(&custom, "").conforms,
        "not in the default set"
    );
    custom.set_validation_options(disallowing(&[Severity::Violation, advisory]));
    assert!(!validate_with(&custom, "").conforms, "listed");
}

/// Nested conformance checking uses the same set: under the default set an inner
/// Warning makes `sh:node` fail, under {Violation} it does not, and an inner Debug
/// never does — while `sh:not` over an inner Debug-only shape sees a CONFORMING
/// shape and so reports.
#[test]
fn nested_conformance_checks_judge_against_the_same_set() {
    let node = |inner_severity: &str| {
        format!(
            "ex:Outer a sh:NodeShape ; sh:targetNode ex:a ; sh:node ex:Inner .
             ex:Inner a sh:NodeShape ; sh:severity {inner_severity} ; sh:nodeKind sh:Literal ."
        )
    };
    let outer_rows = |report: &ValidationReport| -> Vec<String> {
        report
            .results
            .iter()
            .map(|r| r.source_constraint_component.as_str().to_owned())
            .collect()
    };
    assert_eq!(
        outer_rows(&validate(&node("sh:Warning"), "")),
        vec![sh("NodeConstraintComponent")]
    );
    let mut relaxed = loads(&node("sh:Warning"));
    relaxed.set_validation_options(disallowing(&[Severity::Violation]));
    assert_eq!(
        outer_rows(&validate_with(&relaxed, "")),
        Vec::<String>::new()
    );
    assert_eq!(
        outer_rows(&validate(&node("sh:Debug"), "")),
        Vec::<String>::new()
    );

    let not = "ex:Outer a sh:NodeShape ; sh:targetNode ex:a ; sh:not ex:Inner .
               ex:Inner a sh:NodeShape ; sh:severity sh:Debug ; sh:nodeKind sh:Literal .";
    assert_eq!(
        outer_rows(&validate(not, "")),
        vec![sh("NotConstraintComponent")]
    );
    let not_violation = "ex:Outer a sh:NodeShape ; sh:targetNode ex:a ; sh:not ex:Inner .
               ex:Inner a sh:NodeShape ; sh:nodeKind sh:Literal .";
    assert_eq!(
        outer_rows(&validate(not_violation, "")),
        Vec::<String>::new()
    );
}

/// A preparation restored from a product takes a request's set.
#[test]
fn a_restored_preparation_takes_the_request_set() {
    let shapes = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:severity sh:Warning ; sh:nodeKind sh:Literal .",
    );
    let product = PreparedShapes::new(Arc::new(shapes))
        .to_product(&ShapesProfile::CORE)
        .expect("packs");
    let restored = || {
        ShapesProduct::open(&product)
            .expect("opens")
            .admit(&ShapesProfile::CORE, &HostBindings::empty())
            .expect("admits")
    };
    let validate_prepared = |prepared: &PreparedShapes| {
        prepared
            .bind_shared_dataset(data(""))
            .expect("binds")
            .validate()
            .expect("validates")
    };
    assert!(!validate_prepared(&restored()).conforms);
    let relaxed = restored().with_validation_options(disallowing(&[Severity::Violation]));
    assert!(validate_prepared(&relaxed).conforms);
}

/// The empty set has no report that states it and is refused; a one-level set
/// beside it is accepted. A non-IRI level is refused; an IRI is accepted.
#[test]
fn an_empty_or_non_iri_set_is_refused_and_a_named_one_is_accepted() {
    assert!(
        ConformanceDisallows::new(Vec::new())
            .expect_err("empty")
            .contains("empty")
    );
    assert!(ConformanceDisallows::new([Severity::Info]).is_ok());
    assert!(
        ConformanceDisallows::from_iris(["not an iri"])
            .expect_err("not an IRI")
            .contains("not an absolute IRI")
    );
    let parsed = ConformanceDisallows::from_iris([
        "http://www.w3.org/ns/shacl#Trace",
        "http://example.org/ns#Advisory",
    ])
    .expect("IRIs");
    assert!(parsed.contains(&Severity::Trace));
    assert!(!parsed.contains(&Severity::Violation));
    assert_eq!(
        parsed.iris(),
        vec![
            "http://www.w3.org/ns/shacl#Trace".to_owned(),
            "http://example.org/ns#Advisory".to_owned()
        ]
    );
}

// ── Every message, tagged ─────────────────────────────────────────────────────

/// SHACL 1.2 Core copies ALL of a shape's messages into each result: two
/// language-tagged messages produce both, each keeping its tag, in the report
/// graph too; a single untagged message stays a single untagged message.
#[test]
fn every_shape_message_reaches_the_result_with_its_language_tag() {
    let tagged = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ;
           sh:message \"Too many\"@en , \"Zu viele\"@de .",
        "",
    );
    assert_eq!(
        rows(&tagged),
        vec![row(
            "NodeKindConstraintComponent",
            "Violation",
            &["\"Too many\"@en", "\"Zu viele\"@de"]
        )]
    );
    let nt = tagged.to_ntriples();
    assert!(
        nt.contains("<http://www.w3.org/ns/shacl#resultMessage> \"Too many\"@en ."),
        "{nt}"
    );
    assert!(
        nt.contains("<http://www.w3.org/ns/shacl#resultMessage> \"Zu viele\"@de ."),
        "{nt}"
    );

    let plain = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:nodeKind sh:Literal ; sh:message \"one\" .",
        "",
    );
    assert_eq!(
        rows(&plain),
        vec![row(
            "NodeKindConstraintComponent",
            "Violation",
            &["\"one\""]
        )]
    );
    let nt = plain.to_ntriples();
    assert_eq!(nt.matches("resultMessage").count(), 1, "{nt}");
    assert!(
        nt.contains("<http://www.w3.org/ns/shacl#resultMessage> \"one\" ."),
        "{nt}"
    );
}

/// A reifier's message set replaces the shape's for its constraint, every
/// message kept and tagged — including a base direction and an `rdf:HTML`
/// message, both of which SHACL permits.
#[test]
fn a_reifier_message_set_reaches_the_result_whole() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:message \"shape\" ;
           sh:nodeKind sh:Literal {| sh:message \"Too many\"@en , \"Zu viele\"@de--ltr ,
             \"<b>bold</b>\"^^rdf:HTML |} ;
           sh:datatype xsd:string .",
        "",
    );
    assert_eq!(
        rows(&report),
        vec![
            row("DatatypeConstraintComponent", "Violation", &["\"shape\""]),
            row(
                "NodeKindConstraintComponent",
                "Violation",
                &[
                    "\"<b>bold</b>\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML>",
                    "\"Too many\"@en",
                    "\"Zu viele\"@de--ltr"
                ]
            ),
        ]
    );
}
