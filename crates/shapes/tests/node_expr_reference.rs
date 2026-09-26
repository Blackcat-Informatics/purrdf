// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A reference interpreter for SHACL 1.2 Node Expressions, and the differential
//! property that holds the production evaluator to it.
//!
//! # The reference
//!
//! [`Reference::eval`] is the specification's `evalExpr(expr, focusGraph,
//! focusNode, scope)` transcribed clause by clause, over its own expression type
//! ([`Expr`]) and its own graph (a plain list of triples). Each arm quotes the
//! evaluation clause it transcribes. It is deliberately slow and obvious: no
//! indexes, no plans, no prepared queries, no canonicalisation that a clause does
//! not ask for. The only production code it shares is SPARQL's `ORDER BY`
//! comparator ([`purrdf_sparql_eval::compare_values`]), which is where §4.2.8 and
//! §4.4.2/§4.4.3 delegate ("using the same logic as SPARQL ORDER BY", "see SPARQL
//! MIN") — the ordering of RDF terms is SPARQL's definition, not the node
//! expression specification's.
//!
//! # The property
//!
//! A generated [`World`] — a small data graph, a focus node and a node expression
//! over most of the node-expression language — is rendered to Turtle, loaded
//! through the production shapes parser, and evaluated by the production
//! evaluator. The two answers must be IDENTICAL SEQUENCES: the same nodes in the
//! same order with the same multiplicity, or both an evaluation failure.
//!
//! The only order-insensitivity is the one the specification itself grants. Where
//! a clause leaves the order undefined — path value nodes (a set), `sh:union`,
//! `shnex:intersection` ("the order is undefined"), `shnex:instancesOf` ("the
//! distinct nodes") and `shnex:nodesMatching` — the reference, like the
//! production evaluator, returns the members in the canonical term order: the
//! byte order of their N-Triples rendering. That is one admissible order, and
//! fixing it on both sides is STRICTER than comparing those outputs as sets,
//! because an undefined-order output flows into order-sensitive consumers
//! (`shnex:limit`, `shnex:findFirst`, a stable `shnex:orderBy`) whose answers
//! would otherwise depend on it.
//!
//! # What the generator covers, and what directed tests cover instead
//!
//! Every arm of the language except four is generated here. `sh:select` /
//! `sh:sparqlExpr` return a SPARQL solution sequence the reference would need a
//! SPARQL engine to reproduce; SHACL-AF function expressions dispatch through a
//! function registry; a computed `shnex:conformsToShape` shape argument needs a
//! shape index; and `shnex:var` over a non-empty scope needs an `sh:expression`
//! binding. Each has directed tests in `node_expressions.rs`,
//! `sparql_node_expressions.rs` and `custom_node_expressions.rs`.
//!
//! The run is deterministic: a fixed ChaCha seed, [`CASES`] cases, and no
//! on-disk failure persistence.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestCaseError, TestRng, TestRunner};
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::expression::{
    FnCall, NodeExpr, RecursionGuard, SequenceContract, ShapeArg, eval_node_expr,
};
use purrdf_shapes::shapes::{Constraint, Shapes};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

/// The number of generated worlds the differential property checks.
const CASES: u32 = 2048;

/// The fixed seed of the generator.
const SEED: [u8; 32] = *b"shacl12 node-expr reference seed";

const EX: &str = "http://example.org/ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

const PREFIXES: &str = r"
@prefix ex:     <http://example.org/ns#> .
@prefix rdf:    <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:   <http://www.w3.org/2000/01/rdf-schema#> .
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix shnex:  <http://www.w3.org/ns/shacl-node-expr#> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .
@prefix xsd:    <http://www.w3.org/2001/XMLSchema#> .
";

/// The shapes and custom functions every generated expression may name.
const FIXED_SHAPES: &str = r"
ex:Integers a sh:NodeShape ; sh:datatype xsd:integer .
ex:Anything a sh:NodeShape .
ex:HasP a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount 1 ] .
ex:IsC0 a sh:NodeShape ; sh:class ex:C0 .

ex:ident a sh:ListParameterExpressionFunction ;
    rdfs:subClassOf sh:ListParameterExpression ;
    sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ] ;
    sh:bodyExpression [ shnex:arg 0 ] .

ex:TwiceExpression a sh:NamedParameterExpressionFunction ;
    rdfs:subClassOf sh:NamedParameterExpression ;
    sh:parameter [ sh:path ex:twiceOf ; sh:keyParameter true ] ;
    sh:bodyExpression [ shnex:concat ( [ shnex:arg ex:twiceOf ] [ shnex:arg ex:twiceOf ] ) ] .
";

// ── The generated world ────────────────────────────────────────────────────────

/// A predicate a path may walk.
#[derive(Clone, Copy, Debug)]
enum Pred {
    P,
    Q,
    Type,
}

impl Pred {
    fn iri(self) -> String {
        match self {
            Self::P => format!("{EX}p"),
            Self::Q => format!("{EX}q"),
            Self::Type => RDF_TYPE.to_owned(),
        }
    }
}

/// A shape a shape-valued operand may name.
#[derive(Clone, Copy, Debug)]
enum ShapeRef {
    /// `ex:Integers` — `sh:datatype xsd:integer`.
    Integers,
    /// `ex:Anything` — a node shape with no constraints.
    Anything,
    /// `ex:HasP` — at least one `ex:p` value.
    HasP,
    /// `ex:IsC0` — a SHACL instance of `ex:C0`.
    IsC0,
    /// `[]` — the authored empty shape.
    Bare,
}

/// A constant term.
#[derive(Clone, Copy, Debug)]
enum Constant {
    Node(u8),
    Class(u8),
    Int(u8),
    Bool(bool),
}

/// An object of a generated data triple.
#[derive(Clone, Copy, Debug)]
enum Object {
    Node(u8),
    Int(u8),
}

/// A node expression of the generated fragment, in the specification's own terms.
#[derive(Clone, Debug)]
enum Expr {
    Const(Constant),
    This,
    FocusVar,
    UnboundVar,
    Empty,
    List(Vec<Constant>),
    Path {
        pred: Pred,
        inverse: bool,
    },
    PathFrom {
        pred: Pred,
        focus: Box<Self>,
    },
    Exists(Box<Self>),
    If {
        cond: Box<Self>,
        then: Box<Self>,
        els: Box<Self>,
    },
    Distinct(Box<Self>),
    Intersection(Vec<Self>),
    Union(Vec<Self>),
    Concat(Vec<Self>),
    Remove {
        nodes: Box<Self>,
        remove: Box<Self>,
    },
    Filter {
        shape: ShapeRef,
        nodes: Box<Self>,
    },
    Limit {
        n: u8,
        nodes: Box<Self>,
    },
    Offset {
        n: u8,
        nodes: Box<Self>,
    },
    OrderBy {
        nodes: Box<Self>,
        key: Box<Self>,
        desc: bool,
    },
    FlatMap {
        nodes: Box<Self>,
        map: Box<Self>,
    },
    FindFirst {
        shape: ShapeRef,
        nodes: Box<Self>,
    },
    MatchAll {
        shape: ShapeRef,
        nodes: Box<Self>,
    },
    Count(Box<Self>),
    Min(Box<Self>),
    Max(Box<Self>),
    Sum(Box<Self>),
    InstancesOf(Box<Self>),
    NodesMatching(ShapeRef),
    ConformsTo {
        node: Box<Self>,
        shape: ShapeRef,
    },
    Bound(Box<Self>),
    Coalesce(Vec<Self>),
    Ident(Box<Self>),
    Twice(Box<Self>),
}

