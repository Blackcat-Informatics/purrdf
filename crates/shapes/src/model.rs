// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Namespace IRI constants for SHACL, RDF, RDFS, and XSD.
//!
//! All constants are plain `&'static str` IRIs. Native query patterns key on the
//! IRI string directly; constructors like [`crate::term::NamedNode::from`] wrap one
//! into a term when a value is needed.

/// SHACL namespace constants (`http://www.w3.org/ns/shacl#`).
pub mod sh {
    pub const CONFORMS: &str = "http://www.w3.org/ns/shacl#conforms";

    pub const VALIDATION_REPORT: &str = "http://www.w3.org/ns/shacl#ValidationReport";

    pub const VALIDATION_RESULT: &str = "http://www.w3.org/ns/shacl#ValidationResult";

    pub const RESULT: &str = "http://www.w3.org/ns/shacl#result";

    pub const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl#focusNode";

    pub const RESULT_PATH: &str = "http://www.w3.org/ns/shacl#resultPath";

    pub const VALUE: &str = "http://www.w3.org/ns/shacl#value";

    pub const RESULT_SEVERITY: &str = "http://www.w3.org/ns/shacl#resultSeverity";

    pub const RESULT_MESSAGE: &str = "http://www.w3.org/ns/shacl#resultMessage";

    pub const SOURCE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#sourceConstraintComponent";

    pub const SOURCE_SHAPE: &str = "http://www.w3.org/ns/shacl#sourceShape";

    pub const VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";

    pub const WARNING: &str = "http://www.w3.org/ns/shacl#Warning";

    pub const INFO: &str = "http://www.w3.org/ns/shacl#Info";

    // ── Shape type terms ───────────────────────────────────────────────────────

    pub const NODE_SHAPE: &str = "http://www.w3.org/ns/shacl#NodeShape";

    pub const PROPERTY_SHAPE: &str = "http://www.w3.org/ns/shacl#PropertyShape";

    // ── Target predicates ──────────────────────────────────────────────────────

    pub const TARGET_CLASS: &str = "http://www.w3.org/ns/shacl#targetClass";

    pub const TARGET_SUBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetSubjectsOf";

    pub const TARGET_OBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetObjectsOf";

    pub const TARGET_NODE: &str = "http://www.w3.org/ns/shacl#targetNode";

    // ── Property shape plumbing ────────────────────────────────────────────────

    pub const PROPERTY: &str = "http://www.w3.org/ns/shacl#property";

    pub const PATH: &str = "http://www.w3.org/ns/shacl#path";

    pub const INVERSE_PATH: &str = "http://www.w3.org/ns/shacl#inversePath";

    // ── Composite path forms (§2.3.1) ──────────────────────────────────────────

    pub const ALTERNATIVE_PATH: &str = "http://www.w3.org/ns/shacl#alternativePath";

    pub const ZERO_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrMorePath";

    pub const ONE_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#oneOrMorePath";

    pub const ZERO_OR_ONE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrOnePath";

    // ── Constraint predicates (supported) ─────────────────────────────────────

    pub const CLASS: &str = "http://www.w3.org/ns/shacl#class";

    pub const DATATYPE: &str = "http://www.w3.org/ns/shacl#datatype";

    pub const NODE_KIND: &str = "http://www.w3.org/ns/shacl#nodeKind";

    pub const MIN_COUNT: &str = "http://www.w3.org/ns/shacl#minCount";

    pub const MAX_COUNT: &str = "http://www.w3.org/ns/shacl#maxCount";

    pub const IN: &str = "http://www.w3.org/ns/shacl#in";

    pub const HAS_VALUE: &str = "http://www.w3.org/ns/shacl#hasValue";

    pub const PATTERN: &str = "http://www.w3.org/ns/shacl#pattern";

    pub const FLAGS: &str = "http://www.w3.org/ns/shacl#flags";

    pub const MIN_LENGTH: &str = "http://www.w3.org/ns/shacl#minLength";

