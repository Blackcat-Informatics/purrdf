// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What one execution of a SHACL shape rule can read of the evaluation graph: the
//! read set the delta-driven re-execution of an iterating shape rule is decided by
//! ([`super::eval`], "Re-executing a shape rule").
//!
//! A shape rule is executed once per focus node, and within one layer the evaluation
//! graph only grows. An execution's answer is a function of its focus node, its rule
//! constants and the evaluation graph — so if no triple the execution could read has
//! been added since the focus node was last executed, executing it again yields the
//! triples it yielded then, which are all in the graph already. The read set is how the
//! engine tells: one [`ReadPattern`] per triple pattern, property path or node-expression
//! path the rule reads through, with the positions its constants and its focus node fix.
//! A new triple can change the answer for focus node `x` only if it matches one of the
//! patterns with the focus position read as `x`; a pattern with no focus position — a
//! join step away from the focus node — can change the answer for every focus node.
//! A rule whose answer can differ between two executions over one graph is
//! [`RuleReads::Volatile`] and is always executed for every focus node — exactly the
//! SHACL execution, with nothing skipped.
//!
//! Every classification errs toward reading MORE: a position the analysis cannot pin is
//! [`Anchor::Any`], a predicate it cannot bound is `None`, and a construct whose answer it
//! cannot prove repeatable makes the rule volatile. Each can only cost re-executions;
//! none can skip one whose answer could have changed.
//!
//! The focus position is sound because SHACL pre-binding is substitution — the query is
//! evaluated with every `$this` replaced by the focus node — and the pre-binding
//! restrictions a shape rule's query is checked against at load ([`crate::rules`])
//! exclude the constructs under which a pattern mentioning `$this` could match a triple
//! about another node: `MINUS`, a `VALUES` or `AS` binding `$this`, a subquery that does
//! not project it.

use std::collections::BTreeSet;

use purrdf_sparql_algebra::{
    AggregateFunction, Expression, Flow, Function, GraphPattern, NamedNodePattern, NodeRef,
    ParserOptions, PropertyPathExpression, Query, SparqlParser, TermPattern, TriplePattern, Visit,
    walk_pre_post,
};

use crate::expression::NodeExpr;
use crate::rules::{Rule, RuleBody};
use crate::shapes::Path;

/// One position of a [`ReadPattern`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Anchor {
    /// Any term.
    Any,
    /// The focus node the execution runs for.
    Focus,
    /// This IRI.
    Iri(String),
}

/// A triple shape one execution reads through: a new triple matching it, with
/// [`Anchor::Focus`] read as the execution's focus node, may change the answer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReadPattern {
    /// The subject position.
    pub(crate) subject: Anchor,
    /// The predicate IRI, or `None` for any predicate.
    pub(crate) predicate: Option<String>,
    /// The object position.
    pub(crate) object: Anchor,
}

impl ReadPattern {
    /// Any triple with predicate `predicate` (any predicate when `None`), wherever it
    /// sits relative to the focus node.
    pub(crate) fn unanchored(predicate: Option<&str>) -> Self {
        Self {
            subject: Anchor::Any,
            predicate: predicate.map(ToOwned::to_owned),
            object: Anchor::Any,
        }
    }
}

/// What one execution of a shape rule can read of the evaluation graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuleReads {
    /// Two executions over one graph may answer differently — the rule mints fresh
    /// blank nodes, calls `RAND()`, `NOW()`, `UUID()` or `STRUUID()`, selects solutions
    /// by an order the data does not fix (`LIMIT`/`OFFSET`, `SAMPLE`, `GROUP_CONCAT`,
    /// `FOLD`, an ordered node-expression sequence), or calls a function whose body the
    /// analysis does not read. Such a rule is executed for every focus node, every time.
    Volatile,
    /// The triple shapes an execution reads through, deduplicated and sorted.
    Patterns(Vec<ReadPattern>),
}

