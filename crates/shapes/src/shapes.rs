// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL shapes graph parser.
//!
//! Parses a SHACL Core shapes graph (loaded into an oxigraph `Store`) into a
//! fully typed [`Shapes`] structure.  No evaluation logic lives here — that is
//! Task 3.  Covers full SHACL Core: all six property-path forms (§2.3.1),
//! property-pair constraints (§4.3), qualified value shapes (§4.5.4–4.5.5), and
//! SHACL-AF SPARQL constraints/targets.  Malformed constructs (e.g. a literal
//! `sh:equals` object, a one-member sequence path) cause a hard `Err` rather
//! than a silent skip.

use std::sync::{Arc, OnceLock};

use ::purrdf::FastSet;
use ::purrdf::RdfDataset;

use purrdf_sparql_eval::UserFunctionRegistry;

use crate::components::{ComponentRegistry, severity_from_term};
use crate::data::{GraphFilter, native_quads};
use crate::expression::NodeExpr;
use crate::model::{BoxRoleVocab, rdf, rdfs, sh};
use crate::report::Severity;
use crate::term::{NamedNode, Term};

mod parser;

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
    /// `sh:targetNode ex:SomeNode` (or a literal)
    Node(Term),
    /// The shape node is itself an `rdfs:Class` → implicit class target.
    ImplicitClass(Term),
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
    Ask { ask: String },
    /// A `SELECT` query validator (`sh:SPARQLSelectValidator`).
    Select { select: String },
}

/// A single SHACL constraint on a shape or property shape.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// `sh:class ex:C`
    Class(NamedNode),
    /// `sh:datatype xsd:integer`
    Datatype(NamedNode),
    /// `sh:nodeKind sh:IRI` etc.
    NodeKind(NodeKindValue),
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
        /// `Err(String)` stores the compilation error so per-value violation
        /// semantics (bad regex → violation, not hard abort) are preserved.
        compiled: Arc<OnceLock<Result<regex::Regex, String>>>,
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
    /// `sh:closed true` (with optional `sh:ignoredProperties`).
    ///
    /// A node-shape-level constraint: every predicate used on the focus node must
    /// be declared by one of the shape's `sh:property` simple-predicate paths, be
    /// listed in `ignored`, or be `rdf:type` (always allowed). Only emitted when
    /// `sh:closed true`.
    Closed {
        /// Predicates explicitly exempted from the closed-world check
        /// (`sh:ignoredProperties`), in addition to the implicit `rdf:type`.
        ignored: Vec<NamedNode>,
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
        message: Option<String>,
        /// Optional per-constraint severity override (from `sh:severity` on the
        /// constraint blank node).
        severity: Option<Severity>,
    },
    /// `sh:equals ex:p` — the value node set must equal the objects of `ex:p`
    /// from the same focus node (spec §4.3.1).
    Equals(NamedNode),
    /// `sh:disjoint ex:p` — no value node may also be an object of `ex:p` from
    /// the same focus node (spec §4.3.2).
    Disjoint(NamedNode),
    /// `sh:lessThan ex:p` — every value node must be `<` every object of `ex:p`
    /// from the same focus node, under SPARQL `<` semantics (spec §4.3.3).
    LessThan(NamedNode),
    /// `sh:lessThanOrEquals ex:p` — every value node must be `<=` every object
    /// of `ex:p` from the same focus node (spec §4.3.4).
    LessThanOrEquals(NamedNode),
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
        message: Option<String>,
        /// Optional per-constraint severity override (from `sh:severity` on the
        /// expression node).
        severity: Option<Severity>,
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
        message: Option<String>,
        /// Optional severity override (shape → validator → component).
        severity: Option<Severity>,
    },
}

