// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL 1.2 RL rule dependencies, decided on the rule-set IR.
//!
//! SPARQL 1.2 RL §4.3: "Rule R1 depends on R2 if any triple pattern in the body of R1,
//! whether as a triple pattern element or inside a negation element, depends on a triple
//! template in the head of R2", where "A triple pattern matches a triple template if the
//! triple template can generate a triple that matches the triple pattern", "If a variable
//! is used more then once in a triple template or triple pattern, then the same RDF term
//! is used as the replacement", and the replacement reaches "variables inside triple
//! terms". The dependency is CLOSED when "A triple pattern occurring inside a negation
//! element of R1 matches a triple template in the rule head of R2" or "Rule R1 depends on
//! rule R2 and R1 is a run-once rule".
//!
//! The decision is made here rather than on the lowered clauses because a triple term
//! with variables is, in a clause, an opaque guard variable (the guard takes the term
//! apart or builds it), so a clause-level matcher can only say "anything": a head
//! `?x :p <<( ?a :b :c )>>` would depend-match a negated `?x :p <<( :x :y :z )>>` and
//! refuse a stratifiable rule set. Here both are structured terms and are unified
//! position by position, nested triple terms included, so a dependency is found exactly
//! when a substitution exists. The graph is then stratified by
//! [`purrdf_datalog::schedule::stratify_dependency_graph`], the one stratifier.
//!
//! # Patterns that match the base graph only
//!
//! A pattern of a `WHERE DATA` body or of a `NOT DATA` element is matched against the
//! base graph alone — SPARQL 1.2 RL §6.4 evaluates a `rule.data` body as
//! `evalRuleElements(B, SEQ0, GD, GD)` and a `DATA` negation's inner body against `GD`,
//! with §6.5 passing `G0`, "the input base graph", as `GD` — so no rule's output can
//! affect it, and it induces no dependency: §4.3 "A rule R1 depends on a rule R2 if the
//! output of the second rule affects the evaluation of the body of the first rule". The
//! suite's `eval2/eval-dft-value-neg-01` — a run-once rule whose `NOT DATA` element reads
//! the predicate its head writes — is required to evaluate, and would be a closed
//! self-dependency otherwise.
//!
//! # SHACL rules
//!
//! A SHACL rule is opaque to this analysis: it reads the whole evaluation graph with
//! unknown polarity, so it depends, closed, on every rule with a head, and its head may
//! generate any triple.

use std::collections::BTreeMap;

use purrdf_datalog::schedule::{DependencyGraph, Schedule, stratify_dependency_graph};
use purrdf_datalog::seminaive::EvalError;

use super::ir::{Element, IrRule, IrRuleBody, PatternTerm, RuleSet, TriplePattern};
use crate::term::Term;

/// The dependency graph of `set`.
pub(crate) fn dependency_graph(set: &RuleSet<'_>) -> DependencyGraph {
    let mut graph = DependencyGraph::new(set.rules.len());
    let heads: Vec<Option<&[TriplePattern]>> = set.rules.iter().map(head).collect();
    for (r1, rule) in set.rules.iter().enumerate() {
        let run_once = rule.is_run_once();
        match &rule.body {
            IrRuleBody::Shacl(_) => {
                for (r2, other) in set.rules.iter().enumerate() {
                    if has_head(other) {
                        graph.depend(r1, r2, true);
                    }
                }
            }
            IrRuleBody::Elements(element_rule) => {
                if element_rule.data {
                    continue;
                }
                let reads = body_reads(&element_rule.body);
                for (r2, other) in set.rules.iter().enumerate() {
                    for (pattern, negated) in &reads {
                        let generated = match heads[r2] {
                            Some(templates) => templates
                                .iter()
                                .any(|template| may_generate(pattern, template)),
                            None => has_head(other),
                        };
                        if generated {
                            graph.depend(r1, r2, *negated || run_once);
                        }
                    }
                }
            }
        }
    }
    graph
}

/// Stratify `set` by its dependency graph.
pub(crate) fn stratify(set: &RuleSet<'_>) -> Result<Schedule, EvalError> {
    let run_once: Vec<bool> = set.rules.iter().map(IrRule::is_run_once).collect();
    stratify_dependency_graph(&dependency_graph(set), &run_once)
}

