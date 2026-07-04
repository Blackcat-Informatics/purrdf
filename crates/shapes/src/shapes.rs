// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL shapes graph parser.
//!
//! Parses a SHACL Core shapes graph (loaded into an oxigraph [`Store`]) into a
//! fully typed [`Shapes`] structure.  No evaluation logic lives here — that is
//! Task 3.  Covers full SHACL Core: all six property-path forms (§2.3.1),
//! property-pair constraints (§4.3), qualified value shapes (§4.5.4–4.5.5), and
//! SHACL-AF SPARQL constraints/targets.  Malformed constructs (e.g. a literal
//! `sh:equals` object, a one-member sequence path) cause a hard `Err` rather
//! than a silent skip.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use ::purrdf::RdfDataset;

use crate::components::{severity_from_term, ComponentRegistry, Validator, ValidatorKind};
use crate::data::{GraphFilter, IrDataGraph, ShaclDataGraph};
use crate::expression::{FnCall, NodeExpr};
use crate::model::{rdf, rdfs, sh, BoxRoleVocab};
use crate::report::Severity;
use crate::term::{NamedNode, Term};

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
    ///
    /// The query is validated (parseable + SELECT-form) at shape-load time. The
    /// native SPARQL engine re-parses the text at eval time, so only the query
    /// string is retained.
    Sparql {
        /// The SPARQL SELECT query text (with any injected PREFIX header).
        select: String,
    },
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
/// [`Shapes::shapes_dataset`] so the validation engine can expose it as a
/// named graph to SHACL-SPARQL queries.
pub fn from_dataset_with_config_and_graph(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    box_role_vocab: Option<BoxRoleVocab>,
    shapes_graph: Option<String>,
) -> Result<Shapes, String> {
    let data = IrDataGraph::new(Arc::clone(dataset));
    let mut parser = Parser::new(
        &data,
        doc_prefixes,
        box_role_vocab,
        Arc::clone(dataset),
        shapes_graph,
    );
    parser.parse()
}

// ── Internal parser ────────────────────────────────────────────────────────────

struct Parser<'s> {
    data: &'s IrDataGraph,
    /// Tracks shape nodes currently being parsed to prevent infinite recursion
    /// through `sh:node` or `sh:and/or/xone` cycles.
    in_flight: HashSet<String>,
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
}

// ── Prefix-header helper (used by shapes and component registry) ───────────────

/// Return all objects for `(subject, predicate, ?)`.
fn objects_of(data: &IrDataGraph, subject: &Term, predicate: &str) -> Vec<Term> {
    if !subject.is_subject() {
        return vec![];
    }
    let pred = Term::NamedNode(NamedNode::from(predicate));
    data.quads_for_pattern(Some(subject), Some(&pred), None, GraphFilter::AnyGraph)
        .into_iter()
        .map(|q| q.object)
        .collect()
}

