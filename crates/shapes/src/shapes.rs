// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL shapes graph parser.
//!
//! Parses a SHACL Core shapes graph from a frozen [`RdfDataset`] into a fully
//! typed [`Shapes`] structure. No evaluation logic lives here. Covers full
//! SHACL Core: all six property-path forms (§2.3.1),
//! property-pair constraints (§4.3), qualified value shapes (§4.5.4–4.5.5), and
//! SHACL-AF SPARQL constraints/targets.  Malformed constructs (e.g. a literal
//! `sh:equals` object, a one-member sequence path) cause a hard `Err` rather
//! than a silent skip.

use std::sync::{Arc, OnceLock};

use ::purrdf::FastMap;
use ::purrdf::FastSet;
use ::purrdf::RdfDataset;

use purrdf_sparql_eval::{AggregateRegistry, UserFunctionRegistry};

use crate::components::{ComponentRegistry, severity_from_term};
use crate::data::{GraphFilter, native_quads};
use crate::error::ShapesError;
use crate::expression::NodeExpr;
use crate::imports::{ShapesImports, resolve_shapes_imports};
use crate::model::{BoxRoleVocab, rdf, sh};
use crate::provenance::ParseProvenance;
use crate::report::Severity;
use crate::term::{Literal, NamedNode, Term};
use parser::annotations::Annotated;

pub(crate) mod link;
mod parser;

pub(crate) use parser::node_expr::boolean_value as parser_boolean;
pub(crate) mod prefixes;

/// Re-derive a shapes graph's `sh:SPARQLFunction` declarations from the shapes
/// dataset it carries.
///
/// Re-exported here because the prepared-product codec (`crate::product`) is not a
/// descendant of this module and so cannot name `parser`, which stays private: the
/// sub-parsers are this module's internals, and exactly one of them is a published
/// step that a restore has to re-run.
pub(crate) use parser::functions::register_declared_sparql_functions;

// ── Public types ───────────────────────────────────────────────────────────────

/// The `sh:nodeKind` value IRI mapped to a typed enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKindValue {
    /// `sh:IRI`
    Iri,
    /// `sh:BlankNode`
    BlankNode,
    /// `sh:Literal`
    Literal,
    /// `sh:BlankNodeOrIRI`
    BlankNodeOrIri,
    /// `sh:BlankNodeOrLiteral`
    BlankNodeOrLiteral,
    /// `sh:IRIOrLiteral`
    IriOrLiteral,
    /// `sh:TripleTerm` (SHACL 1.2 Core §4.1.3): an RDF 1.2 triple term, which
    /// matches no other node kind.
    TripleTerm,
}

impl NodeKindValue {
    /// The `sh:NodeKind` instance IRI this value is spelled with.
    #[must_use]
    pub const fn iri(&self) -> &'static str {
        match self {
            Self::Iri => sh::IRI,
            Self::BlankNode => sh::BLANK_NODE,
            Self::Literal => sh::LITERAL,
            Self::BlankNodeOrIri => sh::BLANK_NODE_OR_IRI,
            Self::BlankNodeOrLiteral => sh::BLANK_NODE_OR_LITERAL,
            Self::IriOrLiteral => sh::IRI_OR_LITERAL,
            Self::TripleTerm => sh::TRIPLE_TERM,
        }
    }

    /// The node kind `iri` names, if it is one of the seven `sh:NodeKind`
    /// instances.
    #[must_use]
    pub fn from_iri(iri: &str) -> Option<Self> {
        [
            Self::Iri,
            Self::BlankNode,
            Self::Literal,
            Self::BlankNodeOrIri,
            Self::BlankNodeOrLiteral,
            Self::IriOrLiteral,
            Self::TripleTerm,
        ]
        .into_iter()
        .find(|kind| kind.iri() == iri)
    }

    /// Whether this is one of the four BASIC node kinds — the only ones SHACL 1.2
    /// Core §4.1.3 permits as members of a `sh:nodeKind` list ("members of those
    /// lists in a shape are one of the following four instances of the class
    /// sh:NodeKind: sh:BlankNode, sh:IRI, sh:Literal, and sh:TripleTerm").
    #[must_use]
    pub const fn is_basic(&self) -> bool {
        matches!(
            self,
            Self::Iri | Self::BlankNode | Self::Literal | Self::TripleTerm
        )
    }
}

/// A SHACL property path (spec §2.3.1 — all six path forms are modelled).
#[derive(Debug, Clone)]
pub enum Path {
    /// A plain IRI predicate path (`ex:name`).
    Predicate(NamedNode),
    /// An inverse path (`[ sh:inversePath ex:parent ]`).
    Inverse(Box<Self>),
    /// A sequence path — an RDF list of at least two paths in path position
    /// (`sh:path ( ex:a ex:b )`).
    Sequence(Vec<Self>),
    /// An alternative path (`[ sh:alternativePath ( ex:a ex:b ) ]`).
    Alternative(Vec<Self>),
    /// A zero-or-more path (`[ sh:zeroOrMorePath ex:next ]`) — reflexive
    /// transitive closure.
    ZeroOrMore(Box<Self>),
    /// A one-or-more path (`[ sh:oneOrMorePath ex:next ]`) — transitive closure.
    OneOrMore(Box<Self>),
    /// A zero-or-one path (`[ sh:zeroOrOnePath ex:next ]`).
    ZeroOrOne(Box<Self>),
}

/// A SHACL target declaration on a node shape.
#[derive(Debug, Clone)]
pub enum Target {
    /// `sh:targetClass ex:SomeClass`
    Class(NamedNode),
    /// `sh:targetSubjectsOf ex:pred`
    SubjectsOf(NamedNode),
    /// `sh:targetObjectsOf ex:pred`
    ObjectsOf(NamedNode),
    /// `sh:targetNode ex:SomeNode` (or a literal, or a triple term): a CONSTANT
    /// node expression, whose output is itself.
    Node(Term),
    /// An implicit class target (SHACL 1.2 Core, "Implicit Class Targets and
    /// sh:ShapeClass"): "If s is a SHACL instance of sh:NodeShape or
    /// sh:PropertyShape in a shapes graph SG and s is also a SHACL instance of
    /// rdfs:Class in SG then the set of SHACL instances of s in a data graph DG is
    /// a target from DG for s in SG", and "If s is a SHACL instance of
    /// sh:ShapeClass in a shapes graph SG then the set of SHACL instances of s in
    /// a data graph DG is a target from DG for s in SG." The term is the shape
    /// node itself, always an IRI (a blank one is an ill-formed shape).
    ImplicitClass(Term),
    /// `sh:targetNode [ … ]` — a STRUCTURED node expression (SHACL 1.2 Core, "Node
    /// targets"): "If s is a shape in a shapes graph SG and s has value expr for
    /// sh:targetNode in SG, then the output nodes of evalExpr(expr, data graph, s,
    /// {}) are targets for the data graph DG as focus graph." Evaluated once per
    /// validation, with the SHAPE as the focus node and the data graph as the
    /// focus graph. A constant value is [`Target::Node`]; the empty expression
    /// targets nothing and is not recorded.
    NodeExpression(NodeExpr),
    /// `sh:targetWhere w` (SHACL 1.2 Core, "Where Targets"): "If s is a shape in
    /// a shapes graph SG and s has value w for sh:targetWhere in SG then the set
    /// of nodes in a data graph DG that conform to w is a target from DG for s in
    /// SG." The "nodes in a data graph" are the NODES of the graph as RDF 1.2
    /// Concepts defines them — "the set of subjects and objects of the asserted
    /// triples of the graph" — so a triple term that is an object is a candidate
    /// and a term occurring only inside a triple term is not.
    Where(Box<Shape>),
    /// `sh:target [ rdf:type sh:SPARQLTarget ; sh:select "SELECT ?this …" ]`
    /// or a `sh:target [ rdf:type <CustomTargetType> ; <param> <value> ]` that
    /// has been instantiated from a `sh:SPARQLTargetType` declaration.
    ///
    /// The query is validated (parseable + SELECT-form) at shape-load time. The
    /// native SPARQL engine re-parses the text at eval time, so only the query
    /// string is retained. `substitutions` holds pre-bound parameter values for
    /// `sh:SPARQLTargetType` instances; it is empty for plain `sh:SPARQLTarget`.
    Sparql {
        /// The SPARQL SELECT query text (with any injected PREFIX header).
        select: String,
        /// Pre-bound parameter substitutions for `sh:SPARQLTargetType` instances.
        substitutions: Vec<(String, Term)>,
    },
}

/// A parsed `sh:SPARQLTargetType` declaration.
#[derive(Debug, Clone)]
pub struct SparqlTargetType {
    /// The target type IRI.
    pub id: Term,
    /// Parameters in declaration order, each naming the predicate that supplies
    /// the value at a target instance.
    pub params: Vec<TargetTypeParam>,
    /// The raw SPARQL SELECT query text (without prefix header; the header is
    /// injected when the target type is instantiated on a shape).
    pub select: String,
}

/// A single parameter of a `sh:SPARQLTargetType` declaration.
#[derive(Debug, Clone)]
pub struct TargetTypeParam {
    /// The predicate IRI that supplies the parameter value at the target instance.
    pub predicate: NamedNode,
    /// The SPARQL variable name (local name of the predicate).
    pub var: String,
}

/// The SPARQL validator form carried by a custom constraint component constraint.
#[derive(Debug, Clone)]
pub enum ComponentValidator {
    /// An `ASK` query validator (`sh:SPARQLAskValidator`).
    Ask {
        /// The raw `ASK` query text (`sh:ask`).
        ask: String,
    },
    /// A `SELECT` query validator (`sh:SPARQLSelectValidator`).
    Select {
        /// The raw `SELECT` query text (`sh:select`).
        select: String,
    },
}

/// A SHACL-SPARQL result annotation (`sh:ResultAnnotation`): a property the
/// validation results of a SPARQL-based constraint or validator carry beyond the
/// report vocabulary.
///
/// SHACL 1.2 SPARQL Extensions, "Annotation Properties": "Any such annotation
/// property needs to be declared via a value of sh:resultAnnotation at the subject
/// of the sh:select or sh:ask triple." For each solution, "Use the value of the
/// property sh:annotationVarName. If no such value exists, use the local name of
/// the value of sh:annotationProperty as the variable name. If a variable name
/// could be determined, then the SHACL processor copies the binding for the given
/// variable as a value for the property specified using sh:annotationProperty into
/// the validation result that is being produced for the current solution. If the
/// variable has no binding in the result set solution, then the values of
/// sh:annotationValue are used, if present."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultAnnotation {
    /// The annotation property (`sh:annotationProperty`, exactly one IRI).
    pub property: NamedNode,
    /// The SPARQL variable the value is copied from: `sh:annotationVarName`, or
    /// else the local name of [`Self::property`]; `None` when neither names a
    /// SPARQL variable, so only [`Self::default_values`] apply.
    pub variable: Option<String>,
    /// The `sh:annotationValue` values, used when the variable is unbound in a
    /// solution, in canonical term order.
    pub default_values: Vec<Term>,
}

/// How `sh:closed` decides the properties a value node may carry (SHACL 1.2
/// Core §7.9.1).
#[derive(Debug, Clone)]
pub enum ClosedMode {
    /// `sh:closed true`: "P is the set of IRI properties that can be reached from
    /// the current shape via the SPARQL path `sh:property/sh:path`." `rdf:type`
    /// is permitted only when `sh:ignoredProperties` lists it.
    Declared,
    /// `sh:closed sh:ByTypes`: "P is the set of IRI properties that can be
    /// reached from the value node via the following algorithm, plus `rdf:type`"
    /// — `collectProperties(T)` for each `rdf:type` `T` of the value node in the
    /// data graph. Everything that algorithm reads besides the value node's own
    /// types is in the shapes graph, so the whole of it is resolved at load into
    /// the shared [`ClosedTypeIndex`]; validation only reads the value node's
    /// types and looks them up.
    ByTypes(Arc<ClosedTypeIndex>),
}

/// The `sh:closed sh:ByTypes` index of one shapes graph: for every node `T` for
/// which SHACL 1.2 Core §7.9.1's `collectProperties(T)` is non-empty, the IRI
/// properties it collects.
///
/// The algorithm, verbatim from the specification:
///
/// ```text
/// function collectProperties(S)
///     add all IRI properties that can be reached from S via the SPARQL path
///             sh:property/sh:path
///     if S is a SHACL instance of rdfs:Class in the shapes graph {
///         for each triple in the shapes graph matching (S rdfs:subClassOf ?o)
///             collectProperties(?o)
///         for each triple in the shapes graph matching (?s sh:targetClass S)
///             collectProperties(?s)
///     }
///     if S is a SHACL instance of sh:NodeShape in the shapes graph
///         for each triple in the shapes graph matching (S sh:node ?o)
///             collectProperties(?o)
/// for each rdf:type T of the value node in the data graph
///     collectProperties(T)
/// ```
///
/// Every read inside `collectProperties` is a read of the SHAPES graph, so the
/// result for each `T` is a function of the shapes graph alone and is computed
/// once, at load, visiting each node at most once per `T` as the specification
/// requires. A `T` whose collection is empty has no entry: it permits nothing, and
/// [`Self::properties`] answers it with the empty slice.
///
/// Held by `Arc` so every `sh:closed sh:ByTypes` constraint of a shapes graph
/// shares one index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClosedTypeIndex {
    /// `(T, properties)`, sorted by `T` in canonical term order; each property
    /// list is sorted by IRI, deduplicated and non-empty.
    entries: Vec<(Term, Vec<NamedNode>)>,
}