/// A property shape, reached via `sh:property` from a node shape.
#[derive(Debug, Clone)]
pub struct PropertyShape {
    /// The property path this shape applies to.
    pub path: Path,
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
    /// Optional human-readable message.
    pub message: Option<String>,
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
    /// Optional human-readable message.
    pub message: Option<String>,
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
    /// Node shapes extracted from the shapes graph.
    pub node_shapes: Vec<Shape>,
    /// The caller-supplied box-role vocabulary these shapes were parsed with;
    /// carried into validation so data-graph role lookups use the same terms.
    /// `None` = the box-role feature is inactive.
    pub box_role_vocab: Option<BoxRoleVocab>,
    /// SHACL-AF SPARQL-based functions (`sh:SPARQLFunction`) declared in the shapes
    /// graph, built once here and threaded into the SPARQL evaluator so calls in
    /// `sh:sparql`/`sh:SPARQLTarget` queries and `sh:expression` node expressions
    /// resolve. Empty when the graph declares no functions.
    pub functions: Arc<UserFunctionRegistry>,
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
}

impl Default for Shapes {
    fn default() -> Self {
        Self {
            node_shapes: Vec::new(),
            box_role_vocab: None,
            functions: Arc::new(UserFunctionRegistry::new()),
            target_types: std::collections::BTreeMap::new(),
            shapes_graph: None,
            shapes_dataset: ::purrdf::RdfDatasetBuilder::new()
                .freeze()
                .expect("empty shapes dataset"),
        }
    }
}

// ── Public entry point ─────────────────────────────────────────────────────────

