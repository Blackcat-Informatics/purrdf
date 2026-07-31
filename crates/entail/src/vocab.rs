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

// --- OWL 2 RL property-axiom, class-axiom and schema vocabulary (Tables 5, 7 and 9). ---

/// `owl:AnnotationProperty` — the class `prp-ap` types the built-in annotation properties
/// with.
pub(crate) const OWL_ANNOTATIONPROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
/// `owl:versionInfo` — a built-in annotation property.
pub(crate) const OWL_VERSIONINFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";
/// `owl:priorVersion` — a built-in annotation property.
pub(crate) const OWL_PRIORVERSION: &str = "http://www.w3.org/2002/07/owl#priorVersion";
/// `owl:backwardCompatibleWith` — a built-in annotation property.
pub(crate) const OWL_BACKWARDCOMPATIBLEWITH: &str =
    "http://www.w3.org/2002/07/owl#backwardCompatibleWith";
/// `owl:incompatibleWith` — a built-in annotation property.
pub(crate) const OWL_INCOMPATIBLEWITH: &str = "http://www.w3.org/2002/07/owl#incompatibleWith";
/// `owl:deprecated` — a built-in annotation property.
pub(crate) const OWL_DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";
/// `owl:InverseFunctionalProperty`.
pub(crate) const OWL_INVERSEFUNCTIONALPROPERTY: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
/// `owl:IrreflexiveProperty`.
pub(crate) const OWL_IRREFLEXIVEPROPERTY: &str =
    "http://www.w3.org/2002/07/owl#IrreflexiveProperty";
/// `owl:AsymmetricProperty`.
pub(crate) const OWL_ASYMMETRICPROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";
/// `owl:propertyChainAxiom`.
pub(crate) const OWL_PROPERTYCHAINAXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
/// `owl:propertyDisjointWith`.
pub(crate) const OWL_PROPERTYDISJOINTWITH: &str =
    "http://www.w3.org/2002/07/owl#propertyDisjointWith";
/// `owl:AllDisjointProperties`.
pub(crate) const OWL_ALLDISJOINTPROPERTIES: &str =
    "http://www.w3.org/2002/07/owl#AllDisjointProperties";
/// `owl:AllDisjointClasses`.
pub(crate) const OWL_ALLDISJOINTCLASSES: &str = "http://www.w3.org/2002/07/owl#AllDisjointClasses";
/// `owl:members` — the list-valued property of `owl:AllDisjoint*` and `owl:AllDifferent`.
pub(crate) const OWL_MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";
/// `owl:distinctMembers` — `owl:AllDifferent`'s other list-valued property.
pub(crate) const OWL_DISTINCTMEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
/// `owl:hasKey`.
pub(crate) const OWL_HASKEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
/// `owl:sourceIndividual` — a negative property assertion's subject.
pub(crate) const OWL_SOURCEINDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#sourceIndividual";
/// `owl:assertionProperty` — a negative property assertion's predicate.
pub(crate) const OWL_ASSERTIONPROPERTY: &str = "http://www.w3.org/2002/07/owl#assertionProperty";
/// `owl:targetIndividual` — a negative OBJECT-property assertion's object.
pub(crate) const OWL_TARGETINDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#targetIndividual";
/// `owl:targetValue` — a negative DATA-property assertion's object.
pub(crate) const OWL_TARGETVALUE: &str = "http://www.w3.org/2002/07/owl#targetValue";

// --- OWL 2 RL equality and datatype vocabulary (Tables 4 and 8). ---

// --- OWL 2 role characteristics, self restrictions and built-in roles. ---

/// `owl:ReflexiveProperty` — the global role axiom `⊤ ⊑ ∃r.Self`.
pub(crate) const OWL_REFLEXIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";
/// `owl:hasSelf` — the local reflexivity restriction `∃r.Self`.
pub(crate) const OWL_HASSELF: &str = "http://www.w3.org/2002/07/owl#hasSelf";
/// `owl:disjointUnionOf` — `C ≡ C₁ ⊔ … ⊔ Cₙ` with the `Cᵢ` pairwise disjoint.
pub(crate) const OWL_DISJOINTUNIONOF: &str = "http://www.w3.org/2002/07/owl#disjointUnionOf";
/// `owl:NegativePropertyAssertion` — the reified `¬p(s, o)` axiom's class.
pub(crate) const OWL_NEGATIVEPROPERTYASSERTION: &str =
    "http://www.w3.org/2002/07/owl#NegativePropertyAssertion";
/// `owl:topObjectProperty` — the universal object role.
pub(crate) const OWL_TOPOBJECTPROPERTY: &str = "http://www.w3.org/2002/07/owl#topObjectProperty";
/// `owl:bottomObjectProperty` — the empty object role.
pub(crate) const OWL_BOTTOMOBJECTPROPERTY: &str =
    "http://www.w3.org/2002/07/owl#bottomObjectProperty";