impl ClosedTypeIndex {
    /// Build the index from `(T, properties)` pairs in any order, canonicalizing
    /// it: keys sorted in canonical term order, each property list sorted by IRI
    /// and deduplicated, and empty entries dropped.
    ///
    /// # Errors
    ///
    /// Returns an error when one `T` appears twice, because two entries for one
    /// node would make [`Self::properties`] answer with only one of them.
    pub(crate) fn from_entries(mut entries: Vec<(Term, Vec<NamedNode>)>) -> Result<Self, String> {
        entries.retain(|(_, properties)| !properties.is_empty());
        for (_, properties) in &mut entries {
            properties.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            properties.dedup();
        }
        entries.sort_by(|(a, _), (b, _)| crate::term::canonical_cmp(a, b));
        if let Some(pair) = entries.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(format!(
                "the sh:closed sh:ByTypes index names {} twice",
                pair[0].0
            ));
        }
        Ok(Self { entries })
    }

    /// The index `entries` spell, or `None` when they are not already in the
    /// canonical form [`Self::from_entries`] produces — the prepared-product
    /// reader's gate, which refuses rather than re-sorts, so a product's bytes
    /// stay the canonical form of the value they decode to.
    pub(crate) fn from_canonical_entries(entries: Vec<(Term, Vec<NamedNode>)>) -> Option<Self> {
        let types_ascend = entries.windows(2).all(|pair| {
            crate::term::canonical_cmp(&pair[0].0, &pair[1].0) == std::cmp::Ordering::Less
        });
        let properties_ascend = entries.iter().all(|(_, properties)| {
            !properties.is_empty()
                && properties
                    .windows(2)
                    .all(|pair| pair[0].as_str() < pair[1].as_str())
        });
        (types_ascend && properties_ascend).then_some(Self { entries })
    }

    /// The IRI properties `collectProperties(ty)` reaches, sorted by IRI; empty
    /// when it reaches none.
    #[must_use]
    pub fn properties(&self, ty: &Term) -> &[NamedNode] {
        self.entries
            .binary_search_by(|(key, _)| crate::term::canonical_cmp(key, ty))
            .map_or(&[], |position| self.entries[position].1.as_slice())
    }

    /// Every `(T, properties)` entry, in canonical term order of `T`.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = (&Term, &[NamedNode])> {
        self.entries
            .iter()
            .map(|(ty, properties)| (ty, properties.as_slice()))
    }

    /// Whether no node collects any property.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A single SHACL constraint on a shape or property shape.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// One `sh:class` value: a single class IRI (`sh:class ex:C`, one member) or a
    /// SHACL list of class IRIs (`sh:class ( ex:C ex:D )`).
    ///
    /// SHACL 1.2 Core §4.1.1: "when $class is a blank node SHACL list then the set
    /// consists of exactly the members of the list. For each value node that is
    /// either a literal, or a non-literal that is not a SHACL instance of any of
    /// the classes in the data graph, there is a validation result". So a value
    /// node conforms when it is an instance of ANY member; separate `sh:class`
    /// values stay separate constraints (a conjunction). Never empty.
    Class(Vec<NamedNode>),
    /// One `sh:datatype` value: a single datatype IRI or a SHACL list of them
    /// (SHACL 1.2 Core §4.1.2 — a value node conforms when its datatype matches
    /// ANY member). Never empty.
    Datatype(Vec<NamedNode>),
    /// One `sh:nodeKind` value: a single `sh:NodeKind` IRI or a SHACL list of the
    /// four basic kinds (SHACL 1.2 Core §4.1.3 — a value node conforms when it
    /// matches ANY member). Never empty.
    NodeKind(Vec<NodeKindValue>),
    /// `sh:minCount 1`
    MinCount(u64),
    /// `sh:maxCount 5`
    MaxCount(u64),
    /// `sh:in ( … )` — list of allowed values.
    In(Vec<Term>),
    /// `sh:hasValue ex:v`
    HasValue(Term),
    /// `sh:pattern "…"` with optional `sh:flags "…"`.
    Pattern {
        /// The regex string.
        regex: String,
        /// Optional regex flags (e.g. `"i"`).
        flags: Option<String>,
        /// Once-compiled regex cache: the pattern is compiled at most once per
        /// `Constraint` instance regardless of how many focus nodes are validated.
        /// `Arc` makes the field `Clone`; `OnceLock` makes it `Send + Sync`.
        /// `Err` carries the precise
        /// [`XsdRegexError`](purrdf_core::xsd_regex::XsdRegexError) — not a lossy
        /// string — so per-value violation semantics (bad regex → violation, not
        /// hard abort) survive *and* the report can name the exact offending
        /// construct via the compiler's own message.
        compiled: Arc<
            OnceLock<
                Result<
                    purrdf_core::xsd_regex::CompiledPattern,
                    purrdf_core::xsd_regex::XsdRegexError,
                >,
            >,
        >,
    },
    /// `sh:minLength 3`
    MinLength(u64),
    /// `sh:maxLength 255`
    MaxLength(u64),
    /// `sh:uniqueLang true`
    UniqueLang(bool),
    /// `sh:languageIn ( "en" "fr" )` — every value node must be a
    /// language-tagged literal whose tag matches one of the listed tags (basic
    /// filtering / prefix match per SHACL: a value tag matches an entry iff it
    /// equals it or is a subtag, e.g. `"en"` matches `"en-US"`).
    LanguageIn(Vec<String>),
    /// `sh:not <shape>` — the focus/value node must NOT conform to the shape.
    Not(Box<Shape>),
    /// `sh:closed true` or `sh:closed sh:ByTypes` (with optional
    /// `sh:ignoredProperties`), SHACL 1.2 Core §7.9.1.
    ///
    /// A node-shape-level constraint: every predicate used on the focus node must
    /// be in the permitted set `mode` defines or be listed in `ignored`. Under
    /// [`ClosedMode::Declared`] `rdf:type` is not implicitly permitted; under
    /// [`ClosedMode::ByTypes`] it is. Only emitted when `sh:closed` is `true` or
    /// `sh:ByTypes`.
    Closed {
        /// Predicates explicitly exempted from the closed-world check
        /// (`sh:ignoredProperties`).
        ignored: Vec<NamedNode>,
        /// Which set of properties the shape permits besides `ignored`.
        mode: ClosedMode,
    },
    /// `sh:minInclusive "0"^^xsd:integer`
    MinInclusive(Term),
    /// `sh:maxInclusive "100"^^xsd:integer`
    MaxInclusive(Term),
    /// `sh:minExclusive "0"^^xsd:integer`
    MinExclusive(Term),
    /// `sh:maxExclusive "100"^^xsd:integer`
    MaxExclusive(Term),
    /// `sh:and ( … )` — list of shapes, all must conform.
    And(Vec<Shape>),
    /// `sh:or ( … )` — at least one must conform.
    Or(Vec<Shape>),
    /// `sh:xone ( … )` — exactly one must conform.
    Xone(Vec<Shape>),
    /// `sh:node <shape>` — focus node must conform to the referenced shape.
    Node(Box<Shape>),
    /// `sh:sparql [ sh:select "SELECT $this …" ]` — SPARQL-AF constraint.
    ///
    /// The constraint blank node carries its own optional `sh:message` and
    /// `sh:severity` overrides; missing values fall back to the shape defaults
    /// at evaluation time.
    ///
    /// The query is validated (parseable + SELECT-form) at shape-load time. The
    /// native SPARQL engine re-parses the text at eval time, so only the query
    /// string is retained.
    Sparql {
        /// The SPARQL SELECT query text (with any injected PREFIX header).
        select: String,
        /// Optional per-constraint message override (from `sh:message` on the
        /// constraint blank node).
        messages: Vec<Literal>,
        /// Optional per-constraint severity override (from `sh:severity` on the
        /// constraint blank node).
        severity: Option<Severity>,
        /// The result annotations the constraint node declares
        /// (`sh:resultAnnotation`), in canonical order; empty for none.
        annotations: Vec<ResultAnnotation>,
    },
    /// `sh:equals <path>` (SHACL 1.2 Core §7.6.1) — the value node set must equal
    /// the set of nodes reachable from the same focus node along the path. An IRI
    /// value is the predicate path of that IRI; "The values of sh:equals in a
    /// shape are well-formed SHACL property paths."
    Equals(Path),
    /// `sh:disjoint <path>` (SHACL 1.2 Core §7.6.2) — no value node may also be
    /// reachable from the same focus node along the path.
    Disjoint(Path),
    /// `sh:subsetOf <path>` (SHACL 1.2 Core §7.6.3) — every value node must also
    /// be reachable from the same focus node along the path.
    SubsetOf(Path),
    /// `sh:lessThan <path>` (SHACL 1.2 Core §7.6.4) — every value node must be
    /// `<` every node reachable from the same focus node along the path, under
    /// SPARQL `<` semantics. Property shapes only.
    LessThan(Path),
    /// `sh:lessThanOrEquals <path>` (SHACL 1.2 Core §7.6.5) — every value node
    /// must be `<=` every node reachable from the same focus node along the path.
    /// Property shapes only.
    LessThanOrEquals(Path),
    /// `sh:qualifiedValueShape` + `sh:qualifiedMinCount`/`sh:qualifiedMaxCount`
    /// (spec §4.5.4–4.5.5).
    QualifiedValueShape {
        /// The qualified value shape the counted value nodes must conform to.
        shape: Box<Shape>,
        /// Sibling qualified value shapes (spec §4.5.5): the values of
        /// `sh:property/sh:qualifiedValueShape` on all parents of this property
        /// shape, minus this constraint's own shape. Populated only when
        /// `disjoint` is true (empty otherwise — never consulted).
        siblings: Vec<Shape>,
        /// `sh:qualifiedMinCount`, if declared.
        min_count: Option<u64>,
        /// `sh:qualifiedMaxCount`, if declared.
        max_count: Option<u64>,
        /// `sh:qualifiedValueShapesDisjoint true` — value nodes conforming to
        /// any sibling qualified shape are excluded before counting.
        disjoint: bool,
    },
    /// `sh:expression <node expression>` — SHACL-AF §5.7 expression constraint.
    ///
    /// For each value node the expression is evaluated with that value node as
    /// the focus; the constraint is satisfied iff the result is exactly the
    /// canonical `"true"^^xsd:boolean` term.
    Expression {
        /// The parsed node expression to evaluate per value node.
        expr: NodeExpr,
        /// Optional per-constraint message override (from `sh:message` on the
        /// expression node).
        messages: Vec<Literal>,
        /// Optional per-constraint severity override (from `sh:severity` on the
        /// expression node).
        severity: Option<Severity>,
    },
    /// `sh:nodeByExpression <node expression>` — SHACL 1.2 Node Expressions §7.2.
    ///
    /// For each value node `v` the expression is evaluated with `v` as the focus
    /// node and the EMPTY scope (the spec's own `evalExpr(expr, data graph, v,
    /// {})`); every output node is a shape IRI, and `v` must conform to each of
    /// them. This is the mirror image of [`Constraint::Expression`]: there the
    /// expression computes a boolean about the value node, here it computes the
    /// SHAPES the value node is judged against.
    NodeByExpression {
        /// The parsed node expression producing the node shapes to check against.
        expr: NodeExpr,
        /// The IRI-named shapes of the shapes graph, so an output shape IRI
        /// resolves to a parsed shape at validation time.
        ///
        /// Shared by `Arc` across every `sh:nodeByExpression` constraint of one
        /// shapes graph: the expression's output shape IRIs are only known during
        /// evaluation, so the resolution table has to travel with the constraint,
        /// and cloning the shapes per constraint would be pure waste. The
        /// `OnceLock` is filled once, at the end of the shapes-graph parse, so a
        /// shape carrying this constraint can itself appear in the table.
        shapes: Arc<OnceLock<FastMap<Term, Shape>>>,
        /// Optional per-constraint message override (from `sh:message` on the
        /// expression node).
        messages: Vec<Literal>,
        /// Optional per-constraint severity override (from `sh:severity` on the
        /// expression node).
        severity: Option<Severity>,
    },
    /// `sh:minListLength n` (SHACL 1.2 Core §4.9.2): every value node is a SHACL
    /// list with at least `n` members.
    MinListLength(u64),
    /// `sh:maxListLength n` (SHACL 1.2 Core §4.9.3): every value node is a SHACL
    /// list with at most `n` members.
    MaxListLength(u64),
    /// `sh:uniqueMembers true` (SHACL 1.2 Core §4.9.4): every value node is a
    /// SHACL list with no member occurring twice. `false` checks nothing.
    UniqueMembers(bool),
    /// `sh:memberShape <shape>` (SHACL 1.2 Core §4.9.1): every value node is a
    /// SHACL list whose every member conforms to the shape.
    MemberShape(Box<Shape>),
    /// `sh:singleLine true` (SHACL 1.2 Core §7.4.4): no literal value node has a
    /// lexical form containing a line break (form feed, carriage return, line
    /// feed or vertical tab). `false` checks nothing.
    SingleLine(bool),
    /// One `sh:rootClass` value (SHACL 1.2 Core §7.9.4): a root class IRI (one
    /// member) or a SHACL list of them. Every value node is an IRI that is one of
    /// the roots or reaches one through `rdfs:subClassOf*` in the data graph.
    RootClass(Vec<NamedNode>),
    /// `sh:someValue <shape>` (SHACL 1.2 Core §7.8.3): at least one value node
    /// conforms to the shape.
    SomeValue(Box<Shape>),
    /// One `sh:uniqueValuesFor` value (SHACL 1.2 Core §7.9.5): no value node
    /// shares exactly the same values for every listed property with another
    /// target node of the shape that declares it.
    UniqueValuesFor {
        /// The property IRI (one member) or the members of the SHACL list.
        properties: Vec<NamedNode>,
        /// The node of the shape that declares the constraint — `S` itself. Its
        /// target nodes include the data graph's `n sh:shape S` declarations,
        /// which name the shape by this term.
        shape: Term,
        /// The target declarations of the shape node that declares the
        /// constraint — `$targetNodes` is "the target nodes of S", a fact about
        /// that shape node however the evaluation reached it (a nested
        /// `sh:node` shape or a property shape keeps its own targets here).
        targets: Vec<Target>,
    },
    /// A SHACL-SPARQL custom constraint component usage.
    ///
    /// Emitted when a shape node carries values for all required parameters of a
    /// declared `sh:ConstraintComponent`. The validator query text already has
    /// any needed `PREFIX` header prepended.
    Component {
        /// The component IRI (`sh:ConstraintComponent` instance).
        component: NamedNode,
        /// The shape node that sourced this component usage.
        source_shape: Term,
        /// Parameter bindings: SPARQL variable local name → value term.
        bindings: Vec<(String, Term)>,
        /// The selected validator (ASK or SELECT).
        validator: ComponentValidator,
        /// Optional message override (shape → validator → component).
        messages: Vec<Literal>,
        /// Optional severity override (shape → validator → component).
        severity: Option<Severity>,
        /// The result annotations the selected validator declares
        /// (`sh:resultAnnotation`), in canonical order; empty for none.
        annotations: Vec<ResultAnnotation>,
    },
}

/// Which constraint of a shape a [`ConstraintAnnotation`] annotates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnnotatedConstraint {
    /// The constraint at this index of the shape's `constraints`.
    Constraint(usize),
    /// A property shape's `sh:reifierShape` / `sh:reificationRequired` constraint
    /// (`sh:ReifierShapeConstraintComponent`), which a [`PropertyShape`] carries
    /// in its own fields rather than in `constraints`.
    Reifier,
}

/// A per-constraint override read from RDF 1.2 reifier annotations in the shapes
/// graph.
///
/// SHACL 1.2 Core, "Declaring the Severity of a Shape or Constraint": "In
/// addition to declaring severities per shape, the property sh:severity can also
/// be used on a reifier for a triple where the shape is the subject and one of the
/// parameters of the constraint is the predicate. Let T be the set of triples that
/// represent a constraint in a shape. A shapes graph can specify at most one value
/// for the property sh:severity in the reifiers of the triples in T." "Declaring
/// Messages for a Shape or Constraint" says the same of `sh:message`, and the
/// result rules put the reifier first: `sh:resultSeverity` is "the value of
/// sh:severity at a reifier of any of the triples containing the parameters of
/// the constraint that caused the result", then "the value of sh:severity of the
/// shape", and "Messages declared using reification have precedence over those
/// declared at the surrounding shape".
///
/// A reifier `sh:deactivated true` has no annotation here: "the constraints that
/// use the triple are called deactivated constraints. Deactivated constraints are
/// ignored during validation", so the parser does not emit the constraint at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintAnnotation {
    /// The constraint the override applies to.
    pub constraint: AnnotatedConstraint,
    /// The reifier `sh:severity`, which beats the shape's (and a SHACL-SPARQL
    /// constraint node's own) severity.
    pub severity: Option<Severity>,
    /// The reifier `sh:message` values — the whole set — which beat the shape's
    /// (and a constraint node's own) messages; empty when the reifier states none.
    pub messages: Vec<Literal>,
}

/// The annotation for `which`, if the shape's sorted annotation list has one.
///
/// A linear scan: the list is empty for every shape whose shapes graph annotates
/// no constraint, and holds at most one entry per constraint otherwise.
#[inline]
pub(crate) fn annotation_for(
    annotations: &[ConstraintAnnotation],
    which: AnnotatedConstraint,
) -> Option<&ConstraintAnnotation> {
    annotations
        .iter()
        .find(|annotation| annotation.constraint == which)
}

/// Whether `value` is a literal SHACL permits as an `sh:message`.
pub(crate) fn parser_is_text_literal(value: &Term) -> bool {
    parser::wellformed::is_text_literal(value, true)
}

/// A property shape, reached via `sh:property` from a node shape.
#[derive(Debug, Clone)]
pub struct PropertyShape {
    /// The property shape's own RDF identity, preserved for result provenance
    /// and SHACL-SPARQL `currentShape` bindings.
    pub id: Term,
    /// The property path this shape applies to.
    pub path: Path,
    /// The `sh:values` node expression, when the shape declares one.
    ///
    /// SHACL 1.2 Core, "Value Nodes of Property Shapes": "For property shapes with
    /// a value for sh:path p the set of value nodes is produced by the following
    /// steps: Add all nodes in the data graph that can be reached from the focus
    /// node with the path mapping of p. If e is the value of sh:values at the
    /// property shape, then add the output nodes of evalExpr(e, data graph, focus
    /// node, {}). If the set is still empty and d is the value of sh:defaultValue
    /// at the property shape, then add the output nodes of evalExpr(d, data graph,
    /// focus node, {})." The computed nodes are UNIONED with the path's value
    /// nodes, never substituted for them. "A property shape has at most one value
    /// for the property sh:values and this value is a well-formed node
    /// expression", and "A property shape can only have values for sh:values
    /// and/or sh:defaultValue when its value for sh:path is a Predicate Path", so
    /// `Some` here implies [`Path::Predicate`].
    pub values: Option<NodeExpr>,
    /// The `sh:defaultValue` node expression, when the shape declares one: the
    /// value nodes when the path and [`Self::values`] produce none (see
    /// [`Self::values`] for the rule, quoted). Same arity and path restriction.
    pub default_value: Option<NodeExpr>,
    /// Constraints on values reached via the path.
    pub constraints: Vec<Constraint>,
    /// Property shapes nested under THIS property shape via `sh:property`
    /// (spec §2.1: `sh:property` may appear on any shape). Each nested shape is
    /// evaluated with every value node of this shape's path as its focus node.
    pub property_shapes: Vec<Self>,
    /// Node shapes that RDF 1.2 reifiers for this focus/path/value triple must conform to.
    pub reifier_shapes: Vec<Shape>,
    /// Whether at least one RDF 1.2 reifier is required for each focus/path/value triple.
    pub reification_required: bool,
    /// Severity override (default `Violation`).
    pub severity: Severity,
    /// The shape's `sh:message` values, every one, as literals (language tag,
    /// direction and datatype kept) in [`crate::report::canonical_messages`] order;
    /// empty when it declares none.
    pub messages: Vec<Literal>,
    /// The per-constraint reifier annotations on this shape's constraints, sorted
    /// by [`ConstraintAnnotation::constraint`], at most one per constraint; empty
    /// when the shapes graph annotates none of them.
    pub constraint_annotations: Vec<ConstraintAnnotation>,
    /// Whether `sh:deactivated true` is set — a deactivated property shape
    /// validates nothing.
    pub deactivated: bool,
    /// Optional graph-box role annotations on this property shape, read via the
    /// caller-supplied [`BoxRoleVocab`] (empty when no vocab is configured).
    pub box_roles: Vec<NamedNode>,
}

/// A node shape.
#[derive(Debug, Clone)]
pub struct Shape {
    /// The shape node identity (IRI or blank node).
    pub id: Term,
    /// Target declarations for this shape.
    pub targets: Vec<Target>,
    /// Node-level constraints (not path-scoped).
    pub constraints: Vec<Constraint>,
    /// Property shapes nested under this shape via `sh:property`.
    pub property_shapes: Vec<PropertyShape>,
    /// Severity override (default `Violation`).
    pub severity: Severity,
    /// The shape's `sh:message` values, every one, as literals (language tag,
    /// direction and datatype kept) in [`crate::report::canonical_messages`] order;
    /// empty when it declares none.
    pub messages: Vec<Literal>,
    /// The per-constraint reifier annotations on this shape's constraints, sorted
    /// by [`ConstraintAnnotation::constraint`], at most one per constraint; empty
    /// when the shapes graph annotates none of them. Never holds
    /// [`AnnotatedConstraint::Reifier`], which only a property shape has.
    pub constraint_annotations: Vec<ConstraintAnnotation>,
    /// Whether `sh:deactivated true` is set.
    pub deactivated: bool,
    /// Optional graph-box role annotations on this shape, read via the
    /// caller-supplied [`BoxRoleVocab`] (empty when no vocab is configured).
    pub box_roles: Vec<NamedNode>,
    /// SHACL-AF rules (`sh:rule`) attached to this shape. Empty for shapes that
    /// declare no rules (the common case); populated only on top-level shapes,
    /// which are the only shapes the rules engine drives (rules fire against a
    /// shape's target focus nodes).
    pub rules: Vec<crate::rules::Rule>,
}

/// The parsed shapes graph — a collection of top-level [`Shape`]s.
#[allow(
    clippy::struct_field_names,
    reason = "field names mirror the public Shapes API contract (shapes_graph / shapes_dataset)"
)]
#[derive(Debug, Clone)]
pub struct Shapes {
    /// The TOP-LEVEL shapes of the shapes graph: every shape validation starts
    /// from. That is every node shape, every property shape with targets of its
    /// own (wrapped as a single-property [`Shape`]), and every other shape an
    /// explicit shape target (`sh:shape` in the data graph) can name — a shape
    /// reached only through `sh:property` or `sh:node` is here too, with no
    /// targets of its own, so that a data graph's `n sh:shape s` can make `n` its
    /// focus node.
    pub node_shapes: Vec<Shape>,
    /// The shapes graph's rules that are not attached to a shape — its global rules —
    /// its rule sets, and whether it declares the rules entailment regime (SHACL 1.2
    /// Inference Rules). Shape rules live on their shapes ([`Shape::rules`]).
    pub rules: crate::rules::RuleGraph,
    /// The caller-supplied box-role vocabulary these shapes were parsed with;
    /// carried into validation so data-graph role lookups use the same terms.
    /// `None` = the box-role feature is inactive.
    pub box_role_vocab: Option<BoxRoleVocab>,
    /// SHACL-AF SPARQL-based functions (`sh:SPARQLFunction`) declared in the shapes
    /// graph, built once here and threaded into the SPARQL evaluator so calls in
    /// `sh:sparql`/`sh:SPARQLTarget` queries and `sh:expression` node expressions
    /// resolve. Empty when the graph declares no functions.
    pub functions: Arc<UserFunctionRegistry>,
    /// A caller-injected table of custom SPARQL aggregates (`AGG(<iri>, …)`) in
    /// scope for this shapes graph's `sh:sparql`/`sh:SPARQLTarget`/`sh:rule`
    /// query bodies. Unlike [`Self::functions`], nothing in SHACL-AF declares a
    /// custom aggregate from the graph itself — a host that wants `AGG(<iri>,
    /// …)` to resolve inside these shapes' queries builds an
    /// [`AggregateRegistry`], registers its aggregates, and assigns it to this
    /// PUBLIC field after parsing (`shapes.aggregates = Arc::new(registry)`),
    /// exactly the way it would replace [`Self::functions`] with a hand-built
    /// table. Every validation entry point reads THIS field directly — not a
    /// caller-installed thread-local scope — to build the aggregate scope for
    /// the query bodies it evaluates, sequential and parallel focus-chunk
    /// workers alike, via [`crate::sparql::enter_aggregate_scope`]. That holds
    /// for every public surface that reaches a focus node, including
    /// [`crate::engine::PreparedValidator::validate`],
    /// [`crate::engine::PreparedValidator::validate_focus_nodes`], and
    /// [`crate::engine::PreparedValidator::validate_focus_node_ids`], each of
    /// which validates on a prepared validator long after the scope
    /// [`crate::engine::PreparedValidator::new`] installed for its own target
    /// resolution has already been dropped. Empty by default, in which case
    /// `AGG(<iri>, …)` calls fail with the usual "no custom aggregate is
    /// registered" error.
    pub aggregates: Arc<AggregateRegistry>,
    /// The caller's validation-request options — today the conformance-disallow
    /// set ([`crate::engine::ValidationOptions::conformance_disallows`]). Read
    /// with [`Shapes::validation_options`], set with
    /// [`Shapes::set_validation_options`].
    ///
    /// Like [`Self::aggregates`], this is host configuration and never read from
    /// the shapes graph: SHACL 1.2 Core says "The conformance-disallow set is
    /// defined by the validation engine. A validation engine MAY provide
    /// mechanisms to customize this set." Every validation entry point that takes
    /// these shapes — the free functions, [`crate::engine::PreparedShapes`] and
    /// every [`crate::engine::PreparedValidator`] bound from it — answers under it.
    ///
    /// Not a public field, because it is NOT part of the declarative model a
    /// prepared product carries (the product model census reaches exactly the
    /// public fields): a preparation restored from a product starts from the
    /// default and takes a request's options through
    /// [`crate::engine::PreparedShapes::with_validation_options`]. `pub(crate)`
    /// rather than private for the reason [`Self::parse_provenance`] gives.
    pub(crate) validation_options: crate::engine::ValidationOptions,
    /// SHACL-AF `sh:SPARQLTargetType` declarations declared in the shapes graph,
    /// keyed by target-type IRI string. Empty when the graph declares no custom
    /// target types.
    pub target_types: std::collections::BTreeMap<String, SparqlTargetType>,
    /// The named-graph IRI under which the original shapes dataset is exposed
    /// to SHACL-SPARQL queries, when known.
    pub shapes_graph: Option<String>,
    /// The original frozen shapes dataset, retained so validation can expose it
    /// as a named graph to SHACL-SPARQL paths.
    pub(crate) shapes_dataset: Arc<RdfDataset>,
    /// The caller-supplied inputs this parse ran with, recorded by the parser
    /// that consumed them.
    ///
    /// Not public, and readable outside the crate only through
    /// [`Shapes::provenance`], because an identity a caller can assign is a claim
    /// rather than a fact about the parse — see [`ParseProvenance`] for the
    /// forgery that prevents. `pub(crate)` rather than fully private for the same
    /// reason [`Self::shapes_dataset`] is: in-crate construction uses functional
    /// update from [`Shapes::default`], which requires every field to be nameable
    /// at the construction site. The visibility is identical from outside the
    /// crate, where `Shapes` has been unconstructible by struct literal all along.
    pub(crate) parse_provenance: ParseProvenance,
}

