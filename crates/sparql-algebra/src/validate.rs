// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Borrowed structural admission for compiler-built query algebra.

use std::collections::BTreeSet;

use crate::{
    AggregateExpression, Expression, Function, GraphPattern, GroundTerm, Literal, NamedNodePattern,
    OrderExpression, ParseError, PropertyPathExpression, Query, Result, TermPattern, TriplePattern,
    Variable,
};

// Unlike a flat graph-combinator spine, expressions, paths and RDF values are
// recursively evaluated. Keep their own envelope within native-stack bounds.
const MAX_VALUE_NESTING: usize = 512;

#[derive(Clone, Copy)]
struct Depth {
    structural: usize,
    values: usize,
}

impl Depth {
    const ROOT: Self = Self {
        structural: 1,
        values: 0,
    };
}

impl Query {
    /// Check the structural invariants needed to evaluate compiler-built algebra.
    ///
    /// Walks borrowed nodes without rendering or parsing SPARQL. Runtime expression
    /// errors (including ill-typed datatype lexical forms) remain runtime errors.
    /// Blank labels remain opaque identifiers, including scope-qualified labels.
    /// Registry admission belongs to the evaluator, which owns those registries.
    ///
    /// # Errors
    /// Refuses invalid absolute IRIs, language tags, binding widths and output-name
    /// collisions, malformed typed calls or ranges, and unsafe recursive nesting.
    pub fn validate(&self) -> Result<()> {
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
        let mut stack = vec![(Node::Pattern(pattern), Depth::ROOT)];
        match self {
            Self::Construct { template, .. } => {
                for quad in template {
                    if let Some(graph) = &quad.graph {
                        named(graph)?;
                    }
                    stack.push((Node::Triple(&quad.triple), Depth::ROOT));
                }
            }
            Self::Describe { targets, .. } => {
                for target in targets {
                    named(target)?;
                }
            }
            Self::Select { .. } | Self::Ask { .. } => {}
        }
        while let Some((node, depth)) = stack.pop() {
            // The parser's brace limit is not an algebra-depth limit: sibling
            // operators can produce a spine as long as its combinator budget.
            // Each nested SELECT may add uncharged query modifiers and leaf
            // wrappers, so reserve eight additional nodes per braced group.
            if depth.structural
                > crate::MAX_GRAPH_PATTERN_NODES + 8 * crate::MAX_GRAPH_PATTERN_DEPTH
            {
                return Err(invalid(
                    "query algebra exceeds the structural nesting limit",
                ));
            }
            if depth.values > MAX_VALUE_NESTING {
                return Err(invalid(
                    "query expression, path or term nesting exceeds the safety limit",
                ));
            }
            let child_depth = Depth {
                structural: depth.structural + 1,
                values: depth.values + usize::from(!matches!(node, Node::Pattern(_))),
            };
            node.check(&mut stack, child_depth)?;
        }
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> ParseError {
    ParseError::syntax(message, 0)
}

fn iri(value: &str) -> Result<()> {
    let parsed = purrdf_iri::parse(value).map_err(|e| invalid(e.to_string()))?;
    if !parsed.has_scheme() {
        return Err(invalid("relative IRI in query algebra"));
    }
    Ok(())
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

enum Node<'a> {
    Pattern(&'a GraphPattern),
    Expr(&'a Expression),
    Path(&'a PropertyPathExpression),
    Triple(&'a TriplePattern),
    Term(&'a TermPattern),
    Ground(&'a GroundTerm),
}

fn order<'a>(values: &'a [OrderExpression], stack: &mut Vec<(Node<'a>, Depth)>, depth: Depth) {
    for OrderExpression::Asc(expr) | OrderExpression::Desc(expr) in values {
        stack.push((Node::Expr(expr), depth));
    }
}

fn aggregate<'a>(
    value: &'a AggregateExpression,
    stack: &mut Vec<(Node<'a>, Depth)>,
    depth: Depth,
) -> Result<()> {
    if let crate::AggregateFunction::Custom(name) = value.function() {
        iri(name.as_str())?;
    }
    for expr in value.args() {
        stack.push((Node::Expr(expr), depth));
    }
    for (_, value) in value.scalarvals() {
        literal(value)?;
    }
    order(value.order_by(), stack, depth);
    Ok(())
}

impl<'a> Node<'a> {
    fn check(self, stack: &mut Vec<(Self, Depth)>, depth: Depth) -> Result<()> {
        match self {
            Self::Pattern(pattern) => Self::pattern(pattern, stack, depth)?,
            Self::Expr(expr) => Self::expression(expr, stack, depth)?,
            Self::Path(path) => Self::path(path, stack, depth)?,
            Self::Triple(triple) => {
                named(&triple.predicate)?;
                stack.push((Self::Term(&triple.subject), depth));
                stack.push((Self::Term(&triple.object), depth));
            }
            Self::Term(term) => match term {
                TermPattern::NamedNode(n) => iri(n.as_str())?,
                TermPattern::Variable(v) => variable(v)?,
                TermPattern::Literal(l) => literal(l)?,
                TermPattern::Triple(t) => stack.push((Self::Triple(t), depth)),
                TermPattern::BlankNode(_) => {}
            },
            Self::Ground(term) => match term {
                GroundTerm::NamedNode(n) => iri(n.as_str())?,
                GroundTerm::Literal(l) => literal(l)?,
                GroundTerm::BlankNode(_) => {}
                GroundTerm::Triple(t) => {
                    if matches!(t.subject, GroundTerm::Literal(_) | GroundTerm::Triple(_)) {
                        return Err(invalid(
                            "a ground triple term requires an IRI or blank subject",
                        ));
                    }
                    iri(t.predicate.as_str())?;
                    stack.push((Self::Ground(&t.subject), depth));
                    stack.push((Self::Ground(&t.object), depth));
                }
            },
        }
        Ok(())
    }

    fn pattern(
        pattern: &'a GraphPattern,
        stack: &mut Vec<(Self, Depth)>,
        depth: Depth,
    ) -> Result<()> {
        use GraphPattern as G;
        match pattern {
            G::Bgp { patterns } => {
                for triple in patterns {
                    stack.push((Self::Triple(triple), depth));
                }
            }
            G::Path {
                subject,
                path,
                object,
            } => {
                stack.extend([
                    (Self::Term(subject), depth),
                    (Self::Path(path), depth),
                    (Self::Term(object), depth),
                ]);
            }
            G::Join { left, right }
            | G::Lateral { left, right }
            | G::Union { left, right }
            | G::Minus { left, right } => {
                stack.extend([(Self::Pattern(left), depth), (Self::Pattern(right), depth)]);
            }
            G::LeftJoin {
                left,
                right,
                expression,
            } => {
                stack.extend([(Self::Pattern(left), depth), (Self::Pattern(right), depth)]);
                if let Some(expr) = expression {
                    stack.push((Self::Expr(expr), depth));
                }
            }
            G::Filter { expr, inner } => {
                stack.extend([(Self::Expr(expr), depth), (Self::Pattern(inner), depth)]);
            }
            G::Graph { name, inner } | G::Service { name, inner, .. } => {
                named(name)?;
                stack.push((Self::Pattern(inner), depth));
            }
            G::Extend {
                inner,
                variable: target,
                expression,
            } => {
                variable(target)?;
                stack.extend([
                    (Self::Pattern(inner), depth),
                    (Self::Expr(expression), depth),
                ]);
            }
            G::Values {
                variables,
                bindings,
            } => {
                distinct_variables(variables)?;
                for row in bindings {
                    if row.len() != variables.len() {
                        return Err(invalid("VALUES row width differs from its variables"));
                    }
                    for term in row.iter().flatten() {
                        stack.push((Self::Ground(term), depth));
                    }
                }
            }
            G::OrderBy { inner, expression } => {
                stack.push((Self::Pattern(inner), depth));
                order(expression, stack, depth);
            }
            G::Project { inner, variables } => {
                // Repeated projection variables are normalized by the result schema.
                for value in variables {
                    variable(value)?;
                }
                stack.push((Self::Pattern(inner), depth));
            }
            G::Distinct { inner } | G::Reduced { inner } | G::Slice { inner, .. } => {
                stack.push((Self::Pattern(inner), depth));
            }
            G::Group {
                inner,
                variables,
                aggregates,
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
                stack.push((Self::Pattern(inner), depth));
                for (_, value) in aggregates {
                    aggregate(value, stack, depth)?;
                }
            }
            G::PropertyFunction(call) => {
                iri(&call.iri)?;
                for term in call.subject_args.iter().chain(&call.object_args) {
                    stack.push((Self::Term(term), depth));
                }
            }
            G::Unfold {
                inner,
                expression,
                element,
                companion,
            } => {
                distinct_variables(std::iter::once(element).chain(companion))?;
                stack.extend([
                    (Self::Pattern(inner), depth),
                    (Self::Expr(expression), depth),
                ]);
            }
        }
        Ok(())
    }

    fn expression(
        expr: &'a Expression,
        stack: &mut Vec<(Self, Depth)>,
        depth: Depth,
    ) -> Result<()> {
        use Expression as E;
        match expr {
            E::NamedNode(n) => iri(n.as_str())?,
            E::Literal(l) => literal(l)?,
            E::Variable(v) | E::Bound(v) => variable(v)?,
            E::Or(a, b)
            | E::And(a, b)
            | E::Equal(a, b)
            | E::SameTerm(a, b)
            | E::Greater(a, b)
            | E::GreaterOrEqual(a, b)
            | E::Less(a, b)
            | E::LessOrEqual(a, b)
            | E::Add(a, b)
            | E::Subtract(a, b)
            | E::Multiply(a, b)
            | E::Divide(a, b) => stack.extend([(Self::Expr(a), depth), (Self::Expr(b), depth)]),
            E::UnaryPlus(x) | E::UnaryMinus(x) | E::Not(x) => stack.push((Self::Expr(x), depth)),
            E::In(x, args) => {
                stack.push((Self::Expr(x), depth));
                for arg in args {
                    stack.push((Self::Expr(arg), depth));
                }
            }
            E::If(a, b, c) => stack.extend([
                (Self::Expr(a), depth),
                (Self::Expr(b), depth),
                (Self::Expr(c), depth),
            ]),
            E::Coalesce(args) => {
                for arg in args {
                    stack.push((Self::Expr(arg), depth));
                }
            }
            E::FunctionCall(function, args) => {
                match function {
                    Function::Custom(n) => iri(n.as_str())?,
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
                }
                for arg in args {
                    stack.push((Self::Expr(arg), depth));
                }
            }
            E::Exists(pattern) => stack.push((Self::Pattern(pattern), depth)),
        }
        Ok(())
    }

    fn path(
        path: &'a PropertyPathExpression,
        stack: &mut Vec<(Self, Depth)>,
        depth: Depth,
    ) -> Result<()> {
        use PropertyPathExpression as P;
        match path {
            P::NamedNode(n) => iri(n.as_str())?,
            P::Reverse(x) | P::ZeroOrMore(x) | P::OneOrMore(x) | P::ZeroOrOne(x) => {
                stack.push((Self::Path(x), depth));
            }
            P::Sequence(a, b) | P::Alternative(a, b) => {
                stack.extend([(Self::Path(a), depth), (Self::Path(b), depth)]);
            }
            P::Range { inner, min, max } => {
                if max.is_some_and(|max| *min > max) {
                    return Err(invalid("path range lower bound exceeds upper bound"));
                }
                stack.push((Self::Path(inner), depth));
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
}
