// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! `LANGDIR`/`hasLANG`/`hasLANGDIR`/`STRLANGDIR`, `ADJUST`, the `VERSION`
//! declaration itself, …) is mentioned by the Basic profile's syntax
//! restriction, so none of those is gated here.
//!
//! `sameValue` (§17.4.2.2) is deliberately NOT in that "available and simply
//! not gated" list, even though it is a real SPARQL 1.2 addition: its own spec
//! text says *"This function cannot be used directly in expressions"* — it
//! replaces `RDFterm-equal` from SPARQL 1.1 to define what the `=` operator
//! does with two RDF terms the operator-mapping table (§17.3) does not cover
//! directly, not a name a query can call. There is therefore no profile
//! question to ask about it: it was never callable syntax under ANY version
//! label, Basic included. Its semantics (including the `sameValue`-only
//! cross-type NaN carve-out — `"NaN"^^xsd:double = "NaN"^^xsd:float` is `true`)
//! are what `crate::expr`'s `=`/`sameValue` evaluation already implements; see
//! `crate::expr::sparql_value_eq`'s docs.
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
            for q in template {
                check_quad_pattern(q)?;
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

/// One node of the admission walk still to be checked.
enum Pending<'a> {
    Pattern(&'a GraphPattern),
    Expression(&'a Expression),
}

/// Check every node under `pattern`, depth first, and refuse at the first gated
/// construct.
///
/// The walk keeps its own work list — one entry per node still to be checked — so a
/// request of any depth is admitted without a machine-stack frame per level. A node's
/// parts are pushed in reverse, so they pop in the order the profile reads them: a
/// `FILTER`'s expression before its pattern, an `OPTIONAL`'s left operand, then its
/// right, then its condition, an `ORDER BY`'s keys before its pattern, a `GROUP BY`'s
/// aggregate arguments before the pattern being grouped. The leaf checks — a triple or
/// path term, a `VALUES` cell, a call's arguments, a function name — run when their
/// node pops, so the first refusal names the same construct a pre-order reading of the
/// query in that order reaches first. A user-defined function's body is admitted
/// wherever the call is evaluated, through the same walk.
fn check_pattern(pattern: &GraphPattern) -> Result<(), EvalError> {
    check_from(Pending::Pattern(pattern))
}

/// [`check_pattern`]'s walk, from a pattern or an expression.
fn check_from(root: Pending<'_>) -> Result<(), EvalError> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        // Everything pushed for this node is reversed below, so the parts pop in the
        // order they are pushed here.
        let first = pending.len();
        match node {
            Pending::Pattern(pattern) => match pattern {
                GraphPattern::Bgp { patterns } => {
                    for t in patterns {
                        check_triple_pattern(t)?;
                    }
                }
                GraphPattern::Path {
                    subject, object, ..
                } => {
                    check_term_pattern(subject)?;
                    check_term_pattern(object)?;
                }
                GraphPattern::Join { left, right }
                | GraphPattern::Minus { left, right }
                | GraphPattern::Lateral { left, right } => {
                    pending.push(Pending::Pattern(left));
                    pending.push(Pending::Pattern(right));
                }
                GraphPattern::Union { arms } => pending.extend(arms.iter().map(Pending::Pattern)),
                GraphPattern::LeftJoin {
                    left,
                    right,
                    expression,
                } => {
                    pending.push(Pending::Pattern(left));
                    pending.push(Pending::Pattern(right));
                    if let Some(e) = expression {
                        pending.push(Pending::Expression(e));
                    }
                }
                GraphPattern::Filter { expr, inner } => {
                    pending.push(Pending::Expression(expr));
                    pending.push(Pending::Pattern(inner));
                }
                GraphPattern::Graph { inner, .. } | GraphPattern::Service { inner, .. } => {
                    pending.push(Pending::Pattern(inner));
                }
                GraphPattern::Extend {
                    inner, expression, ..
                } => {
                    pending.push(Pending::Expression(expression));
                    pending.push(Pending::Pattern(inner));
                }
                // `UNFOLD` introduces no triple-term SYNTAX of its own — its targets are
                // plain variables and its operand is an ordinary expression — so the
                // Basic profile has nothing to gate at this node and it is transparent,
                // exactly like `BIND`. (A composite VALUE may of course hold a triple
                // term; the profile restricts syntax, not the values a query computes,
                // the same way it leaves `cdt:List(…)` alone.)
                GraphPattern::Unfold {
                    inner, expression, ..
                } => {
                    pending.push(Pending::Expression(expression));
                    pending.push(Pending::Pattern(inner));
                }
                GraphPattern::Values { bindings, .. } => {
                    for row in bindings {
                        for cell in row.iter().flatten() {
                            check_ground_term(cell)?;
                        }
                    }
                }
                GraphPattern::OrderBy { inner, expression } => {
                    for oe in expression {
                        match oe {
                            OrderExpression::Asc(e) | OrderExpression::Desc(e) => {
                                pending.push(Pending::Expression(e));
                            }
                        }
                    }
                    pending.push(Pending::Pattern(inner));
                }
                GraphPattern::Project { inner, .. }
                | GraphPattern::Distinct { inner }
                | GraphPattern::Reduced { inner }
                | GraphPattern::Slice { inner, .. } => pending.push(Pending::Pattern(inner)),
                GraphPattern::Group {
                    inner, aggregates, ..
                } => {
                    for (_, agg) in aggregates {
                        pending.extend(aggregate_expressions(agg).map(Pending::Expression));
                    }
                    pending.push(Pending::Pattern(inner));
                }
                GraphPattern::PropertyFunction(call) => check_property_function(call)?,
            },
            Pending::Expression(expr) => match expr {
                Expression::NamedNode(_)
                | Expression::Literal(_)
                | Expression::Variable(_)
                | Expression::Bound(_) => {}
                Expression::Or(operands) | Expression::And(operands) => {
                    pending.extend(operands.iter().map(Pending::Expression));
                }
                Expression::Arithmetic(first, steps) => {
                    pending.push(Pending::Expression(first));
                    pending.extend(
                        steps
                            .iter()
                            .map(|(_, operand)| Pending::Expression(operand)),
                    );
                }
                Expression::Equal(a, b)
                | Expression::SameTerm(a, b)
                | Expression::Greater(a, b)
                | Expression::GreaterOrEqual(a, b)
                | Expression::Less(a, b)
                | Expression::LessOrEqual(a, b) => {
                    pending.push(Pending::Expression(a));
                    pending.push(Pending::Expression(b));
                }
                Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                    pending.push(Pending::Expression(a));
                }
                Expression::In(e, list) => {
                    pending.push(Pending::Expression(e));
                    pending.extend(list.iter().map(Pending::Expression));
                }
                Expression::If(cond, then, els) => {
                    pending.push(Pending::Expression(cond));
                    pending.push(Pending::Expression(then));
                    pending.push(Pending::Expression(els));
                }
                Expression::Coalesce(list) => pending.extend(list.iter().map(Pending::Expression)),
                Expression::FunctionCall(func, args) => {
                    check_function(func)?;
                    pending.extend(args.iter().map(Pending::Expression));
                }
                Expression::Exists(pattern) => pending.push(Pending::Pattern(pattern)),
            },
        }
        pending[first..].reverse();
    }
    Ok(())
}