    pub const UNIQUE_LANG: &str = "http://www.w3.org/ns/shacl#uniqueLang";

    pub const MIN_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#minInclusive";

    pub const MAX_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxInclusive";

    pub const AND: &str = "http://www.w3.org/ns/shacl#and";

    pub const OR: &str = "http://www.w3.org/ns/shacl#or";

    pub const XONE: &str = "http://www.w3.org/ns/shacl#xone";

    pub const NODE: &str = "http://www.w3.org/ns/shacl#node";

    pub const REIFIER_SHAPE: &str = "http://www.w3.org/ns/shacl#reifierShape";

    pub const REIFICATION_REQUIRED: &str = "http://www.w3.org/ns/shacl#reificationRequired";

    // ── Shape metadata (benign, not constraints) ───────────────────────────────

    pub const SEVERITY: &str = "http://www.w3.org/ns/shacl#severity";

    pub const MESSAGE: &str = "http://www.w3.org/ns/shacl#message";

    pub const DEACTIVATED: &str = "http://www.w3.org/ns/shacl#deactivated";

    pub const NAME: &str = "http://www.w3.org/ns/shacl#name";

    pub const DESCRIPTION: &str = "http://www.w3.org/ns/shacl#description";

    pub const ORDER: &str = "http://www.w3.org/ns/shacl#order";

    pub const GROUP: &str = "http://www.w3.org/ns/shacl#group";

    // ── sh:nodeKind value IRIs ─────────────────────────────────────────────────

    pub const IRI: &str = "http://www.w3.org/ns/shacl#IRI";

    pub const BLANK_NODE: &str = "http://www.w3.org/ns/shacl#BlankNode";

    pub const LITERAL: &str = "http://www.w3.org/ns/shacl#Literal";

    pub const BLANK_NODE_OR_IRI: &str = "http://www.w3.org/ns/shacl#BlankNodeOrIRI";

    pub const BLANK_NODE_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#BlankNodeOrLiteral";

    pub const IRI_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#IRIOrLiteral";

    // ── SHACL-AF and advanced constraint predicates ────────────────────────────

    pub const SPARQL: &str = "http://www.w3.org/ns/shacl#sparql";

    pub const TARGET: &str = "http://www.w3.org/ns/shacl#target";

    pub const QUALIFIED_VALUE_SHAPE: &str = "http://www.w3.org/ns/shacl#qualifiedValueShape";

    pub const QUALIFIED_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMinCount";

    pub const QUALIFIED_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMaxCount";

    pub const QUALIFIED_VALUE_SHAPES_DISJOINT: &str =
        "http://www.w3.org/ns/shacl#qualifiedValueShapesDisjoint";

    pub const LESS_THAN: &str = "http://www.w3.org/ns/shacl#lessThan";

    pub const LESS_THAN_OR_EQUALS: &str = "http://www.w3.org/ns/shacl#lessThanOrEquals";

    pub const EQUALS: &str = "http://www.w3.org/ns/shacl#equals";

    pub const DISJOINT: &str = "http://www.w3.org/ns/shacl#disjoint";

    pub const NOT: &str = "http://www.w3.org/ns/shacl#not";

    pub const CLOSED: &str = "http://www.w3.org/ns/shacl#closed";

    pub const IGNORED_PROPERTIES: &str = "http://www.w3.org/ns/shacl#ignoredProperties";

    pub const LANGUAGE_IN: &str = "http://www.w3.org/ns/shacl#languageIn";

    pub const MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#maxLength";

    pub const MIN_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#minExclusive";

    pub const MAX_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxExclusive";

    pub const SELECT: &str = "http://www.w3.org/ns/shacl#select";

    /// `sh:ask` — the ASK query of a SHACL-SPARQL validator.
    pub const ASK: &str = "http://www.w3.org/ns/shacl#ask";

