// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Enforcement of the `VERSION "1.2-basic"` profile (SPARQL 1.2 Query
//! specification §4.3.1, "Version Labels").
//!
//! # Spec evidence for what the Basic profile restricts
//!
//! The SPARQL 1.2 Query specification's §4.3.1 "Version Labels" table (fetched
//! 2026-08-14 from <https://www.w3.org/TR/sparql12-query/#version-labels>) is
//! the sole normative source for what a declared `VERSION` string means. Its
//! entry for `"1.2-basic"` reads, verbatim:
//!
//! > **Version Label**: `"1.2-basic"`
//! > **Syntax**: SPARQL 1.2 query or update syntax, without triple terms and
//! > without triple patterns that have a triple pattern in their subject or
//! > object position
//! > **Semantics**: SPARQL 1.2 Query Language, SPARQL 1.2 Update
//!
//! The same section also states the conformance chain that motivates the
//! profile: *"If a query conforms to version '1.1', it also conforms to
//! version '1.2-basic', and if a query conforms to version '1.2-basic', it
//! also conforms to version '1.2'."* SPARQL 1.1 has no triple-term syntax at
//! all, which is consistent with reading the Basic profile as "SPARQL 1.2
//! minus the triple-term/reification feature area" rather than an
//! independently invented restriction set.
//!
//! Appendix A, "Changes between SPARQL 1.1 Query Language and SPARQL 1.2 Query
//! Language" (non-normative but corroborating), groups the entire feature area
//! under one bullet: *"Update grammar for triple terms, reifiers, reified
//! triples, annotation syntax, and triple term functions in 19.7 Grammar"*,
//! followed immediately by *"Add functions related to triple terms to 17.4.6
//! Functions on Triple Terms: TRIPLE, isTRIPLE, SUBJECT, PREDICATE, OBJECT"*.
//! The spec's own changelog therefore bundles the triple-term grammar (`<<( s
//! p o )>>`), the reifying-triple/annotation sugar (`<< s p o >>`, `{| ... |}`)
//! that desugars onto it, AND the five accessor/constructor functions into one
//! normative unit — the unit the Basic profile's "without triple terms"
//! sentence excludes. No OTHER SPARQL 1.2 addition (base-direction literals,
//! `LANGDIR`/`hasLANG`/`hasLANGDIR`/`STRLANGDIR`, `ADJUST`, `sameValue`, the
//! `VERSION` declaration itself, …) is mentioned by the Basic profile's syntax
//! restriction, so none of those is gated here.
//!
//! # What is gated, and why
//!
//! This module refuses, under `VERSION "1.2-basic"`:
//!
//! 1. **A triple term or reifying triple in a triple/quad pattern or a
//!    property-path pattern's endpoint** ([`TermPattern::Triple`]). This is the
//!    grammar's `TripleTerm` (`<<( s p o )>>`) production directly, AND —
//!    because `purrdf-sparql-algebra`'s parser desugars a reifying triple
//!    `<< s p o [~r] >>` (and the `{| ... |}` annotation sugar built on it)
//!    into a base triple pattern plus an auxiliary `r rdf:reifies <<( s p o
//!    )>>` triple (see `crate::parser::Parser::emit_reifies` in that crate) —
//!    every use of the reifying-triple/annotation syntax too, including
//!    nesting (`ReifiedTripleSubject`/`ReifiedTripleObject` admit a nested
//!    `ReifiedTriple` or `TripleTerm` per the grammar), which is exactly the
//!    spec's second clause ("triple patterns that have a triple pattern in
//!    their subject or object position").
//! 2. **A ground triple term in a `VALUES` data block**
//!    ([`GroundTerm::Triple`], grammar production `TripleTermData`) — the
//!    ground-data counterpart of 1.
//! 3. **The RDF 1.2 "Functions on Triple Terms" (§17.4.6)**: `TRIPLE()`,
//!    `isTRIPLE()`, `SUBJECT()`, `PREDICATE()`, `OBJECT()`. The `<<( s p o
//!    )>>` *expression*-position spelling of a triple term also lowers to
//!    [`Function::Triple`] in this crate's algebra (see
//!    `crate::parser::Parser::parse_triple_term_expr` in
//!    `purrdf-sparql-algebra`, which documents "it denotes the same value as
//!    TRIPLE(s, p, o), so it lowers to that function call") — the two
//!    spellings are indistinguishable once parsed, so gating one and not the
//!    other would let a Basic-profile author route around the ban by writing
//!    the function-call spelling. `isTRIPLE`/`SUBJECT`/`PREDICATE`/`OBJECT`
//!    are gated on the strength of Appendix A bundling them with the grammar
//!    change as one feature area (see above), not because they themselves
//!    contain `TripleTerm` syntax.
//!
//! This module deliberately does NOT gate a bare variable that happens, at
//! evaluation time, to be *bound* to an RDF 1.2 triple-term value already
//! present in the underlying dataset (e.g. `SELECT ?s WHERE { ?x :p ?t }`
//! where `?t` binds to a triple term the data contains) — the Basic profile's
//! "Syntax" column restricts what the QUERY TEXT may write, not what values
//! the data may contain, and RDF 1.2 triples may legally carry a triple term
//! as their object regardless of the query's declared profile.