impl Expr {
    /// The kind's name, for the coverage tally.
    const fn kind(&self) -> &'static str {
        match self {
            Self::Const(_) => "constant",
            Self::This => "sh:this",
            Self::FocusVar => "var focusNode",
            Self::UnboundVar => "var unbound",
            Self::Empty => "empty",
            Self::List(_) => "list",
            Self::Path { .. } => "pathValues",
            Self::PathFrom { .. } => "pathValues focusNode",
            Self::Exists(_) => "exists",
            Self::If { .. } => "if",
            Self::Distinct(_) => "distinct",
            Self::Intersection(_) => "intersection",
            Self::Union(_) => "sh:union",
            Self::Concat(_) => "concat",
            Self::Remove { .. } => "remove",
            Self::Filter { .. } => "filterShape",
            Self::Limit { .. } => "limit",
            Self::Offset { .. } => "offset",
            Self::OrderBy { .. } => "orderBy",
            Self::FlatMap { .. } => "flatMap",
            Self::FindFirst { .. } => "findFirst",
            Self::MatchAll { .. } => "matchAll",
            Self::Count(_) => "count",
            Self::Min(_) => "min",
            Self::Max(_) => "max",
            Self::Sum(_) => "sum",
            Self::InstancesOf(_) => "instancesOf",
            Self::NodesMatching(_) => "nodesMatching",
            Self::ConformsTo { .. } => "conformsToShape",
            Self::Bound(_) => "sparql:bound",
            Self::Coalesce(_) => "sparql:coalesce",
            Self::Ident(_) => "custom list function",
            Self::Twice(_) => "custom named function",
        }
    }

    /// Every kind this expression contains, itself included.
    fn kinds(&self, out: &mut BTreeMap<&'static str, usize>) {
        *out.entry(self.kind()).or_insert(0) += 1;
        match self {
            Self::Const(_)
            | Self::This
            | Self::FocusVar
            | Self::UnboundVar
            | Self::Empty
            | Self::List(_)
            | Self::Path { .. }
            | Self::NodesMatching(_) => {}
            Self::PathFrom { focus: inner, .. }
            | Self::Exists(inner)
            | Self::Distinct(inner)
            | Self::Filter { nodes: inner, .. }
            | Self::Limit { nodes: inner, .. }
            | Self::Offset { nodes: inner, .. }
            | Self::FindFirst { nodes: inner, .. }
            | Self::MatchAll { nodes: inner, .. }
            | Self::Count(inner)
            | Self::Min(inner)
            | Self::Max(inner)
            | Self::Sum(inner)
            | Self::InstancesOf(inner)
            | Self::ConformsTo { node: inner, .. }
            | Self::Bound(inner)
            | Self::Ident(inner)
            | Self::Twice(inner) => inner.kinds(out),
            Self::If { cond, then, els } => {
                cond.kinds(out);
                then.kinds(out);
                els.kinds(out);
            }
            Self::Intersection(items)
            | Self::Union(items)
            | Self::Concat(items)
            | Self::Coalesce(items) => {
                for item in items {
                    item.kinds(out);
                }
            }
            Self::Remove { nodes, remove } => {
                nodes.kinds(out);
                remove.kinds(out);
            }
            Self::OrderBy { nodes, key, .. } => {
                nodes.kinds(out);
                key.kinds(out);
            }
            Self::FlatMap { nodes, map } => {
                nodes.kinds(out);
                map.kinds(out);
            }
        }
    }
}

/// One generated case.
#[derive(Clone, Debug)]
struct World {
    edges: Vec<(u8, Pred, Object)>,
    types: Vec<(u8, u8)>,
    focus: u8,
    expr: Expr,
}

// ── Terms ──────────────────────────────────────────────────────────────────────

fn iri(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!("{EX}{local}")))
}

fn integer(value: i64) -> Term {
    Term::Literal(Literal::new_typed_literal(
        value.to_string(),
        NamedNode::new_unchecked(XSD_INTEGER),
    ))
}

fn boolean(value: bool) -> Term {
    Term::Literal(Literal::new_typed_literal(
        if value { "true" } else { "false" },
        NamedNode::new_unchecked(XSD_BOOLEAN),
    ))
}

impl Constant {
    fn term(self) -> Term {
        match self {
            Self::Node(i) => iri(&format!("n{i}")),
            Self::Class(c) => iri(&format!("C{c}")),
            Self::Int(k) => integer(i64::from(k)),
            Self::Bool(b) => boolean(b),
        }
    }

    fn turtle(self) -> String {
        match self {
            Self::Node(i) => format!("ex:n{i}"),
            Self::Class(c) => format!("ex:C{c}"),
            Self::Int(k) => k.to_string(),
            Self::Bool(b) => b.to_string(),
        }
    }
}

impl Object {
    fn term(self) -> Term {
        match self {
            Self::Node(i) => iri(&format!("n{i}")),
            Self::Int(k) => integer(i64::from(k)),
        }
    }
}

/// The integer value of `term`, when it is an `xsd:integer` literal.
fn integer_value(term: &Term) -> Option<i64> {
    match term {
        Term::Literal(lit) if lit.datatype_str() == XSD_INTEGER => lit.value().parse().ok(),
        _ => None,
    }
}

/// The canonical term order: the byte order of the N-Triples rendering.
fn canonical(mut terms: Vec<Term>) -> Vec<Term> {
    terms.sort_by_cached_key(ToString::to_string);
    terms.dedup();
    terms
}

/// SPARQL `ORDER BY` order over two terms.
fn sparql_order(left: &Term, right: &Term) -> Ordering {
    purrdf_sparql_eval::compare_values(&left.to_term_value(), &right.to_term_value())
}

// ── Rendering the world to Turtle ──────────────────────────────────────────────

impl ShapeRef {
    const fn turtle(self) -> &'static str {
        match self {
            Self::Integers => "ex:Integers",
            Self::Anything => "ex:Anything",
            Self::HasP => "ex:HasP",
            Self::IsC0 => "ex:IsC0",
            Self::Bare => "[]",
        }
    }
}

