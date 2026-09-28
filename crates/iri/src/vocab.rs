// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The W3C vocabulary IRIs the workspace reads and writes, spelled once.
//!
//! Every constant here is a term of a **W3C-published** vocabulary — RDF,
//! RDFS, OWL 2, SHACL and SKOS — and PurRDF mints none of its own (repo law:
//! *PurRDF is NOT an ontology*). The module exists so that a crate that needs
//! `rdf:type` reaches for [`rdf::TYPE`] instead of carrying a private copy of
//! the string, which is how the workspace came to hold a dozen spellings of
//! the same twenty IRIs.
//!
//! Each namespace is a submodule whose `NS` is the namespace IRI and whose
//! other constants are its terms under their local names in `UPPER_SNAKE`
//! case — `owl:sameAs` is [`owl::SAME_AS`], `sh:NodeShape` is
//! [`sh::NODE_SHAPE`] — so a caller writes `use purrdf_iri::vocab::rdf;` and
//! then `rdf::TYPE`. Where two SHACL local names differ only in case
//! (`sh:rule`/`sh:Rule`, `sh:parameter`/`sh:Parameter`,
//! `sh:resultAnnotation`/`sh:ResultAnnotation`) the class carries a `_CLASS`
//! suffix or the property a `_PROPERTY` suffix, as the shapes crate already
//! spelled them.
//!
//! Every term constant is built by concatenation from its namespace's `NS`,
//! so a term cannot drift from its namespace, and each submodule's `TERMS`
//! roster lists every term as `(local name, IRI)` for a test or a compactor
//! that needs the closed set rather than one member.
//!
//! ```rust
//! use purrdf_iri::vocab::{owl, rdf, rdfs, sh, RDF_NS};
//!
//! assert_eq!(rdf::TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
//! assert_eq!(rdfs::SUB_CLASS_OF, "http://www.w3.org/2000/01/rdf-schema#subClassOf");
//! assert_eq!(owl::SAME_AS, "http://www.w3.org/2002/07/owl#sameAs");
//! assert_eq!(sh::NODE_SHAPE, "http://www.w3.org/ns/shacl#NodeShape");
//! assert_eq!(RDF_NS, rdf::NS);
//! assert!(rdf::TERMS.iter().any(|&(local, iri)| local == "type" && iri == rdf::TYPE));
//! ```
//!
//! Beside the tables sit the two pieces of RDF 1.2 literal law that every
//! codec answers from these terms: [`language_datatype_iri`], the datatype a
//! language tag (and optional base direction) implies, and
//! [`TextDirection`], the `ltr`/`rtl` token of a directional literal.

use core::fmt;