/// The accumulating read set of one rule.
#[derive(Default)]
struct Reads {
    /// Some construct makes the rule's answer unrepeatable.
    volatile: bool,
    /// The triple shapes read through.
    patterns: BTreeSet<ReadPattern>,
}

/// The anchor of a SPARQL pattern term: the pre-bound `$this`, an IRI constant, or any
/// term (a variable, a blank node, a literal, a triple term).
fn sparql_anchor(term: &TermPattern) -> Anchor {
    match term {
        TermPattern::Variable(variable) if variable.as_str() == "this" => Anchor::Focus,
        TermPattern::NamedNode(iri) => Anchor::Iri(iri.as_str().to_owned()),
        TermPattern::Variable(_)
        | TermPattern::BlankNode(_)
        | TermPattern::Literal(_)
        | TermPattern::Triple(_) => Anchor::Any,
    }
}

impl Reads {
    /// The classification the accumulated read set denotes.
    fn finish(self) -> RuleReads {
        if self.volatile {
            RuleReads::Volatile
        } else {
            RuleReads::Patterns(self.patterns.into_iter().collect())
        }
    }

    /// Every triple of the graph.
    fn any(&mut self) {
        self.patterns.insert(ReadPattern::unanchored(None));
    }

    /// Any triple with predicate `predicate`.
    fn predicate(&mut self, predicate: &str) {
        self.patterns
            .insert(ReadPattern::unanchored(Some(predicate)));
    }

    /// A triple pattern.
    fn triple(&mut self, triple: &TriplePattern) {
        self.patterns.insert(ReadPattern {
            subject: sparql_anchor(&triple.subject),
            predicate: match &triple.predicate {
                NamedNodePattern::NamedNode(p) => Some(p.as_str().to_owned()),
                NamedNodePattern::Variable(_) => None,
            },
            object: sparql_anchor(&triple.object),
        });
    }

    /// A graph pattern: every triple shape it reads through.
    fn pattern(&mut self, pattern: &GraphPattern) {
        self.sparql(NodeRef::Pattern(pattern));
    }

    /// Every node of the SPARQL tree under `root`, over [`walk_pre_post`]'s work list:
    /// a basic graph pattern reads its triples, a property path every step it takes,
    /// `EXISTS` reads its pattern, and a slice, an order-dependent aggregate or a
    /// function call may make the rule volatile.
    fn sparql(&mut self, root: NodeRef<'_>) {
        walk_pre_post(root, |step, node| {
            if step == Visit::Exit {
                return Flow::Descend;
            }
            match node {
                NodeRef::Pattern(pattern) => self.pattern_node(pattern),
                NodeRef::Path(path) => self.path_node(path),
                NodeRef::Expr(expression) => {
                    if let Expression::FunctionCall(function, _) = expression {
                        self.function(function);
                    }
                    Flow::Descend
                }
                NodeRef::Aggregate(aggregate) => {
                    match aggregate.function() {
                        AggregateFunction::Count
                        | AggregateFunction::Sum
                        | AggregateFunction::Avg
                        | AggregateFunction::Min
                        | AggregateFunction::Max => {}
                        AggregateFunction::Sample
                        | AggregateFunction::GroupConcat
                        | AggregateFunction::Fold
                        | AggregateFunction::Custom(_) => self.volatile = true,
                    }
                    Flow::Descend
                }
                NodeRef::Order(_) => Flow::Descend,
                // A triple is read where its basic graph pattern is entered; the terms
                // of a path's ends and of a quoted triple read nothing themselves.
                NodeRef::Triple(_) | NodeRef::Term(_) | NodeRef::Ground(_) => Flow::Skip,
            }
        });
    }

