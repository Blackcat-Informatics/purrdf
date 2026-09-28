// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What a parsed query did with its predicate IRIs: which became relation CALLS, and
//! which stayed ordinary triple patterns.
//!
//! # Why anyone needs to ask
//!
//! Whether a predicate IRI is a data edge or a call to a registered relation is not
//! a property of the query text — the same bytes mean either thing depending on the
//! [`ExtensionEnv`](crate::extension_env::ExtensionEnv) they are read against. That
//! is correct, and it is also invisible: a relation IRI the environment does not
//! recognize lowers to a perfectly ordinary triple pattern, matches whatever the
//! graph happens to hold, and answers. Nothing fails.
//!
//! So a host that has wired a relation and wants to know whether its queries will
//! actually reach it has no way to find out by running them: an empty answer is
//! exactly what a correctly-resolved relation over no matching rows returns. This
//! module answers the question directly, from the algebra the parse produced.
//!
//! # Read the ALGEBRA, not the text
//!
//! The classification is a walk over the parsed pattern rather than a scan of the
//! source, and that distinction is the whole point. Only the parser knows which
//! spellings reach predicate position at all: a variable predicate is never a call,
//! a property path is never a call, and an IRI inside a `VALUES` block or an
//! expression is not in predicate position to begin with. A text scan would report
//! every occurrence of an IRI and be wrong about all of those.

use std::collections::BTreeSet;

use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern, Query};

/// The predicate IRIs one parsed query used, split by what the parse made of them.
///
/// Both halves are IRI-sorted sets, so the report is a pure function of the algebra
/// rather than of traversal order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PredicateUse {
    /// Predicate IRIs the parser lowered to relation call nodes.
    ///
    /// Non-empty exactly when the environment this query was parsed against
    /// recognized the IRI — by exact registration or under a declared namespace.
    pub calls: BTreeSet<String>,
    /// Predicate IRIs that stayed ordinary triple patterns, matched against the data
    /// graph like any other edge.
    ///
    /// An IRI a host BELIEVES it registered appearing here is the answer to "why did
    /// my relation never run?".
    pub data: BTreeSet<String>,
}

impl PredicateUse {
    /// Whether this query reached no relation at all.
    #[must_use]
    pub fn calls_nothing(&self) -> bool {
        self.calls.is_empty()
    }
}

/// Classify every predicate IRI in `query`'s WHERE pattern.
///
/// A CONSTRUCT's template is deliberately not walked: a template writes triples, it
/// does not read them, so no predicate in it is ever a call.
#[must_use]
pub fn predicate_use(query: &Query) -> PredicateUse {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. } => pattern,
    };
    let mut out = PredicateUse::default();
    walk(pattern, &mut out);
    out
}