/// Return the first object for `(subject, predicate, ?)`, if any.
fn first_object_of(data: &IrDataGraph, subject: &Term, predicate: &str) -> Option<Term> {
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
    data: &IrDataGraph,
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
        data: &'s IrDataGraph,
        doc_prefixes: &[(String, String)],
        box_role_vocab: Option<BoxRoleVocab>,
        shapes_dataset: Arc<RdfDataset>,
        shapes_graph: Option<String>,
    ) -> Self {
        Self {
            data,
            in_flight: HashSet::new(),
            doc_prefixes: doc_prefixes.to_vec(),
            box_role_vocab,
            component_registry: ComponentRegistry::default(),
            shapes_dataset,
            shapes_graph,
        }
    }

    fn parse(&mut self) -> Result<Shapes, String> {
        // --- collect all top-level shape node terms ---
        let mut shape_ids: HashSet<Term> = HashSet::new();
        // Track which nodes are property-shape-only (reachable only via sh:property)
        // so we don't list them as top-level node shapes.
        let mut property_shape_nodes: HashSet<Term> = HashSet::new();

        // 1. Nodes typed sh:NodeShape
        for quad in self.quads_with(None, Some(rdf::TYPE), Some(sh::NODE_SHAPE)) {
            shape_ids.insert(quad.subject);
        }

        // 2. Nodes typed sh:PropertyShape (collect to exclude from top-level)
        for quad in self.quads_with(None, Some(rdf::TYPE), Some(sh::PROPERTY_SHAPE)) {
            property_shape_nodes.insert(quad.subject);
        }

        // 3. Subjects of sh:targetClass / sh:targetSubjectsOf / sh:targetObjectsOf / sh:targetNode
        for pred in [
            sh::TARGET_CLASS,
            sh::TARGET_SUBJECTS_OF,
            sh::TARGET_OBJECTS_OF,
            sh::TARGET_NODE,
        ] {
            for quad in self.quads_with(None, Some(pred), None) {
                shape_ids.insert(quad.subject);
            }
        }

        // 4. Nodes that are sh:property owners with shape constraints (implicit shapes)
        //    and nodes that are rdfs:Class AND carry sh:targetClass or sh:NodeShape type
        //    (already caught above).  We also add any node that has sh:property
        //    and is thus acting as a shape container.
        for quad in self.quads_with(None, Some(sh::PROPERTY), None) {
            // Only add if not exclusively a property shape itself
            if !property_shape_nodes.contains(&quad.subject) {
                shape_ids.insert(quad.subject);
            }
        }

        // 5. Nodes that carry sh:property as objects → record as property-shape-only
        for quad in self.quads_with(None, Some(sh::PROPERTY), None) {
            property_shape_nodes.insert(quad.object);
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

        Ok(Shapes {
            node_shapes,
            box_role_vocab: self.box_role_vocab.clone(),
            shapes_graph: self.shapes_graph.clone(),
            shapes_dataset: Arc::clone(&self.shapes_dataset),
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
        })
    }

    /// Pattern-query the shapes dataset over ALL graphs. `subject`/`object` are IRI
    /// constants or `None` wildcards; `predicate` is an IRI constant or `None`.
    fn quads_with(
        &self,
        subject: Option<&str>,
        predicate: Option<&str>,
        object: Option<&str>,
    ) -> Vec<crate::data::Quad> {
        let s = subject.map(|iri| Term::NamedNode(NamedNode::from(iri)));
        let p = predicate.map(|iri| Term::NamedNode(NamedNode::from(iri)));
        let o = object.map(|iri| Term::NamedNode(NamedNode::from(iri)));
        self.data
            .quads_for_pattern(s.as_ref(), p.as_ref(), o.as_ref(), GraphFilter::AnyGraph)
    }

    /// Whether `(subject, rdf:type, class_iri)` is asserted in any graph.
    fn has_type(&self, subject: &Term, class_iri: &str) -> bool {
        if !subject.is_subject() {
            return false;
        }
        let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
        let class = Term::NamedNode(NamedNode::from(class_iri));
        !self
            .data
            .quads_for_pattern(
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
        self.data
            .quads_for_pattern(Some(subject), Some(&pred), None, GraphFilter::AnyGraph)
            .into_iter()
            .map(|q| q.object)
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

        Ok(Shape {
            id: id.clone(),
            targets,
            constraints,
            property_shapes,
            severity,
            message,
            deactivated,
            box_roles,
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
        if let Term::NamedNode(_) = id {
            if self.has_type(id, rdfs::CLASS) {
                targets.push(Target::ImplicitClass(id.clone()));
            }
        }

        // sh:target — SHACL-AF extension targets.  Only sh:SPARQLTarget is
        // supported; any other rdf:type on the target blank node is a hard error.
        let mut sparql_targets: Vec<Term> = self.objects_of(id, sh::TARGET);
        sparql_targets.sort_by_key(Term::to_string);
        for t_node in sparql_targets {
            // Require rdf:type sh:SPARQLTarget on the target blank node.
            if !self.has_type(&t_node, sh::SPARQL_TARGET) {
                return Err(format!(
                    "unsupported sh:target type on shape {id}: target node {t_node} \
                     is not typed sh:SPARQLTarget"
                ));
            }

            // sh:select is required on the SPARQLTarget blank node.
            let raw_select = self
                .first_object_of(&t_node, sh::SELECT)
                .and_then(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!("sh:SPARQLTarget on shape {id} is missing a sh:select string literal")
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

            targets.push(Target::Sparql { select });
        }

        Ok(targets)
    }

    /// Parse all constraints declared directly on a shape node.
    ///
    /// Does NOT include `sh:property` sub-shapes (handled separately).
    /// `is_property_shape` selects the right custom-component validator
    /// (`sh:propertyValidator` vs `sh:nodeValidator`) and is passed down from
    /// both node shapes and property shapes.
    fn parse_constraints(
        &mut self,
        id: &Term,
        is_property_shape: bool,
    ) -> Result<Vec<Constraint>, String> {
        let mut constraints: Vec<Constraint> = Vec::new();

        // sh:class — sorted for determinism
        let mut classes: Vec<NamedNode> = self
            .objects_of(id, sh::CLASS)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        classes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in classes {
            constraints.push(Constraint::Class(n));
        }

        // sh:datatype
        let mut datatypes: Vec<NamedNode> = self
            .objects_of(id, sh::DATATYPE)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        datatypes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in datatypes {
            constraints.push(Constraint::Datatype(n));
        }

        // sh:nodeKind
        for t in self.objects_of(id, sh::NODE_KIND) {
            if let Term::NamedNode(n) = &t {
                let nk = parse_node_kind(n.as_str())
                    .ok_or_else(|| format!("unknown sh:nodeKind value <{}> on {id}", n.as_str()))?;
                constraints.push(Constraint::NodeKind(nk));
            }
        }

        // sh:minCount
        for t in self.objects_of(id, sh::MIN_COUNT) {
            let v = parse_u64(&t).ok_or_else(|| {
                format!("sh:minCount value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MinCount(v));
        }

        // sh:maxCount
        for t in self.objects_of(id, sh::MAX_COUNT) {
            let v = parse_u64(&t).ok_or_else(|| {
                format!("sh:maxCount value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MaxCount(v));
        }

        // sh:minLength
        for t in self.objects_of(id, sh::MIN_LENGTH) {
            let v = parse_u64(&t).ok_or_else(|| {
                format!("sh:minLength value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MinLength(v));
        }

        // sh:maxLength
        for t in self.objects_of(id, sh::MAX_LENGTH) {
            let v = parse_u64(&t).ok_or_else(|| {
                format!("sh:maxLength value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MaxLength(v));
        }

        // sh:languageIn — an RDF list of language-tag string literals
        let mut lang_in_lists: Vec<Term> = self.objects_of(id, sh::LANGUAGE_IN);
        lang_in_lists.sort_by_key(ToString::to_string);
        for list_head in lang_in_lists {
            let items = self.walk_rdf_list(&list_head, id)?;
            let mut tags: Vec<String> = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Term::Literal(lit) => tags.push(lit.value().to_owned()),
                    other => {
                        return Err(format!(
                            "sh:languageIn list on {id} contains a non-literal language tag: {other}"
                        ));
                    }
                }
            }
            constraints.push(Constraint::LanguageIn(tags));
        }

        // sh:not — a single nested shape (mirrors sh:node)
        let mut not_refs: Vec<Term> = self.objects_of(id, sh::NOT);
        not_refs.sort_by_key(ToString::to_string);
        for not_ref in not_refs {
            let inner = self.parse_node_shape(not_ref)?;
            constraints.push(Constraint::Not(Box::new(inner)));
        }

        // sh:closed (+ sh:ignoredProperties) — node-shape-level closed-world check.
        // Only emit the constraint when sh:closed is true.
        let is_closed = self
            .first_object_of(id, sh::CLOSED)
            .is_some_and(|t| match &t {
                Term::Literal(lit) => lit.value() == "true",
                _ => false,
            });
        if is_closed {
            let mut ignored: Vec<NamedNode> = Vec::new();
            let mut ignored_lists: Vec<Term> = self.objects_of(id, sh::IGNORED_PROPERTIES);
            ignored_lists.sort_by_key(ToString::to_string);
            for list_head in ignored_lists {
                for item in self.walk_rdf_list(&list_head, id)? {
                    match item {
                        Term::NamedNode(n) => ignored.push(n),
                        // sh:ignoredProperties members must be IRIs; silently
                        // skipping a non-IRI would let a malformed shapes graph load
                        // and feed bad data downstream (hard-fail, no silent drop).
                        other => {
                            return Err(format!(
                                "sh:ignoredProperties list on {id} contains a non-IRI member: {other}"
                            ));
                        }
                    }
                }
            }
            ignored.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            ignored.dedup();
            constraints.push(Constraint::Closed { ignored });
        }

        // sh:uniqueLang
        for t in self.objects_of(id, sh::UNIQUE_LANG) {
            if let Term::Literal(lit) = &t {
                let flag = lit.value() == "true";
                constraints.push(Constraint::UniqueLang(flag));
            }
        }

        // sh:minInclusive / sh:maxInclusive
        let mut min_inc: Vec<Term> = self.objects_of(id, sh::MIN_INCLUSIVE);
        min_inc.sort_by_key(ToString::to_string);
        for t in min_inc {
            constraints.push(Constraint::MinInclusive(t));
        }

        let mut max_inc: Vec<Term> = self.objects_of(id, sh::MAX_INCLUSIVE);
        max_inc.sort_by_key(ToString::to_string);
        for t in max_inc {
            constraints.push(Constraint::MaxInclusive(t));
        }

        // sh:minExclusive / sh:maxExclusive
        let mut min_exc: Vec<Term> = self.objects_of(id, sh::MIN_EXCLUSIVE);
        min_exc.sort_by_key(ToString::to_string);
        for t in min_exc {
            constraints.push(Constraint::MinExclusive(t));
        }

        let mut max_exc: Vec<Term> = self.objects_of(id, sh::MAX_EXCLUSIVE);
        max_exc.sort_by_key(ToString::to_string);
        for t in max_exc {
            constraints.push(Constraint::MaxExclusive(t));
        }

        // sh:hasValue
        let mut hv: Vec<Term> = self.objects_of(id, sh::HAS_VALUE);
        hv.sort_by_key(ToString::to_string);
        for t in hv {
            constraints.push(Constraint::HasValue(t));
        }

        // sh:in
        let mut in_lists: Vec<Term> = self.objects_of(id, sh::IN);
        in_lists.sort_by_key(ToString::to_string);
        for list_head in in_lists {
            let items = self.walk_rdf_list(&list_head, id)?;
            constraints.push(Constraint::In(items));
        }

        // sh:pattern + optional sh:flags
        let mut patterns: Vec<String> = self
            .objects_of(id, sh::PATTERN)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .collect();
        patterns.sort();
        let flags_val: Option<String> = self
            .objects_of(id, sh::FLAGS)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .min(); // take the lexicographically smallest if multiple
        for regex in patterns {
            constraints.push(Constraint::Pattern {
                regex,
                flags: flags_val.clone(),
                compiled: Arc::new(OnceLock::new()),
            });
        }

        // sh:and / sh:or / sh:xone — each is an RDF list of shape nodes
        let mut and_lists: Vec<Term> = self.objects_of(id, sh::AND);
        and_lists.sort_by_key(ToString::to_string);
        for list_head in and_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::And(members));
        }

        let mut or_lists: Vec<Term> = self.objects_of(id, sh::OR);
        or_lists.sort_by_key(ToString::to_string);
        for list_head in or_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::Or(members));
        }

        let mut xone_lists: Vec<Term> = self.objects_of(id, sh::XONE);
        xone_lists.sort_by_key(ToString::to_string);
        for list_head in xone_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::Xone(members));
        }

        // sh:node
        let mut node_refs: Vec<Term> = self.objects_of(id, sh::NODE);
        node_refs.sort_by_key(ToString::to_string);
        for node_ref in node_refs {
            let inner = self.parse_node_shape(node_ref)?;
            constraints.push(Constraint::Node(Box::new(inner)));
        }

        // sh:sparql — SHACL-AF SPARQL constraint components.
        // The blank node may or may not carry rdf:type sh:SPARQLConstraint;
        // we require only sh:select (which must be a SELECT query).
        let mut sparql_cnodes: Vec<Term> = self.objects_of(id, sh::SPARQL);
        sparql_cnodes.sort_by_key(ToString::to_string);
        for c_node in sparql_cnodes {
            // sh:select is required.
            let raw_select = self
                .first_object_of(&c_node, sh::SELECT)
                .and_then(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!(
                        "sh:sparql constraint on shape {id} is missing a sh:select string literal"
                    )
                })?;
            // SHACL-AF sh:prefixes may be declared on the shape or the sh:sparql node.
            let select = format!("{}{raw_select}", self.prefix_header(&[id, &c_node]));

            // Parse-time query validation via the native parser (hard-fail on
            // unparsable queries). SHACL-SPARQL requires a SELECT; ASK/CONSTRUCT/
            // DESCRIBE parse but cannot bind ?this and would panic at eval — reject
            // at the boundary.
            match purrdf_sparql_algebra::SparqlParser::new().parse_query(&select) {
                Ok(query @ purrdf_sparql_algebra::Query::Select { .. }) => {
                    // The query runs with $this pre-bound to each focus node;
                    // the SHACL-SPARQL §5.2.1 pre-binding restrictions (no
                    // MINUS / SERVICE / VALUES, no `AS $this`, subqueries must
                    // project $this) reject it as a hard failure at load.
                    crate::prebinding::check_select(&query, &["this"])
                        .map_err(|e| format!("sh:sparql constraint on shape {id}: {e}"))?;
                }
                Ok(_) => {
                    return Err(format!(
                        "sh:sparql constraint on shape {id} must be a SELECT query (ASK/CONSTRUCT/DESCRIBE are not valid SHACL-SPARQL)"
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "sh:sparql constraint on shape {id} has an unparsable sh:select query: {e}"
                    ));
                }
            }

            // Optional per-constraint sh:message override.
            let mut messages: Vec<String> = self
                .objects_of(&c_node, sh::MESSAGE)
                .into_iter()
                .filter_map(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .collect();
            messages.sort();
            let message = messages.into_iter().next();

            // Optional per-constraint sh:severity override.
            let severity = self
                .first_object_of(&c_node, sh::SEVERITY)
                .and_then(|t| severity_from_term(&t));

            constraints.push(Constraint::Sparql {
                select,
                message,
                severity,
            });
        }

        // sh:expression — SHACL-AF §5.7 expression constraint component. Each
        // object is a node expression parsed via `parse_node_expr`; the optional
        // sh:message / sh:severity on the expression node override the shape
        // defaults at eval time (mirroring sh:sparql).
        let mut expr_nodes: Vec<Term> = self.objects_of(id, sh::EXPRESSION);
        expr_nodes.sort_by_key(ToString::to_string);
        for expr_node in expr_nodes {
            let expr = self.parse_node_expr(&expr_node)?;

            let mut messages: Vec<String> = self
                .objects_of(&expr_node, sh::MESSAGE)
                .into_iter()
                .filter_map(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .collect();
            messages.sort();
            let message = messages.into_iter().next();

            let severity = self
                .first_object_of(&expr_node, sh::SEVERITY)
                .and_then(|t| severity_from_term(&t));

            constraints.push(Constraint::Expression {
                expr,
                message,
                severity,
            });
        }

        // sh:equals / sh:disjoint / sh:lessThan / sh:lessThanOrEquals — the
        // property-pair constraint components (§4.3). Each object must be an IRI;
        // a non-IRI object is malformed and hard-fails (no silent drop).
        for (pred, make) in [
            (
                sh::EQUALS,
                Constraint::Equals as fn(NamedNode) -> Constraint,
            ),
            (sh::DISJOINT, Constraint::Disjoint as fn(_) -> _),
            (sh::LESS_THAN, Constraint::LessThan as fn(_) -> _),
            (
                sh::LESS_THAN_OR_EQUALS,
                Constraint::LessThanOrEquals as fn(_) -> _,
            ),
        ] {
            let mut props: Vec<NamedNode> = Vec::new();
            for t in self.objects_of(id, pred) {
                match t {
                    Term::NamedNode(n) => props.push(n),
                    other => {
                        return Err(format!(
                            "<{pred}> on shape {id} must be an IRI, got {other}"
                        ));
                    }
                }
            }
            props.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            for n in props {
                constraints.push(make(n));
            }
        }

        // sh:qualifiedValueShape + sh:qualifiedMinCount / sh:qualifiedMaxCount
        // (§4.5.4–4.5.5). The counts require the shape and vice versa — a
        // dangling half of the pair is malformed and hard-fails.
        constraints.extend(self.parse_qualified_value_shapes(id)?);

        // Custom SHACL-SPARQL constraint components. A shape that carries values
        // for all required parameters of a declared component is treated as a
        // usage of that component. Components are processed in deterministic
        // order; parameter bindings follow the component's declared parameter
        // order. All validators applicable to the current shape scope are
        // emitted as separate constraints; if none apply, the component is
        // skipped silently.
        let shape_severity = self
            .first_object_of(id, sh::SEVERITY)
            .and_then(|t| severity_from_term(&t));
        let mut shape_messages: Vec<String> = self
            .objects_of(id, sh::MESSAGE)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .collect();
        shape_messages.sort();
        let shape_message = shape_messages.into_iter().next();

        let mut components: Vec<&crate::components::Component> =
            self.component_registry.components.values().collect();
        components.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        for component in components {
            let mut bindings: Vec<(String, Term)> = Vec::new();
            let mut missing_required = false;
            for param in &component.parameters {
                let values = self.objects_of(id, param.path.as_str());
                if values.len() > 1 {
                    return Err(format!(
                        "shape {id} declares {count} values for parameter <{path}> of component <{component}>, only one is allowed",
                        count = values.len(),
                        path = param.path,
                        component = component.id
                    ));
                }
                if let Some(value) = values.into_iter().next() {
                    bindings.push((param.name.clone(), value));
                } else if !param.optional {
                    missing_required = true;
                    break;
                }
            }
            if missing_required {
                continue;
            }

            let matching: Vec<&Validator> = if is_property_shape {
                component
                    .property_validators
                    .iter()
                    .chain(component.validators.iter())
                    .collect()
            } else {
                component
                    .node_validators
                    .iter()
                    .chain(component.validators.iter())
                    .collect()
            };
            if matching.is_empty() {
                continue;
            }

            for validator in matching {
                let component_validator = match &validator.kind {
                    ValidatorKind::Ask => ComponentValidator::Ask {
                        ask: validator.query_text.clone(),
                    },
                    ValidatorKind::Select => ComponentValidator::Select {
                        select: validator.query_text.clone(),
                    },
                };

                let severity = shape_severity
                    .clone()
                    .or_else(|| validator.severity.clone())
                    .or_else(|| component.severity.clone());
                let message = shape_message
                    .clone()
                    .or_else(|| validator.message.clone())
                    .or_else(|| component.message.clone());

                constraints.push(Constraint::Component {
                    component: component.id.clone(),
                    source_shape: id.clone(),
                    bindings: bindings.clone(),
                    validator: component_validator,
                    message,
                    severity,
                });
            }
        }

        Ok(constraints)
    }

    /// Parse the qualified-value-shape constraint(s) declared on `id`.
    ///
    /// Returns one [`Constraint::QualifiedValueShape`] per `sh:qualifiedValueShape`
    /// object (sorted for determinism). The declared `sh:qualifiedMinCount` /
    /// `sh:qualifiedMaxCount` apply to each. When
    /// `sh:qualifiedValueShapesDisjoint true` is set, the sibling qualified value
    /// shapes (§4.5.5: the values of `sh:property/sh:qualifiedValueShape` on the
    /// parents of `id`, minus the constraint's own shape) are parsed and stored.
    fn parse_qualified_value_shapes(&mut self, id: &Term) -> Result<Vec<Constraint>, String> {
        let mut qvs_nodes: Vec<Term> = self.objects_of(id, sh::QUALIFIED_VALUE_SHAPE);
        qvs_nodes.sort_by_key(ToString::to_string);

        let min_count = match self.first_object_of(id, sh::QUALIFIED_MIN_COUNT) {
            Some(t) => Some(parse_u64(&t).ok_or_else(|| {
                format!("sh:qualifiedMinCount value is not a non-negative integer on {id}")
            })?),
            None => None,
        };
        let max_count = match self.first_object_of(id, sh::QUALIFIED_MAX_COUNT) {
            Some(t) => Some(parse_u64(&t).ok_or_else(|| {
                format!("sh:qualifiedMaxCount value is not a non-negative integer on {id}")
            })?),
            None => None,
        };

        if qvs_nodes.is_empty() {
            // sh:qualifiedMinCount / sh:qualifiedMaxCount without an
            // sh:qualifiedValueShape leaves the constraint component INACTIVE
            // (its mandatory parameter is absent — W3C core/node/qualified-001
            // expects the dangling counts to be ignored, not a hard failure).
            return Ok(vec![]);
        }
        if min_count.is_none() && max_count.is_none() {
            return Err(format!(
                "sh:qualifiedValueShape on {id} requires sh:qualifiedMinCount or \
                 sh:qualifiedMaxCount"
            ));
        }

        let disjoint = self
            .first_object_of(id, sh::QUALIFIED_VALUE_SHAPES_DISJOINT)
            .is_some_and(|t| matches!(&t, Term::Literal(lit) if lit.value() == "true"));

        let mut out = Vec::with_capacity(qvs_nodes.len());
        for qvs_node in &qvs_nodes {
            let shape = self.parse_inline_shape(qvs_node.clone())?;
            let siblings = if disjoint {
                self.parse_sibling_qualified_shapes(id, qvs_node)?
            } else {
                vec![]
            };
            out.push(Constraint::QualifiedValueShape {
                shape: Box::new(shape),
                siblings,
                min_count,
                max_count,
                disjoint,
            });
        }
        Ok(out)
    }

    /// Collect and parse the sibling qualified value shapes of `own_qvs` (§4.5.5):
    /// all values of `sh:property/sh:qualifiedValueShape` reachable from the
    /// parents of the property shape `ps_id`, minus `own_qvs` itself.
    fn parse_sibling_qualified_shapes(
        &mut self,
        ps_id: &Term,
        own_qvs: &Term,
    ) -> Result<Vec<Shape>, String> {
        let property = Term::NamedNode(NamedNode::from(sh::PROPERTY));
        let mut sibling_nodes: Vec<Term> = Vec::new();
        let mut seen: HashSet<Term> = HashSet::new();
        // Parents: subjects of (?, sh:property, ps_id).
        let mut parents: Vec<Term> = self
            .data
            .quads_for_pattern(None, Some(&property), Some(ps_id), GraphFilter::AnyGraph)
            .into_iter()
            .map(|q| q.subject)
            .collect();
        parents.sort_by_key(Term::to_string);
        parents.dedup();
        for parent in &parents {
            let mut sibling_ps: Vec<Term> = self.objects_of(parent, sh::PROPERTY);
            sibling_ps.sort_by_key(Term::to_string);
            for ps in sibling_ps {
                let mut qvs: Vec<Term> = self.objects_of(&ps, sh::QUALIFIED_VALUE_SHAPE);
                qvs.sort_by_key(Term::to_string);
                for q in qvs {
                    if &q != own_qvs && seen.insert(q.clone()) {
                        sibling_nodes.push(q);
                    }
                }
            }
        }
        let mut siblings = Vec::with_capacity(sibling_nodes.len());
        for node in sibling_nodes {
            siblings.push(self.parse_inline_shape(node)?);
        }
        Ok(siblings)
    }

    /// Parse a property shape node.
    fn parse_property_shape(&mut self, ps_node: &Term) -> Result<PropertyShape, String> {
        let ps_str = ps_node.to_string();

        // sh:path is required
        let path_node = self
            .first_object_of(ps_node, sh::PATH)
            .ok_or_else(|| format!("property shape {ps_str} missing sh:path"))?;

        let path = self.parse_path(&path_node, ps_node, &mut HashSet::new())?;

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
        in_flight: &mut HashSet<String>,
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
        in_flight: &mut HashSet<String>,
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
        let mut seen: HashSet<String> = HashSet::new();

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
            })
        } else {
            self.parse_node_shape(id)
        }
    }

    // ── SHACL-AF node expressions (spec §5) ─────────────────────────────────────

    /// Parse a shapes-graph node into a SHACL-AF [`NodeExpr`] (spec §5).
    ///
    /// Paging/ordering wrappers (`sh:limit` / `sh:offset` / `sh:orderby`) are
    /// peeled first and applied on top of the node's *core* expression in SPARQL
    /// pipeline order (`ORDER BY` → `OFFSET` → `LIMIT`, with `LIMIT` outermost);
    /// everything else dispatches through [`parse_node_expr_core`](Self::parse_node_expr_core).
    ///
    /// A blank node is guarded against cyclic self-reference (mirroring
    /// [`parse_inline_shape`](Self::parse_inline_shape)); the guard key is
    /// namespaced so it never collides with the shape-parsing `in_flight` set.
    fn parse_node_expr(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // NOTE: paging/ordering surface (`sh:limit`/`sh:offset`/`sh:orderby`) is
        // under-specified by SHACL-AF. Assumption pinned here (a later corpus
        // task validates it): these keys WRAP the same node's core expression —
        // the inner operand is this very node parsed with the paging keys
        // ignored, NOT a separate `sh:nodes` operand. A node carrying only paging
        // keys (no core expression) therefore hard-fails in `parse_node_expr_core`.
        let guard_key = format!("nodeexpr:{node}");
        let is_blank = matches!(node, Term::BlankNode(_));
        if is_blank {
            if self.in_flight.contains(&guard_key) {
                return Err(format!("cyclic node expression on {node}"));
            }
            self.in_flight.insert(guard_key.clone());
        }
        let result = self.parse_node_expr_wrapped(node);
        if is_blank {
            self.in_flight.remove(&guard_key);
        }
        result
    }

    /// Apply the paging/ordering wrappers on top of the core expression.
    fn parse_node_expr_wrapped(&mut self, node: &Term) -> Result<NodeExpr, String> {
        let mut expr = self.parse_node_expr_core(node)?;

        // ORDER BY (innermost wrapper). `sh:orderby` names the sort-key node
        // expression (evaluated element-as-focus); direction is the separate
        // `sh:desc` boolean flag (default ascending).
        if let Some(key_node) = self.first_object_of(node, sh::ORDERBY) {
            let key = self.parse_node_expr(&key_node)?;
            let descending = match self.first_object_of(node, sh::DESC) {
                None => false,
                Some(term @ Term::Literal(_)) => {
                    let Term::Literal(lit) = &term else {
                        unreachable!()
                    };
                    match purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()) {
                        Ok(Some(purrdf_xsd::XsdValue::Boolean(b))) => b,
                        _ => {
                            return Err(format!(
                                "sh:desc must be an xsd:boolean literal, got {term}"
                            ));
                        }
                    }
                }
                Some(other) => {
                    return Err(format!(
                        "sh:desc must be an xsd:boolean literal, got {other}"
                    ));
                }
            };
            expr = NodeExpr::OrderBy {
                of: Box::new(expr),
                key: Box::new(key),
                descending,
            };
        }

        // OFFSET.
        if let Some(off) = self.first_object_of(node, sh::OFFSET) {
            let n = parse_u64(&off).ok_or_else(|| {
                format!("sh:offset value is not a non-negative integer on {node}")
            })?;
            expr = NodeExpr::Offset {
                of: Box::new(expr),
                n,
            };
        }

        // LIMIT (outermost wrapper).
        if let Some(lim) = self.first_object_of(node, sh::LIMIT) {
            let n = parse_u64(&lim)
                .ok_or_else(|| format!("sh:limit value is not a non-negative integer on {node}"))?;
            expr = NodeExpr::Limit {
                of: Box::new(expr),
                n,
            };
        }

        Ok(expr)
    }

    /// Parse the non-paging *core* of a node expression.
    ///
    /// Dispatches on the single structural SHACL-AF key the node carries, in a
    /// fixed deterministic order, and hard-fails when a node carries two
    /// mutually-exclusive expression keys (ambiguous).
    fn parse_node_expr_core(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // Literals are always constant term expressions (they bear no triples).
        if matches!(node, Term::Literal(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // The focus-node expression `sh:this`.
        if let Term::NamedNode(n) = node {
            if n.as_str() == sh::THIS {
                return Ok(NodeExpr::This);
            }
        }

        // Which mutually-exclusive structural key does the node carry?
        let primary = [
            sh::PATH,
            sh::FILTER_SHAPE,
            sh::UNION,
            sh::INTERSECTION,
            sh::IF,
            sh::COUNT,
            sh::DISTINCT,
            sh::MIN,
            sh::MAX,
            sh::SUM,
            sh::EXISTS,
        ];
        let present: Vec<&str> = primary
            .into_iter()
            .filter(|&p| self.first_object_of(node, p).is_some())
            .collect();
        if present.len() > 1 {
            return Err(format!(
                "ambiguous node expression on {node}: multiple expression keys {present:?}"
            ));
        }

        if let Some(&key) = present.first() {
            return self.parse_structural_node_expr(node, key);
        }

        // No structural key: a function call, a plain constant IRI, or malformed.
        self.parse_call_or_constant(node)
    }

    /// Dispatch a node carrying exactly one structural expression `key`.
    fn parse_structural_node_expr(&mut self, node: &Term, key: &str) -> Result<NodeExpr, String> {
        match key {
            sh::PATH => {
                let path_node = self
                    .first_object_of(node, sh::PATH)
                    .ok_or_else(|| format!("sh:path node expression on {node} lost its object"))?;
                let path = self.parse_path(&path_node, node, &mut HashSet::new())?;
                Ok(NodeExpr::Path(path))
            }
            sh::FILTER_SHAPE => {
                let shape_ref = self
                    .first_object_of(node, sh::FILTER_SHAPE)
                    .ok_or_else(|| {
                        format!("sh:filterShape node expression on {node} lost its object")
                    })?;
                let nodes_obj = self.first_object_of(node, sh::NODES).ok_or_else(|| {
                    format!("sh:filterShape node expression on {node} requires sh:nodes")
                })?;
                let inner = self.parse_node_expr(&nodes_obj)?;
                let shape = self.parse_inline_shape(shape_ref)?;
                Ok(NodeExpr::Filter {
                    nodes: Box::new(inner),
                    shape: Box::new(shape),
                })
            }
            sh::UNION => Ok(NodeExpr::Union(self.parse_node_expr_list(node, sh::UNION)?)),
            sh::INTERSECTION => Ok(NodeExpr::Intersection(
                self.parse_node_expr_list(node, sh::INTERSECTION)?,
            )),
            sh::IF => {
                let cond_obj = self
                    .first_object_of(node, sh::IF)
                    .ok_or_else(|| format!("sh:if node expression on {node} lost its object"))?;
                let cond = self.parse_node_expr(&cond_obj)?;
                // Per spec a missing `sh:then`/`sh:else` yields the empty set; the
                // empty union is the canonical empty-set node expression.
                let then = match self.first_object_of(node, sh::THEN) {
                    Some(t) => self.parse_node_expr(&t)?,
                    None => NodeExpr::Union(vec![]),
                };
                let els = match self.first_object_of(node, sh::ELSE) {
                    Some(t) => self.parse_node_expr(&t)?,
                    None => NodeExpr::Union(vec![]),
                };
                Ok(NodeExpr::If {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els: Box::new(els),
                })
            }
            sh::COUNT => {
                let of_obj = self
                    .first_object_of(node, sh::COUNT)
                    .ok_or_else(|| format!("sh:count node expression on {node} lost its object"))?;
                // Distinct counting is `[ sh:count [ sh:distinct <expr> ] ]`: an
                // inner `sh:distinct` lowers to `Count { distinct: true, .. }`.
                match self.parse_node_expr(&of_obj)? {
                    NodeExpr::Distinct(inner) => Ok(NodeExpr::Count {
                        distinct: true,
                        of: inner,
                    }),
                    other => Ok(NodeExpr::Count {
                        distinct: false,
                        of: Box::new(other),
                    }),
                }
            }
            sh::DISTINCT => {
                let of_obj = self.first_object_of(node, sh::DISTINCT).ok_or_else(|| {
                    format!("sh:distinct node expression on {node} lost its object")
                })?;
                Ok(NodeExpr::Distinct(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::MIN => {
                let of_obj = self
                    .first_object_of(node, sh::MIN)
                    .ok_or_else(|| format!("sh:min node expression on {node} lost its object"))?;
                Ok(NodeExpr::Min(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::MAX => {
                let of_obj = self
                    .first_object_of(node, sh::MAX)
                    .ok_or_else(|| format!("sh:max node expression on {node} lost its object"))?;
                Ok(NodeExpr::Max(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::SUM => {
                let of_obj = self
                    .first_object_of(node, sh::SUM)
                    .ok_or_else(|| format!("sh:sum node expression on {node} lost its object"))?;
                Ok(NodeExpr::Sum(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::EXISTS => {
                // Adopted semantics: `sh:exists` is a node-expression predicate —
                // true iff its inner NODE EXPRESSION yields at least one node for
                // the focus. (A shape does not "produce nodes", so the operand is
                // an expression, not a shape.)
                let inner_obj = self.first_object_of(node, sh::EXISTS).ok_or_else(|| {
                    format!("sh:exists node expression on {node} lost its object")
                })?;
                let inner = self.parse_node_expr(&inner_obj)?;
                Ok(NodeExpr::Exists(Box::new(inner)))
            }
            other => Err(format!(
                "internal error: unhandled node-expression key <{other}> on {node}"
            )),
        }
    }

    /// Parse the RDF list at `(node, predicate)` into a vector of node expressions.
    fn parse_node_expr_list(
        &mut self,
        node: &Term,
        predicate: &str,
    ) -> Result<Vec<NodeExpr>, String> {
        let list_head = self
            .first_object_of(node, predicate)
            .ok_or_else(|| format!("<{predicate}> node expression on {node} lost its object"))?;
        let items = self.walk_rdf_list(&list_head, node)?;
        let mut exprs = Vec::with_capacity(items.len());
        for item in items {
            exprs.push(self.parse_node_expr(&item)?);
        }
        Ok(exprs)
    }

    /// Parse a node carrying no structural key: a function call or a plain
    /// constant IRI (a blank node with neither hard-fails).
    fn parse_call_or_constant(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // A function-call node expression is always a blank node `[ <fn> ( … ) ]`.
        // A NamedNode reaching here (not a literal, not sh:this, no structural key)
        // is therefore a plain constant IRI — even when it bears unrelated outgoing
        // triples in the shapes graph (e.g. an `rdfs:label`).
        if matches!(node, Term::NamedNode(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // The SHACL-AF vocabulary terms that structure a node expression — none of
        // them can be the predicate of a function-call expression.
        const KNOWN: &[&str] = &[
            sh::PATH,
            sh::FILTER_SHAPE,
            sh::NODES,
            sh::UNION,
            sh::INTERSECTION,
            sh::IF,
            sh::THEN,
            sh::ELSE,
            sh::COUNT,
            sh::DISTINCT,
            sh::MIN,
            sh::MAX,
            sh::SUM,
            sh::LIMIT,
            sh::OFFSET,
            sh::ORDERBY,
            sh::DESC,
            sh::EXISTS,
        ];
        // Gather the candidate (function IRI, arg-list head) triples, ignoring
        // rdf:type (a classification triple) and every SHACL structural key.
        let mut candidates: Vec<(NamedNode, Term)> = self
            .data
            .quads_for_pattern(Some(node), None, None, GraphFilter::AnyGraph)
            .into_iter()
            .filter(|q| q.predicate.as_str() != rdf::TYPE && !KNOWN.contains(&q.predicate.as_str()))
            .map(|q| (q.predicate, q.object))
            .collect();
        candidates.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        candidates.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        if candidates.is_empty() {
            // No structural key and no function predicate: only blank nodes reach
            // here (NamedNodes returned early as constants above), and a blank
            // node with neither is malformed.
            return Err(format!(
                "unrecognised node expression on {node}: no SHACL-AF key and no function call"
            ));
        }
        if candidates.len() > 1 {
            return Err(format!(
                "ambiguous function-call node expression on {node}: multiple candidate predicates"
            ));
        }

        let (fn_iri, args_head) = candidates
            .into_iter()
            .next()
            .ok_or_else(|| format!("internal error: function-call candidate vanished on {node}"))?;
        // The single object must be an RDF list of argument node expressions
        // (`rdf:nil` is the empty argument list).
        let nil = Term::NamedNode(NamedNode::from(rdf::NIL));
        let is_list = args_head == nil || self.first_object_of(&args_head, rdf::FIRST).is_some();
        if !is_list {
            return Err(format!(
                "function-call node expression <{}> on {node} must carry an RDF list of arguments",
                fn_iri.as_str()
            ));
        }
        let items = self.walk_rdf_list(&args_head, node)?;
        let mut args = Vec::with_capacity(items.len());
        for item in items {
            args.push(self.parse_node_expr(&item)?);
        }
        // A user-defined function is typed `sh:SPARQLFunction` (or `sh:Function`)
        // in the shapes graph; anything else is treated as a builtin.
        let iri_term = Term::NamedNode(fn_iri.clone());
        let user_defined =
            self.has_type(&iri_term, sh::SPARQL_FUNCTION) || self.has_type(&iri_term, sh::FUNCTION);
        let call = if user_defined {
            FnCall::UserDefined { iri: fn_iri, args }
        } else {
            FnCall::Builtin { iri: fn_iri, args }
        };
        Ok(NodeExpr::Call(call))
    }
}

// ── Helper functions ───────────────────────────────────────────────────────────

/// Parse `sh:nodeKind` object IRI into a [`NodeKindValue`].
fn parse_node_kind(iri: &str) -> Option<NodeKindValue> {
    match iri {
        "http://www.w3.org/ns/shacl#IRI" => Some(NodeKindValue::Iri),
        "http://www.w3.org/ns/shacl#BlankNode" => Some(NodeKindValue::BlankNode),
        "http://www.w3.org/ns/shacl#Literal" => Some(NodeKindValue::Literal),
        "http://www.w3.org/ns/shacl#BlankNodeOrIRI" => Some(NodeKindValue::BlankNodeOrIri),
        "http://www.w3.org/ns/shacl#BlankNodeOrLiteral" => Some(NodeKindValue::BlankNodeOrLiteral),
        "http://www.w3.org/ns/shacl#IRIOrLiteral" => Some(NodeKindValue::IriOrLiteral),
        _ => None,
    }
}

/// Parse a typed integer literal or plain literal integer into a `u64`.
fn parse_u64(term: &Term) -> Option<u64> {
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
        assert!(all
            .iter()
            .any(|c| matches!(c, Constraint::LessThan(n) if n.as_str().ends_with("end"))));
        assert!(all
            .iter()
            .any(|c| matches!(c, Constraint::LessThanOrEquals(n) if n.as_str().ends_with("last"))));
        assert!(all
            .iter()
            .any(|c| matches!(c, Constraint::Equals(n) if n.as_str().ends_with('b'))));
        assert!(all
            .iter()
            .any(|c| matches!(c, Constraint::Disjoint(n) if n.as_str().ends_with('d'))));
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
        let data = IrDataGraph::new(Arc::clone(&dataset));
        let mut parser = Parser::new(&data, &[], None, Arc::clone(&dataset), None);
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
                assert!(shape
                    .constraints
                    .iter()
                    .any(|c| matches!(c, Constraint::NodeKind(NodeKindValue::Iri))));
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
