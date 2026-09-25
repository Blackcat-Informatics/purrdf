// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Which implementation every node-expression FUNCTION CALL in a shapes graph
//! binds to — asked and answered before any validation runs.
//!
//! # The question this exists to answer
//!
//! A shapes graph that merges the W3C SHACL 1.2 vocabularies declares every
//! built-in function, and may declare custom ones beside them. Loading it resolves
//! each call site against the spec symbol table ([`crate::spec`]); the answer is
//! not visible in a validation report, where a call bound to the wrong
//! implementation and a call bound to the right one can produce the same verdict.
//! This report names, per call site, what the call reached:
//!
//! * [`FunctionBinding::Native`] — a built-in the engine implements: a
//!   `sparql:<NAME>` call, an XPath `fn:` call the engine lowers to SPARQL,
//!   `shnex:conformsToShape`, or a SPARQL-based `sh:select` / `sh:sparqlExpr`
//!   expression;
//! * [`FunctionBinding::Custom`] — a custom node-expression function the shapes
//!   graph declares with a `sh:bodyExpression`;
//! * [`FunctionBinding::SparqlRegistered`] — a `sh:SPARQLFunction` the shapes graph
//!   declares with a SPARQL body;
//! * [`FunctionBinding::HostExtension`] — an IRI the shapes graph does not declare,
//!   resolved at evaluation against the function registry the host installed.
//!
//! Structural node expressions (paths, counts, filters, conditionals, …) are not
//! calls and are not listed; the functions inside them are.

use std::collections::BTreeSet;

use crate::expression::{FnCall, NodeExpr, ShapeArg};
use crate::model::{sh, shnex};
use crate::rules::RuleBody;
use crate::shapes::{Constraint, PropertyShape, Shape, Shapes, Target};

/// What a call site binds to. See the [module docs](self).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionBinding {
    /// A built-in the engine implements.
    Native,
    /// A custom node-expression function with a `sh:bodyExpression`.
    Custom,
    /// A `sh:SPARQLFunction` with a SPARQL body.
    SparqlRegistered,
    /// An IRI the shapes graph does not declare, resolved against the host's
    /// function registry at evaluation.
    HostExtension,
}

/// One call site: where it is, which function it calls, and what that bound to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallSite {
    /// The construct the call sits in, e.g. `sh:expression on <http://…/S>`.
    pub owner: String,
    /// The called function's IRI.
    pub function: String,
    /// What the call bound to.
    pub binding: FunctionBinding,
}

/// Every call site of a shapes graph, sorted and de-duplicated, so the report is
/// a pure function of the shapes graph rather than of traversal order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FunctionResolution {
    sites: BTreeSet<CallSite>,
}

impl FunctionResolution {
    /// Every call site, in order.
    pub fn sites(&self) -> impl Iterator<Item = &CallSite> {
        self.sites.iter()
    }

    /// The distinct bindings `function` has anywhere in the graph.
    #[must_use]
    pub fn bindings_of(&self, function: &str) -> BTreeSet<FunctionBinding> {
        self.sites
            .iter()
            .filter(|site| site.function == function)
            .map(|site| site.binding)
            .collect()
    }

    /// Whether the graph calls no function at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }
}

impl Shapes {
    /// Which implementation every node-expression function call in this shapes
    /// graph binds to. See the [module docs](crate::function_resolution).
    #[must_use]
    pub fn function_resolution(&self) -> FunctionResolution {
        let mut walk = Walk {
            shapes: self,
            out: FunctionResolution::default(),
            seen_shapes: BTreeSet::new(),
            fns_in_flight: BTreeSet::new(),
        };
        for shape in &self.node_shapes {
            walk.shape(shape);
        }
        walk.out
    }
}

/// The walk, with the same two guards `extension_usage` documents: shapes are a
/// visited set (a shape's calls are recorded under its own id), custom-function
/// bodies are a stack (the owner label embeds the caller).
struct Walk<'a> {
    shapes: &'a Shapes,
    out: FunctionResolution,
    seen_shapes: BTreeSet<String>,
    fns_in_flight: BTreeSet<String>,
}

