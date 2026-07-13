// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Namespace IRI constants for SHACL, RDF, RDFS, and XSD.
//!
//! All constants are plain `&'static str` IRIs. Native query patterns key on the
//! IRI string directly; constructors like [`crate::term::NamedNode::from`] wrap one
//! into a term when a value is needed.

/// SHACL namespace constants (`http://www.w3.org/ns/shacl#`).
pub mod sh {
    /// `sh:conforms` — whether the data graph conforms (boolean on a `sh:ValidationReport`).
    pub const CONFORMS: &str = "http://www.w3.org/ns/shacl#conforms";

    /// `sh:ValidationReport` — the class of validation reports.
    pub const VALIDATION_REPORT: &str = "http://www.w3.org/ns/shacl#ValidationReport";

    /// `sh:ValidationResult` — the class of individual validation results.
    pub const VALIDATION_RESULT: &str = "http://www.w3.org/ns/shacl#ValidationResult";

    /// `sh:result` — links a validation report to its validation results.
    pub const RESULT: &str = "http://www.w3.org/ns/shacl#result";

    /// `sh:focusNode` — the focus node a validation result is about.
    pub const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl#focusNode";

    /// `sh:resultPath` — the property path the reported value nodes were reached through.
    pub const RESULT_PATH: &str = "http://www.w3.org/ns/shacl#resultPath";

    /// `sh:value` — the value node a validation result reports.
    pub const VALUE: &str = "http://www.w3.org/ns/shacl#value";

    /// `sh:resultSeverity` — the severity of a validation result.
    pub const RESULT_SEVERITY: &str = "http://www.w3.org/ns/shacl#resultSeverity";

    /// `sh:resultMessage` — the human-readable message of a validation result.
    pub const RESULT_MESSAGE: &str = "http://www.w3.org/ns/shacl#resultMessage";

    /// `sh:sourceConstraintComponent` — the constraint component that produced a result.
    pub const SOURCE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#sourceConstraintComponent";

    /// `sh:sourceShape` — the shape that produced a validation result.
    pub const SOURCE_SHAPE: &str = "http://www.w3.org/ns/shacl#sourceShape";

    /// `sh:Violation` — the default (most severe) result severity.
    pub const VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";

    /// `sh:Warning` — the warning result severity.
    pub const WARNING: &str = "http://www.w3.org/ns/shacl#Warning";

    /// `sh:Info` — the informational result severity.
    pub const INFO: &str = "http://www.w3.org/ns/shacl#Info";

    // ── Shape type terms ───────────────────────────────────────────────────────

    /// `sh:NodeShape` — the class of node shapes.
    pub const NODE_SHAPE: &str = "http://www.w3.org/ns/shacl#NodeShape";

    /// `sh:PropertyShape` — the class of property shapes.
    pub const PROPERTY_SHAPE: &str = "http://www.w3.org/ns/shacl#PropertyShape";

    // ── Target predicates ──────────────────────────────────────────────────────

    /// `sh:targetClass` — targets all SHACL instances of the given class.
    pub const TARGET_CLASS: &str = "http://www.w3.org/ns/shacl#targetClass";

    /// `sh:targetSubjectsOf` — targets all subjects of triples with the given predicate.
    pub const TARGET_SUBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetSubjectsOf";

    /// `sh:targetObjectsOf` — targets all objects of triples with the given predicate.
    pub const TARGET_OBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetObjectsOf";

    /// `sh:targetNode` — targets an explicitly named node.
    pub const TARGET_NODE: &str = "http://www.w3.org/ns/shacl#targetNode";

    // ── Property shape plumbing ────────────────────────────────────────────────

    /// `sh:property` — attaches a property shape to a shape.
    pub const PROPERTY: &str = "http://www.w3.org/ns/shacl#property";

    /// `sh:path` — the property path of a property shape.
    pub const PATH: &str = "http://www.w3.org/ns/shacl#path";

    /// `sh:inversePath` — an inverse property path.
    pub const INVERSE_PATH: &str = "http://www.w3.org/ns/shacl#inversePath";