use purrdf_sparql_algebra::{
    AggregateExpression, Expression, Function, GraphPattern, GraphUpdateOperation, GroundTerm,
    OrderExpression, PropertyFunctionCall, QuadPattern, Query, TermPattern, TriplePattern, Update,
};

use crate::error::EvalError;
use crate::eval::AdmittedRequest;

/// Admit `request` under the `VERSION "1.2-basic"` profile: `Ok(())` if it uses
/// no gated construct, otherwise a typed [`EvalError::Unsupported`] naming the
/// first offending construct found (a deterministic pre-order walk of the
/// algebra, so the same query always names the same construct).
///
/// Called from `crate::eval::admit_version` — the shared chokepoint both the
/// query and the update evaluator pass through — ONLY when the request's
/// declared version is [`purrdf_sparql_algebra::SparqlVersion::V12Basic`]; a
/// `VERSION "1.2"` (or undeclared-version) request never reaches this
/// function, so the full profile is unaffected by this gate.
pub(crate) fn admit(request: AdmittedRequest<'_>) -> Result<(), EvalError> {
    match request {
        AdmittedRequest::Query(query) => admit_query(query),
        AdmittedRequest::Update(update) => admit_update(update),
    }
}

fn admit_query(query: &Query) -> Result<(), EvalError> {
    match query {
        Query::Select { pattern, .. }
        | Query::Ask { pattern, .. }
        | Query::Describe { pattern, .. } => check_pattern(pattern),
        Query::Construct {
            template, pattern, ..
        } => {
            for t in template {
                check_triple_pattern(t)?;
            }
            check_pattern(pattern)
        }
    }
}

fn admit_update(update: &Update) -> Result<(), EvalError> {
    for op in &update.operations {
        check_operation(op)?;
    }
    Ok(())
}

fn check_operation(op: &GraphUpdateOperation) -> Result<(), EvalError> {
    match op {
        GraphUpdateOperation::InsertData { data } | GraphUpdateOperation::DeleteData { data } => {
            for q in data {
                check_quad_pattern(q)?;
            }
            Ok(())
        }
        GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            pattern,
            with: _,
            using: _,
        } => {
            for q in delete.iter().chain(insert.iter()) {
                check_quad_pattern(q)?;
            }
            check_pattern(pattern)
        }
        GraphUpdateOperation::Load { .. }
        | GraphUpdateOperation::Clear { .. }
        | GraphUpdateOperation::Drop { .. }
        | GraphUpdateOperation::Create { .. }
        | GraphUpdateOperation::Add { .. }
        | GraphUpdateOperation::Move { .. }
        | GraphUpdateOperation::Copy { .. } => Ok(()),
    }
}

fn check_quad_pattern(q: &QuadPattern) -> Result<(), EvalError> {
    check_triple_pattern(&q.triple)
}

fn check_triple_pattern(t: &TriplePattern) -> Result<(), EvalError> {
    check_term_pattern(&t.subject)?;
    check_term_pattern(&t.object)
}

fn check_term_pattern(t: &TermPattern) -> Result<(), EvalError> {
    match t {
        TermPattern::NamedNode(_)
        | TermPattern::BlankNode(_)
        | TermPattern::Literal(_)
        | TermPattern::Variable(_) => Ok(()),
        TermPattern::Triple(_) => Err(refuse(
            "an RDF 1.2 triple term or reifying triple (`<<( s p o )>>` / `<<s p o>>`) \
             in a triple pattern",
        )),
    }
}

fn check_ground_term(t: &GroundTerm) -> Result<(), EvalError> {
    match t {
        GroundTerm::NamedNode(_) | GroundTerm::Literal(_) | GroundTerm::BlankNode(_) => Ok(()),
        GroundTerm::Triple(_) => Err(refuse(
            "an RDF 1.2 ground triple term (`<<( ... )>>`) in a VALUES data block",
        )),
    }
}

