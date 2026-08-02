// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The OWL 2 construct inventory: every reserved term the reverse mapping can meet, and
//! what this layer does with it.
//!
//! # Why this table exists
//!
//! The reverse mapping used to end in a catch-all. Any structural triple it did not
//! recognize returned `Ok(())`, so `owl:TransitiveProperty`, `owl:SymmetricProperty`,
//! `owl:InverseFunctionalProperty`, `owl:differentFrom`, property disjointness and
//! `owl:hasKey` were parsed and thrown away without a word — and `owl:propertyChainAxiom`
//! was worse than thrown away, because it fell past the structural filter and was ingested
//! as a ROLE ASSERTION whose object was the axiom's RDF list head. A knowledge base that
//! quietly loses axioms answers a question the caller did not ask, and one that mis-reads
//! an axiom answers it wrongly.
//!
//! [`OWL2_CONSTRUCTS`] replaces that with a total function. Every reserved term is listed
//! here with exactly one [`Support`]:
//!
//! * [`Support::Handled`] — the term reaches the knowledge base, and the note says as what;
//! * [`Support::Inert`] — the term carries no OWL 2 Direct-Semantics content, so reading it
//!   changes no model and ignoring it loses nothing (annotations, the ontology header);
//! * [`Support::Bounded`] — the term is NOT fully handled, and names the
//!   [`Construct`] boundary the run raises for it.
//!
//! There is no fourth answer, and no default: [`support_of`] is driven by this table alone,
//! and a reserved term the table does not name is [`Construct::UnrecognizedTerm`] — a
//! boundary, never silence. `every_owl2_construct_is_handled_or_bounded` drives a minimal
//! fixture through the parser for each entry and asserts the outcome matches the entry, so
//! the table cannot claim something the parser does not do.
//!
//! # Determinism
//!
//! The table is a `&'static` slice in a fixed reading order (RDF, then RDFS, then the OWL 2
//! vocabulary alphabetically within each block), and [`support_of`] is a linear scan over
//! it. Nothing here is a map, so nothing here has an iteration order to leak.

use crate::report::Construct;
use crate::vocab::{
    OWL_ALLDIFFERENT, OWL_ALLDISJOINTCLASSES, OWL_ALLDISJOINTPROPERTIES, OWL_ALLVALUESFROM,
    OWL_ANNOTATEDPROPERTY, OWL_ANNOTATEDSOURCE, OWL_ANNOTATEDTARGET, OWL_ANNOTATION,
    OWL_ANNOTATIONPROPERTY, OWL_ASSERTIONPROPERTY, OWL_ASYMMETRICPROPERTY, OWL_AXIOM,
    OWL_BACKWARDCOMPATIBLEWITH, OWL_BOTTOMDATAPROPERTY, OWL_BOTTOMOBJECTPROPERTY, OWL_CARDINALITY,
    OWL_CLASS, OWL_COMPLEMENTOF, OWL_DATARANGE, OWL_DATATYPECOMPLEMENTOF, OWL_DATATYPEPROPERTY,
    OWL_DEPRECATED, OWL_DIFFERENTFROM, OWL_DISJOINTUNIONOF, OWL_DISJOINTWITH, OWL_DISTINCTMEMBERS,
    OWL_EQUIVALENTCLASS, OWL_EQUIVALENTPROPERTY, OWL_FUNCTIONALPROPERTY, OWL_HASKEY, OWL_HASSELF,
    OWL_HASVALUE, OWL_IMPORTS, OWL_INCOMPATIBLEWITH, OWL_INTERSECTIONOF,
    OWL_INVERSEFUNCTIONALPROPERTY, OWL_INVERSEOF, OWL_IRREFLEXIVEPROPERTY, OWL_MAXCARDINALITY,
    OWL_MAXQUALIFIEDCARDINALITY, OWL_MEMBERS, OWL_MINCARDINALITY, OWL_MINQUALIFIEDCARDINALITY,
    OWL_NAMEDINDIVIDUAL, OWL_NEGATIVEPROPERTYASSERTION, OWL_NOTHING, OWL_OBJECTPROPERTY,
    OWL_ONCLASS, OWL_ONDATARANGE, OWL_ONDATATYPE, OWL_ONEOF, OWL_ONPROPERTIES, OWL_ONPROPERTY,
    OWL_ONTOLOGY, OWL_ONTOLOGYPROPERTY, OWL_PRIORVERSION, OWL_PROPERTYCHAINAXIOM,
    OWL_PROPERTYDISJOINTWITH, OWL_QUALIFIEDCARDINALITY, OWL_RATIONAL, OWL_REAL,
    OWL_REFLEXIVEPROPERTY, OWL_RESTRICTION, OWL_SAMEAS, OWL_SOMEVALUESFROM, OWL_SOURCEINDIVIDUAL,
    OWL_SYMMETRICPROPERTY, OWL_TARGETINDIVIDUAL, OWL_TARGETVALUE, OWL_THING, OWL_TOPDATAPROPERTY,
    OWL_TOPOBJECTPROPERTY, OWL_TRANSITIVEPROPERTY, OWL_UNIONOF, OWL_VERSIONINFO, OWL_VERSIONIRI,
    OWL_WITHRESTRICTIONS, RDF_FIRST, RDF_NIL, RDF_PROPERTY, RDF_REST, RDF_TYPE, RDFS_CLASS,
    RDFS_COMMENT, RDFS_DATATYPE, RDFS_DOMAIN, RDFS_ISDEFINEDBY, RDFS_LABEL, RDFS_LITERAL,
    RDFS_RANGE, RDFS_SEEALSO, RDFS_SUBCLASSOF, RDFS_SUBPROPERTYOF,
};