/// Parse shapes from a frozen [`RdfDataset`].
///
/// Identifies node shapes, parses all their targets, constraints, and property
/// shapes.  Unsupported SHACL features return `Err` immediately (hard-fail).
///
/// # Errors
///
/// Returns `Err(String)` when an unsupported SHACL construct is encountered or
/// when required structural data (e.g. `sh:path`) is missing.
pub fn from_dataset(dataset: &Arc<RdfDataset>) -> Result<Shapes, String> {
    from_dataset_with_prefixes(dataset, &[])
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
/// Returns `Err(String)` on any unsupported SHACL construct or missing
/// structural data — see [`from_dataset`].
pub fn from_dataset_with_prefixes(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
) -> Result<Shapes, String> {
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
/// Returns `Err(String)` on any unsupported SHACL construct or missing
/// structural data — see [`from_dataset`].
pub fn from_dataset_with_config(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
) -> Result<Shapes, String> {
    from_dataset_with_config_and_graph(dataset, doc_prefixes, box_role_vocab, None)
}

/// Parse shapes from a dataset with the full caller configuration plus an
/// explicit shapes-graph IRI. The original `dataset` is retained as
/// `Shapes::shapes_dataset` so the validation engine can expose it as a
/// named graph to SHACL-SPARQL queries.
pub fn from_dataset_with_config_and_graph(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
) -> Result<Shapes, String> {
    let mut parser = Parser::new(
        dataset.as_ref(),
        doc_prefixes,
        box_role_vocab,
        Arc::clone(dataset),
        shapes_graph,
    );
    parser.parse()
}

// ── Internal parser ────────────────────────────────────────────────────────────

pub(crate) struct Parser<'s> {
    data: &'s RdfDataset,
    /// Tracks shape nodes currently being parsed to prevent infinite recursion
    /// through `sh:node` or `sh:and/or/xone` cycles.
    in_flight: FastSet<String>,
    /// The shapes document's `@prefix` map (prefix → namespace), used as the
    /// fallback PREFIX header for SHACL-AF `sh:select` queries.
    doc_prefixes: Vec<(String, String)>,
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
    target_types: std::collections::BTreeMap<String, SparqlTargetType>,
}

// ── Prefix-header helper (used by shapes and component registry) ───────────────

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

/// Return the first object for `(subject, predicate, ?)`, if any.
fn first_object_of(data: &RdfDataset, subject: &Term, predicate: &str) -> Option<Term> {
    objects_of(data, subject, predicate).into_iter().next()
}

/// Build the SPARQL `PREFIX` header prepended to a SHACL-SPARQL query.
///
/// Two sources contribute, lowest precedence first:
///
/// 1. The shapes **document's** `@prefix` declarations (the pySHACL-compatible
///    fallback — real shapes rely on these without `sh:prefixes`).
/// 2. SHACL-AF `sh:prefixes` → `sh:declare` on the `owners` (spec §5.2.1), which
///    **override** the document fallback for any colliding prefix.
///
/// Output is one `PREFIX p: <ns>` line per prefix, sorted (a `BTreeMap` keeps
/// it deterministic and one-entry-per-prefix). Empty when nothing is declared.
pub(crate) fn build_prefix_header(
    data: &RdfDataset,
    doc_prefixes: &[(String, String)],
    owners: &[&Term],
) -> String {
    let mut map: std::collections::BTreeMap<String, String> = doc_prefixes
        .iter()
        .map(|(p, ns)| (p.clone(), ns.clone()))
        .collect();
    // `sh:prefix` is a string literal; `sh:namespace` is typically an
    // `xsd:anyURI` literal but the SHACL spec also permits a bare IRI
    // (NamedNode). Accept both lexical forms so an IRI-valued namespace is
    // not silently dropped (which would omit a PREFIX line and break the
    // dependent SHACL-AF query — a silent under-validation, P11/§11).
    let term_value = |t: Term| match t {
        Term::Literal(lit) => Some(lit.value().to_owned()),
        Term::NamedNode(node) => Some(node.as_str().to_owned()),
        _ => None, // blank node / quoted triple: not a prefix or namespace value
    };
    for owner in owners {
        for prefixes_node in objects_of(data, owner, sh::PREFIXES) {
            for declare in objects_of(data, &prefixes_node, sh::DECLARE) {
                let prefix = first_object_of(data, &declare, sh::PREFIX).and_then(term_value);
                let namespace = first_object_of(data, &declare, sh::NAMESPACE).and_then(term_value);
                if let (Some(p), Some(ns)) = (prefix, namespace) {
                    map.insert(p, ns); // sh:prefixes overrides the document fallback
                }
            }
        }
    }
    use std::fmt::Write as _;
    let mut header = String::new();
    for (prefix, namespace) in map {
        let _ = writeln!(header, "PREFIX {prefix}: <{namespace}>");
    }
    header
}

impl<'s> Parser<'s> {
    fn new(
        data: &'s RdfDataset,
        doc_prefixes: &[(String, String)],
        box_role_vocab: Option<BoxRoleVocab>,
        shapes_dataset: Arc<RdfDataset>,
        shapes_graph: Option<String>,
    ) -> Self {
        Self {
            data,
            in_flight: FastSet::default(),
            doc_prefixes: doc_prefixes.to_vec(),
            box_role_vocab,
            component_registry: ComponentRegistry::default(),
            shapes_dataset,
            shapes_graph,
            target_types: std::collections::BTreeMap::new(),
        }
    }

    fn parse(&mut self) -> Result<Shapes, String> {
        // --- collect all top-level shape node terms ---
        let mut shape_ids: FastSet<Term> = FastSet::default();
        // Track which nodes are property-shape-only (reachable only via sh:property)
        // so we don't list them as top-level node shapes.
        let mut property_shape_nodes: FastSet<Term> = FastSet::default();

        // 1. Nodes typed sh:NodeShape
        for (subject, _, _) in self.quads_with(None, Some(rdf::TYPE), Some(sh::NODE_SHAPE)) {
            shape_ids.insert(subject);
        }

        // 2. Nodes typed sh:PropertyShape (collect to exclude from top-level)
        for (subject, _, _) in self.quads_with(None, Some(rdf::TYPE), Some(sh::PROPERTY_SHAPE)) {
            property_shape_nodes.insert(subject);
        }

        // 3. Subjects of sh:targetClass / sh:targetSubjectsOf / sh:targetObjectsOf / sh:targetNode
        for pred in [
            sh::TARGET_CLASS,
            sh::TARGET_SUBJECTS_OF,
            sh::TARGET_OBJECTS_OF,
            sh::TARGET_NODE,
        ] {
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

        // Custom SHACL-SPARQL constraint components are parsed up-front; any
        // malformed component, parameter, or validator query is a hard failure.
        self.component_registry = ComponentRegistry::parse(self.data, &self.doc_prefixes)?;

        // SHACL-AF parameterized target types are parsed up-front so that
        // `sh:target` blank nodes can be instantiated during shape target parsing.
        self.target_types = self.parse_sparql_target_types()?;

        // Parse each top-level shape in stable (sorted) order. A node with
        // sh:path is a (standalone) PROPERTY shape: its path-scoped constraints
        // are wrapped in a single-property Shape carrying the node's targets.
        let mut node_shapes: Vec<Shape> = Vec::new();
        let mut ids: Vec<Term> = shape_ids.into_iter().collect();
        ids.sort_by_key(Term::to_string);
        for term in ids {
            let shape = if self.first_object_of(&term, sh::PATH).is_some() {
                self.parse_standalone_property_shape(term.clone())?
            } else {
                self.parse_node_shape(term.clone())?
            };
            node_shapes.push(shape);
        }

        let functions = self.parse_sparql_functions()?;

        Ok(Shapes {
            node_shapes,
            box_role_vocab: self.box_role_vocab.clone(),
            functions: Arc::new(functions),
            target_types: self.target_types.clone(),
            shapes_graph: self.shapes_graph.clone(),
            shapes_dataset: Arc::clone(&self.shapes_dataset),
        })
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
    /// SHACL-AF `sh:target`, or the implicit `rdfs:Class` target).
    fn has_own_targets(&self, id: &Term) -> bool {
        for pred in [
            sh::TARGET_CLASS,
            sh::TARGET_SUBJECTS_OF,
            sh::TARGET_OBJECTS_OF,
            sh::TARGET_NODE,
            sh::TARGET,
        ] {
            if self.first_object_of(id, pred).is_some() {
                return true;
            }
        }
        matches!(id, Term::NamedNode(_)) && self.has_type(id, rdfs::CLASS)
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
            message: None,
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
        if !subject.is_subject() {
            return vec![];
        }
        let pred = Term::NamedNode(NamedNode::from(predicate));
        native_quads(
            self.data,
            Some(subject),
            Some(&pred),
            None,
            GraphFilter::AnyGraph,
        )
        .into_iter()
        .map(|(_, _, object)| object)
        .collect()
    }

    /// Return the first object for `(subject, predicate, ?)`, if any.
    fn first_object_of(&self, subject: &Term, predicate: &str) -> Option<Term> {
        self.objects_of(subject, predicate).into_iter().next()
    }

    /// Collect deterministic graph-box role annotations from a shape node via
    /// the caller-supplied [`BoxRoleVocab`]. With no vocab configured the
    /// box-role feature is inactive and this returns an empty list.
    fn box_roles_of(&self, subject: &Term) -> Vec<NamedNode> {
        let Some(vocab) = &self.box_role_vocab else {
            return vec![];
        };
        let mut roles: Vec<NamedNode> = self
            .objects_of(subject, &vocab.graph_box_role)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        roles.sort_unstable();
        roles.dedup();
        roles
    }

    /// Build the SPARQL `PREFIX` header prepended to a SHACL-AF `sh:select` query.
    ///
    /// oxigraph's SPARQL parser has no SHACL awareness, so prefixed names in a
    /// query must be declared in the query text. Two sources contribute, lowest
    /// precedence first:
    ///
    /// 1. The shapes **document's** `@prefix` declarations (the pySHACL-compatible
    ///    fallback — real shapes rely on these without `sh:prefixes`).
    /// 2. SHACL-AF `sh:prefixes` → `sh:declare` on the shape and/or the
    ///    `sh:sparql` / `sh:SPARQLTarget` node (spec §5.2.1), which **override**
    ///    the document fallback for any colliding prefix.
    ///
    /// Output is one `PREFIX p: <ns>` line per prefix, sorted (a `BTreeMap` keeps
    /// it deterministic and one-entry-per-prefix). Empty when nothing is declared.
    fn prefix_header(&self, owners: &[&Term]) -> String {
        build_prefix_header(self.data, &self.doc_prefixes, owners)
    }

    /// Parse a top-level node shape.
    fn parse_node_shape(&mut self, id: Term) -> Result<Shape, String> {
        let id_str = id.to_string();

        // Guard against recursive `sh:node` cycles
        if self.in_flight.contains(&id_str) {
            // Return a minimal stand-in to break the cycle; cyclic shapes are
            // unusual but not forbidden.
            return Ok(Shape {
                id,
                targets: vec![],
                constraints: vec![],
                property_shapes: vec![],
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
                rules: vec![],
            });
        }
        self.in_flight.insert(id_str.clone());
        let result = self.parse_shape_inner(&id);
        self.in_flight.remove(&id_str);
        result
    }

    /// Inner parse logic shared between top-level and anonymous/inline shapes.
    fn parse_shape_inner(&mut self, id: &Term) -> Result<Shape, String> {
        // -- Severity --
        let severity = self
            .first_object_of(id, sh::SEVERITY)
            .and_then(|t| severity_from_term(&t))
            .unwrap_or(Severity::Violation);

        // -- Message (take first by stable sort of string representation) --
        let mut messages: Vec<String> = self
            .objects_of(id, sh::MESSAGE)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .collect();
        messages.sort();
        let message = messages.into_iter().next();

        // -- Deactivated --
        let deactivated = self
            .first_object_of(id, sh::DEACTIVATED)
            .is_some_and(|t| match &t {
                Term::Literal(lit) => lit.value() == "true",
                _ => false,
            });

        // -- Targets (only for top-level node shapes; anonymous shapes have none) --
        let targets = self.parse_targets(id)?;

        // -- Property shapes (via sh:property) --
        let mut property_shape_nodes: Vec<Term> = self.objects_of(id, sh::PROPERTY);
        property_shape_nodes.sort_by_key(ToString::to_string);
        let mut property_shapes = Vec::new();
        for ps_node in property_shape_nodes {
            property_shapes.push(self.parse_property_shape(&ps_node)?);
        }

        // -- Node-level constraints --
        let constraints = self.parse_constraints(id, false)?;
        let box_roles = self.box_roles_of(id);
        // -- SHACL-AF rules (sh:rule) --
        let rules = self.parse_rules(id)?;

        Ok(Shape {
            id: id.clone(),
            targets,
            constraints,
            property_shapes,
            severity,
            message,
            deactivated,
            box_roles,
            rules,
        })
    }

    /// Parse all target declarations for a shape node.
    fn parse_targets(&self, id: &Term) -> Result<Vec<Target>, String> {
        let mut targets: Vec<Target> = Vec::new();

        // sh:targetClass
        let mut tc: Vec<NamedNode> = self
            .objects_of(id, sh::TARGET_CLASS)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        tc.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in tc {
            targets.push(Target::Class(n));
        }

        // sh:targetSubjectsOf
        let mut tso: Vec<NamedNode> = self
            .objects_of(id, sh::TARGET_SUBJECTS_OF)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        tso.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in tso {
            targets.push(Target::SubjectsOf(n));
        }

        // sh:targetObjectsOf
        let mut too: Vec<NamedNode> = self
            .objects_of(id, sh::TARGET_OBJECTS_OF)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        too.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in too {
            targets.push(Target::ObjectsOf(n));
        }

        // sh:targetNode
        let mut tn: Vec<Term> = self.objects_of(id, sh::TARGET_NODE);
        tn.sort_by_key(ToString::to_string);
        for t in tn {
            targets.push(Target::Node(t));
        }

        // Implicit class target: shape node is itself typed rdfs:Class
        if let Term::NamedNode(_) = id
            && self.has_type(id, rdfs::CLASS)
        {
            targets.push(Target::ImplicitClass(id.clone()));
        }

        // sh:target — SHACL-AF extension targets. Supports plain sh:SPARQLTarget
        // and parameterized sh:SPARQLTargetType instances.
        let mut sparql_targets: Vec<Term> = self.objects_of(id, sh::TARGET);
        sparql_targets.sort_by_key(Term::to_string);
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
                let select = format!("{}{raw_select}", self.prefix_header(&[id, &t_node]));

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
            let mut matched: Option<(NamedNode, SparqlTargetType)> = None;
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

            // Collect parameter bindings from the target instance.
            let mut substitutions: Vec<(String, Term)> = Vec::new();
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
                    return Err(format!(
                        "sh:target instance of <{type_iri}> on shape {id} is missing required parameter <{pred}>",
                        pred = param.predicate.as_str()
                    ));
                };
                substitutions.push((param.var.clone(), value));
            }

            // Build the query with prefixes from the shape, the target instance,
            // and the target-type declaration itself.
            let select = format!(
                "{}{}",
                self.prefix_header(&[id, &t_node, &target_type.id]),
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

        // severity
        let severity = self
            .first_object_of(ps_node, sh::SEVERITY)
            .and_then(|t| severity_from_term(&t))
            .unwrap_or(Severity::Violation);

        // message
        let mut messages: Vec<String> = self
            .objects_of(ps_node, sh::MESSAGE)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .collect();
        messages.sort();
        let message = messages.into_iter().next();

        // sh:deactivated — a deactivated property shape validates nothing.
        let deactivated = self
            .first_object_of(ps_node, sh::DEACTIVATED)
            .is_some_and(|t| matches!(&t, Term::Literal(lit) if lit.value() == "true"));

        // constraints on the property shape
        let constraints = self.parse_constraints(ps_node, true)?;

        // Nested sh:property on a property shape (spec §2.1: sh:property may
        // appear on ANY shape) — each nested property shape applies to THIS
        // shape's value nodes. `in_flight` breaks sh:property cycles by
        // skipping a shape already being parsed (mirrors parse_node_shape's
        // cycle stub).
        let mut nested_nodes: Vec<Term> = self.objects_of(ps_node, sh::PROPERTY);
        nested_nodes.sort_by_key(ToString::to_string);
        let mut property_shapes: Vec<PropertyShape> = Vec::new();
        if !nested_nodes.is_empty() {
            self.in_flight.insert(ps_str.clone());
            for nested in nested_nodes {
                if self.in_flight.contains(&nested.to_string()) {
                    continue;
                }
                match self.parse_property_shape(&nested) {
                    Ok(parsed) => property_shapes.push(parsed),
                    Err(e) => {
                        self.in_flight.remove(&ps_str);
                        return Err(e);
                    }
                }
            }
            self.in_flight.remove(&ps_str);
        }

        let box_roles = self.box_roles_of(ps_node);
        let reification_required = self
            .objects_of(ps_node, sh::REIFICATION_REQUIRED)
            .into_iter()
            .any(|t| matches!(t, Term::Literal(lit) if lit.value() == "true"));

        let mut reifier_shape_nodes: Vec<Term> = self.objects_of(ps_node, sh::REIFIER_SHAPE);
        reifier_shape_nodes.sort_by_key(ToString::to_string);
        if (!reifier_shape_nodes.is_empty() || reification_required)
            && !matches!(path, Path::Predicate(_))
        {
            return Err(format!(
                "sh:reifierShape or sh:reificationRequired on property shape {ps_str} requires an IRI sh:path"
            ));
        }
        let mut reifier_shapes = Vec::new();
        for node in reifier_shape_nodes {
            reifier_shapes.push(self.parse_node_shape(node)?);
        }

        Ok(PropertyShape {
            path,
            constraints,
            property_shapes,
            reifier_shapes,
            reification_required,
            severity,
            message,
            deactivated,
            box_roles,
        })
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

        // sh:inversePath
        if let Some(inner) = self.first_object_of(path_node, sh::INVERSE_PATH) {
            let inner_path = self.parse_path(&inner, shape_id, in_flight)?;
            return Ok(Path::Inverse(Box::new(inner_path)));
        }

        // sh:alternativePath — an RDF list of at least two alternatives.
        if let Some(list_head) = self.first_object_of(path_node, sh::ALTERNATIVE_PATH) {
            let items = self.walk_rdf_list(&list_head, shape_id)?;
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
            return Ok(Path::Alternative(parts));
        }

        // sh:zeroOrMorePath / sh:oneOrMorePath / sh:zeroOrOnePath
        for (pred, make) in [
            (
                sh::ZERO_OR_MORE_PATH,
                Path::ZeroOrMore as fn(Box<Path>) -> Path,
            ),
            (sh::ONE_OR_MORE_PATH, Path::OneOrMore as fn(_) -> _),
            (sh::ZERO_OR_ONE_PATH, Path::ZeroOrOne as fn(_) -> _),
        ] {
            if let Some(inner) = self.first_object_of(path_node, pred) {
                let inner_path = self.parse_path(&inner, shape_id, in_flight)?;
                return Ok(make(Box::new(inner_path)));
            }
        }

        Err(format!(
            "unrecognised sh:path blank node structure on shape {shape_id}"
        ))
    }

    /// Walk an RDF list (`rdf:first`/`rdf:rest`/`rdf:nil`) and collect items.
    fn walk_rdf_list(&self, head: &Term, shape_id: &Term) -> Result<Vec<Term>, String> {
        let nil = Term::NamedNode(NamedNode::from(rdf::NIL));
        let mut items = Vec::new();
        let mut current = head.clone();
        let mut seen: FastSet<String> = FastSet::default();

        loop {
            if current == nil {
                break;
            }
            let key = current.to_string();
            if seen.contains(&key) {
                return Err(format!("cyclic RDF list on shape {shape_id}"));
            }
            seen.insert(key);

            if let Some(first) = self.first_object_of(&current, rdf::FIRST) {
                items.push(first);
            }
            match self.first_object_of(&current, rdf::REST) {
                Some(rest) => current = rest,
                None => break,
            }
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
        if self.in_flight.contains(&id_str) {
            return Ok(Shape {
                id,
                targets: vec![],
                constraints: vec![],
                property_shapes: vec![],
                severity: Severity::Violation,
                message: None,
                deactivated: false,
                box_roles: vec![],
                rules: vec![],
            });
        }

        if self.first_object_of(&id, sh::PATH).is_some() {
            // Treat as an inline property shape
            self.in_flight.insert(id_str.clone());
            let ps = self.parse_property_shape(&id);
            self.in_flight.remove(&id_str);
            let ps = ps?;
            Ok(Shape {
                id,
                targets: vec![],
                constraints: vec![],
                property_shapes: vec![ps],
                severity: Severity::Violation,
                message: None,
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

/// The local name of an IRI: the substring after the last `#` or `/`. Used to
/// derive a `sh:SPARQLFunction` parameter's pre-bound SPARQL variable name from its
/// predicate IRI (SHACL-AF §5.1).
pub(crate) fn local_name(iri: &str) -> &str {
    let cut = iri.rfind(['#', '/']).map_or(0, |i| i + 1);
    &iri[cut..]
}

/// Parse a typed integer literal or plain literal integer into a `u64`.
pub(crate) fn parse_u64(term: &Term) -> Option<u64> {
    if let Term::Literal(lit) = term {
        lit.value().parse::<u64>().ok()
    } else {
        None
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
        crate::text_ingest::parse_turtle_to_dataset(ttl).expect("Turtle parse error")
    }

    /// Parse shapes from a test dataset (shim over [`from_dataset`]).
    fn from_store(dataset: &Arc<RdfDataset>) -> Result<Shapes, String> {
        from_dataset(dataset)
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

        let shapes = crate::engine::parse_shapes(&ttl)
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
        assert!(
            all.iter()
                .any(|c| matches!(c, Constraint::LessThan(n) if n.as_str().ends_with("end")))
        );
        assert!(
            all.iter().any(
                |c| matches!(c, Constraint::LessThanOrEquals(n) if n.as_str().ends_with("last"))
            )
        );
        assert!(
            all.iter()
                .any(|c| matches!(c, Constraint::Equals(n) if n.as_str().ends_with('b')))
        );
        assert!(
            all.iter()
                .any(|c| matches!(c, Constraint::Disjoint(n) if n.as_str().ends_with('d')))
        );
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
        assert_eq!(shape.message.as_deref(), Some("This is a warning"));
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
            .any(|c| matches!(c, Constraint::NodeKind(NodeKindValue::Iri)));
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
                .any(|c| matches!(c, Constraint::NodeKind(NodeKindValue::Literal))),
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
            Constraint::Closed { ignored } => Some(ignored),
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
        // shapes graph must HARD-fail to load rather than silently dropping it
        // (Gap H).
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
        let mut parser = Parser::new(dataset.as_ref(), &[], None, Arc::clone(&dataset), None);
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
                assert!(matches!(*then, NodeExpr::Union(ref v) if v.is_empty()));
                assert!(matches!(*els, NodeExpr::Union(ref v) if v.is_empty()));
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
                        .any(|c| matches!(c, Constraint::NodeKind(NodeKindValue::Iri)))
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
