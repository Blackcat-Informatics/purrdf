// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Naming the ONE node expression a standalone evaluation runs — the selector every host
//! (the CLI's `node-expr`, Python's `eval_node_expr`, WASM's `shaclEvalNodeExpr`, the C
//! ABI's `purrdf_shacl_eval_node_expr`) hands to this boundary.
//!
//! SHACL 1.2 node expressions are overwhelmingly written as blank-node structures — the
//! object of `sh:values`, `sh:targetWhere`, `sh:expression`, an argument of another
//! expression — and a blank node written `[ … ]` has no label a caller could name. So a
//! selector names the expression one of three ways:
//!
//! * [`ExprSelector::Node`] — the node itself: an absolute IRI, or `_:label` for a blank
//!   node the shapes document labels so. An IRI that is the subject of no triple is a
//!   constant expression, evaluating to itself.
//! * [`ExprSelector::At`] — the node REACHED from a named node by a sequence of
//!   predicates: from `node`, each predicate in turn must reach exactly one value, and the
//!   last value is the expression. `ex:S` then `sh:values` names `ex:S`'s node expression;
//!   the W3C SHACL 1.2 `sht:EvalNodeExpr` entry `E` names its expression as `E`, then
//!   `mf:action`, then `sht:nodeExpr`. A step reaching no value or several is
//!   [`ExprSelectorError::NotOneValue`], naming the step and the count — never the first
//!   of several, and never an empty expression.
//! * [`ExprSelector::Turtle`] — the expression written INLINE as a Turtle document, read
//!   under the shapes document's base and prefixes (its own directives outrank them) and
//!   merged into the shapes graph, so its shape references and function calls bind exactly
//!   as they would inside the shapes graph. The expression is the fragment's ROOT: the one
//!   blank node that is the subject of a fragment triple and the object of none. No root,
//!   or several, is [`ExprSelectorError::NotOneRoot`], naming the count. The fragment's
//!   blank nodes are standardized apart from the shapes document's, so a label the two
//!   happen to share names two nodes, as two documents' blank nodes are.
//!
//! [`ExprSelector::parse`] decides every term the selector spells before any document is
//! read — a malformed term is the caller's argument's fault — and
//! [`ParsedExprSelector::select`] resolves it against the shapes graph.

use std::fmt;
use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm, RdfTriple};
use purrdf_shapes::ShapesError;
use purrdf_shapes::data::{GraphFilter, native_quads};
use purrdf_shapes::free_expression;
use purrdf_shapes::term::{NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_document;

/// How a host names the node expression to evaluate. See the [module docs](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprSelector<'a> {
    /// The expression node itself: an absolute IRI, or `_:label` for a blank node the
    /// shapes document labels so (see [`free_expression::parse_term`]).
    Node(&'a str),
    /// The node reached from `node` (an absolute IRI or `_:label`) by following `via`,
    /// one predicate (an absolute IRI) per step, each step reaching exactly one value.
    At {
        /// The named node the walk starts from.
        node: &'a str,
        /// The predicates to follow, in order; at least one.
        via: &'a [&'a str],
    },
    /// The expression written inline as a Turtle document whose one root blank node is
    /// the expression.
    Turtle(&'a str),
}

/// A selector whose terms are decided, ready to resolve against a shapes graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedExprSelector {
    kind: Parsed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Parsed {
    Node(Term),
    At { node: Term, via: Vec<NamedNode> },
    Turtle(String),
}

/// The expression a selector resolved to: the shapes graph it lives in — the host's own,
/// or, for an inline expression, the host's merged with the fragment — and its node.
#[derive(Debug, Clone)]
pub struct SelectedExpression {
    /// The shapes graph carrying the expression.
    pub shapes: Arc<RdfDataset>,
    /// The expression node in [`Self::shapes`].
    pub root: Term,
}

/// Why a selector names no single expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprSelectorError {
    /// A term the selector spells is not one: `role` names it (`expr`, `expr-at`,
    /// `expr-via 2`), `message` says why.
    Term {
        /// Which of the selector's terms.
        role: String,
        /// Why it is not a term.
        message: String,
    },
    /// The walk's start node is a literal or a triple term, which is the subject of no
    /// triple, so no predicate can reach anything from it.
    StartNotANode {
        /// The start node, as N-Triples.
        node: String,
    },
    /// A walk with no predicate, which would select the start node itself — name that
    /// with [`ExprSelector::Node`].
    EmptyPath,
    /// A walk predicate that is not an IRI.
    PredicateNotIri {
        /// The step, counted from 1.
        step: usize,
        /// The term given, as N-Triples.
        predicate: String,
    },
    /// A step reached no value or several.
    NotOneValue {
        /// The step, counted from 1.
        step: usize,
        /// The node the step started from, as N-Triples.
        subject: String,
        /// The step's predicate IRI.
        predicate: String,
        /// How many values the step reached.
        count: usize,
    },
    /// The inline expression is not a Turtle document.
    Turtle {
        /// The Turtle codec's diagnostics, one per malformed statement.
        errors: Vec<String>,
    },
    /// The inline expression has no root blank node, or several.
    NotOneRoot {
        /// How many roots the fragment has.
        count: usize,
    },
    /// A host named no selector, or several: [`ExprSelector::from_parts`] takes exactly
    /// one of the node, the walk and the inline expression.
    NotOneSelector {
        /// How many of the three were given.
        count: usize,
    },
    /// Walk predicates given with no walk start.
    ViaWithoutAt,
}