/// An element rule's head templates; `None` for a SHACL rule, whose head is opaque.
fn head<'r>(rule: &'r IrRule<'_>) -> Option<&'r [TriplePattern]> {
    match &rule.body {
        IrRuleBody::Elements(element_rule) => Some(&element_rule.head),
        IrRuleBody::Shacl(_) => None,
    }
}

/// Whether a rule can generate any triple at all.
fn has_head(rule: &IrRule<'_>) -> bool {
    match &rule.body {
        IrRuleBody::Elements(element_rule) => !element_rule.head.is_empty(),
        IrRuleBody::Shacl(_) => true,
    }
}

/// The body patterns a rule's output can affect, each with whether it is inside a
/// negation element: `NOT DATA` elements match the base graph alone and are skipped.
fn body_reads(elements: &[Element]) -> Vec<(&TriplePattern, bool)> {
    let mut reads = Vec::new();
    for element in elements {
        match element {
            Element::Pattern(pattern) => reads.push((pattern, false)),
            Element::Negation { elements, data } => {
                if !*data {
                    for inner in elements {
                        if let Element::Pattern(pattern) = inner {
                            reads.push((pattern, true));
                        }
                    }
                }
            }
            Element::Filter(_) | Element::Assign { .. } => {}
        }
    }
    reads
}

/// Whether the template `template` can generate a triple the pattern `pattern` matches.
pub(crate) fn may_generate(pattern: &TriplePattern, template: &TriplePattern) -> bool {
    let mut unifier = Unifier::default();
    pattern
        .positions()
        .into_iter()
        .zip(template.positions())
        .all(|(left, right)| {
            let a = unifier.node(Side::Pattern, left);
            let b = unifier.node(Side::Template, right);
            unifier.unify(&a, &b)
        })
}

/// Which side of the match a variable belongs to: the two are separate namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Side {
    /// The body pattern.
    Pattern,
    /// The head template.
    Template,
}

/// A term under unification.
#[derive(Debug, Clone)]
enum Node {
    /// A variable (a body blank node is one too), by index.
    Var(usize),
    /// A constant term, a ground triple term included.
    Const(Term),
    /// A head blank node: a term no pattern constant can equal, since it is fresh.
    Fresh(String),
    /// A triple term with at least one variable inside.
    Triple(Box<[Self; 3]>),
}

/// A substitution over the two sides' variables.
#[derive(Debug, Default)]
struct Unifier {
    /// Each variable's binding, if any.
    bindings: Vec<Option<Node>>,
    /// The index of each `(side, name)` variable.
    names: BTreeMap<(Side, String), usize>,
}

impl Unifier {
    /// The node of one position.
    fn node(&mut self, side: Side, term: &PatternTerm) -> Node {
        match term {
            PatternTerm::Variable(name) => self.var(side, format!("?{name}")),
            PatternTerm::BlankNode(label) => match side {
                Side::Pattern => self.var(side, format!("_:{label}")),
                Side::Template => Node::Fresh(label.clone()),
            },
            PatternTerm::Term(term) => Node::Const(term.clone()),
            PatternTerm::Triple(inner) => Node::Triple(Box::new(
                inner.positions().map(|position| self.node(side, position)),
            )),
        }
    }

    fn var(&mut self, side: Side, name: String) -> Node {
        let next = self.bindings.len();
        let index = *self.names.entry((side, name)).or_insert(next);
        if index == next {
            self.bindings.push(None);
        }
        Node::Var(index)
    }

    /// Follow variable bindings to a representative.
    fn walk(&self, node: &Node) -> Node {
        let mut node = node.clone();
        while let Node::Var(index) = node {
            match &self.bindings[index] {
                Some(bound) => node = bound.clone(),
                None => return Node::Var(index),
            }
        }
        node
    }

    /// Whether variable `index` occurs in `node`.
    fn occurs(&self, index: usize, node: &Node) -> bool {
        match self.walk(node) {
            Node::Var(other) => other == index,
            Node::Triple(parts) => parts.iter().any(|part| self.occurs(index, part)),
            Node::Const(_) | Node::Fresh(_) => false,
        }
    }

