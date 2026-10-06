// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The W3C vocabularies the workspace names, one module per namespace.
//!
//! Every constant is a term IRI defined by the W3C Recommendation its module
//! cites, transcribed from that specification's namespace document. PurRDF mints
//! no vocabulary of its own: a term that is not in a W3C namespace — an
//! application ontology, schema.org, Dublin Core — is caller-supplied
//! configuration and never a constant here. The XML Schema datatypes live with
//! their value space, in `purrdf_xsd::datatype`.
//!
//! Each module's `NS` is its namespace IRI and every term in it starts with that
//! IRI, so a term constant and the namespace concatenated with its local name are
//! the same string. Code names a term through its constant and never spells the
//! IRI out a second time; the workspace's helper census holds that line.
//!
//! Where a namespace defines two terms whose names differ only in case (a class
//! and a property, `sh:Rule` and `sh:rule`), the class keeps the plain constant
//! name unless the pair is already published otherwise, and the other carries a
//! `_CLASS` or `_PROPERTY` suffix; the doc comment of each names its term.
//!
//! # Examples
//!
//! ```rust
//! use purrdf_iri::vocab::{language_datatype_iri, rdf, rdfs};
//!
//! assert_eq!(rdf::TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
//! assert_eq!(rdfs::LABEL.strip_prefix(rdfs::NS), Some("label"));
//! assert_eq!(language_datatype_iri(false), rdf::LANG_STRING);
//! assert_eq!(language_datatype_iri(true), rdf::DIR_LANG_STRING);
//! ```

/// The datatype RDF 1.2 gives a language-tagged string: `rdf:dirLangString`
/// when it carries a base direction, otherwise `rdf:langString`.
///
/// RDF 1.2 Concepts §3.3 fixes both: a literal with a language tag and no base
/// direction has datatype `rdf:langString`, and one with a base direction has
/// `rdf:dirLangString`. There is no third case, so every surface that
/// materializes a language-tagged literal's datatype asks this function.
#[must_use]
pub const fn language_datatype_iri(has_direction: bool) -> &'static str {
    if has_direction {
        rdf::DIR_LANG_STRING
    } else {
        rdf::LANG_STRING
    }
}

/// RDF 1.2 Concepts and Abstract Syntax (`http://www.w3.org/1999/02/22-rdf-syntax-ns#`).
///
/// Specification: <https://www.w3.org/TR/rdf12-concepts/>.
pub mod rdf {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

    ///`rdf:_` — the stem of the container membership properties `rdf:_1`,
    /// `rdf:_2`, … (RDF 1.2 Schema §5.1.6): append a decimal index of 1 or more.
    pub const MEMBER_PREFIX: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#_";

    /// `rdf:Alt`.
    pub const ALT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Alt";

    /// `rdf:Bag`.
    pub const BAG: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Bag";

    /// `rdf:dirLangString` — the datatype of a directional language-tagged string,
    /// new in RDF 1.2; RDF 1.2 Semantics §8 requires every interpretation to
    /// recognize it.
    pub const DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

    /// `rdf:first` — the head of an RDF collection cell.
    pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";

    /// `rdf:HTML`.
    pub const HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";

    /// `rdf:JSON`.
    pub const JSON: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON";

    /// `rdf:langRange` — a language-range facet over `rdf:langString`.
    pub const LANG_RANGE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langRange";

    /// `rdf:langString` — a datatype RDF 1.2 Semantics §8 requires every RDF interpretation to
    /// recognize.
    pub const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

    /// `rdf:List`.
    pub const LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";

    /// `rdf:nil` — the empty RDF collection.
    pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

    /// `rdf:object`.
    pub const OBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";

    /// `rdf:PlainLiteral` — a datatype supported in OWL 2 RL.
    pub const PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";

    /// `rdf:predicate`.
    pub const PREDICATE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";

    /// `rdf:Property`.
    pub const PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";

    /// `rdf:reifies` — the RDF 1.2 predicate linking a reifier to the triple term it reifies.
    pub const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

    /// `rdf:rest` — the tail of an RDF collection cell.
    pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";

    /// `rdf:Seq`.
    pub const SEQ: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Seq";

    /// `rdf:Statement`.
    pub const STATEMENT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement";

    /// `rdf:subject`.
    pub const SUBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";

    /// `rdf:type` — the instance-of predicate.
    pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

    /// `rdf:value`.
    pub const VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

    /// `rdf:XMLLiteral` — a datatype supported in OWL 2 RL.
    pub const XML_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";
}

/// RDF 1.2 Schema (`http://www.w3.org/2000/01/rdf-schema#`).
///
/// Specification: <https://www.w3.org/TR/rdf12-schema/>.
pub mod rdfs {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2000/01/rdf-schema#";

    /// `rdfs:Class` — the class of RDFS classes.
    pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";

    /// `rdfs:comment` — a human-readable description of a resource.
    pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";

    /// `rdfs:Container`.
    pub const CONTAINER: &str = "http://www.w3.org/2000/01/rdf-schema#Container";

    /// `rdfs:ContainerMembershipProperty`.
    pub const CONTAINER_MEMBERSHIP_PROPERTY: &str =
        "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty";

    /// `rdfs:Datatype`.
    pub const DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";

    /// `rdfs:domain`.
    pub const DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";

    /// `rdfs:isDefinedBy`.
    pub const IS_DEFINED_BY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";

    /// `rdfs:label` — a human-readable name for a resource.
    pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

    /// `rdfs:Literal`.
    pub const LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";

    /// `rdfs:member`.
    pub const MEMBER: &str = "http://www.w3.org/2000/01/rdf-schema#member";

    /// `rdfs:Proposition` — the class RDF 1.2 Semantics §9.2.1's `rdfs14` / `rdfs14a` type a
    /// triple term's surrogate blank node with.
    pub const PROPOSITION: &str = "http://www.w3.org/2000/01/rdf-schema#Proposition";

    /// `rdfs:range` — the range predicate (a property's values are instances of
    /// the range class).
    pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";

    /// `rdfs:Resource`.
    pub const RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";

    /// `rdfs:seeAlso`.
    pub const SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";

    /// `rdfs:subClassOf` — the RDFS subclass predicate.
    pub const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    /// `rdfs:subPropertyOf`.
    pub const SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
}

/// OWL 2 Web Ontology Language (`http://www.w3.org/2002/07/owl#`).
///
/// Specification: <https://www.w3.org/TR/owl2-syntax/>.
///
/// The two datatypes OWL 2 adds to the XSD datatype map, `owl:real` and
/// `owl:rational`, live with their value space in `purrdf_xsd::datatype`.
pub mod owl {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2002/07/owl#";

    /// `owl:AllDifferent` — the class whose `owl:members` / `owl:distinctMembers` list
    /// `eq-diff2` and `eq-diff3` read.
    pub const ALL_DIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";

    /// `owl:AllDisjointClasses`.
    pub const ALL_DISJOINT_CLASSES: &str = "http://www.w3.org/2002/07/owl#AllDisjointClasses";

    /// `owl:AllDisjointProperties`.
    pub const ALL_DISJOINT_PROPERTIES: &str = "http://www.w3.org/2002/07/owl#AllDisjointProperties";

    /// `owl:allValuesFrom`.
    pub const ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";

    /// `owl:annotatedProperty` — the predicate of a reified axiom or annotation.
    pub const ANNOTATED_PROPERTY: &str = "http://www.w3.org/2002/07/owl#annotatedProperty";

    /// `owl:annotatedSource` — the subject of a reified axiom or annotation.
    pub const ANNOTATED_SOURCE: &str = "http://www.w3.org/2002/07/owl#annotatedSource";

    /// `owl:annotatedTarget` — the object of a reified axiom or annotation.
    pub const ANNOTATED_TARGET: &str = "http://www.w3.org/2002/07/owl#annotatedTarget";

    /// `owl:Annotation` — the class of a reified (annotated) annotation.
    pub const ANNOTATION: &str = "http://www.w3.org/2002/07/owl#Annotation";

    /// `owl:AnnotationProperty` — the class `prp-ap` types the built-in annotation properties
    /// with.
    pub const ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";

    /// `owl:assertionProperty` — a negative property assertion's predicate.
    pub const ASSERTION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#assertionProperty";

    /// `owl:AsymmetricProperty`.
    pub const ASYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";

    /// `owl:Axiom` — the class of a reified (annotated) axiom.
    pub const AXIOM: &str = "http://www.w3.org/2002/07/owl#Axiom";

    /// `owl:backwardCompatibleWith` — a built-in annotation property.
    pub const BACKWARD_COMPATIBLE_WITH: &str =
        "http://www.w3.org/2002/07/owl#backwardCompatibleWith";

    /// `owl:bottomDataProperty` — the empty data role.
    pub const BOTTOM_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomDataProperty";

    /// `owl:bottomObjectProperty` — the empty object role.
    pub const BOTTOM_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomObjectProperty";

    /// `owl:cardinality`.
    pub const CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";

    /// `owl:Class`.
    pub const CLASS: &str = "http://www.w3.org/2002/07/owl#Class";

    /// `owl:complementOf`.
    pub const COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";

    /// `owl:DataRange` — OWL 1's deprecated spelling of `rdfs:Datatype`.
    pub const DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#DataRange";

    /// `owl:datatypeComplementOf` — the complement of a data range.
    pub const DATATYPE_COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#datatypeComplementOf";

    /// `owl:DatatypeProperty`.
    pub const DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";

    /// `owl:deprecated` — a built-in annotation property.
    pub const DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";

    /// `owl:differentFrom` — the negation of `owl:sameAs`, which `eq-diff1` clashes against
    /// and `dt-diff` concludes.
    pub const DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";

