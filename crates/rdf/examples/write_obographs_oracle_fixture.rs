// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit a production RDF→OBO Graphs projection for the independent schema oracle.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use purrdf_rdf::{
    BlankScope, OboGraphsConfig, OboGraphsVocabulary, OboMetadataRoles, OboOwlRoles, OboRdfRoles,
    ProjectionLimits, RdfDatasetBuilder, RdfLiteral, TermId, project_obo_graphs,
};

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const OBO: &str = "http://www.geneontology.org/formats/oboInOwl#";
const EX: &str = "http://example.org/obographs-oracle/";

fn iri(builder: &mut RdfDatasetBuilder, value: &str) -> TermId {
    builder.intern_iri(value)
}

fn push_iri(
    builder: &mut RdfDatasetBuilder,
    subject: &str,
    predicate: &str,
    object: &str,
) -> (TermId, TermId, TermId) {
    let subject = iri(builder, subject);
    let predicate = iri(builder, predicate);
    let object = iri(builder, object);
    builder.push_quad(subject, predicate, object, None);
    (subject, predicate, object)
}

fn config() -> Result<OboGraphsConfig, Box<dyn Error>> {
    let vocabulary = OboGraphsVocabulary::new(
        OboRdfRoles::new(
            format!("{RDF}type"),
            format!("{RDF}reifies"),
            format!("{RDF}first"),
            format!("{RDF}rest"),
            format!("{RDF}nil"),
            format!("{XSD}string"),
            format!("{XSD}boolean"),
        )?,
        OboOwlRoles::new(
            format!("{RDFS}label"),
            format!("{RDFS}comment"),
            format!("{RDFS}subClassOf"),
            format!("{RDFS}subPropertyOf"),
            format!("{RDFS}domain"),
            format!("{RDFS}range"),
            format!("{OWL}Ontology"),
            format!("{OWL}Class"),
            format!("{OWL}NamedIndividual"),
            format!("{OWL}ObjectProperty"),
            format!("{OWL}AnnotationProperty"),
            format!("{OWL}DatatypeProperty"),
            format!("{OWL}equivalentClass"),
            format!("{OWL}intersectionOf"),
            format!("{OWL}Restriction"),
            format!("{OWL}onProperty"),
            format!("{OWL}someValuesFrom"),
            format!("{OWL}allValuesFrom"),
            format!("{OWL}propertyChainAxiom"),
            format!("{OWL}deprecated"),
        )?,
        OboMetadataRoles::new(
            format!("{EX}definition"),
            format!("{OBO}hasExactSynonym"),
            format!("{OBO}hasBroadSynonym"),
            format!("{OBO}hasNarrowSynonym"),
            format!("{OBO}hasRelatedSynonym"),
            format!("{OBO}hasSynonymType"),
            format!("{OBO}hasDbXref"),
            format!("{OBO}inSubset"),
            format!("{OWL}versionInfo"),
        )?,
    )?;
    Ok(OboGraphsConfig::new(
        format!("{EX}ontology"),
        vocabulary,
        ProjectionLimits::new(16, 1_000_000, 2_000_000, 3_000_000, 16)?,
        1_000,
    )?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: write_obographs_oracle_fixture OUTPUT_JSON")?,
    );
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = format!("{RDF}type");
    let rdf_first = format!("{RDF}first");
    let rdf_rest = format!("{RDF}rest");
    let rdf_nil = format!("{RDF}nil");
    let subclass = format!("{RDFS}subClassOf");
    let equivalent = format!("{OWL}equivalentClass");
    let restriction_type = format!("{OWL}Restriction");
    let on_property = format!("{OWL}onProperty");
    let some_values = format!("{OWL}someValuesFrom");
    let class_a = format!("{EX}A");
    let class_b = format!("{EX}B");
    let class_c = format!("{EX}C");
    let property_p = format!("{EX}p");
    let property_q = format!("{EX}q");
    let property_r = format!("{EX}r");

    push_iri(
        &mut builder,
        &format!("{EX}ontology"),
        &rdf_type,
        &format!("{OWL}Ontology"),
    );
    for class in [&class_a, &class_b, &class_c] {
        push_iri(&mut builder, class, &rdf_type, &format!("{OWL}Class"));
    }
    for property in [&property_p, &property_q, &property_r] {
        push_iri(
            &mut builder,
            property,
            &rdf_type,
            &format!("{OWL}ObjectProperty"),
        );
    }

    let a = iri(&mut builder, &class_a);
    let label = iri(&mut builder, &format!("{RDFS}label"));
    let alpha = builder.intern_literal(RdfLiteral {
        lexical_form: "Alpha".to_owned(),
        datatype: None,
        language: None,
        direction: None,
    });
    builder.push_quad(a, label, alpha, None);
    let definition = iri(&mut builder, &format!("{EX}definition"));
    let definition_text = builder.intern_literal(RdfLiteral {
        lexical_form: "The alpha class".to_owned(),
        datatype: None,
        language: None,
        direction: None,
    });
    builder.push_quad(a, definition, definition_text, None);
    push_iri(
        &mut builder,
        &class_a,
        &format!("{OBO}inSubset"),
        &format!("{EX}subset"),
    );

    let basic_edge = push_iri(&mut builder, &class_a, &subclass, &class_b);
    let basic_edge_term = builder.intern_triple(basic_edge.0, basic_edge.1, basic_edge.2);
    let reifier = iri(&mut builder, &format!("{EX}axiom"));
    builder.push_reifier(reifier, basic_edge_term);
    let confidence = iri(&mut builder, &format!("{EX}confidence"));
    let high = iri(&mut builder, &format!("{EX}high"));
    builder.push_annotation(reifier, confidence, high);

    push_iri(&mut builder, &class_a, &equivalent, &class_b);
    let expression = builder.intern_blank("expression", BlankScope::DEFAULT);
    let list_one = builder.intern_blank("intersection-1", BlankScope::DEFAULT);
    let list_two = builder.intern_blank("intersection-2", BlankScope::DEFAULT);
    let restriction = builder.intern_blank("restriction", BlankScope::DEFAULT);
    let c = iri(&mut builder, &class_c);
    let equivalent_id = iri(&mut builder, &equivalent);
    builder.push_quad(c, equivalent_id, expression, None);
    let intersection = iri(&mut builder, &format!("{OWL}intersectionOf"));
    builder.push_quad(expression, intersection, list_one, None);
    let first = iri(&mut builder, &rdf_first);
    let rest = iri(&mut builder, &rdf_rest);
    builder.push_quad(list_one, first, a, None);
    builder.push_quad(list_one, rest, list_two, None);
    builder.push_quad(list_two, first, restriction, None);
    let nil = iri(&mut builder, &rdf_nil);
    builder.push_quad(list_two, rest, nil, None);
    let rdf_type_id = iri(&mut builder, &rdf_type);
    let restriction_type_id = iri(&mut builder, &restriction_type);
    builder.push_quad(restriction, rdf_type_id, restriction_type_id, None);
    let on_property_id = iri(&mut builder, &on_property);
    let p = iri(&mut builder, &property_p);
    builder.push_quad(restriction, on_property_id, p, None);
    let some_values_id = iri(&mut builder, &some_values);
    let b = iri(&mut builder, &class_b);
    builder.push_quad(restriction, some_values_id, b, None);

    push_iri(
        &mut builder,
        &property_p,
        &format!("{RDFS}domain"),
        &class_a,
    );
    push_iri(&mut builder, &property_p, &format!("{RDFS}range"), &class_b);

    let chain_head = builder.intern_blank("chain-1", BlankScope::DEFAULT);
    let chain_tail = builder.intern_blank("chain-2", BlankScope::DEFAULT);
    let r = iri(&mut builder, &property_r);
    let chain = iri(&mut builder, &format!("{OWL}propertyChainAxiom"));
    builder.push_quad(r, chain, chain_head, None);
    builder.push_quad(chain_head, first, p, None);
    builder.push_quad(chain_head, rest, chain_tail, None);
    let q = iri(&mut builder, &property_q);
    builder.push_quad(chain_tail, first, q, None);
    builder.push_quad(chain_tail, rest, nil, None);

    let dataset = builder.freeze()?;
    let configuration = config()?;
    let projection = project_obo_graphs(dataset.as_ref(), &configuration)?;
    let bytes = projection.document.to_canonical_json(&configuration)?;
    fs::write(output, bytes)?;
    Ok(())
}