fn pred_turtle(pred: Pred, inverse: bool) -> String {
    let name = match pred {
        Pred::P => "ex:p",
        Pred::Q => "ex:q",
        Pred::Type => "rdf:type",
    };
    if inverse {
        format!("[ sh:inversePath {name} ]")
    } else {
        name.to_owned()
    }
}

fn list_turtle(items: &[Expr]) -> String {
    let mut out = String::from("(");
    for item in items {
        out.push(' ');
        out.push_str(&item.turtle());
    }
    out.push_str(" )");
    out
}

impl Expr {
    fn turtle(&self) -> String {
        match self {
            Self::Const(c) => c.turtle(),
            Self::This => "sh:this".to_owned(),
            Self::FocusVar => r#"[ shnex:var "focusNode" ]"#.to_owned(),
            Self::UnboundVar => r#"[ shnex:var "unbound" ]"#.to_owned(),
            Self::Empty => "[]".to_owned(),
            Self::List(items) => {
                let members: Vec<String> = items.iter().map(|c| c.turtle()).collect();
                format!("( {} )", members.join(" "))
            }
            Self::Path { pred, inverse } => {
                format!("[ shnex:pathValues {} ]", pred_turtle(*pred, *inverse))
            }
            Self::PathFrom { pred, focus } => format!(
                "[ shnex:pathValues {} ; shnex:focusNode {} ]",
                pred_turtle(*pred, false),
                focus.turtle()
            ),
            Self::Exists(e) => format!("[ shnex:exists {} ]", e.turtle()),
            Self::If { cond, then, els } => format!(
                "[ shnex:if {} ; shnex:then {} ; shnex:else {} ]",
                cond.turtle(),
                then.turtle(),
                els.turtle()
            ),
            Self::Distinct(e) => format!("[ shnex:distinct {} ]", e.turtle()),
            Self::Intersection(items) => format!("[ shnex:intersection {} ]", list_turtle(items)),
            Self::Union(items) => format!("[ sh:union {} ]", list_turtle(items)),
            Self::Concat(items) => format!("[ shnex:concat {} ]", list_turtle(items)),
            Self::Remove { nodes, remove } => format!(
                "[ shnex:nodes {} ; shnex:remove {} ]",
                nodes.turtle(),
                remove.turtle()
            ),
            Self::Filter { shape, nodes } => format!(
                "[ shnex:filterShape {} ; shnex:nodes {} ]",
                shape.turtle(),
                nodes.turtle()
            ),
            Self::Limit { n, nodes } => {
                format!("[ shnex:limit {n} ; shnex:nodes {} ]", nodes.turtle())
            }
            Self::Offset { n, nodes } => {
                format!("[ shnex:offset {n} ; shnex:nodes {} ]", nodes.turtle())
            }
            Self::OrderBy { nodes, key, desc } => format!(
                "[ shnex:orderBy {} ; shnex:nodes {} ; shnex:desc {desc} ]",
                key.turtle(),
                nodes.turtle()
            ),
            Self::FlatMap { nodes, map } => format!(
                "[ shnex:flatMap {} ; shnex:nodes {} ]",
                map.turtle(),
                nodes.turtle()
            ),
            Self::FindFirst { shape, nodes } => format!(
                "[ shnex:findFirst {} ; shnex:nodes {} ]",
                shape.turtle(),
                nodes.turtle()
            ),
            Self::MatchAll { shape, nodes } => format!(
                "[ shnex:matchAll {} ; shnex:nodes {} ]",
                shape.turtle(),
                nodes.turtle()
            ),
            Self::Count(e) => format!("[ shnex:count {} ]", e.turtle()),
            Self::Min(e) => format!("[ shnex:min {} ]", e.turtle()),
            Self::Max(e) => format!("[ shnex:max {} ]", e.turtle()),
            Self::Sum(e) => format!("[ shnex:sum {} ]", e.turtle()),
            Self::InstancesOf(e) => format!("[ shnex:instancesOf {} ]", e.turtle()),
            Self::NodesMatching(shape) => format!("[ shnex:nodesMatching {} ]", shape.turtle()),
            Self::ConformsTo { node, shape } => {
                // The shape member of this list parameter function is a node
                // expression, so a named shape is written as its IRI; the empty
                // shape `[]` would be the EMPTY EXPRESSION here, not a shape, so
                // the generator never passes `ShapeRef::Bare` to this kind.
                format!(
                    "[ shnex:conformsToShape ( {} {} ) ]",
                    node.turtle(),
                    shape.turtle()
                )
            }
            Self::Bound(e) => format!("[ sparql:bound ( {} ) ]", e.turtle()),
            Self::Coalesce(items) => format!("[ sparql:coalesce {} ]", list_turtle(items)),
            Self::Ident(e) => format!("[ ex:ident ( {} ) ]", e.turtle()),
            Self::Twice(e) => format!("[ ex:twiceOf {} ]", e.turtle()),
        }
    }
}

impl World {
    fn data_turtle(&self) -> String {
        let mut out = String::from(PREFIXES);
        out.push_str("ex:C1 rdfs:subClassOf ex:C0 .\n");
        for (subject, pred, object) in &self.edges {
            let object = match object {
                Object::Node(i) => format!("ex:n{i}"),
                Object::Int(k) => k.to_string(),
            };
            let _ = writeln!(
                out,
                "ex:n{subject} {} {object} .",
                pred_turtle(*pred, false)
            );
        }
        for (node, class) in &self.types {
            let _ = writeln!(out, "ex:n{node} a ex:C{class} .");
        }
        out
    }

    fn shapes_turtle(&self) -> String {
        format!(
            "{PREFIXES}{FIXED_SHAPES}\nex:Holder a sh:NodeShape ; sh:expression {} .\n",
            self.expr.turtle()
        )
    }

    /// The data graph as the reference reads it: a list of triples.
    fn triples(&self) -> Vec<(Term, Term, Term)> {
        let mut out = vec![(
            iri("C1"),
            Term::NamedNode(NamedNode::new_unchecked(RDFS_SUB_CLASS_OF)),
            iri("C0"),
        )];
        for (subject, pred, object) in &self.edges {
            out.push((
                iri(&format!("n{subject}")),
                Term::NamedNode(NamedNode::new_unchecked(pred.iri())),
                object.term(),
            ));
        }
        for (node, class) in &self.types {
            out.push((
                iri(&format!("n{node}")),
                Term::NamedNode(NamedNode::new_unchecked(RDF_TYPE)),
                iri(&format!("C{class}")),
            ));
        }
        out
    }
}

// ── The reference interpreter ──────────────────────────────────────────────────

/// `evalExpr` over a list of triples.
struct Reference {
    triples: Vec<(Term, Term, Term)>,
}

type Output = Result<Vec<Term>, String>;

