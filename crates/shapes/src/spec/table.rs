// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! THE table: every SHACL 1.2 spec term this engine implements, as rows.
//!
//! Every IRI here is DEFINED BY a W3C specification — SHACL 1.2 Core, SHACL 1.2
//! Node Expressions, SHACL 1.2 SPARQL Extensions, or SHACL Advanced Features — and
//! transcribed from the vendored vocabularies under `crates/shapes/spec/`. PurRDF
//! mints none of them. Adding a row is how the engine claims a term; nothing else
//! in the crate keeps a second list of "native" terms.

use crate::expression::SparqlCallForm;
use crate::model::{rdf, sh, shnex};

// ── Row types ─────────────────────────────────────────────────────────────────

/// The declaring class of a node-expression function (SHACL 1.2 Node Expressions
/// §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionClass {
    /// `sh:NamedParameterExpressionFunction` (§6.1): arguments under the
    /// parameters' own `sh:path` IRIs; recognised at a call site by a key
    /// parameter.
    NamedParameter,
    /// `sh:ListParameterExpressionFunction` (§6.2): the function's own IRI is its
    /// list parameter property.
    ListParameter,
    /// `sh:NodeExpressionFunction` and nothing more specific — `shnex:EmptyExpression`,
    /// which has no parameter at all.
    Plain,
}

impl FunctionClass {
    /// The class IRI a declaration is typed with.
    #[must_use]
    pub const fn iri(self) -> &'static str {
        match self {
            Self::NamedParameter => sh::NAMED_PARAMETER_EXPRESSION_FUNCTION,
            Self::ListParameter => sh::LIST_PARAMETER_EXPRESSION_FUNCTION,
            Self::Plain => sh::NODE_EXPRESSION_FUNCTION,
        }
    }

    /// The class `iri` names, if it is one of the three declaring classes.
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        [Self::NamedParameter, Self::ListParameter, Self::Plain]
            .into_iter()
            .find(|class| class.iri() == iri)
    }
}

/// One parameter of a built-in function's signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionParam {
    /// The parameter's `sh:path`.
    pub path: &'static str,
    /// Whether the parameter carries `sh:keyParameter true`.
    pub key: bool,
    /// Whether the parameter may be omitted at a call site, as the SPECIFICATION
    /// TEXT defines it. See [`SPEC_TEXT_OPTIONALITY`] for the parameters where the
    /// vocabulary file's `sh:optional` disagrees with the spec text.
    pub optional: bool,
}

/// A native node-expression kind: the one IR arm a built-in function lowers to.
///
/// SHACL Advanced Features and SHACL 1.2 Node Expressions give several of the
/// same operations two IRIs. This enum is the one name each operation has inside
/// PurRDF: every accepted key maps onto a kind, every kind lowers to one
/// `NodeExpr` arm, and that arm has exactly one evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExprKind {
    /// `sh:path` / `shnex:pathValues` — path value nodes.
    Path,
    /// `sh:filterShape` / `shnex:filterShape` — shape-filtered nodes.
    FilterShape,
    /// `sh:union` — SHACL-AF set union (no `shnex:` spelling exists).
    Union,
    /// `sh:intersection` / `shnex:intersection` — set intersection.
    Intersection,
    /// `shnex:concat` — sequence concatenation (no `sh:` spelling exists).
    Concat,
    /// `sh:if` / `shnex:if` — conditional.
    If,
    /// `sh:count` / `shnex:count` — cardinality.
    Count,
    /// `sh:distinct` / `shnex:distinct` — duplicate elimination.
    Distinct,
    /// `sh:min` / `shnex:min` — minimum.
    Min,
    /// `sh:max` / `shnex:max` — maximum.
    Max,
    /// `sh:sum` / `shnex:sum` — sum.
    Sum,
    /// `sh:exists` / `shnex:exists` — existence predicate.
    Exists,
    /// `shnex:var` — a scope/focus variable reference.
    Var,
    /// `rdf:first` — an RDF collection read as a `shnex:ListExpression`.
    List,
    /// `shnex:remove` — set difference preserving input order.
    Remove,
    /// `shnex:limit` — the named-parameter limit expression.
    Limit,
    /// `shnex:offset` — the named-parameter offset expression.
    Offset,
    /// `shnex:orderBy` — the named-parameter order-by expression.
    OrderBy,
    /// `shnex:flatMap` — per-node mapping with concatenation.
    FlatMap,
    /// `shnex:findFirst` — the first conforming input node.
    FindFirst,
    /// `shnex:matchAll` — whether every input node conforms.
    MatchAll,
    /// `shnex:instancesOf` — the SHACL instances of a class.
    InstancesOf,
    /// `shnex:nodesMatching` — every conforming node of the focus graph.
    NodesMatching,
    /// `shnex:conformsToShape` — a two-argument conformance predicate.
    ConformsToShape,
    /// `sh:select` / `sh:sparqlExpr` — a SPARQL-based node expression.
    Select,
    /// `shnex:arg` — an argument reference inside a custom function's body.
    Arg,
}

