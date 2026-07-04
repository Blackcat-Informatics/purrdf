// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standard vocabulary IRIs shared by every entailment engine.
//!
//! These are spec-supplied `rdf:`/`rdfs:`/`owl:` IRIs from the RDF 1.1 Semantics /
//! OWL 2 calculus — PurRDF mints **none** of its own. Every engine (the RDFS/OWL-RL
//! chase, the OWL-Direct tableau, the RIF-Core evaluator) draws its constant IRIs
//! from this one table so there is a single source of truth.

/// `rdf:type`.
pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:Property`.
pub(crate) const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
/// `rdfs:subClassOf`.
pub(crate) const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
/// `rdfs:subPropertyOf`.
pub(crate) const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
/// `rdfs:domain`.
pub(crate) const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
/// `rdfs:range`.
pub(crate) const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
/// `rdfs:Class`.
pub(crate) const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
/// `rdfs:Resource`.
pub(crate) const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
/// `owl:equivalentClass`.
pub(crate) const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
/// `owl:equivalentProperty`.
pub(crate) const OWL_EQUIVALENTPROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
/// `owl:inverseOf`.
pub(crate) const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
/// `owl:SymmetricProperty`.
pub(crate) const OWL_SYMMETRICPROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
/// `owl:TransitiveProperty`.
pub(crate) const OWL_TRANSITIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";

// --- OWL 2 DL class-expression and axiom vocabulary (OWL-Direct reverse mapping). ---

/// `owl:Thing` — the top concept ⊤.
pub(crate) const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
/// `owl:Nothing` — the bottom concept ⊥.
pub(crate) const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
/// `owl:Class`.
pub(crate) const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
/// `owl:Restriction`.
pub(crate) const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
/// `owl:onProperty`.
pub(crate) const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
/// `owl:someValuesFrom`.
pub(crate) const OWL_SOMEVALUESFROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
/// `owl:allValuesFrom`.
pub(crate) const OWL_ALLVALUESFROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
/// `owl:intersectionOf`.
pub(crate) const OWL_INTERSECTIONOF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
/// `owl:unionOf`.
pub(crate) const OWL_UNIONOF: &str = "http://www.w3.org/2002/07/owl#unionOf";
/// `owl:complementOf`.
pub(crate) const OWL_COMPLEMENTOF: &str = "http://www.w3.org/2002/07/owl#complementOf";
/// `owl:oneOf`.
pub(crate) const OWL_ONEOF: &str = "http://www.w3.org/2002/07/owl#oneOf";
/// `owl:hasValue`.
pub(crate) const OWL_HASVALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
/// `owl:minCardinality`.
pub(crate) const OWL_MINCARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";
/// `owl:maxCardinality`.
pub(crate) const OWL_MAXCARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
/// `owl:cardinality`.
pub(crate) const OWL_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";
/// `owl:minQualifiedCardinality`.
pub(crate) const OWL_MINQUALIFIEDCARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#minQualifiedCardinality";
/// `owl:maxQualifiedCardinality`.
pub(crate) const OWL_MAXQUALIFIEDCARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
/// `owl:qualifiedCardinality`.
pub(crate) const OWL_QUALIFIEDCARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#qualifiedCardinality";
/// `owl:onClass`.
pub(crate) const OWL_ONCLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
/// `owl:disjointWith`.
pub(crate) const OWL_DISJOINTWITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
/// `owl:sameAs`.
pub(crate) const OWL_SAMEAS: &str = "http://www.w3.org/2002/07/owl#sameAs";
/// `owl:FunctionalProperty`.
pub(crate) const OWL_FUNCTIONALPROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
/// `owl:ObjectProperty`.
pub(crate) const OWL_OBJECTPROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
/// `owl:DatatypeProperty`.
pub(crate) const OWL_DATATYPEPROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
/// `owl:NamedIndividual`.
pub(crate) const OWL_NAMEDINDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
/// `owl:Ontology`.
pub(crate) const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
/// `rdf:first`.
pub(crate) const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
/// `rdf:rest`.
pub(crate) const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
/// `rdf:nil`.
pub(crate) const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