impl Reference {
    /// The value nodes of a predicate path, or of its inverse, from `focus`.
    ///
    /// SHACL Core value nodes are a set; the reference returns it in canonical
    /// order.
    fn value_nodes(&self, focus: &Term, pred: Pred, inverse: bool) -> Vec<Term> {
        let pred = pred.iri();
        let mut out = Vec::new();
        for (s, p, o) in &self.triples {
            let Term::NamedNode(p) = p else { continue };
            if p.as_str() != pred {
                continue;
            }
            if inverse && o == focus {
                out.push(s.clone());
            } else if !inverse && s == focus {
                out.push(o.clone());
            }
        }
        canonical(out)
    }

    /// Whether `node` is a SHACL instance of `class`: an `rdf:type` whose object
    /// reaches `class` through zero or more `rdfs:subClassOf`.
    fn is_instance(&self, node: &Term, class: &Term) -> bool {
        self.triples.iter().any(|(s, p, o)| {
            s == node
                && matches!(p, Term::NamedNode(p) if p.as_str() == RDF_TYPE)
                && self.is_subclass(o, class)
        })
    }

    fn is_subclass(&self, sub: &Term, sup: &Term) -> bool {
        let mut frontier = vec![sub.clone()];
        let mut seen: Vec<Term> = Vec::new();
        while let Some(next) = frontier.pop() {
            if &next == sup {
                return true;
            }
            if seen.contains(&next) {
                continue;
            }
            seen.push(next.clone());
            for (s, p, o) in &self.triples {
                if s == &next && matches!(p, Term::NamedNode(p) if p.as_str() == RDFS_SUB_CLASS_OF)
                {
                    frontier.push(o.clone());
                }
            }
        }
        false
    }

    /// Whether `node` conforms to `shape`, decided from each fixed shape's own
    /// definition.
    fn conforms(&self, node: &Term, shape: ShapeRef) -> bool {
        match shape {
            ShapeRef::Integers => integer_value(node).is_some(),
            ShapeRef::Anything | ShapeRef::Bare => true,
            ShapeRef::HasP => !self.value_nodes(node, Pred::P, false).is_empty(),
            ShapeRef::IsC0 => self.is_instance(node, &iri("C0")),
        }
    }

    /// Every node of the graph: the subjects and objects of its triples.
    fn nodes(&self) -> Vec<Term> {
        canonical(
            self.triples
                .iter()
                .flat_map(|(s, _, o)| [s.clone(), o.clone()])
                .collect(),
        )
    }

    /// One argument of a LIST parameter function (§3.2.2): "each argument of a
    /// list parameter function must evaluate to an individual, single node … An
    /// evaluation failure must be produced if there is more than one output node."
    fn argument(&self, expr: &Expr, focus: &Term) -> Result<Option<Term>, String> {
        let mut out = self.eval(expr, focus)?;
        match out.len() {
            0 => Ok(None),
            1 => Ok(out.pop()),
            n => Err(format!("argument produced {n} nodes")),
        }
    }