    // ── Composite path forms (§2.3.1) ──────────────────────────────────────────

    /// `sh:alternativePath` — an alternative path over a SHACL list of member paths.
    pub const ALTERNATIVE_PATH: &str = "http://www.w3.org/ns/shacl#alternativePath";

    /// `sh:zeroOrMorePath` — a zero-or-more (`*`) property path.
    pub const ZERO_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrMorePath";

    /// `sh:oneOrMorePath` — a one-or-more (`+`) property path.
    pub const ONE_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#oneOrMorePath";

    /// `sh:zeroOrOnePath` — a zero-or-one (`?`) property path.
    pub const ZERO_OR_ONE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrOnePath";

    // ── Constraint predicates (supported) ─────────────────────────────────────

    /// `sh:class` — value nodes must be SHACL instances of the given class.
    pub const CLASS: &str = "http://www.w3.org/ns/shacl#class";

    /// `sh:datatype` — value nodes must be literals of the given datatype.
    pub const DATATYPE: &str = "http://www.w3.org/ns/shacl#datatype";

    /// `sh:nodeKind` — value nodes must match the given node kind.
    pub const NODE_KIND: &str = "http://www.w3.org/ns/shacl#nodeKind";

    /// `sh:minCount` — the minimum number of value nodes.
    pub const MIN_COUNT: &str = "http://www.w3.org/ns/shacl#minCount";

    /// `sh:maxCount` — the maximum number of value nodes.
    pub const MAX_COUNT: &str = "http://www.w3.org/ns/shacl#maxCount";

    /// `sh:in` — value nodes must be members of the given SHACL list.
    pub const IN: &str = "http://www.w3.org/ns/shacl#in";

    /// `sh:hasValue` — at least one value node must equal the given term.
    pub const HAS_VALUE: &str = "http://www.w3.org/ns/shacl#hasValue";

    /// `sh:pattern` — the string form of each value node must match the given regex.
    pub const PATTERN: &str = "http://www.w3.org/ns/shacl#pattern";

    /// `sh:flags` — regex flags accompanying `sh:pattern`.
    pub const FLAGS: &str = "http://www.w3.org/ns/shacl#flags";

    /// `sh:minLength` — the minimum string length of value nodes.
    pub const MIN_LENGTH: &str = "http://www.w3.org/ns/shacl#minLength";

    /// `sh:uniqueLang` — no two value nodes may share the same language tag.
    pub const UNIQUE_LANG: &str = "http://www.w3.org/ns/shacl#uniqueLang";

    /// `sh:minInclusive` — inclusive lower bound on literal value nodes.
    pub const MIN_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#minInclusive";

    /// `sh:maxInclusive` — inclusive upper bound on literal value nodes.
    pub const MAX_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxInclusive";

    /// `sh:and` — value nodes must conform to every shape in the given list.
    pub const AND: &str = "http://www.w3.org/ns/shacl#and";

    /// `sh:or` — value nodes must conform to at least one shape in the given list.
    pub const OR: &str = "http://www.w3.org/ns/shacl#or";

    /// `sh:xone` — value nodes must conform to exactly one shape in the given list.
    pub const XONE: &str = "http://www.w3.org/ns/shacl#xone";

    /// `sh:node` — value nodes must conform to the given node shape.
    pub const NODE: &str = "http://www.w3.org/ns/shacl#node";

    /// `sh:reifierShape` — reifiers of value nodes must conform to the given shape (SHACL 1.2).
    pub const REIFIER_SHAPE: &str = "http://www.w3.org/ns/shacl#reifierShape";

    /// `sh:reificationRequired` — whether each value node must carry at least one reifier (SHACL 1.2).
    pub const REIFICATION_REQUIRED: &str = "http://www.w3.org/ns/shacl#reificationRequired";

    // ── Shape metadata (benign, not constraints) ───────────────────────────────

    /// `sh:severity` — overrides the severity of results produced by a shape.
    pub const SEVERITY: &str = "http://www.w3.org/ns/shacl#severity";