impl ExprKind {
    /// A stable label for the table's canonical rendering.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::FilterShape => "filterShape",
            Self::Union => "union",
            Self::Intersection => "intersection",
            Self::Concat => "concat",
            Self::If => "if",
            Self::Count => "count",
            Self::Distinct => "distinct",
            Self::Min => "min",
            Self::Max => "max",
            Self::Sum => "sum",
            Self::Exists => "exists",
            Self::Var => "var",
            Self::List => "list",
            Self::Remove => "remove",
            Self::Limit => "limit",
            Self::Offset => "offset",
            Self::OrderBy => "orderBy",
            Self::FlatMap => "flatMap",
            Self::FindFirst => "findFirst",
            Self::MatchAll => "matchAll",
            Self::InstancesOf => "instancesOf",
            Self::NodesMatching => "nodesMatching",
            Self::ConformsToShape => "conformsToShape",
            Self::Select => "select",
            Self::Arg => "arg",
        }
    }
}

/// How a built-in function is implemented.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Implementation {
    /// Recognised by `key` — for a named-parameter function, its dispatching key
    /// parameter; for a list-parameter function, its own IRI — and lowered to
    /// `kind`.
    Keyed {
        /// The predicate that selects this function at a call site.
        key: &'static str,
        /// The IR arm it lowers to.
        kind: ExprKind,
    },
    /// The empty expression: a blank node that is the subject of no triple
    /// (SHACL 1.2 Node Expressions §4.1.1). No key selects it.
    Empty,
}

/// One built-in node-expression function row.
#[derive(Clone, Copy, Debug)]
pub struct FunctionRow {
    pub(crate) iri: &'static str,
    pub(crate) class: FunctionClass,
    pub(crate) params: &'static [FunctionParam],
    pub(crate) implementation: Implementation,
}

impl FunctionRow {
    /// The function IRI.
    #[must_use]
    pub const fn iri(&self) -> &'static str {
        self.iri
    }

    /// The declaring class.
    #[must_use]
    pub const fn class(&self) -> FunctionClass {
        self.class
    }

    /// The signature, in the vocabulary's own parameter order.
    #[must_use]
    pub const fn params(&self) -> &'static [FunctionParam] {
        self.params
    }
}

/// The value a component parameter takes: the SHACL 1.2 Core syntax rule for the
/// parameter, as the shapes-graph well-formedness check enforces it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueRule {
    /// An IRI.
    Iri,
    /// A `sh:NodeKind` IRI (one of the seven), or a well-formed SHACL list whose
    /// members are the four basic kinds (SHACL 1.2 Core §4.1.3).
    NodeKind,
    /// A literal with datatype `xsd:integer` whose value is at least 0.
    NonNegativeInteger,
    /// A literal with datatype `xsd:boolean`.
    Boolean,
    /// A literal with datatype `xsd:string`.
    StringLiteral,
    /// A literal (SHACL 1.2 Core §4.5: "The values of sh:minInclusive in a shape
    /// are literals").
    Literal,
    /// Any RDF term.
    Term,
    /// A well-formed SHACL list of RDF terms.
    TermList,
    /// A well-formed SHACL list of IRIs.
    IriList,
    /// A well-formed SHACL list of `xsd:string` literals.
    LanguageTagList,
    /// A shape: an IRI or a blank node.
    Shape,
    /// A well-formed SHACL list of shapes.
    ShapeList,
    /// A SPARQL-based constraint node (`sh:select` and optional `sh:prefixes`):
    /// an IRI or a blank node.
    SparqlConstraint,
    /// A node expression: any RDF term.
    NodeExpression,
    /// An IRI or a well-formed SHACL list of IRIs.
    IriOrIriList,
    /// A well-formed SHACL property path.
    Path,
    /// A literal with datatype `xsd:boolean`, or the IRI `sh:ByTypes` (SHACL 1.2
    /// Core §4.8.1).
    ClosedValue,
}

impl ValueRule {
    /// A stable label for the table's canonical rendering.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Iri => "iri",
            Self::NodeKind => "node-kind",
            Self::NonNegativeInteger => "non-negative-integer",
            Self::Boolean => "boolean",
            Self::StringLiteral => "string-literal",
            Self::Literal => "literal",
            Self::Term => "term",
            Self::TermList => "term-list",
            Self::IriList => "iri-list",
            Self::LanguageTagList => "language-tag-list",
            Self::Shape => "shape",
            Self::ShapeList => "shape-list",
            Self::SparqlConstraint => "sparql-constraint",
            Self::NodeExpression => "node-expression",
            Self::IriOrIriList => "iri-or-iri-list",
            Self::Path => "path",
            Self::ClosedValue => "closed-value",
        }
    }

    /// What a value of this rule is, for a diagnostic ("… must be {describe}").
    pub(crate) const fn describe(self) -> &'static str {
        match self {
            Self::Iri => "an IRI",
            Self::NodeKind => {
                "one of the sh:NodeKind IRIs, or a SHACL list of sh:BlankNode / sh:IRI / \
                 sh:Literal / sh:TripleTerm"
            }
            Self::NonNegativeInteger => "an xsd:integer literal of at least 0",
            Self::Boolean => "an xsd:boolean literal",
            Self::StringLiteral => "an xsd:string literal",
            Self::Literal => "a literal",
            Self::Term => "an RDF term",
            Self::TermList => "a well-formed SHACL list",
            Self::IriList => "a well-formed SHACL list of IRIs",
            Self::LanguageTagList => "a well-formed SHACL list of xsd:string literals",
            Self::Shape => "a shape (an IRI or a blank node)",
            Self::ShapeList => "a well-formed SHACL list of shapes (IRIs or blank nodes)",
            Self::SparqlConstraint => "a SPARQL-based constraint (an IRI or a blank node)",
            Self::NodeExpression => "a node expression",
            Self::IriOrIriList => "an IRI or a well-formed SHACL list of IRIs",
            Self::Path => "a well-formed SHACL property path",
            Self::ClosedValue => "an xsd:boolean literal or sh:ByTypes",
        }
    }
}

