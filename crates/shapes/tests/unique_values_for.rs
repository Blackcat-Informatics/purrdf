// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `sh:uniqueValuesFor` (SHACL 1.2 Core §7.9.5), end to end.
//!
//! The textual definition: "Let $targetNodes be the target nodes of S. For each
//! value node V for which there exists another node in $targetNodes that has
//! exactly the same values for all properties in $properties as V there is a
//! validation result. No result is produced if V has no values for any of the
//! properties in $properties." And the note: "matching of literals needs to be
//! exact, e.g. "04"^^xsd:byte does not match "4"^^xsd:integer."
//!
//! Every test pairs a treatment row with a control row that differs from it, so
//! a constraint that silently stopped constraining cannot pass by conforming
//! everywhere, and every refusal is paired with the valid neighbour beside it.
//! The W3C tests `core/node/uniqueValuesFor-001` to `-005` are graded by
//! `w3c12_conformance.rs`; these cover what they do not: exactness, the
//! no-values rule under a list, value SETS, property shapes, a shape reached
//! through `sh:node`, the prepared focus-node APIs and the incremental change
//! path.
//!
//! Fixture IRIs are under `example.org`: PurRDF mints no vocabulary IRIs.

use std::sync::Arc;

use purrdf::ir::ViewLimits;
use purrdf::{DatasetMut, MutableDataset, QuadValues, RdfDataset, TermValue};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:   <http://example.org/ns#> .
@prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
";

const EX: &str = "http://example.org/ns#";
const UNIQUE: &str = "http://www.w3.org/ns/shacl#UniqueValuesForConstraintComponent";
const NODE: &str = "http://www.w3.org/ns/shacl#NodeConstraintComponent";

fn load(shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None)
}

#[track_caller]
fn loads(shapes_ttl: &str) -> Shapes {
    load(shapes_ttl).unwrap_or_else(|error| panic!("the shapes graph must load: {error}"))
}

fn data(data_ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}{data_ttl}"), None).expect("data parses")
}

#[track_caller]
fn validate(shapes_ttl: &str, data_ttl: &str) -> ValidationReport {
    validate_dataset_with_shapes_graph(&data(data_ttl), &loads(shapes_ttl), None)
        .expect("validation runs")
}

fn ex(local: &str) -> String {
    format!("<{EX}{local}>")
}

fn ex_term(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!("{EX}{local}")))
}

/// `(focus node, component, result path, value)` of every result, sorted.
fn rows(report: &ValidationReport) -> Vec<(String, String, String, String)> {
    let mut out: Vec<_> = report
        .results
        .iter()
        .map(|r| {
            (
                r.focus_node.to_string(),
                r.source_constraint_component.as_str().to_owned(),
                r.result_path
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
                r.value
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
            )
        })
        .collect();
    out.sort();
    out
}

/// A node-shape `sh:uniqueValuesFor` result: the focus node, no path, no value.
fn node_result(local: &str) -> (String, String, String, String) {
    (ex(local), UNIQUE.to_owned(), String::new(), String::new())
}

const RECORD_SHAPE: &str =
    "ex:S a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor ex:id .";

/// The specification's own example: two targets sharing a value both violate,
/// with no `sh:value` and no `sh:resultPath`; a NON-target sharing a value with
/// a target makes neither violate.
#[test]
fn every_target_sharing_a_value_is_reported_and_a_non_target_is_not_compared() {
    let report = validate(
        RECORD_SHAPE,
        "ex:r1 a ex:Record ; ex:id \"One\" .
         ex:unrelated ex:id \"One\" .
         ex:r2 a ex:Record ; ex:id \"Two\" .
         ex:r3 a ex:Record ; ex:id \"Two\" .",
    );
    assert_eq!(rows(&report), vec![node_result("r2"), node_result("r3")]);
    // Control: the same graph with distinct ids conforms.
    let control = validate(
        RECORD_SHAPE,
        "ex:r1 a ex:Record ; ex:id \"One\" .
         ex:unrelated ex:id \"One\" .
         ex:r2 a ex:Record ; ex:id \"Two\" .
         ex:r3 a ex:Record ; ex:id \"Three\" .",
    );
    assert!(control.conforms, "{:?}", control.results);
}

/// "matching of literals needs to be exact": `"04"^^xsd:byte` and
/// `"4"^^xsd:integer` are equal in value space and do NOT collide; the same term
/// twice does.
#[test]
fn literal_matching_is_exact() {
    let distinct = validate(
        RECORD_SHAPE,
        "ex:a a ex:Record ; ex:id \"04\"^^xsd:byte .
         ex:b a ex:Record ; ex:id \"4\"^^xsd:integer .",
    );
    assert!(distinct.conforms, "{:?}", distinct.results);
    let same = validate(
        RECORD_SHAPE,
        "ex:a a ex:Record ; ex:id \"4\"^^xsd:integer .
         ex:b a ex:Record ; ex:id \"4\"^^xsd:integer .",
    );
    assert_eq!(rows(&same), vec![node_result("a"), node_result("b")]);
    // A language tag is part of the term too.
    let tagged = validate(
        RECORD_SHAPE,
        "ex:a a ex:Record ; ex:id \"x\"@en .
         ex:b a ex:Record ; ex:id \"x\" .",
    );
    assert!(tagged.conforms, "{:?}", tagged.results);
}

