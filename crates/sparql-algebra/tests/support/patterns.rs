// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Algebra built by hand for the query front end's integration tests: names,
//! variables, a one-triple basic graph pattern, and the `WHERE` body of a parsed
//! query.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_sparql_algebra::{
    GraphPattern, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
};

/// `<http://example.org/{local}>`.
pub fn example(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

/// The variable `?{name}`.
pub fn var(name: &str) -> Variable {
    Variable::new(name)
}

/// The basic graph pattern of the one triple `?subject <predicate> ?object`.
pub fn triple_bgp(subject: &str, predicate: NamedNode, object: &str) -> GraphPattern {
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(var(subject)),
            predicate: NamedNodePattern::NamedNode(predicate),
            object: TermPattern::Variable(var(object)),
        }],
    }
}

/// A parsed query's `WHERE` body: the pattern under its projection, or the
/// pattern itself when there is none.
pub fn where_body(pattern: &GraphPattern) -> GraphPattern {
    match pattern {
        GraphPattern::Project { inner, .. } => (**inner).clone(),
        other => other.clone(),
    }
}