impl Shapes {
    /// The validation-request options every validation of these shapes answers
    /// under (see [`crate::engine::ValidationOptions`]).
    #[must_use]
    pub fn validation_options(&self) -> &crate::engine::ValidationOptions {
        &self.validation_options
    }

    /// Validate these shapes under `options` from now on — a host's
    /// conformance-disallow set, for one.
    pub fn set_validation_options(&mut self, options: crate::engine::ValidationOptions) {
        self.validation_options = options;
    }

    /// Borrow the original frozen dataset retained when these shapes were parsed.
    ///
    /// The same allocation `parse_shapes` interned, not a copy of it, so a
    /// consumer needing the RDF behind a shapes graph never has to reparse the
    /// source text. Borrowing leaves the choice of paying for retention with the
    /// caller: `Arc::clone(shapes.dataset())` keeps the dataset alive
    /// independently of the `Shapes` value.
    ///
    /// Document-prefix handling remains part of [`crate::engine::parse_shapes`]:
    /// the dataset contains RDF statements, not the source document's prefix map.
    #[must_use]
    pub const fn dataset(&self) -> &Arc<RdfDataset> {
        &self.shapes_dataset
    }

    /// The caller-supplied inputs these shapes were parsed with.
    ///
    /// Paired with [`Self::dataset`] this is everything needed to re-derive the
    /// shapes: the dataset is the RDF, the provenance is the configuration that
    /// decided what the RDF means. A consumer that serializes a `Shapes` reads
    /// its identity from here rather than accepting it as an argument, so the
    /// recorded identity is the parse that happened and not the one the caller
    /// remembers — see [`ParseProvenance`].
    #[must_use]
    pub const fn provenance(&self) -> &ParseProvenance {
        &self.parse_provenance
    }
}

impl Default for Shapes {
    fn default() -> Self {
        Self {
            node_shapes: Vec::new(),
            rules: crate::rules::RuleGraph::default(),
            box_role_vocab: None,
            functions: Arc::new(UserFunctionRegistry::new()),
            aggregates: Arc::new(AggregateRegistry::new()),
            validation_options: crate::engine::ValidationOptions::default(),
            target_types: std::collections::BTreeMap::new(),
            shapes_graph: None,
            shapes_dataset: ::purrdf::RdfDatasetBuilder::new()
                .freeze()
                .expect("empty shapes dataset"),
            parse_provenance: ParseProvenance::default(),
        }
    }
}

// ── Public entry point ─────────────────────────────────────────────────────────

/// Parse shapes from a frozen [`RdfDataset`].
///
/// Identifies node shapes, parses all their targets, constraints, and property
/// shapes.  Unsupported SHACL features return `Err` immediately (hard-fail).
///
/// The shapes graph's `owl:imports` closure must already be in `dataset` (see
/// [`crate::imports`]); a shapes graph that imports a document it does not hold is refused.
/// [`from_dataset_with_base`] is the constructor that takes an import table.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the shapes graph's `owl:imports` closure is not in hand;
/// [`ShapesError::Invalid`] when an unsupported SHACL construct is encountered or when
/// required structural data (e.g. `sh:path`) is missing.
pub fn from_dataset(dataset: &Arc<RdfDataset>) -> Result<Shapes, ShapesError> {
    from_dataset_with_prefixes(dataset, &[])
}

/// What the linker ([`crate::spec`]) made of a shapes graph's DECLARATIONS: the
/// custom node-expression functions it indexed, their key parameters, the built-in
/// list-parameter functions it bound natively, and the custom constraint
/// components it registered.
///
/// An inspection surface for the linker's guarantees — above all that a built-in's
/// declaration indexes and registers NOTHING — which a parsed [`Shapes`] cannot
/// show, because neither the custom-function index nor the component registry
/// outlives the parse.
#[doc(hidden)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkedDeclarations {
    /// The custom node-expression functions indexed, in IRI order.
    pub custom_functions: Vec<String>,
    /// Every custom key parameter, with the function it identifies, in path order.
    pub custom_key_parameters: Vec<(String, String)>,
    /// The built-in list-parameter functions bound natively, in IRI order.
    pub native_list_functions: Vec<String>,
    /// The custom constraint components registered, in IRI order.
    pub registered_components: Vec<String>,
}

impl LinkedDeclarations {
    /// Whether the custom-function index holds `iri`.
    #[must_use]
    pub fn custom_function(&self, iri: &str) -> bool {
        self.custom_functions.iter().any(|f| f == iri)
    }

    /// The custom function the key parameter `path` identifies, if any.
    #[must_use]
    pub fn by_key_parameter(&self, path: &str) -> Option<&str> {
        self.custom_key_parameters
            .iter()
            .find(|(key, _)| key == path)
            .map(|(_, function)| function.as_str())
    }
}

/// Run the linker over `dataset`'s declarations alone — the two steps a parse
/// runs before any shape is read — and report what it indexed and registered.
///
/// # Errors
///
/// The linker's own refusal, exactly as a parse of `dataset` would report it.
#[doc(hidden)]
pub fn __linked_declarations(dataset: &Arc<RdfDataset>) -> Result<LinkedDeclarations, String> {
    let parser = Parser::new(dataset.as_ref(), None, &[], None, Arc::clone(dataset), None);
    let registry = ComponentRegistry::parse(dataset.as_ref(), &parser.prefix_resolver)?;
    let linked = parser.discover_custom_functions()?;
    let mut registered_components: Vec<String> = registry.components.keys().cloned().collect();
    registered_components.sort();
    Ok(LinkedDeclarations {
        custom_functions: linked
            .custom
            .iter()
            .map(|f| f.iri.as_str().to_owned())
            .collect(),
        custom_key_parameters: linked.custom.key_parameters(),
        native_list_functions: linked.native_list.into_iter().collect(),
        registered_components,
    })
}

/// Parse shapes from a dataset, with the shapes document's `@prefix` declarations
/// available as a fallback prefix map for SHACL-AF `sh:select` queries.
///
/// SHACL-AF queries may use prefixed names. The spec resolves them via
/// `sh:prefixes`/`sh:declare`, but real-world shapes (and pySHACL) also rely on
/// the shapes *document's* own `@prefix` declarations. Since the frozen IR does not
/// retain document prefix maps, the caller (the engine) captures them from the
/// Turtle source and threads them here. `sh:prefixes` declarations take precedence
/// over these document-level fallbacks.
///
/// # Errors
///
/// Everything [`from_dataset`] refuses.
pub fn from_dataset_with_prefixes(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
) -> Result<Shapes, ShapesError> {
    from_dataset_with_config(dataset, doc_prefixes, None)
}

/// Parse shapes from a dataset with the full caller configuration: the
/// document prefix fallback map (see [`from_dataset_with_prefixes`]) plus the
/// optional caller-supplied [`BoxRoleVocab`].
///
/// PurRDF mints no vocabulary IRIs, so the box-role annotation feature has no
/// default vocabulary: with `box_role_vocab = None` it is INACTIVE (every
/// parsed `box_roles` list stays empty and validation performs no role
/// lookups).
///
/// # Errors
///
/// Everything [`from_dataset`] refuses.
pub fn from_dataset_with_config(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
) -> Result<Shapes, ShapesError> {
    from_dataset_with_config_and_graph(dataset, doc_prefixes, box_role_vocab, None)
}

/// Parse shapes from a dataset with the full caller configuration plus an
/// explicit shapes-graph IRI. The original `dataset` is retained as
/// `Shapes::shapes_dataset` so the validation engine can expose it as a
/// named graph to SHACL-SPARQL queries.
///
/// # Errors
///
/// Everything [`from_dataset`] refuses.
pub fn from_dataset_with_config_and_graph(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
) -> Result<Shapes, ShapesError> {
    from_dataset_with_base(
        dataset,
        None,
        doc_prefixes,
        box_role_vocab,
        shapes_graph,
        &ShapesImports::new(),
    )
}

/// [`from_dataset_with_config_and_graph`] plus the base the source document's
/// relative IRI references were resolved against, recorded into the parsed
/// [`Shapes::provenance`].
///
/// A dataset is already resolved — by the time the IR exists every relative
/// reference has become an absolute IRI — so the base changes nothing about THIS
/// parse. It is threaded anyway because it is the one parse input that only a
/// caller who read the SOURCE TEXT itself can supply: dropping it here would make
/// a shapes graph that resolved `<PersonShape>` against `https://example.org/`
/// indistinguishable from one that resolved it against anything else, and nothing
/// downstream could recover which.
///
/// Published rather than `pub(crate)`, because a caller who first materializes a
/// dataset from text it read itself — folding an `owl:imports` closure at the
/// dataset level, say, the way `purrdf-validate`'s prepared-shapes-product writer
/// does — DOES hold a real base at that point, and a parameter it could only
/// answer `None` to would be the fabricated default this crate refuses to invent
/// everywhere else. A caller with no base to give still has every narrower
/// overload above, each of which spends `None` on this parameter for it.
///
/// # The shapes graph's `owl:imports` closure
///
/// `imports` supplies the documents the shapes graph's `owl:imports` name, and `base` —
/// the IRI the shapes document was read under — is declared loaded, so an import of the
/// document's own IRI names the document in hand. The closure is resolved by
/// [`resolve_shapes_imports`] before a single shape is read and the parse runs over the
/// merged graph, so an imported document's shapes are shapes of the result and its
/// `@prefix` map joins the document prefix fallback after `doc_prefixes`. An import nothing
/// in hand resolves, or a table entry nothing imports, is refused. Every narrower
/// constructor above spends an EMPTY table here, which still refuses an unresolved import.
///
/// # Errors
///
/// [`ShapesError::Imports`] when the closure is not in hand or the table cannot be used;
/// [`ShapesError::Invalid`] on any unsupported SHACL construct or missing structural data.
pub fn from_dataset_with_base(
    dataset: &Arc<RdfDataset>,
    base: Option<&str>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
    imports: &ShapesImports,
) -> Result<Shapes, ShapesError> {
    let loaded: Vec<&str> = base.into_iter().collect();
    let resolved = resolve_shapes_imports(dataset, doc_prefixes, &loaded, imports)?;
    from_resolved_dataset(
        &resolved.dataset,
        base,
        &resolved.prefixes,
        box_role_vocab,
        shapes_graph,
    )
    .map_err(ShapesError::Invalid)
}

/// Parse a shapes graph whose `owl:imports` closure is ALREADY folded in, without
/// resolving it again.
///
/// Only a prepared product's rebuild reaches this: the dataset a product carries is the
/// merged closure its packer resolved through [`from_dataset_with_base`], and resolving the
/// merged graph a second time would ask for documents that are already in it. Every other
/// constructor resolves first.
pub(crate) fn from_resolved_dataset(
    dataset: &Arc<RdfDataset>,
    base: Option<&str>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
) -> Result<Shapes, String> {
    let mut parser = Parser::new(
        dataset.as_ref(),
        base.map(ToOwned::to_owned),
        doc_prefixes,
        box_role_vocab,
        Arc::clone(dataset),
        shapes_graph,
    );
    parser.parse()
}

/// Parse a shapes graph AND, in the same parse, the node expressions rooted at
/// `roots` — each a node of `dataset` that is itself a node expression (SHACL 1.2
/// Node Expressions §3), such as the `sht:nodeExpr` of a W3C test entry or a
/// caller's own free-standing expression.
///
/// A node expression is not self-contained: it may call a custom function the
/// shapes graph declares (§6), name a shape it judges nodes against, or compute a
/// shape IRI resolved against the shapes graph's own top-level shapes (§7.2).
/// Every one of those is bound by the shapes parse — declarations are interned
/// before any expression is read, and bodies and the shape index are installed by
/// the one linking pass afterwards — so the roots are parsed by the SAME parser,
/// after the shapes and before that linking pass. An expression parsed any other
/// way would carry call sites to declarations no linking pass ever reached.
///
/// The returned expressions are in `roots` order, one per root. They evaluate
/// through [`crate::expression::eval_node_expr_in_scope`] with the returned
/// [`Shapes`]' `functions` and `aggregates` in scope
/// ([`crate::sparql::enter_function_scope`], [`crate::sparql::enter_aggregate_scope`]),
/// exactly as validation evaluates the expressions its shapes carry.
///
/// # Errors
///
/// `imports` is the shapes graph's import table, resolved exactly as
/// [`from_dataset_with_base`] resolves it. The shapes graph's own blank nodes keep their
/// labels through the merge, so a root spelled `_:e` names the same node it did before.
///
/// # Errors
///
/// Anything [`from_dataset_with_base`] refuses, and [`ShapesError::Invalid`] when any root
/// is not a well-formed node expression.
pub fn from_dataset_with_node_expressions(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    shapes_graph: Option<String>,
    roots: &[Term],
    imports: &ShapesImports,
) -> Result<(Shapes, Vec<NodeExpr>), ShapesError> {
    let resolved = resolve_shapes_imports(dataset, doc_prefixes, &[], imports)?;
    let mut parser = Parser::new(
        resolved.dataset.as_ref(),
        None,
        &resolved.prefixes,
        None,
        Arc::clone(&resolved.dataset),
        shapes_graph,
    );
    parser
        .parse_with_expressions(roots)
        .map_err(ShapesError::Invalid)
}

// ── Internal parser ────────────────────────────────────────────────────────────

/// A parse currently on the parser's stack.
///
/// The parser walks two DIFFERENT node vocabularies over the same shapes graph —
/// shapes and SHACL-AF node expressions — and one node may legitimately be both.
/// Distinguishing them by VARIANT (rather than by namespacing a rendered string)
/// makes the two in-flight domains disjoint by construction, and keeps the node
/// expression's key the term itself instead of a formatted rendering of it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum InFlight {
    /// A shape node being parsed, keyed by its rendered node id (the parser
    /// already carries that rendering for its error messages).
    Shape(String),
    /// A SHACL-AF node expression being parsed, keyed by its node term.
    NodeExpr(Term),
}

pub(crate) struct Parser<'s> {
    data: &'s RdfDataset,
    /// Tracks the shape nodes and node-expression nodes currently being parsed,
    /// to prevent infinite recursion through `sh:node` / `sh:and/or/xone` cycles
    /// and through node-expression cycles (`sh:union`, `sh:orderby`, …).
    in_flight: FastSet<InFlight>,
    /// The base the source document's relative IRI references were resolved
    /// against, carried only so [`Shapes::provenance`] can report it; `None` when
    /// the caller supplied none or entered with an already-resolved dataset.
    base: Option<String>,
    /// The prefix sources every SHACL-SPARQL query header is built from
    /// ([`prefixes::PrefixResolver`]), including the shapes document's `@prefix` map
    /// the parse's provenance records.
    prefix_resolver: prefixes::PrefixResolver,
    /// The caller-supplied box-role vocabulary; `None` = feature inactive.
    box_role_vocab: Option<BoxRoleVocab>,
    /// Registry of SHACL-SPARQL custom constraint components declared in the
    /// shapes graph. Populated before shape parsing so malformed components are
    /// rejected as hard failures.
    component_registry: ComponentRegistry,
    /// The original frozen shapes dataset, retained so validation can expose it
    /// as a named graph to SHACL-SPARQL paths.
    shapes_dataset: Arc<RdfDataset>,
    /// The named-graph IRI under which the shapes dataset is exposed.
    shapes_graph: Option<String>,
    /// Registry of SHACL-AF `sh:SPARQLTargetType` declarations declared in the
    /// shapes graph. Populated before shape parsing so target-type instances can
    /// be resolved during target parsing.
    target_types: std::collections::BTreeMap<String, parser::target_types::ParsedTargetType>,
    /// The shared top-level-shape index handed to every `sh:nodeByExpression`
    /// constraint (SHACL 1.2 Node Expressions §7.2).
    ///
    /// A `sh:nodeByExpression` expression only yields its shape IRIs during
    /// VALIDATION, so the constraint must carry a way to resolve them. The handle
    /// is created empty here and filled exactly once, at the end of [`Self::parse`],
    /// from the parsed top-level shapes — the same resolution domain
    /// [`crate::rules`] already uses for `sh:condition`. Creating the handle before
    /// any shape is parsed is what makes the arrangement re-entrant: a shape that
    /// carries a `sh:nodeByExpression` is itself in the index, and a map built
    /// eagerly during its own parse would recurse forever.
    node_shape_index: Arc<OnceLock<FastMap<Term, Shape>>>,
    /// Every custom node-expression function the shapes graph declares
    /// (SHACL 1.2 Node Expressions §6), populated before any shape is parsed so a
    /// call site inside a shape can resolve to the interned declaration.
    ///
    /// Bodies are installed after shape parsing, for the reason
    /// [`crate::shapes::parser::custom_fn`] gives: a body may call any declared
    /// function, itself included.
    custom_fns: parser::custom_fn::CustomFnIndex,
    /// The built-in LIST-parameter functions the shapes graph DECLARES (the
    /// vocabulary's `shnex:conformsToShape`, `sparql:<NAME>`), bound natively by
    /// the linker. Kept apart from [`Self::custom_fns`]: a built-in declaration
    /// adds nothing to the custom index, and is recorded only so SHACL 1.2 SPARQL
    /// Extensions §7.3 registration can give it its native implementation.
    native_list_fns: std::collections::BTreeSet<String>,
    /// Whether `sh:rule` is parsed on the shape currently being read.
    ///
    /// It is switched OFF for the duration of a `sh:condition`'s own shape parse,
    /// and for exactly one reason: a condition is evaluated by
    /// [`crate::constraints::conforms`], which reads a shape's CONSTRAINTS and
    /// never its rules, so a condition shape's rules are dead weight — and
    /// carrying them would make condition resolution non-terminating, because a
    /// shape whose rule names that same shape as its condition (the W3C
    /// `square-triple` case does exactly this) would re-enter its own rule parse
    /// without bound. Dropping the one thing a condition cannot use is what makes
    /// resolving conditions at LOAD time finite.
    parse_rules_enabled: bool,
    /// Every CONSTANT shape IRI a `sh:nodeByExpression` names, with the shape that
    /// named it, checked against [`Self::node_shape_index`] once that index is
    /// filled (SHACL 1.2 Node Expressions §7.2).
    ///
    /// `Constraint::NodeByExpression` resolves its produced shape IRIs at
    /// VALIDATION time, per value node, because in general the expression only
    /// yields them then. But when the expression IS a constant — the ordinary
    /// `sh:nodeByExpression ex:MyShape` spelling — the answer is already decided at
    /// load, and deferring it means a shape that happens to target nothing (or
    /// whose path yields no value node) SHIPS the broken constraint: the shapes
    /// graph loads green, validates green, and checks nothing. That is the same
    /// resolve-at-firing-time defect `sh:condition` carried.
    ///
    /// The check runs against the very index the validator uses, so it can refuse
    /// only what validation would have refused anyway — just at the moment the
    /// author can act on it.
    node_by_expr_constants: Vec<(Term, Term)>,
    /// The shape whose constraints are being parsed, for `sh:prefixes` resolution
    /// inside a node expression.
    ///
    /// SHACL-AF lets `sh:prefixes` sit on the SHAPE or on the constraint node, and
    /// `sh:sparql` honours both (it builds its header from `&[id, &c_node]`). A
    /// `sh:select` / `sh:sparqlExpr` NODE EXPRESSION honoured only its own node, so
    /// the identical `sh:prefixes` declaration that works for `sh:sparql` produced
    /// an "unparsable query" for a node expression — a refusal of a legal
    /// document, reported as a syntax error in the author's SPARQL.
    ///
    /// Set and RESTORED around each shape's constraint parse (never simply
    /// cleared), so an inline shape nested inside an expression cannot strip the
    /// enclosing shape's prefixes from the expressions that follow it.
    current_shape: Option<Term>,
    /// The shapes graph's `sh:closed sh:ByTypes` index, built on the first
    /// `sh:closed sh:ByTypes` the parse meets and shared by every later one — a
    /// shapes graph without one never pays for it.
    closed_type_index: Option<Arc<ClosedTypeIndex>>,
    /// The shapes graph's reifier and annotation side tables, indexed once, for
    /// the per-constraint annotations ([`parser::annotations`]).
    annotation_index: parser::annotations::AnnotationIndex,
    /// Every annotated `(shape, parameter, value)` statement a constraint parse
    /// applied, so a statement no route applied is refused rather than ignored
    /// ([`Self::check_annotations_applied`]).
    annotations_applied: FastSet<(Term, String, Term)>,
}