/// One parameter of a constraint component.
#[derive(Clone, Copy, Debug)]
pub struct ComponentParam {
    /// The parameter's `sh:path`.
    pub path: &'static str,
    /// Whether the vocabulary declares the parameter `sh:optional true`.
    pub optional: bool,
    /// What a value of the parameter must be.
    pub value: ValueRule,
    /// Whether a shape has at most one value for the parameter (SHACL 1.2 Core:
    /// "A shape has at most one value for …", and §3.1.1 for every parameter of a
    /// multi-parameter component).
    pub single: bool,
    /// Whether node shapes cannot have any value for the parameter (SHACL 1.2
    /// Core: "Node shapes cannot have any value for …").
    pub property_only: bool,
}

impl ComponentParam {
    /// This parameter, single-valued.
    const fn single(self) -> Self {
        Self {
            single: true,
            ..self
        }
    }

    /// This parameter, forbidden on node shapes.
    const fn property_only(self) -> Self {
        Self {
            property_only: true,
            ..self
        }
    }
}

/// Where the parsed model carries a component's instances.
#[derive(Clone, Copy, Debug)]
pub enum Carrier {
    /// These `Constraint` variants.
    Constraint(&'static [&'static str]),
    /// This field of a node or property shape (the component's argument is a
    /// shape that is carried structurally, not as a constraint).
    ShapeField(&'static str),
    /// Nothing: the engine does not implement the component
    /// ([`ComponentStatus::Unimplemented`]).
    None,
}

/// Whether the engine evaluates a declared component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentStatus {
    /// The engine parses and evaluates the component natively.
    Native,
    /// The vocabulary declares the component and this engine does not evaluate
    /// it: a shape using one of its parameters is a load error rather than a
    /// silent conformance.
    Unimplemented,
}

/// One constraint-component row.
#[derive(Clone, Copy, Debug)]
pub struct ComponentRow {
    pub(crate) iri: &'static str,
    pub(crate) params: &'static [ComponentParam],
    pub(crate) status: ComponentStatus,
    pub(crate) carrier: Carrier,
}

impl ComponentRow {
    /// The component IRI.
    #[must_use]
    pub const fn iri(&self) -> &'static str {
        self.iri
    }

    /// The component's parameters, in the vocabulary's order.
    #[must_use]
    pub const fn params(&self) -> &'static [ComponentParam] {
        self.params
    }

    /// Whether the engine evaluates it.
    #[must_use]
    pub const fn status(&self) -> ComponentStatus {
        self.status
    }

    /// Where the parsed model carries its instances.
    #[must_use]
    pub const fn carrier(&self) -> Carrier {
        self.carrier
    }
}

/// Which specification an alias spelling comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasSource {
    /// The SHACL Advanced Features 1.0 node-expression vocabulary in the `sh:`
    /// namespace, which SHACL 1.2 Node Expressions re-spells under `shnex:`.
    ShaclAdvancedFeatures,
    /// The W3C `shnex-sparql.ttl` vocabulary, which spells two SPARQL functions
    /// differently from the SPARQL Working Group's `sparql-ns.ttl`.
    ShnexSparqlVocabulary,
}

impl AliasSource {
    /// A stable label for the table's canonical rendering.
    const fn label(self) -> &'static str {
        match self {
            Self::ShaclAdvancedFeatures => "shacl-af",
            Self::ShnexSparqlVocabulary => "shnex-sparql",
        }
    }
}

/// What an alias key does on a node expression.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AliasRole {
    /// It selects an expression kind, exactly as a native key parameter does.
    Primary(ExprKind),
    /// It is an operand of the kind another key selects (`sh:nodes`, `sh:then`,
    /// `sh:else`), or a paging wrapper peeled from any core expression
    /// (`sh:limit`, `sh:offset`, `sh:orderby`, and `sh:desc` beside it).
    Operand,
}

/// One accepted alias spelling of a node-expression key.
#[derive(Clone, Copy, Debug)]
pub struct KeyAlias {
    pub(crate) iri: &'static str,
    pub(crate) role: AliasRole,
    pub(crate) source: AliasSource,
}

impl KeyAlias {
    /// The alias IRI.
    #[must_use]
    pub const fn iri(&self) -> &'static str {
        self.iri
    }

    /// Where the spelling comes from.
    #[must_use]
    pub const fn source(&self) -> AliasSource {
        self.source
    }
}

