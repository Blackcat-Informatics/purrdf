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
/// `rdf:subject`.
pub(crate) const RDF_SUBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";
/// `rdf:predicate`.
pub(crate) const RDF_PREDICATE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";
/// `rdf:object`.
pub(crate) const RDF_OBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";
/// `rdf:reifies` — RDF 1.2's reifier property.
pub(crate) const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
/// `rdf:value`.
pub(crate) const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
/// `rdf:List`.
pub(crate) const RDF_LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";
/// `rdf:Statement`.
pub(crate) const RDF_STATEMENT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement";
/// `rdf:Alt`.
pub(crate) const RDF_ALT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Alt";
/// `rdf:Bag`.
pub(crate) const RDF_BAG: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Bag";
/// `rdf:Seq`.
pub(crate) const RDF_SEQ: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Seq";
/// `rdf:langString` — a datatype RDF 1.2 Semantics §8 requires every RDF interpretation to
/// recognize.
pub(crate) const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
/// `rdf:dirLangString` — likewise mandatory, and new in RDF 1.2.
pub(crate) const RDF_DIRLANGSTRING: &str =
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
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
/// `rdfs:Literal`.
pub(crate) const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
/// `rdfs:Datatype`.
pub(crate) const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
/// `rdfs:Container`.
pub(crate) const RDFS_CONTAINER: &str = "http://www.w3.org/2000/01/rdf-schema#Container";
/// `rdfs:ContainerMembershipProperty`.
pub(crate) const RDFS_CONTAINERMEMBERSHIPPROPERTY: &str =
    "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty";
/// `rdfs:member`.
pub(crate) const RDFS_MEMBER: &str = "http://www.w3.org/2000/01/rdf-schema#member";
/// `rdfs:seeAlso`.
pub(crate) const RDFS_SEEALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
/// `rdfs:isDefinedBy`.
pub(crate) const RDFS_ISDEFINEDBY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";
/// `rdfs:comment`.
pub(crate) const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
/// `rdfs:label`.
pub(crate) const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
/// `rdfs:Proposition` — the class RDF 1.2 Semantics §9.2.1's `rdfs14` / `rdfs14a` type a
/// triple term's surrogate blank node with.
pub(crate) const RDFS_PROPOSITION: &str = "http://www.w3.org/2000/01/rdf-schema#Proposition";
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
/// `xsd:string` — the datatype a literal carries when it carries no other (RDF 1.1 C0.1),
/// and therefore the one a canonical N-Quads surface leaves implicit.
pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

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