    /// `sh:message` — a human-readable message copied onto results produced by a shape.
    pub const MESSAGE: &str = "http://www.w3.org/ns/shacl#message";

    /// `sh:deactivated` — a `true` value disables the shape entirely.
    pub const DEACTIVATED: &str = "http://www.w3.org/ns/shacl#deactivated";

    /// `sh:name` — a human-readable shape name (non-validating).
    pub const NAME: &str = "http://www.w3.org/ns/shacl#name";

    /// `sh:description` — a human-readable shape description (non-validating).
    pub const DESCRIPTION: &str = "http://www.w3.org/ns/shacl#description";

    /// `sh:order` — a numeric ordering hint; also the execution order of SHACL-AF rules.
    pub const ORDER: &str = "http://www.w3.org/ns/shacl#order";

    /// `sh:group` — groups related property shapes (non-validating).
    pub const GROUP: &str = "http://www.w3.org/ns/shacl#group";

    // ── sh:nodeKind value IRIs ─────────────────────────────────────────────────

    /// `sh:IRI` — node kind: IRIs only.
    pub const IRI: &str = "http://www.w3.org/ns/shacl#IRI";

    /// `sh:BlankNode` — node kind: blank nodes only.
    pub const BLANK_NODE: &str = "http://www.w3.org/ns/shacl#BlankNode";

    /// `sh:Literal` — node kind: literals only.
    pub const LITERAL: &str = "http://www.w3.org/ns/shacl#Literal";

    /// `sh:BlankNodeOrIRI` — node kind: blank nodes or IRIs.
    pub const BLANK_NODE_OR_IRI: &str = "http://www.w3.org/ns/shacl#BlankNodeOrIRI";

    /// `sh:BlankNodeOrLiteral` — node kind: blank nodes or literals.
    pub const BLANK_NODE_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#BlankNodeOrLiteral";

    /// `sh:IRIOrLiteral` — node kind: IRIs or literals.
    pub const IRI_OR_LITERAL: &str = "http://www.w3.org/ns/shacl#IRIOrLiteral";

    // ── SHACL-AF and advanced constraint predicates ────────────────────────────

    /// `sh:sparql` — attaches a SPARQL constraint to a shape.
    pub const SPARQL: &str = "http://www.w3.org/ns/shacl#sparql";

    /// `sh:target` — attaches a custom (e.g. SPARQL-based) target to a shape.
    pub const TARGET: &str = "http://www.w3.org/ns/shacl#target";

    /// `sh:qualifiedValueShape` — the shape counted by the qualified cardinality constraints.
    pub const QUALIFIED_VALUE_SHAPE: &str = "http://www.w3.org/ns/shacl#qualifiedValueShape";

    /// `sh:qualifiedMinCount` — the minimum number of value nodes conforming to the qualified shape.
    pub const QUALIFIED_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMinCount";

    /// `sh:qualifiedMaxCount` — the maximum number of value nodes conforming to the qualified shape.
    pub const QUALIFIED_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMaxCount";

    /// `sh:qualifiedValueShapesDisjoint` — sibling qualified value shapes must match disjoint value sets.
    pub const QUALIFIED_VALUE_SHAPES_DISJOINT: &str =
        "http://www.w3.org/ns/shacl#qualifiedValueShapesDisjoint";

    /// `sh:lessThan` — value nodes must compare less than the sibling property's values.
    pub const LESS_THAN: &str = "http://www.w3.org/ns/shacl#lessThan";

    /// `sh:lessThanOrEquals` — value nodes must compare less than or equal to the sibling property's values.
    pub const LESS_THAN_OR_EQUALS: &str = "http://www.w3.org/ns/shacl#lessThanOrEquals";

    /// `sh:equals` — the value set must equal the sibling property's value set.
    pub const EQUALS: &str = "http://www.w3.org/ns/shacl#equals";

    /// `sh:disjoint` — the value set must be disjoint with the sibling property's value set.
    pub const DISJOINT: &str = "http://www.w3.org/ns/shacl#disjoint";