/// The expressions an aggregate is checked through: its arguments, then its own
/// `ORDER BY` keys.
fn aggregate_expressions(agg: &AggregateExpression) -> impl Iterator<Item = &Expression> {
    agg.args()
        .iter()
        .chain(agg.order_by().iter().map(crate::modifier::order_sort_key))
}

fn check_property_function(call: &PropertyFunctionCall) -> Result<(), EvalError> {
    for t in call.subject_args.iter().chain(call.object_args.iter()) {
        check_term_pattern(t)?;
    }
    Ok(())
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

/// The work-list walk against a recursive reading of the same rules, over generated
/// shapes, and at a depth no recursive reading could reach on a small stack.
#[cfg(test)]
mod walk_tests {
    use purrdf_sparql_algebra::{
        AggregateFunction, ArithmeticOperator, BlankNode, Chain, Child, GroundTerm, GroundTriple,
        Literal, NamedNode, NamedNodePattern, NonEmpty, Variable,
    };

    use super::*;

    // ── The recursive reference ────────────────────────────────────────────────────

    /// [`check_pattern`] as a recursion, one frame per algebra level.
    fn reference_pattern(pattern: &GraphPattern) -> Result<(), EvalError> {
        match pattern {
            GraphPattern::Bgp { patterns } => patterns.iter().try_for_each(check_triple_pattern),
            GraphPattern::Path {
                subject, object, ..
            } => {
                check_term_pattern(subject)?;
                check_term_pattern(object)
            }
            GraphPattern::Join { left, right }
            | GraphPattern::Minus { left, right }
            | GraphPattern::Lateral { left, right } => {
                reference_pattern(left)?;
                reference_pattern(right)
            }
            GraphPattern::Union { arms } => arms.iter().try_for_each(reference_pattern),
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                reference_pattern(left)?;
                reference_pattern(right)?;
                expression.as_ref().map_or(Ok(()), reference_expression)
            }
            GraphPattern::Filter { expr, inner } => {
                reference_expression(expr)?;
                reference_pattern(inner)
            }
            GraphPattern::Graph { inner, .. } | GraphPattern::Service { inner, .. } => {
                reference_pattern(inner)
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => {
                reference_expression(expression)?;
                reference_pattern(inner)
            }
            GraphPattern::Values { bindings, .. } => bindings
                .iter()
                .flatten()
                .flatten()
                .try_for_each(check_ground_term),
            GraphPattern::OrderBy { inner, expression } => {
                for oe in expression {
                    reference_expression(crate::modifier::order_sort_key(oe))?;
                }
                reference_pattern(inner)
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => reference_pattern(inner),
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                for (_, agg) in aggregates {
                    aggregate_expressions(agg).try_for_each(reference_expression)?;
                }
                reference_pattern(inner)
            }
            GraphPattern::PropertyFunction(call) => check_property_function(call),
        }
    }

    fn reference_expression(expr: &Expression) -> Result<(), EvalError> {
        match expr {
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_) => Ok(()),
            Expression::Or(operands) | Expression::And(operands) => {
                operands.iter().try_for_each(reference_expression)
            }
            Expression::Arithmetic(first, steps) => {
                reference_expression(first)?;
                steps
                    .iter()
                    .try_for_each(|(_, operand)| reference_expression(operand))
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                reference_expression(a)?;
                reference_expression(b)
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                reference_expression(a)
            }
            Expression::In(e, list) => {
                reference_expression(e)?;
                list.iter().try_for_each(reference_expression)
            }
            Expression::If(cond, then, els) => {
                reference_expression(cond)?;
                reference_expression(then)?;
                reference_expression(els)
            }
            Expression::Coalesce(list) => list.iter().try_for_each(reference_expression),
            Expression::FunctionCall(func, args) => {
                check_function(func)?;
                args.iter().try_for_each(reference_expression)
            }
            Expression::Exists(pattern) => reference_pattern(pattern),
        }
    }

    // ── A deterministic shape generator ────────────────────────────────────────────

    /// A choice sequence: every decision is drawn from one SplitMix64 stream, so a seed
    /// names one shape.
    struct Choices {
        state: purrdf_testkit::rng::SplitMix64,
        /// How many more nodes the shape may hold.
        budget: usize,
    }

    impl Choices {
        fn new(seed: u64) -> Self {
            Self {
                state: purrdf_testkit::rng::SplitMix64::new(seed),
                budget: 40,
            }
        }

        fn choose(&mut self, options: usize) -> usize {
            self.state.below_usize(options)
        }

        fn spend(&mut self) -> bool {
            if self.budget == 0 {
                return false;
            }
            self.budget -= 1;
            true
        }
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("http://example.org/{local}"))
    }

    fn var(choices: &mut Choices) -> Variable {
        Variable::new(format!("v{}", choices.choose(4)))
    }

    /// A term: a variable, an IRI, a literal, a blank node, or — one time in eight — a
    /// quoted triple, whose own subject and object are terms again.
    fn term(choices: &mut Choices) -> TermPattern {
        match choices.choose(8) {
            0 | 1 => TermPattern::Variable(var(choices)),
            2 | 3 => TermPattern::NamedNode(iri("n")),
            4 => TermPattern::Literal(Literal::new_simple("lit")),
            5 | 6 => TermPattern::BlankNode(BlankNode::new("b")),
            _ if choices.spend() => TermPattern::Triple(Child::new(TriplePattern {
                subject: term(choices),
                predicate: NamedNodePattern::NamedNode(iri("p")),
                object: term(choices),
            })),
            _ => TermPattern::Variable(var(choices)),
        }
    }

    fn ground(choices: &mut Choices) -> GroundTerm {
        match choices.choose(6) {
            0 | 1 => GroundTerm::NamedNode(iri("g")),
            2 | 3 => GroundTerm::Literal(Literal::new_simple("cell")),
            4 => GroundTerm::BlankNode(BlankNode::new("gb")),
            _ if choices.spend() => GroundTerm::Triple(Child::new(GroundTriple {
                subject: ground(choices),
                predicate: iri("p"),
                object: ground(choices),
            })),
            _ => GroundTerm::NamedNode(iri("g")),
        }
    }

    fn triple(choices: &mut Choices) -> TriplePattern {
        TriplePattern {
            subject: term(choices),
            predicate: if choices.choose(3) == 0 {
                NamedNodePattern::Variable(var(choices))
            } else {
                NamedNodePattern::NamedNode(iri("p"))
            },
            object: term(choices),
        }
    }

    fn bgp(choices: &mut Choices) -> GraphPattern {
        GraphPattern::Bgp {
            patterns: (0..choices.choose(3)).map(|_| triple(choices)).collect(),
        }
    }

    /// A gated function one time in ten, an ordinary one otherwise.
    fn function(choices: &mut Choices) -> Function {
        match choices.choose(10) {
            0 => match choices.choose(5) {
                0 => Function::Triple,
                1 => Function::IsTriple,
                2 => Function::Subject,
                3 => Function::Predicate,
                _ => Function::Object,
            },
            1 | 2 => Function::Str,
            3 => Function::IsIri,
            _ => Function::StrLen,
        }
    }

    fn expression(choices: &mut Choices) -> Expression {
        if !choices.spend() {
            return Expression::Variable(var(choices));
        }
        match choices.choose(12) {
            0 => Expression::Variable(var(choices)),
            1 => Expression::Bound(var(choices)),
            2 => Expression::Literal(Literal::new_simple("x")),
            3 => Expression::Or(
                Chain::try_from(vec![expression(choices), expression(choices)])
                    .expect("two operands"),
            ),
            4 => Expression::Arithmetic(
                Child::new(expression(choices)),
                NonEmpty::try_from(vec![(ArithmeticOperator::Add, expression(choices))])
                    .expect("one step"),
            ),
            5 => Expression::Equal(
                Child::new(expression(choices)),
                Child::new(expression(choices)),
            ),
            6 => Expression::Not(Child::new(expression(choices))),
            7 => Expression::In(
                Child::new(expression(choices)),
                vec![expression(choices), expression(choices)].into(),
            ),
            8 => Expression::If(
                Child::new(expression(choices)),
                Child::new(expression(choices)),
                Child::new(expression(choices)),
            ),
            9 => Expression::Coalesce(vec![expression(choices)].into()),
            10 => Expression::FunctionCall(function(choices), vec![expression(choices)].into()),
            _ => Expression::Exists(Child::new(pattern(choices))),
        }
    }

    fn aggregate(choices: &mut Choices) -> (Variable, AggregateExpression) {
        let function = if choices.choose(2) == 0 {
            AggregateFunction::Count
        } else {
            AggregateFunction::Sample
        };
        (
            Variable::new("agg"),
            AggregateExpression::new(
                function,
                vec![expression(choices)],
                Vec::new(),
                Vec::new(),
                false,
            )
            .expect("one argument"),
        )
    }

    fn pattern(choices: &mut Choices) -> GraphPattern {
        if !choices.spend() {
            return bgp(choices);
        }
        match choices.choose(16) {
            0 => bgp(choices),
            1 => GraphPattern::Path {
                subject: term(choices),
                path: purrdf_sparql_algebra::PropertyPathExpression::NamedNode(iri("p")),
                object: term(choices),
            },
            2 => GraphPattern::Join {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            3 => GraphPattern::LeftJoin {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
                expression: (choices.choose(2) == 0).then(|| expression(choices)),
            },
            4 => GraphPattern::Filter {
                expr: expression(choices),
                inner: Child::new(pattern(choices)),
            },
            5 => GraphPattern::Union {
                arms: Chain::try_from(vec![pattern(choices), pattern(choices)]).expect("two arms"),
            },
            6 => GraphPattern::Extend {
                inner: Child::new(pattern(choices)),
                variable: var(choices),
                expression: expression(choices),
            },
            7 => GraphPattern::Minus {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            8 => GraphPattern::Values {
                variables: vec![Variable::new("a"), Variable::new("b")],
                bindings: (0..choices.choose(3))
                    .map(|_| {
                        vec![
                            Some(ground(choices)),
                            (choices.choose(2) == 0).then(|| ground(choices)),
                        ]
                    })
                    .collect(),
            },
            9 => GraphPattern::OrderBy {
                inner: Child::new(pattern(choices)),
                expression: vec![
                    OrderExpression::Asc(expression(choices)),
                    OrderExpression::Desc(expression(choices)),
                ],
            },
            10 => GraphPattern::Project {
                inner: Child::new(pattern(choices)),
                variables: vec![var(choices)],
            },
            11 => GraphPattern::Group {
                inner: Child::new(pattern(choices)),
                variables: vec![var(choices)],
                aggregates: vec![aggregate(choices), aggregate(choices)],
            },
            12 => GraphPattern::PropertyFunction(PropertyFunctionCall {
                iri: "http://example.org/rel".to_owned(),
                subject_args: vec![term(choices)],
                object_args: vec![term(choices), term(choices)],
            }),
            13 => GraphPattern::Unfold {
                inner: Child::new(pattern(choices)),
                expression: expression(choices),
                element: Variable::new("e"),
                companion: None,
            },
            14 => GraphPattern::Service {
                name: NamedNodePattern::NamedNode(iri("svc")),
                inner: Child::new(pattern(choices)),
                silent: false,
            },
            _ => GraphPattern::Lateral {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
        }
    }

    /// A verdict, comparable: the refusal's text names the construct found first.
    fn verdict(result: Result<(), EvalError>) -> Result<(), String> {
        result.map_err(|error| error.to_string())
    }

    // ── The tests ──────────────────────────────────────────────────────────────────

    /// The work list names the same first construct the recursion names, or admits
    /// the same shape, for every generated shape.
    #[test]
    fn the_walk_agrees_with_its_recursive_reference_on_generated_shapes() {
        let mut refused = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let shape = pattern(&mut choices);
            let expected = verdict(reference_pattern(&shape));
            refused += usize::from(expected.is_err());
            assert_eq!(
                verdict(check_pattern(&shape)),
                expected,
                "seed {seed}: {shape:?}"
            );
        }
        assert!(
            refused > 40 && refused < 360,
            "the generator produces both admitted and refused shapes ({refused} of 400 refused)"
        );
    }

    /// `depth` levels of `FILTER(BOUND(?x))` over a one-triple pattern whose object is
    /// `bottom`.
    fn deep_filters(depth: usize, bottom: TermPattern) -> GraphPattern {
        let mut shape = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(iri("p")),
                object: bottom,
            }],
        };
        for _ in 0..depth {
            shape = GraphPattern::Filter {
                expr: Expression::Bound(Variable::new("x")),
                inner: Child::new(shape),
            };
        }
        shape
    }

    /// A hundred thousand nested filters are admitted on a 128 KiB stack, and the
    /// triple term at the bottom of the same shape is still refused there — the
    /// refusal and its admitted neighbour, both at a depth no frame-per-level walk
    /// answers on that stack.
    #[test]
    fn a_hundred_thousand_level_pattern_is_admitted_or_refused_on_a_128_kib_thread() {
        let outcomes = purrdf_stack::on_stack(128 * 1024, || {
            let admitted = deep_filters(100_000, TermPattern::Variable(Variable::new("o")));
            let refused = deep_filters(
                100_000,
                TermPattern::Triple(Child::new(TriplePattern {
                    subject: TermPattern::Variable(Variable::new("a")),
                    predicate: NamedNodePattern::NamedNode(iri("q")),
                    object: TermPattern::Variable(Variable::new("b")),
                })),
            );
            (
                verdict(check_pattern(&admitted)),
                verdict(check_pattern(&refused)),
            )
        })
        .expect("spawn");
        assert_eq!(outcomes.0, Ok(()));
        let refusal = outcomes
            .1
            .expect_err("the triple term at the bottom is refused");
        assert!(refusal.contains("triple term"), "{refusal}");
    }
}