/// Define one namespace module body: its `NS`, one documented constant per
/// term built by concatenation from `NS`, and the `TERMS` roster.
///
/// The namespace IRI is written exactly once, at the `ns` line; every term
/// is `concat!` of that literal and its local name, so the roster and the
/// constants are projections of one spelling.
macro_rules! namespace {
    (
        prefix $prefix:literal;
        ns $ns:literal;
        $(
            $(#[$meta:meta])*
            $name:ident = $local:literal;
        )*
    ) => {
        #[doc = concat!("The `", $prefix, ":` namespace IRI.")]
        pub const NS: &str = $ns;

        $(
            #[doc = concat!("`", $prefix, ":", $local, "`")]
            $(#[$meta])*
            pub const $name: &str = concat!($ns, $local);
        )*

        /// Every term of this namespace as `(local name, IRI)`, in
        /// declaration order — the closed roster for a test or a compactor
        /// that needs the set rather than one member.
        pub const TERMS: &[(&str, &str)] = &[ $( ($local, $name) ),* ];
    };
}

/// The RDF namespace, `http://www.w3.org/1999/02/22-rdf-syntax-ns#`.
pub const RDF_NS: &str = rdf::NS;
/// The RDF Schema namespace, `http://www.w3.org/2000/01/rdf-schema#`.
pub const RDFS_NS: &str = rdfs::NS;
/// The OWL 2 namespace, `http://www.w3.org/2002/07/owl#`.
pub const OWL_NS: &str = owl::NS;
/// The SHACL namespace, `http://www.w3.org/ns/shacl#`.
pub const SH_NS: &str = sh::NS;
/// The SKOS namespace, `http://www.w3.org/2004/02/skos/core#`.
pub const SKOS_NS: &str = skos::NS;

/// The RDF vocabulary (RDF 1.2 Concepts §5 and RDF 1.2 Schema §5).
pub mod rdf {
    namespace! {
        prefix "rdf";
        ns "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
        TYPE = "type";
        FIRST = "first";
        REST = "rest";
        NIL = "nil";
        /// RDF 1.2's reifier property.
        REIFIES = "reifies";
        /// The datatype of a language-tagged string without a base direction.
        LANG_STRING = "langString";
        /// The datatype of a language-tagged string with a base direction
        /// (new in RDF 1.2).
        DIR_LANG_STRING = "dirLangString";
        JSON = "JSON";
        HTML = "HTML";
        XML_LITERAL = "XMLLiteral";
        /// OWL 2 RL's plain-literal datatype.
        PLAIN_LITERAL = "PlainLiteral";
        /// The language-range facet over `rdf:langString`.
        LANG_RANGE = "langRange";
        VALUE = "value";
        SUBJECT = "subject";
        PREDICATE = "predicate";
        OBJECT = "object";
        STATEMENT = "Statement";
        LIST = "List";
        PROPERTY = "Property";
        BAG = "Bag";
        SEQ = "Seq";
        ALT = "Alt";
    }
}

/// The RDF Schema vocabulary (RDF 1.2 Schema).
pub mod rdfs {
    namespace! {
        prefix "rdfs";
        ns "http://www.w3.org/2000/01/rdf-schema#";
        LABEL = "label";
        COMMENT = "comment";
        SUB_CLASS_OF = "subClassOf";
        SUB_PROPERTY_OF = "subPropertyOf";
        DOMAIN = "domain";
        RANGE = "range";
        CLASS = "Class";
        RESOURCE = "Resource";
        LITERAL = "Literal";
        DATATYPE = "Datatype";
        MEMBER = "member";
        SEE_ALSO = "seeAlso";
        IS_DEFINED_BY = "isDefinedBy";
        CONTAINER = "Container";
        CONTAINER_MEMBERSHIP_PROPERTY = "ContainerMembershipProperty";
        /// The class RDF 1.2 Semantics types a triple term's surrogate with.
        PROPOSITION = "Proposition";
    }
}

/// The OWL 2 vocabulary (OWL 2 Web Ontology Language: RDF-Based Semantics
/// and the Mapping to RDF Graphs).
pub mod owl {
    namespace! {
        prefix "owl";
        ns "http://www.w3.org/2002/07/owl#";
        EQUIVALENT_CLASS = "equivalentClass";
        EQUIVALENT_PROPERTY = "equivalentProperty";
        INVERSE_OF = "inverseOf";
        SYMMETRIC_PROPERTY = "SymmetricProperty";
        TRANSITIVE_PROPERTY = "TransitiveProperty";
        THING = "Thing";
        NOTHING = "Nothing";
        CLASS = "Class";
        RESTRICTION = "Restriction";
        ON_PROPERTY = "onProperty";
        SOME_VALUES_FROM = "someValuesFrom";
        ALL_VALUES_FROM = "allValuesFrom";
        INTERSECTION_OF = "intersectionOf";
        UNION_OF = "unionOf";
        COMPLEMENT_OF = "complementOf";
        ONE_OF = "oneOf";
        HAS_VALUE = "hasValue";
        MIN_CARDINALITY = "minCardinality";
        MAX_CARDINALITY = "maxCardinality";
        CARDINALITY = "cardinality";
        MIN_QUALIFIED_CARDINALITY = "minQualifiedCardinality";
        MAX_QUALIFIED_CARDINALITY = "maxQualifiedCardinality";
        QUALIFIED_CARDINALITY = "qualifiedCardinality";
        ON_CLASS = "onClass";
        DISJOINT_WITH = "disjointWith";
        SAME_AS = "sameAs";
        FUNCTIONAL_PROPERTY = "FunctionalProperty";
        OBJECT_PROPERTY = "ObjectProperty";
        DATATYPE_PROPERTY = "DatatypeProperty";
        NAMED_INDIVIDUAL = "NamedIndividual";
        ONTOLOGY = "Ontology";
        ANNOTATION_PROPERTY = "AnnotationProperty";
        VERSION_INFO = "versionInfo";
        PRIOR_VERSION = "priorVersion";
        BACKWARD_COMPATIBLE_WITH = "backwardCompatibleWith";
        INCOMPATIBLE_WITH = "incompatibleWith";
        DEPRECATED = "deprecated";
        INVERSE_FUNCTIONAL_PROPERTY = "InverseFunctionalProperty";
        IRREFLEXIVE_PROPERTY = "IrreflexiveProperty";
        ASYMMETRIC_PROPERTY = "AsymmetricProperty";
        PROPERTY_CHAIN_AXIOM = "propertyChainAxiom";
        PROPERTY_DISJOINT_WITH = "propertyDisjointWith";
        ALL_DISJOINT_PROPERTIES = "AllDisjointProperties";
        ALL_DISJOINT_CLASSES = "AllDisjointClasses";
        MEMBERS = "members";
        DISTINCT_MEMBERS = "distinctMembers";
        HAS_KEY = "hasKey";
        SOURCE_INDIVIDUAL = "sourceIndividual";
        ASSERTION_PROPERTY = "assertionProperty";
        TARGET_INDIVIDUAL = "targetIndividual";
        TARGET_VALUE = "targetValue";
        REFLEXIVE_PROPERTY = "ReflexiveProperty";
        HAS_SELF = "hasSelf";
        DISJOINT_UNION_OF = "disjointUnionOf";
        NEGATIVE_PROPERTY_ASSERTION = "NegativePropertyAssertion";
        TOP_OBJECT_PROPERTY = "topObjectProperty";
        BOTTOM_OBJECT_PROPERTY = "bottomObjectProperty";
        TOP_DATA_PROPERTY = "topDataProperty";
        BOTTOM_DATA_PROPERTY = "bottomDataProperty";
        ON_DATATYPE = "onDatatype";
        WITH_RESTRICTIONS = "withRestrictions";
        DATATYPE_COMPLEMENT_OF = "datatypeComplementOf";
        ON_DATA_RANGE = "onDataRange";
        ON_PROPERTIES = "onProperties";
        /// OWL 1's deprecated spelling of `rdfs:Datatype`.
        DATA_RANGE = "DataRange";
        REAL = "real";
        RATIONAL = "rational";
        IMPORTS = "imports";
        VERSION_IRI = "versionIRI";
        ONTOLOGY_PROPERTY = "OntologyProperty";
        AXIOM = "Axiom";
        ANNOTATION = "Annotation";
        ANNOTATED_SOURCE = "annotatedSource";
        ANNOTATED_PROPERTY = "annotatedProperty";
        ANNOTATED_TARGET = "annotatedTarget";
        DIFFERENT_FROM = "differentFrom";
        ALL_DIFFERENT = "AllDifferent";
    }
}

/// The SHACL vocabulary (SHACL 1.2 Core, SPARQL Extensions, Node
/// Expressions, Inference Rules and the Advanced Features functions).
pub mod sh {
    namespace! {
        prefix "sh";
        ns "http://www.w3.org/ns/shacl#";
        CONFORMS = "conforms";
        VALIDATION_REPORT = "ValidationReport";
        VALIDATION_RESULT = "ValidationResult";
        RESULT = "result";
        FOCUS_NODE = "focusNode";
        RESULT_PATH = "resultPath";
        VALUE = "value";
        RESULT_SEVERITY = "resultSeverity";
        RESULT_MESSAGE = "resultMessage";
        SOURCE_CONSTRAINT_COMPONENT = "sourceConstraintComponent";
        SOURCE_SHAPE = "sourceShape";
        VIOLATION = "Violation";
        WARNING = "Warning";
        INFO = "Info";
        DEBUG = "Debug";
        TRACE = "Trace";
        CONFORMANCE_DISALLOWS = "conformanceDisallows";
        SHAPES_GRAPH_WELL_FORMED = "shapesGraphWellFormed";
        NODE_SHAPE = "NodeShape";
        PROPERTY_SHAPE = "PropertyShape";
        SHAPE_CLASS = "ShapeClass";
        TARGET_CLASS = "targetClass";
        TARGET_SUBJECTS_OF = "targetSubjectsOf";
        TARGET_OBJECTS_OF = "targetObjectsOf";
        TARGET_NODE = "targetNode";
        TARGET_WHERE = "targetWhere";
        SHAPE = "shape";
        PROPERTY = "property";
        PATH = "path";
        INVERSE_PATH = "inversePath";
        ALTERNATIVE_PATH = "alternativePath";
        ZERO_OR_MORE_PATH = "zeroOrMorePath";
        ONE_OR_MORE_PATH = "oneOrMorePath";
        ZERO_OR_ONE_PATH = "zeroOrOnePath";
        CLASS = "class";
        DATATYPE = "datatype";
        NODE_KIND = "nodeKind";
        MIN_COUNT = "minCount";
        MAX_COUNT = "maxCount";
        IN = "in";
        HAS_VALUE = "hasValue";
        PATTERN = "pattern";
        FLAGS = "flags";
        MIN_LENGTH = "minLength";
        UNIQUE_LANG = "uniqueLang";
        MIN_INCLUSIVE = "minInclusive";
        MAX_INCLUSIVE = "maxInclusive";
        AND = "and";
        OR = "or";
        XONE = "xone";
        NODE = "node";
        REIFIER_SHAPE = "reifierShape";
        REIFICATION_REQUIRED = "reificationRequired";
        SEVERITY = "severity";
        MESSAGE = "message";
        DEACTIVATED = "deactivated";
        NAME = "name";
        DESCRIPTION = "description";
        ORDER = "order";
        GROUP = "group";
        IRI = "IRI";
        BLANK_NODE = "BlankNode";
        LITERAL = "Literal";
        BLANK_NODE_OR_IRI = "BlankNodeOrIRI";
        BLANK_NODE_OR_LITERAL = "BlankNodeOrLiteral";
        IRI_OR_LITERAL = "IRIOrLiteral";
        TRIPLE_TERM = "TripleTerm";
        BY_TYPES = "ByTypes";
        DETAIL = "detail";
        VALUES = "values";
        DEFAULT_VALUE = "defaultValue";
        EXPECTED_PREDICATE = "expectedPredicate";
        SPARQL = "sparql";
        TARGET = "target";
        QUALIFIED_VALUE_SHAPE = "qualifiedValueShape";
        QUALIFIED_MIN_COUNT = "qualifiedMinCount";
        QUALIFIED_MAX_COUNT = "qualifiedMaxCount";
        QUALIFIED_VALUE_SHAPES_DISJOINT = "qualifiedValueShapesDisjoint";
        LESS_THAN = "lessThan";
        LESS_THAN_OR_EQUALS = "lessThanOrEquals";
        EQUALS = "equals";
        DISJOINT = "disjoint";
        NOT = "not";
        CLOSED = "closed";
        IGNORED_PROPERTIES = "ignoredProperties";
        LANGUAGE_IN = "languageIn";
        MAX_LENGTH = "maxLength";
        MIN_EXCLUSIVE = "minExclusive";
        MAX_EXCLUSIVE = "maxExclusive";
        SELECT = "select";
        ASK = "ask";
        DESCRIBE = "describe";
        UPDATE = "update";
        /// The property `sh:resultAnnotation`; the class is
        /// [`RESULT_ANNOTATION_CLASS`].
        RESULT_ANNOTATION = "resultAnnotation";
        /// The class `sh:ResultAnnotation`; the property is
        /// [`RESULT_ANNOTATION`].
        RESULT_ANNOTATION_CLASS = "ResultAnnotation";
        ANNOTATION_PROPERTY = "annotationProperty";
        ANNOTATION_VAR_NAME = "annotationVarName";
        ANNOTATION_VALUE = "annotationValue";
        SPARQL_EXPR = "sparqlExpr";
        PREFIXES = "prefixes";
        DECLARE = "declare";
        PREFIX = "prefix";
        NAMESPACE = "namespace";
        SPARQL_CONSTRAINT = "SPARQLConstraint";
        SPARQL_TARGET = "SPARQLTarget";
        SPARQL_TARGET_TYPE = "SPARQLTargetType";
        SPARQL_CONSTRAINT_COMPONENT = "SPARQLConstraintComponent";
        EXPRESSION = "expression";
        EXPRESSION_CONSTRAINT_COMPONENT = "ExpressionConstraintComponent";
        THIS = "this";
        FILTER_SHAPE = "filterShape";
        NODES = "nodes";
        UNION = "union";
        INTERSECTION = "intersection";
        IF = "if";
        THEN = "then";
        ELSE = "else";
        COUNT = "count";
        DISTINCT = "distinct";
        MINUS = "minus";
        MIN = "min";
        MAX = "max";
        SUM = "sum";
        LIMIT = "limit";
        OFFSET = "offset";
        ORDERBY = "orderby";
        DESC = "desc";
        EXISTS = "exists";
        NODE_BY_EXPRESSION = "nodeByExpression";
        NODE_BY_EXPRESSION_CONSTRAINT_COMPONENT = "NodeByExpressionConstraintComponent";
        SPARQL_FUNCTION = "SPARQLFunction";
        FUNCTION = "Function";
        RETURN_TYPE = "returnType";
        PREDICATE = "predicate";
        BODY_EXPRESSION = "bodyExpression";
        LIST_PARAMETER_EXPRESSION_FUNCTION = "ListParameterExpressionFunction";
        LIST_PARAMETER_EXPRESSION = "ListParameterExpression";
        NAMED_PARAMETER_EXPRESSION_FUNCTION = "NamedParameterExpressionFunction";
        NAMED_PARAMETER_EXPRESSION = "NamedParameterExpression";
        KEY_PARAMETER = "keyParameter";
        /// The property `sh:rule`; the class is [`RULE_CLASS`].
        RULE = "rule";
        TRIPLE_RULE = "TripleRule";
        SPARQL_RULE = "SPARQLRule";
        SUBJECT = "subject";
        OBJECT = "object";
        CONSTRUCT = "construct";
        CONDITION = "condition";
        /// The class `sh:Rule`; the property is [`RULE`].
        RULE_CLASS = "Rule";
        LAYER = "layer";
        RUN_ONCE = "runOnce";
        RULE_PROCESSOR = "ruleProcessor";
        RULE_SET = "RuleSet";
        HAS_RULE = "hasRule";
        INCLUDES_RULE_SET = "includesRuleSet";
        SPARQL_RULE_TEMPLATE = "SPARQLRuleTemplate";
        TEMP_TRIPLE = "tempTriple";
        SOURCE_RULE = "sourceRule";
        RULES_GRAPH = "RulesGraph";
        ENTAILMENT = "entailment";
        RULES_ENTAILMENT = "RulesEntailment";
        CONSTRAINT_COMPONENT = "ConstraintComponent";
        /// The class `sh:Parameter`; the property is [`PARAMETER_PROPERTY`].
        PARAMETER = "Parameter";
        /// The property `sh:parameter`; the class is [`PARAMETER`].
        PARAMETER_PROPERTY = "parameter";
        NODE_VALIDATOR = "nodeValidator";
        PROPERTY_VALIDATOR = "propertyValidator";
        VALIDATOR = "validator";
        OPTIONAL = "optional";
        SPARQL_ASK_VALIDATOR = "SPARQLAskValidator";
        SPARQL_SELECT_VALIDATOR = "SPARQLSelectValidator";
        LABEL_TEMPLATE = "labelTemplate";
        MIN_COUNT_CONSTRAINT_COMPONENT = "MinCountConstraintComponent";
        MAX_COUNT_CONSTRAINT_COMPONENT = "MaxCountConstraintComponent";
        CLASS_CONSTRAINT_COMPONENT = "ClassConstraintComponent";
        DATATYPE_CONSTRAINT_COMPONENT = "DatatypeConstraintComponent";
        NODE_KIND_CONSTRAINT_COMPONENT = "NodeKindConstraintComponent";
        IN_CONSTRAINT_COMPONENT = "InConstraintComponent";
        HAS_VALUE_CONSTRAINT_COMPONENT = "HasValueConstraintComponent";
        PATTERN_CONSTRAINT_COMPONENT = "PatternConstraintComponent";
        MIN_LENGTH_CONSTRAINT_COMPONENT = "MinLengthConstraintComponent";
        UNIQUE_LANG_CONSTRAINT_COMPONENT = "UniqueLangConstraintComponent";
        MIN_INCLUSIVE_CONSTRAINT_COMPONENT = "MinInclusiveConstraintComponent";
        MAX_INCLUSIVE_CONSTRAINT_COMPONENT = "MaxInclusiveConstraintComponent";
        MIN_EXCLUSIVE_CONSTRAINT_COMPONENT = "MinExclusiveConstraintComponent";
        MAX_EXCLUSIVE_CONSTRAINT_COMPONENT = "MaxExclusiveConstraintComponent";
        AND_CONSTRAINT_COMPONENT = "AndConstraintComponent";
        OR_CONSTRAINT_COMPONENT = "OrConstraintComponent";
        XONE_CONSTRAINT_COMPONENT = "XoneConstraintComponent";
        NODE_CONSTRAINT_COMPONENT = "NodeConstraintComponent";
        REIFIER_SHAPE_CONSTRAINT_COMPONENT = "ReifierShapeConstraintComponent";
        MAX_LENGTH_CONSTRAINT_COMPONENT = "MaxLengthConstraintComponent";
        NOT_CONSTRAINT_COMPONENT = "NotConstraintComponent";
        LANGUAGE_IN_CONSTRAINT_COMPONENT = "LanguageInConstraintComponent";
        CLOSED_CONSTRAINT_COMPONENT = "ClosedConstraintComponent";
        EQUALS_CONSTRAINT_COMPONENT = "EqualsConstraintComponent";
        DISJOINT_CONSTRAINT_COMPONENT = "DisjointConstraintComponent";
        LESS_THAN_CONSTRAINT_COMPONENT = "LessThanConstraintComponent";
        LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT = "LessThanOrEqualsConstraintComponent";
        QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT = "QualifiedMinCountConstraintComponent";
        QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT = "QualifiedMaxCountConstraintComponent";
        PROPERTY_CONSTRAINT_COMPONENT = "PropertyConstraintComponent";
        SINGLE_LINE = "singleLine";
        SINGLE_LINE_CONSTRAINT_COMPONENT = "SingleLineConstraintComponent";
        MIN_LIST_LENGTH = "minListLength";
        MIN_LIST_LENGTH_CONSTRAINT_COMPONENT = "MinListLengthConstraintComponent";
        MAX_LIST_LENGTH = "maxListLength";
        MAX_LIST_LENGTH_CONSTRAINT_COMPONENT = "MaxListLengthConstraintComponent";
        UNIQUE_MEMBERS = "uniqueMembers";
        UNIQUE_MEMBERS_CONSTRAINT_COMPONENT = "UniqueMembersConstraintComponent";
        MEMBER_SHAPE = "memberShape";
        MEMBER_SHAPE_CONSTRAINT_COMPONENT = "MemberShapeConstraintComponent";
        ROOT_CLASS = "rootClass";
        ROOT_CLASS_CONSTRAINT_COMPONENT = "RootClassConstraintComponent";
        SOME_VALUE = "someValue";
        SOME_VALUE_CONSTRAINT_COMPONENT = "SomeValueConstraintComponent";
        SUBSET_OF = "subsetOf";
        SUBSET_OF_CONSTRAINT_COMPONENT = "SubsetOfConstraintComponent";
        UNIQUE_VALUES_FOR = "uniqueValuesFor";
        UNIQUE_VALUES_FOR_CONSTRAINT_COMPONENT = "UniqueValuesForConstraintComponent";
        NODE_EXPRESSION_FUNCTION = "NodeExpressionFunction";
        SELECT_EXPRESSION = "SelectExpression";
        SPARQL_EXPR_EXPRESSION = "SPARQLExprExpression";
    }
}

/// The SKOS vocabulary (SKOS Simple Knowledge Organization System Reference):
/// the namespace and the terms the workspace's mapping and documentation
/// surfaces read.
pub mod skos {
    namespace! {
        prefix "skos";
        ns "http://www.w3.org/2004/02/skos/core#";
        CONCEPT = "Concept";
        CONCEPT_SCHEME = "ConceptScheme";
        IN_SCHEME = "inScheme";
        PREF_LABEL = "prefLabel";
        DEFINITION = "definition";
        BROADER = "broader";
        EXACT_MATCH = "exactMatch";
        CLOSE_MATCH = "closeMatch";
    }
}

/// The datatype IRI a language tag and an optional base direction imply for
/// a literal (RDF 1.2 Concepts §3.3): [`rdf::DIR_LANG_STRING`] when the
/// literal carries both a language tag and a base direction,
/// [`rdf::LANG_STRING`] when it carries a language tag alone, and `None`
/// when it carries no language tag — in which case no language datatype
/// applies and the literal's own datatype (or `xsd:string`) stands.
///
/// A base direction without a language tag is not a well-formed literal
/// (RDF 1.2 Concepts requires a language tag wherever a base direction is
/// present); this function answers `None` for it because no language
/// datatype applies, and the shape error is the caller's to raise.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::vocab::{language_datatype_iri, rdf};
///
/// assert_eq!(language_datatype_iri(true, true), Some(rdf::DIR_LANG_STRING));
/// assert_eq!(language_datatype_iri(true, false), Some(rdf::LANG_STRING));
/// assert_eq!(language_datatype_iri(false, false), None);
/// assert_eq!(language_datatype_iri(false, true), None);
/// ```
#[must_use]
pub const fn language_datatype_iri(
    has_language: bool,
    has_direction: bool,
) -> Option<&'static str> {
    match (has_language, has_direction) {
        (true, true) => Some(rdf::DIR_LANG_STRING),
        (true, false) => Some(rdf::LANG_STRING),
        (false, _) => None,
    }
}

/// The base direction of an RDF 1.2 directional language-tagged literal
/// (RDF 1.2 Concepts §3.3): the `ltr`/`rtl` token that follows the `--` of
/// `"text"@en--ltr` in the concrete syntaxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextDirection {
    /// Left-to-right base direction (`ltr`).
    Ltr,
    /// Right-to-left base direction (`rtl`).
    Rtl,
}

