// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cardinality follows the native SHACL construct, without vocabulary imports.

use purrdf_shapes::engine::{parse_shapes, validate_graphs};
use purrdf_shapes::shapes::from_dataset;

const PREFIXES: &str = r"
    @prefix ex: <http://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

/// No imported `sh:ConstraintComponent` declarations are needed to reject
/// multiple values of a parameter that SHACL explicitly makes single-valued.
#[test]
fn native_singletons_reject_multiple_distinct_values() {
    let cases = [
        ("datatype", "xsd:string, xsd:integer"),
        ("nodeKind", "sh:IRI, sh:Literal"),
        ("minCount", "1, 2"),
        ("maxCount", "1, 2"),
        ("minExclusive", "1, 2"),
        ("minInclusive", "1, 2"),
        ("maxExclusive", "1, 2"),
        ("maxInclusive", "1, 2"),
        ("minLength", "1, 2"),
        ("maxLength", "1, 2"),
        ("languageIn", "(\"en\"), (\"fr\")"),
        ("uniqueLang", "true, false"),
        ("in", "(ex:a), (ex:b)"),
        ("pattern", "\"^a\", \"z$\""),
        ("flags", "\"i\", \"m\""),
        ("closed", "true, false"),
        ("ignoredProperties", "(ex:a), (ex:b)"),
        ("qualifiedValueShape", "ex:A, ex:B"),
        ("qualifiedMinCount", "1, 2"),
        ("qualifiedMaxCount", "1, 2"),
        ("qualifiedValueShapesDisjoint", "true, false"),
        ("reifierShape", "ex:A, ex:B"),
        ("reificationRequired", "true, false"),
        ("severity", "sh:Warning, sh:Violation"),
        ("deactivated", "true, false"),
        ("order", "1, 2"),
    ];
    for (parameter, values) in cases {
        let ttl = format!(
            "{PREFIXES}
             ex:Shape a sh:PropertyShape; sh:targetNode ex:n; sh:path ex:p;
                 sh:{parameter} {values} ."
        );
        let Err(error) = parse_shapes(&ttl, None) else {
            panic!("sh:{parameter} must reject multiple values");
        };
        assert!(error.contains("<http://example.org/Shape>"), "{error}");
        assert!(
            error.contains(&format!("<http://www.w3.org/ns/shacl#{parameter}>")),
            "{error}"
        );
        assert!(error.contains("distinct value"), "{error}");
    }
}

/// Metadata must be counted before filtering values by their expected kind.
#[test]
fn malformed_parameter_path_cannot_hide_an_extra_value() {
    let ttl = format!(
        "{PREFIXES}
         ex:Component a sh:ConstraintComponent; sh:parameter ex:Parameter .
         ex:Parameter sh:path ex:p, 42 ."
    );
    let error = parse_shapes(&ttl, None).expect_err("two sh:path values");
    assert!(error.contains("<http://example.org/Parameter>"), "{error}");
    assert!(
        error.contains("<http://www.w3.org/ns/shacl#path>"),
        "{error}"
    );
}

/// The same metadata rules apply to constraint, function, and target declarations
/// even when no shape has targets and their bodies would never execute.
#[test]
fn declaration_singletons_are_checked_before_activation() {
    let declarations = [
        (
            "optional",
            "ex:C a sh:ConstraintComponent; sh:parameter ex:P .
             ex:P sh:path ex:p; sh:optional true, false .",
        ),
        (
            "select",
            "ex:T a sh:SPARQLTargetType;
             sh:select \"SELECT ?this WHERE {}\", \"SELECT ?this WHERE { FILTER(false) }\" .",
        ),
        (
            "ask",
            "ex:F a sh:SPARQLFunction; sh:ask \"ASK {}\", \"ASK { FILTER(false) }\" .",
        ),
        (
            "returnType",
            "ex:F a sh:SPARQLFunction; sh:ask \"ASK {}\";
             sh:returnType xsd:boolean, xsd:string .",
        ),
    ];
    for (parameter, declaration) in declarations {
        let error = parse_shapes(&format!("{PREFIXES}{declaration}"), None)
            .expect_err("malformed unused declaration must fail at load");
        assert!(
            error.contains(&format!("<http://www.w3.org/ns/shacl#{parameter}>")),
            "{error}"
        );
    }
}

