// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Variable **pre-binding** by algebra rewrite (purrdf S5, EPIC #906 GAP-A).
//!
//! This is the native replacement for the oxigraph-family
//! `PreparedSparqlQuery::substitute_variable`, which SHACL-AF uses to inject the
//! focus node into a `sh:SPARQLConstraint` / `sh:SPARQLTarget` query as `$this`.
//!
//! ## Semantics: substitution = pre-binding (NOT term replacement)
//!
//! `substitute_variable(var, value)` pre-binds `var` to `value` **before**
//! evaluation, exactly as if the query's `WHERE` had been joined with a single-row
//! `VALUES { ?var value }`. This:
//!
//! * propagates into `OPTIONAL`/`MINUS`/`EXISTS`/sub-queries by ordinary
//!   correlation (the join sits beneath them in the algebra, so any occurrence of
//!   `var` is constrained), and
//! * keeps `var` **projectable** — a `SELECT ?var …` still emits the column,
//!   because the join is injected *below* the projection/solution-modifier stack,
//!   not by deleting `var` from the head.
//!
//! It is therefore implemented as the injection of a single-row [`GraphPattern::Values`]
//! `JOIN`ed onto the **core** `WHERE` pattern — the pattern reached by descending
//! through the outer solution-modifier/filter wrappers (`Project`, `Distinct`,
//! `Reduced`, `Slice`, `OrderBy`, `Group`, `Extend`, `Filter`). Injecting at the
//! core (rather than at the very root) is what makes the binding visible to
//! `FILTER`/`ORDER BY`/`GROUP BY`/`DISTINCT` evaluation *and* to the projected
//! variable list simultaneously.
//!
//! ## Blank-node focus nodes
//!
//! SHACL focus nodes *can* be blank nodes, and the SPARQL grammar forbids a blank in
//! a real `VALUES` cell. Rather than forking the substitution path, the algebra
//! carries an **injection-only** [`GroundTerm::BlankNode`] variant the parser never
//! produces (see its docs): the single-row `VALUES`-join rewrite is then **uniform**
//! across every focus-node kind (IRI, literal, ground triple, AND blank). The
//! evaluator interns the injected blank as an ordinary `purrdf-core`
//! `TermValue::Blank` via the normal `VALUES` evaluation path, so blank focus nodes
//! are pre-bound with exactly the same below-the-modifiers semantics as the others —
//! no focus-node kind is dropped, and the rewrite stays a pure algebra transform
//! (`purrdf-sparql-algebra` remains oxigraph-free and `TermValue`-free).

use crate::algebra::{GraphPattern, Query};
use crate::ast::{GroundTerm, Variable};

impl Query {
    /// Pre-bind `var` to `value` by injecting a single-row `VALUES { ?var value }`
    /// `JOIN` at the core `WHERE` pattern (see the module docs for the exact
    /// semantics). The query head (`SELECT`/`CONSTRUCT`/`DESCRIBE`/`ASK`) and every
    /// solution modifier are preserved; only the `WHERE` algebra is rewritten.
    ///
    /// `value` may be any [`GroundTerm`] — IRI, literal, ground triple, or the
    /// injection-only [`GroundTerm::BlankNode`] (so a blank-node focus node is
    /// pre-bound through the identical rewrite).
    #[must_use]
    pub fn substitute_variable(self, var: &Variable, value: GroundTerm) -> Query {
        let seed = GraphPattern::Values {
            variables: vec![var.clone()],
            bindings: vec![vec![Some(value)]],
        };
        self.map_core_pattern(|core| GraphPattern::Join {
            left: Box::new(seed),
            right: Box::new(core),
        })
    }