    /// `owl:disjointUnionOf` — `C ≡ C₁ ⊔ … ⊔ Cₙ` with the `Cᵢ` pairwise disjoint.
    pub const DISJOINT_UNION_OF: &str = "http://www.w3.org/2002/07/owl#disjointUnionOf";

    /// `owl:disjointWith`.
    pub const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";

    /// `owl:distinctMembers` — `owl:AllDifferent`'s other list-valued property.
    pub const DISTINCT_MEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";

    /// `owl:equivalentClass`.
    pub const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";

    /// `owl:equivalentProperty`.
    pub const EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";

    /// `owl:FunctionalProperty`.
    pub const FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";

    /// `owl:hasKey`.
    pub const HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";

    /// `owl:hasSelf` — the local reflexivity restriction `∃r.Self`.
    pub const HAS_SELF: &str = "http://www.w3.org/2002/07/owl#hasSelf";

    /// `owl:hasValue`.
    pub const HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";

    /// `owl:imports` — the ontology-document import that fixes the imports closure.
    pub const IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";

    /// `owl:incompatibleWith` — a built-in annotation property.
    pub const INCOMPATIBLE_WITH: &str = "http://www.w3.org/2002/07/owl#incompatibleWith";

    /// `owl:intersectionOf`.
    pub const INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";

    /// `owl:InverseFunctionalProperty`.
    pub const INVERSE_FUNCTIONAL_PROPERTY: &str =
        "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";

    /// `owl:inverseOf`.
    pub const INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";

    /// `owl:IrreflexiveProperty`.
    pub const IRREFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";

    /// `owl:maxCardinality`.
    pub const MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";

    /// `owl:maxQualifiedCardinality`.
    pub const MAX_QUALIFIED_CARDINALITY: &str =
        "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";

    /// `owl:members` — the list-valued property of `owl:AllDisjoint*` and `owl:AllDifferent`.
    pub const MEMBERS: &str = "http://www.w3.org/2002/07/owl#members";

    /// `owl:minCardinality`.
    pub const MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";

    /// `owl:minQualifiedCardinality`.
    pub const MIN_QUALIFIED_CARDINALITY: &str =
        "http://www.w3.org/2002/07/owl#minQualifiedCardinality";

    /// `owl:NamedIndividual`.
    pub const NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";

    /// `owl:NegativePropertyAssertion` — the reified `¬p(s, o)` axiom's class.
    pub const NEGATIVE_PROPERTY_ASSERTION: &str =
        "http://www.w3.org/2002/07/owl#NegativePropertyAssertion";

    /// `owl:Nothing` — the bottom concept ⊥.
    pub const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";

    /// `owl:ObjectProperty`.
    pub const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";

    /// `owl:onClass`.
    pub const ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";

    /// `owl:onDataRange` — the filler of a qualified DATA cardinality restriction.
    pub const ON_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";

    /// `owl:onDatatype` — the base datatype of a datatype restriction.
    pub const ON_DATATYPE: &str = "http://www.w3.org/2002/07/owl#onDatatype";

    /// `owl:oneOf`.
    pub const ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";

    /// `owl:onProperties` — the property list of an n-ary data restriction.
    pub const ON_PROPERTIES: &str = "http://www.w3.org/2002/07/owl#onProperties";

    /// `owl:onProperty`.
    pub const ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";

    /// `owl:Ontology`.
    pub const ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";

    /// `owl:OntologyProperty` — the class of ontology-header properties.
    pub const ONTOLOGY_PROPERTY: &str = "http://www.w3.org/2002/07/owl#OntologyProperty";

    /// `owl:priorVersion` — a built-in annotation property.
    pub const PRIOR_VERSION: &str = "http://www.w3.org/2002/07/owl#priorVersion";

    /// `owl:propertyChainAxiom`.
    pub const PROPERTY_CHAIN_AXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";

    /// `owl:propertyDisjointWith`.
    pub const PROPERTY_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#propertyDisjointWith";

    /// `owl:qualifiedCardinality`.
    pub const QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#qualifiedCardinality";

    /// `owl:ReflexiveProperty` — the global role axiom `⊤ ⊑ ∃r.Self`.
    pub const REFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";

    /// `owl:Restriction`.
    pub const RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";

    /// `owl:sameAs`.
    pub const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";

    /// `owl:someValuesFrom`.
    pub const SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";

    /// `owl:sourceIndividual` — a negative property assertion's subject.
    pub const SOURCE_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#sourceIndividual";

    /// `owl:SymmetricProperty`.
    pub const SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";

    /// `owl:targetIndividual` — a negative OBJECT-property assertion's object.
    pub const TARGET_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#targetIndividual";

    /// `owl:targetValue` — a negative DATA-property assertion's object.
    pub const TARGET_VALUE: &str = "http://www.w3.org/2002/07/owl#targetValue";

    /// `owl:Thing` — the top concept ⊤.
    pub const THING: &str = "http://www.w3.org/2002/07/owl#Thing";

    /// `owl:topDataProperty` — the universal data role.
    pub const TOP_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#topDataProperty";

    /// `owl:topObjectProperty` — the universal object role.
    pub const TOP_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#topObjectProperty";

    /// `owl:TransitiveProperty`.
    pub const TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";

    /// `owl:unionOf`.
    pub const UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";

    /// `owl:versionInfo` — a built-in annotation property.
    pub const VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";

    /// `owl:versionIRI` — the ontology's version identity.
    pub const VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";

    /// `owl:withRestrictions` — the facet list of a datatype restriction.
    pub const WITH_RESTRICTIONS: &str = "http://www.w3.org/2002/07/owl#withRestrictions";
}

/// Shapes Constraint Language (SHACL) (`http://www.w3.org/ns/shacl#`).
///
/// Specification: <https://www.w3.org/TR/shacl12-core/>.
pub mod sh {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/shacl#";

    /// `sh:alternativePath` — an alternative path over a SHACL list of member paths.
    pub const ALTERNATIVE_PATH: &str = "http://www.w3.org/ns/shacl#alternativePath";

    /// `sh:and` — value nodes must conform to every shape in the given list.
    pub const AND: &str = "http://www.w3.org/ns/shacl#and";

    /// `sh:AndConstraintComponent` — the component reported for `sh:and` violations.
    pub const AND_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#AndConstraintComponent";

    /// `sh:annotationProperty` — the property a result annotation sets.
    pub const ANNOTATION_PROPERTY: &str = "http://www.w3.org/ns/shacl#annotationProperty";

    /// `sh:annotationValue` — a result annotation's default values.
    pub const ANNOTATION_VALUE: &str = "http://www.w3.org/ns/shacl#annotationValue";

    /// `sh:annotationVarName` — the SPARQL variable a result annotation reads.
    pub const ANNOTATION_VAR_NAME: &str = "http://www.w3.org/ns/shacl#annotationVarName";

    /// `sh:ask` — the ASK query of a SHACL-SPARQL validator.
    pub const ASK: &str = "http://www.w3.org/ns/shacl#ask";

    /// `sh:BlankNode` — node kind: blank nodes only.
    pub const BLANK_NODE: &str = "http://www.w3.org/ns/shacl#BlankNode";

    /// `sh:BlankNodeOrIRI` — node kind: blank nodes or IRIs.
    pub const BLANK_NODE_OR_IRI: &str = "http://www.w3.org/ns/shacl#BlankNodeOrIRI";

    /// `sh:BlankNodeOrLiteral` — node kind: blank nodes or literals.
    pub const BLANK_NODE_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#BlankNodeOrLiteral";

    /// `sh:bodyExpression` — the node expression that IS a custom function's body
    /// (SHACL 1.2 Node Expressions §6.1/§6.2; SHACL 1.2 SPARQL Extensions §7).
    /// Exactly one value is required on a declaring node.
    pub const BODY_EXPRESSION: &str = "http://www.w3.org/ns/shacl#bodyExpression";

    /// `sh:ByTypes` — the non-boolean value of `sh:closed` (SHACL 1.2 Core
    /// §7.9.1).
    pub const BY_TYPES: &str = "http://www.w3.org/ns/shacl#ByTypes";

    /// `sh:class` — value nodes must be SHACL instances of the given class.
    pub const CLASS: &str = "http://www.w3.org/ns/shacl#class";

    /// `sh:ClassConstraintComponent` — the component reported for `sh:class` violations.
    pub const CLASS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ClassConstraintComponent";

    /// `sh:closed` — restricts focus nodes to the properties declared by the shape.
    pub const CLOSED: &str = "http://www.w3.org/ns/shacl#closed";

    /// `sh:ClosedConstraintComponent` — the component reported for `sh:closed` violations.
    pub const CLOSED_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ClosedConstraintComponent";

    /// `sh:condition` — a shape a focus node must conform to for a rule to fire.
    pub const CONDITION: &str = "http://www.w3.org/ns/shacl#condition";

    /// `sh:conformanceDisallows` — a severity level the validation report's
    /// conformance-disallow set holds.
    pub const CONFORMANCE_DISALLOWS: &str = "http://www.w3.org/ns/shacl#conformanceDisallows";

    /// `sh:conforms` — whether the data graph conforms (boolean on a `sh:ValidationReport`).
    pub const CONFORMS: &str = "http://www.w3.org/ns/shacl#conforms";

    /// `sh:ConstraintComponent` — the class of constraint components.
    pub const CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#ConstraintComponent";

    /// `sh:construct` — the SPARQL CONSTRUCT query text of a `sh:SPARQLRule`.
    pub const CONSTRUCT: &str = "http://www.w3.org/ns/shacl#construct";

    /// `sh:count` — a count aggregation node expression.
    pub const COUNT: &str = "http://www.w3.org/ns/shacl#count";

    /// `sh:DataGraph`.
    pub const DATA_GRAPH: &str = "http://www.w3.org/ns/shacl#DataGraph";

    /// `sh:datatype` — value nodes must be literals of the given datatype.
    pub const DATATYPE: &str = "http://www.w3.org/ns/shacl#datatype";