    /// One graph-pattern node's own reads; its operands are walked after it.
    fn pattern_node(&mut self, pattern: &GraphPattern) -> Flow {
        match pattern {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    self.triple(triple);
                }
                Flow::Skip
            }
            // Which solutions a slice keeps depends on an order the data need not fix.
            GraphPattern::Slice { .. } => {
                self.volatile = true;
                Flow::Descend
            }
            GraphPattern::Values { .. } => Flow::Skip,
            // Contextual assignment does not carry the ordinary SHACL focus
            // substitution proof. Inspect its reads, but never reuse this rule
            // from an analysis that assumes every `$this` remains the focus.
            GraphPattern::Apply { .. } => {
                self.volatile = true;
                Flow::Descend
            }
            GraphPattern::Service { .. } | GraphPattern::PropertyFunction(_) => {
                self.any();
                Flow::Skip
            }
            GraphPattern::Path { .. }
            | GraphPattern::Join { .. }
            | GraphPattern::Lateral { .. }
            | GraphPattern::Union { .. }
            | GraphPattern::Minus { .. }
            | GraphPattern::LeftJoin { .. }
            | GraphPattern::Filter { .. }
            | GraphPattern::Graph { .. }
            | GraphPattern::Project { .. }
            | GraphPattern::Distinct { .. }
            | GraphPattern::Reduced { .. }
            | GraphPattern::Extend { .. }
            | GraphPattern::Unfold { .. }
            | GraphPattern::OrderBy { .. }
            | GraphPattern::Group { .. } => Flow::Descend,
        }
    }

    /// One property-path node's own reads: every step it takes, anywhere in the graph.
    fn path_node(&mut self, path: &PropertyPathExpression) -> Flow {
        match path {
            PropertyPathExpression::NamedNode(p) => {
                self.predicate(p.as_str());
                Flow::Skip
            }
            PropertyPathExpression::Reverse(_)
            | PropertyPathExpression::OneOrMore(_)
            | PropertyPathExpression::Sequence(_)
            | PropertyPathExpression::Alternative(_) => Flow::Descend,
            // A zero-length step matches every node of the graph when neither end is
            // bound, so it reads the graph's nodes, whatever their predicates.
            PropertyPathExpression::ZeroOrMore(_) | PropertyPathExpression::ZeroOrOne(_) => {
                self.any();
                Flow::Skip
            }
            PropertyPathExpression::Range { min, .. } => {
                if *min == 0 {
                    self.any();
                }
                Flow::Descend
            }
            PropertyPathExpression::NegatedPropertySet(_)
            | PropertyPathExpression::Wildcard { .. } => {
                self.any();
                Flow::Skip
            }
        }
    }

    /// A function call makes the rule volatile when its answer is fresh on every call
    /// or comes from caller code the analysis does not read.
    fn function(&mut self, function: &Function) {
        match function {
            // Fresh on every call.
            Function::BNode
            | Function::Rand
            | Function::Now
            | Function::Uuid
            | Function::StrUuid
            // Caller code the analysis does not read: a `sh:SPARQLFunction`
            // body, an extension function reading lists or standpoints (and
            // minting fresh lists).
            | Function::Custom(_)
            | Function::Purrdf(_) => self.volatile = true,
            _ => {}
        }
    }

    /// A SHACL path walked from the nodes of a node expression. `from_focus` says those
    /// nodes are exactly the focus node, so the path's first step is anchored at it.
    fn shacl_path(&mut self, path: &Path, from_focus: bool) {
        match path {
            Path::Predicate(p) if from_focus => {
                self.patterns.insert(ReadPattern {
                    subject: Anchor::Focus,
                    predicate: Some(p.as_str().to_owned()),
                    object: Anchor::Any,
                });
            }
            Path::Inverse(inner) if from_focus && matches!(**inner, Path::Predicate(_)) => {
                let Path::Predicate(p) = &**inner else {
                    return;
                };
                self.patterns.insert(ReadPattern {
                    subject: Anchor::Any,
                    predicate: Some(p.as_str().to_owned()),
                    object: Anchor::Focus,
                });
            }
            Path::Predicate(p) => self.predicate(p.as_str()),
            // A zero-length step from a node yields that node, so it reads nothing.
            Path::Inverse(inner)
            | Path::ZeroOrMore(inner)
            | Path::OneOrMore(inner)
            | Path::ZeroOrOne(inner) => self.shacl_path(inner, false),
            Path::Sequence(steps) | Path::Alternative(steps) => {
                for step in steps {
                    self.shacl_path(step, false);
                }
            }
        }
    }

    /// A node expression, recursively. `at_focus` says the expression is evaluated at
    /// the rule's focus node (rather than at the nodes of an enclosing expression).
    fn node_expression(&mut self, expression: &NodeExpr, at_focus: bool) {
        match expression {
            NodeExpr::Constant(_)
            | NodeExpr::This
            | NodeExpr::Empty
            | NodeExpr::List(_)
            | NodeExpr::Var(_) => {}
            NodeExpr::Path(path) => self.shacl_path(path, at_focus),
            NodeExpr::PathValues { path, focus } => {
                self.shacl_path(path, at_focus && matches!(**focus, NodeExpr::This));
                self.node_expression(focus, at_focus);
            }
            NodeExpr::Union(list) | NodeExpr::Intersection(list) | NodeExpr::Concat(list) => {
                for item in list {
                    self.node_expression(item, at_focus);
                }
            }
            NodeExpr::Count { of, .. }
            | NodeExpr::Distinct(of)
            | NodeExpr::Min(of)
            | NodeExpr::Max(of)
            | NodeExpr::Sum(of)
            | NodeExpr::Exists(of) => self.node_expression(of, at_focus),
            NodeExpr::Remove { nodes, remove } => {
                self.node_expression(nodes, at_focus);
                self.node_expression(remove, at_focus);
            }
            NodeExpr::FlatMap { nodes, map } => {
                self.node_expression(nodes, at_focus);
                self.node_expression(map, false);
            }
            NodeExpr::If { cond, then, els } => {
                self.node_expression(cond, at_focus);
                self.node_expression(then, at_focus);
                self.node_expression(els, at_focus);
            }
            // A sequence cut or ordered by position depends on an order the data need
            // not fix.
            NodeExpr::Limit { of, .. } | NodeExpr::Offset { of, .. } => {
                self.volatile = true;
                self.node_expression(of, at_focus);
            }
            NodeExpr::OrderBy { of, key, .. } => {
                self.volatile = true;
                self.node_expression(of, at_focus);
                self.node_expression(key, false);
            }
            // Shape conformance and SHACL instance membership read whatever the shape's
            // constraints and the class hierarchy read.
            NodeExpr::Filter { nodes, .. }
            | NodeExpr::FindFirst { nodes, .. }
            | NodeExpr::MatchAll { nodes, .. }
            | NodeExpr::InstancesOf(nodes) => {
                self.any();
                self.node_expression(nodes, at_focus);
            }
            NodeExpr::NodesMatching(_) | NodeExpr::ConformsToShape { .. } => self.any(),
            // Caller code the analysis does not read.
            NodeExpr::Call(_)
            | NodeExpr::CustomCall { .. }
            | NodeExpr::Arg(_)
            | NodeExpr::Select { .. } => self.volatile = true,
        }
    }
}