fn check_pattern(pattern: &GraphPattern) -> Result<(), EvalError> {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            for t in patterns {
                check_triple_pattern(t)?;
            }
            Ok(())
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            check_term_pattern(subject)?;
            check_term_pattern(object)
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right } => {
            check_pattern(left)?;
            check_pattern(right)
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            check_pattern(left)?;
            check_pattern(right)?;
            match expression {
                Some(e) => check_expression(e),
                None => Ok(()),
            }
        }
        GraphPattern::Filter { expr, inner } => {
            check_expression(expr)?;
            check_pattern(inner)
        }
        GraphPattern::Graph { inner, .. } | GraphPattern::Service { inner, .. } => {
            check_pattern(inner)
        }
        GraphPattern::Extend {
            inner, expression, ..
        } => {
            check_expression(expression)?;
            check_pattern(inner)
        }
        GraphPattern::Values { bindings, .. } => {
            for row in bindings {
                for cell in row.iter().flatten() {
                    check_ground_term(cell)?;
                }
            }
            Ok(())
        }
        GraphPattern::OrderBy { inner, expression } => {
            for oe in expression {
                match oe {
                    OrderExpression::Asc(e) | OrderExpression::Desc(e) => check_expression(e)?,
                }
            }
            check_pattern(inner)
        }
        GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => check_pattern(inner),
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            for (_, agg) in aggregates {
                check_aggregate(agg)?;
            }
            check_pattern(inner)
        }
        GraphPattern::PropertyFunction(call) => check_property_function(call),
    }
}

fn check_aggregate(agg: &AggregateExpression) -> Result<(), EvalError> {
    for e in agg.args() {
        check_expression(e)?;
    }
    Ok(())
}

fn check_property_function(call: &PropertyFunctionCall) -> Result<(), EvalError> {
    for t in call.subject_args.iter().chain(call.object_args.iter()) {
        check_term_pattern(t)?;
    }
    Ok(())
}

fn check_expression(expr: &Expression) -> Result<(), EvalError> {
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
            check_expression(a)?;
            check_expression(b)
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            check_expression(a)
        }
        Expression::In(e, list) => {
            check_expression(e)?;
            for x in list {
                check_expression(x)?;
            }
            Ok(())
        }
        Expression::If(cond, then, els) => {
            check_expression(cond)?;
            check_expression(then)?;
            check_expression(els)
        }
        Expression::Coalesce(list) => {
            for x in list {
                check_expression(x)?;
            }
            Ok(())
        }
        Expression::FunctionCall(func, args) => {
            check_function(func)?;
            for a in args {
                check_expression(a)?;
            }
            Ok(())
        }
        Expression::Exists(pattern) => check_pattern(pattern),
    }
}

/// Refuse the five "Functions on Triple Terms" (SPARQL 1.2 Query specification
/// §17.4.6); every other [`Function`] variant is unrestricted by the Basic
/// profile (see the module docs).
fn check_function(func: &Function) -> Result<(), EvalError> {
    let name = match func {
        Function::Triple => Some("TRIPLE()"),
        Function::IsTriple => Some("isTRIPLE()"),
        Function::Subject => Some("SUBJECT()"),
        Function::Predicate => Some("PREDICATE()"),
        Function::Object => Some("OBJECT()"),
        _ => None,
    };
    match name {
        Some(name) => Err(refuse(format!(
            "the RDF 1.2 triple-term function {name} (SPARQL 1.2 Query specification §17.4.6)"
        ))),
        None => Ok(()),
    }
}

fn refuse(construct: impl Into<String>) -> EvalError {
    EvalError::unsupported(format!(
        "VERSION \"1.2-basic\" admits no RDF 1.2 triple-term construct \
         (SPARQL 1.2 Query specification §4.3.1 Version Labels); found {}",
        construct.into()
    ))
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_algebra::SparqlParser;

    use super::*;

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("parses")
    }

    #[test]
    fn triple_term_pattern_is_refused() {
        let q = parse(
            "VERSION \"1.2-basic\"\n\
             PREFIX : <http://example.org/>\n\
             SELECT * WHERE { ?r :reifies <<( ?s ?p ?o )>> }",
        );
        let err = admit_query(&q).expect_err("triple term must be refused");
        assert!(err.to_string().contains("triple term"), "{err}");
    }

    #[test]
    fn reifying_triple_pattern_is_refused() {
        let q = parse(
            "VERSION \"1.2-basic\"\n\
             PREFIX : <http://example.org/>\n\
             SELECT * WHERE { << ?s :p ?o >> :q ?v }",
        );
        let err = admit_query(&q).expect_err("reifying triple must be refused");
        assert!(err.to_string().contains("triple"), "{err}");
    }

    #[test]
    fn triple_functions_are_refused() {
        for expr in ["isTRIPLE(?t)", "SUBJECT(?t)", "PREDICATE(?t)", "OBJECT(?t)"] {
            let q = parse(&format!(
                "VERSION \"1.2-basic\"\n\
                 PREFIX : <http://example.org/>\n\
                 SELECT (({expr}) AS ?x) WHERE {{ ?s :p ?t }}"
            ));
            let err = admit_query(&q).expect_err(&format!("{expr} must be refused"));
            assert!(err.to_string().contains("triple-term function"), "{err}");
        }
    }

    #[test]
    fn plain_query_is_admitted() {
        let q = parse(
            "VERSION \"1.2-basic\"\n\
             PREFIX : <http://example.org/>\n\
             SELECT * WHERE { ?s :p ?o . FILTER(?o > 1) }",
        );
        admit_query(&q).expect("plain BGP + FILTER must be admitted");
    }
}