    /// `sh:DatatypeConstraintComponent` — the component reported for `sh:datatype` violations.
    pub const DATATYPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#DatatypeConstraintComponent";

    /// `sh:deactivated` — a `true` value disables the shape entirely.
    pub const DEACTIVATED: &str = "http://www.w3.org/ns/shacl#deactivated";

    /// `sh:Debug` — "a debug message that is not a constraint violation".
    pub const DEBUG: &str = "http://www.w3.org/ns/shacl#Debug";

    /// `sh:declare` — attaches a prefix declaration to a prefix-owning node.
    pub const DECLARE: &str = "http://www.w3.org/ns/shacl#declare";

    /// `sh:defaultValue` — the node expression computing a property shape's
    /// value nodes when no other value exists (SHACL 1.2 Core §2.3), and a
    /// parameter declaration's documented default.
    pub const DEFAULT_VALUE: &str = "http://www.w3.org/ns/shacl#defaultValue";

    /// The adopted PurRDF DASH-extension direction flag for `sh:orderby`
    /// (boolean; `true` ⇒ descending, default ascending).
    pub const DESC: &str = "http://www.w3.org/ns/shacl#desc";

    /// `sh:describe` — the DESCRIBE query of a `sh:SPARQLDescribeExecutable`.
    pub const DESCRIBE: &str = "http://www.w3.org/ns/shacl#describe";

    /// `sh:description` — a human-readable shape description (non-validating).
    pub const DESCRIPTION: &str = "http://www.w3.org/ns/shacl#description";

    /// `sh:detail` — the nested results of a validation result (SHACL 1.2 Core
    /// §3.6.2.7).
    pub const DETAIL: &str = "http://www.w3.org/ns/shacl#detail";

    /// `sh:disjoint` — the value set must be disjoint with the sibling property's value set.
    pub const DISJOINT: &str = "http://www.w3.org/ns/shacl#disjoint";

    /// `sh:DisjointConstraintComponent` — the component reported for `sh:disjoint` violations.
    pub const DISJOINT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#DisjointConstraintComponent";

    /// `sh:distinct` — a distinct node expression (deduplicates its input).
    pub const DISTINCT: &str = "http://www.w3.org/ns/shacl#distinct";

    /// `sh:else` — the else-branch of an if/then/else node expression.
    pub const ELSE: &str = "http://www.w3.org/ns/shacl#else";

    /// `sh:entailment` — an entailment regime a shapes graph requires.
    pub const ENTAILMENT: &str = "http://www.w3.org/ns/shacl#entailment";

    /// `sh:equals` — the value set must equal the sibling property's value set.
    pub const EQUALS: &str = "http://www.w3.org/ns/shacl#equals";

    /// `sh:EqualsConstraintComponent` — the component reported for `sh:equals` violations.
    pub const EQUALS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#EqualsConstraintComponent";

    /// `sh:exists` — a boolean existence node expression.
    pub const EXISTS: &str = "http://www.w3.org/ns/shacl#exists";

    /// `sh:expectedPredicate` — a predicate whose `sh:values` / `sh:defaultValue`
    /// derived triples a rule expects to be present while it executes (SHACL 1.2
    /// Inference Rules §3.8).
    pub const EXPECTED_PREDICATE: &str = "http://www.w3.org/ns/shacl#expectedPredicate";

    /// `sh:expression` — attaches a node-expression constraint to a shape.
    pub const EXPRESSION: &str = "http://www.w3.org/ns/shacl#expression";

    /// `sh:ExpressionConstraintComponent` — the constraint component reported for failed `sh:expression` constraints.
    pub const EXPRESSION_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ExpressionConstraintComponent";

    /// `sh:filterShape` — the filter shape of a filter-shape node expression.
    pub const FILTER_SHAPE: &str = "http://www.w3.org/ns/shacl#filterShape";

    /// `sh:flags` — regex flags accompanying `sh:pattern`.
    pub const FLAGS: &str = "http://www.w3.org/ns/shacl#flags";

    /// `sh:focusNode` — the focus node a validation result is about.
    pub const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl#focusNode";

    /// `sh:Function` — the class of SHACL-AF functions.
    pub const FUNCTION: &str = "http://www.w3.org/ns/shacl#Function";

    /// `sh:group` — groups related property shapes (non-validating).
    pub const GROUP: &str = "http://www.w3.org/ns/shacl#group";

    /// `sh:hasRule` — a rule set's member rule.
    pub const HAS_RULE: &str = "http://www.w3.org/ns/shacl#hasRule";

    /// `sh:hasValue` — at least one value node must equal the given term.
    pub const HAS_VALUE: &str = "http://www.w3.org/ns/shacl#hasValue";

    /// `sh:HasValueConstraintComponent` — the component reported for `sh:hasValue` violations.
    pub const HAS_VALUE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#HasValueConstraintComponent";

    /// `sh:if` — the condition of an if/then/else node expression.
    pub const IF: &str = "http://www.w3.org/ns/shacl#if";

    /// `sh:ignoredProperties` — predicates exempt from `sh:closed` checking.
    pub const IGNORED_PROPERTIES: &str = "http://www.w3.org/ns/shacl#ignoredProperties";

    /// `sh:in` — value nodes must be members of the given SHACL list.
    pub const IN: &str = "http://www.w3.org/ns/shacl#in";

    /// `sh:includesRuleSet` — a rule set a rule set includes.
    pub const INCLUDES_RULE_SET: &str = "http://www.w3.org/ns/shacl#includesRuleSet";

    /// `sh:InConstraintComponent` — the component reported for `sh:in` violations.
    pub const IN_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#InConstraintComponent";

    /// `sh:Info` — the informational result severity.
    pub const INFO: &str = "http://www.w3.org/ns/shacl#Info";

    /// `sh:intersection` — an intersection node expression over a list of member expressions.
    pub const INTERSECTION: &str = "http://www.w3.org/ns/shacl#intersection";

    /// `sh:inversePath` — an inverse property path.
    pub const INVERSE_PATH: &str = "http://www.w3.org/ns/shacl#inversePath";

    /// `sh:IRI` — node kind: IRIs only.
    pub const IRI: &str = "http://www.w3.org/ns/shacl#IRI";

    /// `sh:IRIOrLiteral` — node kind: IRIs or literals.
    pub const IRI_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#IRIOrLiteral";

    /// `sh:JSFunction`.
    pub const JS_FUNCTION: &str = "http://www.w3.org/ns/shacl#JSFunction";

    /// `sh:JSRule`.
    pub const JS_RULE: &str = "http://www.w3.org/ns/shacl#JSRule";

    /// `sh:JSTarget`.
    pub const JS_TARGET: &str = "http://www.w3.org/ns/shacl#JSTarget";

    /// `sh:JSTargetType`.
    pub const JS_TARGET_TYPE: &str = "http://www.w3.org/ns/shacl#JSTargetType";

    /// `sh:JSValidator`.
    pub const JS_VALIDATOR: &str = "http://www.w3.org/ns/shacl#JSValidator";

    /// `sh:keyParameter` — marks a parameter as the KEY under which a custom named
    /// parameter function is recognised at a call site (SHACL 1.2 Node Expressions
    /// §6.1). At least one parameter of such a function must carry `true`, and key
    /// parameters must be disjoint across functions.
    pub const KEY_PARAMETER: &str = "http://www.w3.org/ns/shacl#keyParameter";

    /// `sh:labelTemplate` — how a constraint of a component could be rendered to
    /// humans (SHACL 1.2 SPARQL Extensions, "Label Templates").
    pub const LABEL_TEMPLATE: &str = "http://www.w3.org/ns/shacl#labelTemplate";

    /// `sh:languageIn` — value-node language tags must be in the given list.
    pub const LANGUAGE_IN: &str = "http://www.w3.org/ns/shacl#languageIn";

    /// `sh:LanguageInConstraintComponent` — the component reported for `sh:languageIn` violations.
    pub const LANGUAGE_IN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LanguageInConstraintComponent";

    /// `sh:layer` — the numeric layer a rule executes in.
    pub const LAYER: &str = "http://www.w3.org/ns/shacl#layer";

    /// `sh:lessThan` — value nodes must compare less than the sibling property's values.
    pub const LESS_THAN: &str = "http://www.w3.org/ns/shacl#lessThan";

    /// `sh:LessThanConstraintComponent` — the component reported for `sh:lessThan` violations.
    pub const LESS_THAN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LessThanConstraintComponent";

    /// `sh:lessThanOrEquals` — value nodes must compare less than or equal to the sibling property's values.
    pub const LESS_THAN_OR_EQUALS: &str = "http://www.w3.org/ns/shacl#lessThanOrEquals";

    /// `sh:LessThanOrEqualsConstraintComponent` — the component reported for `sh:lessThanOrEquals` violations.
    pub const LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent";

    /// `sh:limit` — a limit node expression (truncates its ordered input).
    pub const LIMIT: &str = "http://www.w3.org/ns/shacl#limit";

    /// `sh:ListParameterExpression` — the class custom list parameter functions
    /// are declared SHACL subclasses of (SHACL 1.2 Node Expressions §6.2).
    pub const LIST_PARAMETER_EXPRESSION: &str =
        "http://www.w3.org/ns/shacl#ListParameterExpression";

    /// `sh:ListParameterExpressionFunction` — the class whose SHACL instances are
    /// custom LIST parameter functions: the function's own IRI is its list
    /// parameter property and its body reads arguments by INDEX
    /// (SHACL 1.2 Node Expressions §6.2). SHACL 1.2 SPARQL Extensions §7.3 asks a
    /// SPARQL engine to register a function for every instance of this class.
    pub const LIST_PARAMETER_EXPRESSION_FUNCTION: &str =
        "http://www.w3.org/ns/shacl#ListParameterExpressionFunction";