    /// `sh:not` — value nodes must not conform to the given shape.
    pub const NOT: &str = "http://www.w3.org/ns/shacl#not";

    /// `sh:closed` — restricts focus nodes to the properties declared by the shape.
    pub const CLOSED: &str = "http://www.w3.org/ns/shacl#closed";

    /// `sh:ignoredProperties` — predicates exempt from `sh:closed` checking.
    pub const IGNORED_PROPERTIES: &str = "http://www.w3.org/ns/shacl#ignoredProperties";

    /// `sh:languageIn` — value-node language tags must be in the given list.
    pub const LANGUAGE_IN: &str = "http://www.w3.org/ns/shacl#languageIn";

    /// `sh:maxLength` — the maximum string length of value nodes.
    pub const MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#maxLength";

    /// `sh:minExclusive` — exclusive lower bound on literal value nodes.
    pub const MIN_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#minExclusive";

    /// `sh:maxExclusive` — exclusive upper bound on literal value nodes.
    pub const MAX_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxExclusive";

    /// `sh:select` — the SELECT query of a SHACL-SPARQL constraint or validator.
    pub const SELECT: &str = "http://www.w3.org/ns/shacl#select";

    /// `sh:ask` — the ASK query of a SHACL-SPARQL validator.
    pub const ASK: &str = "http://www.w3.org/ns/shacl#ask";

    // ── SHACL-AF prefix declarations (sh:prefixes / sh:declare) ───────────────

    /// `sh:prefixes` — links a SPARQL-bearing node to its prefix declarations.
    pub const PREFIXES: &str = "http://www.w3.org/ns/shacl#prefixes";

    /// `sh:declare` — attaches a prefix declaration to a prefix-owning node.
    pub const DECLARE: &str = "http://www.w3.org/ns/shacl#declare";

    /// `sh:prefix` — the prefix label of a declaration.
    pub const PREFIX: &str = "http://www.w3.org/ns/shacl#prefix";

    /// `sh:namespace` — the namespace IRI of a declaration.
    pub const NAMESPACE: &str = "http://www.w3.org/ns/shacl#namespace";

    /// `sh:SPARQLConstraint` — the class of SPARQL-based constraints.
    pub const SPARQL_CONSTRAINT: &str = "http://www.w3.org/ns/shacl#SPARQLConstraint";

    /// `sh:SPARQLTarget` — the class of SPARQL-based custom targets.
    pub const SPARQL_TARGET: &str = "http://www.w3.org/ns/shacl#SPARQLTarget";

    /// `sh:SPARQLTargetType` — the metaclass of parameterized SPARQL-based target types.
    pub const SPARQL_TARGET_TYPE: &str = "http://www.w3.org/ns/shacl#SPARQLTargetType";

    /// `sh:SPARQLConstraintComponent` — the constraint component reported for SPARQL constraint violations.
    pub const SPARQL_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#SPARQLConstraintComponent";

    // ── SHACL-AF node expressions (§node expressions) ─────────────────────────

    /// `sh:expression` — attaches a node-expression constraint to a shape.
    pub const EXPRESSION: &str = "http://www.w3.org/ns/shacl#expression";

    /// `sh:ExpressionConstraintComponent` — the constraint component reported for failed `sh:expression` constraints.
    pub const EXPRESSION_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ExpressionConstraintComponent";

    /// `sh:this` — the focus node: the focus-node expression and the pre-bound `$this` variable.
    pub const THIS: &str = "http://www.w3.org/ns/shacl#this";

    /// `sh:filterShape` — the filter shape of a filter-shape node expression.
    pub const FILTER_SHAPE: &str = "http://www.w3.org/ns/shacl#filterShape";

    /// `sh:nodes` — the input-nodes expression of a filter, path, or aggregate node expression.
    pub const NODES: &str = "http://www.w3.org/ns/shacl#nodes";

