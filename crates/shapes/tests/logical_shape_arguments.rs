// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The shape ARGUMENT of a logical constraint may itself be a PROPERTY shape.
//!
//! SHACL §4.6 states every logical constraint (`sh:not`, `sh:and`, `sh:or`,
//! `sh:xone`, `sh:node`) and `sh:qualifiedValueShape` (§4.7.3) in terms of "the
//! value node conforms to $shape" — and §2.1 lets ANY shape be either a node
//! shape or a property shape. A shape argument carrying `sh:path` therefore
//! scopes its constraints to that path's value nodes, exactly as it would under
//! `sh:property`. Dropping the path silently re-reads a path-scoped constraint as
//! a node-scoped one, which is how `sh:not [ sh:path ex:p ; sh:minCount 1 ]` came
//! to report a violation for a node with NO `ex:p` value at all: `sh:minCount 1`
//! read against the single value node `$this` is trivially satisfied, so the
//! negated shape "conformed" no matter what the data said.
//!
//! The same rule governs `sh:reifierShape` (RDF 1.2 SHACL), where the drop shows
//! up with the opposite sign: there a discarded path makes the reifier shape
//! check NOTHING, so invalid data passes silently.
//!
//! Every test drives the production surface (`engine::parse_shapes` +
//! `engine::validate_dataset`) and asserts BOTH directions — the node that must
//! be reported and the node that must not — so a fix cannot trade one wrong
//! answer for the other. The W3C corpus cannot hold this: none of its `sh:not`
//! vectors gives the constraint a shape argument with `sh:path`.

use std::sync::Arc;

use purrdf_shapes::engine::{parse_shapes, validate_dataset};
use purrdf_shapes::term::Term;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
";

/// Two `ex:Thing`s that differ only in whether they carry an `ex:flag` value.
const DATA: &str = r#"
ex:hasFlag a ex:Thing ; ex:flag "yes" .
ex:noFlag  a ex:Thing .
"#;

/// The local names of the focus nodes reported against `shapes_ttl`, sorted and
/// deduplicated so an assertion names nodes rather than result counts.
fn reported_focus_nodes(shapes_ttl: &str) -> Vec<String> {
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{DATA}"), None).expect("data parse");
    let report = validate_dataset(&data, &shapes).expect("validation runs");
    let mut focus: Vec<String> = report
        .results
        .iter()
        .map(|r| match &r.focus_node {
            Term::NamedNode(n) => n
                .as_str()
                .rsplit('#')
                .next()
                .expect("an IRI always yields a final segment")
                .to_owned(),
            other => other.to_string(),
        })
        .collect();
    focus.sort();
    focus.dedup();
    focus
}

/// Assert the exact set of reported focus nodes, naming both directions.
fn assert_reports(shapes_ttl: &str, expected: &[&str], what: &str) {
    let got = reported_focus_nodes(shapes_ttl);
    assert_eq!(got, expected, "{what}");
}

// ── sh:not (SHACL §4.6.1) ─────────────────────────────────────────────────────

/// `sh:not` over an ANONYMOUS property shape. `ex:noFlag` has zero `ex:flag`
/// values, fails `sh:minCount 1`, and therefore does NOT conform — so `sh:not`
/// must stay silent about it.
#[test]
fn not_over_anonymous_property_shape() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:not [ sh:path ex:flag ; sh:minCount 1 ] .
",
        &["hasFlag"],
        "sh:not over an anonymous property shape must report only the node that \
         CONFORMS to the negated shape",
    );
}

/// The same shape written as a NAMED `sh:PropertyShape`. The spelling of the
/// argument is not a semantic distinction; the answer must not move.
#[test]
fn not_over_named_property_shape() {
    assert_reports(
        r"
ex:Inner a sh:PropertyShape ; sh:path ex:flag ; sh:minCount 1 .
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; sh:not ex:Inner .
",
        &["hasFlag"],
        "sh:not over a named property shape must agree with the anonymous spelling",
    );
}

/// The node-shape spelling, which was already correct. Pinned so the fix for the
/// two above cannot trade one wrong answer for another.
#[test]
fn not_over_node_shape_wrapping_a_property_shape() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:not [ a sh:NodeShape ; sh:property [ sh:path ex:flag ; sh:minCount 1 ] ] .
",
        &["hasFlag"],
        "the node-shape spelling of sh:not must keep its (already correct) answer",
    );
}

// ── sh:node (SHACL §4.6.6) ────────────────────────────────────────────────────

/// `sh:node` is the positive form: the focus node must CONFORM. `ex:noFlag`
/// fails the path-scoped `sh:minCount 1` and must be reported; `ex:hasFlag`
/// satisfies it and must not be.
#[test]
fn node_over_anonymous_property_shape() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:node [ sh:path ex:flag ; sh:minCount 1 ] .
",
        &["noFlag"],
        "sh:node over an anonymous property shape must report the node that fails \
         the path-scoped constraint",
    );
}