    /// `sh:Literal` — node kind: literals only.
    pub const LITERAL: &str = "http://www.w3.org/ns/shacl#Literal";

    /// `sh:max` — a maximum aggregation node expression.
    pub const MAX: &str = "http://www.w3.org/ns/shacl#max";

    /// `sh:maxCount` — the maximum number of value nodes.
    pub const MAX_COUNT: &str = "http://www.w3.org/ns/shacl#maxCount";

    /// `sh:MaxCountConstraintComponent` — the component reported for `sh:maxCount` violations.
    pub const MAX_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxCountConstraintComponent";

    /// `sh:maxExclusive` — exclusive upper bound on literal value nodes.
    pub const MAX_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxExclusive";

    /// `sh:MaxExclusiveConstraintComponent` — the component reported for `sh:maxExclusive` violations.
    pub const MAX_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent";

    /// `sh:maxInclusive` — inclusive upper bound on literal value nodes.
    pub const MAX_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxInclusive";

    /// `sh:MaxInclusiveConstraintComponent` — the component reported for `sh:maxInclusive` violations.
    pub const MAX_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent";

    /// `sh:maxLength` — the maximum string length of value nodes.
    pub const MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#maxLength";

    /// `sh:MaxLengthConstraintComponent` — the component reported for `sh:maxLength` violations.
    pub const MAX_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent";

    /// `sh:maxListLength` — the parameter of `sh:MaxListLengthConstraintComponent`.
    pub const MAX_LIST_LENGTH: &str = "http://www.w3.org/ns/shacl#maxListLength";

    /// `sh:MaxListLengthConstraintComponent`.
    pub const MAX_LIST_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxListLengthConstraintComponent";

    /// `sh:memberShape` — the parameter of `sh:MemberShapeConstraintComponent`.
    pub const MEMBER_SHAPE: &str = "http://www.w3.org/ns/shacl#memberShape";

    /// `sh:MemberShapeConstraintComponent`.
    pub const MEMBER_SHAPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MemberShapeConstraintComponent";

    /// `sh:message` — a human-readable message copied onto results produced by a shape.
    pub const MESSAGE: &str = "http://www.w3.org/ns/shacl#message";

    /// `sh:min` — a minimum aggregation node expression.
    pub const MIN: &str = "http://www.w3.org/ns/shacl#min";

    /// `sh:minCount` — the minimum number of value nodes.
    pub const MIN_COUNT: &str = "http://www.w3.org/ns/shacl#minCount";

    /// `sh:MinCountConstraintComponent` — the component reported for `sh:minCount` violations.
    pub const MIN_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinCountConstraintComponent";

    /// `sh:minExclusive` — exclusive lower bound on literal value nodes.
    pub const MIN_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#minExclusive";

    /// `sh:MinExclusiveConstraintComponent` — the component reported for `sh:minExclusive` violations.
    pub const MIN_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent";

    /// `sh:minInclusive` — inclusive lower bound on literal value nodes.
    pub const MIN_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#minInclusive";

    /// `sh:MinInclusiveConstraintComponent` — the component reported for `sh:minInclusive` violations.
    pub const MIN_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent";

    /// `sh:minLength` — the minimum string length of value nodes.
    pub const MIN_LENGTH: &str = "http://www.w3.org/ns/shacl#minLength";

    /// `sh:MinLengthConstraintComponent` — the component reported for `sh:minLength` violations.
    pub const MIN_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinLengthConstraintComponent";

    /// `sh:minListLength` — the parameter of `sh:MinListLengthConstraintComponent`.
    pub const MIN_LIST_LENGTH: &str = "http://www.w3.org/ns/shacl#minListLength";

    /// `sh:MinListLengthConstraintComponent`.
    pub const MIN_LIST_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinListLengthConstraintComponent";

    /// `sh:minus` — the removed-nodes operand of a SHACL Advanced Features 1.1
    /// minus expression, whose input is its `sh:nodes`; SHACL 1.2 Node Expressions
    /// spells the same expression `shnex:remove`.
    pub const MINUS: &str = "http://www.w3.org/ns/shacl#minus";

    /// `sh:name` — a human-readable shape name (non-validating).
    pub const NAME: &str = "http://www.w3.org/ns/shacl#name";

    /// `sh:NamedParameterExpression` — the class custom named parameter functions
    /// are declared SHACL subclasses of (SHACL 1.2 Node Expressions §6.1).
    pub const NAMED_PARAMETER_EXPRESSION: &str =
        "http://www.w3.org/ns/shacl#NamedParameterExpression";

    /// `sh:NamedParameterExpressionFunction` — the class whose SHACL instances are
    /// custom NAMED parameter functions: arguments are supplied under the
    /// parameters' own `sh:path` IRIs (SHACL 1.2 Node Expressions §6.1).
    pub const NAMED_PARAMETER_EXPRESSION_FUNCTION: &str =
        "http://www.w3.org/ns/shacl#NamedParameterExpressionFunction";

    /// `sh:namespace` — the namespace IRI of a declaration.
    pub const NAMESPACE: &str = "http://www.w3.org/ns/shacl#namespace";

    /// `sh:node` — value nodes must conform to the given node shape.
    pub const NODE: &str = "http://www.w3.org/ns/shacl#node";

    /// `sh:nodeByExpression` — the node shapes every value node must conform to,
    /// computed by a node expression (SHACL 1.2 Node Expressions §7.2).
    pub const NODE_BY_EXPRESSION: &str = "http://www.w3.org/ns/shacl#nodeByExpression";

    /// `sh:NodeByExpressionConstraintComponent` — the constraint component IRI
    /// reported for a failed `sh:nodeByExpression` constraint (SHACL 1.2 Node
    /// Expressions §7.2).
    pub const NODE_BY_EXPRESSION_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeByExpressionConstraintComponent";

    /// `sh:NodeConstraintComponent` — the component reported for `sh:node` violations.
    pub const NODE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeConstraintComponent";

    /// `sh:NodeExpressionFunction` — the class of node-expression functions that
    /// are neither list- nor named-parameter functions (`shnex:EmptyExpression`).
    pub const NODE_EXPRESSION_FUNCTION: &str = "http://www.w3.org/ns/shacl#NodeExpressionFunction";

    /// `sh:nodeKind` — value nodes must match the given node kind.
    pub const NODE_KIND: &str = "http://www.w3.org/ns/shacl#nodeKind";

    /// `sh:NodeKindConstraintComponent` — the component reported for `sh:nodeKind` violations.
    pub const NODE_KIND_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeKindConstraintComponent";

    /// `sh:nodes` — the input-nodes expression of a filter, path, or aggregate node expression.
    pub const NODES: &str = "http://www.w3.org/ns/shacl#nodes";

    /// `sh:NodeShape` — the class of node shapes.
    pub const NODE_SHAPE: &str = "http://www.w3.org/ns/shacl#NodeShape";

    /// `sh:nodeValidator` — the validator a component uses on node shapes.
    pub const NODE_VALIDATOR: &str = "http://www.w3.org/ns/shacl#nodeValidator";

    /// `sh:not` — value nodes must not conform to the given shape.
    pub const NOT: &str = "http://www.w3.org/ns/shacl#not";

    /// `sh:NotConstraintComponent` — the component reported for `sh:not` violations.
    pub const NOT_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#NotConstraintComponent";

    /// `sh:object` — the object node expression of a `sh:TripleRule`.
    pub const OBJECT: &str = "http://www.w3.org/ns/shacl#object";

    /// `sh:offset` — an offset node expression (skips a prefix of its ordered input).
    pub const OFFSET: &str = "http://www.w3.org/ns/shacl#offset";

    /// `sh:oneOrMorePath` — a one-or-more (`+`) property path.
    pub const ONE_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#oneOrMorePath";

    /// `sh:optional` — marks a constraint-component parameter as optional.
    pub const OPTIONAL: &str = "http://www.w3.org/ns/shacl#optional";

    /// `sh:or` — value nodes must conform to at least one shape in the given list.
    pub const OR: &str = "http://www.w3.org/ns/shacl#or";

    /// `sh:OrConstraintComponent` — the component reported for `sh:or` violations.
    pub const OR_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#OrConstraintComponent";

    /// `sh:order` — a numeric ordering hint; also the execution order of SHACL-AF rules.
    pub const ORDER: &str = "http://www.w3.org/ns/shacl#order";

    /// `sh:orderby` — the sort-key expression of an order-by node expression.
    pub const ORDERBY: &str = "http://www.w3.org/ns/shacl#orderby";

    /// `sh:Parameter` — the class of constraint-component parameters.
    pub const PARAMETER: &str = "http://www.w3.org/ns/shacl#Parameter";

    /// `sh:parameter` — attaches a parameter declaration to a constraint component.
    pub const PARAMETER_PROPERTY: &str = "http://www.w3.org/ns/shacl#parameter";

    /// `sh:path` — the property path of a property shape.
    pub const PATH: &str = "http://www.w3.org/ns/shacl#path";

    /// `sh:pattern` — the string form of each value node must match the given regex.
    pub const PATTERN: &str = "http://www.w3.org/ns/shacl#pattern";

    /// `sh:PatternConstraintComponent` — the component reported for `sh:pattern` violations.
    pub const PATTERN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#PatternConstraintComponent";

    /// `sh:predicate` — an alternative to `sh:path` naming a parameter's predicate
    /// (its local name is the pre-bound SPARQL variable).
    pub const PREDICATE: &str = "http://www.w3.org/ns/shacl#predicate";

    /// `sh:prefix` — the prefix label of a declaration.
    pub const PREFIX: &str = "http://www.w3.org/ns/shacl#prefix";

    /// `sh:prefixes` — links a SPARQL-bearing node to its prefix declarations.
    pub const PREFIXES: &str = "http://www.w3.org/ns/shacl#prefixes";