    /// `sh:union` — a union node expression over a list of member expressions.
    pub const UNION: &str = "http://www.w3.org/ns/shacl#union";

    /// `sh:intersection` — an intersection node expression over a list of member expressions.
    pub const INTERSECTION: &str = "http://www.w3.org/ns/shacl#intersection";

    /// `sh:if` — the condition of an if/then/else node expression.
    pub const IF: &str = "http://www.w3.org/ns/shacl#if";

    /// `sh:then` — the then-branch of an if/then/else node expression.
    pub const THEN: &str = "http://www.w3.org/ns/shacl#then";

    /// `sh:else` — the else-branch of an if/then/else node expression.
    pub const ELSE: &str = "http://www.w3.org/ns/shacl#else";

    /// `sh:count` — a count aggregation node expression.
    pub const COUNT: &str = "http://www.w3.org/ns/shacl#count";

    /// `sh:distinct` — a distinct node expression (deduplicates its input).
    pub const DISTINCT: &str = "http://www.w3.org/ns/shacl#distinct";

    /// `sh:min` — a minimum aggregation node expression.
    pub const MIN: &str = "http://www.w3.org/ns/shacl#min";

    /// `sh:max` — a maximum aggregation node expression.
    pub const MAX: &str = "http://www.w3.org/ns/shacl#max";

    /// `sh:sum` — a sum aggregation node expression.
    pub const SUM: &str = "http://www.w3.org/ns/shacl#sum";

    /// `sh:limit` — a limit node expression (truncates its ordered input).
    pub const LIMIT: &str = "http://www.w3.org/ns/shacl#limit";

    /// `sh:offset` — an offset node expression (skips a prefix of its ordered input).
    pub const OFFSET: &str = "http://www.w3.org/ns/shacl#offset";

    /// `sh:orderby` — the sort-key expression of an order-by node expression.
    pub const ORDERBY: &str = "http://www.w3.org/ns/shacl#orderby";

    /// The adopted PurRDF DASH-extension direction flag for `sh:orderby`
    /// (boolean; `true` ⇒ descending, default ascending).
    pub const DESC: &str = "http://www.w3.org/ns/shacl#desc";

    /// `sh:exists` — a boolean existence node expression.
    pub const EXISTS: &str = "http://www.w3.org/ns/shacl#exists";

    /// `sh:SPARQLFunction` — the class of SPARQL-bodied SHACL-AF functions.
    pub const SPARQL_FUNCTION: &str = "http://www.w3.org/ns/shacl#SPARQLFunction";

    /// `sh:Function` — the class of SHACL-AF functions.
    pub const FUNCTION: &str = "http://www.w3.org/ns/shacl#Function";

    /// `sh:returnType` — the declared datatype/class of a function's return value.
    pub const RETURN_TYPE: &str = "http://www.w3.org/ns/shacl#returnType";

    /// `sh:predicate` — an alternative to `sh:path` naming a parameter's predicate
    /// (its local name is the pre-bound SPARQL variable).
    pub const PREDICATE: &str = "http://www.w3.org/ns/shacl#predicate";

    // ── SHACL-AF rules (§rules) ───────────────────────────────────────────────

    /// `sh:rule` — attaches a rule to a shape.
    pub const RULE: &str = "http://www.w3.org/ns/shacl#rule";

    /// `sh:TripleRule` — the rule type whose head is a single `sh:subject` /
    /// `sh:predicate` / `sh:object` node-expression triple.
    pub const TRIPLE_RULE: &str = "http://www.w3.org/ns/shacl#TripleRule";

    /// `sh:SPARQLRule` — the rule type whose head is a SPARQL `sh:construct` query.
    pub const SPARQL_RULE: &str = "http://www.w3.org/ns/shacl#SPARQLRule";

    /// `sh:subject` — the subject node expression of a `sh:TripleRule`.
    pub const SUBJECT: &str = "http://www.w3.org/ns/shacl#subject";

    /// `sh:object` — the object node expression of a `sh:TripleRule`.
    pub const OBJECT: &str = "http://www.w3.org/ns/shacl#object";

