// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The class plan must cover every class evaluation can reach — and must keep the
//! two "no id" answers apart.
//!
//! `ValidationPlan::class_id` answers a class IRI with its interned dataset id.
//! There are two negative answers and they are NOT the same condition:
//!
//! * the class is planned but the DATA GRAPH never names it — entirely normal,
//!   read as "nothing is an instance"; and
//! * the class is missing from the CATALOG — a defect in the planning walk, which
//!   used to be an `expect`, i.e. a panic, and a panic ABORTS across the PyO3 and
//!   C ABI boundaries rather than surfacing as an error a host can handle.
//!
//! The second is now an error; the first must NOT have become one. Both
//! directions are executed here.
//!
//! A shape reachable only through the `sh:nodeByExpression` shape index is the
//! case that exposes the gap, because the plans `constraints::conforms` and the
//! rules engine build cover ONE shape rather than the whole shapes graph.
//!
//! Test IRIs live under `example.org`.

use std::sync::Arc;

use purrdf_shapes::constraints::conforms;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{parse_shapes, validate_dataset};
use purrdf_shapes::shapes::Shape;
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
";

fn ex(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/ns#{local}"
    )))
}

// ── The believed-INVALID direction: a class the walk never planned ────────────

/// A shape reached ONLY through the `sh:nodeByExpression` shape index names an
/// `sh:class` the outer shape never mentions. `constraints::conforms` plans a
/// single shape, so before the walk entered the index this class was absent from
/// the catalog and `class_id` aborted the process.
///
/// The assertion is on the VERDICT, not on the absence of a panic alone: closing
/// the gap has to leave the constraint evaluating correctly, and an evaluation
/// that merely stopped panicking while answering "cannot match" for every node
/// would be the over-refusal this fix must not introduce.
#[test]
fn a_shape_reached_only_through_the_shape_index_is_planned() {
    let shapes_ttl = r"
ex:Outer a sh:NodeShape ;
    sh:targetClass ex:Thing ;
    sh:property [ sh:path ex:ref ; sh:nodeByExpression ex:Indexed ] .

ex:Indexed a sh:NodeShape ;
    sh:property [ sh:path ex:kind ; sh:class ex:Special ] .
";
    let data_ttl = r"
ex:good a ex:Thing ; ex:ref ex:r1 .
ex:bad  a ex:Thing ; ex:ref ex:r2 .
ex:r1 ex:kind ex:k1 .
ex:r2 ex:kind ex:k2 .
ex:k1 a ex:Special .
ex:k2 a ex:Ordinary .
";
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let shapes_ds: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes data");
    let store = ShaclData::new(Arc::clone(&data), shapes_ds, None);

    let outer: &Shape = shapes
        .node_shapes
        .iter()
        .find(|s| s.id == ex("Outer"))
        .expect("ex:Outer is a top-level shape");

    // `conforms` builds a plan for ex:Outer ALONE. ex:Special is reachable only
    // through the index entry ex:Indexed.
    assert!(
        conforms(&store, &ex("good"), outer).expect("the single-shape plan covers ex:Special"),
        "ex:good's referenced node has an ex:Special kind, so it conforms"
    );
    assert!(
        !conforms(&store, &ex("bad"), outer).expect("the single-shape plan covers ex:Special"),
        "ex:bad's referenced node has a non-Special kind, so it does not conform"
    );
}

// ── The neighbouring VALID direction: a class with no instances ───────────────

/// A shapes graph may name an `sh:class` the data graph genuinely lacks. That is
/// a soft "cannot match", not an error: the constraint must still EVALUATE and
/// report the value nodes that are not instances of it.
#[test]
fn a_class_absent_from_the_data_graph_still_reports_a_violation() {
    let shapes_ttl = r"
ex:S a sh:NodeShape ;
    sh:targetClass ex:Thing ;
    sh:property [ sh:path ex:ref ; sh:class ex:NeverInstantiated ] .
";
    let data_ttl = r"
ex:a a ex:Thing ; ex:ref ex:r1 .
";
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let report =
        validate_dataset(&data, &shapes).expect("a class with no instances is not an error");
    assert!(
        !report.conforms,
        "ex:r1 is no instance of ex:NeverInstantiated"
    );
    assert_eq!(report.results.len(), 1, "exactly one sh:class violation");
    assert_eq!(report.results[0].focus_node, ex("a"));
    assert_eq!(
        report.results[0].source_constraint_component,
        NamedNode::new_unchecked("http://www.w3.org/ns/shacl#ClassConstraintComponent".to_owned()),
        "the constraint was EVALUATED, not skipped"
    );
}

/// The same soft `None` on the TARGET side: `sh:targetClass` naming a class the
/// data never instantiates resolves to an empty focus set, not a failure.
#[test]
fn a_target_class_absent_from_the_data_graph_is_an_empty_target_set() {
    let shapes_ttl = r"
ex:S a sh:NodeShape ;
    sh:targetClass ex:NeverInstantiated ;
    sh:property [ sh:path ex:ref ; sh:minCount 1 ] .
";
    let data_ttl = r"
ex:a a ex:Thing .
";
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let report = validate_dataset(&data, &shapes).expect("an empty target class is not an error");
    assert!(report.conforms, "no focus node, so nothing to report");
    assert!(report.results.is_empty());
}
