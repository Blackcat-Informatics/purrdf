// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `sh:values`, `sh:defaultValue`, `sh:expectedPredicate` — and the term that
//! looks like a fourth, `compliance` — each observed doing exactly what its
//! specification text says, and no more.
//!
//! Every case runs TWO shapes graphs that differ only by the term under test,
//! against the same `example.org` data, through the public surface a caller uses
//! (`validate_dataset_with_shapes_graph`, or `entail_dataset` for the rules side),
//! and compares the outputs:
//!
//! * where the specification gives the term an effect, the outputs DIFFER, and
//!   the case pins how;
//! * where it makes the term documentation, the census classifies that position
//!   non-validating and the outputs are BYTE-IDENTICAL — beside a third row that
//!   differs from both, so the identity is observed by an oracle that could have
//!   told the two apart.
//!
//! Each case quotes the sentence it implements.
//!
//! # `compliance`
//!
//! No SHACL 1.2 specification — Core, Node Expressions, SPARQL Extensions,
//! Inference Rules — and no SHACL 1.2 vocabulary defines a term `sh:compliance`.
//! The only `compliance` in the SHACL 1.2 material is `sht:compliance`, in the
//! W3C test suite's manifest vocabulary: the conformance classes a test entry
//! exercises (`sht:compliance sht:SPARQL, sht:NodeExpr` on
//! `inference-rules/expectedPredicate-example`, whose manifest graph is also its
//! shapes graph). So the two halves are pinned: `sh:compliance` is not a term of
//! the language, and a shape that carries it is refused like any other unknown
//! `sh:` predicate; `sht:compliance` is a triple outside the SHACL namespaces, which
//! a shapes graph may carry and which changes nothing.

use std::sync::Arc;

use purrdf::{RdfDataset, canonicalize};
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::entail_dataset;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::spec::census::{Role, Site, TermClass, classify};
use purrdf_shapes::spec::declared_terms;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "
@prefix ex:     <http://example.org/ns#> .
@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix sht:    <http://www.w3.org/ns/shacl-test#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

fn load(shapes_ttl: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{PREFIXES}{shapes_ttl}"), None).map_err(String::from)
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

/// `(focus node, value, component)` of every result, sorted.
fn results(report: &ValidationReport) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = report
        .results
        .iter()
        .map(|r| {
            (
                r.focus_node.to_string(),
                r.value
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
                r.source_constraint_component.as_str().to_owned(),
            )
        })
        .collect();
    out.sort();
    out
}

/// The RDFC-1.0 canonical N-Quads of `entail_dataset(data, shapes)`.
#[track_caller]
fn entail(shapes_ttl: &str, data_ttl: &str) -> String {
    let entailed = entail_dataset(data(data_ttl).as_ref(), &loads(shapes_ttl)).expect("rules run");
    canonicalize(entailed.as_ref()).nquads
}

fn ex(local: &str) -> String {
    format!("<http://example.org/ns#{local}>")
}

const SH_DATATYPE: &str = "http://www.w3.org/ns/shacl#DatatypeConstraintComponent";
const SH_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#MaxCountConstraintComponent";
const SH_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#MinCountConstraintComponent";

// ── sh:values (validation) ────────────────────────────────────────────────────

/// SHACL 1.2 Core, "Value Nodes of Property Shapes": "Add all nodes in the data
/// graph that can be reached from the focus node with the path mapping of p. If e
/// is the value of sh:values at the property shape, then add the output nodes of
/// evalExpr(e, data graph, focus node, {})."
///
/// The computed nodes are ADDED to the path's: `ex:a` has one asserted `ex:p` and
/// `sh:values` computes a second, so `sh:maxCount 1` fails at `ex:a`; `ex:b` has
/// no asserted value, so the computed one alone satisfies `sh:minCount 1` there.
/// The control, without `sh:values`, reports the opposite node.
#[test]
fn values_adds_the_expression_output_to_the_path_values() {
    let data = "ex:a a ex:C ; ex:p 3 ; ex:q 4 .  ex:b a ex:C ; ex:q 5 .";
    let with = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:values [ sh:path ex:q ] ;
                         sh:minCount 1 ; sh:maxCount 1 ] .",
        data,
    );
    let without = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:maxCount 1 ] .",
        data,
    );
    assert_eq!(
        results(&with),
        vec![(ex("a"), String::new(), SH_MAX_COUNT.to_owned())]
    );
    assert_eq!(
        results(&without),
        vec![(ex("b"), String::new(), SH_MIN_COUNT.to_owned())]
    );
}