    /// `sh:construct` — the SPARQL CONSTRUCT query text of a `sh:SPARQLRule`.
    pub const CONSTRUCT: &str = "http://www.w3.org/ns/shacl#construct";

    /// `sh:condition` — a shape a focus node must conform to for a rule to fire.
    pub const CONDITION: &str = "http://www.w3.org/ns/shacl#condition";

    // ── Custom constraint-component vocabulary ───────────────────────────────

    /// `sh:ConstraintComponent` — the class of constraint components.
    pub const CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#ConstraintComponent";

    /// `sh:Parameter` — the class of constraint-component parameters.
    pub const PARAMETER: &str = "http://www.w3.org/ns/shacl#Parameter";

    /// `sh:parameter` — attaches a parameter declaration to a constraint component.
    pub const PARAMETER_PROPERTY: &str = "http://www.w3.org/ns/shacl#parameter";

    /// `sh:nodeValidator` — the validator a component uses on node shapes.
    pub const NODE_VALIDATOR: &str = "http://www.w3.org/ns/shacl#nodeValidator";

    /// `sh:propertyValidator` — the validator a component uses on property shapes.
    pub const PROPERTY_VALIDATOR: &str = "http://www.w3.org/ns/shacl#propertyValidator";

    /// `sh:validator` — the default validator of a constraint component.
    pub const VALIDATOR: &str = "http://www.w3.org/ns/shacl#validator";

    /// `sh:optional` — marks a constraint-component parameter as optional.
    pub const OPTIONAL: &str = "http://www.w3.org/ns/shacl#optional";

    /// `sh:SPARQLAskValidator` — the class of ASK-query-based validators.
    pub const SPARQL_ASK_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLAskValidator";

    /// `sh:SPARQLSelectValidator` — the class of SELECT-query-based validators.
    pub const SPARQL_SELECT_VALIDATOR: &str = "http://www.w3.org/ns/shacl#SPARQLSelectValidator";

    // ── Constraint component IRIs (sh:*ConstraintComponent) ──────────────────

    /// `sh:MinCountConstraintComponent` — the component reported for `sh:minCount` violations.
    pub const MIN_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinCountConstraintComponent";

    /// `sh:MaxCountConstraintComponent` — the component reported for `sh:maxCount` violations.
    pub const MAX_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxCountConstraintComponent";

    /// `sh:ClassConstraintComponent` — the component reported for `sh:class` violations.
    pub const CLASS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ClassConstraintComponent";

    /// `sh:DatatypeConstraintComponent` — the component reported for `sh:datatype` violations.
    pub const DATATYPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#DatatypeConstraintComponent";

    /// `sh:NodeKindConstraintComponent` — the component reported for `sh:nodeKind` violations.
    pub const NODE_KIND_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeKindConstraintComponent";

    /// `sh:InConstraintComponent` — the component reported for `sh:in` violations.
    pub const IN_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#InConstraintComponent";

    /// `sh:HasValueConstraintComponent` — the component reported for `sh:hasValue` violations.
    pub const HAS_VALUE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#HasValueConstraintComponent";

    /// `sh:PatternConstraintComponent` — the component reported for `sh:pattern` violations.
    pub const PATTERN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#PatternConstraintComponent";

    /// `sh:MinLengthConstraintComponent` — the component reported for `sh:minLength` violations.
    pub const MIN_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinLengthConstraintComponent";

    /// `sh:UniqueLangConstraintComponent` — the component reported for `sh:uniqueLang` violations.
    pub const UNIQUE_LANG_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#UniqueLangConstraintComponent";

    /// `sh:MinInclusiveConstraintComponent` — the component reported for `sh:minInclusive` violations.
    pub const MIN_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent";

    /// `sh:MaxInclusiveConstraintComponent` — the component reported for `sh:maxInclusive` violations.
    pub const MAX_INCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent";

    /// `sh:MinExclusiveConstraintComponent` — the component reported for `sh:minExclusive` violations.
    pub const MIN_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent";