impl Walk<'_> {
    fn record(&mut self, owner: &str, function: &str, binding: FunctionBinding) {
        self.out.sites.insert(CallSite {
            owner: owner.to_owned(),
            function: function.to_owned(),
            binding,
        });
    }

    fn shape(&mut self, shape: &Shape) {
        let id = shape.id.to_string();
        if !self.seen_shapes.insert(id.clone()) {
            return;
        }
        for rule in &shape.rules {
            if let RuleBody::Triple {
                subject,
                predicate,
                object,
            } = &rule.body
            {
                let owner = format!("sh:rule on {id}");
                self.node_expr(subject, &owner);
                self.node_expr(predicate, &owner);
                self.node_expr(object, &owner);
            }
            for condition in &rule.conditions {
                self.shape(condition);
            }
        }
        self.targets(&shape.targets, &id);
        self.constraints(&shape.constraints, &id);
        for property in &shape.property_shapes {
            self.property(property);
        }
    }

    fn property(&mut self, property: &PropertyShape) {
        let id = property.id.to_string();
        if let Some(expr) = &property.values {
            self.node_expr(expr, &format!("sh:values on {id}"));
        }
        if let Some(expr) = &property.default_value {
            self.node_expr(expr, &format!("sh:defaultValue on {id}"));
        }
        self.constraints(&property.constraints, &id);
        for nested in &property.property_shapes {
            self.property(nested);
        }
        for reifier in &property.reifier_shapes {
            self.shape(reifier);
        }
    }

    fn constraints(&mut self, constraints: &[Constraint], owner: &str) {
        for constraint in constraints {
            match constraint {
                Constraint::Expression { expr, .. } => {
                    self.node_expr(expr, &format!("sh:expression on {owner}"));
                }
                Constraint::NodeByExpression { expr, .. } => {
                    self.node_expr(expr, &format!("sh:nodeByExpression on {owner}"));
                }
                Constraint::Not(shape)
                | Constraint::Node(shape)
                | Constraint::MemberShape(shape)
                | Constraint::SomeValue(shape) => {
                    self.shape(shape);
                }
                Constraint::And(shapes) | Constraint::Or(shapes) | Constraint::Xone(shapes) => {
                    for shape in shapes {
                        self.shape(shape);
                    }
                }
                Constraint::QualifiedValueShape {
                    shape, siblings, ..
                } => {
                    self.shape(shape);
                    for sibling in siblings {
                        self.shape(sibling);
                    }
                }
                // No node expression and no nested shape.
                Constraint::Class(_)
                | Constraint::Datatype(_)
                | Constraint::NodeKind(_)
                | Constraint::MinCount(_)
                | Constraint::MaxCount(_)
                | Constraint::In(_)
                | Constraint::HasValue(_)
                | Constraint::Pattern { .. }
                | Constraint::MinLength(_)
                | Constraint::MaxLength(_)
                | Constraint::UniqueLang(_)
                | Constraint::LanguageIn(_)
                | Constraint::Closed { .. }
                | Constraint::MinInclusive(_)
                | Constraint::MaxInclusive(_)
                | Constraint::MinExclusive(_)
                | Constraint::MaxExclusive(_)
                | Constraint::Sparql { .. }
                | Constraint::Equals(_)
                | Constraint::Disjoint(_)
                | Constraint::SubsetOf(_)
                | Constraint::LessThan(_)
                | Constraint::LessThanOrEquals(_)
                | Constraint::MinListLength(_)
                | Constraint::MaxListLength(_)
                | Constraint::UniqueMembers(_)
                | Constraint::SingleLine(_)
                | Constraint::RootClass(_)
                | Constraint::Component { .. } => {}
                Constraint::UniqueValuesFor { targets, .. } => self.targets(targets, owner),
            }
        }
    }

    /// The call sites a shape's target declarations carry: a structured
    /// `sh:targetNode` is a node expression, and a `sh:targetWhere` shape is
    /// walked as any nested shape is. Wildcard-free, as the constraint walk is.
    fn targets(&mut self, targets: &[Target], owner: &str) {
        for target in targets {
            match target {
                Target::NodeExpression(expr) => {
                    self.node_expr(expr, &format!("sh:targetNode on {owner}"));
                }
                Target::Where(shape) => self.shape(shape),
                Target::Class(_)
                | Target::SubjectsOf(_)
                | Target::ObjectsOf(_)
                | Target::Node(_)
                | Target::ImplicitClass(_)
                | Target::Sparql { .. } => {}
            }
        }
    }

    /// The binding a non-`sparql:`, non-custom call IRI reaches.
    fn call_binding(&self, iri: &str) -> FunctionBinding {
        if crate::expression::builtin_keyword(iri).is_some() {
            return FunctionBinding::Native;
        }
        let functions = &self.shapes.functions;
        if functions.resolve(iri).is_some() {
            FunctionBinding::SparqlRegistered
        } else if functions.resolve_expr(iri).is_some() {
            FunctionBinding::Custom
        } else {
            FunctionBinding::HostExtension
        }
    }

    fn node_expr(&mut self, expr: &NodeExpr, owner: &str) {
        match expr {
            NodeExpr::Call(FnCall::Sparql { iri, args, .. }) => {
                self.record(owner, iri.as_str(), FunctionBinding::Native);
                self.exprs(args, owner);
            }
            NodeExpr::Call(FnCall::Builtin { iri, args } | FnCall::UserDefined { iri, args }) => {
                let binding = self.call_binding(iri.as_str());
                self.record(owner, iri.as_str(), binding);
                self.exprs(args, owner);
            }
            NodeExpr::CustomCall { func, args } => {
                let iri = func.iri.as_str().to_owned();
                self.record(owner, &iri, FunctionBinding::Custom);
                for (_, arg) in args {
                    self.node_expr(arg, owner);
                }
                if self.fns_in_flight.insert(iri.clone()) {
                    if let Some(body) = func.body.get() {
                        self.node_expr(body, &format!("{owner} via <{iri}>"));
                    }
                    self.fns_in_flight.remove(&iri);
                }
            }
            NodeExpr::ConformsToShape { node, shape } => {
                self.record(owner, shnex::CONFORMS_TO_SHAPE, FunctionBinding::Native);
                self.node_expr(node, owner);
                match shape {
                    ShapeArg::Named(shape) => self.shape(shape),
                    ShapeArg::Computed { expr, .. } => self.node_expr(expr, owner),
                }
            }
            NodeExpr::Select { key, .. } => {
                let function = if *key == "sh:select" {
                    sh::SELECT_EXPRESSION
                } else {
                    sh::SPARQL_EXPR_EXPRESSION
                };
                self.record(owner, function, FunctionBinding::Native);
            }
            NodeExpr::Union(items) | NodeExpr::Intersection(items) | NodeExpr::Concat(items) => {
                self.exprs(items, owner);
            }
            NodeExpr::If { cond, then, els } => {
                self.node_expr(cond, owner);
                self.node_expr(then, owner);
                self.node_expr(els, owner);
            }
            NodeExpr::Count { of, .. }
            | NodeExpr::Distinct(of)
            | NodeExpr::Min(of)
            | NodeExpr::Max(of)
            | NodeExpr::Sum(of)
            | NodeExpr::Limit { of, .. }
            | NodeExpr::Offset { of, .. }
            | NodeExpr::InstancesOf(of)
            | NodeExpr::Exists(of) => self.node_expr(of, owner),
            NodeExpr::OrderBy { of, key, .. } => {
                self.node_expr(of, owner);
                self.node_expr(key, owner);
            }
            NodeExpr::Filter { nodes, shape }
            | NodeExpr::FindFirst { nodes, shape }
            | NodeExpr::MatchAll { nodes, shape } => {
                self.node_expr(nodes, owner);
                self.shape(shape);
            }
            NodeExpr::Remove { nodes, remove } => {
                self.node_expr(nodes, owner);
                self.node_expr(remove, owner);
            }
            NodeExpr::FlatMap { nodes, map } => {
                self.node_expr(nodes, owner);
                self.node_expr(map, owner);
            }
            NodeExpr::PathValues { focus, .. } => self.node_expr(focus, owner),
            NodeExpr::NodesMatching(shape) => self.shape(shape),
            // Leaves: no call and no nested expression.
            NodeExpr::Constant(_)
            | NodeExpr::This
            | NodeExpr::Path(_)
            | NodeExpr::Empty
            | NodeExpr::Var(_)
            | NodeExpr::Arg(_)
            | NodeExpr::List(_) => {}
        }
    }

    fn exprs(&mut self, exprs: &[NodeExpr], owner: &str) {
        for expr in exprs {
            self.node_expr(expr, owner);
        }
    }
}