// ── Graph read helper (used by the parser's free functions and `prefixes`) ─────

/// Return all objects for `(subject, predicate, ?)`.
fn objects_of(data: &RdfDataset, subject: &Term, predicate: &str) -> Vec<Term> {
    if !subject.is_subject() {
        return vec![];
    }
    let pred = Term::NamedNode(NamedNode::from(predicate));
    native_quads(
        data,
        Some(subject),
        Some(&pred),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, object)| object)
    .collect()
}

impl<'s> Parser<'s> {
    fn new(
        data: &'s RdfDataset,
        base: Option<String>,
        doc_prefixes: &[(String, String)],
        box_role_vocab: Option<BoxRoleVocab>,
        shapes_dataset: Arc<RdfDataset>,
        shapes_graph: Option<String>,
    ) -> Self {
        Self {
            data,
            in_flight: FastSet::default(),
            base,
            prefix_resolver: prefixes::PrefixResolver::new(doc_prefixes),
            box_role_vocab,
            component_registry: ComponentRegistry::default(),
            shapes_dataset,
            shapes_graph,
            target_types: std::collections::BTreeMap::new(),
            node_shape_index: Arc::new(OnceLock::new()),
            custom_fns: parser::custom_fn::CustomFnIndex::default(),
            native_list_fns: std::collections::BTreeSet::new(),
            parse_rules_enabled: true,
            node_by_expr_constants: Vec::new(),
            current_shape: None,
            closed_type_index: None,
            annotation_index: parser::annotations::AnnotationIndex::build(data),
            annotations_applied: FastSet::default(),
        }
    }

    fn parse(&mut self) -> Result<Shapes, String> {
        self.parse_with_expressions(&[]).map(|(shapes, _)| shapes)
    }

    /// The whole shapes parse, plus the free-standing node expressions rooted at
    /// `roots` (see [`from_dataset_with_node_expressions`]), parsed after the
    /// shapes and before the linking pass so their call sites and shape handles
    /// are the ones that pass installs.
    fn parse_with_expressions(
        &mut self,
        roots: &[Term],
    ) -> Result<(Shapes, Vec<NodeExpr>), String> {
        self.check_builtin_cardinalities()?;

        // --- collect all top-level shape node terms ---
        let mut shape_ids: FastSet<Term> = FastSet::default();
        // Track which nodes are property-shape-only (reachable only via sh:property)
        // so we don't list them as top-level node shapes.
        let mut property_shape_nodes: FastSet<Term> = FastSet::default();

        // 1./2. The SHACL instances of sh:NodeShape (top-level) and of
        //    sh:PropertyShape (collected to exclude from top-level) — SHACL 1.2
        //    Core's first clause of "shape": "s is a SHACL instance of
        //    sh:NodeShape or sh:PropertyShape". A SHACL instance, not only a node
        //    typed with the class itself: a type that is a SHACL subclass of either
        //    counts, and so does `sh:ShapeClass`, which SHACL 1.2 Core makes "an
        //    rdfs:subClassOf of both sh:NodeShape and rdfs:Class".
        {
            let mut instances = parser::shacl_instance::ShaclInstances::new(self.data);
            let mut typed: Vec<::purrdf::TermId> = Vec::new();
            let mut seen = ::purrdf::IdSet::default();
            if let Some(rdf_type) = self.data.term_id_by_iri(rdf::TYPE) {
                for quad in crate::data::quads_for_pattern_ids(
                    self.data,
                    None,
                    Some(rdf_type),
                    None,
                    GraphFilter::AnyGraph,
                ) {
                    if seen.insert(quad.s) {
                        typed.push(quad.s);
                    }
                }
            }
            for node in typed {
                let node_shape = instances.is_node_shape(node);
                let property_shape = instances.is_property_shape(node);
                if !node_shape && !property_shape {
                    continue;
                }
                let term = crate::term::term_id_to_native(self.data, node);
                // A property shape is top-level when it has targets of its own,
                // and an implicit class target is one: a property shape that is
                // also a class is validated against the class's instances.
                if node_shape || (property_shape && instances.has_implicit_class_target(node)) {
                    shape_ids.insert(term.clone());
                }
                if property_shape {
                    property_shape_nodes.insert(term);
                }
            }
        }

        // 3. Subjects of Core target predicates and SHACL-AF sh:target — the
        //    target rows of the spec symbol table.
        for pred in crate::spec::target_predicates() {
            for (subject, _, _) in self.quads_with(None, Some(pred), None) {
                shape_ids.insert(subject);
            }
        }

        // 4. Nodes that are sh:property owners with shape constraints (implicit shapes)
        //    and nodes that are rdfs:Class AND carry sh:targetClass or sh:NodeShape type
        //    (already caught above).  We also add any node that has sh:property
        //    and is thus acting as a shape container.
        for (subject, _, _) in self.quads_with(None, Some(sh::PROPERTY), None) {
            // Only add if not exclusively a property shape itself
            if !property_shape_nodes.contains(&subject) {
                shape_ids.insert(subject);
            }
        }

        // 5. Nodes that carry sh:property as objects → record as property-shape-only
        for (_, _, object) in self.quads_with(None, Some(sh::PROPERTY), None) {
            property_shape_nodes.insert(object);
        }

        // Remove property-shape nodes from the top-level set — UNLESS the node
        // declares its own sh:target* (spec §3.1: every shape with targets is
        // validated against them; a standalone `sh:PropertyShape` carrying
        // `sh:targetNode`/`sh:targetClass` is a first-class validatable shape).
        // A property shape reachable only via sh:property has no targets of its
        // own and validates solely through its parent.
        for ps in &property_shape_nodes {
            if !self.has_own_targets(ps) {
                shape_ids.remove(ps);
            }
        }

        // 6. Every shape an explicit shape target can name. SHACL 1.2 Core,
        //    "Explicit shape targets": "If s is a shape in a shapes graph and n is
        //    a node in the data graph. If n has value s for sh:shape in the data
        //    graph, then n is a target for s." ANY shape — a property shape reached
        //    only through `sh:property`, a shape reached only through `sh:node` —
        //    so each is validated at top level too, where the data graph's
        //    `sh:shape` statements (read at validation, see
        //    `crate::target_eval`) supply its focus nodes. It declares no target
        //    of its own (one that did is already top-level), so without such a
        //    statement it validates nothing.
        //
        //    A data graph names a shape by its TERM, and it can name a blank node
        //    of the shapes graph only when the two are one graph — in which case
        //    the `sh:shape` statement is in the shapes graph too. So an IRI shape
        //    is always included, and a blank one only when a `sh:shape` statement
        //    here names it.
        {
            let named: FastSet<Term> = self
                .quads_with(None, Some(sh::SHAPE), None)
                .into_iter()
                .map(|(_, _, object)| object)
                .collect();
            for shape in self.targetable_shapes() {
                if matches!(shape, Term::NamedNode(_)) || named.contains(&shape) {
                    shape_ids.insert(shape);
                }
            }
        }

        // Custom SHACL-SPARQL constraint components are parsed up-front; any
        // malformed component, parameter, or validator query is a hard failure.
        self.component_registry = ComponentRegistry::parse(self.data, &self.prefix_resolver)?;

        // Every shape of the shapes graph, checked against the census before any
        // is parsed: an unknown or refused term, or an ill-typed parameter
        // value, is a load error rather than a silent no-op.
        self.check_well_formed()?;

        // SHACL-AF parameterized target types are parsed up-front so that
        // `sh:target` blank nodes can be instantiated during shape target parsing.
        self.target_types = self.parse_sparql_target_types()?;

        // SHACL 1.2 Node Expressions §6 custom function DECLARATIONS are discovered
        // up front, before any shape is parsed, so a `[ ex:f ( … ) ]` call site
        // inside a shape resolves to the interned declaration rather than to a
        // builtin. Their bodies are installed after shape parsing (see below).
        let linked = self.discover_custom_functions()?;
        self.custom_fns = linked.custom;
        self.native_list_fns = linked.native_list;

        // Parse each top-level shape in stable (sorted) order. A node with
        // sh:path is a (standalone) PROPERTY shape: its path-scoped constraints
        // are wrapped in a single-property Shape carrying the node's targets.
        let mut node_shapes: Vec<Shape> = Vec::new();
        let mut ids: Vec<Term> = shape_ids.into_iter().collect();
        crate::term::sort_terms_canonical(&mut ids);
        for term in ids {
            let shape = if self.first_object_of(&term, sh::PATH).is_some() {
                self.parse_standalone_property_shape(term)?
            } else {
                self.parse_node_shape(term)?
            };
            node_shapes.push(shape);
        }

        // The caller's free-standing node expressions, read by the same parser so
        // the linking pass below reaches their call sites too.
        let expressions: Vec<NodeExpr> = roots
            .iter()
            .map(|root| self.parse_node_expr(root))
            .collect::<Result<_, _>>()?;

        // The custom functions' own bodies. Deferred to here because a body is a
        // node expression that may call any declared function — itself included —
        // so it can only be parsed once every declaration is interned. They are
        // INSTALLED by the linking pass below, together with the shape index.
        // The global rules, rule sets and the rules entailment regime (SHACL 1.2
        // Inference Rules), parsed after every shape so a global rule's conditions and
        // node expressions resolve exactly as a shape rule's do.
        let rules = self.parse_rule_graph()?;

        let custom_fns = self.custom_fns.clone();
        let bodies = self.parse_custom_function_bodies(&custom_fns)?;

        let mut functions = UserFunctionRegistry::new();
        self.parse_sparql_functions(&mut functions)?;

        // The post-tree linking pass: install the bodies, fill the one shared
        // `sh:nodeByExpression` resolution table (§7.2), register the
        // expression-bodied functions — and PROVE the sharing topology, which is
        // the part no downstream test could observe. See `shapes::link`.
        let declarations: Vec<Arc<crate::expression::CustomFunction>> =
            custom_fns.iter().map(Arc::clone).collect();
        link::link_shapes(
            &node_shapes,
            &self.node_shape_index,
            &declarations,
            &self.native_list_fns,
            bodies,
            &mut functions,
        )
        .map_err(|error| error.to_string())?;
        link::link_global_rules(&rules.global_rules, &self.node_shape_index)
            .map_err(|error| error.to_string())?;

        self.check_node_by_expression_constants()?;

        let shapes = Shapes {
            node_shapes,
            rules,
            box_role_vocab: self.box_role_vocab.clone(),
            functions: Arc::new(functions),
            aggregates: Arc::new(AggregateRegistry::new()),
            validation_options: crate::engine::ValidationOptions::default(),
            target_types: self
                .target_types
                .iter()
                .map(|(iri, parsed)| (iri.clone(), parsed.declaration.clone()))
                .collect(),
            shapes_graph: self.shapes_graph.clone(),
            shapes_dataset: Arc::clone(&self.shapes_dataset),
            // Recorded HERE, at the only site that has all four values in hand,
            // so the identity a `Shapes` reports is the one its parse used. The
            // prefix map goes out in the parser's own order — the fact, not a
            // normalization of it.
            parse_provenance: ParseProvenance::new(
                self.base.clone(),
                self.prefix_resolver.document().to_vec(),
                self.box_role_vocab.clone(),
                self.shapes_graph.clone(),
            ),
        };
        Ok((shapes, expressions))
    }

    /// Resolve NOW every shape IRI a `sh:nodeByExpression` already names, against
    /// the index [`link::link_shapes`] has just filled.
    ///
    /// The constraint otherwise resolves per value node during validation, so a
    /// shape that targets nothing — or whose path yields no value node — would ship
    /// a constraint naming a shape that does not exist: green load, green report,
    /// nothing checked. The check runs against the very index the validator uses, so
    /// it refuses only what validation would refuse.
    ///
    /// An UNFILLED index means no live constraint ever took a handle, so there is
    /// nothing a recorded constant could be checked for — the constraint that
    /// recorded it was discarded during parsing and will never fire. Refusing there
    /// would be over-refusal of a shapes graph that is correct.
    ///
    /// # Errors
    ///
    /// Hard-fails when a constant names a node that is not a top-level shape of this
    /// shapes graph.
    fn check_node_by_expression_constants(&self) -> Result<(), String> {
        let Some(index) = self.node_shape_index.get() else {
            return Ok(());
        };
        for (shape_id, named) in &self.node_by_expr_constants {
            if !index.contains_key(named) {
                return Err(format!(
                    "sh:nodeByExpression on shape {shape_id} names {named}, which is not a shape \
                     of this shapes graph; the constraint would resolve it only when a value node \
                     reached it, so a shape with no targets would load and validate green while \
                     checking nothing"
                ));
            }
        }
        Ok(())
    }

    /// A handle on the shared top-level-shape index for a `sh:nodeByExpression`
    /// constraint (SHACL 1.2 Node Expressions §7.2).
    ///
    /// Taking a handle is also what tells [`Self::parse`] the index is wanted: a
    /// shapes graph with no such constraint never calls this, so the index is never
    /// built.
    pub(crate) fn share_node_shape_index(&self) -> Arc<OnceLock<FastMap<Term, Shape>>> {
        Arc::clone(&self.node_shape_index)
    }