/// One accepted alias spelling of a SPARQL function name.
#[derive(Clone, Copy, Debug)]
pub struct SparqlAlias {
    /// The `sparql:` local name the alias spells.
    pub local: &'static str,
    /// The `sparql-ns.ttl` local name of the same function.
    pub sparql_ns_name: &'static str,
    pub(crate) form: SparqlCallForm,
    /// Where the spelling comes from.
    pub source: AliasSource,
}

/// One target predicate the engine implements.
#[derive(Clone, Copy, Debug)]
pub struct TargetRow {
    /// The predicate that declares the target on a shape.
    pub predicate: &'static str,
}

// ── Constructors ──────────────────────────────────────────────────────────────

const fn key(path: &'static str) -> FunctionParam {
    FunctionParam {
        path,
        key: true,
        optional: false,
    }
}

const fn required(path: &'static str) -> FunctionParam {
    FunctionParam {
        path,
        key: false,
        optional: false,
    }
}

const fn optional(path: &'static str) -> FunctionParam {
    FunctionParam {
        path,
        key: false,
        optional: true,
    }
}

const fn named(
    iri: &'static str,
    params: &'static [FunctionParam],
    key: &'static str,
    kind: ExprKind,
) -> FunctionRow {
    FunctionRow {
        iri,
        class: FunctionClass::NamedParameter,
        params,
        implementation: Implementation::Keyed { key, kind },
    }
}

const fn param(path: &'static str, value: ValueRule) -> ComponentParam {
    ComponentParam {
        path,
        optional: false,
        value,
        single: false,
        property_only: false,
    }
}

const fn optional_param(path: &'static str, value: ValueRule) -> ComponentParam {
    ComponentParam {
        path,
        optional: true,
        value,
        single: false,
        property_only: false,
    }
}

const fn native(
    iri: &'static str,
    params: &'static [ComponentParam],
    variants: &'static [&'static str],
) -> ComponentRow {
    ComponentRow {
        iri,
        params,
        status: ComponentStatus::Native,
        carrier: Carrier::Constraint(variants),
    }
}

const fn unimplemented(iri: &'static str, params: &'static [ComponentParam]) -> ComponentRow {
    ComponentRow {
        iri,
        params,
        status: ComponentStatus::Unimplemented,
        carrier: Carrier::None,
    }
}

/// `shnex:` function-class IRIs, which `crate::model` does not name because
/// nothing but this table reads them.
macro_rules! shnex_iri {
    ($local:literal) => {
        concat!("http://www.w3.org/ns/shacl-node-expr#", $local)
    };
}

// ── The rows ──────────────────────────────────────────────────────────────────