/// Walk `pattern` and every sub-pattern, recording what each leaf says.
///
/// The walk keeps a work list of the patterns and expressions still to be read, so a
/// query of any depth is classified without a machine-stack frame per level. Each
/// node's parts come from `visit_pattern_parts` and `visit_expression_parts`, the
/// crate's exhaustive shallow visitors — exhaustive being the load-bearing word: a
/// graph-pattern or expression variant added later fails to compile there rather
/// than being silently skipped here, which is the difference between a
/// classification that stays true and one that quietly stops covering a construct.
///
/// The three leaves that carry predicates are handled directly, because the visitor
/// treats them as leaves: it exists to propagate truncation, and a BGP is where
/// truncation originates rather than something it passes through.
///
/// # Expression-position patterns count
///
/// The pattern visitor yields two part kinds, and BOTH carry patterns. Following
/// only `Child` misses every pattern reachable only through an expression, and
/// `FILTER EXISTS { ?s <rel> ?o }` is exactly that shape: the inner pattern arrives
/// as `ExpressionPart::Exists`, at any expression nesting depth. A relation invoked
/// only inside a `FILTER EXISTS` would then report `calls_nothing() == true` — the
/// precise opposite of the fact this module exists to make visible, and a silent
/// one. The report is a pair of sorted sets, so the order the nodes are read in does
/// not reach it.
fn walk(pattern: &GraphPattern, out: &mut PredicateUse) {
    use crate::governor::soundness::{
        ExpressionPart, PatternPart, visit_expression_parts, visit_pattern_parts,
    };

    enum Node<'a> {
        Pattern(&'a GraphPattern),
        Expression(&'a purrdf_sparql_algebra::Expression),
    }

    let mut pending = vec![Node::Pattern(pattern)];
    while let Some(node) = pending.pop() {
        match node {
            Node::Pattern(GraphPattern::Bgp { patterns }) => {
                for triple in patterns {
                    record_data(&triple.predicate, out);
                }
            }
            Node::Pattern(GraphPattern::PropertyFunction(call)) => {
                out.calls.insert(call.iri.clone());
            }
            // A property path is never a call: the parser only lowers a BARE,
            // length-one predicate IRI to a call node, so anything that reached a
            // `Path` is a path operator and reads the graph.
            Node::Pattern(GraphPattern::Path { path, .. }) => record_path(path, out),
            Node::Pattern(pattern) => {
                visit_pattern_parts(pattern, &mut |part| {
                    pending.push(match part {
                        PatternPart::Child(child, _) => Node::Pattern(child),
                        PatternPart::Expression(expr) => Node::Expression(expr),
                    });
                    // `false` keeps the visit going; this walk has no early exit.
                    false
                });
            }
            Node::Expression(expr) => {
                visit_expression_parts(expr, &mut |part| {
                    match part {
                        ExpressionPart::Sub(inner) => pending.push(Node::Expression(inner)),
                        ExpressionPart::Exists(inner) => pending.push(Node::Pattern(inner)),
                        // A named function is not a pattern and carries none.
                        ExpressionPart::Call(_) => {}
                    }
                    false
                });
            }
        }
    }
}

/// Record a triple-pattern predicate, when it is an IRI rather than a variable.
fn record_data(predicate: &NamedNodePattern, out: &mut PredicateUse) {
    if let NamedNodePattern::NamedNode(node) = predicate {
        out.data.insert(node.as_str().to_owned());
    }
}

/// Record every IRI a property path names, all of them as data, over a work list of
/// the path's nodes.
fn record_path(path: &purrdf_sparql_algebra::PropertyPathExpression, out: &mut PredicateUse) {
    use purrdf_sparql_algebra::PropertyPathExpression as P;
    let mut pending = vec![path];
    while let Some(path) = pending.pop() {
        match path {
            P::NamedNode(node) => {
                out.data.insert(node.as_str().to_owned());
            }
            P::Reverse(inner)
            | P::ZeroOrMore(inner)
            | P::OneOrMore(inner)
            | P::ZeroOrOne(inner)
            | P::Range { inner, .. } => pending.push(inner),
            P::Sequence(elements) | P::Alternative(elements) => pending.extend(elements.iter()),
            // A negated set names the predicates it EXCLUDES. They are still predicate
            // IRIs the query mentions and still not calls, so they are reported as data —
            // a host asking "did my relation IRI become a call here?" gets the right
            // answer either way, and gets told the IRI was seen.
            P::NegatedPropertySet(elements) => {
                for element in elements {
                    out.data.insert(element.predicate.as_str().to_owned());
                }
            }
            // A wildcard names no predicate, so there is nothing to report — its
            // namespace bound is a restriction on matching, not an IRI the query uses.
            P::Wildcard { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::predicate_use;
    use purrdf_sparql_algebra::{ParserOptions, SparqlParser};

    const REL: &str = "http://example.org/rel/near";

    fn parse(text: &str, declared: &[&str]) -> purrdf_sparql_algebra::Query {
        let options = ParserOptions {
            property_fn_iris: declared.iter().map(|s| (*s).to_owned()).collect(),
            ..ParserOptions::default()
        };
        SparqlParser::new()
            .parse_query_with(text, &options)
            .expect("the fixture parses")
    }

    /// The same text, two environments, two answers — which is the fact this module
    /// exists to make visible.
    #[test]
    fn the_same_predicate_is_a_call_or_data_depending_on_the_environment() {
        let text = format!("SELECT ?s WHERE {{ ?s <{REL}> ?o }}");

        let recognized = predicate_use(&parse(&text, &[REL]));
        assert_eq!(recognized.calls.iter().collect::<Vec<_>>(), vec![REL]);
        assert!(
            recognized.data.is_empty(),
            "the predicate became a call, so it is not data: {recognized:?}"
        );

        let unrecognized = predicate_use(&parse(&text, &[]));
        assert!(
            unrecognized.calls_nothing(),
            "nothing was registered, so nothing is a call"
        );
        assert_eq!(unrecognized.data.iter().collect::<Vec<_>>(), vec![REL]);
    }

    /// A predicate nested inside OPTIONAL/UNION/FILTER-EXISTS is still found: the
    /// recursion is the crate's own exhaustive visitor, not a hand-listed set of
    /// places to look.
    #[test]
    fn a_call_nested_under_other_operators_is_still_found() {
        let text = format!(
            "SELECT ?s WHERE {{ ?s a <http://example.org/T> \
             OPTIONAL {{ {{ ?s <{REL}> ?o }} UNION {{ ?s <http://example.org/p> ?o }} }} }}"
        );
        let used = predicate_use(&parse(&text, &[REL]));
        assert_eq!(used.calls.iter().collect::<Vec<_>>(), vec![REL]);
        assert!(
            used.data.contains("http://example.org/p"),
            "the sibling UNION branch's ordinary predicate is data: {used:?}"
        );
    }

    /// A variable predicate is never a call and is not reported as data either —
    /// there is no IRI to report.
    #[test]
    fn a_variable_predicate_is_reported_as_neither() {
        let used = predicate_use(&parse("SELECT ?s WHERE { ?s ?p ?o }", &[REL]));
        assert!(used.calls_nothing());
        assert!(
            used.data.is_empty(),
            "no IRI appeared in predicate position"
        );
    }

    /// A relation invoked ONLY inside `FILTER EXISTS` is still a call.
    ///
    /// The inner pattern is reachable only through the filter expression, not as a
    /// child pattern, so a walk that followed children alone reported
    /// `calls_nothing() == true` here -- the exact opposite of the fact this module
    /// exists to surface, delivered silently. The module doc named FILTER-EXISTS
    /// while no test exercised it.
    #[test]
    fn a_call_reachable_only_through_filter_exists_is_still_found() {
        let text = format!(
            "SELECT ?s WHERE {{ ?s a <http://example.org/T> \
             FILTER EXISTS {{ ?s <{REL}> ?o }} }}"
        );
        let used = predicate_use(&parse(&text, &[REL]));
        assert_eq!(
            used.calls.iter().collect::<Vec<_>>(),
            vec![REL],
            "the relation is invoked inside the EXISTS body: {used:?}"
        );
    }

    /// And `NOT EXISTS`, which the algebra spells as a negated `Exists`, so it is
    /// reached one expression level deeper.
    #[test]
    fn a_call_inside_not_exists_is_found_too() {
        let text = format!(
            "SELECT ?s WHERE {{ ?s a <http://example.org/T> \
             FILTER NOT EXISTS {{ ?s <{REL}> ?o }} }}"
        );
        let used = predicate_use(&parse(&text, &[REL]));
        assert_eq!(used.calls.iter().collect::<Vec<_>>(), vec![REL]);
    }

    /// The valid neighbour: with nothing registered the same EXISTS body is ordinary
    /// data, so the new recursion did not turn every EXISTS predicate into a call.
    #[test]
    fn an_unregistered_iri_inside_filter_exists_is_data() {
        let text = format!(
            "SELECT ?s WHERE {{ ?s a <http://example.org/T> \
             FILTER EXISTS {{ ?s <{REL}> ?o }} }}"
        );
        let used = predicate_use(&parse(&text, &[]));
        assert!(used.calls_nothing());
        assert!(used.data.contains(REL), "it is reported, as data: {used:?}");
    }

    /// A property path names data, never a call: the parser lowers only a bare,
    /// length-one predicate IRI to a call node.
    #[test]
    fn a_property_path_names_data_even_when_its_iri_is_registered() {
        let text = format!("SELECT ?s WHERE {{ ?s <{REL}>* ?o }}");
        let used = predicate_use(&parse(&text, &[REL]));
        assert!(
            used.calls_nothing(),
            "a path is not a call however the IRI is configured: {used:?}"
        );
        assert_eq!(used.data.iter().collect::<Vec<_>>(), vec![REL]);
    }
}

/// The work-list walk against a recursive reading of the same algebra, over generated
/// shapes, and at a depth no recursive reading could reach on a small stack.
#[cfg(test)]
mod walk_tests {
    use purrdf_sparql_algebra::{
        Chain, Child, Expression, GraphPattern, NamedNode, NamedNodePattern, NegatedPathElement,
        PropertyFunctionCall, PropertyPathExpression as P, TermPattern, TriplePattern, Variable,
    };

    use super::{PredicateUse, record_data, record_path, walk};

    // ── The recursive reference ────────────────────────────────────────────────────

    fn reference_walk(pattern: &GraphPattern, out: &mut PredicateUse) {
        use crate::governor::soundness::{PatternPart, visit_pattern_parts};
        match pattern {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    record_data(&triple.predicate, out);
                }
                return;
            }
            GraphPattern::PropertyFunction(call) => {
                out.calls.insert(call.iri.clone());
                return;
            }
            GraphPattern::Path { path, .. } => {
                reference_record_path(path, out);
                return;
            }
            _ => {}
        }
        let mut pending: Vec<&GraphPattern> = Vec::new();
        visit_pattern_parts(pattern, &mut |part| {
            match part {
                PatternPart::Child(child, _) => pending.push(child),
                PatternPart::Expression(expr) => reference_collect_exists(expr, &mut pending),
            }
            false
        });
        for child in pending {
            reference_walk(child, out);
        }
    }

    fn reference_collect_exists<'a>(expr: &'a Expression, pending: &mut Vec<&'a GraphPattern>) {
        use crate::governor::soundness::{ExpressionPart, visit_expression_parts};
        visit_expression_parts(expr, &mut |part| {
            match part {
                ExpressionPart::Sub(inner) => reference_collect_exists(inner, pending),
                ExpressionPart::Exists(inner) => pending.push(inner),
                ExpressionPart::Call(_) => {}
            }
            false
        });
    }

    fn reference_record_path(path: &P, out: &mut PredicateUse) {
        match path {
            P::NamedNode(node) => {
                out.data.insert(node.as_str().to_owned());
            }
            P::Reverse(inner)
            | P::ZeroOrMore(inner)
            | P::OneOrMore(inner)
            | P::ZeroOrOne(inner) => {
                reference_record_path(inner, out);
            }
            P::Sequence(elements) | P::Alternative(elements) => {
                for element in elements {
                    reference_record_path(element, out);
                }
            }
            P::NegatedPropertySet(elements) => {
                for element in elements {
                    out.data.insert(element.predicate.as_str().to_owned());
                }
            }
            P::Range { inner, .. } => reference_record_path(inner, out),
            P::Wildcard { .. } => {}
        }
    }

    // ── A deterministic shape generator ────────────────────────────────────────────

    struct Choices {
        state: u64,
        budget: usize,
    }

    impl Choices {
        fn new(seed: u64) -> Self {
            Self {
                state: seed,
                budget: 40,
            }
        }

        fn choose(&mut self, options: usize) -> usize {
            let draw = purrdf_testkit::rng::splitmix64_next(&mut self.state);
            usize::try_from(draw % options as u64).expect("a choice fits usize")
        }

        fn spend(&mut self) -> bool {
            if self.budget == 0 {
                return false;
            }
            self.budget -= 1;
            true
        }
    }

    fn iri(n: usize) -> NamedNode {
        NamedNode::new_unchecked(format!("http://example.org/p{n}"))
    }

    fn predicate(choices: &mut Choices) -> NamedNodePattern {
        if choices.choose(4) == 0 {
            NamedNodePattern::Variable(Variable::new("p"))
        } else {
            NamedNodePattern::NamedNode(iri(choices.choose(6)))
        }
    }

    fn path(choices: &mut Choices) -> P {
        if !choices.spend() {
            return P::NamedNode(iri(choices.choose(6)));
        }
        match choices.choose(8) {
            0 | 1 => P::NamedNode(iri(choices.choose(6))),
            2 => P::Reverse(Child::new(path(choices))),
            3 => P::ZeroOrMore(Child::new(path(choices))),
            4 => P::Sequence(Chain::try_from(vec![path(choices), path(choices)]).expect("two")),
            5 => P::Alternative(Chain::try_from(vec![path(choices), path(choices)]).expect("two")),
            6 => P::NegatedPropertySet(vec![NegatedPathElement {
                predicate: iri(choices.choose(6)),
                inverse: choices.choose(2) == 0,
            }]),
            _ => P::Range {
                inner: Child::new(path(choices)),
                min: 1,
                max: Some(2),
            },
        }
    }

    fn bgp(choices: &mut Choices) -> GraphPattern {
        GraphPattern::Bgp {
            patterns: (0..=choices.choose(2))
                .map(|_| TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: predicate(choices),
                    object: TermPattern::Variable(Variable::new("o")),
                })
                .collect(),
        }
    }

    fn expression(choices: &mut Choices) -> Expression {
        if !choices.spend() {
            return Expression::Variable(Variable::new("v"));
        }
        match choices.choose(4) {
            0 => Expression::Not(Child::new(expression(choices))),
            1 => Expression::Or(
                Chain::try_from(vec![expression(choices), expression(choices)]).expect("two"),
            ),
            2 => Expression::FunctionCall(
                purrdf_sparql_algebra::Function::Str,
                vec![expression(choices)].into(),
            ),
            _ => Expression::Exists(Child::new(pattern(choices))),
        }
    }

    fn pattern(choices: &mut Choices) -> GraphPattern {
        if !choices.spend() {
            return bgp(choices);
        }
        match choices.choose(9) {
            0 | 1 => bgp(choices),
            2 => GraphPattern::PropertyFunction(PropertyFunctionCall {
                iri: format!("http://example.org/rel{}", choices.choose(3)),
                subject_args: vec![TermPattern::Variable(Variable::new("s"))],
                object_args: vec![TermPattern::Variable(Variable::new("o"))],
            }),
            3 => GraphPattern::Path {
                subject: TermPattern::Variable(Variable::new("s")),
                path: path(choices),
                object: TermPattern::Variable(Variable::new("o")),
            },
            4 => GraphPattern::Join {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            5 => GraphPattern::LeftJoin {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
                expression: Some(expression(choices)),
            },
            6 => GraphPattern::Filter {
                expr: expression(choices),
                inner: Child::new(pattern(choices)),
            },
            7 => GraphPattern::Union {
                arms: Chain::try_from(vec![pattern(choices), pattern(choices)]).expect("two"),
            },
            _ => GraphPattern::Extend {
                inner: Child::new(pattern(choices)),
                variable: Variable::new("x"),
                expression: expression(choices),
            },
        }
    }

    // ── The tests ──────────────────────────────────────────────────────────────────

    /// The work list reports exactly what the recursion reports, for every generated
    /// shape.
    #[test]
    fn the_walk_agrees_with_its_recursive_reference_on_generated_shapes() {
        let mut with_calls = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let shape = pattern(&mut choices);
            let mut ours = PredicateUse::default();
            walk(&shape, &mut ours);
            let mut expected = PredicateUse::default();
            reference_walk(&shape, &mut expected);
            assert_eq!(ours, expected, "seed {seed}: {shape:?}");
            with_calls += usize::from(!ours.calls_nothing());
        }
        assert!(
            with_calls > 40 && with_calls < 360,
            "the generator produces shapes with and without calls ({with_calls} of 400)"
        );
    }

    /// A call reached only through a hundred thousand nested `FILTER EXISTS` bodies,
    /// and a predicate named at the bottom of a hundred thousand path operators, are
    /// both reported on a 128 KiB stack.
    #[test]
    fn a_hundred_thousand_level_query_is_classified_on_a_128_kib_thread() {
        let used = std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let mut shape = GraphPattern::PropertyFunction(PropertyFunctionCall {
                    iri: "http://example.org/rel".to_owned(),
                    subject_args: vec![TermPattern::Variable(Variable::new("s"))],
                    object_args: vec![],
                });
                for _ in 0..100_000 {
                    shape = GraphPattern::Filter {
                        expr: Expression::Exists(Child::new(shape)),
                        inner: Child::new(GraphPattern::Bgp {
                            patterns: vec![TriplePattern {
                                subject: TermPattern::Variable(Variable::new("s")),
                                predicate: NamedNodePattern::NamedNode(iri(1)),
                                object: TermPattern::Variable(Variable::new("o")),
                            }],
                        }),
                    };
                }
                let mut deep_path = P::NamedNode(iri(2));
                for _ in 0..100_000 {
                    deep_path = P::Reverse(Child::new(deep_path));
                }
                let mut used = PredicateUse::default();
                walk(&shape, &mut used);
                record_path(&deep_path, &mut used);
                used
            })
            .expect("spawn")
            .join()
            .expect("the 128 KiB thread returned");
        assert_eq!(
            used.calls.iter().collect::<Vec<_>>(),
            vec!["http://example.org/rel"]
        );
        assert_eq!(
            used.data.iter().collect::<Vec<_>>(),
            vec!["http://example.org/p1", "http://example.org/p2"]
        );
    }
}