    /// `sh:property` — attaches a property shape to a shape.
    pub const PROPERTY: &str = "http://www.w3.org/ns/shacl#property";

    /// `sh:PropertyConstraintComponent` — the component behind `sh:property`,
    /// whose argument is carried as a property shape rather than a constraint.
    pub const PROPERTY_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#PropertyConstraintComponent";

    /// `sh:PropertyShape` — the class of property shapes.
    pub const PROPERTY_SHAPE: &str = "http://www.w3.org/ns/shacl#PropertyShape";

    /// `sh:propertyValidator` — the validator a component uses on property shapes.
    pub const PROPERTY_VALIDATOR: &str = "http://www.w3.org/ns/shacl#propertyValidator";

    /// `sh:qualifiedMaxCount` — the maximum number of value nodes conforming to the qualified shape.
    pub const QUALIFIED_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMaxCount";

    /// `sh:QualifiedMaxCountConstraintComponent` — the component reported for `sh:qualifiedMaxCount` violations.
    pub const QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#QualifiedMaxCountConstraintComponent";

    /// `sh:qualifiedMinCount` — the minimum number of value nodes conforming to the qualified shape.
    pub const QUALIFIED_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMinCount";

    /// `sh:QualifiedMinCountConstraintComponent` — the component reported for `sh:qualifiedMinCount` violations.
    pub const QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent";

    /// `sh:qualifiedValueShape` — the shape counted by the qualified cardinality constraints.
    pub const QUALIFIED_VALUE_SHAPE: &str = "http://www.w3.org/ns/shacl#qualifiedValueShape";

    /// `sh:qualifiedValueShapesDisjoint` — sibling qualified value shapes must match disjoint value sets.
    pub const QUALIFIED_VALUE_SHAPES_DISJOINT: &str =
        "http://www.w3.org/ns/shacl#qualifiedValueShapesDisjoint";

    /// `sh:reificationRequired` — whether each value node must carry at least one reifier (SHACL 1.2).
    pub const REIFICATION_REQUIRED: &str = "http://www.w3.org/ns/shacl#reificationRequired";

    /// `sh:reifierShape` — reifiers of value nodes must conform to the given shape (SHACL 1.2).
    pub const REIFIER_SHAPE: &str = "http://www.w3.org/ns/shacl#reifierShape";

    /// `sh:ReifierShapeConstraintComponent` — the component reported for `sh:reifierShape` and `sh:reificationRequired` violations.
    pub const REIFIER_SHAPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ReifierShapeConstraintComponent";

    /// `sh:result` — links a validation report to its validation results.
    pub const RESULT: &str = "http://www.w3.org/ns/shacl#result";

    /// `sh:ResultAnnotation` — the class of result annotations.
    pub const RESULT_ANNOTATION_CLASS: &str = "http://www.w3.org/ns/shacl#ResultAnnotation";

    /// `sh:resultAnnotation` — links a SPARQL-based constraint or validator to a
    /// result annotation.
    pub const RESULT_ANNOTATION: &str = "http://www.w3.org/ns/shacl#resultAnnotation";

    /// `sh:resultMessage` — the human-readable message of a validation result.
    pub const RESULT_MESSAGE: &str = "http://www.w3.org/ns/shacl#resultMessage";

    /// `sh:resultPath` — the property path the reported value nodes were reached through.
    pub const RESULT_PATH: &str = "http://www.w3.org/ns/shacl#resultPath";

    /// `sh:resultSeverity` — the severity of a validation result.
    pub const RESULT_SEVERITY: &str = "http://www.w3.org/ns/shacl#resultSeverity";

    /// `sh:returnType` — the declared datatype/class of a function's return value.
    pub const RETURN_TYPE: &str = "http://www.w3.org/ns/shacl#returnType";

    /// `sh:rootClass` — the parameter of `sh:RootClassConstraintComponent`.
    pub const ROOT_CLASS: &str = "http://www.w3.org/ns/shacl#rootClass";

    /// `sh:RootClassConstraintComponent`.
    pub const ROOT_CLASS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#RootClassConstraintComponent";

    /// `sh:Rule` — the class of SHACL rules.
    pub const RULE_CLASS: &str = "http://www.w3.org/ns/shacl#Rule";

    /// `sh:rule` — attaches a rule to a shape.
    pub const RULE: &str = "http://www.w3.org/ns/shacl#rule";

    /// `sh:ruleProcessor` — a non-standard processor a rule or rule set requires.
    pub const RULE_PROCESSOR: &str = "http://www.w3.org/ns/shacl#ruleProcessor";

    /// `sh:RulesEntailment` — the SHACL rules entailment regime.
    pub const RULES_ENTAILMENT: &str = "http://www.w3.org/ns/shacl#RulesEntailment";

    /// `sh:RuleSet` — the class of rule sets.
    pub const RULE_SET: &str = "http://www.w3.org/ns/shacl#RuleSet";

    /// `sh:RulesGraph` — the class of rules graphs.
    pub const RULES_GRAPH: &str = "http://www.w3.org/ns/shacl#RulesGraph";

    /// `sh:runOnce` — marks a rule executed once, before the layer's iterating rules.
    pub const RUN_ONCE: &str = "http://www.w3.org/ns/shacl#runOnce";

    /// `sh:select` — the SELECT query of a SHACL-SPARQL constraint or validator.
    pub const SELECT: &str = "http://www.w3.org/ns/shacl#select";

    /// `sh:SelectExpression` — the built-in named-parameter function keyed by
    /// `sh:select` (SHACL 1.2 SPARQL Extensions §6.1).
    pub const SELECT_EXPRESSION: &str = "http://www.w3.org/ns/shacl#SelectExpression";

    /// `sh:severity` — overrides the severity of results produced by a shape.
    pub const SEVERITY: &str = "http://www.w3.org/ns/shacl#severity";

    /// `sh:shape` — a DATA-graph statement `n sh:shape s` makes `n` a target of
    /// the shape `s` (SHACL 1.2 Core, "Explicit shape targets").
    pub const SHAPE: &str = "http://www.w3.org/ns/shacl#shape";

    /// `sh:ShapeClass` — "an rdfs:subClassOf of both sh:NodeShape and rdfs:Class"
    /// (SHACL 1.2 Core, "Implicit Class Targets and sh:ShapeClass").
    pub const SHAPE_CLASS: &str = "http://www.w3.org/ns/shacl#ShapeClass";

    /// `sh:ShapesGraph`.
    pub const SHAPES_GRAPH: &str = "http://www.w3.org/ns/shacl#ShapesGraph";

    /// `sh:shapesGraph`.
    pub const SHAPES_GRAPH_PROPERTY: &str = "http://www.w3.org/ns/shacl#shapesGraph";

    /// `sh:shapesGraphWellFormed` — whether the processor checked, and was certain, that
    /// the shapes graph of the validation is well-formed.
    pub const SHAPES_GRAPH_WELL_FORMED: &str = "http://www.w3.org/ns/shacl#shapesGraphWellFormed";

    /// `sh:singleLine` — the parameter of `sh:SingleLineConstraintComponent`.
    pub const SINGLE_LINE: &str = "http://www.w3.org/ns/shacl#singleLine";

    /// `sh:SingleLineConstraintComponent`.
    pub const SINGLE_LINE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#SingleLineConstraintComponent";

    /// `sh:someValue` — the parameter of `sh:SomeValueConstraintComponent`.
    pub const SOME_VALUE: &str = "http://www.w3.org/ns/shacl#someValue";

    /// `sh:SomeValueConstraintComponent`.
    pub const SOME_VALUE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#SomeValueConstraintComponent";

    /// `sh:sourceConstraintComponent` — the constraint component that produced a result.
    pub const SOURCE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#sourceConstraintComponent";
    /// The actual SPARQL-based constraint that produced a validation result.
    pub const SOURCE_CONSTRAINT: &str = "http://www.w3.org/ns/shacl#sourceConstraint";

    /// `sh:sourceRule` — links, on a reifier, an inferred triple with its rule.
    pub const SOURCE_RULE: &str = "http://www.w3.org/ns/shacl#sourceRule";

    /// `sh:sourceShape` — the shape that produced a validation result.
    pub const SOURCE_SHAPE: &str = "http://www.w3.org/ns/shacl#sourceShape";

    /// `sh:sparql` — attaches a SPARQL constraint to a shape.
    pub const SPARQL: &str = "http://www.w3.org/ns/shacl#sparql";

    /// `sh:SPARQLAskValidator` — the class of ASK-query-based validators.
    pub const SPARQL_ASK_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLAskValidator";

    /// `sh:SPARQLConstraint` — the class of SPARQL-based constraints.
    pub const SPARQL_CONSTRAINT: &str = "http://www.w3.org/ns/shacl#SPARQLConstraint";

    /// `sh:SPARQLConstraintComponent` — the constraint component reported for SPARQL constraint violations.
    pub const SPARQL_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#SPARQLConstraintComponent";

    /// `sh:sparqlExpr` — the SPARQL expression of a SPARQL expr expression
    /// (SHACL 1.2 SPARQL Extensions §6.2, function name `sh:SPARQLExprExpression`).
    pub const SPARQL_EXPR: &str = "http://www.w3.org/ns/shacl#sparqlExpr";

    /// `sh:SPARQLExprExpression` — the built-in named-parameter function keyed by
    /// `sh:sparqlExpr` (SHACL 1.2 SPARQL Extensions §6.2).
    pub const SPARQL_EXPR_EXPRESSION: &str = "http://www.w3.org/ns/shacl#SPARQLExprExpression";

    /// `sh:SPARQLFunction` — the class of SPARQL-bodied SHACL-AF functions.
    pub const SPARQL_FUNCTION: &str = "http://www.w3.org/ns/shacl#SPARQLFunction";