/// Every node-expression function the engine implements under a `sh:` or `shnex:`
/// IRI. `sparql:` functions are not rows: a `sparql:<NAME>` IRI is native exactly
/// when `crate::expression::sparql_ns_lowering` resolves `NAME`, which answers
/// from the SPARQL parser's own keyword table.
pub(crate) static FUNCTIONS: &[FunctionRow] = &[
    // SHACL 1.2 SPARQL Extensions §6.1 / §6.2.
    named(
        sh::SELECT_EXPRESSION,
        &[key(sh::SELECT), optional(sh::PREFIXES)],
        sh::SELECT,
        ExprKind::Select,
    ),
    named(
        sh::SPARQL_EXPR_EXPRESSION,
        &[key(sh::SPARQL_EXPR), optional(sh::PREFIXES)],
        sh::SPARQL_EXPR,
        ExprKind::Select,
    ),
    // SHACL 1.2 Node Expressions §4.
    named(
        shnex_iri!("ConcatExpression"),
        &[key(shnex::CONCAT)],
        shnex::CONCAT,
        ExprKind::Concat,
    ),
    named(
        shnex_iri!("CountExpression"),
        &[key(shnex::COUNT)],
        shnex::COUNT,
        ExprKind::Count,
    ),
    named(
        shnex_iri!("DistinctExpression"),
        &[key(shnex::DISTINCT)],
        shnex::DISTINCT,
        ExprKind::Distinct,
    ),
    FunctionRow {
        iri: shnex_iri!("EmptyExpression"),
        class: FunctionClass::Plain,
        params: &[],
        implementation: Implementation::Empty,
    },
    named(
        shnex_iri!("ExistsExpression"),
        &[key(shnex::EXISTS)],
        shnex::EXISTS,
        ExprKind::Exists,
    ),
    named(
        shnex_iri!("FilterShapeExpression"),
        &[key(shnex::FILTER_SHAPE), required(shnex::NODES)],
        shnex::FILTER_SHAPE,
        ExprKind::FilterShape,
    ),
    named(
        shnex_iri!("FindFirstExpression"),
        &[key(shnex::FIND_FIRST), optional(shnex::NODES)],
        shnex::FIND_FIRST,
        ExprKind::FindFirst,
    ),
    named(
        shnex_iri!("FlatMapExpression"),
        &[key(shnex::FLAT_MAP), optional(shnex::NODES)],
        shnex::FLAT_MAP,
        ExprKind::FlatMap,
    ),
    named(
        shnex_iri!("IfExpression"),
        &[key(shnex::IF), optional(shnex::THEN), optional(shnex::ELSE)],
        shnex::IF,
        ExprKind::If,
    ),
    named(
        shnex_iri!("InstancesOfExpression"),
        &[key(shnex::INSTANCES_OF)],
        shnex::INSTANCES_OF,
        ExprKind::InstancesOf,
    ),
    named(
        shnex_iri!("IntersectionExpression"),
        &[key(shnex::INTERSECTION)],
        shnex::INTERSECTION,
        ExprKind::Intersection,
    ),
    named(
        shnex_iri!("LimitExpression"),
        &[key(shnex::LIMIT), required(shnex::NODES)],
        shnex::LIMIT,
        ExprKind::Limit,
    ),
    // Both collection cells are key parameters; `rdf:first` is the one that
    // selects the list reading, `rdf:rest` is read by that arm.
    named(
        shnex_iri!("ListExpression"),
        &[key(rdf::FIRST), key(rdf::REST)],
        rdf::FIRST,
        ExprKind::List,
    ),
    named(
        shnex_iri!("MatchAllExpression"),
        &[key(shnex::MATCH_ALL), optional(shnex::NODES)],
        shnex::MATCH_ALL,
        ExprKind::MatchAll,
    ),
    named(
        shnex_iri!("MaxExpression"),
        &[key(shnex::MAX)],
        shnex::MAX,
        ExprKind::Max,
    ),
    named(
        shnex_iri!("MinExpression"),
        &[key(shnex::MIN)],
        shnex::MIN,
        ExprKind::Min,
    ),
    named(
        shnex_iri!("NodesMatchingExpression"),
        &[key(shnex::NODES_MATCHING)],
        shnex::NODES_MATCHING,
        ExprKind::NodesMatching,
    ),
    named(
        shnex_iri!("OffsetExpression"),
        &[key(shnex::OFFSET), required(shnex::NODES)],
        shnex::OFFSET,
        ExprKind::Offset,
    ),
    named(
        shnex_iri!("OrderByExpression"),
        &[
            key(shnex::ORDER_BY),
            required(shnex::NODES),
            optional(shnex::DESC),
        ],
        shnex::ORDER_BY,
        ExprKind::OrderBy,
    ),
    named(
        shnex_iri!("PathValuesExpression"),
        &[key(shnex::PATH_VALUES), optional(shnex::FOCUS_NODE)],
        shnex::PATH_VALUES,
        ExprKind::Path,
    ),
    named(
        shnex_iri!("RemoveExpression"),
        &[key(shnex::REMOVE), required(shnex::NODES)],
        shnex::REMOVE,
        ExprKind::Remove,
    ),
    named(
        shnex_iri!("SumExpression"),
        &[key(shnex::SUM)],
        shnex::SUM,
        ExprKind::Sum,
    ),
    named(
        shnex_iri!("VarExpression"),
        &[key(shnex::VAR)],
        shnex::VAR,
        ExprKind::Var,
    ),
    // §4.5.3: a list-parameter function, so its own IRI is its call key.
    FunctionRow {
        iri: shnex::CONFORMS_TO_SHAPE,
        class: FunctionClass::ListParameter,
        params: &[],
        implementation: Implementation::Keyed {
            key: shnex::CONFORMS_TO_SHAPE,
            kind: ExprKind::ConformsToShape,
        },
    },
];

/// The parameters whose optionality the SPECIFICATION TEXT decides differently
/// from the vocabulary file's `sh:optional`, as `(function IRI, parameter path)`.
///
/// SHACL 1.2 SPARQL Extensions §6.1 and §6.2 make `sh:prefixes` optional on both
/// SPARQL-based node expressions ("The node expression may also have a value for
/// `sh:prefixes`"), while the vocabulary's `sh:SelectExpression-prefixes` and
/// `sh:SPARQLExprExpression-prefixes` carry no `sh:optional true`. The spec text
/// is authoritative, so the table records them optional and the ratchet admits
/// exactly these disagreements.
pub static SPEC_TEXT_OPTIONALITY: &[(&str, &str)] = &[
    (sh::SELECT_EXPRESSION, sh::PREFIXES),
    (sh::SPARQL_EXPR_EXPRESSION, sh::PREFIXES),
];