    /// The first object of `(subject, predicate, ?)` as a string literal value.
    fn first_string_object(&self, subject: &Term, predicate: &str) -> Option<String> {
        self.first_object_of(subject, predicate)
            .and_then(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
    }

    /// The first object of `(subject, predicate, ?)` as an IRI string.
    fn first_iri_object(&self, subject: &Term, predicate: &str) -> Option<String> {
        self.first_object_of(subject, predicate)
            .and_then(|t| match t {
                Term::NamedNode(n) => Some(n.as_str().to_owned()),
                _ => None,
            })
    }

    /// Whether `id` declares any SHACL target of its own (`sh:targetClass`,
    /// `sh:targetSubjectsOf`, `sh:targetObjectsOf`, `sh:targetNode`,
    /// `sh:targetWhere`, SHACL-AF `sh:target`, or an implicit class target).
    fn has_own_targets(&self, id: &Term) -> bool {
        for pred in crate::spec::target_predicates() {
            if self.first_object_of(id, pred).is_some() {
                return true;
            }
        }
        matches!(id, Term::NamedNode(_)) && self.has_implicit_class_target(id)
    }

    /// Whether the shape node `id` carries an implicit class target: it is a
    /// SHACL instance of `sh:NodeShape` or `sh:PropertyShape` and of `rdfs:Class`
    /// in the shapes graph, which every SHACL instance of `sh:ShapeClass` is (see
    /// [`parser::shacl_instance`]).
    fn has_implicit_class_target(&self, id: &Term) -> bool {
        crate::data::resolve_id(self.data, id).is_some_and(|node| {
            parser::shacl_instance::ShaclInstances::new(self.data).has_implicit_class_target(node)
        })
    }

    /// Parse a TOP-LEVEL property shape (a node with `sh:path` and its own
    /// targets) into a wrapper [`Shape`]: the targets live on the wrapper, the
    /// path-scoped constraints in its single `property_shapes` entry.
    fn parse_standalone_property_shape(&mut self, id: Term) -> Result<Shape, String> {
        let targets = self.parse_targets(&id)?;
        let rules = self.parse_rules(&id)?;
        let ps = self.parse_property_shape(&id)?;
        let deactivated = ps.deactivated;
        Ok(Shape {
            id,
            targets,
            constraints: vec![],
            property_shapes: vec![ps],
            severity: Severity::Violation,
            messages: vec![],
            constraint_annotations: vec![],
            deactivated,
            box_roles: vec![],
            rules,
        })
    }

    /// Pattern-query the shapes dataset over ALL graphs. `subject`/`object` are IRI
    /// constants or `None` wildcards; `predicate` is an IRI constant or `None`.
    fn quads_with(
        &self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
    ) -> Vec<(Term, NamedNode, Term)> {
        let s = subject.map(|iri| Term::NamedNode(NamedNode::from(iri)));
        let p = predicate.map(|iri| Term::NamedNode(NamedNode::from(iri)));
        let o = object.map(|iri| Term::NamedNode(NamedNode::from(iri)));
        native_quads(
            self.data,
            s.as_ref(),
            p.as_ref(),
            o.as_ref(),
            GraphFilter::AnyGraph,
        )
    }

    /// Whether `(subject, rdf:type, class_iri)` is asserted in any graph.
    fn has_type(&self, subject: &Term, class_iri: &str) -> bool {
        if !subject.is_subject() {
            return false;
        }
        let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
        let class = Term::NamedNode(NamedNode::from(class_iri));
        !native_quads(
            self.data,
            Some(subject),
            Some(&rdf_type),
            Some(&class),
            GraphFilter::AnyGraph,
        )
        .is_empty()
    }

    /// Return all objects for `(subject, predicate, ?)`.
    fn objects_of(&self, subject: &Term, predicate: &str) -> Vec<Term> {
        let Some(subject_id) = crate::data::resolve_id(self.data, subject) else {
            return vec![];
        };
        let Some(predicate_id) = self.data.term_id_by_iri(predicate) else {
            return vec![];
        };
        let mut seen = ::purrdf::IdSet::default();
        crate::data::quads_for_pattern_ids(
            self.data,
            Some(subject_id),
            Some(predicate_id),
            None,
            GraphFilter::AnyGraph,
        )
        .filter(|quad| seen.insert(quad.o))
        .map(|quad| crate::term::term_id_to_native(self.data, quad.o))
        .collect()
    }

    /// Return the first object for `(subject, predicate, ?)`, if any.
    fn first_object_of(&self, subject: &Term, predicate: &str) -> Option<Term> {
        self.objects_of(subject, predicate).into_iter().next()
    }

    /// Collect deterministic graph-box role annotations from a shape node via
    /// the caller-supplied [`BoxRoleVocab`]. With no vocab configured the
    /// box-role feature is inactive and this returns an empty list.
    fn box_roles_of(&self, subject: &Term) -> Result<Vec<NamedNode>, String> {
        let Some(vocab) = &self.box_role_vocab else {
            return Ok(vec![]);
        };
        let mut roles: Vec<NamedNode> = Vec::new();
        for value in self.objects_of(subject, &vocab.graph_box_role) {
            match value {
                Term::NamedNode(n) => roles.push(n),
                other => {
                    return Err(format!(
                        "the graph-box role <{}> on {subject} must be an IRI, got {other}",
                        vocab.graph_box_role
                    ));
                }
            }
        }
        roles.sort_unstable();
        roles.dedup();
        Ok(roles)
    }

    /// Build the SPARQL `PREFIX` header prepended to a SHACL-AF `sh:select` query.
    ///
    /// The SPARQL `PREFIX` header for a query whose owners are `owners` (the
    /// query node, and the shape or component carrying it). SPARQL prefixed names
    /// must be declared in the query text, so every SHACL-SPARQL query is compiled
    /// with this header prepended; [`prefixes`] states which sources contribute and
    /// how they rank.
    ///
    /// # Errors
    ///
    /// When the prefix declarations the query reaches make the shapes graph
    /// ill-formed ([`prefixes::PrefixResolver::header`]).
    fn prefix_header(&self, owners: &[&Term]) -> Result<String, String> {
        self.prefix_resolver.header(self.data, owners)
    }

    /// The `sh:message` values of `node`, every one, in
    /// [`crate::report::canonical_messages`] order. SHACL 1.2 Core: "The values of
    /// sh:message are literals with datatype xsd:string, rdf:dirLangString,
    /// rdf:langString, or rdf:HTML"; a value of any other kind is refused rather
    /// than skipped, and none is dropped — "all validation results produced as a
    /// result of the shape will have exactly these messages".
    pub(crate) fn messages_of(&self, node: &Term) -> Result<Vec<Literal>, String> {
        let mut messages: Vec<Literal> = Vec::new();
        for value in self.objects_of(node, sh::MESSAGE) {
            match value {
                Term::Literal(lit)
                    if parser::wellformed::is_text_literal(&Term::Literal(lit.clone()), true) =>
                {
                    messages.push(lit);
                }
                other => {
                    return Err(format!(
                        "sh:message on {node} must be an xsd:string, rdf:langString, \
                         rdf:dirLangString or rdf:HTML literal, got {other}"
                    ));
                }
            }
        }
        Ok(crate::report::canonical_messages(messages))
    }

    /// The `sh:severity` of `node`, which must be an IRI (SHACL 1.2 Core
    /// §3.6.2.4); a literal or blank node is refused rather than skipped.
    pub(crate) fn severity_of(&self, node: &Term) -> Result<Option<Severity>, String> {
        match self.first_object_of(node, sh::SEVERITY) {
            None => Ok(None),
            Some(value) => severity_from_term(&value)
                .map(Some)
                .ok_or_else(|| format!("sh:severity on {node} must be an IRI, got {value}")),
        }
    }

    /// Whether `node` is deactivated: its `sh:deactivated` value must be a
    /// well-typed `xsd:boolean` literal, and only the term `true` deactivates (see
    /// [`parser::node_expr::boolean_value`] for why the comparison is by term). A
    /// node-expression value is a SHACL 1.2 form this engine does not evaluate,
    /// and is refused rather than read as `false`.
    pub(crate) fn deactivated_of(&self, node: &Term) -> Result<bool, String> {
        match self.first_object_of(node, sh::DEACTIVATED) {
            None => Ok(false),
            Some(value) => parser::node_expr::boolean_value(&value).ok_or_else(|| {
                format!(
                    "sh:deactivated on {node} must be an xsd:boolean literal, got {value}; a \
                     node-expression value of sh:deactivated is not evaluated by this engine"
                )
            }),
        }
    }

    /// Whether `node` is the EMPTY node expression: a blank node that is the
    /// subject of no triple (SHACL 1.2 Node Expressions §4.1.1).
    pub(crate) fn is_empty_expression(&self, node: &Term) -> bool {
        matches!(node, Term::BlankNode(_))
            && native_quads(self.data, Some(node), None, None, GraphFilter::AnyGraph).is_empty()
    }

    /// Parse a top-level node shape.
    fn parse_node_shape(&mut self, id: Term) -> Result<Shape, String> {
        let id_str = id.to_string();

        // Guard against recursive `sh:node` cycles
        let key = InFlight::Shape(id_str);
        if self.in_flight.contains(&key) {
            // Return a minimal stand-in to break the cycle; cyclic shapes are
            // unusual but not forbidden.
            return Ok(Shape {
                id,
                targets: vec![],
                constraints: vec![],
                property_shapes: vec![],
                severity: Severity::Violation,
                messages: vec![],
                constraint_annotations: vec![],
                deactivated: false,
                box_roles: vec![],
                rules: vec![],
            });
        }
        self.in_flight.insert(key.clone());
        let result = self.parse_shape_inner(&id);
        self.in_flight.remove(&key);
        result
    }

    /// Inner parse logic shared between top-level and anonymous/inline shapes.
    fn parse_shape_inner(&mut self, id: &Term) -> Result<Shape, String> {
        // -- Severity --
        let severity = self.severity_of(id)?.unwrap_or(Severity::Violation);

        // -- Message (take first by stable sort of string representation) --
        let messages = self.messages_of(id)?;

        // -- Deactivated --
        let deactivated = self.deactivated_of(id)?;

        // -- Targets (only for top-level node shapes; anonymous shapes have none) --
        let targets = self.parse_targets(id)?;

        // -- Property shapes (via sh:property) --
        let mut property_shape_nodes: Vec<Term> = self.objects_of(id, sh::PROPERTY);
        crate::term::sort_terms_canonical(&mut property_shape_nodes);
        let mut property_shapes = Vec::new();
        for ps_node in property_shape_nodes {
            let annotated = self.constraint_annotation(id, &[(sh::PROPERTY, &ps_node)])?;
            let ps = self.parse_property_shape(&ps_node)?;
            if let Some(ps) = annotate_property_edge(ps, annotated) {
                property_shapes.push(ps);
            }
        }

        // -- Node-level constraints --
        let parsed = self.parse_constraints(id, false)?;
        let box_roles = self.box_roles_of(id)?;
        // -- SHACL-AF rules (sh:rule) --
        let rules = self.parse_rules(id)?;
        self.check_annotations_applied(id)?;

        Ok(Shape {
            id: id.clone(),
            targets,
            constraints: parsed.constraints,
            property_shapes,
            severity,
            messages,
            constraint_annotations: parsed.annotations,
            deactivated,
            box_roles,
            rules,
        })
    }

    /// Parse all target declarations for a shape node.
    fn parse_targets(&mut self, id: &Term) -> Result<Vec<Target>, String> {
        let mut targets: Vec<Target> = Vec::new();

        // sh:targetClass / sh:targetSubjectsOf / sh:targetObjectsOf — each value
        // must be an IRI (SHACL 1.2 Core §2.1.3); anything else is refused rather
        // than skipped, since a skipped target silently selects nothing.
        for (predicate, make) in [
            (sh::TARGET_CLASS, Target::Class as fn(NamedNode) -> Target),
            (sh::TARGET_SUBJECTS_OF, Target::SubjectsOf as fn(_) -> _),
            (sh::TARGET_OBJECTS_OF, Target::ObjectsOf as fn(_) -> _),
        ] {
            let mut iris: Vec<NamedNode> = Vec::new();
            for value in self.objects_of(id, predicate) {
                match value {
                    Term::NamedNode(n) => iris.push(n),
                    other => {
                        return Err(format!(
                            "<{predicate}> on shape {id} must be an IRI, got {other}"
                        ));
                    }
                }
            }
            iris.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            for n in iris {
                targets.push(make(n));
            }
        }

        // sh:targetNode — SHACL 1.2 Core, "Node targets": "Each value of
        // sh:targetNode in a shape is a well-formed node expression", and "If s is a
        // shape in a shapes graph SG and s has value expr for sh:targetNode in SG,
        // then the output nodes of evalExpr(expr, data graph, s, {}) are targets
        // for the data graph DG as focus graph." An IRI, a literal and a triple
        // term are constant expressions whose output is themselves, so they stay
        // constants. A blank node that is the subject of no triple is the EMPTY
        // expression (Node Expressions: "its output nodes are the empty list"), so
        // it targets nothing. Any other blank node is a structured expression,
        // parsed here with the shape as the prefix owner (as a constraint's
        // expression is) and evaluated at validation from the shape.
        let mut tn: Vec<Term> = self.objects_of(id, sh::TARGET_NODE);
        crate::term::sort_terms_canonical(&mut tn);
        for t in tn {
            match &t {
                Term::BlankNode(_) if self.is_empty_expression(&t) => {}
                Term::BlankNode(_) => {
                    let saved_shape = self.current_shape.replace(id.clone());
                    let expr = self.parse_node_expr(&t);
                    self.current_shape = saved_shape;
                    let expr = expr.map_err(|e| {
                        format!(
                            "sh:targetNode on shape {id} is not a well-formed node expression: {e}"
                        )
                    })?;
                    targets.push(Target::NodeExpression(expr));
                }
                Term::NamedNode(_) | Term::Literal(_) | Term::Triple(_) => {
                    targets.push(Target::Node(t));
                }
            }
        }

        // Implicit class target — see [`Target::ImplicitClass`] for the two
        // textual definitions. Decided on SHACL INSTANCES in the shapes graph
        // (`rdf:type` then `rdfs:subClassOf*`), not on a direct `rdf:type
        // rdfs:Class`. A blank shape that would qualify is an ill-formed shape,
        // refused by the well-formedness pass before any shape is parsed.
        if let Term::NamedNode(_) = id
            && self.has_implicit_class_target(id)
        {
            targets.push(Target::ImplicitClass(id.clone()));
        }

        // sh:targetWhere — SHACL 1.2 Core, "Where Targets": "Each value of
        // sh:targetWhere in a shape is a well-formed shape", and "If s is a shape
        // in a shapes graph SG and s has value w for sh:targetWhere in SG then the
        // set of nodes in a data graph DG that conform to w is a target from DG for
        // s in SG." The value is parsed the way `sh:node`'s is: one carrying
        // `sh:path` is a property shape, not a node shape whose path is discarded.
        let mut wheres: Vec<Term> = self.objects_of(id, sh::TARGET_WHERE);
        crate::term::sort_terms_canonical(&mut wheres);
        for w in wheres {
            if !matches!(w, Term::NamedNode(_) | Term::BlankNode(_)) {
                return Err(format!(
                    "sh:targetWhere on shape {id} must be a shape (an IRI or a blank node), got {w}"
                ));
            }
            targets.push(Target::Where(Box::new(self.parse_inline_shape(w)?)));
        }

        // sh:target — SHACL-AF extension targets. Supports plain sh:SPARQLTarget
        // and parameterized sh:SPARQLTargetType instances.
        let mut sparql_targets: Vec<Term> = self.objects_of(id, sh::TARGET);
        crate::term::sort_terms_canonical(&mut sparql_targets);
        for t_node in sparql_targets {
            if self.has_type(&t_node, sh::SPARQL_TARGET) {
                // Plain sh:SPARQLTarget: sh:select is required on the blank node.
                let raw_select = self
                    .first_object_of(&t_node, sh::SELECT)
                    .and_then(|t| match t {
                        Term::Literal(lit) => Some(lit.value().to_owned()),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        format!(
                            "sh:SPARQLTarget on shape {id} is missing a sh:select string literal"
                        )
                    })?;
                // SHACL-AF sh:prefixes may be declared on the shape or the target node.
                let select = format!("{}{raw_select}", self.prefix_header(&[id, &t_node])?);

                // Parse-time query validation via the native parser (hard-fail on
                // unparsable queries). SHACL-SPARQL requires a SELECT; ASK/CONSTRUCT/
                // DESCRIBE parse but cannot bind ?this and would panic at eval — reject
                // at the boundary.
                match purrdf_sparql_algebra::SparqlParser::new().parse_query(&select) {
                    Ok(purrdf_sparql_algebra::Query::Select { .. }) => {}
                    Ok(_) => {
                        return Err(format!(
                            "sh:SPARQLTarget on shape {id} must be a SELECT query (ASK/CONSTRUCT/DESCRIBE are not valid SHACL-SPARQL)"
                        ));
                    }
                    Err(e) => {
                        return Err(format!(
                            "sh:SPARQLTarget on shape {id} has an unparsable sh:select query: {e}"
                        ));
                    }
                }

                targets.push(Target::Sparql {
                    select,
                    substitutions: vec![],
                });
                continue;
            }

            // Not a plain SPARQLTarget: look for an rdf:type that names a declared
            // sh:SPARQLTargetType.
            let type_terms: Vec<Term> = self.objects_of(&t_node, rdf::TYPE);
            let mut matched: Option<(NamedNode, parser::target_types::ParsedTargetType)> = None;
            for t in type_terms {
                if let Term::NamedNode(n) = &t
                    && let Some(target_type) = self.target_types.get(n.as_str())
                {
                    matched = Some((n.clone(), target_type.clone()));
                    break;
                }
            }
            let Some((type_iri, target_type)) = matched else {
                return Err(format!(
                    "unsupported sh:target type on shape {id}: target node {t_node} \
                     is neither typed sh:SPARQLTarget nor a declared sh:SPARQLTargetType"
                ));
            };
            let parser::target_types::ParsedTargetType {
                declaration: target_type,
                optional_predicates,
            } = target_type;

            // Collect parameter bindings from the target instance.
            let mut substitutions: Vec<(String, Term)> = Vec::new();
            let mut missing_required = false;
            for param in &target_type.params {
                let values = self.objects_of(&t_node, param.predicate.as_str());
                if values.len() > 1 {
                    return Err(format!(
                        "sh:target instance of <{type_iri}> on shape {id} has {count} values for parameter <{pred}>, only one is allowed",
                        count = values.len(),
                        pred = param.predicate.as_str()
                    ));
                }
                let Some(value) = values.into_iter().next() else {
                    missing_required |= !optional_predicates.contains(&param.predicate);
                    continue;
                };
                if matches!(value, Term::BlankNode(_)) {
                    return Err(format!(
                        "sh:target instance of <{type_iri}> on shape {id} has a blank node value for parameter <{pred}>, which is not allowed",
                        pred = param.predicate.as_str()
                    ));
                }
                substitutions.push((param.var.clone(), value));
            }
            // SHACL-AF target instances lacking any mandatory parameter
            // contribute no focus nodes. Check every supplied parameter before
            // applying this activation rule so malformed values still fail.
            if missing_required {
                continue;
            }

            // Build the query with prefixes from the shape, the target instance,
            // and the target-type declaration itself.
            let select = format!(
                "{}{}",
                self.prefix_header(&[id, &t_node, &target_type.id])?,
                target_type.select
            );
            match purrdf_sparql_algebra::SparqlParser::new().parse_query(&select) {
                Ok(purrdf_sparql_algebra::Query::Select { .. }) => {}
                Ok(_) => {
                    return Err(format!(
                        "sh:target instance of <{type_iri}> on shape {id} must be a SELECT query"
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "sh:target instance of <{type_iri}> on shape {id} has an unparsable sh:select query: {e}"
                    ));
                }
            }

            targets.push(Target::Sparql {
                select,
                substitutions,
            });
        }

        Ok(targets)
    }

    /// Parse a property shape node.
    fn parse_property_shape(&mut self, ps_node: &Term) -> Result<PropertyShape, String> {
        let ps_str = ps_node.to_string();

        // sh:path is required
        let path_node = self
            .first_object_of(ps_node, sh::PATH)
            .ok_or_else(|| format!("property shape {ps_str} missing sh:path"))?;

        let path = self.parse_path(&path_node, ps_node, &mut FastSet::default())?;

        // sh:values / sh:defaultValue — the computed value nodes (see
        // `PropertyShape::values` for the Core rule).
        let values = self.computed_values_expr(ps_node, &path, sh::VALUES)?;
        let default_value = self.computed_values_expr(ps_node, &path, sh::DEFAULT_VALUE)?;

        // severity
        let severity = self.severity_of(ps_node)?.unwrap_or(Severity::Violation);

        // message
        let messages = self.messages_of(ps_node)?;

        // sh:deactivated — a deactivated property shape validates nothing.
        let deactivated = self.deactivated_of(ps_node)?;

        // constraints on the property shape
        let parsed = self.parse_constraints(ps_node, true)?;
        let mut constraint_annotations = parsed.annotations;

        // Nested sh:property on a property shape (spec §2.1: sh:property may
        // appear on ANY shape) — each nested property shape applies to THIS
        // shape's value nodes. `in_flight` breaks sh:property cycles by
        // skipping a shape already being parsed (mirrors parse_node_shape's
        // cycle stub).
        let mut nested_nodes: Vec<Term> = self.objects_of(ps_node, sh::PROPERTY);
        crate::term::sort_terms_canonical(&mut nested_nodes);
        let mut property_shapes: Vec<PropertyShape> = Vec::new();
        if !nested_nodes.is_empty() {
            let key = InFlight::Shape(ps_str.clone());
            self.in_flight.insert(key.clone());
            for nested in nested_nodes {
                let annotated =
                    match self.constraint_annotation(ps_node, &[(sh::PROPERTY, &nested)]) {
                        Ok(annotated) => annotated,
                        Err(e) => {
                            self.in_flight.remove(&key);
                            return Err(e);
                        }
                    };
                if self
                    .in_flight
                    .contains(&InFlight::Shape(nested.to_string()))
                {
                    continue;
                }
                match self.parse_property_shape(&nested) {
                    Ok(parsed) => property_shapes.extend(annotate_property_edge(parsed, annotated)),
                    Err(e) => {
                        self.in_flight.remove(&key);
                        return Err(e);
                    }
                }
            }
            self.in_flight.remove(&key);
        }

        let box_roles = self.box_roles_of(ps_node)?;
        let reification_required_term = self.first_object_of(ps_node, sh::REIFICATION_REQUIRED);
        let mut reification_required = match &reification_required_term {
            None => false,
            Some(value) => parser::node_expr::boolean_value(value).ok_or_else(|| {
                format!(
                    "sh:reificationRequired on property shape {ps_str} must be an xsd:boolean \
                     literal, got {value}"
                )
            })?,
        };

        let mut reifier_shape_nodes: Vec<Term> = self.objects_of(ps_node, sh::REIFIER_SHAPE);
        crate::term::sort_terms_canonical(&mut reifier_shape_nodes);
        if (!reifier_shape_nodes.is_empty() || reification_required)
            && !matches!(path, Path::Predicate(_))
        {
            return Err(format!(
                "sh:reifierShape or sh:reificationRequired on property shape {ps_str} requires an IRI sh:path"
            ));
        }
        // The `sh:ReifierShapeConstraintComponent` constraint's T: its
        // `sh:reifierShape` statements and its `sh:reificationRequired` one.
        let mut reifier_triples: Vec<(&str, &Term)> = reifier_shape_nodes
            .iter()
            .map(|node| (sh::REIFIER_SHAPE, node))
            .collect();
        if let Some(term) = &reification_required_term {
            reifier_triples.push((sh::REIFICATION_REQUIRED, term));
        }
        let reifier_annotation = if !reifier_shape_nodes.is_empty() || reification_required {
            self.constraint_annotation(ps_node, &reifier_triples)?
        } else {
            // `sh:reificationRequired false` alone is an inactive component.
            for &(predicate, value) in &reifier_triples {
                self.apply_to_nothing(ps_node, predicate, value)?;
            }
            Annotated::Plain
        };
        let reifier_deactivated = reifier_annotation == Annotated::Deactivated;
        if let Annotated::Override { severity, messages } = reifier_annotation {
            constraint_annotations.push(ConstraintAnnotation {
                constraint: AnnotatedConstraint::Reifier,
                severity,
                messages,
            });
        }
        let mut reifier_shapes = Vec::new();
        for node in reifier_shape_nodes {
            // A reifier shape is a SHAPE, and a shape carrying `sh:path` is a
            // property shape whose constraints scope to that path's value nodes —
            // here, the reifier's. Parsing it as a node shape would drop the path
            // and re-read the constraints against the reifier itself, which is a
            // SILENT DROP rather than a wrong answer: `sh:minCount 1` over the one
            // value node `$this` always holds, so the reifier shape would check
            // nothing and every reifier would pass.
            reifier_shapes.push(self.parse_inline_shape(node)?);
        }
        if reifier_deactivated {
            // "Deactivated constraints are ignored during validation": the
            // reifier shapes were still parsed, so an ill-formed one is refused.
            reifier_shapes.clear();
            reification_required = false;
        }
        self.check_annotations_applied(ps_node)?;

        Ok(PropertyShape {
            id: ps_node.clone(),
            path,
            values,
            default_value,
            constraints: parsed.constraints,
            property_shapes,
            reifier_shapes,
            reification_required,
            severity,
            messages,
            constraint_annotations,
            deactivated,
            box_roles,
        })
    }

    /// The `sh:values` or `sh:defaultValue` node expression of property shape
    /// `ps_node` (`predicate` names which), or `None` when it declares none.
    ///
    /// SHACL 1.2 Core, "Property Shapes": "A property shape has at most one value
    /// for the property sh:values and this value is a well-formed node
    /// expression. A property shape has at most one value for the property
    /// sh:defaultValue and this value is a well-formed node expression. A property
    /// shape can only have values for sh:values and/or sh:defaultValue when its
    /// value for sh:path is a Predicate Path." Each of the three is a load error
    /// here. An IRI, a literal and a triple term are constant expressions whose
    /// output is themselves; a blank node that is the subject of no triple is the
    /// empty expression; any other blank node is a structured expression, parsed
    /// with the property shape as its prefix owner, as a constraint's is.
    fn computed_values_expr(
        &mut self,
        ps_node: &Term,
        path: &Path,
        predicate: &str,
    ) -> Result<Option<NodeExpr>, String> {
        let mut values: Vec<Term> = self.objects_of(ps_node, predicate);
        let local = if predicate == sh::VALUES {
            "sh:values"
        } else {
            "sh:defaultValue"
        };
        let node = match values.len() {
            0 => return Ok(None),
            1 => values.pop().ok_or_else(|| {
                format!("internal defect: {local} on property shape {ps_node} vanished")
            })?,
            n => {
                return Err(format!(
                    "property shape {ps_node} has {n} values for {local}; SHACL 1.2 Core: \"A \
                     property shape has at most one value for the property {local}\""
                ));
            }
        };
        if !matches!(path, Path::Predicate(_)) {
            return Err(format!(
                "property shape {ps_node} has a value for {local} but its sh:path is not an IRI; \
                 SHACL 1.2 Core: \"A property shape can only have values for sh:values and/or \
                 sh:defaultValue when its value for sh:path is a Predicate Path\""
            ));
        }
        let expr = match &node {
            Term::NamedNode(_) | Term::Literal(_) | Term::Triple(_) => NodeExpr::Constant(node),
            Term::BlankNode(_) if self.is_empty_expression(&node) => NodeExpr::Empty,
            Term::BlankNode(_) => {
                let saved_shape = self.current_shape.replace(ps_node.clone());
                let parsed = self.parse_node_expr(&node);
                self.current_shape = saved_shape;
                parsed.map_err(|e| {
                    format!(
                        "{local} on property shape {ps_node} is not a well-formed node \
                         expression: {e}"
                    )
                })?
            }
        };
        Ok(Some(expr))
    }

    /// Parse an `sh:path` value into a [`Path`] (all six §2.3.1 path forms).
    ///
    /// `in_flight` tracks the blank path nodes currently being expanded so a
    /// cyclic path structure (a blank node reachable from itself) hard-fails
    /// instead of recursing forever.
    fn parse_path(
        &self,
        path_node: &Term,
        shape_id: &Term,
        in_flight: &mut FastSet<String>,
    ) -> Result<Path, String> {
        match path_node {
            Term::NamedNode(nn) => Ok(Path::Predicate(nn.clone())),
            Term::BlankNode(label) => {
                if !in_flight.insert(label.clone()) {
                    return Err(format!("cyclic sh:path structure on shape {shape_id}"));
                }
                let result = self.parse_blank_path(path_node, shape_id, in_flight);
                in_flight.remove(label);
                result
            }
            _ => Err(format!(
                "sh:path on shape {shape_id} must be an IRI or blank node, got {path_node}"
            )),
        }
    }

    /// Parse the blank-node forms of an `sh:path` value: sequence (RDF list),
    /// inverse, alternative, and the three closure paths.
    fn parse_blank_path(
        &self,
        path_node: &Term,
        shape_id: &Term,
        in_flight: &mut FastSet<String>,
    ) -> Result<Path, String> {
        // RDF list in path position = sequence path (at least two members).
        if self.first_object_of(path_node, rdf::FIRST).is_some() {
            let items = self.walk_rdf_list(path_node, shape_id)?;
            if items.len() < 2 {
                return Err(format!(
                    "sequence path on shape {shape_id} must have at least two members, \
                     got {}",
                    items.len()
                ));
            }
            let mut parts = Vec::with_capacity(items.len());
            for item in &items {
                parts.push(self.parse_path(item, shape_id, in_flight)?);
            }
            return Ok(Path::Sequence(parts));
        }

        // The five blank-node path forms (SHACL 1.2 Core §2.3.1.3–§2.3.1.7). Each
        // is a blank node with EXACTLY ONE triple, whose predicate names the form:
        // a second triple — another form, a second value, or anything else — makes
        // the node match none of them, so it is refused rather than read as
        // whichever form happens to be looked at first.
        // One statement asserted in several named graphs is still one statement.
        let mut outgoing: Vec<(NamedNode, Term)> = Vec::new();
        for (_, predicate, object) in native_quads(
            self.data,
            Some(path_node),
            None,
            None,
            GraphFilter::AnyGraph,
        ) {
            if !outgoing.contains(&(predicate.clone(), object.clone())) {
                outgoing.push((predicate, object));
            }
        }
        let [(predicate, inner)] = outgoing.as_slice() else {
            return Err(format!(
                "sh:path blank node {path_node} on shape {shape_id} is not a well-formed SHACL \
                 path: a non-list path node carries exactly one of sh:inversePath, \
                 sh:alternativePath, sh:zeroOrMorePath, sh:oneOrMorePath or sh:zeroOrOnePath and \
                 nothing else, and it carries {} triples",
                outgoing.len()
            ));
        };
        match predicate.as_str() {
            sh::INVERSE_PATH => {
                let inner_path = self.parse_path(inner, shape_id, in_flight)?;
                Ok(Path::Inverse(Box::new(inner_path)))
            }
            // sh:alternativePath — an RDF list of at least two alternatives.
            sh::ALTERNATIVE_PATH => {
                let items = self.walk_rdf_list(inner, shape_id)?;
                if items.len() < 2 {
                    return Err(format!(
                        "sh:alternativePath on shape {shape_id} must have at least two \
                         members, got {}",
                        items.len()
                    ));
                }
                let mut parts = Vec::with_capacity(items.len());
                for item in &items {
                    parts.push(self.parse_path(item, shape_id, in_flight)?);
                }
                Ok(Path::Alternative(parts))
            }
            sh::ZERO_OR_MORE_PATH => Ok(Path::ZeroOrMore(Box::new(
                self.parse_path(inner, shape_id, in_flight)?,
            ))),
            sh::ONE_OR_MORE_PATH => Ok(Path::OneOrMore(Box::new(
                self.parse_path(inner, shape_id, in_flight)?,
            ))),
            sh::ZERO_OR_ONE_PATH => Ok(Path::ZeroOrOne(Box::new(
                self.parse_path(inner, shape_id, in_flight)?,
            ))),
            other => Err(format!(
                "unrecognised sh:path blank node structure on shape {shape_id}: <{other}> is not \
                 a SHACL path predicate"
            )),
        }
    }

    /// Walk a well-formed SHACL list and collect its members.
    ///
    /// SHACL 1.2 Core §1.4: "A SHACL list in an RDF graph G is an IRI or a blank
    /// node that is either rdf:nil (provided that rdf:nil has no value for either
    /// rdf:first or rdf:rest), or has exactly one value for the property rdf:first
    /// in G and exactly one value for the property rdf:rest in G that is also a
    /// SHACL list in G, and the list does not have itself as a value of the
    /// property path rdf:rest+ in G." Every clause is enforced: a cell without
    /// `rdf:first` or `rdf:rest`, with two of either, a literal cell, or a cycle is
    /// refused rather than read as a shorter list.
    fn walk_rdf_list(&self, head: &Term, shape_id: &Term) -> Result<Vec<Term>, String> {
        let nil = Term::NamedNode(NamedNode::from(rdf::NIL));
        let mut items = Vec::new();
        let mut current = head.clone();
        let mut seen: FastSet<Term> = FastSet::default();

        loop {
            if !matches!(current, Term::NamedNode(_) | Term::BlankNode(_)) {
                return Err(format!(
                    "the RDF list at {head} on {shape_id} is not a well-formed SHACL list: \
                     {current} is not an IRI or a blank node"
                ));
            }
            let firsts = self.objects_of(&current, rdf::FIRST);
            let rests = self.objects_of(&current, rdf::REST);
            if current == nil {
                if !firsts.is_empty() || !rests.is_empty() {
                    return Err(format!(
                        "the RDF list at {head} on {shape_id} is not a well-formed SHACL list: \
                         rdf:nil has an rdf:first or rdf:rest value"
                    ));
                }
                break;
            }
            if !seen.insert(current.clone()) {
                return Err(format!("cyclic RDF list on shape {shape_id}"));
            }
            let ([first], [rest]) = (firsts.as_slice(), rests.as_slice()) else {
                return Err(format!(
                    "the RDF list at {head} on {shape_id} is not a well-formed SHACL list: the \
                     cell {current} has {} rdf:first and {} rdf:rest values, where a list cell \
                     has exactly one of each",
                    firsts.len(),
                    rests.len()
                ));
            };
            items.push(first.clone());
            current = rest.clone();
        }
        Ok(items)
    }

    /// Walk an RDF list of shape nodes, parsing each as an anonymous shape.
    ///
    /// Members that carry `sh:path` are treated as single-property inline shapes
    /// (the path+constraints go into a `property_shapes` entry); otherwise the
    /// constraints are node-level.
    fn parse_shape_list(&mut self, head: &Term, shape_id: &Term) -> Result<Vec<Shape>, String> {
        let items = self.walk_rdf_list(head, shape_id)?;
        let mut shapes = Vec::new();
        for item in items {
            let shape = self.parse_inline_shape(item)?;
            shapes.push(shape);
        }
        Ok(shapes)
    }

    /// Parse an inline / anonymous shape (member of `sh:and`, `sh:or`, etc.).
    ///
    /// If the node has `sh:path`, it is treated as an inline property shape:
    /// the path+constraints are wrapped into a single `PropertyShape` entry and
    /// the resulting `Shape` has empty node-level constraints.
    fn parse_inline_shape(&mut self, id: Term) -> Result<Shape, String> {
        let id_str = id.to_string();

        // Guard against cycles
        let key = InFlight::Shape(id_str);
        if self.in_flight.contains(&key) {
            return Ok(Shape {
                id,
                targets: vec![],
                constraints: vec![],
                property_shapes: vec![],
                severity: Severity::Violation,
                messages: vec![],
                constraint_annotations: vec![],
                deactivated: false,
                box_roles: vec![],
                rules: vec![],
            });
        }

        if self.first_object_of(&id, sh::PATH).is_some() {
            // Treat as an inline property shape
            self.in_flight.insert(key.clone());
            let ps = self.parse_property_shape(&id);
            self.in_flight.remove(&key);
            let ps = ps?;
            Ok(Shape {
                id,
                targets: vec![],
                constraints: vec![],
                property_shapes: vec![ps],
                severity: Severity::Violation,
                messages: vec![],
                constraint_annotations: vec![],
                deactivated: false,
                box_roles: vec![],
                rules: vec![],
            })
        } else {
            self.parse_node_shape(id)
        }
    }
}

// ── Helper functions ───────────────────────────────────────────────────────────

/// A property shape as the `sh:property` statement that reaches it is annotated.
///
/// `sh:property` is a constraint parameter (`sh:PropertyConstraintComponent`),
/// and "the validation results are the results of validating v as focus node
/// against the property shape $property" — the results that constraint causes
/// are the property shape's. So a reifier on the `sh:property` statement
/// annotates exactly those results: `sh:deactivated true` drops this reach of
/// the property shape (the same shape reached through another, unannotated
/// statement still validates), and `sh:severity` / `sh:message` take the place of
/// the property shape's own for this reach, below any reifier annotation on the
/// property shape's own constraint statements, which names the constraint that
/// caused the result more narrowly still.
fn annotate_property_edge(mut ps: PropertyShape, annotated: Annotated) -> Option<PropertyShape> {
    match annotated {
        Annotated::Deactivated => None,
        Annotated::Plain => Some(ps),
        Annotated::Override { severity, messages } => {
            if let Some(severity) = severity {
                ps.severity = severity;
            }
            if !messages.is_empty() {
                ps.messages = messages;
            }
            Some(ps)
        }
    }
}

/// The local name of an IRI: the substring after the last `#` or `/`. Used to
/// derive a `sh:SPARQLFunction` parameter's pre-bound SPARQL variable name from its
/// predicate IRI (SHACL-AF §5.1).
pub(crate) fn local_name(iri: &str) -> &str {
    let cut = iri.rfind(['#', '/']).map_or(0, |i| i + 1);
    &iri[cut..]
}

/// Parse an `xsd:integer` literal into a `u64`; `None` for any other term.
pub(crate) fn parse_u64(term: &Term) -> Option<u64> {
    // SHACL's count, length and limit parameters are "literals with datatype
    // xsd:integer": a plain string "1" or a decimal 1.0 is ill-typed, not a count.
    match term {
        Term::Literal(lit) if lit.datatype_str() == crate::model::xsd::INTEGER => {
            lit.value().parse::<u64>().ok()
        }
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::FnCall;
    use purrdf_sparql_eval::UserFnBody;

    /// Parse Turtle into a frozen dataset (the in-crate tests historically used an
    /// oxigraph store; the name is kept so the call sites stay stable).
    fn load_store(ttl: &str) -> Arc<RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("Turtle parse error")
    }

    /// Parse shapes from a test dataset (shim over [`from_dataset`]).
    fn from_store(dataset: &Arc<RdfDataset>) -> Result<Shapes, String> {
        from_dataset(dataset).map_err(String::from)
    }

    const PREFIXES: &str = r"
        @prefix sh:   <http://www.w3.org/ns/shacl#> .
        @prefix ex:   <http://example.org/ns#> .
        @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
    ";

    // ── SHACL-AF sh:SPARQLFunction declaration parsing ────────────────────────

    #[test]
    fn sparql_function_declaration_parsed_into_registry() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:multiply a sh:SPARQLFunction ;
                sh:parameter [ sh:path ex:op1 ; sh:order 1 ; sh:datatype xsd:integer ] ;
                sh:parameter [ sh:path ex:op2 ; sh:order 2 ; sh:optional true ] ;
                sh:returnType xsd:integer ;
                sh:select "SELECT ((?op1 * ?op2) AS ?result) WHERE {{}}" .
            "#
        );
        let shapes = from_store(&load_store(&ttl)).expect("parse");
        assert_eq!(shapes.functions.len(), 1);
        let func = shapes
            .functions
            .resolve("http://example.org/ns#multiply")
            .expect("multiply registered");
        // Parameters ordered by sh:order; op2 is optional so required == 1.
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].var, "op1");
        assert_eq!(func.params[1].var, "op2");
        assert_eq!(func.required, 1);
        assert_eq!(func.kind, UserFnBody::Select);
        assert_eq!(
            func.params[0].constraint.datatype.as_deref(),
            Some("http://www.w3.org/2001/XMLSchema#integer")
        );
        assert_eq!(
            func.return_constraint.datatype.as_deref(),
            Some("http://www.w3.org/2001/XMLSchema#integer")
        );
    }

    #[test]
    fn sparql_function_with_both_select_and_ask_is_rejected() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:bad a sh:SPARQLFunction ;
                sh:select "SELECT ?result WHERE {{}}" ;
                sh:ask "ASK {{}}" .
            "#
        );
        let err = from_store(&load_store(&ttl)).expect_err("both bodies must fail");
        assert!(err.contains("both sh:select and sh:ask"), "got: {err}");
    }

    #[test]
    fn sparql_function_with_reserved_param_name_is_rejected() {
        // A parameter whose derived variable name is the SHACL-reserved `this`
        // would shadow the injected focus-node binding during evaluation.
        let ttl = format!(
            r#"{PREFIXES}
            ex:bad a sh:SPARQLFunction ;
                sh:parameter [ sh:path ex:this ; sh:order 1 ] ;
                sh:select "SELECT (1 AS ?result) WHERE {{}}" .
            "#
        );
        let err = from_store(&load_store(&ttl)).expect_err("reserved param name must fail");
        assert!(err.contains("reserved"), "got: {err}");
    }

    #[test]
    fn sparql_function_with_non_numeric_order_is_rejected() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:bad a sh:SPARQLFunction ;
                sh:parameter [ sh:path ex:op1 ; sh:order "first" ] ;
                sh:select "SELECT (1 AS ?result) WHERE {{}}" .
            "#
        );
        let err = from_store(&load_store(&ttl)).expect_err("non-numeric sh:order must fail");
        assert!(err.contains("sh:order"), "got: {err}");
    }

    #[test]
    fn sparql_function_with_colliding_param_names_is_rejected() {
        // Two parameters whose predicate local names both resolve to "arg".
        let ttl = format!(
            r#"{PREFIXES}
            @prefix other: <http://other.example/ns#> .
            ex:clash a sh:SPARQLFunction ;
                sh:parameter [ sh:path ex:arg ; sh:order 1 ] ;
                sh:parameter [ sh:path other:arg ; sh:order 2 ] ;
                sh:select "SELECT ?result WHERE {{}}" .
            "#
        );
        let err = from_store(&load_store(&ttl)).expect_err("collision must fail");
        assert!(err.contains("collides"), "got: {err}");
    }

    // ── Test 1: targetClass + sh:property with minCount/maxCount ──────────────

    #[test]
    fn test_target_class_and_property_min_max_count() {
        let ttl = format!(
            r"{PREFIXES}
            ex:PersonShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                    sh:maxCount 1 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");

        assert_eq!(shapes.node_shapes.len(), 1, "expected exactly 1 node shape");
        let shape = &shapes.node_shapes[0];

        // Must have one Class target pointing at ex:Person
        assert_eq!(shape.targets.len(), 1);
        match &shape.targets[0] {
            Target::Class(nn) => {
                assert_eq!(nn.as_str(), "http://example.org/ns#Person");
            }
            other => panic!("expected Target::Class, got {other:?}"),
        }

        // Must have one property shape
        assert_eq!(shape.property_shapes.len(), 1);
        let ps = &shape.property_shapes[0];

        // Path must be Predicate(ex:name)
        match &ps.path {
            Path::Predicate(nn) => {
                assert_eq!(nn.as_str(), "http://example.org/ns#name");
            }
            other => panic!("expected Path::Predicate, got {other:?}"),
        }

        // Constraints must include MinCount(1) and MaxCount(1)
        let has_min = ps
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::MinCount(1)));
        let has_max = ps
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::MaxCount(1)));
        assert!(has_min, "expected MinCount(1)");
        assert!(has_max, "expected MaxCount(1)");
    }

    // ── Test 2: sh:or with two property-path members ──────────────────────────

    #[test]
    fn test_or_with_two_predicate_path_members() {
        let ttl = format!(
            r"{PREFIXES}
            ex:AltShape a sh:NodeShape ;
                sh:or (
                    [ sh:path ex:a ; sh:minCount 1 ]
                    [ sh:path ex:b ; sh:minCount 1 ]
                ) .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");

        assert_eq!(shapes.node_shapes.len(), 1);
        let shape = &shapes.node_shapes[0];

        // Must have exactly one Or constraint
        let or_constraints: Vec<&Vec<Shape>> = shape
            .constraints
            .iter()
            .filter_map(|c| match c {
                Constraint::Or(members) => Some(members),
                _ => None,
            })
            .collect();
        assert_eq!(or_constraints.len(), 1, "expected exactly one sh:or");

        let members = or_constraints[0];
        assert_eq!(members.len(), 2, "expected two members in sh:or");

        // Each member should be an inline property shape with one property_shape entry
        for (i, member) in members.iter().enumerate() {
            assert_eq!(
                member.property_shapes.len(),
                1,
                "member {i} should have one property shape"
            );
            let ps = &member.property_shapes[0];
            let has_min = ps
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::MinCount(1)));
            assert!(has_min, "member {i} property shape should have MinCount(1)");
        }
    }

    // ── Test 3: sh:inversePath parses to Path::Inverse ────────────────────────

    #[test]
    fn test_inverse_path() {
        let ttl = format!(
            r"{PREFIXES}
            ex:InverseShape a sh:NodeShape ;
                sh:targetClass ex:Child ;
                sh:property [
                    sh:path [ sh:inversePath ex:parent ] ;
                    sh:minCount 1 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");

        assert_eq!(shapes.node_shapes.len(), 1);
        let shape = &shapes.node_shapes[0];
        assert_eq!(shape.property_shapes.len(), 1);
        let ps = &shape.property_shapes[0];

        match &ps.path {
            Path::Inverse(inner) => match inner.as_ref() {
                Path::Predicate(nn) => {
                    assert_eq!(nn.as_str(), "http://example.org/ns#parent");
                }
                other => panic!("expected inner Predicate, got {other:?}"),
            },
            other => panic!("expected Path::Inverse, got {other:?}"),
        }
    }

    // ── Test 4a: sh:sparql with valid query → parses successfully ────────────────

    #[test]
    fn test_sparql_constraint_parses() {
        // The sh:select value is a self-contained SPARQL query using full IRIs
        // (no prefix declarations needed) so the SPARQL parser can validate it.
        let ttl = format!(
            r#"{PREFIXES}
            ex:SparqlShape a sh:NodeShape ;
                sh:targetClass ex:Foo ;
                sh:sparql [
                    sh:select "SELECT $this WHERE {{ $this <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/ns#Foo> . }}" ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:sparql with a valid query must parse");

        assert_eq!(shapes.node_shapes.len(), 1, "expected exactly 1 node shape");
        let shape = &shapes.node_shapes[0];

        let sparql_c = shape.constraints.iter().find_map(|c| match c {
            Constraint::Sparql { select, .. } => Some(select.as_str()),
            _ => None,
        });
        assert!(
            sparql_c.is_some(),
            "expected a Constraint::Sparql, got: {:?}",
            shape.constraints
        );
        assert!(
            sparql_c.unwrap().contains("$this"),
            "select text should contain '$this'"
        );
    }

    // ── sh:namespace as a bare IRI (NamedNode), not an xsd:anyURI literal ────────
    #[test]
    fn test_sparql_prefixes_namespace_as_iri() {
        // SHACL §5.2.1 permits sh:namespace to be an IRI (NamedNode); the PREFIX
        // line must still be injected so the prefixed query name resolves.
        let ttl = format!(
            r#"{PREFIXES}
            ex:NsDecls sh:declare [ sh:prefix "ex" ; sh:namespace <http://example.org/ns#> ] .
            ex:PrefShape a sh:NodeShape ;
                sh:targetClass ex:Foo ;
                sh:prefixes ex:NsDecls ;
                sh:sparql [
                    sh:select "SELECT $this WHERE {{ $this a ex:Foo . }}" ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("IRI-valued sh:namespace must parse");
        let shape = &shapes.node_shapes[0];
        let select = shape
            .constraints
            .iter()
            .find_map(|c| match c {
                Constraint::Sparql { select, .. } => Some(select.clone()),
                _ => None,
            })
            .expect("expected a Constraint::Sparql");
        assert!(
            select.contains("PREFIX ex: <http://example.org/ns#>"),
            "IRI-valued sh:namespace must inject a PREFIX line; got: {select}"
        );
    }

    // ── Importable prefix set resolves through the production reader ─────────
    #[test]
    fn core_prefixes_import_resolves_registry_only_prefixes() {
        let vocab = purrdf_slice::SliceVocab::for_namespace("https://example.org/meta/");
        let core_set = purrdf_slice::emit_core_prefixes(&vocab);
        let shape = r#"
            meta:SpanImportProofShape a sh:NodeShape ;
                sh:prefixes meta:CorePrefixes ;
                sh:targetClass meta:Thing ;
                sh:sparql [
                    a sh:SPARQLConstraint ;
                    sh:message "registry-only prefixes must resolve via the imported set" ;
                    sh:select """
                        SELECT $this WHERE {
                            $this skos:prefLabel ?l ; dcterms:title ?t ; prov:wasDerivedFrom ?d .
                        }
                    """ ;
                ] .
        "#;
        let ttl = format!("{core_set}\n{shape}");

        let shapes = crate::engine::parse_shapes(&ttl, None)
            .expect("sh:prefixes meta:CorePrefixes must resolve registry-only prefixes");
        let select = shapes
            .node_shapes
            .iter()
            .find(|s| s.id.to_string().contains("SpanImportProofShape"))
            .and_then(|s| {
                s.constraints.iter().find_map(|c| match c {
                    Constraint::Sparql { select, .. } => Some(select.clone()),
                    _ => None,
                })
            })
            .expect("expected a Constraint::Sparql on the proof shape");

        for (prefix, ns) in [
            ("skos", "http://www.w3.org/2004/02/skos/core#"),
            ("dcterms", "http://purl.org/dc/terms/"),
            ("prov", "http://www.w3.org/ns/prov#"),
        ] {
            let line = format!("PREFIX {prefix}: <{ns}>");
            assert!(
                select.contains(&line),
                "registry prefix `{prefix}:` must resolve via meta:CorePrefixes; \
                 missing `{line}` in injected header:\n{select}"
            );
        }
    }

    // ── Test 4b: sh:sparql with malformed query → Err at parse time ──────────────

    #[test]
    fn test_sparql_constraint_malformed_query_errs() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:BadShape a sh:NodeShape ;
                sh:targetClass ex:Foo ;
                sh:sparql [
                    sh:select "SELECT $this WHERE {{" ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let result = from_store(&store);
        assert!(
            result.is_err(),
            "a malformed sh:select must cause a hard parse-time error, got {result:?}"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("unparsable") || err.contains("parse") || err.contains("syntax"),
            "error message should indicate a query parse failure, got: {err}"
        );
    }

    // ── Test 4c: sh:SPARQLTarget with an ASK query → Err at parse time ───────────

    #[test]
    fn test_sparql_target_ask_query_rejected() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:AskShape a sh:NodeShape ;
                sh:target [
                    a sh:SPARQLTarget ;
                    sh:select "ASK {{ ?this a <http://example.org/ns#Foo> }}" ;
                ] ;
                sh:property [ sh:path ex:p ; sh:minCount 1 ] .
        "#
        );
        let store = load_store(&ttl);
        let result = from_store(&store);
        assert!(
            result.is_err(),
            "a non-SELECT sh:SPARQLTarget must be rejected at shape-load, got {result:?}"
        );
        assert!(
            result.unwrap_err().contains("SELECT"),
            "error should explain that a SELECT is required"
        );
    }

    // ── Test 4d: sh:sparql constraint with a CONSTRUCT query → Err at parse time ──

    #[test]
    fn test_sparql_constraint_construct_query_rejected() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:ConstructShape a sh:NodeShape ;
                sh:targetClass ex:Foo ;
                sh:sparql [
                    sh:select "CONSTRUCT {{ ?this a <http://example.org/ns#Bar> }} WHERE {{ ?this a <http://example.org/ns#Foo> }}" ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let result = from_store(&store);
        assert!(
            result.is_err(),
            "a non-SELECT sh:sparql constraint must be rejected at shape-load, got {result:?}"
        );
        assert!(
            result.unwrap_err().contains("SELECT"),
            "error should explain that a SELECT is required"
        );
    }

    // ── Test 5: sh:qualifiedValueShape parses (§4.5.4) ─────────────────────────

    #[test]
    fn test_qualified_value_shape_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:QShape a sh:NodeShape ;
                sh:targetClass ex:Bar ;
                sh:property [
                    sh:path ex:item ;
                    sh:qualifiedValueShape [ sh:class ex:Item ] ;
                    sh:qualifiedMinCount 1 ;
                    sh:qualifiedMaxCount 3 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:qualifiedValueShape must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        let qvs = ps.constraints.iter().find_map(|c| match c {
            Constraint::QualifiedValueShape {
                shape,
                siblings,
                min_count,
                max_count,
                disjoint,
            } => Some((shape, siblings, min_count, max_count, disjoint)),
            _ => None,
        });
        let (shape, siblings, min_count, max_count, disjoint) =
            qvs.expect("expected QualifiedValueShape constraint");
        assert_eq!(*min_count, Some(1));
        assert_eq!(*max_count, Some(3));
        assert!(!disjoint, "no sh:qualifiedValueShapesDisjoint declared");
        assert!(siblings.is_empty(), "siblings only collected when disjoint");
        assert!(
            shape
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Class(_))),
            "qualified shape should carry sh:class"
        );
    }

    #[test]
    fn test_qualified_value_shape_disjoint_collects_siblings() {
        let ttl = format!(
            r"{PREFIXES}
            ex:HandShape a sh:NodeShape ;
                sh:targetClass ex:Hand ;
                sh:property [
                    sh:path ex:digit ;
                    sh:qualifiedValueShape [ sh:class ex:Thumb ] ;
                    sh:qualifiedMinCount 1 ;
                    sh:qualifiedValueShapesDisjoint true ;
                ] ;
                sh:property [
                    sh:path ex:digit ;
                    sh:qualifiedValueShape [ sh:class ex:Finger ] ;
                    sh:qualifiedMinCount 4 ;
                    sh:qualifiedValueShapesDisjoint true ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("disjoint qualified shapes must parse");
        let shape = &shapes.node_shapes[0];
        assert_eq!(shape.property_shapes.len(), 2);
        for ps in &shape.property_shapes {
            let (siblings, disjoint) = ps
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::QualifiedValueShape {
                        siblings, disjoint, ..
                    } => Some((siblings, disjoint)),
                    _ => None,
                })
                .expect("expected QualifiedValueShape");
            assert!(disjoint);
            assert_eq!(
                siblings.len(),
                1,
                "each qualified shape has exactly one sibling (the other one)"
            );
        }
    }

    #[test]
    fn test_qualified_value_shape_without_counts_errors() {
        let ttl = format!(
            r"{PREFIXES}
            ex:QShape a sh:NodeShape ;
                sh:targetClass ex:Bar ;
                sh:property [
                    sh:path ex:item ;
                    sh:qualifiedValueShape [ sh:class ex:Item ] ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let err = from_store(&store).expect_err("counts are required with a qualified shape");
        assert!(
            err.contains("qualifiedMinCount"),
            "error should name the missing count, got: {err}"
        );
    }

    // ── Property-pair constraints parse (§4.3) ─────────────────────────────────

    #[test]
    fn test_property_pair_constraints_parse() {
        let ttl = format!(
            r"{PREFIXES}
            ex:PairShape a sh:NodeShape ;
                sh:targetClass ex:Event ;
                sh:property [ sh:path ex:start ; sh:lessThan ex:end ] ;
                sh:property [ sh:path ex:first ; sh:lessThanOrEquals ex:last ] ;
                sh:property [ sh:path ex:a ; sh:equals ex:b ] ;
                sh:property [ sh:path ex:c ; sh:disjoint ex:d ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("property-pair constraints must parse");
        let shape = &shapes.node_shapes[0];
        let all: Vec<&Constraint> = shape
            .property_shapes
            .iter()
            .flat_map(|ps| ps.constraints.iter())
            .collect();
        assert!(all.iter().any(
            |c| matches!(c, Constraint::LessThan(Path::Predicate(n)) if n.as_str().ends_with("end"))
        ));
        assert!(
            all.iter().any(
                |c| matches!(c, Constraint::LessThanOrEquals(Path::Predicate(n)) if n.as_str().ends_with("last"))
            )
        );
        assert!(all.iter().any(
            |c| matches!(c, Constraint::Equals(Path::Predicate(n)) if n.as_str().ends_with('b'))
        ));
        assert!(all.iter().any(
            |c| matches!(c, Constraint::Disjoint(Path::Predicate(n)) if n.as_str().ends_with('d'))
        ));
    }

    #[test]
    fn test_property_pair_non_iri_object_errors() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:PairShape a sh:NodeShape ;
                sh:targetClass ex:Event ;
                sh:property [ sh:path ex:start ; sh:lessThan "notAnIri" ] .
        "#
        );
        let store = load_store(&ttl);
        let err = from_store(&store).expect_err("a literal sh:lessThan object must hard-fail");
        assert!(
            err.contains("lessThan") && err.contains("IRI"),
            "error should name the malformed pair object, got: {err}"
        );
    }

    // ── Test 6: severity, message, deactivated metadata ───────────────────────

    #[test]
    fn test_shape_metadata() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:MetaShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:severity sh:Warning ;
                sh:message "This is a warning" ;
                sh:deactivated true .
        "#
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");
        assert_eq!(shapes.node_shapes.len(), 1);
        let shape = &shapes.node_shapes[0];
        assert_eq!(shape.severity, Severity::Warning);
        assert_eq!(
            shape.messages.first().map(Literal::value),
            Some("This is a warning")
        );
        assert!(shape.deactivated);
    }

    // ── Test 6b: custom SHACL-SPARQL constraint component detection ────────────

    #[test]
    fn test_custom_component_constraint_detected() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/optional-001.ttl"
        ))
        .expect("fixture exists");
        let dataset: Arc<RdfDataset> = ::purrdf::parse_dataset(
            ttl.as_bytes(),
            "text/turtle",
            Some("http://datashapes.org/sh/tests/sparql/component/optional-001.test"),
        )
        .expect("fixture parses");
        let shapes = from_dataset(&dataset).expect("shapes with custom component must parse");

        let test_shape1 = shapes
            .node_shapes
            .iter()
            .find(|s| s.id.to_string().contains("TestShape1"))
            .expect("TestShape1 present");
        let (component, bindings, validator) = test_shape1
            .constraints
            .iter()
            .find_map(|c| match c {
                Constraint::Component {
                    component,
                    bindings,
                    validator,
                    ..
                } => Some((component, bindings, validator)),
                _ => None,
            })
            .expect("TestShape1 should have a Constraint::Component");

        assert_eq!(
            component.as_str(),
            "http://datashapes.org/sh/tests/sparql/component/optional-001.test#TestConstraintComponent"
        );
        assert_eq!(bindings.len(), 1, "only requiredParam is bound");
        assert_eq!(bindings[0].0, "requiredParam");
        assert!(
            matches!(bindings[0].1, Term::Literal(_)),
            "binding value should be a literal"
        );
        assert!(
            matches!(validator, ComponentValidator::Ask { .. }),
            "optional-001 validator is ASK"
        );
    }

    // ── Test 7: sh:in list ─────────────────────────────────────────────────────

    #[test]
    fn test_in_list_constraint() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:InShape a sh:NodeShape ;
                sh:targetClass ex:Color ;
                sh:property [
                    sh:path ex:value ;
                    sh:in ( "red" "green" "blue" ) ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");
        assert_eq!(shapes.node_shapes.len(), 1);
        let ps = &shapes.node_shapes[0].property_shapes[0];
        let in_constraint = ps.constraints.iter().find_map(|c| match c {
            Constraint::In(items) => Some(items),
            _ => None,
        });
        assert!(in_constraint.is_some(), "expected In constraint");
        assert_eq!(in_constraint.unwrap().len(), 3);
    }

    // ── Test 8: sh:nodeKind ────────────────────────────────────────────────────

    #[test]
    fn test_node_kind_iri() {
        let ttl = format!(
            r"{PREFIXES}
            ex:IriShape a sh:NodeShape ;
                sh:targetClass ex:Resource ;
                sh:nodeKind sh:IRI .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");
        let shape = &shapes.node_shapes[0];
        let has_nk = shape
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::NodeKind(kinds) if kinds.as_slice() == [NodeKindValue::Iri]));
        assert!(has_nk, "expected NodeKind(Iri)");
    }

    // ── Test 9: sh:pattern + sh:flags ─────────────────────────────────────────

    #[test]
    fn test_pattern_with_flags() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:PatShape a sh:NodeShape ;
                sh:targetClass ex:Code ;
                sh:property [
                    sh:path ex:code ;
                    sh:pattern "^[A-Z]+" ;
                    sh:flags "i" ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        let pat = ps.constraints.iter().find_map(|c| match c {
            Constraint::Pattern { regex, flags, .. } => Some((regex, flags)),
            _ => None,
        });
        assert!(pat.is_some(), "expected Pattern constraint");
        let (regex, flags) = pat.unwrap();
        assert_eq!(regex.as_str(), "^[A-Z]+");
        assert_eq!(flags.as_deref(), Some("i"));
    }

    // ── Test 10: sh:and ────────────────────────────────────────────────────────

    #[test]
    fn test_and_constraint() {
        let ttl = format!(
            r"{PREFIXES}
            ex:AndShape a sh:NodeShape ;
                sh:targetClass ex:Entity ;
                sh:and (
                    [ sh:class ex:Named ]
                    [ sh:nodeKind sh:IRI ]
                ) .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("parse must succeed");
        let shape = &shapes.node_shapes[0];
        let and_c = shape.constraints.iter().find_map(|c| match c {
            Constraint::And(members) => Some(members),
            _ => None,
        });
        assert!(and_c.is_some(), "expected And constraint");
        assert_eq!(and_c.unwrap().len(), 2);
    }

    // ── Test 11: all composite path forms parse (§2.3.1) ───────────────────────

    #[test]
    fn test_zero_or_more_path_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:StarShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [
                    sh:path [ sh:zeroOrMorePath ex:link ] ;
                    sh:nodeKind sh:IRI ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:zeroOrMorePath must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        match &ps.path {
            Path::ZeroOrMore(inner) => match inner.as_ref() {
                Path::Predicate(nn) => assert_eq!(nn.as_str(), "http://example.org/ns#link"),
                other => panic!("expected inner Predicate, got {other:?}"),
            },
            other => panic!("expected Path::ZeroOrMore, got {other:?}"),
        }
    }

    #[test]
    fn test_one_or_more_and_zero_or_one_paths_parse() {
        let ttl = format!(
            r"{PREFIXES}
            ex:PlusShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [ sh:path [ sh:oneOrMorePath ex:link ] ; sh:nodeKind sh:IRI ] .
            ex:OptShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [ sh:path [ sh:zeroOrOnePath ex:link ] ; sh:nodeKind sh:IRI ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("closure paths must parse");
        let mut saw_plus = false;
        let mut saw_opt = false;
        for shape in &shapes.node_shapes {
            for ps in &shape.property_shapes {
                match &ps.path {
                    Path::OneOrMore(_) => saw_plus = true,
                    Path::ZeroOrOne(_) => saw_opt = true,
                    other => panic!("expected a closure path, got {other:?}"),
                }
            }
        }
        assert!(saw_plus, "expected a OneOrMore path");
        assert!(saw_opt, "expected a ZeroOrOne path");
    }

    #[test]
    fn test_sequence_path_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:SeqShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [
                    sh:path ( ex:address ex:city ) ;
                    sh:minCount 1 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sequence path must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        match &ps.path {
            Path::Sequence(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], Path::Predicate(n) if n.as_str().ends_with("address")));
                assert!(matches!(&parts[1], Path::Predicate(n) if n.as_str().ends_with("city")));
            }
            other => panic!("expected Path::Sequence, got {other:?}"),
        }
    }

    #[test]
    fn test_alternative_path_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:AltShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [
                    sh:path [ sh:alternativePath ( ex:email ex:phone ) ] ;
                    sh:minCount 1 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("alternative path must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        match &ps.path {
            Path::Alternative(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], Path::Predicate(n) if n.as_str().ends_with("email")));
                assert!(matches!(&parts[1], Path::Predicate(n) if n.as_str().ends_with("phone")));
            }
            other => panic!("expected Path::Alternative, got {other:?}"),
        }
    }

    #[test]
    fn test_nested_path_combination_parses() {
        // An alternative whose second branch is an inverse of a zeroOrMore —
        // nested combinations must compose.
        let ttl = format!(
            r"{PREFIXES}
            ex:NestShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [
                    sh:path [ sh:alternativePath (
                        ( ex:a ex:b )
                        [ sh:inversePath [ sh:zeroOrMorePath ex:c ] ]
                    ) ] ;
                    sh:minCount 1 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("nested path combination must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        let Path::Alternative(parts) = &ps.path else {
            panic!("expected Path::Alternative, got {:?}", ps.path);
        };
        assert_eq!(parts.len(), 2);
        assert!(matches!(&parts[0], Path::Sequence(seq) if seq.len() == 2));
        assert!(matches!(
            &parts[1],
            Path::Inverse(inner) if matches!(inner.as_ref(), Path::ZeroOrMore(_))
        ));
    }

    #[test]
    fn test_single_member_sequence_path_errors() {
        let ttl = format!(
            r"{PREFIXES}
            ex:SeqShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [ sh:path ( ex:only ) ; sh:minCount 1 ] .
        "
        );
        let store = load_store(&ttl);
        let err = from_store(&store).expect_err("a one-member sequence path is malformed");
        assert!(
            err.contains("at least two members"),
            "error should explain the arity rule, got: {err}"
        );
    }

    #[test]
    fn test_reifier_shape_requires_predicate_path() {
        let ttl = format!(
            r"{PREFIXES}
            ex:ContextualShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [
                    sh:path [ sh:inversePath ex:knows ] ;
                    sh:reifierShape ex:StatementContextShape ;
                ] .

            ex:StatementContextShape a sh:NodeShape .
        "
        );
        let store = load_store(&ttl);
        let result = from_store(&store);
        assert!(
            result.is_err(),
            "sh:reifierShape on a non-IRI path must cause a hard error"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("requires an IRI sh:path"),
            "error should document the supported path boundary, got: {err}"
        );
    }

    // ──: sh:maxLength parses to Constraint::MaxLength ────────────────────

    #[test]
    fn test_max_length_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:MaxLenShape a sh:NodeShape ;
                sh:targetClass ex:Tag ;
                sh:property [
                    sh:path ex:code ;
                    sh:maxLength 5 ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:maxLength must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        let has_max = ps
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::MaxLength(5)));
        assert!(has_max, "expected MaxLength(5), got {:?}", ps.constraints);
    }

    // ──: sh:languageIn parses to Constraint::LanguageIn ──────────────────

    #[test]
    fn test_language_in_parses() {
        let ttl = format!(
            r#"{PREFIXES}
            ex:LangShape a sh:NodeShape ;
                sh:targetClass ex:Doc ;
                sh:property [
                    sh:path ex:label ;
                    sh:languageIn ( "en" "fr" ) ;
                ] .
        "#
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:languageIn must parse");
        let ps = &shapes.node_shapes[0].property_shapes[0];
        let tags = ps.constraints.iter().find_map(|c| match c {
            Constraint::LanguageIn(tags) => Some(tags),
            _ => None,
        });
        assert!(
            tags.is_some(),
            "expected LanguageIn, got {:?}",
            ps.constraints
        );
        assert_eq!(
            tags.unwrap().as_slice(),
            &["en".to_owned(), "fr".to_owned()]
        );
    }

    // ──: sh:not parses to Constraint::Not(nested shape) ──────────────────

    #[test]
    fn test_not_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:NotShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:not [ sh:nodeKind sh:Literal ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:not must parse");
        let shape = &shapes.node_shapes[0];
        let not_c = shape.constraints.iter().find_map(|c| match c {
            Constraint::Not(inner) => Some(inner),
            _ => None,
        });
        assert!(not_c.is_some(), "expected Not, got {:?}", shape.constraints);
        let inner = not_c.unwrap();
        assert!(
            inner
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::NodeKind(kinds) if kinds.as_slice() == [NodeKindValue::Literal])),
            "nested shape should carry NodeKind(Literal)"
        );
    }

    // ──: sh:closed true (+ sh:ignoredProperties) parses to Closed ────────

    #[test]
    fn test_closed_parses() {
        let ttl = format!(
            r"{PREFIXES}
            ex:ClosedShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:closed true ;
                sh:ignoredProperties ( rdf:type ex:extra ) ;
                sh:property [ sh:path ex:name ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:closed must parse");
        let shape = &shapes.node_shapes[0];
        let ignored = shape.constraints.iter().find_map(|c| match c {
            Constraint::Closed {
                ignored,
                mode: ClosedMode::Declared,
            } => Some(ignored),
            _ => None,
        });
        assert!(
            ignored.is_some(),
            "expected Closed, got {:?}",
            shape.constraints
        );
        let ignored = ignored.unwrap();
        assert!(
            ignored
                .iter()
                .any(|n| n.as_str() == "http://example.org/ns#extra"),
            "ignoredProperties should include ex:extra"
        );
    }

    #[test]
    fn test_ignored_properties_non_iri_member_errors() {
        // A non-IRI sh:ignoredProperties member (a literal) is malformed: the
        // shapes graph must HARD-fail to load rather than silently dropping it.
        let ttl = format!(
            r#"{PREFIXES}
            ex:ClosedShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:closed true ;
                sh:ignoredProperties ( rdf:type "oops" ) ;
                sh:property [ sh:path ex:name ] .
        "#
        );
        let store = load_store(&ttl);
        let err = from_store(&store).expect_err("non-IRI ignoredProperties member must error");
        assert!(
            err.contains("ignoredProperties") && err.contains("non-IRI"),
            "error should name the malformed ignoredProperties member, got: {err}"
        );
    }

    // ──: sh:closed false emits NO Closed constraint ──────────────────────

    #[test]
    fn test_closed_false_emits_nothing() {
        let ttl = format!(
            r"{PREFIXES}
            ex:OpenShape a sh:NodeShape ;
                sh:targetClass ex:Person ;
                sh:closed false ;
                sh:property [ sh:path ex:name ] .
        "
        );
        let store = load_store(&ttl);
        let shapes = from_store(&store).expect("sh:closed false must parse");
        let shape = &shapes.node_shapes[0];
        assert!(
            !shape
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Closed { .. })),
            "sh:closed false must not emit a Closed constraint"
        );
    }

    #[test]
    fn test_reification_required_requires_predicate_path() {
        let ttl = format!(
            r"{PREFIXES}
            ex:ContextualShape a sh:NodeShape ;
                sh:targetClass ex:Node ;
                sh:property [
                    sh:path [ sh:inversePath ex:knows ] ;
                    sh:reificationRequired true ;
                ] .
        "
        );
        let store = load_store(&ttl);
        let result = from_store(&store);
        assert!(
            result.is_err(),
            "sh:reificationRequired on a non-IRI path must cause a hard error"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("sh:reifierShape or sh:reificationRequired")
                && err.contains("requires an IRI sh:path"),
            "error should document the supported path boundary, got: {err}"
        );
    }

    // ── SHACL-AF node expressions (Task 2 parser) ──────────────────────────────

    /// Build a `Parser` over a Turtle shapes graph and parse `subject`'s
    /// `ex:expr` object as a node expression.
    fn parse_expr(ttl: &str) -> Result<NodeExpr, String> {
        let dataset = load_store(ttl);
        let mut parser = Parser::new(
            dataset.as_ref(),
            None,
            &[],
            None,
            Arc::clone(&dataset),
            None,
        );
        let root = Term::NamedNode(NamedNode::from("http://example.org/ns#root"));
        let expr_obj = parser
            .first_object_of(&root, "http://example.org/ns#expr")
            .expect("ex:root ex:expr must be present");
        parser.parse_node_expr(&expr_obj)
    }

    /// Wrap an `ex:expr` object triple in the standard prefix header.
    fn expr_ttl(body: &str) -> String {
        format!("{PREFIXES}\n{body}")
    }

    #[test]
    fn node_expr_constant_iri() {
        let expr = parse_expr(&expr_ttl("ex:root ex:expr ex:someConstant .")).expect("parse");
        match expr {
            NodeExpr::Constant(Term::NamedNode(n)) => {
                assert_eq!(n.as_str(), "http://example.org/ns#someConstant");
            }
            other => panic!("expected Constant(NamedNode), got {other:?}"),
        }
    }

    #[test]
    fn node_expr_this() {
        let expr = parse_expr(&expr_ttl("ex:root ex:expr sh:this .")).expect("parse");
        assert!(matches!(expr, NodeExpr::This), "got {expr:?}");
    }

    #[test]
    fn node_expr_path() {
        let expr = parse_expr(&expr_ttl("ex:root ex:expr [ sh:path ex:knows ] .")).expect("parse");
        match expr {
            NodeExpr::Path(Path::Predicate(n)) => {
                assert_eq!(n.as_str(), "http://example.org/ns#knows");
            }
            other => panic!("expected Path(Predicate), got {other:?}"),
        }
    }

    #[test]
    fn node_expr_union_two_members() {
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:union ( sh:this [ sh:path ex:knows ] ) ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Union(members) => {
                assert_eq!(members.len(), 2);
                assert!(matches!(members[0], NodeExpr::This));
                assert!(matches!(members[1], NodeExpr::Path(_)));
            }
            other => panic!("expected Union, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_intersection_two_members() {
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:intersection ( [ sh:path ex:a ] [ sh:path ex:b ] ) ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Intersection(members) => assert_eq!(members.len(), 2),
            other => panic!("expected Intersection, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_if_then_else() {
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:if sh:this ; sh:then ex:yes ; sh:else ex:no ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::If { cond, then, els } => {
                assert!(matches!(*cond, NodeExpr::This));
                assert!(matches!(*then, NodeExpr::Constant(_)));
                assert!(matches!(*els, NodeExpr::Constant(_)));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_if_missing_branches_default_empty() {
        let expr = parse_expr(&expr_ttl("ex:root ex:expr [ sh:if sh:this ] .")).expect("parse");
        match expr {
            NodeExpr::If { then, els, .. } => {
                // A missing branch is the SHACL 1.2 Node Expressions §4.1.1 empty
                // expression, which is the arm that names "no output nodes"
                // directly rather than encoding it as a zero-operand union.
                assert!(matches!(*then, NodeExpr::Empty));
                assert!(matches!(*els, NodeExpr::Empty));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_count_plain() {
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:count [ sh:path ex:knows ] ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Count { distinct, of } => {
                assert!(!distinct, "plain count is not distinct");
                assert!(matches!(*of, NodeExpr::Path(_)));
            }
            other => panic!("expected Count, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_count_distinct() {
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:count [ sh:distinct [ sh:path ex:knows ] ] ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Count { distinct, of } => {
                assert!(distinct, "sh:count over sh:distinct is a distinct count");
                assert!(
                    matches!(*of, NodeExpr::Path(_)),
                    "inner unwraps the distinct"
                );
            }
            other => panic!("expected Count, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_min_max_sum() {
        let min = parse_expr(&expr_ttl("ex:root ex:expr [ sh:min [ sh:path ex:v ] ] ."))
            .expect("parse min");
        assert!(matches!(min, NodeExpr::Min(_)), "got {min:?}");
        let max = parse_expr(&expr_ttl("ex:root ex:expr [ sh:max [ sh:path ex:v ] ] ."))
            .expect("parse max");
        assert!(matches!(max, NodeExpr::Max(_)), "got {max:?}");
        let sum = parse_expr(&expr_ttl("ex:root ex:expr [ sh:sum [ sh:path ex:v ] ] ."))
            .expect("parse sum");
        assert!(matches!(sum, NodeExpr::Sum(_)), "got {sum:?}");
    }

    #[test]
    fn node_expr_exists_is_a_node_expression() {
        // Adopted semantics: `sh:exists` takes a NODE EXPRESSION, not a shape.
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:exists [ sh:path ex:p ] ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Exists(inner) => {
                assert!(
                    matches!(*inner, NodeExpr::Path(_)),
                    "exists operand should be a node expression, got {inner:?}"
                );
            }
            other => panic!("expected Exists, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_filter_shape() {
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:filterShape [ sh:nodeKind sh:IRI ] ; sh:nodes sh:this ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Filter { nodes, shape } => {
                assert!(matches!(*nodes, NodeExpr::This));
                assert!(
                    shape
                        .constraints
                        .iter()
                        .any(|c| matches!(c, Constraint::NodeKind(kinds) if kinds.as_slice() == [NodeKindValue::Iri]))
                );
            }
            other => panic!("expected Filter, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_filter_shape_missing_nodes_errors() {
        let err = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:filterShape [ sh:nodeKind sh:IRI ] ] .",
        ))
        .expect_err("sh:filterShape without sh:nodes must hard-fail");
        assert!(err.contains("sh:nodes"), "got: {err}");
    }

    #[test]
    fn node_expr_builtin_function_call() {
        // A function IRI with no rdf:type classification is a builtin.
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ ex:multiply ( sh:this ex:two ) ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Call(FnCall::Builtin { iri, args }) => {
                assert_eq!(iri.as_str(), "http://example.org/ns#multiply");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], NodeExpr::This));
            }
            other => panic!("expected Call(Builtin), got {other:?}"),
        }
    }

    #[test]
    fn node_expr_user_defined_function_call() {
        // The function IRI is typed sh:SPARQLFunction → user-defined.
        let expr = parse_expr(&expr_ttl(
            "ex:myFn a sh:SPARQLFunction .\n\
             ex:root ex:expr [ ex:myFn ( sh:this ) ] .",
        ))
        .expect("parse");
        match expr {
            NodeExpr::Call(FnCall::UserDefined { iri, args }) => {
                assert_eq!(iri.as_str(), "http://example.org/ns#myFn");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Call(UserDefined), got {other:?}"),
        }
    }

    #[test]
    fn node_expr_ambiguous_keys_error() {
        let err = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:min [ sh:path ex:a ] ; sh:max [ sh:path ex:b ] ] .",
        ))
        .expect_err("two mutually-exclusive expression keys must hard-fail");
        assert!(err.contains("ambiguous"), "got: {err}");
    }

    #[test]
    fn node_expr_limit_offset_orderby_wrap_core() {
        // Paging keys wrap the same node's core expression: LIMIT(OFFSET(ORDERBY(core))).
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ sh:path ex:v ; sh:orderby sh:this ; sh:desc true ; sh:offset 2 ; sh:limit 5 ] .",
        ))
        .expect("parse");
        let NodeExpr::Limit { of, n } = expr else {
            panic!("expected outermost Limit, got {expr:?}");
        };
        assert_eq!(n, 5);
        let NodeExpr::Offset { of, n } = *of else {
            panic!("expected Offset under Limit, got {of:?}");
        };
        assert_eq!(n, 2);
        let NodeExpr::OrderBy {
            of,
            key,
            descending,
        } = *of
        else {
            panic!("expected OrderBy under Offset, got {of:?}");
        };
        assert!(descending, "sh:desc true ⇒ descending");
        assert!(matches!(*key, NodeExpr::This), "sort key is sh:this");
        assert!(matches!(*of, NodeExpr::Path(_)), "core is the path");
    }

    #[test]
    fn node_expr_function_call_with_orderby_and_desc() {
        // A blank-node function-call core that ALSO carries the paging keys
        // `sh:orderby` + `sh:desc` must parse cleanly: `sh:desc` is a wrapper
        // predicate, NOT an extra function-call candidate (regression: without
        // excluding it from the KNOWN scan the node reads as ambiguous).
        let expr = parse_expr(&expr_ttl(
            "ex:root ex:expr [ <http://www.w3.org/2005/xpath-functions#numeric-abs> ( ex:x ) ; \
             sh:orderby sh:this ; sh:desc true ] .",
        ))
        .expect("function call + orderby + desc must parse, not report ambiguous");
        let NodeExpr::OrderBy { of, descending, .. } = expr else {
            panic!("expected OrderBy wrapping the call, got {expr:?}");
        };
        assert!(descending, "sh:desc true ⇒ descending");
        match *of {
            NodeExpr::Call(FnCall::Builtin { iri, args }) => {
                assert_eq!(
                    iri.as_str(),
                    "http://www.w3.org/2005/xpath-functions#numeric-abs"
                );
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Call(Builtin) core, got {other:?}"),
        }
    }

    #[test]
    fn node_expr_described_constant_iri_is_constant_not_call() {
        // A constant IRI referenced as a node expression that also bears an
        // UNRELATED outgoing triple (`rdfs:label`) in the shapes graph must parse
        // as `Constant`, not be misread as a function call (regression: a
        // NamedNode reaching the call scan with any other triple was ambiguous).
        let expr = parse_expr(&expr_ttl(
            "ex:someConst rdfs:label \"x\" .\n\
             ex:root ex:expr [ sh:union ( ex:someConst ) ] .",
        ))
        .expect("described constant IRI in a union must parse");
        let NodeExpr::Union(members) = expr else {
            panic!("expected Union, got {expr:?}");
        };
        assert_eq!(members.len(), 1);
        match &members[0] {
            NodeExpr::Constant(Term::NamedNode(n)) => {
                assert_eq!(n.as_str(), "http://example.org/ns#someConst");
            }
            other => panic!("expected Constant(NamedNode), got {other:?}"),
        }
    }
}
