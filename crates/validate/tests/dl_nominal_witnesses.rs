// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A WITNESS A NOMINAL ABSORBS GENERATES NOTHING OF ITS OWN.
//!
//! `D ⊑ {n}` says every `D` is `n`, so a fresh `D`-successor is identified with `n` as soon as
//! it carries `D`. When the successor's own label also demands a successor — `∃r.D`, read off
//! a universal over a transitive role, and so present at every node the chain reaches — the
//! order the rules fire in decides whether the search ends: a search that mints the
//! successor's successor before identifying it with `n` grows a fresh chain node every round
//! and stops only at its budget. These knowledge bases all reached `unknown` that way; the
//! decision core now mints only after hyperresolution reaches a fixpoint, and a witness still
//! to be identified with a nominal only after the `⊔`-rule has made that choice, so each one
//! is folded into its nominal before it can generate anything, and all of them decide.
//!
//! Every one is SATISFIABLE: a single element `n` that is its own `r`- and `t`-successor and is
//! `D` is a model of the shape.

use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

/// The prefixes every fixture below is written against.
const PREFIXES: &str = "@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

/// `r` transitive, `j r g`, `j : ∀r.∃r.D`, `D ⊑ {n}`.
const TRANSITIVE_NOMINAL: &str = r":r a owl:ObjectProperty . :s a owl:ObjectProperty . :q a owl:ObjectProperty . :p a owl:ObjectProperty . :t a owl:ObjectProperty .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:TransitiveProperty .
:D rdfs:subClassOf [ owl:oneOf ( :n ) ] .
:j :r :g .
:j a [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom [ a owl:Restriction ; owl:onProperty :r ; owl:someValuesFrom :D ] ] .
";

/// The same over `t ⊑ r`: `j : ∀r.∃t.D`.
const SUB_ROLE_NOMINAL: &str = r":r a owl:ObjectProperty . :s a owl:ObjectProperty . :q a owl:ObjectProperty . :p a owl:ObjectProperty . :t a owl:ObjectProperty .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:TransitiveProperty .
:t rdfs:subPropertyOf :r .
:D rdfs:subClassOf [ owl:oneOf ( :n ) ] .
:j :r :g .
:j a [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom [ a owl:Restriction ; owl:onProperty :t ; owl:someValuesFrom :D ] ] .
";

/// The same with `D ⊑ {n, l}`, an identification the `⊔`-rule chooses.
const SUB_ROLE_NOMINAL_CHOICE: &str = r":r a owl:ObjectProperty . :s a owl:ObjectProperty . :q a owl:ObjectProperty . :p a owl:ObjectProperty . :t a owl:ObjectProperty .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:TransitiveProperty .
:t rdfs:subPropertyOf :r .
:D rdfs:subClassOf [ owl:oneOf ( :n :l ) ] .
:j :r :g .
:j a [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom [ a owl:Restriction ; owl:onProperty :t ; owl:someValuesFrom :D ] ] .
";

/// The same with `t` transitive too.
const TRANSITIVE_SUB_ROLE_NOMINAL_CHOICE: &str = r":r a owl:ObjectProperty . :s a owl:ObjectProperty . :q a owl:ObjectProperty . :p a owl:ObjectProperty . :t a owl:ObjectProperty .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:TransitiveProperty .
:t a owl:TransitiveProperty . :t rdfs:subPropertyOf :r .
:D rdfs:subClassOf [ owl:oneOf ( :n :l ) ] .
:j :r :g .
:j a [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom [ a owl:Restriction ; owl:onProperty :t ; owl:someValuesFrom :D ] ] .
";

/// A fifty-triple generated knowledge base over the same shape, among others.
const GENERATED_FIFTY_TRIPLES: &str = r#":r a owl:ObjectProperty . :s a owl:ObjectProperty . :q a owl:ObjectProperty . :p a owl:ObjectProperty . :t a owl:ObjectProperty .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:TransitiveProperty .
:q owl:inverseOf :r .
:t a owl:TransitiveProperty . :t rdfs:subPropertyOf :r .
:D rdfs:subClassOf [ owl:oneOf ( :n :l ) ] .
:C rdfs:subClassOf :D .
:A rdfs:subClassOf :B .
:C rdfs:subClassOf [ owl:unionOf ( [ owl:complementOf :D ] [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :r ] ; owl:someValuesFrom :C ] ) ] .
:B rdfs:subClassOf [ owl:unionOf ( [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:hasValue :k ] [ owl:complementOf :B ] ) ] .
:a a owl:NamedIndividual .
:b a owl:NamedIndividual .
:c a owl:NamedIndividual .
:d a owl:NamedIndividual .
:e a owl:NamedIndividual .
:f a owl:NamedIndividual .
:g a owl:NamedIndividual .
:h a owl:NamedIndividual .
:i a owl:NamedIndividual .
:j a owl:NamedIndividual .
:k a owl:NamedIndividual .
:l a owl:NamedIndividual .
:m a owl:NamedIndividual .
:n a owl:NamedIndividual .
:d :q :a .
:c :t :g .
:j :s :e .
:f :p :f .
:a :t :d .
:i :s :i .
:c :s :f .
:g :r :d .
:c :s :d .
:i :p :h .
:g :q :f .
:j :q :a .
:i :p :b .
:e :t :c .
:a :p :i .
:c :p :c .
:d :r :i .
:b :s :j .
:g :q :j .
:e a [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :t ] ; owl:someValuesFrom [ a owl:Restriction ; owl:onProperty :t ; owl:someValuesFrom [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:minQualifiedCardinality "2"^^xsd:nonNegativeInteger ; owl:onClass :B ] ] ] .
:j a [ a owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom [ owl:intersectionOf ( [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:allValuesFrom :C ] [ a owl:Restriction ; owl:onProperty :t ; owl:someValuesFrom :D ] ) ] ] .
:c a [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :t ] ; owl:allValuesFrom :B ] .
:i a [ owl:intersectionOf ( [ a owl:Restriction ; owl:onProperty :p ; owl:allValuesFrom [ owl:complementOf :A ] ] [ a owl:Restriction ; owl:onProperty :s ; owl:someValuesFrom :D ] ) ] .
:d a [ owl:complementOf :A ] .
"#;