    /// `evalExpr(expr, focusGraph, focusNode, {})`.
    #[allow(clippy::too_many_lines)]
    fn eval(&self, expr: &Expr, focus: &Term) -> Output {
        match expr {
            // §3.1.1-§3.1.3: "evalExpr(expr, focusGraph, focusNode, scope) -> [expr]".
            Expr::Const(c) => Ok(vec![c.term()]),
            // SHACL-AF §6.1: "Eval(sh:this, $this) produces the set { $this }".
            Expr::This => Ok(vec![focus.clone()]),
            // §4.1.2: "if var is "focusNode" then … -> [focusNode]".
            Expr::FocusVar => Ok(vec![focus.clone()]),
            // §4.1.2: the scope is `{}`, so every other name is "otherwise … -> []".
            Expr::UnboundVar => Ok(Vec::new()),
            // §4.1.1: "An empty expression has the empty list [] as its output nodes".
            Expr::Empty => Ok(Vec::new()),
            // §4.1.3: "the members of the list expression, in the same order as in
            // the list".
            Expr::List(items) => Ok(items.iter().map(|c| c.term()).collect()),
            // §4.1.4: "If shnex:focusNode is not given, $focusNode is the list
            // consisting of exactly the focus node … the output nodes of the path
            // values expression are the list of value nodes of the path".
            Expr::Path { pred, inverse } => Ok(self.value_nodes(focus, *pred, *inverse)),
            // §4.1.4: "If N has 0 members, then the output nodes are the empty list.
            // If N has more than 1 member, an evaluation failure is reported.
            // Otherwise … the list of value nodes of the path for the (only) member".
            Expr::PathFrom { pred, focus: from } => {
                let n = self.eval(from, focus)?;
                match n.as_slice() {
                    [] => Ok(Vec::new()),
                    [only] => Ok(self.value_nodes(only, *pred, false)),
                    _ => Err("shnex:focusNode produced more than one node".to_owned()),
                }
            }
            // §4.1.5: "( true ) if and only if N has at least one member; otherwise
            // … ( false )".
            Expr::Exists(inner) => Ok(vec![boolean(!self.eval(inner, focus)?.is_empty())]),
            // §4.1.6: "If IFs is the list ( true ), then … evalExpr(then, …).
            // Otherwise … evalExpr(else, …)".
            Expr::If { cond, then, els } => {
                if self.eval(cond, focus)? == vec![boolean(true)] {
                    self.eval(then, focus)
                } else {
                    self.eval(els, focus)
                }
            }
            // §4.2.1: "the list of nodes in input in the same order but with
            // duplicates eliminated (the first occurrences of each node shall be
            // kept, the others removed)".
            Expr::Distinct(inner) => {
                let mut out: Vec<Term> = Vec::new();
                for node in self.eval(inner, focus)? {
                    if !out.contains(&node) {
                        out.push(node);
                    }
                }
                Ok(out)
            }
            // §4.2.2: "the nodes that form the intersection of the output nodes
            // produced by each node expression … The intersection does not include
            // duplicates and the order is undefined."
            Expr::Intersection(members) => {
                let lists: Vec<Vec<Term>> = members
                    .iter()
                    .map(|m| self.eval(m, focus))
                    .collect::<Result<_, _>>()?;
                let Some((first, rest)) = lists.split_first() else {
                    return Ok(Vec::new());
                };
                Ok(canonical(
                    first
                        .iter()
                        .filter(|node| rest.iter().all(|list| list.contains(node)))
                        .cloned()
                        .collect(),
                ))
            }
            // SHACL-AF §6.7: "the set of nodes that are in any of the result sets".
            Expr::Union(members) => {
                let mut all = Vec::new();
                for member in members {
                    all.extend(self.eval(member, focus)?);
                }
                Ok(canonical(all))
            }
            // §4.2.3: "the concatenation of all output nodes for each node
            // expression NE in members … The order is preserved".
            Expr::Concat(members) => {
                let mut all = Vec::new();
                for member in members {
                    all.extend(self.eval(member, focus)?);
                }
                Ok(all)
            }
            // §4.2.4: "the nodes in N except those that are also in M, preserving
            // the order of N".
            Expr::Remove { nodes, remove } => {
                let m = self.eval(remove, focus)?;
                let n = self.eval(nodes, focus)?;
                Ok(n.into_iter().filter(|node| !m.contains(node)).collect())
            }
            // §4.2.5: "except those that do not conform to the shape filterShape,
            // preserving the order in the list".
            Expr::Filter { shape, nodes } => Ok(self
                .eval(nodes, focus)?
                .into_iter()
                .filter(|node| self.conforms(node, *shape))
                .collect()),
            // §4.2.6: "the first limit nodes in N from left to right, in the same
            // order".
            Expr::Limit { n, nodes } => Ok(self
                .eval(nodes, focus)?
                .into_iter()
                .take(usize::from(*n))
                .collect()),
            // §4.2.7: "the nodes in N except for the first offset nodes from left to
            // right, in the same order".
            Expr::Offset { n, nodes } => Ok(self
                .eval(nodes, focus)?
                .into_iter()
                .skip(usize::from(*n))
                .collect()),
            // §4.2.8: "Let c(n) be the first output node of evalExpr(orderBy,
            // focusGraph, n, scope) … sorted by c(n) using the same logic as SPARQL
            // ORDER BY. Nodes where c(n) is unbound are considered smaller than those
            // that have any value. If desc is true then the output nodes are
            // returned in the reverse order."
            Expr::OrderBy { nodes, key, desc } => {
                let n = self.eval(nodes, focus)?;
                let mut keyed: Vec<(Option<Term>, Term)> = Vec::new();
                for node in n {
                    let c = self.eval(key, &node)?.into_iter().next();
                    keyed.push((c, node));
                }
                // An insertion sort, so equal keys keep their input order without
                // relying on any library sort's stability.
                let mut sorted: Vec<(Option<Term>, Term)> = Vec::new();
                for entry in keyed {
                    let position = sorted
                        .iter()
                        .position(|(other, _)| match (&entry.0, other) {
                            (None | Some(_), None) => false,
                            (None, Some(_)) => true,
                            (Some(mine), Some(theirs)) => {
                                sparql_order(mine, theirs) == Ordering::Less
                            }
                        })
                        .unwrap_or(sorted.len());
                    sorted.insert(position, entry);
                }
                let mut out: Vec<Term> = sorted.into_iter().map(|(_, node)| node).collect();
                if *desc {
                    out.reverse();
                }
                Ok(out)
            }
            // §4.3.1: "For each node n in N, let Mn be the output nodes of
            // evalExpr(flatMap, focusGraph, n, scope). The output nodes … are
            // produced by concatenating all sequences Mn in the order of the
            // corresponding nodes n in N."
            Expr::FlatMap { nodes, map } => {
                let mut out = Vec::new();
                for node in self.eval(nodes, focus)? {
                    out.extend(self.eval(map, &node)?);
                }
                Ok(out)
            }
            // §4.3.2: "exactly the first node n in N that conforms to the shape
            // shape, or an empty sequence if no such node exists".
            Expr::FindFirst { shape, nodes } => Ok(self
                .eval(nodes, focus)?
                .into_iter()
                .find(|node| self.conforms(node, *shape))
                .into_iter()
                .collect()),
            // §4.3.3: "( true ) if every node n in N conforms to the shape shape;
            // otherwise the output nodes are ( false )".
            Expr::MatchAll { shape, nodes } => Ok(vec![boolean(
                self.eval(nodes, focus)?
                    .iter()
                    .all(|node| self.conforms(node, *shape)),
            )]),
            // §4.4.1: "exactly one xsd:integer literal that is computed as the
            // length of N".
            Expr::Count(inner) => Ok(vec![integer(self.eval(inner, focus)?.len() as i64)]),
            // §4.4.2 / §4.4.3: "at most one node that is computed as the minimum
            // (maximum) value from N, see SPARQL MIN (MAX)".
            Expr::Min(inner) | Expr::Max(inner) => {
                let want = if matches!(expr, Expr::Min(_)) {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
                let mut best: Option<Term> = None;
                for node in self.eval(inner, focus)? {
                    best = match best {
                        Some(current) if sparql_order(&node, &current) != want => Some(current),
                        _ => Some(node),
                    };
                }
                Ok(best.into_iter().collect())
            }
            // §4.4.4: "exactly one node that is computed as the sum of all nodes from
            // N, see SPARQL SUM". SPARQL's SUM of nothing is 0; a SUM over a
            // non-numeric value is a SPARQL aggregate error, whose value is unbound
            // — no node.
            Expr::Sum(inner) => {
                let mut total: i64 = 0;
                for node in self.eval(inner, focus)? {
                    let Some(value) = integer_value(&node) else {
                        return Ok(Vec::new());
                    };
                    total += value;
                }
                Ok(vec![integer(total)])
            }
            // §4.5.1: "An evaluation failure is reported when any of the members of
            // types is not an IRI. The output nodes of the instancesOf expression are
            // the distinct nodes that are SHACL instances in the focus graph of each
            // member of types."
            Expr::InstancesOf(types) => {
                let types = self.eval(types, focus)?;
                if types.iter().any(|t| !matches!(t, Term::NamedNode(_))) {
                    return Err("a class is not an IRI".to_owned());
                }
                Ok(canonical(
                    self.nodes()
                        .into_iter()
                        .filter(|node| types.iter().any(|class| self.is_instance(node, class)))
                        .collect(),
                ))
            }
            // §4.5.2: "the nodes in the focus graph that conform to shape".
            Expr::NodesMatching(shape) => Ok(self
                .nodes()
                .into_iter()
                .filter(|node| self.conforms(node, *shape))
                .collect()),
            // §4.5.3: "the empty list if either node or shape have no value.
            // Otherwise … ( true ) if and only if node conforms to shape", with the
            // list parameter function's one-node bound on its argument.
            Expr::ConformsTo { node, shape } => match self.argument(node, focus)? {
                None => Ok(Vec::new()),
                Some(node) => Ok(vec![boolean(self.conforms(&node, *shape))]),
            },
            // §5 with SPARQL's BOUND: an argument producing no node reaches SPARQL
            // unbound.
            Expr::Bound(arg) => Ok(vec![boolean(self.argument(arg, focus)?.is_some())]),
            // §5 with SPARQL's COALESCE: "the value of the first expression that
            // evaluates without error" — an unbound argument is the error. With
            // none left, COALESCE raises an error, and the call produces no result.
            Expr::Coalesce(args) => {
                let mut first: Option<Term> = None;
                for arg in args {
                    let value = self.argument(arg, focus)?;
                    if first.is_none() {
                        first = value;
                    }
                }
                Ok(first.into_iter().collect())
            }
            // §6.2 over the body `[ shnex:arg 0 ]`: §6.3 evaluates the bound
            // argument, `evalExpr(a, focusGraph, focusNode, {})`, an indexed argument
            // produces at most one node (§3.2.2), and "an evaluation failure is
            // reported when there is more than 1 output node" of the body.
            Expr::Ident(arg) => Ok(self.argument(arg, focus)?.into_iter().collect()),
            // §6.1 over the body `[ shnex:concat ( [ shnex:arg ex:twiceOf ]
            // [ shnex:arg ex:twiceOf ] ) ]`: a named argument carries no bound.
            Expr::Twice(arg) => {
                let mut out = self.eval(arg, focus)?;
                out.extend(self.eval(arg, focus)?);
                Ok(out)
            }
        }
    }
}

// ── The production side ────────────────────────────────────────────────────────

/// The `sh:expression` of `ex:Holder`, through the production parser.
fn holder_expression(shapes: &Shapes) -> Result<NodeExpr, String> {
    shapes
        .node_shapes
        .iter()
        .filter(|shape| shape.id == iri("Holder"))
        .flat_map(|shape| &shape.constraints)
        .find_map(|constraint| match constraint {
            Constraint::Expression { expr, .. } => Some(expr.clone()),
            _ => None,
        })
        .ok_or_else(|| "ex:Holder carries no sh:expression".to_owned())
}

/// The production evaluator's answer for `world`, evaluated twice so a run whose
/// answer depended on anything but its inputs would show.
fn production(world: &World) -> Result<Output, String> {
    let shapes_ttl = world.shapes_turtle();
    let shapes = parse_shapes(&shapes_ttl, None)
        .map_err(|e| format!("the generated shapes graph did not load: {e}\n{shapes_ttl}"))?;
    let expr = holder_expression(&shapes)?;
    let data: Arc<_> = parse_turtle_to_dataset(&world.data_turtle(), None)
        .map_err(|e| format!("the generated data graph did not load: {}", e.join("; ")))?;
    let shapes_ds: Arc<_> = parse_turtle_to_dataset(&shapes_ttl, None)
        .map_err(|e| format!("the generated shapes graph did not parse: {}", e.join("; ")))?;
    let store = ShaclData::new(Arc::clone(&data), shapes_ds, None);
    let focus = iri(&format!("n{}", world.focus));
    let first = eval_node_expr(&store, &focus, &expr, &mut RecursionGuard::new());
    let second = eval_node_expr(&store, &focus, &expr, &mut RecursionGuard::new());
    if first != second {
        return Err(format!(
            "two evaluations of one expression disagree: {first:?} then {second:?}"
        ));
    }
    Ok(first)
}

// ── The generator ──────────────────────────────────────────────────────────────

fn constant() -> impl Strategy<Value = Constant> {
    prop_oneof![
        (0u8..4).prop_map(Constant::Node),
        (0u8..2).prop_map(Constant::Class),
        (0u8..5).prop_map(Constant::Int),
        any::<bool>().prop_map(Constant::Bool),
    ]
}

fn pred() -> impl Strategy<Value = Pred> {
    prop_oneof![Just(Pred::P), Just(Pred::Q), Just(Pred::Type)]
}

fn named_shape() -> impl Strategy<Value = ShapeRef> {
    prop_oneof![
        Just(ShapeRef::Integers),
        Just(ShapeRef::Anything),
        Just(ShapeRef::HasP),
        Just(ShapeRef::IsC0),
    ]
}

fn shape() -> impl Strategy<Value = ShapeRef> {
    prop_oneof![4 => named_shape(), 1 => Just(ShapeRef::Bare)]
}

fn leaf() -> impl Strategy<Value = Expr> {
    prop_oneof![
        3 => constant().prop_map(Expr::Const),
        1 => Just(Expr::This),
        1 => Just(Expr::FocusVar),
        1 => Just(Expr::UnboundVar),
        1 => Just(Expr::Empty),
        4 => prop::collection::vec(constant(), 1..5).prop_map(Expr::List),
        3 => (pred(), any::<bool>()).prop_map(|(pred, inverse)| Expr::Path { pred, inverse }),
        1 => shape().prop_map(Expr::NodesMatching),
        1 => (0u8..2).prop_map(|c| Expr::InstancesOf(Box::new(Expr::Const(Constant::Class(c))))),
    ]
}

fn expression() -> impl Strategy<Value = Expr> {
    leaf().prop_recursive(4, 40, 3, |inner| {
        let boxed = inner.clone().prop_map(Box::new);
        let items = prop::collection::vec(inner.clone(), 2..4);
        prop_oneof![
            (pred(), boxed.clone()).prop_map(|(pred, focus)| Expr::PathFrom { pred, focus }),
            boxed.clone().prop_map(Expr::Exists),
            (boxed.clone(), boxed.clone(), boxed.clone()).prop_map(|(cond, then, els)| Expr::If {
                cond,
                then,
                els
            }),
            boxed.clone().prop_map(Expr::Distinct),
            items.clone().prop_map(Expr::Intersection),
            items.clone().prop_map(Expr::Union),
            items.prop_map(Expr::Concat),
            (boxed.clone(), boxed.clone())
                .prop_map(|(nodes, remove)| Expr::Remove { nodes, remove }),
            (shape(), boxed.clone()).prop_map(|(shape, nodes)| Expr::Filter { shape, nodes }),
            (0u8..4, boxed.clone()).prop_map(|(n, nodes)| Expr::Limit { n, nodes }),
            (0u8..4, boxed.clone()).prop_map(|(n, nodes)| Expr::Offset { n, nodes }),
            (boxed.clone(), boxed.clone(), any::<bool>())
                .prop_map(|(nodes, key, desc)| Expr::OrderBy { nodes, key, desc }),
            (boxed.clone(), boxed.clone()).prop_map(|(nodes, map)| Expr::FlatMap { nodes, map }),
            (shape(), boxed.clone()).prop_map(|(shape, nodes)| Expr::FindFirst { shape, nodes }),
            (shape(), boxed.clone()).prop_map(|(shape, nodes)| Expr::MatchAll { shape, nodes }),
            boxed.clone().prop_map(Expr::Count),
            boxed.clone().prop_map(Expr::Min),
            boxed.clone().prop_map(Expr::Max),
            boxed.clone().prop_map(Expr::Sum),
            boxed.clone().prop_map(Expr::InstancesOf),
            (boxed.clone(), named_shape())
                .prop_map(|(node, shape)| Expr::ConformsTo { node, shape }),
            boxed.clone().prop_map(Expr::Bound),
            prop::collection::vec(inner, 1..4).prop_map(Expr::Coalesce),
            boxed.clone().prop_map(Expr::Ident),
            boxed.prop_map(Expr::Twice),
        ]
    })
}

fn world() -> impl Strategy<Value = World> {
    let object = prop_oneof![
        (0u8..4).prop_map(Object::Node),
        (0u8..5).prop_map(Object::Int),
    ];
    let edge_pred = prop_oneof![Just(Pred::P), Just(Pred::Q)];
    (
        prop::collection::vec((0u8..4, edge_pred, object), 0..10),
        prop::collection::vec((0u8..4, 0u8..2), 0..5),
        0u8..4,
        expression(),
    )
        .prop_map(|(edges, types, focus, expr)| World {
            edges,
            types,
            focus,
            expr,
        })
}

// ── The property ───────────────────────────────────────────────────────────────

/// What the run saw, so a run that compared nothing cannot pass.
#[derive(Debug, Default)]
struct Tally {
    /// Both sides produced output nodes, and at least one node.
    nonempty: u32,
    /// Both sides produced the empty list.
    empty: u32,
    /// Both sides reported an evaluation failure.
    failed: u32,
    /// How many generated expressions contained each kind.
    kinds: BTreeMap<&'static str, usize>,
}

fn render(terms: &[Term]) -> String {
    let rendered: Vec<String> = terms.iter().map(ToString::to_string).collect();
    format!("( {} )", rendered.join(" "))
}

/// The production evaluator produces EXACTLY the sequence the reference
/// interpreter produces, or fails exactly where it fails.
#[test]
fn production_evaluation_matches_the_reference_interpreter() {
    let config = Config {
        cases: CASES,
        // No on-disk regression files: the fixed seed already makes every run
        // identical.
        failure_persistence: None,
        ..Config::default()
    };
    let mut runner =
        TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &SEED));
    let tally = std::cell::RefCell::new(Tally::default());
    let outcome = runner.run(&world(), |world| {
        let reference = Reference {
            triples: world.triples(),
        }
        .eval(&world.expr, &iri(&format!("n{}", world.focus)));
        let produced = production(&world).map_err(TestCaseError::fail)?;
        let mut tally = tally.borrow_mut();
        world.expr.kinds(&mut tally.kinds);
        match (&reference, &produced) {
            (Ok(expected), Ok(actual)) if expected == actual => {
                if expected.is_empty() {
                    tally.empty += 1;
                } else {
                    tally.nonempty += 1;
                }
                Ok(())
            }
            (Err(_), Err(_)) => {
                tally.failed += 1;
                Ok(())
            }
            _ => Err(TestCaseError::fail(format!(
                "the production evaluator disagrees with the reference\n  expression: {}\n  \
                 data:\n{}\n  focus: ex:n{}\n  reference:  {}\n  production: {}",
                world.expr.turtle(),
                world.data_turtle(),
                world.focus,
                reference
                    .as_ref()
                    .map_or_else(|e| format!("failure ({e})"), |v| render(v)),
                produced
                    .as_ref()
                    .map_or_else(|e| format!("failure ({e})"), |v| render(v)),
            ))),
        }
    });
    if let Err(failure) = outcome {
        panic!("{failure}");
    }
    let tally = tally.into_inner();
    println!("NODE-EXPR REFERENCE: {CASES} cases → {tally:?}");
    assert_eq!(
        tally.nonempty + tally.empty + tally.failed,
        CASES,
        "every configured case must have been compared"
    );
    // Non-vacuity: most comparisons are between actual answers, and a good share
    // of those answers carry nodes. A generator that drifted into producing only
    // failures, or only empty lists, would compare nothing worth comparing.
    assert!(
        tally.nonempty * 5 >= CASES * 2,
        "fewer than two cases in five compared a non-empty answer: {tally:?}"
    );
    assert!(
        tally.failed > 0,
        "no case exercised an evaluation failure, so the failure paths went unchecked: {tally:?}"
    );
    // Coverage: every generated kind reached the comparison.
    for kind in [
        "constant",
        "sh:this",
        "var focusNode",
        "var unbound",
        "empty",
        "list",
        "pathValues",
        "pathValues focusNode",
        "exists",
        "if",
        "distinct",
        "intersection",
        "sh:union",
        "concat",
        "remove",
        "filterShape",
        "limit",
        "offset",
        "orderBy",
        "flatMap",
        "findFirst",
        "matchAll",
        "count",
        "min",
        "max",
        "sum",
        "instancesOf",
        "nodesMatching",
        "conformsToShape",
        "sparql:bound",
        "sparql:coalesce",
        "custom list function",
        "custom named function",
    ] {
        assert!(
            tally.kinds.get(kind).copied().unwrap_or(0) > 0,
            "the generator never produced a {kind} expression: {:?}",
            tally.kinds
        );
    }
}

