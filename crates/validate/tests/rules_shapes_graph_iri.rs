// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shapes-graph IRI reaches the RULES entry points of this boundary as it reaches
//! validation: [`apply_rules_to_ntriples`] (Python `apply_rules`, WebAssembly
//! `shaclApplyRules`, C `purrdf_shacl_apply_rules`) and
//! [`entail_to_ntriples_string_with_shapes_graph`] (Python `entail`, WebAssembly
//! `shaclEntail`, C `purrdf_shacl_entail_to_ntriples`). A SHACL-AF SPARQL rule runs in
//! the shapes-graph context, so its `$shapesGraph` is pre-bound to the named IRI.
//!
//! The rule infers `ex:alice ex:shapesGraph ?g` with `?g` = `COALESCE($shapesGraph,
//! ex:none)`: named, `?g` is the IRI; unnamed, `$shapesGraph` is an ordinary, unbound
//! variable and `?g` is `ex:none`. A second rule reads the shapes graph through
//! `GRAPH $shapesGraph`, which only the named run can. Both answers are asserted, so the
//! IRI is observed rather than assumed.

use purrdf_validate::{
    RulesRequest, ShapesError, apply_rules_to_ntriples, entail_to_ntriples_string,
    entail_to_ntriples_string_with_shapes_graph,
};

const SHAPES: &str = r#"@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:S a sh:NodeShape ;
  sh:targetClass ex:Person ;
  ex:marker ex:secret ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <http://example.org/ns#shapesGraph> ?g }
    WHERE { BIND (COALESCE($shapesGraph, <http://example.org/ns#none>) AS ?g) }""" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <http://example.org/ns#marked> ?m }
    WHERE { GRAPH $shapesGraph { $currentShape <http://example.org/ns#marker> ?m } }""" ] .
"#;

const DATA: &str = "<http://example.org/ns#alice> \
    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Person> .\n";

const GRAPH: &str = "http://example.org/shapes-graph";

fn named(iri: &str) -> String {
    format!("<http://example.org/ns#alice> <http://example.org/ns#shapesGraph> <{iri}> .")
}

const UNNAMED: &str = "<http://example.org/ns#alice> <http://example.org/ns#shapesGraph> \
    <http://example.org/ns#none> .";

const MARKED: &str = "<http://example.org/ns#alice> <http://example.org/ns#marked> \
    <http://example.org/ns#secret> .";

fn rules(shapes_graph: Option<&str>, shapes_base: Option<&str>) -> Result<String, ShapesError> {
    apply_rules_to_ntriples(&RulesRequest {
        data_nt: DATA,
        shapes_ttl: Some(SHAPES),
        shapes_base,
        shapes_graph,
        ..RulesRequest::default()
    })
    .map(|outcome| outcome.inferred_ntriples)
}

#[test]
fn a_sparql_rule_sees_the_shapes_graph_iri_it_is_given_and_an_ordinary_variable_without() {
    let with = rules(Some(GRAPH), None).expect("rules run");
    assert!(with.contains(&named(GRAPH)), "{with}");
    assert!(with.contains(MARKED), "{with}");
    assert!(!with.contains(UNNAMED), "{with}");

    let without = rules(None, None).expect("rules run");
    assert!(without.contains(UNNAMED), "{without}");
    assert!(!without.contains(MARKED), "{without}");
    assert!(!without.contains(GRAPH), "{without}");
}

/// A relative IRI resolves against the shapes document's base, exactly as validation's
/// does; with no base it names no graph and is refused, beside the absolute neighbour that
/// runs.
#[test]
fn a_relative_shapes_graph_iri_resolves_against_the_shapes_base() {
    let resolved = rules(Some("shapes-graph"), Some("http://example.org/doc"))
        .expect("a relative IRI with a base runs");
    assert!(resolved.contains(&named(GRAPH)), "{resolved}");
    let refused = rules(Some("shapes-graph"), None).expect_err("no base, no graph");
    assert!(
        refused.to_string().contains("iri-relative-no-base"),
        "{refused}"
    );
}

/// A SPARQL 1.2 RL rule set has no shapes graph, so a shapes-graph IRI beside one is
/// refused rather than dropped; the same rule set without it runs.
#[test]
fn a_shapes_graph_iri_beside_a_sparql_rl_rule_set_is_refused() {
    let srl = "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:tagged true } WHERE { ?x a ex:Person }\n";
    let request = |shapes_graph| RulesRequest {
        data_nt: DATA,
        srl: Some(srl),
        shapes_graph,
        ..RulesRequest::default()
    };
    let refused = apply_rules_to_ntriples(&request(Some(GRAPH))).expect_err("refused");
    assert!(
        refused.to_string().contains("has no shapes graph"),
        "{refused}"
    );
    let ran = apply_rules_to_ntriples(&request(None)).expect("the rule set runs");
    assert!(
        ran.inferred_ntriples
            .contains("<http://example.org/ns#tagged>"),
        "{}",
        ran.inferred_ntriples
    );
}

/// The entailment entry point takes the same IRI: its output carries the named answer
/// with it and the ordinary-variable answer without it, and the plain entry point is the
/// unnamed one.
#[test]
fn entailment_sees_the_shapes_graph_iri_it_is_given() {
    let with = entail_to_ntriples_string_with_shapes_graph(SHAPES, None, Some(GRAPH), DATA, &[])
        .expect("entails");
    assert!(with.contains(&named(GRAPH)), "{with}");
    assert!(with.contains(MARKED), "{with}");
    let without = entail_to_ntriples_string_with_shapes_graph(SHAPES, None, None, DATA, &[])
        .expect("entails");
    assert!(without.contains(UNNAMED), "{without}");
    assert!(!without.contains(MARKED), "{without}");
    assert_eq!(
        without,
        entail_to_ntriples_string(SHAPES, None, DATA, &[]).expect("entails")
    );
}