/// The three reserved namespaces the OWL-2-RDF mapping draws its vocabulary from.
///
/// A term in one of them that [`OWL2_CONSTRUCTS`] does not name is
/// [`Construct::UnrecognizedTerm`]; a term outside all three is a caller's own vocabulary
/// and is read as an ordinary class, property or individual. Splitting on the namespace is
/// what makes "unrecognized" a decidable question rather than a guess.
pub(crate) const RESERVED_NAMESPACES: [&str; 3] = [
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/2002/07/owl#",
];

/// Whether `iri` sits in one of the [`RESERVED_NAMESPACES`].
pub(crate) fn is_reserved(iri: &str) -> bool {
    RESERVED_NAMESPACES
        .iter()
        .any(|namespace| iri.starts_with(namespace))
}

/// Where in the OWL-2-RDF mapping a term is written — which fixes the minimal graph
/// `every_owl2_construct_is_handled_or_bounded` drives through the parser for it.
///
/// A shape is not decoration: it is the difference between a fixture that exercises the
/// construct and one that does not. `owl:someValuesFrom` written on a blank node nothing
/// references states nothing at all, so its shape builds the whole restriction — the
/// `rdfs:subClassOf` that references it, the `owl:Restriction` typing, the
/// `owl:onProperty` — and the test then has a right to demand that the knowledge base grew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// `ex:a <iri> ex:b`.
    IriPredicate,
    /// `ex:a <iri> "1"^^xsd:nonNegativeInteger`.
    LiteralPredicate,
    /// `ex:a <iri> ( ex:b ex:c )`.
    ListPredicate,
    /// `ex:a rdf:type <iri>`.
    TypeObject,
    /// `_:x rdf:type <iri> ; owl:members ( ex:b ex:c )` — an n-ary axiom node.
    AxiomNode,
    /// The four-triple `owl:NegativePropertyAssertion` reification.
    NegativeAssertion,
    /// `ex:a rdfs:subClassOf <iri>` — the term DENOTES a class.
    ClassDenotation,
    /// `ex:a <iri> ex:b` — the term DENOTES a role, exercised by an assertion over it.
    RoleDenotation,
    /// A complete restriction whose constraint is `<iri> ex:C`.
    RestrictionIri,
    /// A complete restriction whose constraint is `<iri> "1"^^xsd:nonNegativeInteger`.
    RestrictionLiteral,
    /// The same, plus the `owl:onClass ex:C` a qualified cardinality needs.
    RestrictionQualified,
    /// A complete qualified-cardinality restriction, for the two terms that are COMPONENTS
    /// of a restriction (`owl:onProperty`, `owl:onClass`) and so cannot be its constraint.
    RestrictionOperand,
    /// A complete restriction whose constraint is `<iri> "true"^^xsd:boolean`.
    RestrictionSelf,
    /// A referenced class expression `_:c <iri> ( ex:b ex:c )`.
    ClassExprList,
    /// A referenced class expression `_:c <iri> ex:b`.
    ClassExprIri,
    /// A referenced class expression over a one-element RDF collection, which is what
    /// exercises the `rdf:first` / `rdf:rest` / `rdf:nil` walk.
    Collection,
    /// A complete restriction whose filler is a DATATYPE RESTRICTION: an `owl:someValuesFrom`
    /// over `[ rdf:type rdfs:Datatype ; owl:onDatatype xsd:integer ;
    /// owl:withRestrictions ( [ xsd:minInclusive "1"^^xsd:integer ] ) ]`.
    DatatypeRestriction,
    /// The same, with `owl:datatypeComplementOf xsd:integer` as the data range.
    DatatypeComplement,
    /// A complete qualified-cardinality restriction whose filler is a data range
    /// (`owl:minQualifiedCardinality` with `owl:onDataRange`).
    DataCardinality,
}

/// What the OWL-Direct layer does with one construct.
///
/// Four answers, and no fifth. The split between the first three is what a caller actually
/// needs to know: an axiom that reaches the knowledge base, a component that is read from
/// the axiom that carries it, and a term that constrains no model at all are three
/// different kinds of "fine", and collapsing them would make the fourth — a boundary —
/// indistinguishable from a construct that simply had nothing to contribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Support {
    /// The construct states an AXIOM that reaches the knowledge base. The note names the DL
    /// form it becomes, and the inventory test demands that its minimal fixture actually
    /// grows the knowledge base.
    Handled(&'static str),
    /// The construct is a COMPONENT of an axiom — an operand list, a restriction's property
    /// or filler, a negative assertion's subject — read from the axiom node that carries it.
    /// On its own it states nothing, so its fixture is not required to grow the knowledge
    /// base; what is required is that the parser recognizes it and raises no boundary.
    Operand(&'static str),
    /// The construct carries no OWL 2 Direct-Semantics content: every interpretation
    /// satisfies it, so reading it constrains no model and not reading it loses nothing.
    Inert(&'static str),
    /// The construct is not fully handled, and raises this boundary. The technical reason
    /// lives on [`Construct::reason`], so the reason and the construct cannot drift apart.
    Bounded(Construct),
}

/// One reserved term of the OWL-2-RDF mapping.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OwlConstruct {
    /// The vocabulary IRI, exactly as the mapping spells it.
    pub(crate) iri: &'static str,
    /// Where the mapping writes it, which fixes the fixture the inventory test builds.
    ///
    /// Read by `every_owl2_construct_is_handled_or_bounded` alone: a shape is a
    /// TEST-fixture recipe, and the parser learns nothing from it — the parser is driven by
    /// [`Support`], which is what makes the test an independent check of the table rather
    /// than a restatement of it.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by the inventory test only")
    )]
    pub(crate) shape: Shape,
    /// What this layer does with it.
    pub(crate) support: Support,
}

/// Shorthand for an axiom-stating entry.
const fn handled(iri: &'static str, shape: Shape, note: &'static str) -> OwlConstruct {
    OwlConstruct {
        iri,
        shape,
        support: Support::Handled(note),
    }
}

