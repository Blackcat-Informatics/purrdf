// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shapes graph's SHACL type of a constraint component and of a SPARQL validator
//! follows `rdfs:subClassOf` whatever the kind of its nodes. SHACL 1.2 Core, "SHACL
//! Type": "The SHACL types of an RDF term in an RDF graph is the set of its values for
//! rdf:type in the graph as well as the SHACL superclasses of these values in the
//! graph" — a blank node is a value of `rdf:type` and a SHACL superclass like any other.
//!
//! Every case sits beside an IRI control that spells the same chain with IRIs only.

use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
    @prefix sh: <http://www.w3.org/ns/shacl#> .\n";

/// A custom component `ex:Required` whose ASK validator requires `ex:p` to reach the
/// parameter's value, declared with `component_type` and `validator_type`, used by a
/// shape whose focus node lacks the required value; `axioms` carry the subclass chains.
fn shapes_graph(axioms: &str, component_type: &str, validator_type: &str) -> String {
    format!(
        "{PREFIXES}{axioms}\n\
         ex:Required a {component_type} ;\n\
             sh:parameter [ sh:path ex:required ] ;\n\
             sh:validator [ a {validator_type} ;\n\
                 sh:ask \"ASK {{ $this <http://example.org/ns#p> $required }}\" ] .\n\
         ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ; ex:required ex:a .\n"
    )
}

/// The number of results validating the focus node, which has `ex:p ex:b` but not
/// `ex:p ex:a`, against `shapes`; zero when `ex:Required` is not a component.
fn results(shapes: &str) -> usize {
    let shapes = parse_shapes(shapes, None).expect("the shapes graph loads");
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}ex:focus ex:p ex:b .\n"), None)
        .expect("data parses");
    validate_dataset_with_shapes_graph(&data, &shapes, None)
        .expect("validates")
        .results
        .len()
}

fn component(axioms: &str) -> usize {
    results(&shapes_graph(
        axioms,
        "ex:CompClass",
        "sh:SPARQLAskValidator",
    ))
}

/// One blank node between the component's class and `sh:ConstraintComponent`
/// registers the component, exactly as an IRI in its place does.
#[test]
fn a_blank_intermediate_registers_the_component() {
    assert_eq!(
        component(
            "ex:CompClass rdfs:subClassOf ex:Mid .\nex:Mid rdfs:subClassOf sh:ConstraintComponent ."
        ),
        1
    );
    assert_eq!(
        component(
            "ex:CompClass rdfs:subClassOf _:x .\n_:x rdfs:subClassOf sh:ConstraintComponent ."
        ),
        1
    );
}

/// A blank `rdf:type` value is a SHACL type: a component typed with a blank class
/// that is a subclass of `sh:ConstraintComponent` registers.
#[test]
fn a_blank_component_type_registers_the_component() {
    let axioms = "_:cc rdfs:subClassOf sh:ConstraintComponent .\n\
        ex:CompClass rdfs:subClassOf sh:ConstraintComponent .";
    assert_eq!(
        results(&shapes_graph(
            axioms,
            "ex:CompClass",
            "sh:SPARQLAskValidator"
        )),
        1
    );
    assert_eq!(
        results(&shapes_graph(axioms, "_:cc", "sh:SPARQLAskValidator")),
        1
    );
}

/// A cycle of blank nodes under the component's class terminates and registers
/// nothing; the same cycle with one exit to `sh:ConstraintComponent` registers.
#[test]
fn a_blank_only_cycle_terminates() {
    let cycle = "ex:CompClass rdfs:subClassOf _:c1 .\n\
        _:c1 rdfs:subClassOf _:c2 .\n_:c2 rdfs:subClassOf _:c1 .";
    assert_eq!(
        component(&format!(
            "{cycle}\n_:c2 rdfs:subClassOf sh:ConstraintComponent ."
        )),
        1
    );
    assert_eq!(component(cycle), 0);
}

/// A blank node on no path to `sh:ConstraintComponent` registers nothing: the
/// component's class ends in a dead-end blank, and the blank that does reach
/// `sh:ConstraintComponent` is not above it.
#[test]
fn a_dead_end_blank_registers_nothing() {
    assert_eq!(
        component(
            "ex:CompClass rdfs:subClassOf _:dead .\n_:other rdfs:subClassOf sh:ConstraintComponent ."
        ),
        0
    );
    assert_eq!(
        component(
            "ex:CompClass rdfs:subClassOf _:dead .\n_:dead rdfs:subClassOf sh:ConstraintComponent ."
        ),
        1
    );
}

/// A validator typed with a class that reaches `sh:SPARQLAskValidator` through a blank
/// node — or typed with a blank class that does — is an ASK validator, not a
/// `validator-class` violation, exactly as with an IRI in its place. A validator whose
/// chain dead-ends in a blank is still refused.
#[test]
fn a_blank_validator_chain_is_an_ask_validator() {
    let component_axiom = "ex:CompClass rdfs:subClassOf sh:ConstraintComponent .\n";
    let validator = |axioms: &str, validator_type: &str| {
        shapes_graph(
            &format!("{component_axiom}{axioms}"),
            "ex:CompClass",
            validator_type,
        )
    };
    assert_eq!(
        results(&validator(
            "ex:MyAsk rdfs:subClassOf ex:MidV .\nex:MidV rdfs:subClassOf sh:SPARQLAskValidator .",
            "ex:MyAsk"
        )),
        1
    );
    assert_eq!(
        results(&validator(
            "ex:MyAsk rdfs:subClassOf _:y .\n_:y rdfs:subClassOf sh:SPARQLAskValidator .",
            "ex:MyAsk"
        )),
        1
    );
    assert_eq!(
        results(&validator(
            "_:vt rdfs:subClassOf sh:SPARQLAskValidator .",
            "_:vt"
        )),
        1
    );
    let refused = parse_shapes(
        &validator(
            "ex:MyAsk rdfs:subClassOf _:y .\n_:z rdfs:subClassOf sh:SPARQLAskValidator .",
            "ex:MyAsk",
        ),
        None,
    )
    .expect_err("a validator of no SPARQL validator class is refused")
    .to_string();
    assert!(refused.contains("validator-class"), "{refused}");
}