    /// Replace the **core** `WHERE` pattern — the one reached by descending through
    /// the outer solution-modifier wrappers — with `f(core)`, reattaching the
    /// modifier stack unchanged on the way back up.
    ///
    /// This is the structural primitive `substitute_variable` is built on; it is
    /// also the hook the evaluator's blank-node pre-binding reuses (it descends the
    /// same wrappers to join its singleton seed at the identical position).
    #[must_use]
    pub fn map_core_pattern(self, f: impl FnOnce(GraphPattern) -> GraphPattern) -> Query {
        match self {
            Query::Select {
                pattern,
                dataset,
                base_iri,
            } => Query::Select {
                pattern: map_core_pattern(pattern, f),
                dataset,
                base_iri,
            },
            Query::Construct {
                template,
                pattern,
                dataset,
                base_iri,
            } => Query::Construct {
                template,
                pattern: map_core_pattern(pattern, f),
                dataset,
                base_iri,
            },
            Query::Describe {
                pattern,
                targets,
                dataset,
                base_iri,
            } => Query::Describe {
                pattern: map_core_pattern(pattern, f),
                targets,
                dataset,
                base_iri,
            },
            Query::Ask {
                pattern,
                dataset,
                base_iri,
            } => Query::Ask {
                pattern: map_core_pattern(pattern, f),
                dataset,
                base_iri,
            },
        }
    }
}

