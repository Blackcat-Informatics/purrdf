// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL-SPARQL **pre-binding restrictions** (spec §5.2.1 *Pre-bound Variables
//! in SPARQL Constraints*, and the appendix on unsupported SPARQL features).
//!
//! A query evaluated with pre-bound variables (`$this`, and `$value` for ASK
//! validators) MUST NOT use constructs whose SPARQL semantics break under
//! pre-binding. A SHACL-SPARQL processor is required to **reject** such a
//! query as a hard failure (`mf:result sht:Failure` in the W3C suite):
//!
//! - `MINUS`,
//! - federated queries (`SERVICE`),
//! - `VALUES`,
//! - `AS ?var` (`BIND`, `SELECT ... (expr AS ?var)`, `GROUP BY ... AS`) where
//!   `?var` is potentially pre-bound,
//! - a subquery that does not project every potentially pre-bound variable
//!   (a nested `SELECT *` whose in-scope variables do not include `$this`
//!   does not project it — W3C `pre-binding-006`; an explicit
//!   `SELECT $this` does — `pre-binding-007`).
//!
//! The checks run at SHAPE-LOAD time (`shapes.rs`), on the already-parsed
//! algebra, so a restricted query never reaches the evaluation engine.

use purrdf_sparql_algebra::{Expression, GraphPattern, OrderExpression, Query};

/// Check a SHACL-SPARQL **SELECT** query (an `sh:select` constraint / validator
/// body) against the pre-binding restrictions with the given pre-bound
/// variable names (no `?`/`$` sigil).
///
/// The OUTERMOST projection is exempt from the subquery-projection rule (the
/// result mapping reads `$this` from the pre-binding, not the projection);
/// every NESTED `SELECT` must project all pre-bound variables.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_select(query: &Query, prebound: &[&str]) -> Result<(), String> {
    let Query::Select { pattern, .. } = query else {
        // Non-SELECT forms are rejected elsewhere (shape-load SELECT-form check).
        return Ok(());
    };
    check_query_body(pattern, prebound)
}

/// Check a SHACL-AF `sh:construct` CONSTRUCT query (a `sh:SPARQLRule` head)
/// against the pre-binding restrictions. The CONSTRUCT `WHERE` algebra is a
/// solution-producing body exactly like a SELECT's, so the same rules apply; the
/// outermost projection (if any) is exempt.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_construct(query: &Query, prebound: &[&str]) -> Result<(), String> {
    let Query::Construct { pattern, .. } = query else {
        // Non-CONSTRUCT forms are rejected elsewhere (rule-load CONSTRUCT check).
        return Ok(());
    };
    check_query_body(pattern, prebound)
}

/// Strip the outer solution modifiers down to the outermost `Project` and check
/// its BODY — nested `Project`s inside the body are subqueries. Shared by
/// [`check_select`] and [`check_construct`].
fn check_query_body(pattern: &GraphPattern, prebound: &[&str]) -> Result<(), String> {
    let mut node = pattern;
    loop {
        match node {
            GraphPattern::Slice { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner } => node = inner,
            GraphPattern::OrderBy { inner, expression } => {
                for order in expression {
                    let (OrderExpression::Asc(e) | OrderExpression::Desc(e)) = order;
                    check_expression(e, prebound)?;
                }
                node = inner;
            }
            GraphPattern::Project { inner, .. } => return check_pattern(inner, prebound),
            other => return check_pattern(other, prebound),
        }
    }
}

/// Check a SHACL-SPARQL **ASK** query (an `sh:ask` validator body) against the
/// pre-binding restrictions. Every `SELECT` inside an ASK body is a subquery,
/// so the subquery-projection rule applies throughout.
///
/// # Errors
///
/// Returns `Err(String)` naming the offending construct.
pub(crate) fn check_ask(query: &Query, prebound: &[&str]) -> Result<(), String> {
    match query {
        Query::Ask { pattern, .. } => check_pattern(pattern, prebound),
        _ => Ok(()),
    }
}