/// `owl:topDataProperty` — the universal data role.
pub(crate) const OWL_TOPDATAPROPERTY: &str = "http://www.w3.org/2002/07/owl#topDataProperty";
/// `owl:bottomDataProperty` — the empty data role.
pub(crate) const OWL_BOTTOMDATAPROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomDataProperty";

// --- OWL 2 data ranges (the concrete domain). ---

/// `owl:onDatatype` — the base datatype of a datatype restriction.
pub(crate) const OWL_ONDATATYPE: &str = "http://www.w3.org/2002/07/owl#onDatatype";
/// `owl:withRestrictions` — the facet list of a datatype restriction.
pub(crate) const OWL_WITHRESTRICTIONS: &str = "http://www.w3.org/2002/07/owl#withRestrictions";
/// `owl:datatypeComplementOf` — the complement of a data range.
pub(crate) const OWL_DATATYPECOMPLEMENTOF: &str =
    "http://www.w3.org/2002/07/owl#datatypeComplementOf";
/// `owl:onDataRange` — the filler of a qualified DATA cardinality restriction.
pub(crate) const OWL_ONDATARANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";
/// `owl:onProperties` — the property list of an n-ary data restriction.
pub(crate) const OWL_ONPROPERTIES: &str = "http://www.w3.org/2002/07/owl#onProperties";
/// `owl:DataRange` — OWL 1's deprecated spelling of `rdfs:Datatype`.
pub(crate) const OWL_DATARANGE: &str = "http://www.w3.org/2002/07/owl#DataRange";
/// `owl:real` — OWL 2's built-in datatype for the real numbers.
pub(crate) const OWL_REAL: &str = "http://www.w3.org/2002/07/owl#real";
/// `owl:rational` — OWL 2's built-in datatype for the rationals.
pub(crate) const OWL_RATIONAL: &str = "http://www.w3.org/2002/07/owl#rational";

// --- The constraining facets of a datatype restriction. ---
//
// A facet is written as the sole predicate of one `owl:withRestrictions` list cell, so
// these IRIs occur in PREDICATE position and are recognized by the data-range reader
// rather than by the class-expression reader. They sit in the XML Schema namespace,
// which is not one of the three the OWL-2-RDF mapping reserves, so the reader must name
// them explicitly: a facet predicate that fell through to the caller's-own-vocabulary
// arm would become an ABox role assertion over a list cell.

/// The XML Schema namespace. An IRI in it names a DATATYPE — a data range — wherever a
/// class expression could otherwise be read.
pub(crate) const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
/// `xsd:minInclusive` — the inclusive lower bound of an ordered value space.
pub(crate) const XSD_MININCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#minInclusive";
/// `xsd:maxInclusive` — the inclusive upper bound of an ordered value space.
pub(crate) const XSD_MAXINCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#maxInclusive";
/// `xsd:minExclusive` — the exclusive lower bound of an ordered value space.
pub(crate) const XSD_MINEXCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#minExclusive";
/// `xsd:maxExclusive` — the exclusive upper bound of an ordered value space.
pub(crate) const XSD_MAXEXCLUSIVE: &str = "http://www.w3.org/2001/XMLSchema#maxExclusive";
/// `xsd:length` — the exact length of a string or binary value.
pub(crate) const XSD_LENGTH: &str = "http://www.w3.org/2001/XMLSchema#length";
/// `xsd:minLength` — the minimum length of a string or binary value.
pub(crate) const XSD_MINLENGTH: &str = "http://www.w3.org/2001/XMLSchema#minLength";
/// `xsd:maxLength` — the maximum length of a string or binary value.
pub(crate) const XSD_MAXLENGTH: &str = "http://www.w3.org/2001/XMLSchema#maxLength";
/// `xsd:pattern` — a regular-expression facet over a lexical space.
pub(crate) const XSD_PATTERN: &str = "http://www.w3.org/2001/XMLSchema#pattern";
/// `rdf:langRange` — a language-range facet over `rdf:langString`.
pub(crate) const RDF_LANGRANGE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langRange";

// --- OWL 2 ontology header and axiom/annotation reification. ---