impl fmt::Display for ExprSelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Term { role, message } => write!(f, "{role}: {message}"),
            Self::StartNotANode { node } => write!(
                f,
                "expression selector: the walk starts from {node}, which is not an IRI or a \
                 blank node and is the subject of no triple; start from the node that \
                 carries the expression"
            ),
            Self::EmptyPath => f.write_str(
                "expression selector: a walk from a node names no predicate to follow; name \
                 the node itself as the expression instead",
            ),
            Self::PredicateNotIri { step, predicate } => write!(
                f,
                "expression selector: step {step}'s predicate {predicate} is not an IRI"
            ),
            Self::NotOneValue {
                step,
                subject,
                predicate,
                count,
            } => write!(
                f,
                "expression selector: step {step}, {subject} <{predicate}>, reaches {count} \
                 values in the shapes graph; each step must reach exactly one"
            ),
            Self::Turtle { errors } => write!(
                f,
                "expression selector: the inline expression is not a Turtle document: {}",
                errors.join("; ")
            ),
            Self::NotOneRoot { count } => write!(
                f,
                "expression selector: the inline expression has {count} root blank nodes \
                 (blank nodes that are the subject of a triple and the object of none); it \
                 must have exactly one, the expression. Name an IRI expression, or a \
                 constant, as the expression node itself"
            ),
            Self::NotOneSelector { count } => write!(
                f,
                "expression selector: {count} of the expression node, the walk start and the \
                 inline expression were given; name the expression exactly one way"
            ),
            Self::ViaWithoutAt => f.write_str(
                "expression selector: walk predicates were given with no walk start; name \
                 the node the walk starts from",
            ),
        }
    }
}

impl std::error::Error for ExprSelectorError {}

impl From<ExprSelectorError> for ShapesError {
    fn from(error: ExprSelectorError) -> Self {
        Self::Invalid(error.to_string())
    }
}