/// Descend through the outer solution-modifier/filter wrappers of `pattern` to its
/// core `WHERE` pattern, apply `f` there, and rebuild the wrapper stack around the
/// result. A pattern that is *itself* the core (a bare BGP/Join/etc. with no
/// wrapper) is handed straight to `f`.
///
/// The recursion descends the single-child wrappers that evaluate expressions over
/// their inner rows. `Filter` is included even though it is a graph-pattern node:
/// `FILTER EXISTS { ?this ... }` must see the pre-bound `?this` in its current
/// solution row, matching `VALUES { ?this value } FILTER ...` source semantics.
/// It deliberately stops at the first multi-child graph-pattern node (`Join`,
/// `Union`, `LeftJoin`, `Graph`, …): that node *is* the core `WHERE` pattern, and
/// the seed must join onto the whole of it.
fn map_core_pattern(
    pattern: GraphPattern,
    f: impl FnOnce(GraphPattern) -> GraphPattern,
) -> GraphPattern {
    match pattern {
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Box::new(map_core_pattern(*inner, f)),
            variables,
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Box::new(map_core_pattern(*inner, f)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Box::new(map_core_pattern(*inner, f)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Box::new(map_core_pattern(*inner, f)),
            start,
            length,
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Box::new(map_core_pattern(*inner, f)),
            expression,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Box::new(map_core_pattern(*inner, f)),
            variables,
            aggregates,
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Box::new(map_core_pattern(*inner, f)),
            variable,
            expression,
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr,
            inner: Box::new(map_core_pattern(*inner, f)),
        },
        // The first non-modifier node is the core WHERE pattern.
        core => f(core),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BlankNode, NamedNode, TermPattern, TriplePattern};
    use crate::{NamedNodePattern, SparqlParser};

    fn iri_value(iri: &str) -> GroundTerm {
        GroundTerm::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn this() -> Variable {
        Variable::new("this")
    }

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("valid SPARQL")
    }

    /// The injected seed is a single-row `VALUES { ?var value }`.
    fn assert_seed(left: &GraphPattern, var: &str, iri: &str) {
        match left {
            GraphPattern::Values {
                variables,
                bindings,
            } => {
                assert_eq!(variables.len(), 1);
                assert_eq!(variables[0].as_str(), var);
                assert_eq!(bindings.len(), 1, "exactly one pre-binding row");
                assert_eq!(bindings[0].len(), 1);
                match &bindings[0][0] {
                    Some(GroundTerm::NamedNode(n)) => assert_eq!(n.as_str(), iri),
                    other => panic!("expected the IRI seed, got {other:?}"),
                }
            }
            other => panic!("expected a VALUES seed on the left, got {other:?}"),
        }
    }

    #[test]
    fn select_injects_join_below_projection() {
        // `SELECT ?this WHERE { ?this :p ?o }` → Project[ Join(Values, Bgp) ].
        // The projection wrapper must survive (so ?this stays projectable) and the
        // seed join must sit *under* it.
        let q = parse("SELECT ?this WHERE { ?this <http://ex/p> ?o }");
        let out = q.substitute_variable(&this(), iri_value("http://ex/focus"));
        let Query::Select { pattern, .. } = out else {
            panic!("still a SELECT");
        };
        let GraphPattern::Project { inner, variables } = pattern else {
            panic!("projection preserved, got {pattern:?}");
        };
        assert_eq!(variables, vec![this()], "?this still projected");
        let GraphPattern::Join { left, .. } = *inner else {
            panic!("seed join injected below projection");
        };
        assert_seed(&left, "this", "http://ex/focus");
    }

    #[test]
    fn seed_sits_below_order_by_and_slice_and_distinct() {
        // Stress the full modifier stack: DISTINCT + ORDER BY + LIMIT. The seed must
        // land at the very bottom (the BGP), beneath all three wrappers.
        let q = parse("SELECT DISTINCT ?this WHERE { ?this <http://ex/p> ?o } ORDER BY ?o LIMIT 5");
        let out = q.substitute_variable(&this(), iri_value("http://ex/f"));
        let Query::Select { pattern, .. } = out else {
            panic!("SELECT");
        };
        // Project → Distinct → Slice → OrderBy → Join(Values, Bgp) (modifier order
        // is the parser's; we just assert a Join-with-Values is reached and nothing
        // above it is a Join/Values).
        let mut node = &pattern;
        loop {
            match node {
                GraphPattern::Project { inner, .. }
                | GraphPattern::Distinct { inner }
                | GraphPattern::Reduced { inner }
                | GraphPattern::Slice { inner, .. }
                | GraphPattern::OrderBy { inner, .. }
                | GraphPattern::Group { inner, .. }
                | GraphPattern::Extend { inner, .. } => node = inner,
                GraphPattern::Join { left, .. } => {
                    assert_seed(left, "this", "http://ex/f");
                    break;
                }
                other => panic!("expected to reach the seed Join, got {other:?}"),
            }
        }
    }

    #[test]
    fn ask_with_no_modifier_wraps_whole_pattern() {
        // `ASK { ?this :p ?o }` has no modifier wrapper: the whole BGP is the core,
        // so the result is a bare Join(Values, Bgp).
        let q = parse("ASK { ?this <http://ex/p> ?o }");
        let out = q.substitute_variable(&this(), iri_value("http://ex/f"));
        let Query::Ask { pattern, .. } = out else {
            panic!("still an ASK");
        };
        let GraphPattern::Join { left, right } = pattern else {
            panic!("bare Join, got {pattern:?}");
        };
        assert_seed(&left, "this", "http://ex/f");
        assert!(matches!(*right, GraphPattern::Bgp { .. }));
    }

    #[test]
    fn blank_focus_injects_a_blank_values_seed() {
        // A blank-node focus pre-binds through the SAME rewrite, carrying the
        // injection-only `GroundTerm::BlankNode`.
        let q = parse("SELECT ?this WHERE { ?this <http://ex/p> ?o }");
        let out = q.substitute_variable(&this(), GroundTerm::BlankNode(BlankNode::new("b0")));
        let Query::Select { pattern, .. } = out else {
            panic!("SELECT");
        };
        let GraphPattern::Project { inner, .. } = pattern else {
            panic!("projection preserved");
        };
        let GraphPattern::Join { left, .. } = *inner else {
            panic!("seed join injected");
        };
        let GraphPattern::Values { bindings, .. } = *left else {
            panic!("VALUES seed");
        };
        match &bindings[0][0] {
            Some(GroundTerm::BlankNode(b)) => assert_eq!(b.as_str(), "b0"),
            other => panic!("expected the blank seed, got {other:?}"),
        }
    }

    #[test]
    fn construct_rewrites_where_not_template() {
        // The CONSTRUCT template must be untouched; only the WHERE pattern is seeded.
        let q = parse("CONSTRUCT { ?this <http://ex/r> ?o } WHERE { ?this <http://ex/p> ?o }");
        let out = q.substitute_variable(&this(), iri_value("http://ex/f"));
        let Query::Construct {
            template, pattern, ..
        } = out
        else {
            panic!("still a CONSTRUCT");
        };
        // Template preserved verbatim.
        assert_eq!(template.len(), 1);
        assert_eq!(
            template[0],
            TriplePattern {
                subject: TermPattern::Variable(this()),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/r")),
                object: TermPattern::Variable(Variable::new("o")),
            }
        );
        let GraphPattern::Join { left, .. } = pattern else {
            panic!("WHERE seeded with a Join, got {pattern:?}");
        };
        assert_seed(&left, "this", "http://ex/f");
    }
}