/// `owl:imports` — the ontology-document import that fixes the imports closure.
pub(crate) const OWL_IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";
/// `owl:versionIRI` — the ontology's version identity.
pub(crate) const OWL_VERSIONIRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
/// `owl:OntologyProperty` — the class of ontology-header properties.
pub(crate) const OWL_ONTOLOGYPROPERTY: &str = "http://www.w3.org/2002/07/owl#OntologyProperty";
/// `owl:Axiom` — the class of a reified (annotated) axiom.
pub(crate) const OWL_AXIOM: &str = "http://www.w3.org/2002/07/owl#Axiom";
/// `owl:Annotation` — the class of a reified (annotated) annotation.
pub(crate) const OWL_ANNOTATION: &str = "http://www.w3.org/2002/07/owl#Annotation";
/// `owl:annotatedSource` — the subject of a reified axiom or annotation.
pub(crate) const OWL_ANNOTATEDSOURCE: &str = "http://www.w3.org/2002/07/owl#annotatedSource";
/// `owl:annotatedProperty` — the predicate of a reified axiom or annotation.
pub(crate) const OWL_ANNOTATEDPROPERTY: &str = "http://www.w3.org/2002/07/owl#annotatedProperty";
/// `owl:annotatedTarget` — the object of a reified axiom or annotation.
pub(crate) const OWL_ANNOTATEDTARGET: &str = "http://www.w3.org/2002/07/owl#annotatedTarget";

/// `owl:differentFrom` — the negation of `owl:sameAs`, which `eq-diff1` clashes against
/// and `dt-diff` concludes.
pub(crate) const OWL_DIFFERENTFROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
/// `owl:AllDifferent` — the class whose `owl:members` / `owl:distinctMembers` list
/// `eq-diff2` and `eq-diff3` read.
pub(crate) const OWL_ALLDIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";

/// `xsd:nonNegativeInteger` — the datatype OWL 2 Profiles §4.3 Table 6 writes the
/// cardinality literals of `cls-maxc1`, `cls-maxc2` and the four `cls-maxqc*` rules with.
pub(crate) const XSD_NONNEGATIVEINTEGER: &str =
    "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

/// `rdf:PlainLiteral` — a datatype supported in OWL 2 RL.
pub(crate) const RDF_PLAINLITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";
/// `rdf:XMLLiteral` — a datatype supported in OWL 2 RL.
pub(crate) const RDF_XMLLITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";
/// `xsd:decimal` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
/// `xsd:integer` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// `xsd:nonPositiveInteger` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_NONPOSITIVEINTEGER: &str =
    "http://www.w3.org/2001/XMLSchema#nonPositiveInteger";
/// `xsd:positiveInteger` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_POSITIVEINTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";
/// `xsd:negativeInteger` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_NEGATIVEINTEGER: &str = "http://www.w3.org/2001/XMLSchema#negativeInteger";
/// `xsd:long` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_LONG: &str = "http://www.w3.org/2001/XMLSchema#long";
/// `xsd:int` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#int";
/// `xsd:short` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_SHORT: &str = "http://www.w3.org/2001/XMLSchema#short";
/// `xsd:byte` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";
/// `xsd:unsignedLong` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_UNSIGNEDLONG: &str = "http://www.w3.org/2001/XMLSchema#unsignedLong";
/// `xsd:unsignedInt` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_UNSIGNEDINT: &str = "http://www.w3.org/2001/XMLSchema#unsignedInt";
/// `xsd:unsignedShort` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_UNSIGNEDSHORT: &str = "http://www.w3.org/2001/XMLSchema#unsignedShort";
/// `xsd:unsignedByte` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_UNSIGNEDBYTE: &str = "http://www.w3.org/2001/XMLSchema#unsignedByte";
/// `xsd:float` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
/// `xsd:double` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
/// `xsd:normalizedString` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_NORMALIZEDSTRING: &str = "http://www.w3.org/2001/XMLSchema#normalizedString";
/// `xsd:token` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_TOKEN: &str = "http://www.w3.org/2001/XMLSchema#token";
/// `xsd:language` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_LANGUAGE: &str = "http://www.w3.org/2001/XMLSchema#language";
/// `xsd:Name` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_NAME: &str = "http://www.w3.org/2001/XMLSchema#Name";
/// `xsd:NCName` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_NCNAME: &str = "http://www.w3.org/2001/XMLSchema#NCName";
/// `xsd:NMTOKEN` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_NMTOKEN: &str = "http://www.w3.org/2001/XMLSchema#NMTOKEN";
/// `xsd:boolean` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
/// `xsd:hexBinary` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_HEXBINARY: &str = "http://www.w3.org/2001/XMLSchema#hexBinary";
/// `xsd:base64Binary` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_BASE64BINARY: &str = "http://www.w3.org/2001/XMLSchema#base64Binary";
/// `xsd:anyURI` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_ANYURI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";
/// `xsd:dateTime` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
/// `xsd:dateTimeStamp` — a datatype supported in OWL 2 RL.
pub(crate) const XSD_DATETIMESTAMP: &str = "http://www.w3.org/2001/XMLSchema#dateTimeStamp";