// ── The per-arm contract table ─────────────────────────────────────────────────

/// A shapes graph carrying one expression of every production arm.
const EVERY_ARM: &str = r#"
ex:Op a sh:NodeShape ; sh:nodeKind sh:IRI .
ex:userFn a sh:SPARQLFunction ;
    sh:returnType xsd:integer ;
    sh:select "SELECT (1 AS ?result) WHERE {}" .
ex:ident a sh:ListParameterExpressionFunction ;
    rdfs:subClassOf sh:ListParameterExpression ;
    sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ] ;
    sh:bodyExpression [ shnex:arg 0 ] .
ex:S a sh:NodeShape ;
    sh:expression ex:constant ,
        sh:this ,
        [ sh:path ex:p ] ,
        [ shnex:filterShape ex:Op ; shnex:nodes () ] ,
        [ sh:union ( 1 2 ) ] ,
        [ shnex:intersection ( 1 2 ) ] ,
        [ shnex:if true ; shnex:then 1 ; shnex:else 2 ] ,
        [ shnex:count ( 1 ) ] ,
        [ shnex:distinct ( 1 ) ] ,
        [ shnex:min ( 1 ) ] ,
        [ shnex:max ( 1 ) ] ,
        [ shnex:sum ( 1 ) ] ,
        [ shnex:limit 1 ; shnex:nodes ( 1 ) ] ,
        [ shnex:offset 1 ; shnex:nodes ( 1 ) ] ,
        [ shnex:orderBy sh:this ; shnex:nodes ( 1 ) ] ,
        [ shnex:exists ( 1 ) ] ,
        [ xsd:string ( 1 ) ] ,
        [ ex:userFn () ] ,
        [ sparql:abs ( 1 ) ] ,
        [ ex:ident ( 1 ) ] ,
        [] ,
        [ shnex:var "focusNode" ] ,
        ( 1 2 ) ,
        [ shnex:pathValues ex:p ; shnex:focusNode ex:a ] ,
        [ shnex:concat ( 1 2 ) ] ,
        [ shnex:nodes ( 1 ) ; shnex:remove ( 2 ) ] ,
        [ shnex:flatMap sh:this ; shnex:nodes ( 1 ) ] ,
        [ shnex:findFirst ex:Op ; shnex:nodes ( 1 ) ] ,
        [ shnex:matchAll ex:Op ; shnex:nodes ( 1 ) ] ,
        [ shnex:instancesOf ex:C ] ,
        [ shnex:nodesMatching ex:Op ] ,
        [ shnex:conformsToShape ( sh:this ex:Op ) ] ,
        [ sh:select "SELECT ?x WHERE { $this <http://example.org/ns#p> ?x }" ] .
