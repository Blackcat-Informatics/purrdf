// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Catchable SPARQL argument failures through standalone and dated validation.

#[path = "support/turtle.rs"]
mod turtle;

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::expression::{RecursionGuard, eval_node_expr};
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use std::sync::Arc;

const PREFIXES: &str = r"
@prefix ex:     <http://example.org/ns#> .
@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
";

/// Evaluate a named root through the standalone public boundary.
fn free_outputs(body: &str) -> Result<Vec<String>, purrdf_shapes::ShapesError> {
    let document =
        purrdf_shapes::text_ingest::parse_turtle_document(&format!("{PREFIXES}{body}"), None)
            .expect("shapes parse");
    let data = parse_turtle_to_dataset("", None).expect("empty data");
    let evaluated = purrdf_shapes::free_expression::evaluate(
        &purrdf_shapes::free_expression::FreeExpression {
            shapes: &document.dataset,
            prefixes: &document.prefixes,
            root: &Term::blank("e"),
            data: data.as_ref(),
            focus: &Term::NamedNode(NamedNode::new_unchecked("http://example.org/n")),
            scope: &[],
            imports: &purrdf_shapes::ShapesImports::new(),
        },
    )?;
    Ok(evaluated.outputs.iter().map(ToString::to_string).collect())
}

fn sparql_node_calls_mask_catchable_argument_failures() {
    let error = "[ shnex:instancesOf 42 ]";
    for (call, expected) in [
        (format!("coalesce (\"ok\" {error})"), "\"ok\""),
        (format!("logical-and (false {error})"), "false"),
        (format!("logical-and ({error} false)"), "false"),
        (format!("logical-or (true {error})"), "true"),
        (format!("logical-or ({error} true)"), "true"),
        (format!("coalesce ({error} \"ok\")"), "\"ok\""),
        (format!("if (true \"ok\" {error})"), "\"ok\""),
        (format!("if (false {error} \"ok\")"), "\"ok\""),
        (format!("in (ex:x {error} ex:x)"), "true"),
        (format!("not-in (ex:x {error} ex:x)"), "false"),
    ] {
        let expected = if expected.starts_with('"') {
            expected.to_owned()
        } else {
            format!("\"{expected}\"^^<{}>", purrdf_xsd::datatype::XSD_BOOLEAN)
        };
        assert_eq!(
            free_outputs(&format!("_:e sparql:{call} ."))
                .unwrap_or_else(|error| panic!("{call}: {error}")),
            vec![expected],
            "{call}",
        );
    }
}

fn sparql_node_calls_preserve_unmasked_failures_and_empty_branches() {
    let error = "[ shnex:instancesOf 42 ]";
    for call in [
        format!("coalesce ({error} {error})"),
        format!("logical-and (true {error})"),
        format!("logical-and ({error} true)"),
        format!("logical-or (false {error})"),
        format!("logical-or ({error} false)"),
        format!("if (true {error} \"ok\")"),
        format!("if (false \"ok\" {error})"),
        format!("if ({error} \"ok\" \"ok\")"),
        format!("in (ex:x {error} ex:y)"),
        format!("not-in (ex:x {error} ex:y)"),
        format!("multiply ({error} 0)"),
    ] {
        let failure =
            free_outputs(&format!("_:e sparql:{call} .")).expect_err("unmasked nested failure");
        assert_eq!(
            failure.failure_class(),
            purrdf_shapes::FailureClass::Evaluation
        );
    }
    for call in [
        format!("if (true [ shnex:pathValues ex:missing ] {error})"),
        format!("if (false {error} [ shnex:pathValues ex:missing ])"),
    ] {
        assert_eq!(
            free_outputs(&format!("_:e sparql:{call} ."))
                .expect("selected empty branch is not a failure"),
            Vec::<String>::new(),
        );
    }
}

fn sparql_argument_failures_follow_the_same_path_in_dated_validation() {
    use purrdf_core::xsd_regex::xpath::{Limits, Profile};
    use purrdf_shapes::engine::{PreparedShapes, ValidationOptions};
    use purrdf_shapes::profile::ShaclProfile;
    for (call, conforms) in [
        ("coalesce (true [ shnex:instancesOf 42 ])", true),
        ("logical-or ([ shnex:instancesOf 42 ] true)", true),
        ("logical-and (false [ shnex:instancesOf 42 ])", false),
    ] {
        let shapes = turtle::loads(
            PREFIXES,
            &format!("ex:S a sh:NodeShape; sh:targetNode ex:n; sh:expression [ sparql:{call} ] ."),
        );
        let prepared = PreparedShapes::new(Arc::new(shapes)).with_validation_options(
            ValidationOptions::default()
                .with_profile(ShaclProfile::ED_20261016)
                .with_xpath_regex(Profile::Xpath31, Limits::new()),
        );
        let report = prepared
            .bind_shared_dataset(turtle::data(PREFIXES, ""))
            .expect("dated request admission")
            .validate()
            .unwrap_or_else(|error| panic!("{call}: {error}"));
        assert_eq!(report.conforms, conforms, "{call}");
    }
}

fn sparql_node_calls_cannot_mask_a_nested_processor_depth_refusal() {
    use purrdf_shapes::expression::MAX_RECURSION_DEPTH;
    use purrdf_shapes::{ProcessorFailure, ShapesError};
    let expr = turtle::expression_of(
        PREFIXES,
        r#"
        ex:S a sh:NodeShape; sh:expression [ sparql:coalesce (
            "ok" [ sh:filterShape [ sh:nodeKind sh:IRI ]; sh:nodes sh:this ]
        ) ] .
    "#,
    );
    let data = parse_turtle_to_dataset("", None).expect("empty data");
    let store = ShaclData::new(Arc::clone(&data), data, None);
    let mut guard = RecursionGuard::with_depth(MAX_RECURSION_DEPTH);
    let failure = eval_node_expr(
        &store,
        &Term::NamedNode(NamedNode::new_unchecked("http://example.org/n")),
        &expr,
        &mut guard,
    )
    .expect_err("processor failure must escape even after a successful argument");
    assert!(
        matches!(
            failure,
            ShapesError::Processor(ProcessorFailure::ValidationDepth {
                depth: 65,
                limit: 64,
                ..
            })
        ),
        "{failure:?}"
    );
}

purrdf_testkit::harness_main!(
    sparql_node_calls_mask_catchable_argument_failures,
    sparql_node_calls_preserve_unmasked_failures_and_empty_branches,
    sparql_argument_failures_follow_the_same_path_in_dated_validation,
    sparql_node_calls_cannot_mask_a_nested_processor_depth_refusal,
);