    // ── SHACL-AF prefix declarations (sh:prefixes / sh:declare) ───────────────

    pub const PREFIXES: &str = "http://www.w3.org/ns/shacl#prefixes";

    pub const DECLARE: &str = "http://www.w3.org/ns/shacl#declare";

    pub const PREFIX: &str = "http://www.w3.org/ns/shacl#prefix";

    pub const NAMESPACE: &str = "http://www.w3.org/ns/shacl#namespace";

    pub const SPARQL_CONSTRAINT: &str = "http://www.w3.org/ns/shacl#SPARQLConstraint";

    pub const SPARQL_TARGET: &str = "http://www.w3.org/ns/shacl#SPARQLTarget";

    pub const SPARQL_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#SPARQLConstraintComponent";

    // ── SHACL-AF node expressions (§node expressions) ─────────────────────────

    pub const EXPRESSION: &str = "http://www.w3.org/ns/shacl#expression";

    pub const EXPRESSION_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ExpressionConstraintComponent";

    pub const THIS: &str = "http://www.w3.org/ns/shacl#this";

    pub const FILTER_SHAPE: &str = "http://www.w3.org/ns/shacl#filterShape";

    pub const NODES: &str = "http://www.w3.org/ns/shacl#nodes";

    pub const UNION: &str = "http://www.w3.org/ns/shacl#union";

    pub const INTERSECTION: &str = "http://www.w3.org/ns/shacl#intersection";

    pub const IF: &str = "http://www.w3.org/ns/shacl#if";

    pub const THEN: &str = "http://www.w3.org/ns/shacl#then";

    pub const ELSE: &str = "http://www.w3.org/ns/shacl#else";

    pub const COUNT: &str = "http://www.w3.org/ns/shacl#count";

    pub const DISTINCT: &str = "http://www.w3.org/ns/shacl#distinct";

    pub const MIN: &str = "http://www.w3.org/ns/shacl#min";

    pub const MAX: &str = "http://www.w3.org/ns/shacl#max";

    pub const SUM: &str = "http://www.w3.org/ns/shacl#sum";

    pub const LIMIT: &str = "http://www.w3.org/ns/shacl#limit";

    pub const OFFSET: &str = "http://www.w3.org/ns/shacl#offset";

    pub const ORDERBY: &str = "http://www.w3.org/ns/shacl#orderby";

    /// The adopted PurRDF DASH-extension direction flag for `sh:orderby`
    /// (boolean; `true` ⇒ descending, default ascending).
    pub const DESC: &str = "http://www.w3.org/ns/shacl#desc";

    pub const EXISTS: &str = "http://www.w3.org/ns/shacl#exists";

    pub const SPARQL_FUNCTION: &str = "http://www.w3.org/ns/shacl#SPARQLFunction";

    pub const FUNCTION: &str = "http://www.w3.org/ns/shacl#Function";

    // ── Custom constraint-component vocabulary ───────────────────────────────

    pub const CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#ConstraintComponent";

    pub const PARAMETER: &str = "http://www.w3.org/ns/shacl#Parameter";

    pub const PARAMETER_PROPERTY: &str = "http://www.w3.org/ns/shacl#parameter";

    pub const NODE_VALIDATOR: &str = "http://www.w3.org/ns/shacl#nodeValidator";

    pub const PROPERTY_VALIDATOR: &str = "http://www.w3.org/ns/shacl#propertyValidator";

    pub const VALIDATOR: &str = "http://www.w3.org/ns/shacl#validator";

    pub const OPTIONAL: &str = "http://www.w3.org/ns/shacl#optional";

    pub const SPARQL_ASK_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLAskValidator";

    pub const SPARQL_SELECT_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLSelectValidator";

    // ── Constraint component IRIs (sh:*ConstraintComponent) ──────────────────

    pub const MIN_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinCountConstraintComponent";

    pub const MAX_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxCountConstraintComponent";

    pub const CLASS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ClassConstraintComponent";