#[test]
fn node_over_named_property_shape() {
    assert_reports(
        r"
ex:Inner a sh:PropertyShape ; sh:path ex:flag ; sh:minCount 1 .
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; sh:node ex:Inner .
",
        &["noFlag"],
        "sh:node over a named property shape must agree with the anonymous spelling",
    );
}

#[test]
fn node_over_node_shape_wrapping_a_property_shape() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:node [ a sh:NodeShape ; sh:property [ sh:path ex:flag ; sh:minCount 1 ] ] .
",
        &["noFlag"],
        "the node-shape spelling of sh:node must keep its answer",
    );
}

// ── sh:and / sh:or / sh:xone (SHACL §4.6.2–§4.6.4) ────────────────────────────

#[test]
fn and_over_property_shape_member() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:and ( [ sh:path ex:flag ; sh:minCount 1 ] ) .
",
        &["noFlag"],
        "sh:and with a property-shape member must report the node failing the \
         path-scoped constraint",
    );
}

#[test]
fn or_over_property_shape_members() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:or ( [ sh:path ex:flag ; sh:minCount 1 ] [ sh:path ex:other ; sh:minCount 1 ] ) .
",
        &["noFlag"],
        "sh:or with property-shape members must report only the node conforming to \
         NEITHER member",
    );
}

#[test]
fn xone_over_property_shape_members() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:xone ( [ sh:path ex:flag ; sh:minCount 1 ] [ sh:path ex:flag ; sh:maxCount 0 ] ) .
",
        &[],
        "sh:xone with property-shape members: each node conforms to exactly one, so \
         nothing is reported",
    );
}

/// The mirror of the above: both members accept `ex:hasFlag` (it has one
/// `ex:flag` value, so `sh:minCount 1` and `sh:maxCount 1` both hold) while
/// `ex:noFlag` conforms only to the second. Exactly one node is reported.
#[test]
fn xone_reports_the_node_matching_both_members() {
    assert_reports(
        r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:xone ( [ sh:path ex:flag ; sh:minCount 1 ] [ sh:path ex:flag ; sh:maxCount 1 ] ) .
",
        &["hasFlag"],
        "sh:xone must report the node conforming to BOTH members",
    );
}

// ── sh:qualifiedValueShape (SHACL §4.7.3) ─────────────────────────────────────

/// The qualified value shape is itself a property shape: each `ex:item` value is
/// the focus of a path-scoped `sh:minCount`, so only the item carrying an
/// `ex:flag` counts toward `sh:qualifiedMinCount`.
#[test]
fn qualified_value_shape_that_is_a_property_shape() {
    let shapes_ttl = r"
ex:S a sh:NodeShape ; sh:targetClass ex:Box ;
    sh:property [
        sh:path ex:item ;
        sh:qualifiedValueShape [ sh:path ex:flag ; sh:minCount 1 ] ;
        sh:qualifiedMinCount 1 ;
    ] .
";
    let data_ttl = r#"
ex:good a ex:Box ; ex:item ex:withFlag .
ex:bad  a ex:Box ; ex:item ex:bare .
ex:withFlag ex:flag "yes" .
"#;
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let report = validate_dataset(&data, &shapes).expect("validation runs");
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect();
    assert_eq!(
        focus,
        vec!["<http://example.org/ns#bad>".to_owned()],
        "only the box whose item fails the path-scoped qualified shape is reported"
    );
}

// ── sh:reifierShape (RDF 1.2 SHACL) ───────────────────────────────────────────

/// A reifier shape carrying `sh:path` constrains the REIFIER's values under that
/// path. Discarding the path here is a silent drop rather than a wrong answer:
/// `sh:minCount 1` read against the single value node `$this` always holds, so
/// every reifier would pass and the shape would check nothing.
#[test]
fn reifier_shape_that_is_a_property_shape() {
    let shapes_ttl = r"
ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
    sh:property [
        sh:path ex:p ;
        sh:reifierShape [ sh:path ex:certainty ; sh:minCount 1 ] ;
    ] .
";
    let data_ttl = r#"
ex:annotated a ex:Thing ; ex:p ex:v1 {| ex:certainty "high" |} .
ex:bare      a ex:Thing ; ex:p ex:v2 {| ex:source ex:s |} .
"#;
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).expect("shapes parse");
    let data: Arc<_> =
        parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parse");
    let report = validate_dataset(&data, &shapes).expect("validation runs");
    let focus: Vec<String> = report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect();
    assert_eq!(
        focus,
        vec!["<http://example.org/ns#bare>".to_owned()],
        "only the statement whose reifier lacks ex:certainty is reported"
    );
}