/// Shorthand for an axiom-component entry.
const fn operand(iri: &'static str, shape: Shape, note: &'static str) -> OwlConstruct {
    OwlConstruct {
        iri,
        shape,
        support: Support::Operand(note),
    }
}

/// Shorthand for a semantically inert entry.
const fn inert(iri: &'static str, shape: Shape, note: &'static str) -> OwlConstruct {
    OwlConstruct {
        iri,
        shape,
        support: Support::Inert(note),
    }
}

/// Shorthand for a bounded entry.
const fn bounded(iri: &'static str, shape: Shape, construct: Construct) -> OwlConstruct {
    OwlConstruct {
        iri,
        shape,
        support: Support::Bounded(construct),
    }
}

/// Every reserved term the OWL-2-RDF reverse mapping can meet, and its [`Support`].
///
/// Reading order is RDF, then RDFS, then OWL 2 — and within the OWL 2 block, the
/// specification's own grouping: class expressions, class axioms, property axioms,
/// individual axioms, keys, data ranges, built-in roles, the ontology header and the
/// annotation vocabulary. That is a reading order for a human; nothing derives an answer
/// from it.
pub(crate) const OWL2_CONSTRUCTS: &[OwlConstruct] = &[
    // --- RDF ---------------------------------------------------------------------------
    handled(
        RDF_TYPE,
        Shape::IriPredicate,
        "the concept assertion a : C, or — when the object is one of the structural classes \
         below — the declaration or axiom that class names",
    ),
    inert(
        RDF_PROPERTY,
        Shape::TypeObject,
        "a property declaration: it names a property without constraining any model",
    ),
    handled(
        RDF_FIRST,
        Shape::Collection,
        "walked by the RDF-collection reader that recovers an owl:intersectionOf / unionOf \
         / oneOf / members / distinctMembers / hasKey / disjointUnionOf operand list",
    ),
    handled(RDF_REST, Shape::Collection, "see rdf:first"),
    handled(
        RDF_NIL,
        Shape::Collection,
        "the empty collection, which terminates the walk",
    ),
    // --- RDFS --------------------------------------------------------------------------
    handled(RDFS_SUBCLASSOF, Shape::IriPredicate, "the GCI C ⊑ D"),
    handled(
        RDFS_SUBPROPERTYOF,
        Shape::IriPredicate,
        "the simple role inclusion r ⊑ s, closed over by the role hierarchy",
    ),
    handled(RDFS_DOMAIN, Shape::IriPredicate, "the GCI ∃r.⊤ ⊑ C"),
    handled(RDFS_RANGE, Shape::IriPredicate, "the GCI ⊤ ⊑ ∀r.C"),
    inert(
        RDFS_CLASS,
        Shape::TypeObject,
        "a class declaration: it names a class without constraining any model",
    ),
    operand(
        RDFS_DATATYPE,
        Shape::TypeObject,
        "marks the node as a DATA RANGE rather than a class expression — which is the only \
         thing that tells an owl:intersectionOf / unionOf / oneOf over the concrete domain \
         apart from one over the abstract one; the range itself is read when an axiom \
         REFERENCES the node",
    ),
    handled(
        RDFS_LITERAL,
        Shape::ClassDenotation,
        "the whole DATA DOMAIN — the data range every literal value inhabits, and the range \
         owl:datatypeComplementOf takes the complement with respect to",
    ),
    inert(
        RDFS_LABEL,
        Shape::LiteralPredicate,
        "an annotation: OWL 2's Direct Semantics assigns annotations no meaning, so every \
         interpretation satisfies one",
    ),
    inert(RDFS_COMMENT, Shape::LiteralPredicate, "an annotation"),
    inert(RDFS_SEEALSO, Shape::IriPredicate, "an annotation"),
    inert(RDFS_ISDEFINEDBY, Shape::IriPredicate, "an annotation"),
    // --- OWL 2: class expressions ------------------------------------------------------
    handled(OWL_THING, Shape::ClassDenotation, "the top concept ⊤"),
    handled(OWL_NOTHING, Shape::ClassDenotation, "the bottom concept ⊥"),
    inert(
        OWL_CLASS,
        Shape::TypeObject,
        "a class declaration: it names a class without constraining any model",
    ),
    operand(
        OWL_RESTRICTION,
        Shape::TypeObject,
        "marks the node as an anonymous class expression; the expression itself is read \
         when a class axiom REFERENCES the node",
    ),
    operand(
        OWL_ONPROPERTY,
        Shape::RestrictionOperand,
        "the role of a restriction — an anonymous owl:inverseOf node here is the inverse \
         role r⁻; read from the restriction that carries it",
    ),
    handled(OWL_SOMEVALUESFROM, Shape::RestrictionIri, "∃r.C"),
    handled(OWL_ALLVALUESFROM, Shape::RestrictionIri, "∀r.C"),
    handled(OWL_HASVALUE, Shape::RestrictionIri, "∃r.{a}"),
    handled(
        OWL_HASSELF,
        Shape::RestrictionSelf,
        "∃r.Self for the boolean true, ¬∃r.Self for false",
    ),
    handled(OWL_INTERSECTIONOF, Shape::ClassExprList, "C₁ ⊓ … ⊓ Cₙ"),
    handled(OWL_UNIONOF, Shape::ClassExprList, "C₁ ⊔ … ⊔ Cₙ"),
    handled(OWL_COMPLEMENTOF, Shape::ClassExprIri, "¬C"),
    handled(
        OWL_ONEOF,
        Shape::ClassExprList,
        "the nominal {a₁,…,aₙ}, pre-split into a disjunction of singletons so the tableau's \
         o-rule only ever sees {a}; an owl:oneOf over LITERALS is a data range instead",
    ),
    handled(OWL_MINCARDINALITY, Shape::RestrictionLiteral, "≥n r.⊤"),
    handled(OWL_MAXCARDINALITY, Shape::RestrictionLiteral, "≤n r.⊤"),
    handled(
        OWL_CARDINALITY,
        Shape::RestrictionLiteral,
        "≥n r.⊤ ⊓ ≤n r.⊤",
    ),
    handled(
        OWL_MINQUALIFIEDCARDINALITY,
        Shape::RestrictionQualified,
        "≥n r.C",
    ),
    handled(
        OWL_MAXQUALIFIEDCARDINALITY,
        Shape::RestrictionQualified,
        "≤n r.C",
    ),
    handled(
        OWL_QUALIFIEDCARDINALITY,
        Shape::RestrictionQualified,
        "≥n r.C ⊓ ≤n r.C",
    ),
    operand(
        OWL_ONCLASS,
        Shape::RestrictionOperand,
        "the filler C of a qualified cardinality restriction, read from that restriction",
    ),
    // --- OWL 2: class axioms -----------------------------------------------------------
    handled(
        OWL_EQUIVALENTCLASS,
        Shape::IriPredicate,
        "the two GCIs C ⊑ D and D ⊑ C",
    ),
    handled(OWL_DISJOINTWITH, Shape::IriPredicate, "the GCI C ⊓ D ⊑ ⊥"),
    handled(
        OWL_ALLDISJOINTCLASSES,
        Shape::AxiomNode,
        "the pairwise GCIs Cᵢ ⊓ Cⱼ ⊑ ⊥ over the axiom's owl:members list",
    ),
    handled(
        OWL_DISJOINTUNIONOF,
        Shape::ListPredicate,
        "C ≡ C₁ ⊔ … ⊔ Cₙ together with the pairwise Cᵢ ⊓ Cⱼ ⊑ ⊥",
    ),
    // --- OWL 2: property axioms and characteristics ------------------------------------
    inert(
        OWL_OBJECTPROPERTY,
        Shape::TypeObject,
        "an object-property declaration: it names a role without constraining any model",
    ),
    inert(
        OWL_DATATYPEPROPERTY,
        Shape::TypeObject,
        "a data-property declaration; the property's own assertions are ingested as \
         abstract role edges over opaque literal terms",
    ),
    handled(
        OWL_EQUIVALENTPROPERTY,
        Shape::IriPredicate,
        "the two role inclusions r ⊑ s and s ⊑ r",
    ),
    handled(
        OWL_INVERSEOF,
        Shape::IriPredicate,
        "the inverse-role pairing r ≡ s⁻, closed over by the role hierarchy",
    ),
    handled(
        OWL_FUNCTIONALPROPERTY,
        Shape::TypeObject,
        "the global GCI ⊤ ⊑ ≤1 r.⊤",
    ),
    handled(
        OWL_INVERSEFUNCTIONALPROPERTY,
        Shape::TypeObject,
        "the global GCI ⊤ ⊑ ≤1 r⁻.⊤",
    ),
    handled(
        OWL_SYMMETRICPROPERTY,
        Shape::TypeObject,
        "r ≡ r⁻, recorded as the role being its own inverse",
    ),
    handled(
        OWL_ASYMMETRICPROPERTY,
        Shape::TypeObject,
        "the tableau clashes on a completion graph holding both an r-edge x→y and an \
         r-edge y→x, self-loops included",
    ),
    handled(
        OWL_TRANSITIVEPROPERTY,
        Shape::TypeObject,
        "the role's neighbourhood becomes its TRANSITIVE closure, so ∀r.C propagates along \
         a path; the role is also marked non-simple for the number-restriction check",
    ),
    handled(
        OWL_REFLEXIVEPROPERTY,
        Shape::TypeObject,
        "the global GCI ⊤ ⊑ ∃r.Self",
    ),
    handled(
        OWL_IRREFLEXIVEPROPERTY,
        Shape::TypeObject,
        "the global GCI ⊤ ⊑ ¬∃r.Self",
    ),
    handled(
        OWL_PROPERTYDISJOINTWITH,
        Shape::IriPredicate,
        "the tableau clashes on a completion graph where one pair x→y carries both roles",
    ),
    handled(
        OWL_ALLDISJOINTPROPERTIES,
        Shape::AxiomNode,
        "the pairwise role disjointness of the axiom's owl:members list",
    ),
    bounded(
        OWL_PROPERTYCHAINAXIOM,
        Shape::ListPredicate,
        Construct::PropertyChain,
    ),
    // --- OWL 2: individual axioms ------------------------------------------------------
    handled(
        OWL_NAMEDINDIVIDUAL,
        Shape::TypeObject,
        "an individual declaration, which is what makes the individual a tableau root and \
         a realization candidate",
    ),
    handled(
        OWL_SAMEAS,
        Shape::IriPredicate,
        "the identification a = b, applied as a node merge",
    ),
    handled(
        OWL_DIFFERENTFROM,
        Shape::IriPredicate,
        "the recorded inequality a ≠ b, which is what lets a ≤n restriction clash",
    ),
    handled(
        OWL_ALLDIFFERENT,
        Shape::AxiomNode,
        "the pairwise inequalities of the axiom's owl:members or owl:distinctMembers list",
    ),
    operand(
        OWL_MEMBERS,
        Shape::ListPredicate,
        "the operand list of owl:AllDifferent, owl:AllDisjointClasses and \
         owl:AllDisjointProperties, read from the typed axiom node that carries it",
    ),
    operand(
        OWL_DISTINCTMEMBERS,
        Shape::ListPredicate,
        "owl:AllDifferent's other operand list (the OWL 1 spelling), read the same way",
    ),
    handled(
        OWL_NEGATIVEPROPERTYASSERTION,
        Shape::NegativeAssertion,
        "the concept assertion s : ∀p.¬{o}, which is exactly ¬p(s, o)",
    ),
    operand(
        OWL_SOURCEINDIVIDUAL,
        Shape::NegativeAssertion,
        "the subject of a negative property assertion, read from the axiom node",
    ),
    operand(
        OWL_ASSERTIONPROPERTY,
        Shape::NegativeAssertion,
        "the property of a negative property assertion",
    ),
    operand(
        OWL_TARGETINDIVIDUAL,
        Shape::NegativeAssertion,
        "the object of a negative OBJECT-property assertion",
    ),
    operand(
        OWL_TARGETVALUE,
        Shape::NegativeAssertion,
        "the object of a negative DATA-property assertion; the literal is an opaque term",
    ),
    handled(
        OWL_HASKEY,
        Shape::ListPredicate,
        "the DL-safe key: two NAMED individuals entailed to be instances of the keyed class \
         that agree on every key property are identified",
    ),
    // --- OWL 2: data ranges (the concrete domain) --------------------------------------
    operand(
        OWL_ONDATATYPE,
        Shape::DatatypeRestriction,
        "the base datatype of a datatype restriction, read from the restriction node together \
         with its owl:withRestrictions facets",
    ),
    operand(
        OWL_WITHRESTRICTIONS,
        Shape::DatatypeRestriction,
        "the constraining facets of a datatype restriction — xsd:minInclusive, \
         xsd:maxInclusive, xsd:minExclusive, xsd:maxExclusive, xsd:length, xsd:minLength and \
         xsd:maxLength are intersected over the base datatype's value space; xsd:pattern and \
         rdf:langRange make the range undecidable here and raise the data-range boundary",
    ),
    handled(
        OWL_DATATYPECOMPLEMENTOF,
        Shape::DatatypeComplement,
        "the data range Δ_D ∖ DR — the complement taken with respect to the WHOLE data \
         domain, so a complement of rdfs:Literal is empty and a class forced into one is \
         unsatisfiable",
    ),
    operand(
        OWL_ONDATARANGE,
        Shape::DataCardinality,
        "the data range a qualified cardinality restriction counts over, read from that \
         restriction exactly as owl:onClass is for an object property",
    ),
    bounded(OWL_ONPROPERTIES, Shape::ClassExprList, Construct::DataRange),
    operand(
        OWL_DATARANGE,
        Shape::TypeObject,
        "OWL 1's spelling of rdfs:Datatype: it marks the node as a data range, and the range \
         is read when an axiom references the node",
    ),
    bounded(OWL_REAL, Shape::ClassDenotation, Construct::DataRange),
    bounded(OWL_RATIONAL, Shape::ClassDenotation, Construct::DataRange),
    // --- OWL 2: built-in roles ---------------------------------------------------------
    bounded(
        OWL_TOPOBJECTPROPERTY,
        Shape::RoleDenotation,
        Construct::BuiltinRole,
    ),
    bounded(
        OWL_BOTTOMOBJECTPROPERTY,
        Shape::RoleDenotation,
        Construct::BuiltinRole,
    ),
    bounded(
        OWL_TOPDATAPROPERTY,
        Shape::RoleDenotation,
        Construct::BuiltinRole,
    ),
    bounded(
        OWL_BOTTOMDATAPROPERTY,
        Shape::RoleDenotation,
        Construct::BuiltinRole,
    ),
    // --- OWL 2: the ontology header ----------------------------------------------------
    inert(
        OWL_ONTOLOGY,
        Shape::TypeObject,
        "the ontology header names the document, not a class or an individual",
    ),
    bounded(
        OWL_IMPORTS,
        Shape::IriPredicate,
        Construct::UnresolvedOntologyImport,
    ),
    inert(
        OWL_VERSIONIRI,
        Shape::IriPredicate,
        "the ontology's version identity — document metadata, not an axiom",
    ),
    inert(
        OWL_ONTOLOGYPROPERTY,
        Shape::TypeObject,
        "declares a header property, which carries no Direct-Semantics content",
    ),
    // --- OWL 2: annotations ------------------------------------------------------------
    inert(
        OWL_ANNOTATIONPROPERTY,
        Shape::TypeObject,
        "declares an annotation property; OWL 2's Direct Semantics assigns annotations no \
         meaning",
    ),
    inert(OWL_VERSIONINFO, Shape::LiteralPredicate, "an annotation"),
    inert(OWL_PRIORVERSION, Shape::IriPredicate, "an annotation"),
    inert(
        OWL_BACKWARDCOMPATIBLEWITH,
        Shape::IriPredicate,
        "an annotation",
    ),
    inert(OWL_INCOMPATIBLEWITH, Shape::IriPredicate, "an annotation"),
    inert(OWL_DEPRECATED, Shape::LiteralPredicate, "an annotation"),
    inert(
        OWL_AXIOM,
        Shape::TypeObject,
        "a reified axiom carries an annotation; the axiom itself is asserted separately, so \
         the reification adds nothing to it",
    ),
    inert(OWL_ANNOTATION, Shape::TypeObject, "a reified annotation"),
    inert(
        OWL_ANNOTATEDSOURCE,
        Shape::IriPredicate,
        "a reification component of owl:Axiom / owl:Annotation",
    ),
    inert(
        OWL_ANNOTATEDPROPERTY,
        Shape::IriPredicate,
        "a reification component of owl:Axiom / owl:Annotation",
    ),
    inert(
        OWL_ANNOTATEDTARGET,
        Shape::IriPredicate,
        "a reification component of owl:Axiom / owl:Annotation",
    ),
];