    /// `sh:SPARQLRule` — the rule type whose head is a SPARQL `sh:construct` query.
    pub const SPARQL_RULE: &str = "http://www.w3.org/ns/shacl#SPARQLRule";

    /// `sh:SPARQLRuleTemplate` — the class of SPARQL rule templates.
    pub const SPARQL_RULE_TEMPLATE: &str = "http://www.w3.org/ns/shacl#SPARQLRuleTemplate";

    /// `sh:SPARQLSelectValidator` — the class of SELECT-query-based validators.
    pub const SPARQL_SELECT_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLSelectValidator";

    /// `sh:SPARQLTarget` — the class of SPARQL-based custom targets.
    pub const SPARQL_TARGET: &str = "http://www.w3.org/ns/shacl#SPARQLTarget";

    /// `sh:SPARQLTargetType` — the metaclass of parameterized SPARQL-based target types.
    pub const SPARQL_TARGET_TYPE: &str = "http://www.w3.org/ns/shacl#SPARQLTargetType";

    /// `sh:subject` — the subject node expression of a `sh:TripleRule`.
    pub const SUBJECT: &str = "http://www.w3.org/ns/shacl#subject";

    /// `sh:subsetOf` — the parameter of `sh:SubsetOfConstraintComponent`.
    pub const SUBSET_OF: &str = "http://www.w3.org/ns/shacl#subsetOf";

    /// `sh:SubsetOfConstraintComponent`.
    pub const SUBSET_OF_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#SubsetOfConstraintComponent";

    /// `sh:sum` — a sum aggregation node expression.
    pub const SUM: &str = "http://www.w3.org/ns/shacl#sum";

    /// `sh:target` — attaches a custom (e.g. SPARQL-based) target to a shape.
    pub const TARGET: &str = "http://www.w3.org/ns/shacl#target";

    /// `sh:targetClass` — targets all SHACL instances of the given class.
    pub const TARGET_CLASS: &str = "http://www.w3.org/ns/shacl#targetClass";

    /// `sh:targetNode` — targets the output nodes of a node expression: a
    /// constant (IRI, literal, triple term) targets itself.
    pub const TARGET_NODE: &str = "http://www.w3.org/ns/shacl#targetNode";

    /// `sh:targetObjectsOf` — targets all objects of triples with the given predicate.
    pub const TARGET_OBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetObjectsOf";

    /// `sh:targetSubjectsOf` — targets all subjects of triples with the given predicate.
    pub const TARGET_SUBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetSubjectsOf";

    /// `sh:targetWhere` — targets every node of the data graph that conforms to
    /// the given shape.
    pub const TARGET_WHERE: &str = "http://www.w3.org/ns/shacl#targetWhere";

    /// `sh:tempTriple` — marks, on a reifier, a temporary inferred triple.
    pub const TEMP_TRIPLE: &str = "http://www.w3.org/ns/shacl#tempTriple";

    /// `sh:then` — the then-branch of an if/then/else node expression.
    pub const THEN: &str = "http://www.w3.org/ns/shacl#then";

    /// `sh:this` — the focus node: the focus-node expression and the pre-bound `$this` variable.
    pub const THIS: &str = "http://www.w3.org/ns/shacl#this";

    /// `sh:Trace` — "a trace message that is not a constraint violation".
    pub const TRACE: &str = "http://www.w3.org/ns/shacl#Trace";

    /// `sh:TripleRule` — the rule type whose head is a single `sh:subject` /
    /// `sh:predicate` / `sh:object` node-expression triple.
    pub const TRIPLE_RULE: &str = "http://www.w3.org/ns/shacl#TripleRule";

    /// `sh:TripleTerm` — node kind: RDF 1.2 triple terms only (SHACL 1.2 Core
    /// §4.1.3).
    pub const TRIPLE_TERM: &str = "http://www.w3.org/ns/shacl#TripleTerm";

    /// `sh:union` — a union node expression over a list of member expressions.
    pub const UNION: &str = "http://www.w3.org/ns/shacl#union";

    /// `sh:uniqueLang` — no two value nodes may share the same language tag.
    pub const UNIQUE_LANG: &str = "http://www.w3.org/ns/shacl#uniqueLang";

    /// `sh:UniqueLangConstraintComponent` — the component reported for `sh:uniqueLang` violations.
    pub const UNIQUE_LANG_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#UniqueLangConstraintComponent";

    /// `sh:uniqueMembers` — the parameter of `sh:UniqueMembersConstraintComponent`.
    pub const UNIQUE_MEMBERS: &str = "http://www.w3.org/ns/shacl#uniqueMembers";

    /// `sh:UniqueMembersConstraintComponent`.
    pub const UNIQUE_MEMBERS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#UniqueMembersConstraintComponent";

    /// `sh:uniqueValuesFor` — the parameter of `sh:UniqueValuesForConstraintComponent`.
    pub const UNIQUE_VALUES_FOR: &str = "http://www.w3.org/ns/shacl#uniqueValuesFor";

    /// `sh:UniqueValuesForConstraintComponent`.
    pub const UNIQUE_VALUES_FOR_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#UniqueValuesForConstraintComponent";

    /// `sh:update` — the UPDATE request of a `sh:SPARQLUpdateExecutable`.
    pub const UPDATE: &str = "http://www.w3.org/ns/shacl#update";

    /// `sh:ValidationReport` — the class of validation reports.
    pub const VALIDATION_REPORT: &str = "http://www.w3.org/ns/shacl#ValidationReport";

    /// `sh:ValidationResult` — the class of individual validation results.
    pub const VALIDATION_RESULT: &str = "http://www.w3.org/ns/shacl#ValidationResult";

    /// `sh:validator` — the default validator of a constraint component.
    pub const VALIDATOR: &str = "http://www.w3.org/ns/shacl#validator";

    /// `sh:value` — the value node a validation result reports.
    pub const VALUE: &str = "http://www.w3.org/ns/shacl#value";

    /// `sh:values` — the node expression computing a property shape's value
    /// nodes (SHACL 1.2 Core §2.3).
    pub const VALUES: &str = "http://www.w3.org/ns/shacl#values";

    /// `sh:Violation` — the default (most severe) result severity.
    pub const VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";

    /// `sh:Warning` — the warning result severity.
    pub const WARNING: &str = "http://www.w3.org/ns/shacl#Warning";

    /// `sh:xone` — value nodes must conform to exactly one shape in the given list.
    pub const XONE: &str = "http://www.w3.org/ns/shacl#xone";

    /// `sh:XoneConstraintComponent` — the component reported for `sh:xone` violations.
    pub const XONE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#XoneConstraintComponent";

    /// `sh:zeroOrMorePath` — a zero-or-more (`*`) property path.
    pub const ZERO_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrMorePath";

    /// `sh:zeroOrOnePath` — a zero-or-one (`?`) property path.
    pub const ZERO_OR_ONE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrOnePath";
}

/// SHACL 1.2 Node Expressions (`http://www.w3.org/ns/shacl-node-expr#`).
///
/// Specification: <https://www.w3.org/TR/shacl12-node-expr/>.
///
/// Where a kind here has an older SHACL Advanced Features spelling in the [`sh`]
/// namespace (`sh:union`, `sh:if`, `sh:count`, …), both spellings name the same
/// node expression: two spec-defined surfaces over one meaning.
pub mod shnex {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/shacl-node-expr#";

    /// `shnex:arg` — the argument key of an arg expression (§6.3): either an IRI
    /// (a custom named parameter function's parameter `sh:path`) or an
    /// `xsd:integer` (a custom list parameter function's zero-based argument
    /// index).
    /// A custom LIST parameter function documents its arguments with
    /// `sh:parameter [ sh:path shnex:arg0 ]`, `shnex:arg1`, … (§6.2); those IRIs
    /// are this constant with the zero-based index appended.
    pub const ARG: &str = "http://www.w3.org/ns/shacl-node-expr#arg";

    /// `shnex:concat` — the member list of a concat expression (§4.2.3).
    pub const CONCAT: &str = "http://www.w3.org/ns/shacl-node-expr#concat";

    /// `shnex:conformsToShape` — the two-member argument list of a conformsToShape
    /// expression (§4.5.3).
    pub const CONFORMS_TO_SHAPE: &str = "http://www.w3.org/ns/shacl-node-expr#conformsToShape";

    /// `shnex:count` — the operand of a count expression (§4.4.1).
    pub const COUNT: &str = "http://www.w3.org/ns/shacl-node-expr#count";

    /// `shnex:desc` — the descending flag of an order by expression (§4.2.8).
    pub const DESC: &str = "http://www.w3.org/ns/shacl-node-expr#desc";

    /// `shnex:distinct` — the operand of a distinct expression (§4.2.1).
    pub const DISTINCT: &str = "http://www.w3.org/ns/shacl-node-expr#distinct";

    /// `shnex:else` — the else-branch of an if expression (§4.1.6).
    pub const ELSE: &str = "http://www.w3.org/ns/shacl-node-expr#else";

    /// `shnex:exists` — the operand of an exists expression (§4.1.5).
    pub const EXISTS: &str = "http://www.w3.org/ns/shacl-node-expr#exists";

    /// `shnex:filterShape` — the filter shape of a filter shape expression (§4.2.5).
    pub const FILTER_SHAPE: &str = "http://www.w3.org/ns/shacl-node-expr#filterShape";

    /// `shnex:findFirst` — the shape of a find first expression (§4.3.2).
    pub const FIND_FIRST: &str = "http://www.w3.org/ns/shacl-node-expr#findFirst";

    /// `shnex:flatMap` — the per-node expression of a flat map expression (§4.3.1).
    pub const FLAT_MAP: &str = "http://www.w3.org/ns/shacl-node-expr#flatMap";