/// Whether a CONSTRUCT template term mints a blank node per solution: it is or quotes
/// a blank node, found over [`walk_pre_post`]'s work list.
fn mints_blank(term: &TermPattern) -> bool {
    !walk_pre_post(NodeRef::Term(term), |_, node| {
        if matches!(node, NodeRef::Term(TermPattern::BlankNode(_))) {
            Flow::Stop
        } else {
            Flow::Descend
        }
    })
}

/// The read set of one execution of shape rule `rule` for one focus node.
///
/// A SPARQL rule is parsed under the parser options in scope, exactly as its execution
/// parses it; a text that does not parse is [`RuleReads::Volatile`], and its execution
/// reports the error. A property-function registry in scope can turn any triple pattern
/// into caller code, so under one every SPARQL rule reads every triple.
pub(crate) fn rule_reads(rule: &Rule) -> RuleReads {
    let mut reads = Reads::default();
    match &rule.body {
        RuleBody::Triple {
            subject,
            predicate,
            object,
        } => {
            for expression in [subject, predicate, object].into_iter().flatten() {
                reads.node_expression(expression, true);
            }
        }
        RuleBody::Sparql { construct, .. } => {
            let options: std::sync::Arc<ParserOptions> =
                crate::sparql::current_parser_options().unwrap_or_default();
            let parsed = SparqlParser::new().parse_query_with(construct, &options);
            let Ok(Query::Construct {
                template,
                pattern,
                dataset,
                ..
            }) = parsed
            else {
                return RuleReads::Volatile;
            };
            if template
                .iter()
                .any(|quad| mints_blank(&quad.triple.subject) || mints_blank(&quad.triple.object))
            {
                reads.volatile = true;
            }
            if !dataset.default.is_empty()
                || !dataset.named.is_empty()
                || crate::sparql::current_property_functions()
                    .is_some_and(|registry| !registry.is_empty())
            {
                reads.any();
            }
            reads.pattern(&pattern);
        }
    }
    reads.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::{NamedNode, Term};

    fn rule(body: RuleBody) -> Rule {
        Rule {
            id: Term::NamedNode(NamedNode::from("http://example.org/r")),
            body,
            conditions: Vec::new(),
            layer: None,
            order: None,
            run_once: false,
            deactivated: false,
            expected_predicates: Vec::new(),
            processors: Vec::new(),
        }
    }

    fn sparql(construct: &str) -> RuleReads {
        rule_reads(&rule(RuleBody::Sparql {
            construct: construct.to_owned(),
            parameters: Vec::new(),
        }))
    }

    fn iri(local: &str) -> String {
        format!("http://example.org/{local}")
    }

    fn pattern(subject: Anchor, predicate: Option<&str>, object: Anchor) -> ReadPattern {
        ReadPattern {
            subject,
            predicate: predicate.map(iri),
            object,
        }
    }

    #[test]
    fn contextual_application_reads_both_sides_without_focus_reuse_certification() {
        use purrdf_sparql_algebra::algebra::ApplicationPolicy;
        use purrdf_sparql_algebra::tree::Child;

        let Query::Ask { pattern: left, .. } = SparqlParser::new()
            .parse_query("ASK { ?this <http://example.org/left> ?o }")
            .unwrap()
        else {
            panic!("ASK fixture")
        };
        let Query::Ask { pattern: right, .. } = SparqlParser::new()
            .parse_query("ASK { FILTER EXISTS { ?s <http://example.org/right> ?o } }")
            .unwrap()
        else {
            panic!("ASK fixture")
        };
        let application = GraphPattern::Apply {
            left: Child::new(left),
            right: Child::new(right),
            policy: Box::new(ApplicationPolicy {
                dataset_required: false,
                row_pipeline: false,
                reduced_adjacent: false,
                group_domain: None,
                inputs: Vec::new(),
                optional: None,
            }),
        };
        let mut reads = Reads::default();
        reads.pattern(&application);
        assert_eq!(reads.patterns.len(), 2);
        assert_eq!(reads.finish(), RuleReads::Volatile);
    }

    #[test]
    fn a_focus_anchored_pattern_keeps_its_anchor() {
        assert_eq!(
            sparql(
                "PREFIX ex: <http://example.org/> CONSTRUCT { ?n a ex:C } \
                 WHERE { $this a ex:C BIND (IRI(CONCAT(STR($this), '0')) AS ?n) }"
            ),
            RuleReads::Patterns(vec![ReadPattern {
                subject: Anchor::Focus,
                predicate: Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_owned()),
                object: Anchor::Iri(iri("C")),
            }])
        );
    }

    #[test]
    fn a_join_away_from_the_focus_is_unanchored() {
        let RuleReads::Patterns(patterns) = sparql(
            "PREFIX ex: <http://example.org/> CONSTRUCT { $this ex:q ?o } \
             WHERE { $this ex:p ?o FILTER NOT EXISTS { ?o ex:r/^ex:s ?x } }",
        ) else {
            panic!("a repeatable query is not volatile");
        };
        assert_eq!(
            patterns,
            vec![
                pattern(Anchor::Any, Some("r"), Anchor::Any),
                pattern(Anchor::Any, Some("s"), Anchor::Any),
                pattern(Anchor::Focus, Some("p"), Anchor::Any),
            ]
        );
    }

    #[test]
    fn unbounded_reads_read_every_triple() {
        for query in [
            "CONSTRUCT { $this <http://example.org/q> ?o } WHERE { ?s ?p ?o }",
            "CONSTRUCT { $this <http://example.org/q> ?o } WHERE { $this <http://example.org/p>* ?o }",
            "CONSTRUCT { $this <http://example.org/q> ?o } WHERE { $this !<http://example.org/p> ?o }",
        ] {
            let RuleReads::Patterns(patterns) = sparql(query) else {
                panic!("{query} is repeatable");
            };
            assert!(
                patterns.contains(&ReadPattern::unanchored(None)),
                "{query}: {patterns:?}"
            );
        }
    }

    #[test]
    fn unrepeatable_answers_are_volatile() {
        for query in [
            "CONSTRUCT { $this <http://example.org/q> [] } WHERE { $this <http://example.org/p> ?o }",
            "CONSTRUCT { $this <http://example.org/q> ?b } WHERE { $this <http://example.org/p> ?o BIND (BNODE() AS ?b) }",
            "CONSTRUCT { $this <http://example.org/q> ?r } WHERE { $this <http://example.org/p> ?o BIND (RAND() AS ?r) }",
            "CONSTRUCT { $this <http://example.org/q> ?o } WHERE { { SELECT $this ?o WHERE { $this <http://example.org/p> ?o } LIMIT 1 } }",
            "CONSTRUCT { $this <http://example.org/q> ?c } WHERE { { SELECT $this (GROUP_CONCAT(?o) AS ?c) WHERE { $this <http://example.org/p> ?o } GROUP BY $this } }",
            "CONSTRUCT { $this <http://example.org/q> ?c } WHERE { $this <http://example.org/p> ?o BIND (<http://example.org/f>(?o) AS ?c) }",
            "not a query",
        ] {
            assert_eq!(sparql(query), RuleReads::Volatile, "{query}");
        }
    }

    #[test]
    fn a_triple_rule_path_from_the_focus_is_anchored() {
        let reads = rule_reads(&rule(RuleBody::Triple {
            subject: None,
            predicate: Some(NodeExpr::Constant(Term::NamedNode(NamedNode::from(
                "http://example.org/q",
            )))),
            object: Some(NodeExpr::Path(Path::Sequence(vec![
                Path::Predicate(NamedNode::from("http://example.org/p")),
                Path::Predicate(NamedNode::from("http://example.org/r")),
            ]))),
        }));
        assert_eq!(
            reads,
            RuleReads::Patterns(vec![
                pattern(Anchor::Any, Some("p"), Anchor::Any),
                pattern(Anchor::Any, Some("r"), Anchor::Any),
            ])
        );
        let reads = rule_reads(&rule(RuleBody::Triple {
            subject: None,
            predicate: Some(NodeExpr::Constant(Term::NamedNode(NamedNode::from(
                "http://example.org/q",
            )))),
            object: Some(NodeExpr::Path(Path::Inverse(Box::new(Path::Predicate(
                NamedNode::from("http://example.org/p"),
            ))))),
        }));
        assert_eq!(
            reads,
            RuleReads::Patterns(vec![pattern(Anchor::Any, Some("p"), Anchor::Focus)])
        );
    }
}