/// Every constraint component SHACL 1.2 Core and SHACL 1.2 SPARQL Extensions
/// declare, with the engine's status for each.
pub(crate) static COMPONENTS: &[ComponentRow] = &[
    native(
        sh::AND_CONSTRAINT_COMPONENT,
        &[param(sh::AND, ValueRule::ShapeList)],
        &["And"],
    ),
    native(
        sh::CLASS_CONSTRAINT_COMPONENT,
        &[param(sh::CLASS, ValueRule::IriOrIriList)],
        &["Class"],
    ),
    native(
        sh::CLOSED_CONSTRAINT_COMPONENT,
        &[
            param(sh::CLOSED, ValueRule::ClosedValue).single(),
            optional_param(sh::IGNORED_PROPERTIES, ValueRule::IriList).single(),
        ],
        &["Closed"],
    ),
    native(
        sh::DATATYPE_CONSTRAINT_COMPONENT,
        &[param(sh::DATATYPE, ValueRule::IriOrIriList).single()],
        &["Datatype"],
    ),
    native(
        sh::DISJOINT_CONSTRAINT_COMPONENT,
        &[param(sh::DISJOINT, ValueRule::Iri)],
        &["Disjoint"],
    ),
    native(
        sh::EQUALS_CONSTRAINT_COMPONENT,
        &[param(sh::EQUALS, ValueRule::Iri)],
        &["Equals"],
    ),
    native(
        sh::HAS_VALUE_CONSTRAINT_COMPONENT,
        &[param(sh::HAS_VALUE, ValueRule::Term)],
        &["HasValue"],
    ),
    native(
        sh::IN_CONSTRAINT_COMPONENT,
        &[param(sh::IN, ValueRule::TermList).single()],
        &["In"],
    ),
    native(
        sh::LANGUAGE_IN_CONSTRAINT_COMPONENT,
        &[param(sh::LANGUAGE_IN, ValueRule::LanguageTagList).single()],
        &["LanguageIn"],
    ),
    native(
        sh::LESS_THAN_CONSTRAINT_COMPONENT,
        &[param(sh::LESS_THAN, ValueRule::Iri).property_only()],
        &["LessThan"],
    ),
    native(
        sh::LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT,
        &[param(sh::LESS_THAN_OR_EQUALS, ValueRule::Iri).property_only()],
        &["LessThanOrEquals"],
    ),
    native(
        sh::MAX_COUNT_CONSTRAINT_COMPONENT,
        &[param(sh::MAX_COUNT, ValueRule::NonNegativeInteger)
            .single()
            .property_only()],
        &["MaxCount"],
    ),
    native(
        sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT,
        &[param(sh::MAX_EXCLUSIVE, ValueRule::Literal).single()],
        &["MaxExclusive"],
    ),
    native(
        sh::MAX_INCLUSIVE_CONSTRAINT_COMPONENT,
        &[param(sh::MAX_INCLUSIVE, ValueRule::Literal).single()],
        &["MaxInclusive"],
    ),
    native(
        sh::MAX_LENGTH_CONSTRAINT_COMPONENT,
        &[param(sh::MAX_LENGTH, ValueRule::NonNegativeInteger).single()],
        &["MaxLength"],
    ),
    native(
        sh::MIN_COUNT_CONSTRAINT_COMPONENT,
        &[param(sh::MIN_COUNT, ValueRule::NonNegativeInteger)
            .single()
            .property_only()],
        &["MinCount"],
    ),
    native(
        sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT,
        &[param(sh::MIN_EXCLUSIVE, ValueRule::Literal).single()],
        &["MinExclusive"],
    ),
    native(
        sh::MIN_INCLUSIVE_CONSTRAINT_COMPONENT,
        &[param(sh::MIN_INCLUSIVE, ValueRule::Literal).single()],
        &["MinInclusive"],
    ),
    native(
        sh::MIN_LENGTH_CONSTRAINT_COMPONENT,
        &[param(sh::MIN_LENGTH, ValueRule::NonNegativeInteger).single()],
        &["MinLength"],
    ),
    native(
        sh::MEMBER_SHAPE_CONSTRAINT_COMPONENT,
        &[param(sh::MEMBER_SHAPE, ValueRule::Shape)],
        &["MemberShape"],
    ),
    native(
        sh::MIN_LIST_LENGTH_CONSTRAINT_COMPONENT,
        &[param(sh::MIN_LIST_LENGTH, ValueRule::NonNegativeInteger).single()],
        &["MinListLength"],
    ),
    native(
        sh::MAX_LIST_LENGTH_CONSTRAINT_COMPONENT,
        &[param(sh::MAX_LIST_LENGTH, ValueRule::NonNegativeInteger).single()],
        &["MaxListLength"],
    ),
    native(
        sh::UNIQUE_MEMBERS_CONSTRAINT_COMPONENT,
        &[param(sh::UNIQUE_MEMBERS, ValueRule::Boolean).single()],
        &["UniqueMembers"],
    ),
    native(
        sh::NODE_CONSTRAINT_COMPONENT,
        &[param(sh::NODE, ValueRule::Shape)],
        &["Node"],
    ),
    native(
        sh::NODE_KIND_CONSTRAINT_COMPONENT,
        &[param(sh::NODE_KIND, ValueRule::NodeKind).single()],
        &["NodeKind"],
    ),
    native(
        sh::NOT_CONSTRAINT_COMPONENT,
        &[param(sh::NOT, ValueRule::Shape)],
        &["Not"],
    ),
    native(
        sh::OR_CONSTRAINT_COMPONENT,
        &[param(sh::OR, ValueRule::ShapeList)],
        &["Or"],
    ),
    native(
        sh::PATTERN_CONSTRAINT_COMPONENT,
        &[
            param(sh::PATTERN, ValueRule::StringLiteral).single(),
            optional_param(sh::FLAGS, ValueRule::StringLiteral).single(),
        ],
        &["Pattern"],
    ),
    ComponentRow {
        iri: sh::PROPERTY_CONSTRAINT_COMPONENT,
        params: &[param(sh::PROPERTY, ValueRule::Shape)],
        status: ComponentStatus::Native,
        carrier: Carrier::ShapeField("property_shapes"),
    },
    native(
        sh::QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT,
        &[
            param(sh::QUALIFIED_MAX_COUNT, ValueRule::NonNegativeInteger).single(),
            param(sh::QUALIFIED_VALUE_SHAPE, ValueRule::Shape)
                .single()
                .property_only(),
            optional_param(sh::QUALIFIED_VALUE_SHAPES_DISJOINT, ValueRule::Boolean).single(),
        ],
        &["QualifiedValueShape"],
    ),
    native(
        sh::QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT,
        &[
            param(sh::QUALIFIED_MIN_COUNT, ValueRule::NonNegativeInteger).single(),
            param(sh::QUALIFIED_VALUE_SHAPE, ValueRule::Shape)
                .single()
                .property_only(),
            optional_param(sh::QUALIFIED_VALUE_SHAPES_DISJOINT, ValueRule::Boolean).single(),
        ],
        &["QualifiedValueShape"],
    ),
    ComponentRow {
        iri: sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT,
        params: &[
            param(sh::REIFIER_SHAPE, ValueRule::Shape).single(),
            optional_param(sh::REIFICATION_REQUIRED, ValueRule::Boolean).single(),
        ],
        status: ComponentStatus::Native,
        carrier: Carrier::ShapeField("reifier_shapes"),
    },
    unimplemented(
        sh::ROOT_CLASS_CONSTRAINT_COMPONENT,
        &[param(sh::ROOT_CLASS, ValueRule::IriOrIriList)],
    ),
    unimplemented(
        sh::SINGLE_LINE_CONSTRAINT_COMPONENT,
        &[param(sh::SINGLE_LINE, ValueRule::Boolean).single()],
    ),
    unimplemented(
        sh::SOME_VALUE_CONSTRAINT_COMPONENT,
        &[param(sh::SOME_VALUE, ValueRule::Shape)],
    ),
    unimplemented(
        sh::SUBSET_OF_CONSTRAINT_COMPONENT,
        &[param(sh::SUBSET_OF, ValueRule::Path)],
    ),
    native(
        sh::UNIQUE_LANG_CONSTRAINT_COMPONENT,
        &[param(sh::UNIQUE_LANG, ValueRule::Boolean)
            .single()
            .property_only()],
        &["UniqueLang"],
    ),
    unimplemented(
        sh::UNIQUE_VALUES_FOR_CONSTRAINT_COMPONENT,
        &[param(sh::UNIQUE_VALUES_FOR, ValueRule::IriOrIriList)],
    ),
    native(
        sh::XONE_CONSTRAINT_COMPONENT,
        &[param(sh::XONE, ValueRule::ShapeList)],
        &["Xone"],
    ),
    // SHACL 1.2 SPARQL Extensions.
    native(
        sh::SPARQL_CONSTRAINT_COMPONENT,
        &[param(sh::SPARQL, ValueRule::SparqlConstraint)],
        &["Sparql"],
    ),
    // SHACL 1.2 Node Expressions §7.
    native(
        sh::EXPRESSION_CONSTRAINT_COMPONENT,
        &[param(sh::EXPRESSION, ValueRule::NodeExpression)],
        &["Expression"],
    ),
    native(
        sh::NODE_BY_EXPRESSION_CONSTRAINT_COMPONENT,
        &[param(sh::NODE_BY_EXPRESSION, ValueRule::NodeExpression)],
        &["NodeByExpression"],
    ),
];