    /// `shnex:focusNode` — the (optional) focus-node expression of a path values
    /// expression (§4.1.4).
    pub const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl-node-expr#focusNode";

    /// `shnex:if` — the condition of an if expression (§4.1.6).
    pub const IF: &str = "http://www.w3.org/ns/shacl-node-expr#if";

    /// `shnex:instancesOf` — the class of an instancesOf expression (§4.5.1).
    pub const INSTANCES_OF: &str = "http://www.w3.org/ns/shacl-node-expr#instancesOf";

    /// `shnex:intersection` — the member list of an intersection expression (§4.2.2).
    pub const INTERSECTION: &str = "http://www.w3.org/ns/shacl-node-expr#intersection";

    /// `shnex:limit` — the maximum node count of a limit expression (§4.2.6).
    pub const LIMIT: &str = "http://www.w3.org/ns/shacl-node-expr#limit";

    /// `shnex:matchAll` — the shape of a match all expression (§4.3.3).
    pub const MATCH_ALL: &str = "http://www.w3.org/ns/shacl-node-expr#matchAll";

    /// `shnex:max` — the operand of a max expression (§4.4.3).
    pub const MAX: &str = "http://www.w3.org/ns/shacl-node-expr#max";

    /// `shnex:min` — the operand of a min expression (§4.4.2).
    pub const MIN: &str = "http://www.w3.org/ns/shacl-node-expr#min";

    /// `shnex:nodes` — the input-nodes expression shared by the remove, filter
    /// shape, limit, offset, order by, flat map, find first and match all
    /// expressions (§4.2.4–§4.3.3).
    pub const NODES: &str = "http://www.w3.org/ns/shacl-node-expr#nodes";

    /// `shnex:nodesMatching` — the shape of a nodes matching expression (§4.5.2).
    pub const NODES_MATCHING: &str = "http://www.w3.org/ns/shacl-node-expr#nodesMatching";

    /// `shnex:offset` — the skipped node count of an offset expression (§4.2.7).
    pub const OFFSET: &str = "http://www.w3.org/ns/shacl-node-expr#offset";

    /// `shnex:orderBy` — the sort-key expression of an order by expression (§4.2.8).
    ///
    /// Note the capital `B`: the SHACL-AF spelling this repository already
    /// supports is the all-lowercase [`sh::ORDERBY`](super::sh::ORDERBY).
    pub const ORDER_BY: &str = "http://www.w3.org/ns/shacl-node-expr#orderBy";

    /// `shnex:pathValues` — the SHACL property path of a path values expression (§4.1.4).
    pub const PATH_VALUES: &str = "http://www.w3.org/ns/shacl-node-expr#pathValues";

    /// `shnex:remove` — the nodes removed by a remove expression (§4.2.4).
    pub const REMOVE: &str = "http://www.w3.org/ns/shacl-node-expr#remove";

    /// `shnex:sum` — the operand of a sum expression (§4.4.4).
    pub const SUM: &str = "http://www.w3.org/ns/shacl-node-expr#sum";

    /// `shnex:then` — the then-branch of an if expression (§4.1.6).
    pub const THEN: &str = "http://www.w3.org/ns/shacl-node-expr#then";

    /// `shnex:var` — the variable name of a var expression (§4.1.2).
    pub const VAR: &str = "http://www.w3.org/ns/shacl-node-expr#var";
}

/// SPARQL 1.2 Query Language (the operator and function vocabulary) (`http://www.w3.org/ns/sparql#`).
///
/// Specification: <https://www.w3.org/TR/sparql12-query/>.
///
/// The SPARQL Working Group's `sparql-ns.ttl` mints one IRI per SPARQL 1.2
/// operator, functional form, function and aggregate, and SHACL 1.2 Node
/// Expressions §5 makes them callable from a node expression. Only the namespace
/// is named here: the local names are the SPARQL grammar's own.
pub mod sparql {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/sparql#";
}

/// SPARQL 1.1 Service Description (`http://www.w3.org/ns/sparql-service-description#`).
///
/// Specification: <https://www.w3.org/TR/sparql11-service-description/>.
pub mod sd {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/sparql-service-description#";
}

/// XPath and XQuery Functions and Operators (`http://www.w3.org/2005/xpath-functions#`).
///
/// Specification: <https://www.w3.org/TR/xpath-functions-31/>.
pub mod xpath {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2005/xpath-functions#";
}

/// SKOS Simple Knowledge Organization System Reference (`http://www.w3.org/2004/02/skos/core#`).
///
/// Specification: <https://www.w3.org/TR/skos-reference/>.
pub mod skos {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2004/02/skos/core#";

    /// `skos:prefLabel`.
    pub const PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
    /// `skos:altLabel`.
    pub const ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";
    /// `skos:hiddenLabel`.
    pub const HIDDEN_LABEL: &str = "http://www.w3.org/2004/02/skos/core#hiddenLabel";

    /// `skos:note`.
    pub const NOTE: &str = "http://www.w3.org/2004/02/skos/core#note";
    /// `skos:definition`.
    pub const DEFINITION: &str = "http://www.w3.org/2004/02/skos/core#definition";
    /// `skos:scopeNote`.
    pub const SCOPE_NOTE: &str = "http://www.w3.org/2004/02/skos/core#scopeNote";
    /// `skos:example`.
    pub const EXAMPLE: &str = "http://www.w3.org/2004/02/skos/core#example";
    /// `skos:historyNote`.
    pub const HISTORY_NOTE: &str = "http://www.w3.org/2004/02/skos/core#historyNote";
    /// `skos:editorialNote`.
    pub const EDITORIAL_NOTE: &str = "http://www.w3.org/2004/02/skos/core#editorialNote";
    /// `skos:changeNote`.
    pub const CHANGE_NOTE: &str = "http://www.w3.org/2004/02/skos/core#changeNote";
}

/// PROV-O: The PROV Ontology (`http://www.w3.org/ns/prov#`).
///
/// Specification: <https://www.w3.org/TR/prov-o/>.
pub mod prov {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/prov#";
}

/// RIF Core Dialect and RIF XML Data Types (`http://www.w3.org/2007/rif#`).
///
/// Specification: <https://www.w3.org/TR/rif-core/>.
pub mod rif {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2007/rif#";

    /// `rif:usedWithProfile`.
    pub const USED_WITH_PROFILE: &str = "http://www.w3.org/2007/rif#usedWithProfile";
}

/// Internationalization Tag Set (ITS) 2.0 (`http://www.w3.org/2005/11/its`).
///
/// Specification: <https://www.w3.org/TR/its20/>.
///
/// The namespace of the ITS 2.0 attributes; `its:dir` carries an RDF 1.2 base
/// direction in RDF/XML and in the SPARQL XML results format.
pub mod its {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2005/11/its";
}

/// Namespaces in XML 1.0 (the `xml:` prefix) (`http://www.w3.org/XML/1998/namespace`).
///
/// Specification: <https://www.w3.org/TR/xml-names/>.
///
/// The namespace bound to the reserved `xml:` prefix; `xml:lang` and `xml:base`
/// are its attributes.
pub mod xml {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/XML/1998/namespace";
}

/// Data Catalog Vocabulary (DCAT) (`http://www.w3.org/ns/dcat#`).
///
/// Specification: <https://www.w3.org/TR/vocab-dcat-3/>.
pub mod dcat {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/dcat#";
}

/// The Organization Ontology (`http://www.w3.org/ns/org#`).
///
/// Specification: <https://www.w3.org/TR/vocab-org/>.
pub mod org {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/org#";
}

/// Web Annotation Vocabulary (`http://www.w3.org/ns/oa#`).
///
/// Specification: <https://www.w3.org/TR/annotation-vocab/>.
pub mod oa {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/oa#";
}

/// ODRL Information Model 2.2 (`http://www.w3.org/ns/odrl/2/`).
///
/// Specification: <https://www.w3.org/TR/odrl-vocab/>.
pub mod odrl {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/odrl/2/";
}

/// Time Ontology in OWL (`http://www.w3.org/2006/time#`).
///
/// Specification: <https://www.w3.org/TR/owl-time/>.
pub mod time {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/2006/time#";
}

/// Semantic Sensor Network Ontology (SOSA) (`http://www.w3.org/ns/sosa/`).
///
/// Specification: <https://www.w3.org/TR/vocab-ssn/>.
pub mod sosa {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/sosa/";
}

/// Semantic Sensor Network Ontology (SSN) (`http://www.w3.org/ns/ssn/`).
///
/// Specification: <https://www.w3.org/TR/vocab-ssn/>.
pub mod ssn {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/ssn/";
}

/// Ontology for Media Resources 1.0 (`http://www.w3.org/ns/ma-ont#`).
///
/// Specification: <https://www.w3.org/TR/mediaont-10/>.
pub mod ma {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "http://www.w3.org/ns/ma-ont#";
}

/// Activity Vocabulary (`https://www.w3.org/ns/activitystreams#`).
///
/// Specification: <https://www.w3.org/TR/activitystreams-vocabulary/>.
pub mod activitystreams {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "https://www.w3.org/ns/activitystreams#";
}

/// Decentralized Identifiers (DIDs) v1.0 (`https://www.w3.org/ns/did#`).
///
/// Specification: <https://www.w3.org/TR/did-core/>.
pub mod did {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "https://www.w3.org/ns/did#";
}

/// Verifiable Credentials Data Model (`https://www.w3.org/2018/credentials#`).
///
/// Specification: <https://www.w3.org/TR/vc-data-model-2.0/>.
pub mod cred {
    /// The namespace IRI: every term of this vocabulary starts with it.
    pub const NS: &str = "https://www.w3.org/2018/credentials#";
}

