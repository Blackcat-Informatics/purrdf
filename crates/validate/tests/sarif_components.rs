// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SARIF for the SHACL 1.2 constraint components, observed end to end.
//!
//! Each `sarif_<component>` test validates a data graph that fails the
//! component through the real engine, renders the report to SARIF, and asserts
//! the result's rule id and the rule descriptor's curated one-line summary and
//! `helpUri` — the anchor of the specification that defines the component.
//! `sarif_detail_related_locations` asserts that the nested results of a
//! `sh:memberShape` failure (`sh:detail`) reach SARIF as `relatedLocations`.

use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_validate::{SarifOptions, report_to_sarif_string};
use serde_json::{Value, json};

const PREFIXES: &str = "
    @prefix ex:   <http://example.org/> .
    @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix sh:   <http://www.w3.org/ns/shacl#> .
    @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
";

const SH: &str = "http://www.w3.org/ns/shacl#";
const SHACL: &str = "https://www.w3.org/TR/shacl/";
const SHACL12_CORE: &str = "https://www.w3.org/TR/shacl12-core/";

/// The SARIF log of validating `data` against `shapes` (both Turtle bodies).
fn sarif(shapes: &str, data: &str) -> Value {
    let shapes = parse_shapes(&format!("{PREFIXES}{shapes}"), None).expect("shapes graph");
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}{data}"), None).expect("data graph");
    let report = validate_dataset_with_shapes_graph(&data, &shapes, None).expect("validation");
    serde_json::from_str(&report_to_sarif_string(&report, &SarifOptions::default()))
        .expect("SARIF JSON")
}

/// Assert that `log` reports the component `local` under its rule id, and that
/// the rule table describes it with `summary` and the `spec` anchor.
#[track_caller]
fn assert_component(log: &Value, local: &str, summary: &str, spec: &str) {
    let rule_id = format!("{SH}{local}");
    let run = &log["runs"][0];
    let results = run["results"].as_array().expect("results");
    assert!(
        results
            .iter()
            .any(|result| result["ruleId"] == rule_id.as_str()),
        "a {local} result: {results:#?}"
    );
    let rule = run["tool"]["driver"]["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .find(|rule| rule["id"] == rule_id.as_str())
        .expect("the component's rule descriptor");
    assert_eq!(rule["name"], local);
    assert_eq!(rule["shortDescription"]["text"], summary);
    assert_eq!(rule["helpUri"], format!("{spec}#{local}"));
}

#[test]
fn sarif_min_list_length() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:minListLength 2 ] .",
        "ex:h a ex:C ; ex:list ( ex:a ) .",
    );
    assert_component(
        &log,
        "MinListLengthConstraintComponent",
        "A value is not a list of at least the minimum length (sh:minListLength).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_max_list_length() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:maxListLength 2 ] .",
        "ex:h a ex:C ; ex:list ( ex:a ex:b ex:c ) .",
    );
    assert_component(
        &log,
        "MaxListLengthConstraintComponent",
        "A value is not a list of at most the maximum length (sh:maxListLength).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_unique_members() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:uniqueMembers true ] .",
        "ex:h a ex:C ; ex:list ( ex:a ex:a ) .",
    );
    assert_component(
        &log,
        "UniqueMembersConstraintComponent",
        "A value is not a list, or a list with a repeated member (sh:uniqueMembers).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_member_shape() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:memberShape [ sh:nodeKind sh:IRI ] ] .",
        "ex:h a ex:C ; ex:list ( ex:a \"b\" ) .",
    );
    assert_component(
        &log,
        "MemberShapeConstraintComponent",
        "A value is not a list whose every member conforms to the shape (sh:memberShape).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_single_line() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:label ; sh:singleLine true ] .",
        "ex:h a ex:C ; ex:label \"two\\nlines\" .",
    );
    assert_component(
        &log,
        "SingleLineConstraintComponent",
        "A literal's lexical form contains a line break (sh:singleLine).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_root_class() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:kind ; sh:rootClass ex:Root ] .",
        "ex:h a ex:C ; ex:kind ex:Other .  ex:Leaf rdfs:subClassOf ex:Root .",
    );
    assert_component(
        &log,
        "RootClassConstraintComponent",
        "A value is not a root class or one of its subclasses (sh:rootClass).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_some_value() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:someValue [ sh:datatype xsd:integer ] ] .",
        "ex:h a ex:C ; ex:p \"x\" .",
    );
    assert_component(
        &log,
        "SomeValueConstraintComponent",
        "No value conforms to the shape (sh:someValue).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_subset_of() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:subsetOf ex:q ] .",
        "ex:h a ex:C ; ex:p ex:a ; ex:q ex:b .",
    );
    assert_component(
        &log,
        "SubsetOfConstraintComponent",
        "A value is not among the values of another path (sh:subsetOf).",
        SHACL12_CORE,
    );
}