/// The `sh:` node-expression spellings of SHACL Advanced Features that SHACL 1.2
/// Node Expressions re-spells under `shnex:`, plus the AF-only keys. Each parses
/// to the same IR arm as its `shnex:` counterpart.
///
/// `sh:limit` / `sh:offset` / `sh:orderby` / `sh:desc` are OPERANDS here, not
/// primary keys: on the AF surface they WRAP the node's own core expression,
/// whereas their `shnex:` counterparts are named-parameter functions carrying
/// their own `shnex:nodes` operand and so are cores in their own right.
pub(crate) static KEY_ALIASES: &[KeyAlias] = &[
    af(sh::PATH, AliasRole::Primary(ExprKind::Path)),
    af(sh::FILTER_SHAPE, AliasRole::Primary(ExprKind::FilterShape)),
    af(sh::UNION, AliasRole::Primary(ExprKind::Union)),
    af(sh::INTERSECTION, AliasRole::Primary(ExprKind::Intersection)),
    af(sh::IF, AliasRole::Primary(ExprKind::If)),
    af(sh::COUNT, AliasRole::Primary(ExprKind::Count)),
    af(sh::DISTINCT, AliasRole::Primary(ExprKind::Distinct)),
    af(sh::MIN, AliasRole::Primary(ExprKind::Min)),
    af(sh::MAX, AliasRole::Primary(ExprKind::Max)),
    af(sh::SUM, AliasRole::Primary(ExprKind::Sum)),
    af(sh::EXISTS, AliasRole::Primary(ExprKind::Exists)),
    af(sh::NODES, AliasRole::Operand),
    af(sh::THEN, AliasRole::Operand),
    af(sh::ELSE, AliasRole::Operand),
    af(sh::LIMIT, AliasRole::Operand),
    af(sh::OFFSET, AliasRole::Operand),
    af(sh::ORDERBY, AliasRole::Operand),
    af(sh::DESC, AliasRole::Operand),
];