/// A generated knowledge base whose witnesses must be identified with nominals under counting bounds — the guard on the ORDER: identifying every waiting witness before anything else is minted makes this search branch a hundred thousand times, so only a witness that is still to be identified waits.
const GENERATED_IDENTIFICATIONS: &str = r#":r a owl:ObjectProperty . :s a owl:ObjectProperty . :q a owl:ObjectProperty . :p a owl:ObjectProperty . :t a owl:ObjectProperty .
:A a owl:Class . :B a owl:Class . :C a owl:Class . :D a owl:Class .
:r a owl:TransitiveProperty .
:q owl:inverseOf :r .
:t a owl:TransitiveProperty . :t rdfs:subPropertyOf :r .
:p rdfs:subPropertyOf :s .
:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:minQualifiedCardinality "2"^^xsd:nonNegativeInteger ; owl:onClass :D ] .
:B rdfs:subClassOf [ owl:unionOf ( [ owl:complementOf :B ] [ owl:unionOf ( [ owl:complementOf :D ] :A ) ] ) ] .
:D rdfs:subClassOf [ owl:oneOf ( :e :n ) ] .
:C rdfs:subClassOf :D .
:C rdfs:subClassOf :D .
:D rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :s ; owl:maxQualifiedCardinality "1"^^xsd:nonNegativeInteger ; owl:onClass [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:allValuesFrom :A ] ] .
:a a owl:NamedIndividual .
:b a owl:NamedIndividual .
:c a owl:NamedIndividual .
:d a owl:NamedIndividual .
:e a owl:NamedIndividual .
:f a owl:NamedIndividual .
:g a owl:NamedIndividual .
:h a owl:NamedIndividual .
:i a owl:NamedIndividual .
:j a owl:NamedIndividual .
:k a owl:NamedIndividual .
:l a owl:NamedIndividual .
:m a owl:NamedIndividual .
:n a owl:NamedIndividual .
:d :r :i .
:c :p :d .
:d :q :f .
:i :t :f .
:i :q :c .
:e :q :a .
:h :p :h .
:d :s :h .
:c :p :c .
:d :s :h .
:i a :C .
:d a [ owl:unionOf ( [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :t ] ; owl:someValuesFrom :C ] :D ) ] .
:e a :D .
:h a [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:minQualifiedCardinality "2"^^xsd:nonNegativeInteger ; owl:onClass [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :s ] ; owl:minQualifiedCardinality "2"^^xsd:nonNegativeInteger ; owl:onClass [ owl:unionOf ( [ owl:complementOf :C ] :A ) ] ] ] .
:c a :C .
"#;

/// The fixture as the canonical N-Quads the string boundary parses.
fn document(body: &str) -> String {
    let ontology = format!("{PREFIXES}{body}");
    let dataset =
        parse_dataset(ontology.as_bytes(), "text/turtle", None).expect("the ontology parses");
    let bytes = serialize_dataset(&*dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("the ontology serializes");
    String::from_utf8(bytes).expect("N-Quads is UTF-8")
}

#[test]
fn every_absorbed_witness_shape_decides_consistent() {
    for (name, body) in [
        ("v_f", TRANSITIVE_NOMINAL),
        ("v_d", SUB_ROLE_NOMINAL),
        ("v_c", SUB_ROLE_NOMINAL_CHOICE),
        ("v_a", TRANSITIVE_SUB_ROLE_NOMINAL_CHOICE),
        ("R2_170", GENERATED_FIFTY_TRIPLES),
        ("R2_2023", GENERATED_IDENTIFICATIONS),
    ] {
        let answer =
            purrdf_validate::regime::consistency_to_string(&document(body), &[], &[], 0, 0)
                .expect("the ontology reverse-maps");
        assert_eq!(answer.answer(), "consistency true\n", "{name}: the verdict");
        assert!(
            answer.certificate().contains("\ncompleteness decided\n"),
            "{name}: a decided verdict, not a truncated search"
        );
    }
}