impl TextDirection {
    /// The lowercase direction token (`"ltr"` or `"rtl"`) as it appears in
    /// the concrete syntaxes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }

    /// The direction a token names, or `None` when it is neither.
    ///
    /// Exact and lowercase only: the RDF 1.2 grammars spell the token as the
    /// literal strings `ltr` and `rtl`, so `LTR` is not a direction and a
    /// literal carrying it is malformed rather than left-to-right.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use purrdf_iri::vocab::TextDirection;
    ///
    /// assert_eq!(TextDirection::parse("ltr"), Some(TextDirection::Ltr));
    /// assert_eq!(TextDirection::parse("rtl"), Some(TextDirection::Rtl));
    /// assert_eq!(TextDirection::parse("LTR"), None);
    /// assert_eq!(TextDirection::parse(""), None);
    /// assert_eq!(TextDirection::Rtl.as_str(), "rtl");
    /// ```
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            _ => None,
        }
    }
}

impl fmt::Display for TextDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OWL_NS, RDF_NS, RDFS_NS, SH_NS, SKOS_NS, TextDirection, language_datatype_iri, owl, rdf,
        rdfs, sh, skos,
    };

    /// One namespace under test: its prefix, the exact W3C namespace string
    /// it must carry, and its `TERMS` roster.
    type Roster = (
        &'static str,
        &'static str,
        &'static [(&'static str, &'static str)],
    );

    /// Every namespace roster with the exact W3C namespace string it must
    /// carry.
    const NAMESPACES: &[Roster] = &[
        (
            "rdf",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            rdf::TERMS,
        ),
        ("rdfs", "http://www.w3.org/2000/01/rdf-schema#", rdfs::TERMS),
        ("owl", "http://www.w3.org/2002/07/owl#", owl::TERMS),
        ("sh", "http://www.w3.org/ns/shacl#", sh::TERMS),
        ("skos", "http://www.w3.org/2004/02/skos/core#", skos::TERMS),
    ];

    #[test]
    fn namespaces_are_exactly_the_w3c_strings() {
        assert_eq!(rdf::NS, "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
        assert_eq!(rdfs::NS, "http://www.w3.org/2000/01/rdf-schema#");
        assert_eq!(owl::NS, "http://www.w3.org/2002/07/owl#");
        assert_eq!(sh::NS, "http://www.w3.org/ns/shacl#");
        assert_eq!(skos::NS, "http://www.w3.org/2004/02/skos/core#");
        assert_eq!(RDF_NS, rdf::NS);
        assert_eq!(RDFS_NS, rdfs::NS);
        assert_eq!(OWL_NS, owl::NS);
        assert_eq!(SH_NS, sh::NS);
        assert_eq!(SKOS_NS, skos::NS);
    }

    #[test]
    fn every_term_is_its_namespace_followed_by_its_local_name() {
        for &(prefix, namespace, terms) in NAMESPACES {
            assert!(!terms.is_empty(), "{prefix} has terms");
            for &(local, iri) in terms {
                assert!(!local.is_empty(), "{prefix}: empty local name");
                assert_eq!(iri, format!("{namespace}{local}"), "{prefix}:{local}");
                assert!(
                    local.bytes().all(|b| b.is_ascii_alphanumeric()),
                    "{prefix}:{local} is a plain local name"
                );
            }
        }
    }

    #[test]
    fn local_names_are_unique_within_a_namespace() {
        for &(prefix, _, terms) in NAMESPACES {
            let mut seen = std::collections::BTreeSet::new();
            for &(local, _) in terms {
                assert!(seen.insert(local), "{prefix}:{local} is listed twice");
            }
        }
    }

    #[test]
    fn pinned_spellings() {
        assert_eq!(rdf::TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        assert_eq!(
            rdf::DIR_LANG_STRING,
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString"
        );
        assert_eq!(
            rdf::XML_LITERAL,
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral"
        );
        assert_eq!(
            rdfs::CONTAINER_MEMBERSHIP_PROPERTY,
            "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty"
        );
        assert_eq!(
            owl::INVERSE_FUNCTIONAL_PROPERTY,
            "http://www.w3.org/2002/07/owl#InverseFunctionalProperty"
        );
        assert_eq!(owl::VERSION_IRI, "http://www.w3.org/2002/07/owl#versionIRI");
        assert_eq!(sh::RULE, "http://www.w3.org/ns/shacl#rule");
        assert_eq!(sh::RULE_CLASS, "http://www.w3.org/ns/shacl#Rule");
        assert_eq!(sh::PARAMETER, "http://www.w3.org/ns/shacl#Parameter");
        assert_eq!(
            sh::PARAMETER_PROPERTY,
            "http://www.w3.org/ns/shacl#parameter"
        );
        assert_eq!(
            sh::RESULT_ANNOTATION_CLASS,
            "http://www.w3.org/ns/shacl#ResultAnnotation"
        );
        assert_eq!(
            sh::LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT,
            "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent"
        );
        assert_eq!(
            skos::EXACT_MATCH,
            "http://www.w3.org/2004/02/skos/core#exactMatch"
        );
    }

    #[test]
    fn roster_sizes_are_pinned() {
        assert_eq!(rdf::TERMS.len(), 22);
        assert_eq!(rdfs::TERMS.len(), 16);
        assert_eq!(owl::TERMS.len(), 77);
        assert_eq!(sh::TERMS.len(), 221);
        assert_eq!(skos::TERMS.len(), 8);
    }

    #[test]
    fn language_datatype_follows_rdf_1_2_concepts() {
        assert_eq!(
            language_datatype_iri(true, true),
            Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString")
        );
        assert_eq!(
            language_datatype_iri(true, false),
            Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#langString")
        );
        assert_eq!(language_datatype_iri(false, false), None);
        // A direction without a language is not a language literal at all.
        assert_eq!(language_datatype_iri(false, true), None);
        // Usable in const context.
        const BOTH: Option<&str> = language_datatype_iri(true, true);
        assert_eq!(BOTH, Some(rdf::DIR_LANG_STRING));
    }

    #[test]
    fn text_direction_round_trips_its_exact_lowercase_tokens() {
        for direction in [TextDirection::Ltr, TextDirection::Rtl] {
            assert_eq!(TextDirection::parse(direction.as_str()), Some(direction));
            assert_eq!(direction.to_string(), direction.as_str());
        }
        assert_eq!(TextDirection::Ltr.as_str(), "ltr");
        assert_eq!(TextDirection::Rtl.as_str(), "rtl");
        // Exact and lowercase only: each refusal beside the token it is not.
        for (refused, accepted) in [
            ("LTR", "ltr"),
            ("Rtl", "rtl"),
            (" ltr", "ltr"),
            ("ltr ", "ltr"),
            ("", "rtl"),
            ("--ltr", "ltr"),
        ] {
            assert_eq!(TextDirection::parse(refused), None, "{refused:?}");
            assert!(TextDirection::parse(accepted).is_some(), "{accepted:?}");
        }
        assert!(TextDirection::Ltr < TextDirection::Rtl);
    }
}