    /// `sh:MaxExclusiveConstraintComponent` — the component reported for `sh:maxExclusive` violations.
    pub const MAX_EXCLUSIVE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent";

    /// `sh:AndConstraintComponent` — the component reported for `sh:and` violations.
    pub const AND_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#AndConstraintComponent";

    /// `sh:OrConstraintComponent` — the component reported for `sh:or` violations.
    pub const OR_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#OrConstraintComponent";

    /// `sh:XoneConstraintComponent` — the component reported for `sh:xone` violations.
    pub const XONE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#XoneConstraintComponent";

    /// `sh:NodeConstraintComponent` — the component reported for `sh:node` violations.
    pub const NODE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#NodeConstraintComponent";

    /// `sh:ReifierShapeConstraintComponent` — the component reported for `sh:reifierShape` and `sh:reificationRequired` violations.
    pub const REIFIER_SHAPE_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ReifierShapeConstraintComponent";

    /// `sh:MaxLengthConstraintComponent` — the component reported for `sh:maxLength` violations.
    pub const MAX_LENGTH_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#MaxLengthConstraintComponent";

    /// `sh:NotConstraintComponent` — the component reported for `sh:not` violations.
    pub const NOT_CONSTRAINT_COMPONENT: &str = "http://www.w3.org/ns/shacl#NotConstraintComponent";

    /// `sh:LanguageInConstraintComponent` — the component reported for `sh:languageIn` violations.
    pub const LANGUAGE_IN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LanguageInConstraintComponent";

    /// `sh:ClosedConstraintComponent` — the component reported for `sh:closed` violations.
    pub const CLOSED_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#ClosedConstraintComponent";

    /// `sh:EqualsConstraintComponent` — the component reported for `sh:equals` violations.
    pub const EQUALS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#EqualsConstraintComponent";

    /// `sh:DisjointConstraintComponent` — the component reported for `sh:disjoint` violations.
    pub const DISJOINT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#DisjointConstraintComponent";

    /// `sh:LessThanConstraintComponent` — the component reported for `sh:lessThan` violations.
    pub const LESS_THAN_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LessThanConstraintComponent";

    /// `sh:LessThanOrEqualsConstraintComponent` — the component reported for `sh:lessThanOrEquals` violations.
    pub const LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent";

    /// `sh:QualifiedMinCountConstraintComponent` — the component reported for `sh:qualifiedMinCount` violations.
    pub const QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT: &str =
        "http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent";

    /// `sh:QualifiedMaxCountConstraintComponent` — the component reported for `sh:qualifiedMaxCount` violations.
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
    /// `rdf:type` — the instance-of predicate.
    pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

    /// `rdf:first` — the head of an RDF collection cell.
    pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";

    /// `rdf:rest` — the tail of an RDF collection cell.
    pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";

    /// `rdf:nil` — the empty RDF collection.
    pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

    /// `rdf:reifies` — the RDF 1.2 predicate linking a reifier to the triple term it reifies.
    pub const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
}

/// RDFS namespace constants.
pub mod rdfs {
    /// The RDFS namespace base IRI.
    pub const BASE: &str = "http://www.w3.org/2000/01/rdf-schema#";

    /// `rdfs:Class` — the class of RDFS classes.
    pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";

    /// `rdfs:subClassOf` — the RDFS subclass predicate.
    pub const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    /// `rdfs:range` — the range predicate (a property's values are instances of
    /// the range class).
    pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";

    /// `rdfs:label` — a human-readable name for a resource.
    pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

    /// `rdfs:comment` — a human-readable description of a resource.
    pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
}

/// XSD namespace base string.
pub mod xsd {
    /// The XSD namespace base IRI.
    pub const BASE: &str = "http://www.w3.org/2001/XMLSchema#";

    /// `xsd:boolean` — the boolean datatype IRI.
    pub const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

    /// `xsd:string` — the string datatype IRI.
    pub const STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

    /// `xsd:integer` — the integer datatype IRI.
    pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
}