#[test]
fn sarif_unique_values_for() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:uniqueValuesFor ex:serial .",
        "ex:h a ex:C ; ex:serial 7 .  ex:i a ex:C ; ex:serial 7 .",
    );
    assert_component(
        &log,
        "UniqueValuesForConstraintComponent",
        "Another target node has the same values for the listed properties (sh:uniqueValuesFor).",
        SHACL12_CORE,
    );
}

/// `sh:closed sh:ByTypes` reports through `sh:ClosedConstraintComponent`, the
/// component `sh:closed true` uses, so it shares that rule.
#[test]
fn sarif_closed_by_types() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ; sh:closed sh:ByTypes .
         ex:C a rdfs:Class ; sh:property [ sh:path ex:p ] .",
        "ex:h a ex:C ; ex:p 1 ; ex:stray 2 .",
    );
    assert_component(
        &log,
        "ClosedConstraintComponent",
        "A node has properties outside a closed shape (sh:closed).",
        SHACL,
    );
}

/// A `sh:memberShape` failure is detailed by the results of each list member
/// that does not conform to the member shape; each becomes a related location
/// after the source shape, carrying the member as its focus node, the member
/// shape's component and shape, and the member result's message.
#[test]
fn sarif_detail_related_locations() {
    let log = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:memberShape ex:Member ] .
         ex:Member a sh:NodeShape ; sh:nodeKind sh:IRI ; sh:message \"a member must be an IRI\" .",
        "ex:h a ex:C ; ex:list ( ex:a \"b\" \"c\" ) .",
    );
    let results = log["runs"][0]["results"].as_array().expect("results");
    let [result] = results.as_slice() else {
        panic!("one result: {results:#?}");
    };
    assert_eq!(
        result["ruleId"],
        format!("{SH}MemberShapeConstraintComponent")
    );
    let related = result["relatedLocations"]
        .as_array()
        .expect("relatedLocations");
    let detail = |member: &str| {
        json!({
            "logicalLocations": [
                { "name": member, "kind": "focusNode" },
                { "name": format!("{SH}NodeKindConstraintComponent"), "kind": "constraintComponent" },
                { "name": "<http://example.org/Member>", "kind": "sourceShape" },
            ],
            "message": { "text": "sh:detail: a member must be an IRI" },
        })
    };
    assert_eq!(related[1..], [detail("b"), detail("c")]);
    assert_eq!(related[0]["message"]["text"], "shape defined here");

    // The control: a list whose every member conforms produces no result, so no
    // detail; a result without details keeps the source shape alone.
    let conforming = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:memberShape ex:Member ] .
         ex:Member a sh:NodeShape ; sh:nodeKind sh:IRI .",
        "ex:h a ex:C ; ex:list ( ex:a ex:b ) .",
    );
    assert!(
        conforming["runs"][0]["results"]
            .as_array()
            .is_none_or(Vec::is_empty),
        "{conforming:#}"
    );
    let plain = sarif(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:list ; sh:minListLength 3 ] .",
        "ex:h a ex:C ; ex:list ( ex:a ) .",
    );
    assert_eq!(
        plain["runs"][0]["results"][0]["relatedLocations"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}