    pub const DATATYPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#DatatypeConstraintComponent";

    pub const NODE_KIND_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeKindConstraintComponent";

    pub const IN_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#InConstraintComponent";

    pub const HAS_VALUE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#HasValueConstraintComponent";

    pub const PATTERN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#PatternConstraintComponent";

    pub const MIN_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinLengthConstraintComponent";

    pub const UNIQUE_LANG_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#UniqueLangConstraintComponent";

    pub const MIN_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent";

    pub const MAX_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent";

    pub const MIN_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent";

    pub const MAX_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent";

    pub const AND_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#AndConstraintComponent";

    pub const OR_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#OrConstraintComponent";

    pub const XONE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#XoneConstraintComponent";

    pub const NODE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeConstraintComponent";

    pub const REIFIER_SHAPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ReifierShapeConstraintComponent";

    pub const MAX_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent";

    pub const NOT_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#NotConstraintComponent";

    pub const LANGUAGE_IN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LanguageInConstraintComponent";

    pub const CLOSED_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ClosedConstraintComponent";

    pub const EQUALS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#EqualsConstraintComponent";

    pub const DISJOINT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#DisjointConstraintComponent";

    pub const LESS_THAN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LessThanConstraintComponent";

    pub const LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent";

    pub const QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent";

    pub const QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#QualifiedMaxCountConstraintComponent";
}

/// The CALLER-SUPPLIED graph-box role vocabulary for the OPTIONAL box-role
/// annotation feature.
///
/// PurRDF is not an ontology and mints no vocabulary IRIs of its own, so there
/// is deliberately NO default (mirroring the `LanguageVocab` /
/// `StatementMetadataVocab` pattern elsewhere in the workspace): a shapes
/// parse / validation run without a configured vocab leaves the box-role
/// feature INACTIVE — every box-role list on shapes and validation results
/// stays empty.
///
/// Configure it through
/// [`from_dataset_with_config`](crate::shapes::from_dataset_with_config) /
/// [`parse_shapes_with_config`](crate::engine::parse_shapes_with_config); the
/// parsed [`Shapes`](crate::shapes::Shapes) carries it into validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoxRoleVocab {
    /// The predicate annotating a shape node or data-graph predicate with its
    /// graph-box role.
    pub graph_box_role: String,
    /// The ABox role individual.
    pub box_abox: String,
    /// The TBox role individual.
    pub box_tbox: String,
    /// The RBox role individual.
    pub box_rbox: String,
    /// The CBox role individual (stamped onto reifier-shape results).
    pub box_cbox: String,
    /// The ConfigBox role individual.
    pub box_config_box: String,
}

impl BoxRoleVocab {
    /// Derive the six term IRIs by concatenation from a namespace whose local
    /// names are `graphBoxRole` / `boxABox` / `boxTBox` / `boxRBox` / `boxCBox`
    /// / `boxConfigBox`.
    #[must_use]
    pub fn for_namespace(ns: &str) -> Self {
        Self {
            graph_box_role: format!("{ns}graphBoxRole"),
            box_abox: format!("{ns}boxABox"),
            box_tbox: format!("{ns}boxTBox"),
            box_rbox: format!("{ns}boxRBox"),
            box_cbox: format!("{ns}boxCBox"),
            box_config_box: format!("{ns}boxConfigBox"),
        }
    }
}

/// RDF namespace constants (`http://www.w3.org/1999/02/22-rdf-syntax-ns#`).
pub mod rdf {
    pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

    pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";

    pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";

    pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

    pub const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
}

/// RDFS namespace constants.
pub mod rdfs {
    pub const BASE: &str = "http://www.w3.org/2000/01/rdf-schema#";

    pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";

    pub const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
}

/// XSD namespace base string.
pub mod xsd {
    pub const BASE: &str = "http://www.w3.org/2001/XMLSchema#";

    pub const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

    pub const STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

    pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
}