"#;

/// Every production arm names the clause that defines its output, and declares
/// the contract that clause gives it. The expected table is written out here, arm
/// by arm, with the section it cites — a contract that moved, or a clause that
/// stopped citing its section, fails by name.
#[test]
fn every_arm_declares_the_contract_its_clause_defines() {
    use SequenceContract::{Multiset, OrderedSequence, Set};
    // (arm, expected contract, the section its clause must cite)
    let expected: BTreeMap<&str, (SequenceContract, &str)> = [
        ("Constant", (OrderedSequence, "§3.1.1")),
        ("This", (OrderedSequence, "Advanced Features §6.1")),
        ("Path", (Set, "§4.1.4")),
        ("Filter", (OrderedSequence, "§4.2.5")),
        ("Union", (Set, "Advanced Features §6.7")),
        ("Intersection", (Set, "§4.2.2")),
        ("If", (OrderedSequence, "§4.1.6")),
        ("Count", (OrderedSequence, "§4.4.1")),
        ("Distinct", (OrderedSequence, "§4.2.1")),
        ("Min", (OrderedSequence, "§4.4.2")),
        ("Max", (OrderedSequence, "§4.4.3")),
        ("Sum", (OrderedSequence, "§4.4.4")),
        ("Limit", (OrderedSequence, "§4.2.6")),
        ("Offset", (OrderedSequence, "§4.2.7")),
        ("OrderBy", (OrderedSequence, "§4.2.8")),
        ("Exists", (OrderedSequence, "§4.1.5")),
        ("Call::Builtin", (Set, "Advanced Features §6.4")),
        ("Call::UserDefined", (Set, "Advanced Features §6.4")),
        ("Call::Sparql", (OrderedSequence, "Node Expressions §5")),
        ("Arg", (OrderedSequence, "§6.3")),
        ("CustomCall", (OrderedSequence, "§6.1 / §6.2")),
        ("Empty", (OrderedSequence, "§4.1.1")),
        ("Var", (OrderedSequence, "§4.1.2")),
        ("List", (OrderedSequence, "§4.1.3")),
        ("PathValues", (Set, "§4.1.4")),
        ("Concat", (OrderedSequence, "§4.2.3")),
        ("Remove", (OrderedSequence, "§4.2.4")),
        ("FlatMap", (OrderedSequence, "§4.3.1")),
        ("FindFirst", (OrderedSequence, "§4.3.2")),
        ("MatchAll", (OrderedSequence, "§4.3.3")),
        ("InstancesOf", (Set, "§4.5.1")),
        ("NodesMatching", (Set, "§4.5.2")),
        ("ConformsToShape", (OrderedSequence, "§4.5.3")),
        ("Select", (Multiset, "SPARQL Extensions §6.1")),
    ]
    .into_iter()
    .collect();

    let shapes =
        parse_shapes(&format!("{PREFIXES}{EVERY_ARM}"), None).expect("the every-arm fixture loads");
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut stack: Vec<NodeExpr> = shapes
        .node_shapes
        .iter()
        .flat_map(|shape| &shape.constraints)
        .filter_map(|constraint| match constraint {
            Constraint::Expression { expr, .. } => Some(expr.clone()),
            _ => None,
        })
        .collect();
    while let Some(expr) = stack.pop() {
        // Exhaustive and wildcard-free: an arm added to `NodeExpr` fails to compile
        // here until the table says what its clause makes of it.
        let arm = match &expr {
            NodeExpr::Constant(_) => "Constant",
            NodeExpr::This => "This",
            NodeExpr::Path(_) => "Path",
            NodeExpr::Filter { .. } => "Filter",
            NodeExpr::Union(_) => "Union",
            NodeExpr::Intersection(_) => "Intersection",
            NodeExpr::If { .. } => "If",
            NodeExpr::Count { .. } => "Count",
            NodeExpr::Distinct(_) => "Distinct",
            NodeExpr::Min(_) => "Min",
            NodeExpr::Max(_) => "Max",
            NodeExpr::Sum(_) => "Sum",
            NodeExpr::Limit { .. } => "Limit",
            NodeExpr::Offset { .. } => "Offset",
            NodeExpr::OrderBy { .. } => "OrderBy",
            NodeExpr::Exists(_) => "Exists",
            NodeExpr::Call(FnCall::Builtin { .. }) => "Call::Builtin",
            NodeExpr::Call(FnCall::UserDefined { .. }) => "Call::UserDefined",
            NodeExpr::Call(FnCall::Sparql { .. }) => "Call::Sparql",
            NodeExpr::Arg(_) => "Arg",
            NodeExpr::CustomCall { func, .. } => {
                stack.push(func.body().expect("the body is installed").clone());
                "CustomCall"
            }
            NodeExpr::Empty => "Empty",
            NodeExpr::Var(_) => "Var",
            NodeExpr::List(_) => "List",
            NodeExpr::PathValues { .. } => "PathValues",
            NodeExpr::Concat(_) => "Concat",
            NodeExpr::Remove { .. } => "Remove",
            NodeExpr::FlatMap { .. } => "FlatMap",
            NodeExpr::FindFirst { .. } => "FindFirst",
            NodeExpr::MatchAll { .. } => "MatchAll",
            NodeExpr::InstancesOf(_) => "InstancesOf",
            NodeExpr::NodesMatching(_) => "NodesMatching",
            NodeExpr::ConformsToShape { shape, .. } => {
                assert!(matches!(shape, ShapeArg::Named(_)));
                "ConformsToShape"
            }
            NodeExpr::Select { .. } => "Select",
        };
        *seen.entry(arm).or_insert(0) += 1;
        let (contract, section) = expected[arm];
        assert_eq!(
            expr.sequence_contract(),
            contract,
            "{arm}: {}",
            expr.spec_clause()
        );
        assert!(
            expr.spec_clause().contains(section),
            "{arm} must cite {section}, got: {}",
            expr.spec_clause()
        );
    }
    let missing: Vec<&&str> = expected
        .keys()
        .filter(|arm| !seen.contains_key(*arm))
        .collect();
    assert!(
        missing.is_empty(),
        "the every-arm fixture reaches no expression of {missing:?}"
    );
}