/// The expression is evaluated with the FOCUS node as its focus — "evalExpr(e,
/// data graph, focus node, {})" — so a computed value is the focus node's, and a
/// computed literal the data graph never interned is a value node like any other:
/// `sparql:multiply` of the focus node's width and height is checked against
/// `sh:datatype xsd:string` and reported with its computed value.
#[test]
fn values_is_evaluated_at_the_focus_node_and_its_output_is_checked() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Rectangle ;
           sh:property [ sh:path ex:area ; sh:datatype xsd:string ;
             sh:values [ sparql:multiply ( [ shnex:pathValues ex:width ]
                                           [ shnex:pathValues ex:height ] ) ] ] .",
        "ex:r a ex:Rectangle ; ex:width 4 ; ex:height 5 .",
    );
    assert_eq!(
        results(&report),
        vec![(
            ex("r"),
            "\"20\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
            SH_DATATYPE.to_owned()
        )]
    );
}

// ── sh:defaultValue (validation) ──────────────────────────────────────────────

/// SHACL 1.2 Core, "Value Nodes of Property Shapes": "If the set is still empty
/// and d is the value of sh:defaultValue at the property shape, then add the
/// output nodes of evalExpr(d, data graph, focus node, {})."
///
/// `ex:a` has no `ex:p`, so the default `"none"` becomes its value node and fails
/// `sh:datatype xsd:integer`; `ex:b` has one, so the set is not empty and the
/// default is not added. The control, identical but for `sh:defaultValue`,
/// reports nothing — the outputs differ.
#[test]
fn default_value_applies_only_when_the_value_set_is_empty() {
    let data = "ex:a a ex:C .  ex:b a ex:C ; ex:p 3 .";
    let with = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:datatype xsd:integer ; sh:defaultValue \"none\" ] .",
        data,
    );
    let without = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:datatype xsd:integer ] .",
        data,
    );
    assert_eq!(
        results(&with),
        vec![(ex("a"), "\"none\"".to_owned(), SH_DATATYPE.to_owned())]
    );
    assert!(results(&without).is_empty(), "{:?}", results(&without));
    assert_ne!(with.to_ntriples(), without.to_ntriples());
}

/// "If the set is STILL empty": the set the default looks at already holds the
/// `sh:values` output, so a `sh:values` that produced a node keeps the default
/// out. `ex:a` gets its computed `ex:q` value and no default; `ex:b` has neither,
/// so it gets the default — and only `ex:b` fails `sh:datatype xsd:integer`.
#[test]
fn default_value_is_not_added_when_values_produced_a_node() {
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetClass ex:C ;
           sh:property [ sh:path ex:p ; sh:datatype xsd:integer ;
                         sh:values [ sh:path ex:q ] ; sh:defaultValue \"none\" ] .",
        "ex:a a ex:C ; ex:q 7 .  ex:b a ex:C .",
    );
    assert_eq!(
        results(&report),
        vec![(ex("b"), "\"none\"".to_owned(), SH_DATATYPE.to_owned())]
    );
}