/// All parameter-bearing declarations use the same RDF boolean boundary;
/// strings and integer literals with boolean-looking lexical forms do not pass.
#[test]
fn optional_requires_a_boolean_term_in_every_declaration_kind() {
    let declarations = [
        "ex:C a sh:ConstraintComponent; sh:parameter ex:P, [sh:path ex:required] .",
        "ex:F a sh:SPARQLFunction; sh:parameter ex:P;
             sh:select \"SELECT ?p WHERE {}\" .",
        "ex:T a sh:SPARQLTargetType; sh:parameter ex:P;
             sh:select \"SELECT ?this WHERE {}\" .",
        "ex:F a sh:NamedParameterExpressionFunction;
             sh:parameter ex:P; sh:bodyExpression true .",
    ];
    for declaration in declarations {
        for invalid in [
            "\"true\"",
            "\"false\"",
            "1",
            "ex:true",
            "\"yes\"^^xsd:boolean",
        ] {
            let ttl = format!(
                "{PREFIXES}{declaration}
                 ex:P sh:path ex:p; sh:keyParameter true; sh:optional {invalid} ."
            );
            let error = parse_shapes(&ttl, None).expect_err("invalid optional boolean");
            assert!(error.contains("sh:optional"), "{declaration}: {error}");
            assert!(error.contains("xsd:boolean"), "{declaration}: {error}");
        }
    }
}

/// Both legal XSD boolean lexical spellings retain their SHACL interpretation.
#[test]
fn optional_accepts_every_xsd_boolean_lexical_form() {
    for value in ["true", "false", "\"1\"^^xsd:boolean", "\"0\"^^xsd:boolean"] {
        let ttl = format!(
            "{PREFIXES}
             ex:C a sh:ConstraintComponent;
                 sh:parameter [sh:path ex:required], [sh:path ex:p; sh:optional {value}] ."
        );
        parse_shapes(&ttl, None)
            .unwrap_or_else(|error| panic!("valid optional boolean {value}: {error}"));
    }
}

/// SHACL metadata predicates on unrelated RDF resources do not turn those
/// resources into declarations or impose shape syntax on their annotations.
#[test]
fn unrelated_rdf_metadata_is_not_a_shacl_declaration() {
    let ttl = format!(
        "{PREFIXES}
         ex:Document sh:order 1, 2; sh:select \"first annotation\", \"second annotation\";
             sh:optional \"true\", \"false\"; sh:path ex:p, ex:q;
             sh:returnType xsd:string, xsd:integer ."
    );
    let shapes = parse_shapes(&ttl, None).expect("ordinary RDF annotations");
    assert!(shapes.node_shapes.is_empty());
}

/// Roles follow SHACL subclass declarations and shape-list references, so
/// metadata checks cover declarations without their built-in type written out.
#[test]
fn inherited_and_list_referenced_roles_enforce_metadata() {
    let declarations = [
        "ex:CustomFunction <http://www.w3.org/2000/01/rdf-schema#subClassOf> sh:SPARQLFunction .
         ex:F a ex:CustomFunction; sh:select \"SELECT ?p WHERE {}\", \"SELECT ?q WHERE {}\" .",
        "ex:Shape a sh:NodeShape; sh:or ([sh:severity sh:Warning, sh:Violation]) .",
    ];
    for declaration in declarations {
        let error = parse_shapes(&format!("{PREFIXES}{declaration}"), None)
            .expect_err("metadata conflict on a recognized SHACL role");
        assert!(error.contains("distinct value"), "{error}");
    }
}