    /// Unify two nodes, extending the substitution; `false` when no substitution exists.
    fn unify(&mut self, a: &Node, b: &Node) -> bool {
        let (a, b) = (self.walk(a), self.walk(b));
        match (&a, &b) {
            (Node::Var(x), Node::Var(y)) if x == y => true,
            (Node::Var(x), other) | (other, Node::Var(x)) => {
                if self.occurs(*x, other) {
                    return false;
                }
                self.bindings[*x] = Some(other.clone());
                true
            }
            (Node::Const(x), Node::Const(y)) => x == y,
            (Node::Fresh(x), Node::Fresh(y)) => x == y,
            (Node::Triple(x), Node::Triple(y)) => {
                let (x, y) = (x.clone(), y.clone());
                x.iter().zip(y.iter()).all(|(l, r)| self.unify(l, r))
            }
            (Node::Const(Term::Triple(ground)), Node::Triple(parts))
            | (Node::Triple(parts), Node::Const(Term::Triple(ground))) => {
                let parts = parts.clone();
                let ground = [
                    Node::Const(ground.subject.clone()),
                    Node::Const(Term::NamedNode(ground.predicate.clone())),
                    Node::Const(ground.object.clone()),
                ];
                parts
                    .iter()
                    .zip(ground.iter())
                    .all(|(l, r)| self.unify(l, r))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::{NamedNode, Triple};

    fn var(name: &str) -> PatternTerm {
        PatternTerm::Variable(name.to_owned())
    }

    fn iri(local: &str) -> Term {
        Term::NamedNode(NamedNode::from(
            format!("http://example.org/{local}").as_str(),
        ))
    }

    fn c(local: &str) -> PatternTerm {
        PatternTerm::Term(iri(local))
    }

    fn t(s: PatternTerm, p: PatternTerm, o: PatternTerm) -> TriplePattern {
        TriplePattern::new(s, p, o)
    }

    fn triple_term(s: PatternTerm, p: PatternTerm, o: PatternTerm) -> PatternTerm {
        PatternTerm::Triple(Box::new(t(s, p, o)))
    }

    #[test]
    fn matching_unifies_through_triple_terms() {
        // Head `?x :p <<( ?a :b :c )>>` against negated `?y :p <<( :x :y :z )>>`: the
        // predicates inside differ, so no triple the head generates matches.
        let template = t(var("x"), c("p"), triple_term(var("a"), c("b"), c("c")));
        let ground = PatternTerm::Term(Term::Triple(Box::new(Triple {
            subject: iri("x"),
            predicate: NamedNode::from("http://example.org/y"),
            object: iri("z"),
        })));
        assert!(!may_generate(&t(var("y"), c("p"), ground), &template));
        // The neighbour whose inner predicate agrees does match.
        let ground = PatternTerm::Term(Term::Triple(Box::new(Triple {
            subject: iri("x"),
            predicate: NamedNode::from("http://example.org/b"),
            object: iri("c"),
        })));
        assert!(may_generate(&t(var("y"), c("p"), ground), &template));
        // A repeated pattern variable must take one value on both sides.
        assert!(!may_generate(
            &t(var("v"), c("p"), var("v")),
            &t(c("a"), c("p"), c("b"))
        ));
        assert!(may_generate(
            &t(var("v"), c("p"), var("v")),
            &t(var("s"), c("p"), c("b"))
        ));
        // A fresh head blank node equals no constant, but binds a variable.
        let fresh = PatternTerm::BlankNode("n".to_owned());
        assert!(!may_generate(
            &t(c("a"), c("q"), c("b")),
            &t(fresh.clone(), c("q"), c("b"))
        ));
        assert!(may_generate(
            &t(var("s"), c("q"), c("b")),
            &t(fresh, c("q"), c("b"))
        ));
        // A variable cannot equal a triple term containing it.
        assert!(!may_generate(
            &t(var("v"), c("p"), triple_term(var("v"), c("q"), c("r"))),
            &t(var("w"), c("p"), var("w"))
        ));
    }
}