const fn af(iri: &'static str, role: AliasRole) -> KeyAlias {
    KeyAlias {
        iri,
        role,
        source: AliasSource::ShaclAdvancedFeatures,
    }
}

/// The argument reference of a custom function body (SHACL 1.2 Node Expressions
/// §6.3): a node-expression key that no function declaration declares, because it
/// is read only inside a declared body.
pub(crate) const ARGUMENT_REFERENCE: (&str, ExprKind) = (shnex::ARG, ExprKind::Arg);

/// The SPARQL function names `shnex-sparql.ttl` spells differently from the
/// SPARQL Working Group's `sparql-ns.ttl`.
///
/// As a matter of the two specifications' text: `shnex-sparql.ttl` declares
/// `sparql:plus` (with `shnex-sparql:infixOperator "+"`) and `sparql:encode`,
/// while `sparql-ns.ttl` names the same two functions `sparql:add` and
/// `sparql:encodeForUri`. Both spellings are accepted; each alias lowers to the
/// same SPARQL surface form as the name it aliases.
pub(crate) static SPARQL_ALIASES: &[SparqlAlias] = &[
    SparqlAlias {
        local: "plus",
        sparql_ns_name: "add",
        form: SparqlCallForm::Infix("+"),
        source: AliasSource::ShnexSparqlVocabulary,
    },
    SparqlAlias {
        local: "encode",
        sparql_ns_name: "encodeForUri",
        form: SparqlCallForm::Call("ENCODE_FOR_URI"),
        source: AliasSource::ShnexSparqlVocabulary,
    },
];

/// The severities the engine recognises as built-in (SHACL 1.2 Core §3.6.3).
pub(crate) static SEVERITIES: &[&str] = &[sh::VIOLATION, sh::WARNING, sh::INFO];

/// The target predicates the engine implements (SHACL 1.2 Core §2.1.3, plus the
/// SHACL-SPARQL `sh:target`).
pub(crate) static TARGETS: &[TargetRow] = &[
    TargetRow {
        predicate: sh::TARGET_CLASS,
    },
    TargetRow {
        predicate: sh::TARGET_SUBJECTS_OF,
    },
    TargetRow {
        predicate: sh::TARGET_OBJECTS_OF,
    },
    TargetRow {
        predicate: sh::TARGET_NODE,
    },
    TargetRow {
        predicate: sh::TARGET,
    },
];

/// The rule types the engine implements.
pub(crate) static RULE_TYPES: &[&str] = &[sh::TRIPLE_RULE, sh::SPARQL_RULE];

/// The canonical text of every row, one fact per line, in table order.
///
/// A prepared shapes product's stage id folds this in, so a product prepared
/// under one table is refused by a build whose table says something else.
pub(crate) fn canonical_lines() -> Vec<String> {
    let mut out = Vec::new();
    for row in FUNCTIONS {
        let implementation = match row.implementation {
            Implementation::Keyed { key, kind } => format!("keyed {key} -> {}", kind.label()),
            Implementation::Empty => "empty".to_owned(),
        };
        out.push(format!(
            "function {} {} {implementation}",
            row.iri,
            row.class.iri()
        ));
        for p in row.params {
            out.push(format!(
                "  param {} key={} optional={}",
                p.path, p.key, p.optional
            ));
        }
    }
    for (function, path) in SPEC_TEXT_OPTIONALITY {
        out.push(format!("spec-text-optional {function} {path}"));
    }
    for row in COMPONENTS {
        let status = match row.status {
            ComponentStatus::Native => "native",
            ComponentStatus::Unimplemented => "unimplemented",
        };
        let carrier = match row.carrier {
            Carrier::Constraint(variants) => format!("constraint {}", variants.join(",")),
            Carrier::ShapeField(field) => format!("field {field}"),
            Carrier::None => "none".to_owned(),
        };
        out.push(format!("component {} {status} {carrier}", row.iri));
        for p in row.params {
            out.push(format!(
                "  param {} optional={} value={} single={} property-only={}",
                p.path,
                p.optional,
                p.value.label(),
                p.single,
                p.property_only
            ));
        }
    }
    for alias in KEY_ALIASES {
        let role = match alias.role {
            AliasRole::Primary(kind) => format!("primary {}", kind.label()),
            AliasRole::Operand => "operand".to_owned(),
        };
        out.push(format!(
            "alias {} {role} {}",
            alias.iri,
            alias.source.label()
        ));
    }
    out.push(format!(
        "argument-reference {} -> {}",
        ARGUMENT_REFERENCE.0,
        ARGUMENT_REFERENCE.1.label()
    ));
    for alias in SPARQL_ALIASES {
        out.push(format!(
            "sparql-alias {} = {} {:?} {}",
            alias.local,
            alias.sparql_ns_name,
            alias.form,
            alias.source.label()
        ));
    }
    for severity in SEVERITIES {
        out.push(format!("severity {severity}"));
    }
    for target in TARGETS {
        out.push(format!("target {}", target.predicate));
    }
    for rule in RULE_TYPES {
        out.push(format!("rule-type {rule}"));
    }
    out
}