impl<'a> ExprSelector<'a> {
    /// The selector a host's optional arguments name: exactly one of `expr` (the node),
    /// `at` (a walk start, with `via` its predicates) and `turtle` (an inline expression).
    /// Every host binding with optional arguments maps them through this one function.
    ///
    /// # Errors
    ///
    /// [`ExprSelectorError::NotOneSelector`] when none or several of the three are given,
    /// and [`ExprSelectorError::ViaWithoutAt`] for predicates with no walk start.
    pub fn from_parts(
        expr: Option<&'a str>,
        at: Option<&'a str>,
        via: &'a [&'a str],
        turtle: Option<&'a str>,
    ) -> Result<Self, ExprSelectorError> {
        if at.is_none() && !via.is_empty() {
            return Err(ExprSelectorError::ViaWithoutAt);
        }
        match (expr, at, turtle) {
            (Some(expr), None, None) => Ok(Self::Node(expr)),
            (None, Some(node), None) => Ok(Self::At { node, via }),
            (None, None, Some(text)) => Ok(Self::Turtle(text)),
            _ => Err(ExprSelectorError::NotOneSelector {
                count: usize::from(expr.is_some())
                    + usize::from(at.is_some())
                    + usize::from(turtle.is_some()),
            }),
        }
    }

    /// Decide every term this selector spells, before any document is read.
    ///
    /// # Errors
    ///
    /// [`ExprSelectorError::Term`] for text that is not one term,
    /// [`ExprSelectorError::StartNotANode`] for a walk from a literal or a triple term,
    /// [`ExprSelectorError::EmptyPath`] for a walk with no predicate, and
    /// [`ExprSelectorError::PredicateNotIri`] for a predicate that is not an IRI.
    pub fn parse(&self) -> Result<ParsedExprSelector, ExprSelectorError> {
        let kind = match *self {
            Self::Node(text) => Parsed::Node(term("expr", text)?),
            Self::At { node, via } => {
                let start = term("expr-at", node)?;
                if !matches!(start, Term::NamedNode(_) | Term::BlankNode(_)) {
                    return Err(ExprSelectorError::StartNotANode {
                        node: start.to_string(),
                    });
                }
                if via.is_empty() {
                    return Err(ExprSelectorError::EmptyPath);
                }
                let via = via
                    .iter()
                    .enumerate()
                    .map(|(index, text)| {
                        let step = index + 1;
                        match term(&format!("expr-via {step}"), text)? {
                            Term::NamedNode(predicate) => Ok(predicate),
                            other => Err(ExprSelectorError::PredicateNotIri {
                                step,
                                predicate: other.to_string(),
                            }),
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Parsed::At { node: start, via }
            }
            Self::Turtle(text) => Parsed::Turtle(text.to_owned()),
        };
        Ok(ParsedExprSelector { kind })
    }
}

fn term(role: &str, text: &str) -> Result<Term, ExprSelectorError> {
    free_expression::parse_term(text).map_err(|message| ExprSelectorError::Term {
        role: role.to_owned(),
        message,
    })
}

impl ParsedExprSelector {
    /// Resolve this selector against the shapes graph `shapes`, whose document prefix map
    /// is `prefixes` and whose base is `base` — the two an inline expression is read
    /// under.
    ///
    /// # Errors
    ///
    /// [`ExprSelectorError::NotOneValue`] for a walk step reaching no value or several,
    /// [`ExprSelectorError::Turtle`] for an inline expression that does not parse, and
    /// [`ExprSelectorError::NotOneRoot`] for one without exactly one root.
    pub fn select(
        &self,
        shapes: &Arc<RdfDataset>,
        prefixes: &[(String, String)],
        base: Option<&str>,
    ) -> Result<SelectedExpression, ExprSelectorError> {
        match &self.kind {
            Parsed::Node(root) => Ok(SelectedExpression {
                shapes: Arc::clone(shapes),
                root: root.clone(),
            }),
            Parsed::At { node, via } => {
                let mut current = node.clone();
                for (index, predicate) in via.iter().enumerate() {
                    let values = native_quads(
                        shapes.as_ref(),
                        Some(&current),
                        Some(&Term::NamedNode(predicate.clone())),
                        None,
                        GraphFilter::AnyGraph,
                    );
                    let [(_, _, value)] = values.as_slice() else {
                        return Err(ExprSelectorError::NotOneValue {
                            step: index + 1,
                            subject: current.to_string(),
                            predicate: predicate.as_str().to_owned(),
                            count: values.len(),
                        });
                    };
                    current = value.clone();
                }
                Ok(SelectedExpression {
                    shapes: Arc::clone(shapes),
                    root: current,
                })
            }
            Parsed::Turtle(text) => inline(shapes, prefixes, base, text),
        }
    }
}

/// Read `text` under the shapes document's prefixes and base, find its one root, and merge
/// it into `shapes` with its blank nodes standardized apart.
fn inline(
    shapes: &Arc<RdfDataset>,
    prefixes: &[(String, String)],
    base: Option<&str>,
    text: &str,
) -> Result<SelectedExpression, ExprSelectorError> {
    // The shapes document's prefixes, declared ahead of the fragment: a directive the
    // fragment writes redeclares its label from that point on, as in any Turtle document.
    let mut document = String::new();
    for (label, namespace) in prefixes {
        document.push_str("@prefix ");
        document.push_str(label);
        document.push_str(": <");
        document.push_str(namespace);
        document.push_str("> .\n");
    }
    document.push_str(text);
    document.push('\n');
    let fragment = parse_turtle_document(&document, base)
        .map_err(|errors| ExprSelectorError::Turtle { errors })?
        .dataset;

    let quads: Vec<RdfQuad> = fragment.owned_quads().collect();
    let reifiers: Vec<_> = fragment.owned_reifiers().collect();
    let annotations: Vec<_> = fragment.owned_annotations().collect();

    // The fragment's triples as (subject, object) pairs — the asserted quads and the
    // reifier and annotation rows alike — and the roots among their blank subjects.
    let mut edges: Vec<(&RdfTerm, &RdfTerm)> = Vec::new();
    let mut reified: Vec<RdfTerm> = Vec::with_capacity(reifiers.len());
    for reifier in &reifiers {
        reified.push(RdfTerm::triple(reifier.statement.clone()));
    }
    for quad in &quads {
        edges.push((&quad.subject, &quad.object));
    }
    for (reifier, statement) in reifiers.iter().zip(&reified) {
        edges.push((&reifier.reifier, statement));
    }
    for annotation in &annotations {
        edges.push((&annotation.reifier, &annotation.object));
    }
    let mut roots: Vec<&str> = Vec::new();
    for (subject, _) in &edges {
        if let RdfTerm::BlankNode(label) = subject
            && !roots.contains(&label.as_str())
            && !edges.iter().any(|(_, object)| *object == *subject)
        {
            roots.push(label);
        }
    }
    let [root] = roots.as_slice() else {
        return Err(ExprSelectorError::NotOneRoot { count: roots.len() });
    };

    // Standardize the fragment's blank nodes apart: every label is prefixed with one no
    // shapes-graph blank label begins with, so no fragment node can land on a shapes node.
    let mut shapes_labels: Vec<String> = Vec::new();
    for quad in shapes.owned_quads() {
        collect_labels(&quad.subject, &mut shapes_labels);
        collect_labels(&quad.object, &mut shapes_labels);
        if let Some(graph) = &quad.graph_name {
            collect_labels(graph, &mut shapes_labels);
        }
    }
    for reifier in shapes.owned_reifiers() {
        collect_labels(&reifier.reifier, &mut shapes_labels);
        collect_labels(&RdfTerm::triple(reifier.statement), &mut shapes_labels);
    }
    for annotation in shapes.owned_annotations() {
        collect_labels(&annotation.reifier, &mut shapes_labels);
        collect_labels(&annotation.object, &mut shapes_labels);
    }
    let mut prefix = String::from("expr");
    while shapes_labels.iter().any(|label| label.starts_with(&prefix)) {
        prefix.push('_');
    }
    let rename = |term: &RdfTerm| relabel(term, &prefix);

    let mut builder = RdfDatasetBuilder::new();
    for quad in shapes.owned_quads() {
        builder.push_owned_quad(&quad);
    }
    for reifier in shapes.owned_reifiers() {
        builder.push_owned_reifier(&reifier);
    }
    for annotation in shapes.owned_annotations() {
        builder.push_owned_annotation(&annotation);
    }
    for graph in shapes.owned_named_graphs() {
        let id = builder.intern_owned_term_scoped(&graph, purrdf_core::BlankScope::DEFAULT);
        builder.declare_named_graph(id);
    }
    for quad in &quads {
        builder.push_owned_quad(&RdfQuad {
            subject: rename(&quad.subject),
            predicate: quad.predicate.clone(),
            object: rename(&quad.object),
            graph_name: quad.graph_name.as_ref().map(rename),
            location: None,
        });
    }
    for reifier in &reifiers {
        let mut renamed = reifier.clone();
        renamed.reifier = rename(&reifier.reifier);
        renamed.statement = relabel_triple(&reifier.statement, &prefix);
        renamed.graph = reifier.graph.as_ref().map(rename);
        renamed.location = None;
        builder.push_owned_reifier(&renamed);
    }
    for annotation in &annotations {
        let mut renamed = annotation.clone();
        renamed.reifier = rename(&annotation.reifier);
        renamed.object = rename(&annotation.object);
        renamed.graph = annotation.graph.as_ref().map(rename);
        renamed.location = None;
        builder.push_owned_annotation(&renamed);
    }
    let merged = builder
        .freeze()
        .map_err(|error| ExprSelectorError::Turtle {
            errors: vec![error.to_string()],
        })?;
    Ok(SelectedExpression {
        shapes: merged,
        root: Term::blank(format!("{prefix}{root}")),
    })
}

/// Every blank label `term` carries, triple terms included.
fn collect_labels(term: &RdfTerm, out: &mut Vec<String>) {
    match term {
        RdfTerm::BlankNode(label) => out.push(label.clone()),
        RdfTerm::Triple(triple) => {
            collect_labels(&triple.subject, out);
            collect_labels(&triple.object, out);
        }
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => {}
    }
}

/// `term` with every blank label prefixed by `prefix`.
fn relabel(term: &RdfTerm, prefix: &str) -> RdfTerm {
    match term {
        RdfTerm::BlankNode(label) => RdfTerm::BlankNode(format!("{prefix}{label}")),
        RdfTerm::Triple(triple) => RdfTerm::triple(relabel_triple(triple, prefix)),
        RdfTerm::Iri(_) | RdfTerm::Literal(_) => term.clone(),
    }
}

fn relabel_triple(triple: &RdfTriple, prefix: &str) -> RdfTriple {
    RdfTriple {
        subject: relabel(&triple.subject, prefix),
        predicate: triple.predicate.clone(),
        object: relabel(&triple.object, prefix),
        location: None,
    }
}
