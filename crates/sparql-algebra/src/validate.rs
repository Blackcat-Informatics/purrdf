// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed structural admission for compiler-built query algebra.

use std::collections::BTreeSet;

use crate::walk::NodeRef;
use crate::{
    AggregateExpression, Expression, Function, GraphPattern, GroundTerm, Literal, NamedNodePattern,
    ParseError, PropertyPathExpression, Query, Result, TermPattern, Variable,
};

impl TermPattern {
    /// How many triple terms this term's longest chain holds, the outermost included:
    /// `0` for an IRI, blank node, literal or variable, `1` for `<<( ?s ?p ?o )>>`, `2`
    /// for `<<( ?s ?p <<( ?s ?p ?o )>> )>>`.
    ///
    /// Counted iteratively, so it needs no more stack however deep the term nests, and
    /// without allocating unless a triple term's subject is itself one.
    #[must_use]
    pub fn triple_term_nesting(&self) -> usize {
        let mut deepest = 0;
        let mut pending: Vec<(&Self, usize)> = Vec::new();
        let mut next = Some((self, 0));
        while let Some((term, above)) = next.take().or_else(|| pending.pop()) {
            if let Self::Triple(triple) = term {
                let depth = above + 1;
                deepest = deepest.max(depth);
                if matches!(triple.subject, Self::Triple(_)) {
                    pending.push((&triple.subject, depth));
                }
                next = Some((&triple.object, depth));
            }
        }
        deepest
    }
}

impl GroundTerm {
    /// How many triple terms this `VALUES` cell's longest chain holds, the outermost
    /// included; see [`TermPattern::triple_term_nesting`].
    #[must_use]
    pub fn triple_term_nesting(&self) -> usize {
        let mut deepest = 0;
        let mut pending: Vec<(&Self, usize)> = Vec::new();
        let mut next = Some((self, 0));
        while let Some((term, above)) = next.take().or_else(|| pending.pop()) {
            if let Self::Triple(triple) = term {
                let depth = above + 1;
                deepest = deepest.max(depth);
                if matches!(triple.subject, Self::Triple(_)) {
                    pending.push((&triple.subject, depth));
                }
                next = Some((&triple.object, depth));
            }
        }
        deepest
    }
}

impl Query {
    /// Check the structural invariants needed to evaluate compiler-built algebra.
    ///
    /// Walks borrowed nodes without rendering or parsing SPARQL. Runtime expression
    /// errors (including ill-typed datatype lexical forms) remain runtime errors.
    /// Blank labels remain opaque identifiers, including scope-qualified labels.
    /// Registry admission belongs to the evaluator, which owns those registries.
    ///
    /// Walks the tree over a work list, so a tree of any height is checked without
    /// recursion; how tall a tree may be to be *evaluated* is the evaluator's to decide.
    ///
    /// # Errors
    /// Refuses invalid absolute IRIs, language tags, binding widths and output-name
    /// collisions, and malformed typed calls or ranges.
    pub fn validate(&self) -> Result<()> {
        self.walk(&mut BTreeSet::new())
    }

    /// The IRIs of every extension function the query calls: each
    /// [`Function::Custom`] in an expression anywhere in the algebra — a `FILTER`, a
    /// `BIND`, a projection, an `ORDER BY`, a `HAVING`, an aggregate's argument, and
    /// inside `EXISTS` / `NOT EXISTS` and sub-queries — sorted and de-duplicated.
    ///
    /// A host that can say which IRIs it cannot evaluate learns, before running the
    /// query, whether the query would reach one. Custom aggregates
    /// ([`crate::AggregateFunction::Custom`]) are a separate seam and are not listed.
    ///
    /// # Errors
    /// Whatever [`Self::validate`] refuses: the walk is the same one.
    pub fn custom_function_calls(&self) -> Result<BTreeSet<String>> {
        let mut calls = BTreeSet::new();
        self.walk(&mut calls)?;
        Ok(calls.into_iter().map(ToOwned::to_owned).collect())
    }