/// DAWG test manifest vocabulary.
pub mod mf {
    /// The vocabulary namespace.
    pub const NS: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
    /// `mf:Manifest`.
    pub const MANIFEST: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#Manifest";
    /// `mf:entries`.
    pub const ENTRIES: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#entries";
    /// `mf:include`.
    pub const INCLUDE: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#include";
    /// `mf:action`.
    pub const ACTION: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#action";
    /// `mf:result`.
    pub const RESULT: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#result";
    /// `mf:name`.
    pub const NAME: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#name";
    /// `mf:status`.
    pub const STATUS: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#status";
    /// `mf:requires`.
    pub const REQUIRES: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#requires";
    /// `mf:QueryEvaluationTest`.
    pub const QUERY_EVALUATION_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#QueryEvaluationTest";
    /// `mf:UpdateEvaluationTest`.
    pub const UPDATE_EVALUATION_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#UpdateEvaluationTest";
    /// `mf:PositiveSyntaxTest`.
    pub const POSITIVE_SYNTAX_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#PositiveSyntaxTest";
    /// `mf:PositiveSyntaxTest11`.
    pub const POSITIVE_SYNTAX_TEST11: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#PositiveSyntaxTest11";
    /// `mf:NegativeSyntaxTest`.
    pub const NEGATIVE_SYNTAX_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#NegativeSyntaxTest";
    /// `mf:NegativeSyntaxTest11`.
    pub const NEGATIVE_SYNTAX_TEST11: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#NegativeSyntaxTest11";
    /// `mf:PositiveUpdateSyntaxTest`.
    pub const POSITIVE_UPDATE_SYNTAX_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#PositiveUpdateSyntaxTest";
    /// `mf:NegativeUpdateSyntaxTest`.
    pub const NEGATIVE_UPDATE_SYNTAX_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#NegativeUpdateSyntaxTest";
    /// `mf:ResultFormatTest`.
    pub const RESULT_FORMAT_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#ResultFormatTest";
    /// `mf:CSVResultFormatTest`.
    pub const CSVRESULT_FORMAT_TEST: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#CSVResultFormatTest";
}

/// DAWG RDF result-set vocabulary.
pub mod rs {
    /// The vocabulary namespace.
    pub const NS: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#";
    /// `rs:ResultSet`.
    pub const RESULT_SET: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#ResultSet";
    /// `rs:resultVariable`.
    pub const RESULT_VARIABLE: &str =
        "http://www.w3.org/2001/sw/DataAccess/tests/result-set#resultVariable";
    /// `rs:solution`.
    pub const SOLUTION: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#solution";
    /// `rs:binding`.
    pub const BINDING: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#binding";
    /// `rs:variable`.
    pub const VARIABLE: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#variable";
    /// `rs:value`.
    pub const VALUE: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#value";
    /// `rs:index`.
    pub const INDEX: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#index";
    /// `rs:boolean`.
    pub const BOOLEAN: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#boolean";
}

/// SHACL test vocabulary.
pub mod sht {
    /// The vocabulary namespace.
    pub const NS: &str = "http://www.w3.org/ns/shacl-test#";
    /// `sht:Validate`.
    pub const VALIDATE: &str = "http://www.w3.org/ns/shacl-test#Validate";
    /// `sht:Failure`.
    pub const FAILURE: &str = "http://www.w3.org/ns/shacl-test#Failure";
    /// `sht:shapesGraph`.
    pub const SHAPES_GRAPH: &str = "http://www.w3.org/ns/shacl-test#shapesGraph";
    /// `sht:dataGraph`.
    pub const DATA_GRAPH: &str = "http://www.w3.org/ns/shacl-test#dataGraph";
    /// `sht:proposed`.
    pub const PROPOSED: &str = "http://www.w3.org/ns/shacl-test#proposed";
    /// `sht:approved`.
    pub const APPROVED: &str = "http://www.w3.org/ns/shacl-test#approved";
    /// `sht:Infer`.
    pub const INFER: &str = "http://www.w3.org/ns/shacl-test#Infer";
    /// `sht:EvalNodeExpr`.
    pub const EVAL_NODE_EXPR: &str = "http://www.w3.org/ns/shacl-test#EvalNodeExpr";
    /// `sht:EvalNodeExprList`.
    pub const EVAL_NODE_EXPR_LIST: &str = "http://www.w3.org/ns/shacl-test#EvalNodeExprList";
    /// `sht:evalNodeExpr`.
    pub const EVAL_NODE_EXPR_PROPERTY: &str = "http://www.w3.org/ns/shacl-test#evalNodeExpr";
    /// `sht:expectedResult`.
    pub const EXPECTED_RESULT: &str = "http://www.w3.org/ns/shacl-test#expectedResult";
    /// `sht:focusNode`.
    pub const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl-test#focusNode";
    /// `sht:nodeExpr`.
    pub const NODE_EXPR: &str = "http://www.w3.org/ns/shacl-test#nodeExpr";
    /// `sht:ignoreOrder`.
    pub const IGNORE_ORDER: &str = "http://www.w3.org/ns/shacl-test#ignoreOrder";
    /// Prefix of the scope variable relation family.
    pub const SCOPE_PREFIX: &str = "http://www.w3.org/ns/shacl-test#scope-";
    /// `sht:schema`.
    pub const SCHEMA: &str = "http://www.w3.org/ns/shacl-test#schema";
    /// `sht:result`.
    pub const RESULT: &str = "http://www.w3.org/ns/shacl-test#result";
}

/// EARL 1.0 vocabulary (https://www.w3.org/TR/EARL10-Schema/).
pub mod earl {
    /// The vocabulary namespace.
    pub const NS: &str = "http://www.w3.org/ns/earl#";
    /// `earl:Assertion`.
    pub const ASSERTION: &str = "http://www.w3.org/ns/earl#Assertion";
    /// `earl:TestResult`.
    pub const TEST_RESULT: &str = "http://www.w3.org/ns/earl#TestResult";
    /// `earl:assertedBy`.
    pub const ASSERTED_BY: &str = "http://www.w3.org/ns/earl#assertedBy";
    /// `earl:subject`.
    pub const SUBJECT: &str = "http://www.w3.org/ns/earl#subject";
    /// `earl:test`.
    pub const TEST: &str = "http://www.w3.org/ns/earl#test";
    /// `earl:result`.
    pub const RESULT: &str = "http://www.w3.org/ns/earl#result";
    /// `earl:outcome`.
    pub const OUTCOME: &str = "http://www.w3.org/ns/earl#outcome";
    /// `earl:mode`.
    pub const MODE: &str = "http://www.w3.org/ns/earl#mode";
    /// `earl:automatic`.
    pub const AUTOMATIC: &str = "http://www.w3.org/ns/earl#automatic";
    /// `earl:passed`.
    pub const PASSED: &str = "http://www.w3.org/ns/earl#passed";
    /// `earl:failed`.
    pub const FAILED: &str = "http://www.w3.org/ns/earl#failed";
    /// `earl:inapplicable`.
    pub const INAPPLICABLE: &str = "http://www.w3.org/ns/earl#inapplicable";
    /// `earl:untested`.
    pub const UNTESTED: &str = "http://www.w3.org/ns/earl#untested";
    /// `earl:cantTell`.
    pub const CANT_TELL: &str = "http://www.w3.org/ns/earl#cantTell";
}

#[cfg(test)]
mod tests {
    use super::{language_datatype_iri, rdf, skos};

    #[test]
    fn skos_label_and_documentation_constants_match_the_reference_local_names() {
        // A consumer can use every term in a const context, without allocating
        // or restating its full IRI outside the vocabulary home.
        const TERMS: [(&str, &str); 10] = [
            (skos::PREF_LABEL, "prefLabel"),
            (skos::ALT_LABEL, "altLabel"),
            (skos::HIDDEN_LABEL, "hiddenLabel"),
            (skos::NOTE, "note"),
            (skos::DEFINITION, "definition"),
            (skos::SCOPE_NOTE, "scopeNote"),
            (skos::EXAMPLE, "example"),
            (skos::HISTORY_NOTE, "historyNote"),
            (skos::EDITORIAL_NOTE, "editorialNote"),
            (skos::CHANGE_NOTE, "changeNote"),
        ];
        for (term, local) in TERMS {
            assert_eq!(term.strip_prefix(skos::NS), Some(local));
        }
    }

    /// Every term constant of a module starts with that module's `NS`, and no
    /// two constants of one module spell the same IRI.
    #[test]
    fn every_term_lies_in_its_modules_namespace_once() {
        let source = include_str!("vocab.rs");
        let mut namespace: Option<&str> = None;
        let mut seen: Vec<&str> = Vec::new();
        let mut checked = 0;
        let mut lines = source.lines();
        while let Some(line) = lines.next() {
            let line = line.trim();
            if line.starts_with("pub mod ") {
                namespace = None;
                seen.clear();
                continue;
            }
            let Some(rest) = line.strip_prefix("pub const ") else {
                continue;
            };
            if !rest.contains(": &str") {
                continue;
            }
            let text = if rest.contains('"') {
                rest
            } else {
                lines.next().expect("a wrapped constant continues").trim()
            };
            let value = text
                .split('"')
                .nth(1)
                .expect("a string constant holds a literal");
            if rest.starts_with("NS:") {
                namespace = Some(value);
                continue;
            }
            let namespace = namespace.expect("NS is declared first in every module");
            assert!(
                value.starts_with(namespace) && value.len() > namespace.len(),
                "{value} is not a term of {namespace}"
            );
            assert!(!seen.contains(&value), "{value} is declared twice");
            seen.push(value);
            checked += 1;
        }
        assert!(checked > 300, "only {checked} terms were read");
    }

    #[test]
    fn a_direction_selects_dir_lang_string_and_its_absence_lang_string() {
        assert_eq!(
            language_datatype_iri(true),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString"
        );
        assert_eq!(
            language_datatype_iri(false),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
        );
        assert_eq!(language_datatype_iri(false), rdf::LANG_STRING);
    }
}