/// "No result is produced if V has no values for any of the properties": two
/// targets with no value for any listed property do not collide, while the same
/// two with one shared value do.
#[test]
fn a_node_with_no_values_for_any_property_gets_no_result() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetClass ex:Concept ;
                    sh:uniqueValuesFor ( ex:notation ex:scheme ) .";
    let empty = validate(shapes, "ex:a a ex:Concept . ex:b a ex:Concept .");
    assert!(empty.conforms, "{:?}", empty.results);
    let shared = validate(
        shapes,
        "ex:a a ex:Concept ; ex:notation \"A1\" . ex:b a ex:Concept ; ex:notation \"A1\" .",
    );
    // Neither has an `ex:scheme` value, and "no values" for one property is
    // still a value SET both share — the exclusion is for a node with no value
    // for ANY listed property, and these have one.
    assert_eq!(rows(&shared), vec![node_result("a"), node_result("b")]);
}

/// The list form compares the whole tuple: the same notation in different
/// schemes is no collision, the same notation in the same scheme is.
#[test]
fn a_list_of_properties_compares_the_whole_tuple() {
    let shapes = "ex:S a sh:NodeShape ; sh:targetClass ex:Concept ;
                    sh:uniqueValuesFor ( ex:notation ex:scheme ) .";
    let report = validate(
        shapes,
        "ex:a a ex:Concept ; ex:notation \"A1\" ; ex:scheme ex:S1 .
         ex:b a ex:Concept ; ex:notation \"A1\" ; ex:scheme ex:S2 .
         ex:c a ex:Concept ; ex:notation \"A1\" ; ex:scheme ex:S2 .",
    );
    assert_eq!(rows(&report), vec![node_result("b"), node_result("c")]);
    // A single-IRI value keyed on ex:notation alone makes all three collide.
    let single = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Concept ; sh:uniqueValuesFor ex:notation .",
        "ex:a a ex:Concept ; ex:notation \"A1\" ; ex:scheme ex:S1 .
         ex:b a ex:Concept ; ex:notation \"A1\" ; ex:scheme ex:S2 .
         ex:c a ex:Concept ; ex:notation \"A1\" ; ex:scheme ex:S2 .",
    );
    assert_eq!(
        rows(&single),
        vec![node_result("a"), node_result("b"), node_result("c")]
    );
}

/// "exactly the same values": value SETS are compared, not any overlap — `{x, y}`
/// against `{x}` is no collision, `{x, y}` against `{y, x}` is.
#[test]
fn values_are_compared_as_whole_sets() {
    let report = validate(
        RECORD_SHAPE,
        "ex:a a ex:Record ; ex:id \"x\", \"y\" .
         ex:b a ex:Record ; ex:id \"x\" .
         ex:c a ex:Record ; ex:id \"y\", \"x\" .",
    );
    assert_eq!(rows(&report), vec![node_result("a"), node_result("c")]);
}

/// A literal `sh:uniqueValuesFor`, or a list holding one, is refused at load; the
/// IRI and IRI-list neighbours load and are honoured (the tests above).
#[test]
fn an_ill_typed_parameter_is_refused_and_iris_load() {
    for shapes in [
        "ex:S a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor \"ex:id\" .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor ( ex:id \"ex:code\" ) .",
    ] {
        let error = load(shapes).expect_err("a literal property is refused");
        assert!(error.contains("uniqueValuesFor"), "{error}");
    }
    loads("ex:S a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor ex:id .");
    loads(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor ( ex:id ex:code ) .",
    );
}

/// On a property shape with targets of its own, the value nodes are compared with
/// the shape's TARGET nodes, and each result names the colliding value node as
/// `sh:value` under the shape's path.
#[test]
fn a_property_shape_compares_its_value_nodes_with_its_targets() {
    let shapes = "ex:PS a sh:PropertyShape ; sh:targetClass ex:Holder ;
                    sh:path ex:item ; sh:uniqueValuesFor ex:id .";
    let report = validate(
        shapes,
        "ex:h a ex:Holder ; ex:id \"X\" ; ex:item ex:i1, ex:i2 .
         ex:i1 ex:id \"X\" .
         ex:i2 ex:id \"Y\" .",
    );
    assert_eq!(
        rows(&report),
        vec![(ex("h"), UNIQUE.to_owned(), ex("item"), ex("i1"))]
    );
    let control = validate(
        shapes,
        "ex:h a ex:Holder ; ex:id \"X\" ; ex:item ex:i1, ex:i2 .
         ex:i1 ex:id \"Z\" .
         ex:i2 ex:id \"Y\" .",
    );
    assert!(control.conforms, "{:?}", control.results);
}