    /// [`Self::validate`]'s walk, recording every [`Function::Custom`] IRI it passes
    /// in `calls`.
    fn walk<'a>(&'a self, calls: &mut BTreeSet<&'a str>) -> Result<()> {
        let (pattern, dataset, base) = match self {
            Self::Select {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Ask {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Construct {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Describe {
                pattern,
                dataset,
                base_iri,
                ..
            } => (pattern, dataset, base_iri),
        };
        for name in dataset.default.iter().chain(&dataset.named) {
            iri(name.as_str())?;
        }
        if let Some(base) = base {
            purrdf_iri::BaseIri::parse(base.as_str()).map_err(|e| invalid(e.to_string()))?;
        }
        let mut stack = vec![NodeRef::Pattern(pattern)];
        match self {
            Self::Construct { template, .. } => {
                for quad in template {
                    if let Some(graph) = &quad.graph {
                        named(graph)?;
                    }
                    stack.push(NodeRef::Triple(&quad.triple));
                }
            }
            Self::Describe { targets, .. } => {
                for target in targets {
                    named(target)?;
                }
            }
            Self::Select { .. } | Self::Ask { .. } => {}
        }
        while let Some(node) = stack.pop() {
            check(node, calls)?;
            node.for_each_child(|child| stack.push(child));
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> ParseError {
    ParseError::syntax(message, 0)
}

/// Accept `value` iff it is a well-formed **absolute** IRI.
///
/// `purrdf_iri::is_absolute` rather than `parse(..)?.has_scheme()`: the two run the
/// same grammar and return the same errors, but `parse` owns a copy of `value` so
/// its component accessors can hand back slices, and this reads one bit and drops
/// it. That copy is a heap `String` per IRI in the query, charged on every
/// `Query::validate` — which a governed SHACL change path used to reach once per
/// focus node. The swap is an ALLOCATION change and not a validation one: what is
/// accepted and what is rejected here is unchanged, which
/// `tests::absolute_iris_are_still_accepted_and_relative_ones_still_refused` pins
/// from both sides.
fn iri(value: &str) -> Result<()> {
    if purrdf_iri::is_absolute(value).map_err(|e| invalid(e.to_string()))? {
        Ok(())
    } else {
        Err(invalid("relative IRI in query algebra"))
    }
}

fn variable(value: &Variable) -> Result<()> {
    // `VARNAME` is position-dependent, so this is a whole-string test and not a
    // per-character one: a name may CONTINUE with a combining mark but may not
    // BEGIN with one, and no position admits `'-'`.
    if !crate::lexer::is_varname(value.as_str()) {
        return Err(invalid("invalid query variable name"));
    }
    Ok(())
}

fn distinct_variables<'a>(values: impl IntoIterator<Item = &'a Variable>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        variable(value)?;
        if !seen.insert(value) {
            return Err(invalid("duplicate output variable in query algebra"));
        }
    }
    Ok(())
}

fn named(value: &NamedNodePattern) -> Result<()> {
    match value {
        NamedNodePattern::NamedNode(n) => iri(n.as_str()),
        NamedNodePattern::Variable(v) => variable(v),
    }
}

fn literal(value: &Literal) -> Result<()> {
    iri(value.datatype().as_str())?;
    match value.language() {
        Some(_) => {
            let expected = if value.direction().is_some() {
                crate::ast::RDF_DIR_LANG_STRING
            } else {
                crate::ast::RDF_LANG_STRING
            };
            if value.datatype().as_str() != expected {
                return Err(invalid(
                    "literal datatype disagrees with its language and direction",
                ));
            }
        }
        None if value.direction().is_some()
            || matches!(
                value.datatype().as_str(),
                crate::ast::RDF_LANG_STRING | crate::ast::RDF_DIR_LANG_STRING
            ) =>
        {
            return Err(invalid(
                "a language-string datatype or direction requires a language tag",
            ));
        }
        None => {}
    }
    if value
        .language()
        .is_some_and(|tag| !crate::parser::is_langtag(tag))
    {
        return Err(invalid("invalid language tag in query algebra"));
    }
    Ok(())
}

/// The checks of `node` itself; its children are checked when they are reached.
/// Every [`Function::Custom`] IRI `node` calls is recorded in `calls`.
fn check<'a>(node: NodeRef<'a>, calls: &mut BTreeSet<&'a str>) -> Result<()> {
    match node {
        NodeRef::Pattern(pattern) => check_pattern(pattern),
        NodeRef::Expr(expr) => check_expression(expr, calls),
        NodeRef::Path(path) => check_path(path),
        NodeRef::Triple(triple) => named(&triple.predicate),
        NodeRef::Term(term) => match term {
            TermPattern::NamedNode(n) => iri(n.as_str()),
            TermPattern::Variable(v) => variable(v),
            TermPattern::Literal(l) => literal(l),
            TermPattern::Triple(_) | TermPattern::BlankNode(_) => Ok(()),
        },
        NodeRef::Ground(term) => match term {
            GroundTerm::NamedNode(n) => iri(n.as_str()),
            GroundTerm::Literal(l) => literal(l),
            GroundTerm::BlankNode(_) => Ok(()),
            GroundTerm::Triple(t) => {
                if matches!(t.subject, GroundTerm::Literal(_) | GroundTerm::Triple(_)) {
                    return Err(invalid(
                        "a ground triple term requires an IRI or blank subject",
                    ));
                }
                iri(t.predicate.as_str())
            }
        },
        NodeRef::Order(_) => Ok(()),
        NodeRef::Aggregate(aggregate) => check_aggregate(aggregate),
    }
}

fn check_aggregate(value: &AggregateExpression) -> Result<()> {
    if let crate::AggregateFunction::Custom(name) = value.function() {
        iri(name.as_str())?;
    }
    for (_, value) in value.scalarvals() {
        literal(value)?;
    }
    Ok(())
}

fn check_pattern(pattern: &GraphPattern) -> Result<()> {
    use GraphPattern as G;
    match pattern {
        G::Bgp { .. }
        | G::Path { .. }
        | G::Join { .. }
        | G::Lateral { .. }
        | G::Minus { .. }
        | G::Union { .. }
        | G::LeftJoin { .. }
        | G::Filter { .. }
        | G::OrderBy { .. }
        | G::Distinct { .. }
        | G::Reduced { .. }
        | G::Slice { .. } => {}
        G::Graph { name, .. } | G::Service { name, .. } => named(name)?,
        G::Extend {
            variable: target, ..
        } => variable(target)?,
        G::Values {
            variables,
            bindings,
        } => {
            distinct_variables(variables)?;
            if bindings.iter().any(|row| row.len() != variables.len()) {
                return Err(invalid("VALUES row width differs from its variables"));
            }
        }
        G::Project { variables, .. } => {
            // Repeated projection variables are normalized by the result schema.
            for value in variables {
                variable(value)?;
            }
        }
        G::Group {
            variables,
            aggregates,
            ..
        } => {
            let mut outputs = BTreeSet::new();
            for value in variables {
                variable(value)?;
                outputs.insert(value);
            }
            for (target, _) in aggregates {
                variable(target)?;
                if !outputs.insert(target) {
                    return Err(invalid(
                        "aggregate output collides with another group output",
                    ));
                }
            }
        }
        G::PropertyFunction(call) => iri(&call.iri)?,
        G::Unfold {
            element, companion, ..
        } => distinct_variables(std::iter::once(element).chain(companion))?,
    }
    Ok(())
}

fn check_expression<'a>(expr: &'a Expression, calls: &mut BTreeSet<&'a str>) -> Result<()> {
    use Expression as E;
    match expr {
        E::NamedNode(n) => iri(n.as_str())?,
        E::Literal(l) => literal(l)?,
        E::Variable(v) | E::Bound(v) => variable(v)?,
        E::FunctionCall(function, args) => match function {
            Function::Custom(n) => {
                iri(n.as_str())?;
                calls.insert(n.as_str());
            }
            Function::Cdt(call) => {
                if crate::CdtFn::from_iri(&call.iri) != Some(call.fn_kind)
                    || !call.fn_kind.arity().admits(args.len())
                {
                    return Err(invalid(
                        "invalid composite datatype function identity or arity",
                    ));
                }
            }
            Function::Purrdf(call) => {
                iri(&call.iri)?;
                if !call.iri.ends_with(call.local_name()) {
                    return Err(invalid("extension function IRI disagrees with its kind"));
                }
            }
            Function::Adjust if args.len() != 2 => {
                return Err(invalid("ADJUST requires two arguments"));
            }
            _ => {}
        },
        E::Or(_)
        | E::And(_)
        | E::Arithmetic(..)
        | E::Equal(..)
        | E::SameTerm(..)
        | E::Greater(..)
        | E::GreaterOrEqual(..)
        | E::Less(..)
        | E::LessOrEqual(..)
        | E::UnaryPlus(_)
        | E::UnaryMinus(_)
        | E::Not(_)
        | E::In(..)
        | E::If(..)
        | E::Coalesce(_)
        | E::Exists(_) => {}
    }
    Ok(())
}

fn check_path(path: &PropertyPathExpression) -> Result<()> {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(n) => iri(n.as_str())?,
        P::Reverse(_)
        | P::ZeroOrMore(_)
        | P::OneOrMore(_)
        | P::ZeroOrOne(_)
        | P::Sequence(_)
        | P::Alternative(_) => {}
        P::Range { min, max, .. } => {
            if max.is_some_and(|max| *min > max) {
                return Err(invalid("path range lower bound exceeds upper bound"));
            }
        }
        P::NegatedPropertySet(values) => {
            for v in values {
                iri(v.predicate.as_str())?;
            }
        }
        P::Wildcard { namespace } => {
            if let Some(n) = namespace {
                iri(n.as_str())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::SparqlParser;

    /// Every extension-function call is listed, wherever the expression sits — a
    /// `FILTER`, a `BIND`, a projection, an aggregate's argument, and a `FILTER` inside
    /// `NOT EXISTS` — and nothing else is: the same IRIs in predicate or object position
    /// and a built-in call are not calls.
    #[test]
    fn every_extension_function_call_is_listed_and_a_predicate_iri_is_not() {
        let query = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/ns#>
                 SELECT ?s (ex:project(?s) AS ?p) (SUM(ex:aggregated(?o)) AS ?sum)
                 WHERE {
                   ?s ex:inFilter ?o .
                   ?s ex:object ex:inFilter .
                   BIND (ex:bound(?o) AS ?b)
                   FILTER (ex:inFilter(?o) && STRLEN(STR(?o)) > 0)
                   FILTER NOT EXISTS { ?s ?q ?r FILTER (ex:inExists(?r)) }
                 }
                 GROUP BY ?s",
            )
            .expect("the query parses");
        let calls: Vec<String> = query
            .custom_function_calls()
            .expect("the query is well-formed")
            .into_iter()
            .collect();
        assert_eq!(
            calls,
            [
                "http://example.org/ns#aggregated",
                "http://example.org/ns#bound",
                "http://example.org/ns#inExists",
                "http://example.org/ns#inFilter",
                "http://example.org/ns#project",
            ]
        );
        let none = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/ns#>
                 SELECT ?s WHERE { ?s ex:inFilter ex:bound FILTER (STRLEN(STR(?s)) > 0) }",
            )
            .expect("the query parses");
        assert!(
            none.custom_function_calls()
                .expect("the query is well-formed")
                .is_empty()
        );
    }
}