/// The [`Support`] of `iri`.
///
/// `None` for a term outside the [`RESERVED_NAMESPACES`] — a caller's own vocabulary, which
/// is read as an ordinary class, property or individual rather than as a construct. A
/// RESERVED term the table does not name answers `Some(Support::Bounded(UnrecognizedTerm))`,
/// so the function is total over the reserved vocabulary and cannot fall through to silence.
pub(crate) fn support_of(iri: &str) -> Option<Support> {
    if let Some(entry) = OWL2_CONSTRUCTS
        .iter()
        .find(|construct| construct.iri == iri)
    {
        return Some(entry.support);
    }
    is_reserved(iri).then_some(Support::Bounded(Construct::UnrecognizedTerm))
}

#[cfg(test)]
mod tests {
    use super::{
        OWL2_CONSTRUCTS, OwlConstruct, RESERVED_NAMESPACES, Shape, Support, is_reserved, support_of,
    };
    use crate::owl_dl::Kb;
    use crate::report::Construct;
    use crate::vocab::{
        OWL_MEMBERS, OWL_ONCLASS, OWL_ONPROPERTY, OWL_RESTRICTION, RDF_FIRST, RDF_NIL, RDF_REST,
        RDF_TYPE, RDFS_DATATYPE, RDFS_SUBCLASSOF, XSD_INTEGER, XSD_MININCLUSIVE,
        XSD_NONNEGATIVEINTEGER,
    };
    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};
    use std::collections::BTreeSet;
    use std::sync::Arc;

    /// Fixture terms are `example.org`: PurRDF mints no vocabulary, and a fixture that
    /// used a reserved IRI for its own data would be testing the table against itself.
    const EX_A: &str = "http://example.org/a";
    /// A fixture individual, and the second member of every fixture list.
    const EX_B: &str = "http://example.org/b";
    /// A fixture class.
    const EX_C: &str = "http://example.org/C";
    /// A fixture property.
    const EX_P: &str = "http://example.org/p";
    /// `xsd:boolean`, the datatype `owl:hasSelf`'s value carries.
    const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

    /// A small builder wrapper that keeps the fixture construction readable.
    struct Fixture {
        builder: RdfDatasetBuilder,
        cells: usize,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                builder: RdfDatasetBuilder::new(),
                cells: 0,
            }
        }

        fn iri(&mut self, iri: &str) -> TermId {
            self.builder.intern_iri(iri)
        }

        fn blank(&mut self, label: &str) -> TermId {
            self.builder.intern_blank(label, BlankScope::DEFAULT)
        }

        fn literal(&mut self, lexical: &str, datatype: &str) -> TermId {
            let value = TermValue::typed_literal(lexical, datatype);
            crate::interner::intern_into(&mut self.builder, &value)
        }

        fn quad(&mut self, s: TermId, p: TermId, o: TermId) {
            self.builder.push_quad(s, p, o, None);
        }

        /// Write `members` as an RDF collection, returning its head.
        fn list(&mut self, members: &[TermId]) -> TermId {
            let first = self.iri(RDF_FIRST);
            let rest = self.iri(RDF_REST);
            let mut head = self.iri(RDF_NIL);
            for &member in members.iter().rev() {
                self.cells += 1;
                let cell = self.blank(&format!("cell{}", self.cells));
                self.quad(cell, first, member);
                self.quad(cell, rest, head);
                head = cell;
            }
            head
        }

        fn freeze(self) -> Arc<RdfDataset> {
            self.builder.freeze().expect("the fixture freezes")
        }
    }

    /// The minimal graph that exercises `construct`, built from its [`Shape`].
    ///
    /// Every shape produces a graph in which the construct is actually REACHED: a
    /// restriction is referenced by a class axiom, a class expression is referenced by a
    /// `rdfs:subClassOf`, an n-ary axiom node carries its operand list. A fixture that
    /// merely mentioned the term would prove nothing, because a class expression nothing
    /// references states nothing.
    fn fixture(construct: &OwlConstruct) -> Arc<RdfDataset> {
        let mut f = Fixture::new();
        let term = f.iri(construct.iri);
        let a = f.iri(EX_A);
        let b = f.iri(EX_B);
        let c = f.iri(EX_C);
        let p = f.iri(EX_P);
        let ty = f.iri(RDF_TYPE);
        let sub_class = f.iri(RDFS_SUBCLASSOF);
        match construct.shape {
            Shape::IriPredicate | Shape::RoleDenotation => f.quad(a, term, b),
            Shape::LiteralPredicate => {
                let value = f.literal("1", XSD_NONNEGATIVEINTEGER);
                f.quad(a, term, value);
            }
            Shape::ListPredicate => {
                let head = f.list(&[b, c]);
                f.quad(a, term, head);
            }
            Shape::TypeObject => f.quad(a, ty, term),
            Shape::AxiomNode => {
                let node = f.blank("axiom");
                let members = f.iri(OWL_MEMBERS);
                let head = f.list(&[b, c]);
                f.quad(node, ty, term);
                f.quad(node, members, head);
            }
            Shape::NegativeAssertion => {
                let node = f.blank("npa");
                let npa = f.iri(super::OWL_NEGATIVEPROPERTYASSERTION);
                let source = f.iri(super::OWL_SOURCEINDIVIDUAL);
                let property = f.iri(super::OWL_ASSERTIONPROPERTY);
                let target = f.iri(super::OWL_TARGETINDIVIDUAL);
                f.quad(node, ty, npa);
                f.quad(node, source, a);
                f.quad(node, property, p);
                f.quad(node, target, b);
            }
            Shape::ClassDenotation => f.quad(a, sub_class, term),
            Shape::RestrictionIri
            | Shape::RestrictionLiteral
            | Shape::RestrictionQualified
            | Shape::RestrictionOperand
            | Shape::RestrictionSelf => {
                let node = f.blank("restriction");
                let restriction = f.iri(OWL_RESTRICTION);
                let on_property = f.iri(OWL_ONPROPERTY);
                f.quad(a, sub_class, node);
                f.quad(node, ty, restriction);
                f.quad(node, on_property, p);
                match construct.shape {
                    Shape::RestrictionIri => f.quad(node, term, c),
                    // The two components are already stated above, so the fixture only has
                    // to complete the restriction into one this layer decodes.
                    Shape::RestrictionOperand => {
                        let qualified = f.iri(super::OWL_MINQUALIFIEDCARDINALITY);
                        let on_class = f.iri(OWL_ONCLASS);
                        let value = f.literal("1", XSD_NONNEGATIVEINTEGER);
                        f.quad(node, qualified, value);
                        f.quad(node, on_class, c);
                    }
                    Shape::RestrictionSelf => {
                        let value = f.literal("true", XSD_BOOLEAN);
                        f.quad(node, term, value);
                    }
                    _ => {
                        let value = f.literal("1", XSD_NONNEGATIVEINTEGER);
                        f.quad(node, term, value);
                        if construct.shape == Shape::RestrictionQualified {
                            let on_class = f.iri(OWL_ONCLASS);
                            f.quad(node, on_class, c);
                        }
                    }
                }
            }
            Shape::ClassExprList | Shape::Collection => {
                let node = f.blank("expr");
                let members: &[TermId] = if construct.shape == Shape::Collection {
                    &[b]
                } else {
                    &[b, c]
                };
                let head = f.list(members);
                f.quad(a, sub_class, node);
                if construct.shape == Shape::Collection {
                    // The collection vocabulary is only reachable through an axiom that
                    // WALKS a list, so the fixture states one and the walk is the test.
                    let intersection = f.iri(super::OWL_INTERSECTIONOF);
                    f.quad(node, intersection, head);
                } else {
                    f.quad(node, term, head);
                }
            }
            Shape::ClassExprIri => {
                let node = f.blank("expr");
                f.quad(a, sub_class, node);
                f.quad(node, term, b);
            }
            Shape::DatatypeRestriction | Shape::DatatypeComplement | Shape::DataCardinality => {
                let node = f.blank("restriction");
                let restriction = f.iri(OWL_RESTRICTION);
                let on_property = f.iri(OWL_ONPROPERTY);
                let datatype = f.iri(RDFS_DATATYPE);
                let integer = f.iri(XSD_INTEGER);
                f.quad(a, sub_class, node);
                f.quad(node, ty, restriction);
                f.quad(node, on_property, p);
                match construct.shape {
                    Shape::DatatypeRestriction => {
                        let range = f.blank("range");
                        let some = f.iri(super::OWL_SOMEVALUESFROM);
                        let on_datatype = f.iri(super::OWL_ONDATATYPE);
                        let with_restrictions = f.iri(super::OWL_WITHRESTRICTIONS);
                        let facet = f.blank("facet");
                        let min_inclusive = f.iri(XSD_MININCLUSIVE);
                        let one = f.literal("1", XSD_INTEGER);
                        f.quad(facet, min_inclusive, one);
                        let head = f.list(&[facet]);
                        f.quad(range, ty, datatype);
                        f.quad(range, on_datatype, integer);
                        f.quad(range, with_restrictions, head);
                        f.quad(node, some, range);
                    }
                    Shape::DatatypeComplement => {
                        let range = f.blank("range");
                        let some = f.iri(super::OWL_SOMEVALUESFROM);
                        let complement = f.iri(super::OWL_DATATYPECOMPLEMENTOF);
                        f.quad(range, ty, datatype);
                        f.quad(range, complement, integer);
                        f.quad(node, some, range);
                    }
                    // `Shape::DataCardinality`.
                    _ => {
                        let qualified = f.iri(super::OWL_MINQUALIFIEDCARDINALITY);
                        let on_data_range = f.iri(super::OWL_ONDATARANGE);
                        let value = f.literal("1", XSD_NONNEGATIVEINTEGER);
                        f.quad(node, qualified, value);
                        f.quad(node, on_data_range, integer);
                    }
                }
            }
        }
        f.freeze()
    }

    /// How much of the knowledge base a fixture filled: every axiom store, summed.
    ///
    /// Zero means the parse recorded NOTHING, which is exactly what a
    /// [`Support::Handled`] entry must not do.
    fn axiom_count(kb: &Kb) -> usize {
        kb.tbox.len()
            + kb.meta.len()
            + kb.unfold.len()
            + kb.inverses.len()
            + kb.role_sub.len()
            + kb.abox_types.len()
            + kb.abox_roles.len()
            + kb.same_as.len()
            + kb.different_from.len()
            + kb.individuals.len()
            + kb.transitive.len()
            + kb.asymmetric.len()
            + kb.disjoint_roles.len()
            + kb.keys.len()
    }

    /// THE INVENTORY GATE: every OWL 2 construct either reaches the knowledge base or
    /// yields a NAMED boundary, and a construct that is neither fails here.
    ///
    /// Each entry's own minimal graph is driven through the real parser and the outcome is
    /// checked against the entry's [`Support`]:
    ///
    /// * [`Support::Handled`] — no boundary, and the knowledge base GREW. This is the
    ///   assertion that catches the old failure mode directly: an axiom that is parsed and
    ///   thrown away leaves the knowledge base empty and fails here.
    /// * [`Support::Operand`] — no boundary. An operand states nothing on its own, so it is
    ///   not required to grow anything; what it may not do is go unrecognized.
    /// * [`Support::Inert`] — no boundary, for the same reason.
    /// * [`Support::Bounded`] — the boundary the entry names is RAISED. Not any boundary:
    ///   the one it claims.
    #[test]
    fn every_owl2_construct_is_handled_or_bounded() {
        for construct in OWL2_CONSTRUCTS {
            let dataset = fixture(construct);
            let kb = Kb::from_dataset(&dataset).unwrap_or_else(|e| {
                panic!("{}: the minimal fixture must parse: {e}", construct.iri)
            });
            let boundaries = kb.boundaries();
            match construct.support {
                Support::Handled(note) => {
                    assert!(
                        boundaries.is_empty(),
                        "{} claims to be handled as {note:?} but raised {boundaries:?}",
                        construct.iri
                    );
                    assert!(
                        axiom_count(&kb) > 0,
                        "{} claims to be handled as {note:?}, and its minimal fixture \
                         reached NOTHING in the knowledge base — which is exactly the \
                         silent drop this table exists to forbid",
                        construct.iri
                    );
                }
                Support::Operand(_) | Support::Inert(_) => {
                    assert!(
                        boundaries.is_empty(),
                        "{} is recognized, so its fixture must raise no boundary; it \
                         raised {boundaries:?}",
                        construct.iri
                    );
                }
                Support::Bounded(expected) => {
                    assert!(
                        boundaries.contains(&expected),
                        "{} claims the {expected} boundary; the run raised {boundaries:?}",
                        construct.iri
                    );
                }
            }
        }
    }

    /// A RESERVED term nobody listed is a NAMED boundary, driven through the real parser —
    /// not merely a table lookup. This is the fallthrough the parser used to answer
    /// `Ok(())` for.
    #[test]
    fn an_unrecognized_reserved_term_raises_a_boundary_rather_than_ok() {
        for namespace in RESERVED_NAMESPACES {
            let invented = format!("{namespace}purrdfNoSuchTerm");
            // As a PREDICATE — the position the old catch-all mis-ingested as a role.
            let mut f = Fixture::new();
            let a = f.iri(EX_A);
            let term = f.iri(&invented);
            let b = f.iri(EX_B);
            f.quad(a, term, b);
            let kb = Kb::from_dataset(&f.freeze()).expect("parse");
            assert!(
                kb.boundaries().contains(&Construct::UnrecognizedTerm),
                "{invented} in predicate position must raise a boundary"
            );
            assert!(
                kb.abox_roles.is_empty(),
                "{invented} must NOT be ingested as a role assertion"
            );

            // As a CLASS — the position an opaque reading would silently admit.
            let mut f = Fixture::new();
            let a = f.iri(EX_A);
            let sub_class = f.iri(RDFS_SUBCLASSOF);
            let term = f.iri(&invented);
            f.quad(a, sub_class, term);
            let kb = Kb::from_dataset(&f.freeze()).expect("parse");
            assert!(
                kb.boundaries().contains(&Construct::UnrecognizedTerm),
                "{invented} in class position must raise a boundary"
            );
        }
    }

    /// A caller's OWN vocabulary is still ordinary data: the reserved-namespace split must
    /// not have turned every predicate into a boundary.
    #[test]
    fn user_vocabulary_is_read_as_data_and_raises_nothing() {
        let mut f = Fixture::new();
        let a = f.iri(EX_A);
        let p = f.iri(EX_P);
        let b = f.iri(EX_B);
        f.quad(a, p, b);
        let kb = Kb::from_dataset(&f.freeze()).expect("parse");
        assert!(kb.boundaries().is_empty(), "{:?}", kb.boundaries());
        assert_eq!(kb.abox_roles.len(), 1);
    }

    /// No IRI is listed twice — a second entry would silently shadow the first.
    #[test]
    fn the_inventory_names_each_term_once() {
        let iris: BTreeSet<&str> = OWL2_CONSTRUCTS.iter().map(|c| c.iri).collect();
        assert_eq!(
            iris.len(),
            OWL2_CONSTRUCTS.len(),
            "the OWL 2 construct inventory repeats an IRI"
        );
    }

    /// Every listed IRI is RESERVED. A caller's own vocabulary is not a construct, and
    /// listing one here would make an ordinary property unreadable as data.
    #[test]
    fn every_listed_term_is_reserved() {
        for construct in OWL2_CONSTRUCTS {
            assert!(
                is_reserved(construct.iri),
                "{} is not in a reserved namespace",
                construct.iri
            );
        }
    }

    /// [`support_of`] is TOTAL over the reserved vocabulary and silent outside it.
    #[test]
    fn support_is_total_over_the_reserved_vocabulary_and_only_there() {
        for construct in OWL2_CONSTRUCTS {
            assert_eq!(
                support_of(construct.iri),
                Some(construct.support),
                "{}",
                construct.iri
            );
        }
        for namespace in RESERVED_NAMESPACES {
            let invented = format!("{namespace}purrdfNoSuchTerm");
            assert_eq!(
                support_of(&invented),
                Some(Support::Bounded(Construct::UnrecognizedTerm)),
                "{invented}"
            );
        }
        assert_eq!(support_of(EX_P), None);
    }

    /// Every bounded entry names one of the six OWL-Direct boundary constructs — never one
    /// of the five the forward chase raises, which are about a different engine entirely.
    #[test]
    fn bounded_entries_name_an_owl_direct_construct() {
        let owl_direct = [
            Construct::PropertyChain,
            Construct::NonSimpleRole,
            Construct::DataRange,
            Construct::BuiltinRole,
            Construct::UnresolvedOntologyImport,
            Construct::UnrecognizedTerm,
        ];
        let mut seen: BTreeSet<Construct> = BTreeSet::new();
        for construct in OWL2_CONSTRUCTS {
            if let Support::Bounded(boundary) = construct.support {
                assert!(
                    owl_direct.contains(&boundary),
                    "{} raises {boundary}, which is not an OWL-Direct boundary",
                    construct.iri
                );
                seen.insert(boundary);
            }
        }
        // `NonSimpleRole` and `UnrecognizedTerm` are raised by a CONDITION rather than by a
        // term, so they are the two that cannot appear in the table; every other
        // OWL-Direct boundary must have at least one term that raises it, or it is a
        // boundary nothing can reach.
        for boundary in [
            Construct::PropertyChain,
            Construct::DataRange,
            Construct::BuiltinRole,
            Construct::UnresolvedOntologyImport,
        ] {
            assert!(seen.contains(&boundary), "no term raises {boundary}");
        }
    }
}