/// "the target nodes of S" belong to S however the evaluation reaches it: a node
/// shape reached through `sh:node` compares its value node with ITS OWN targets,
/// so a node that is not one of them collides with one that is.
#[test]
fn a_shape_reached_through_sh_node_keeps_its_own_targets() {
    let shapes = "ex:Inner a sh:NodeShape ; sh:targetClass ex:Record ; sh:uniqueValuesFor ex:id .
                  ex:Outer a sh:NodeShape ; sh:targetNode ex:x ; sh:node ex:Inner .";
    let report = validate(
        shapes,
        "ex:r a ex:Record ; ex:id \"A\" .
         ex:x ex:id \"A\" .",
    );
    assert_eq!(
        rows(&report),
        vec![(ex("x"), NODE.to_owned(), String::new(), ex("x"))]
    );
    let control = validate(
        shapes,
        "ex:r a ex:Record ; ex:id \"A\" .
         ex:x ex:id \"B\" .",
    );
    assert!(control.conforms, "{:?}", control.results);
}

/// A property shape reached only through `sh:property` declares no target, so its
/// `$targetNodes` is empty and no value node has another target node to collide
/// with — the component reads nothing and reports nothing, whatever the data.
#[test]
fn a_property_shape_without_targets_has_nothing_to_collide_with() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Holder ;
           sh:property [ sh:path ex:item ; sh:uniqueValuesFor ex:id ] .",
        "ex:h a ex:Holder ; ex:id \"X\" ; ex:item ex:i1, ex:i2 .
         ex:i1 ex:id \"X\" . ex:i2 ex:id \"X\" .",
    );
    assert!(report.conforms, "{:?}", report.results);
}

/// The prepared focus-node APIs validate only the focus nodes a caller names, but
/// `$targetNodes` is the shape's FULL target set in the bound data graph: a
/// requested node collides with a target the request did not name. The control
/// focus node, whose id is unique, conforms through the same request.
#[test]
fn the_prepared_focus_node_apis_compare_against_the_full_target_set() {
    let prepared = PreparedShapes::new(Arc::new(loads(RECORD_SHAPE)));
    let validator = prepared
        .bind_dataset(&data(
            "ex:alice a ex:Record ; ex:id \"A\" .
             ex:bob a ex:Record ; ex:id \"A\" .
             ex:carl a ex:Record ; ex:id \"C\" .",
        ))
        .expect("bind");

    let by_term = validator
        .validate_focus_nodes(&[ex_term("alice"), ex_term("carl")])
        .expect("term-keyed validation");
    assert_eq!(rows(&by_term), vec![node_result("alice")]);

    let ids: Vec<_> = ["alice", "carl"]
        .into_iter()
        .map(|local| validator.term_id(&ex_term(local)).expect("interned"))
        .collect();
    let by_id = validator
        .validate_focus_node_ids(&ids)
        .expect("id-keyed validation");
    assert_eq!(rows(&by_id), vec![node_result("alice")]);

    // The whole-graph validation agrees, and names bob too.
    assert_eq!(
        rows(&validator.validate().expect("full validation")),
        vec![node_result("alice"), node_result("bob")]
    );
}

/// **Incremental revalidation: changing one focus node's value changes ANOTHER
/// focus node's verdict, and the change path finds it.**
///
/// `ex:bob`'s id moves from "B" to "A". `ex:alice`'s own data does not change,
/// yet she now violates. The expansion of the change must name her, and
/// re-validating exactly the expansion must report her.
#[test]
fn changing_one_focus_nodes_value_revalidates_another() {
    let prepared = PreparedShapes::new(Arc::new(loads(RECORD_SHAPE)));
    let base = data(
        "ex:alice a ex:Record ; ex:id \"A\" .
         ex:bob a ex:Record ; ex:id \"B\" .",
    );
    let before = prepared
        .bind_dataset(&base)
        .and_then(|validator| validator.validate())
        .expect("base validation");
    assert!(before.conforms, "{:?}", before.results);

    let id = |lexical: &str| QuadValues {
        s: TermValue::iri(format!("{EX}bob")),
        p: TermValue::iri(format!("{EX}id")),
        o: TermValue::simple_literal(lexical),
        g: None,
    };
    let mut mutation = MutableDataset::new(Arc::clone(&base));
    assert!(mutation.remove(&id("B")));
    assert!(mutation.insert(id("A")).expect("insert"));
    let snapshot = Arc::new(mutation.snapshot_view().expect("snapshot"));
    let validator = prepared
        .bind_delta_with_shapes_graph(Arc::clone(&snapshot), None, ViewLimits::default())
        .expect("delta bind");

    let expansion = validator
        .affected_focus_node_ids(&snapshot)
        .expect("expansion");
    let ids = expansion.ids().expect("the footprint is bounded");
    let alice = validator
        .term_id(&ex_term("alice"))
        .expect("ex:alice is interned");
    assert!(
        ids.contains(&alice),
        "ex:alice's verdict moved though only ex:bob changed; the expansion must name her"
    );
    let revalidated = validator
        .validate_focus_node_ids(ids)
        .expect("re-validating the expansion");
    assert_eq!(
        rows(&revalidated),
        vec![node_result("alice"), node_result("bob")]
    );
}
