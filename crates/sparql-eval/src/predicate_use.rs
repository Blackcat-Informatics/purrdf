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
/// Recursion comes from `visit_pattern_parts`, the crate's exhaustive shallow
/// visitor — exhaustive being the load-bearing word: a graph-pattern variant added
/// later fails to compile there rather than being silently skipped here, which is
/// the difference between a classification that stays true and one that quietly
/// stops covering a construct.
///
/// The two leaves that carry predicates are handled directly, because the visitor
/// treats both as leaves: it exists to propagate truncation, and a BGP is where
/// truncation originates rather than something it passes through.
///
/// # Expression-position patterns count
///
/// The visitor yields two part kinds, and BOTH carry patterns. Following only
/// `Child` misses every pattern reachable only through an expression, and
/// `FILTER EXISTS { ?s <rel> ?o }` is exactly that shape: the inner pattern arrives
/// as `ExpressionPart::Exists`. A relation invoked only inside a `FILTER EXISTS`
/// would then report `calls_nothing() == true` — the precise opposite of the fact
/// this module exists to make visible, and a silent one.
fn walk(pattern: &GraphPattern, out: &mut PredicateUse) {
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
        // A property path is never a call: the parser only lowers a BARE,
        // length-one predicate IRI to a call node, so anything that reached a
        // `Path` is a path operator and reads the graph.
        GraphPattern::Path { path, .. } => {
            record_path(path, out);
            return;
        }
        _ => {}
    }

    use crate::governor::soundness::{PatternPart, visit_pattern_parts};

    let mut pending: Vec<&GraphPattern> = Vec::new();
    visit_pattern_parts(pattern, &mut |part| {
        match part {
            PatternPart::Child(child, _) => pending.push(child),
            PatternPart::Expression(expr) => collect_exists_patterns(expr, &mut pending),
        }
        // `false` keeps the visit going; this walk has no early exit.
        false
    });
    for child in pending {
        walk(child, out);
    }
}

/// Collect every pattern reachable through `expr` — the `EXISTS`/`NOT EXISTS` bodies,
/// at any nesting depth.
///
/// Recursion again comes from the crate's own exhaustive expression visitor, for the
/// same reason the pattern walk uses its counterpart: an expression variant added
/// later fails to compile there rather than silently dropping a pattern here.
fn collect_exists_patterns<'a>(
    expr: &'a purrdf_sparql_algebra::Expression,
    pending: &mut Vec<&'a GraphPattern>,
) {
    use crate::governor::soundness::{ExpressionPart, visit_expression_parts};

    visit_expression_parts(expr, &mut |part| {
        match part {
            ExpressionPart::Sub(inner) => collect_exists_patterns(inner, pending),
            ExpressionPart::Exists(inner) => pending.push(inner),
            // A named function is not a pattern and carries none.
            ExpressionPart::Call(_) => {}
        }
        false
    });
}

/// Record a triple-pattern predicate, when it is an IRI rather than a variable.
fn record_data(predicate: &NamedNodePattern, out: &mut PredicateUse) {
    if let NamedNodePattern::NamedNode(node) = predicate {
        out.data.insert(node.as_str().to_owned());
    }
}

/// Record every IRI a property path names, all of them as data.
fn record_path(path: &purrdf_sparql_algebra::PropertyPathExpression, out: &mut PredicateUse) {
    use purrdf_sparql_algebra::PropertyPathExpression as P;
    match path {
        P::NamedNode(node) => {
            out.data.insert(node.as_str().to_owned());
        }
        P::Reverse(inner) | P::ZeroOrMore(inner) | P::OneOrMore(inner) | P::ZeroOrOne(inner) => {
            record_path(inner, out);
        }
        P::Sequence(left, right) | P::Alternative(left, right) => {
            record_path(left, out);
            record_path(right, out);
        }
        // A negated set names the predicates it EXCLUDES. They are still predicate
        // IRIs the query mentions and still not calls, so they are reported as data —
        // a host asking "did my relation IRI become a call here?" gets the right
        // answer either way, and gets told the IRI was seen.
        P::NegatedPropertySet(elements) => {
            for element in elements {
                out.data.insert(element.predicate.as_str().to_owned());
            }
        }
        P::Range { inner, .. } => record_path(inner, out),
        // A wildcard names no predicate, so there is nothing to report — its
        // namespace bound is a restriction on matching, not an IRI the query uses.
        P::Wildcard { .. } => {}
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