/// SHACL 1.2 SPARQL Extensions, on a function's `sh:parameter` declaring
/// `sh:defaultValue "en"`: "note that the SHACL constraints from the sh:parameter
/// declarations are not automatically enforced, nor will the declared
/// sh:defaultValue be used at runtime. These mainly serve documentation
/// purposes."
///
/// So at that position the census classifies `sh:defaultValue` non-validating,
/// and a function whose optional parameter declares `sh:defaultValue 1` answers
/// exactly as the same function without it: the call that omits the argument
/// sees it unbound (`COALESCE($x, 0)` is 0, and the constraint asking for 1
/// reports), BYTE-IDENTICALLY with and without the declaration. The third row
/// passes 1 explicitly and conforms, so the oracle observes the one thing the
/// default would have changed.
#[test]
fn a_parameter_declarations_default_value_is_documentation() {
    let row = classify("http://www.w3.org/ns/shacl#defaultValue").expect("classified");
    assert_eq!(
        row.class_at(Site::ParameterDeclaration),
        TermClass::NonValidating
    );
    assert_eq!(
        row.class_at(Site::Shape),
        TermClass::Structural(Role::ComputedValues)
    );

    let shapes = |default: &str, call: &str| {
        format!(
            "ex:f a sh:SPARQLFunction ;
               sh:parameter [ sh:path ex:x ; sh:datatype xsd:integer ; sh:optional true {default} ] ;
               sh:returnType xsd:integer ;
               sh:select \"SELECT (COALESCE($x, 0) AS ?result) WHERE {{}}\" .
             ex:S a sh:NodeShape ; sh:targetNode ex:a ;
               sh:sparql [ sh:select \"SELECT $this WHERE {{ FILTER (<http://example.org/ns#f>({call}) != 1) }}\" ] ."
        )
    };
    let data = "ex:a ex:p 1 .";
    let declared = validate(&shapes("; sh:defaultValue 1", ""), data);
    let undeclared = validate(&shapes("", ""), data);
    let explicit = validate(&shapes("; sh:defaultValue 1", "1"), data);
    assert_eq!(results(&declared).len(), 1, "{:?}", results(&declared));
    assert_eq!(declared.to_ntriples(), undeclared.to_ntriples());
    assert!(results(&explicit).is_empty(), "{:?}", results(&explicit));
    assert_ne!(declared.to_ntriples(), explicit.to_ntriples());
}

// ── sh:defaultValue / sh:values / sh:expectedPredicate (rules) ────────────────

/// The rules half of the specification's own example (SHACL 1.2 Inference Rules
/// §3.8), over `example.org`.
fn rectangle_shapes(expected_predicate: &str, default_value: &str) -> String {
    format!(
        "ex:RectangleShape a sh:NodeShape ; sh:targetClass ex:Rectangle ;
           sh:property ex:RectangleShape-area ; sh:rule ex:computeSmall .
         ex:RectangleShape-area a sh:PropertyShape ; sh:path ex:area {default_value} ;
           sh:values [ sparql:multiply ( [ shnex:pathValues ex:width ]
                                         [ shnex:pathValues ex:height ] ) ] .
         ex:computeSmall a sh:SPARQLRule {expected_predicate} ;
           sh:construct \"\"\"CONSTRUCT {{ $this <http://example.org/ns#isSmall> true }}
                          WHERE {{ $this <http://example.org/ns#area> ?area . FILTER (?area < 100) }}\"\"\" ."
    )
}

const RECTANGLES: &str = "ex:Incomplete a ex:Rectangle .
    ex:Small a ex:Rectangle ; ex:width 4 ; ex:height 5 .
    ex:Large a ex:Rectangle ; ex:width 11 ; ex:height 10 .";

fn is_small(local: &str) -> String {
    format!(
        "{} <http://example.org/ns#isSmall> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .",
        ex(local)
    )
}

/// SHACL 1.2 Inference Rules §3.8: "For a given predicate p, the derived value
/// nodes are all value nodes that can be computed using sh:defaultValue and
/// sh:values as defined by SHACL 1.2 Core in any (non-deactivated) property shape
/// that uses p as sh:path in the shapes graph. For these derived value nodes v
/// the derived triples are the triples where v is the object, p is the predicate
/// and the subjects are the target nodes of the property shapes. The expected
/// derived triples of a rule are the derived triples for all values of the
/// property sh:expectedPredicate at the rule."
///
/// With `sh:expectedPredicate ex:area` the rule sees `ex:Small ex:area 20` (from
/// `sh:values`) and `ex:Incomplete ex:area 1` (from `sh:defaultValue`), and infers
/// both `ex:isSmall` triples; without it, it sees no `ex:area` at all and infers
/// nothing. And §8: "Delete the derived triples (except those that were also
/// inferred by rules)" — so no `ex:area` triple survives into the output.
#[test]
fn expected_predicate_makes_the_derived_triples_visible_to_the_rule() {
    let with = entail(
        &rectangle_shapes("; sh:expectedPredicate ex:area", "; sh:defaultValue 1"),
        RECTANGLES,
    );
    let without = entail(&rectangle_shapes("", "; sh:defaultValue 1"), RECTANGLES);
    assert!(with.contains(&is_small("Small")), "{with}");
    assert!(with.contains(&is_small("Incomplete")), "{with}");
    assert!(!with.contains(&is_small("Large")), "{with}");
    assert!(
        !with.contains("<http://example.org/ns#area>"),
        "derived triples must be deleted at the end of the layer: {with}"
    );
    assert!(!without.contains("isSmall"), "{without}");
    assert_ne!(with, without);
}

/// The rules-side effect of `sh:defaultValue` alone: with it, the incomplete
/// rectangle's derived `ex:area 1` makes the rule infer `ex:isSmall` for it;
/// without it, the incomplete rectangle has no derived value and no inference,
/// while the small one still has both.
#[test]
fn default_value_is_a_derived_value_for_the_rules() {
    let expected = "; sh:expectedPredicate ex:area";
    let with = entail(
        &rectangle_shapes(expected, "; sh:defaultValue 1"),
        RECTANGLES,
    );
    let without = entail(&rectangle_shapes(expected, ""), RECTANGLES);
    assert!(with.contains(&is_small("Incomplete")), "{with}");
    assert!(!without.contains(&is_small("Incomplete")), "{without}");
    assert!(without.contains(&is_small("Small")), "{without}");
    assert_ne!(with, without);
}

/// "Delete the derived triples (except those that were also inferred by rules)":
/// a rule that ITSELF infers a derived triple keeps it. The copying rule re-derives
/// `ex:area` from what it sees, so every derived area survives in the output; the
/// control without the copying rule loses them all.
#[test]
fn a_derived_triple_a_rule_also_infers_is_kept() {
    let copying = format!(
        "{}
         ex:RectangleShape sh:rule ex:copyArea .
         ex:copyArea a sh:SPARQLRule ; sh:expectedPredicate ex:area ;
           sh:construct \"\"\"CONSTRUCT {{ $this <http://example.org/ns#area> ?a }}
                          WHERE {{ $this <http://example.org/ns#area> ?a }}\"\"\" .",
        rectangle_shapes("; sh:expectedPredicate ex:area", "; sh:defaultValue 1")
    );
    let kept = entail(&copying, RECTANGLES);
    let dropped = entail(
        &rectangle_shapes("; sh:expectedPredicate ex:area", "; sh:defaultValue 1"),
        RECTANGLES,
    );
    let small_area = "<http://example.org/ns#Small> <http://example.org/ns#area> \"20\"^^<http://www.w3.org/2001/XMLSchema#integer> .";
    assert!(kept.contains(small_area), "{kept}");
    assert!(!dropped.contains(small_area), "{dropped}");
}

/// The number of reifier declarations and of reifier annotations in the entailed
/// graph — the RDF 1.2 statement layer, which the triples of the canonical
/// N-Quads do not show.
#[track_caller]
fn reification(shapes_ttl: &str, data_ttl: &str) -> (usize, usize) {
    let entailed = entail_dataset(data(data_ttl).as_ref(), &loads(shapes_ttl)).expect("rules run");
    let reifiers: Vec<purrdf::TermId> = entailed.reifiers().map(|(reifier, _)| reifier).collect();
    let annotations = reifiers
        .iter()
        .map(|reifier| entailed.annotations_of(*reifier).count())
        .sum();
    (reifiers.len(), annotations)
}

/// "Delete the derived triples (except those that were also inferred by rules)
/// and their reifiers" (SHACL 1.2 Inference Rules §8): a rule that reifies each
/// derived `ex:area` triple and annotates the reifier loses both at the end of
/// the layer, because the statement they describe is gone; the rule's other
/// output, about the rectangle itself, stays. The control adds a rule that also
/// infers every derived area, so the areas are kept — and with them all three
/// reifiers and their annotations, which proves the rule does produce them and
/// that only the deletion took them away.
#[test]
fn a_deleted_derived_triple_takes_its_reifier_with_it() {
    let reifying = format!(
        "{}
         ex:RectangleShape sh:rule ex:reifyArea .
         ex:reifyArea a sh:SPARQLRule ; sh:expectedPredicate ex:area ; sh:runOnce true ;
           sh:construct \"\"\"CONSTRUCT {{
               _:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>
                   <<( $this <http://example.org/ns#area> ?a )>> .
               _:r <http://example.org/ns#source> \"derived\" .
               $this <http://example.org/ns#measured> true }}
             WHERE {{ $this <http://example.org/ns#area> ?a }}\"\"\" .",
        rectangle_shapes("; sh:expectedPredicate ex:area", "; sh:defaultValue 1")
    );
    let entailed = entail(&reifying, RECTANGLES);
    assert!(
        entailed.contains(&format!(
            "{} <http://example.org/ns#measured> \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean> .",
            ex("Small")
        )),
        "{entailed}"
    );
    assert!(!entailed.contains("#area"), "{entailed}");
    assert_eq!(reification(&reifying, RECTANGLES), (0, 0));

    let kept = format!(
        "{reifying}
         ex:RectangleShape sh:rule ex:copyArea .
         ex:copyArea a sh:SPARQLRule ; sh:expectedPredicate ex:area ;
           sh:construct \"\"\"CONSTRUCT {{ $this <http://example.org/ns#area> ?a }}
                          WHERE {{ $this <http://example.org/ns#area> ?a }}\"\"\" ."
    );
    assert!(entail(&kept, RECTANGLES).contains("#area"));
    assert_eq!(reification(&kept, RECTANGLES), (3, 3));
}

/// A value of `sh:expectedPredicate` names the predicate of the derived triples,
/// so a literal names none and is refused; the IRI neighbour loads.
#[test]
fn a_literal_expected_predicate_is_refused_and_an_iri_loads() {
    let error = load(&rectangle_shapes(
        "; sh:expectedPredicate \"ex:area\"",
        "; sh:defaultValue 1",
    ))
    .expect_err("a literal sh:expectedPredicate names no predicate");
    assert!(error.contains("sh:expectedPredicate"), "{error}");
    loads(&rectangle_shapes(
        "; sh:expectedPredicate ex:area",
        "; sh:defaultValue 1",
    ));
}

// ── compliance ────────────────────────────────────────────────────────────────

/// `sh:compliance` is not a SHACL 1.2 term: the census does not classify it and
/// the vendored vocabularies do not declare it, so a shape carrying it is
/// refused, as any unknown `sh:` predicate is — and the same shape without it
/// loads and validates.
#[test]
fn sh_compliance_is_not_a_shacl_term_and_is_refused() {
    let iri = "http://www.w3.org/ns/shacl#compliance";
    assert!(classify(iri).is_none());
    assert!(
        !declared_terms()
            .expect("the vendored vocabularies parse")
            .contains(iri),
        "the vendored SHACL 1.2 vocabularies declare no sh:compliance"
    );
    let error = load(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:compliance sht:SPARQL ;
           sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
    )
    .expect_err("an unknown sh: predicate on a shape is refused");
    assert!(error.contains("shacl#compliance"), "{error}");
    let report = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:minCount 1 ] .",
        "ex:b ex:p 1 .",
    );
    assert_eq!(
        results(&report),
        vec![(ex("a"), String::new(), SH_MIN_COUNT.to_owned())]
    );
}

/// `sht:compliance` — the W3C test manifest's statement of which conformance
/// classes an entry exercises — is not SHACL vocabulary, so a shapes graph that
/// carries it (as `expectedPredicate-example`'s does, its manifest being its
/// shapes graph) validates BYTE-IDENTICALLY to the same graph without it. The
/// third row changes the shape itself and reports, so the comparison observes.
#[test]
fn sht_compliance_is_manifest_metadata_and_changes_nothing() {
    let shape = "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
                   sh:property [ sh:path ex:p ; sh:minCount 1 ] .";
    let data = "ex:a ex:p 1 .";
    let with = validate(
        &format!("{shape} ex:test sht:compliance sht:SPARQL, sht:NodeExpr ."),
        data,
    );
    let without = validate(shape, data);
    let changed = validate(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:minCount 2 ] .",
        data,
    );
    assert!(results(&with).is_empty(), "{:?}", results(&with));
    assert_eq!(with.to_ntriples(), without.to_ntriples());
    assert_ne!(with.to_ntriples(), changed.to_ntriples());
}

// ── The shapes-graph reports see inside computed values ───────────────────────

/// `Shapes::function_resolution` reports the call sites a property shape's
/// `sh:values` and `sh:defaultValue` carry, by owner; the control, with constant
/// computed values, reports none.
#[test]
fn function_resolution_reports_the_calls_inside_computed_values() {
    let computed = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property ex:P .
         ex:P sh:path ex:p ;
           sh:values [ sparql:multiply ( [ shnex:pathValues ex:w ] 2 ) ] ;
           sh:defaultValue [ sparql:iri ( \"http://example.org/ns#none\" ) ] .",
    );
    let report = computed.function_resolution();
    let owners = |function: &str| -> Vec<String> {
        report
            .sites()
            .filter(|site| site.function == function)
            .map(|site| site.owner.clone())
            .collect()
    };
    assert_eq!(
        owners("http://www.w3.org/ns/sparql#multiply"),
        vec![format!("sh:values on {}", ex("P"))]
    );
    assert_eq!(
        owners("http://www.w3.org/ns/sparql#iri"),
        vec![format!("sh:defaultValue on {}", ex("P"))]
    );
    let constant = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:values 2 ; sh:defaultValue ex:none ] .",
    );
    assert_eq!(constant.function_resolution().sites().count(), 0);
}

/// `Shapes::extension_usage` reads the SPARQL a `sh:values` carries: a
/// `sh:select` expression names its predicate as a data edge under an empty
/// environment. The control, with a path expression, names nothing.
#[test]
fn extension_usage_reads_the_sparql_inside_computed_values() {
    let relation = "http://example.org/ns#related";
    let computed = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ;
             sh:values [ sh:select \"SELECT ?v WHERE { $this <http://example.org/ns#related> ?v }\" ] ] .",
    );
    let usage = computed.extension_usage(purrdf_sparql_eval::ExtensionEnv::empty());
    assert!(usage.data().contains(relation), "{usage:?}");
    let path = loads(
        "ex:S a sh:NodeShape ; sh:targetNode ex:a ;
           sh:property [ sh:path ex:p ; sh:values [ sh:path ex:related ] ] .",
    );
    let usage = path.extension_usage(purrdf_sparql_eval::ExtensionEnv::empty());
    assert!(!usage.data().contains(relation), "{usage:?}");
}

/// A prepared product carries both computed-value expressions and the rule's
/// expected predicates: the restored preparation validates, and its shapes
/// entail, exactly as the fresh parse does. The `sh:datatype xsd:string` makes
/// every computed area — and the default — a reported value, so a restore that
/// dropped either expression would change the report.
#[test]
fn a_prepared_product_carries_computed_values() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};

    let shapes_ttl = format!(
        "{} ex:RectangleShape-area sh:datatype xsd:string .",
        rectangle_shapes("; sh:expectedPredicate ex:area", "; sh:defaultValue 1")
    );
    let fresh = PreparedShapes::new(Arc::new(loads(&shapes_ttl)));
    let bytes = fresh
        .to_product(&ShapesProfile::CORE)
        .expect("the shapes graph packs");
    let restored = ShapesProduct::open(&bytes)
        .expect("the product opens")
        .admit(&ShapesProfile::CORE, &HostBindings::empty())
        .expect("the product admits");
    let report = |prepared: &PreparedShapes| {
        prepared
            .bind_shared_dataset(data(RECTANGLES))
            .expect("the data graph binds")
            .validate()
            .expect("validation runs")
    };
    let fresh_report = report(&fresh);
    assert_eq!(
        results(&fresh_report).len(),
        3,
        "{:?}",
        results(&fresh_report)
    );
    assert_eq!(fresh_report.to_ntriples(), report(&restored).to_ntriples());
    let entail_with = |prepared: &PreparedShapes| {
        canonicalize(
            entail_dataset(data(RECTANGLES).as_ref(), prepared.shapes())
                .expect("rules run")
                .as_ref(),
        )
        .nquads
    };
    let entailed = entail_with(&fresh);
    assert!(entailed.contains(&is_small("Incomplete")), "{entailed}");
    assert_eq!(entailed, entail_with(&restored));
}