/// Single-parameter components that lack an explicit max-one rule produce one
/// constraint per parameter value, including shape and RDF-list values.
#[test]
fn repeatable_core_parameters_keep_each_constraint() {
    let cases = [
        ("class", "ex:A, ex:B"),
        ("hasValue", "ex:a, ex:b"),
        ("node", "ex:A, ex:B"),
        ("not", "ex:A, ex:B"),
        ("and", "([]), ([])"),
        ("or", "([]), ([])"),
        ("xone", "([]), ([])"),
        ("equals", "ex:a, ex:b"),
        ("disjoint", "ex:a, ex:b"),
        ("lessThan", "ex:a, ex:b"),
        ("lessThanOrEquals", "ex:a, ex:b"),
        (
            "sparql",
            "[sh:select \"SELECT $this WHERE { FILTER(false) }\"],
             [sh:select \"SELECT $this WHERE { FILTER(false) }\"]",
        ),
        ("expression", "true, false"),
    ];
    for (parameter, values) in cases {
        let ttl = format!(
            "{PREFIXES}
             ex:Shape a sh:PropertyShape; sh:targetNode ex:n; sh:path ex:p;
                 sh:{parameter} {values} ."
        );
        let shapes = parse_shapes(&ttl, None)
            .unwrap_or_else(|error| panic!("repeatable sh:{parameter}: {error}"));
        assert_eq!(shapes.node_shapes.len(), 1, "sh:{parameter}");
        assert_eq!(
            shapes.node_shapes[0].property_shapes[0].constraints.len(),
            2,
            "both sh:{parameter} values must contribute constraints"
        );
    }
}

/// Repeated `sh:property` remains conjunctive and both paths are evaluated.
#[test]
fn repeated_property_shapes_validate_both_paths() {
    let ttl = format!(
        "{PREFIXES}
         ex:Shape a sh:NodeShape; sh:targetNode ex:n;
             sh:property [sh:path ex:p; sh:minCount 1],
                         [sh:path ex:q; sh:minCount 1] ."
    );
    let absent = validate_graphs("", &ttl, None).expect("valid shapes");
    assert!(!absent.conforms);
    assert_eq!(absent.results.len(), 2);
    let present = validate_graphs(
        "<http://example.org/n> <http://example.org/p> <http://example.org/a> .\n\
         <http://example.org/n> <http://example.org/q> <http://example.org/b> .",
        &ttl,
        None,
    )
    .expect("valid data");
    assert!(present.conforms);
}

/// AnyGraph operates on RDF values: the same statement in two named graphs is
/// one parameter value, while distinct objects remain separate constraints.
#[test]
fn named_graph_repetitions_count_distinct_rdf_objects() {
    let graph = "ex:Shape a sh:PropertyShape; sh:targetNode ex:n; sh:path ex:p;
                 sh:minCount 1; sh:class ex:A, ex:B .";
    let trig = format!("{PREFIXES} ex:g1 {{ {graph} }} ex:g2 {{ {graph} }}");
    let dataset =
        ::purrdf::parse_dataset(trig.as_bytes(), "application/trig", None).expect("valid TriG");
    let shapes = from_dataset(&dataset).expect("repeated statements are not extra values");
    assert_eq!(shapes.node_shapes.len(), 1);
    assert_eq!(
        shapes.node_shapes[0].property_shapes[0].constraints.len(),
        3
    );
}

/// Equal numeric values with distinct RDF lexical forms are still two objects.
#[test]
fn distinct_lexical_terms_are_not_merged_by_value_equality() {
    let ttl = format!(
        "{PREFIXES}
         ex:Shape a sh:PropertyShape; sh:targetNode ex:n; sh:path ex:p;
             sh:minCount \"1\"^^xsd:integer, \"01\"^^xsd:integer ."
    );
    let error = parse_shapes(&ttl, None).expect_err("distinct RDF terms");
    assert!(
        error.contains("<http://www.w3.org/ns/shacl#minCount>"),
        "{error}"
    );
}

/// The diagnostic identifies the same malformed predicate regardless of input
/// ordering or the IDs assigned by the interner.
#[test]
fn cardinality_diagnostic_is_independent_of_statement_order() {
    let declarations = [
        "ex:Z sh:pattern \"a\", \"b\" .",
        "ex:A sh:flags \"i\", \"m\" .",
    ];
    let forward = parse_shapes(
        &format!("{PREFIXES}{}{}", declarations[0], declarations[1]),
        None,
    )
    .expect_err("multiple conflicts");
    let reversed = parse_shapes(
        &format!("{PREFIXES}{}{}", declarations[1], declarations[0]),
        None,
    )
    .expect_err("multiple conflicts");
    assert_eq!(forward, reversed);
    assert!(forward.contains("<http://example.org/A>"), "{forward}");
}