/// Walk a graph pattern, rejecting every construct the pre-binding
/// restrictions forbid.
fn check_pattern(pattern: &GraphPattern, prebound: &[&str]) -> Result<(), String> {
    match pattern {
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } => Ok(()),
        GraphPattern::Minus { .. } => Err(
            "MINUS is not allowed in a query with pre-bound variables (SHACL-SPARQL §5.2.1)"
                .to_owned(),
        ),
        GraphPattern::Service { .. } => Err(
            "federated queries (SERVICE) are not allowed in a query with pre-bound variables \
             (SHACL-SPARQL §5.2.1)"
                .to_owned(),
        ),
        GraphPattern::Values { .. } => Err(
            "VALUES is not allowed in a query with pre-bound variables (SHACL-SPARQL §5.2.1)"
                .to_owned(),
        ),
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
            check_pattern(left, prebound)?;
            check_pattern(right, prebound)
        }
        GraphPattern::Union { left, right } => {
            check_pattern(left, prebound)?;
            check_pattern(right, prebound)
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            check_pattern(left, prebound)?;
            check_pattern(right, prebound)?;
            expression
                .as_ref()
                .map_or(Ok(()), |e| check_expression(e, prebound))
        }
        GraphPattern::Filter { expr, inner } => {
            check_expression(expr, prebound)?;
            check_pattern(inner, prebound)
        }
        GraphPattern::Graph { inner, .. } => check_pattern(inner, prebound),
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            if prebound.contains(&variable.as_str()) {
                return Err(format!(
                    "assigning a potentially pre-bound variable (... AS ?{}) is not allowed \
                     (SHACL-SPARQL §5.2.1)",
                    variable.as_str()
                ));
            }
            check_expression(expression, prebound)?;
            check_pattern(inner, prebound)
        }
        GraphPattern::OrderBy { inner, expression } => {
            for order in expression {
                let (OrderExpression::Asc(e) | OrderExpression::Desc(e)) = order;
                check_expression(e, prebound)?;
            }
            check_pattern(inner, prebound)
        }
        // A nested SELECT (subquery): its projection must expose every
        // potentially pre-bound variable. A `SELECT *` expands (in the
        // algebra) to the body's in-scope variables — a FILTER-only body
        // exposes nothing, so `$this` is NOT projected and the query must be
        // rejected (W3C pre-binding-006).
        GraphPattern::Project { inner, variables } => {
            for name in prebound {
                if !variables.iter().any(|v| v.as_str() == *name) {
                    return Err(format!(
                        "a subquery must project every potentially pre-bound variable; \
                         ?{name} is not in its projection (SHACL-SPARQL §5.2.1)"
                    ));
                }
            }
            check_pattern(inner, prebound)
        }
        GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => check_pattern(inner, prebound),
        GraphPattern::Group {
            inner,
            variables: _,
            aggregates,
        } => {
            for (variable, _) in aggregates {
                if prebound.contains(&variable.as_str()) {
                    return Err(format!(
                        "assigning a potentially pre-bound variable (aggregate AS ?{}) is not \
                         allowed (SHACL-SPARQL §5.2.1)",
                        variable.as_str()
                    ));
                }
            }
            check_pattern(inner, prebound)
        }
    }
}

/// Walk an expression tree; `EXISTS { … }` bodies are graph patterns and are
/// checked recursively.
fn check_expression(expr: &Expression, prebound: &[&str]) -> Result<(), String> {
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => Ok(()),
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => {
            check_expression(a, prebound)?;
            check_expression(b, prebound)
        }
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            check_expression(inner, prebound)
        }
        Expression::In(head, rest) => {
            check_expression(head, prebound)?;
            for e in rest {
                check_expression(e, prebound)?;
            }
            Ok(())
        }
        Expression::If(c, t, e) => {
            check_expression(c, prebound)?;
            check_expression(t, prebound)?;
            check_expression(e, prebound)
        }
        Expression::Coalesce(items) => {
            for e in items {
                check_expression(e, prebound)?;
            }
            Ok(())
        }
        Expression::FunctionCall(_, args) => {
            for e in args {
                check_expression(e, prebound)?;
            }
            Ok(())
        }
        Expression::Exists(pattern) => check_pattern(pattern, prebound),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_sparql_algebra::SparqlParser;

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("query parses")
    }

    fn check(q: &str) -> Result<(), String> {
        check_select(&parse(q), &["this"])
    }

    #[test]
    fn plain_bgp_and_filter_pass() {
        assert!(check("SELECT $this WHERE { $this ?p ?o . FILTER($this != ?o) }").is_ok());
    }

    #[test]
    fn minus_is_rejected() {
        let err = check("SELECT $this WHERE { $this ?p ?o . MINUS { $this ?p \"x\" } }")
            .expect_err("MINUS must be rejected");
        assert!(err.contains("MINUS"), "{err}");
    }

    #[test]
    fn values_is_rejected() {
        let err = check("SELECT $this WHERE { $this ?p ?o . VALUES ?o { 1 2 } }")
            .expect_err("VALUES must be rejected");
        assert!(err.contains("VALUES"), "{err}");
    }

    #[test]
    fn service_is_rejected() {
        let err =
            check("SELECT $this WHERE { SERVICE <http://example.org/sparql> { $this ?p ?o } }")
                .expect_err("SERVICE must be rejected");
        assert!(err.contains("SERVICE"), "{err}");
    }

    #[test]
    fn bind_as_prebound_is_rejected() {
        let err = check("SELECT $this WHERE { BIND(true AS $this) }")
            .expect_err("BIND ... AS $this must be rejected");
        assert!(err.contains("pre-bound"), "{err}");
    }

    #[test]
    fn bind_of_prebound_into_other_var_passes() {
        // Using $this INSIDE the expression is fine (pre-binding-004); only
        // ASSIGNING to it is restricted.
        assert!(check("SELECT $this WHERE { BIND($this AS ?that) }").is_ok());
    }

    #[test]
    fn subquery_not_projecting_this_is_rejected() {
        let err = check(
            "SELECT $this WHERE { $this ?x ?any . { SELECT ?other WHERE { ?other ?b ?c } } }",
        )
        .expect_err("subquery without $this must be rejected");
        assert!(err.contains("subquery"), "{err}");
    }

    #[test]
    fn subquery_projecting_this_passes() {
        assert!(check("SELECT $this WHERE { { SELECT $this WHERE { $this ?p ?o } } }").is_ok());
    }

    #[test]
    fn outer_projection_without_this_is_not_a_subquery() {
        // The OUTERMOST projection is exempt: the result mapping reads $this
        // from the pre-binding, not the projection.
        assert!(check("SELECT ?o WHERE { $this ?p ?o }").is_ok());
    }

    #[test]
    fn ask_bind_as_value_is_rejected() {
        let q = parse("ASK { BIND(true AS ?value) . FILTER(isLiteral(?value)) }");
        let err =
            check_ask(&q, &["this", "value"]).expect_err("ASK BIND AS ?value must be rejected");
        assert!(err.contains("pre-bound"), "{err}");
    }
}
