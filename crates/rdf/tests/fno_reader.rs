// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parsed RDF 1.2 FnO mappings retain complete native identity and graph content.

use purrdf_rdf::{
    DatasetView, FnoCatalog, RdfDatasetBuilder, TermValue, datasets_isomorphic, parse_dataset,
};

fn parsed_turtle_mappings_obey_both_graph_laws() {
    let text = r#"
@prefix fno: <https://w3id.org/function/ontology#> .
@prefix fnom: <https://w3id.org/function/vocabulary/mapping#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
<urn:test:doc> a owl:Ontology ; rdfs:label "Catalog" ; rdfs:comment "Original banner" ; dcterms:isPartOf <urn:test:ontology> .
<urn:test:function> a fno:Function, owl:Ontology ; rdfs:label "مرحبا"@ar--rtl ; fno:expects ( <urn:test:param> ) ; fno:returns ( <urn:test:output> ) .
<urn:test:param> a fno:Parameter ; fno:type xsd:string ; fno:required true ; fno:predicate <urn:test:value> .
<urn:test:output> a fno:Output ; fno:type xsd:string .
<urn:test:impl> a fno:Implementation ; dcterms:format "application/sparql-query" ; rdfs:seeAlso <urn:test:query> .
_:mapping a fno:Mapping ; rdfs:label "mapping" ; fno:function <urn:test:function> ; fno:implementation <urn:test:impl> ; fno:parameterMapping _:parameter ; fno:returnMapping _:returned .
_:parameter a fnom:PropertyParameterMapping ; rdfs:label "parameter" ; fnom:functionParameter <urn:test:param> ; fnom:implementationProperty "input" .
_:returned a fnom:DefaultReturnMapping ; rdfs:label "returned" ; fnom:functionOutput <urn:test:output> ; fnom:implementationProperty "result" .
"#;
    let source = parse_dataset(text.as_bytes(), "text/turtle", None).unwrap();
    let catalog = FnoCatalog::from_dataset(&source, "urn:test:ontology").unwrap();
    let mapping_class = TermValue::iri("https://w3id.org/function/ontology#Mapping");
    let mapping = source
        .quads()
        .find(|row| source.term_value(row.o).unwrap() == mapping_class)
        .unwrap();
    assert_eq!(
        catalog.as_input().mappings[0].node.term_value(),
        source.term_value(mapping.s).unwrap()
    );
    let mut builder = RdfDatasetBuilder::new();
    for row in purrdf_rdf::fno::to_quads(&catalog) {
        builder.push_owned_quad(&row).unwrap();
    }
    let emitted = builder.freeze().unwrap();
    assert_eq!(
        FnoCatalog::from_dataset(&emitted, "urn:test:ontology").unwrap(),
        catalog
    );
    assert!(datasets_isomorphic(&source, &emitted).unwrap());
}

purrdf_testkit::harness_main!(parsed_turtle_mappings_obey_both_graph_laws);
