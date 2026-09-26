// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed structural admission for compiler-built query algebra.

use std::collections::BTreeSet;

use crate::{
    AggregateExpression, Expression, Function, GraphPattern, GroundTerm, Literal, NamedNodePattern,
    OrderExpression, ParseError, PropertyPathExpression, Query, Result, TermPattern, TriplePattern,
    Variable,
};

/// How deeply triple terms may nest on `wasm32`: as deep as [`crate::WASM_HOST_STACK_BUDGET`]
/// admits them written alone, 2 048 levels. A built tree is held to what a parsed one can
/// be, for the host engine's call stack the walks over it run on.
pub(crate) const WASM_TRIPLE_TERM_DEPTH: usize =
    crate::WASM_HOST_STACK_BUDGET / crate::parser::host_stack_cost("triple term");

/// The tallest tree admitted on `wasm32`, in every node kind: the parser's height count,
/// with room for what a parsed tree holds that its count does not see — a triple term is
/// one level of the parser's count and two here (its term and its triple), and each of
/// the nested groups may carry up to eight wrapper nodes.
const WASM_STRUCTURAL_LIMIT: usize = crate::parser::WASM_TREE_HEIGHT_LIMIT
    + 2 * WASM_TRIPLE_TERM_DEPTH
    + 8 * crate::WASM_GRAPH_PATTERN_DEPTH;

/// The tallest run of expression and path nodes admitted on `wasm32`: the parser's
/// height count, with room for the second node some written levels build.
const WASM_VALUE_LIMIT: usize = crate::parser::WASM_TREE_HEIGHT_LIMIT + 256;

/// How deep a node sits: in every node kind (`structural`, the height a walk over the
/// tree descends), in expression and path nodes (`values`), and in triple terms
/// (`terms`: pattern and `VALUES` triple terms, and `TRIPLE` calls, which build one).
#[derive(Clone, Copy)]
struct Depth {
    structural: usize,
    values: usize,
    terms: usize,
}

impl Depth {
    const ROOT: Self = Self {
        structural: 1,
        values: 0,
        terms: 0,
    };

    /// The depth of `node`'s children.
    fn below(self, node: &Node<'_>) -> Self {
        Self {
            structural: self.structural + 1,
            values: self.values + usize::from(matches!(node, Node::Expr(_) | Node::Path(_))),
            terms: self.terms
                + usize::from(matches!(
                    node,
                    Node::Term(TermPattern::Triple(_))
                        | Node::Ground(GroundTerm::Triple(_))
                        | Node::Expr(Expression::FunctionCall(Function::Triple, _))
                )),
        }
    }

    /// Admit a node this deep, or refuse the tree.
    ///
    /// A tree is admitted only if every walk over it — the evaluator's analyses, its
    /// copies, its drop — fits the stack left here, at the per-level charge the parser
    /// builds trees under ([`crate::parser::walkable`]): a compiler-built tree is held to
    /// what a parsed one is, and the limit is the thread's real capacity. `fits` is the
    /// tallest level already found to fit, so the stack is read once per level of
    /// height rather than once per node.
    ///
    /// On `wasm32` the host engine's call stack, which no measurement reaches, also
    /// bounds the walks: the parser's height and host-stack counts, applied to built
    /// trees (see [`WASM_STRUCTURAL_LIMIT`] and [`WASM_TRIPLE_TERM_DEPTH`]).
    fn admit(self, fits: &mut usize) -> Result<()> {
        if self.structural > *fits {
            if !crate::parser::walkable(self.structural) {
                return Err(ParseError::StackExhausted {
                    construct: "query algebra",
                    at: 0,
                });
            }
            *fits = self.structural;
        }
        if cfg!(target_arch = "wasm32")
            && (self.structural > WASM_STRUCTURAL_LIMIT
                || self.values > WASM_VALUE_LIMIT
                || self.terms > WASM_TRIPLE_TERM_DEPTH)
        {
            return Err(ParseError::HostStackExhausted {
                construct: "query algebra",
                at: 0,
            });
        }
        Ok(())
    }
}

impl GraphPattern {
    /// Refuse this pattern when it is too tall for the walks over it: the height half of
    /// [`Query::validate`], without its checks of IRIs, variables and literals, for a
    /// pattern about to be evaluated where the stack left may differ from where it was
    /// admitted — a raw pattern handed to the evaluator, a prepared query run on another
    /// thread, a pre-bound copy.
    ///
    /// Walks borrowed nodes iteratively, so it needs no more stack than it measures.
    /// Returns how deeply the pattern's triple terms nest — pattern and `VALUES` triple
    /// terms, and `TRIPLE` calls, each of which builds one around its arguments — so an
    /// evaluator can reserve the stack walks over them take (see
    /// [`purrdf_stack::reserve`]); `0` when the pattern has none.
    ///
    /// # Errors
    ///
    /// [`ParseError::StackExhausted`] when a walk as tall as the pattern (every node
    /// kind counted: patterns, expressions, paths, terms) does not fit the stack left
    /// on the calling thread past [`purrdf_stack::MARGIN_BYTES`], at the parser's
    /// per-level charge; on `wasm32`, [`ParseError::HostStackExhausted`] when it is also
    /// past the parser's host-stack counts.
    pub fn validate_height(&self) -> Result<usize> {
        let mut stack = vec![(Node::Pattern(self), Depth::ROOT)];
        let mut fits = 0;
        let mut terms = 0;
        while let Some((node, depth)) = stack.pop() {
            depth.admit(&mut fits)?;
            let below = depth.below(&node);
            terms = terms.max(below.terms);
            node.children(&mut stack, below);
        }
        Ok(terms)
    }
}

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
    /// # Errors
    /// Refuses invalid absolute IRIs, language tags, binding widths and output-name
    /// collisions, malformed typed calls or ranges, and empty property-path chains; and,
    /// with [`ParseError::StackExhausted`], a tree too tall for the walks over it to fit
    /// the stack the calling thread has left (on `wasm32`, one past the parser's
    /// host-stack counts with [`ParseError::HostStackExhausted`]). How deeply triple terms nest is bounded by that stack alone.
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
        // The tallest level whose walk has been found to fit the stack: every node no
        // deeper than it needs no second look, so the stack is read once per level of
        // height rather than once per node.
        let mut fits = 0;
        while let Some((node, depth)) = stack.pop() {
            depth.admit(&mut fits)?;
            node.check(&mut stack, depth.below(&node))?;
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

#[derive(Clone, Copy)]
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
            G::Join { left, right } | G::Lateral { left, right } | G::Minus { left, right } => {
                stack.extend([(Self::Pattern(left), depth), (Self::Pattern(right), depth)]);
            }
            G::Union { arms } => {
                stack.extend(arms.iter().map(|arm| (Self::Pattern(arm), depth)));
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
            E::Or(operands) | E::And(operands) => {
                stack.extend(operands.iter().map(|operand| (Self::Expr(operand), depth)));
            }
            E::Arithmetic(first, steps) => {
                stack.push((Self::Expr(first), depth));
                stack.extend(
                    steps
                        .iter()
                        .map(|(_, operand)| (Self::Expr(operand), depth)),
                );
            }
            E::Equal(a, b)
            | E::SameTerm(a, b)
            | E::Greater(a, b)
            | E::GreaterOrEqual(a, b)
            | E::Less(a, b)
            | E::LessOrEqual(a, b) => {
                stack.extend([(Self::Expr(a), depth), (Self::Expr(b), depth)]);
            }
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
            // The empty chain is the zero-length path or the empty relation, which no
            // SPARQL text spells: it could be neither displayed nor forwarded.
            P::Sequence(elements) | P::Alternative(elements) if elements.is_empty() => {
                return Err(invalid(
                    "an empty property-path sequence or alternative has no SPARQL form",
                ));
            }
            P::Sequence(elements) | P::Alternative(elements) => {
                stack.extend(elements.iter().map(|element| (Self::Path(element), depth)));
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

impl Node<'_> {
    /// Push this node's children, as [`Self::check`] does, checking nothing.
    fn children(self, stack: &mut Vec<(Self, Depth)>, depth: Depth) {
        use Expression as E;
        use GraphPattern as G;
        use PropertyPathExpression as P;
        match self {
            Self::Pattern(pattern) => match pattern {
                G::Bgp { patterns } => {
                    stack.extend(patterns.iter().map(|t| (Self::Triple(t), depth)));
                }
                G::Path {
                    subject,
                    path,
                    object,
                } => stack.extend([
                    (Self::Term(subject), depth),
                    (Self::Path(path), depth),
                    (Self::Term(object), depth),
                ]),
                G::Join { left, right } | G::Lateral { left, right } | G::Minus { left, right } => {
                    stack.extend([(Self::Pattern(left), depth), (Self::Pattern(right), depth)]);
                }
                G::LeftJoin {
                    left,
                    right,
                    expression,
                } => {
                    stack.extend([(Self::Pattern(left), depth), (Self::Pattern(right), depth)]);
                    stack.extend(expression.iter().map(|e| (Self::Expr(e), depth)));
                }
                G::Union { arms } => stack.extend(arms.iter().map(|a| (Self::Pattern(a), depth))),
                G::Filter { expr, inner } => {
                    stack.extend([(Self::Expr(expr), depth), (Self::Pattern(inner), depth)]);
                }
                G::Extend {
                    inner, expression, ..
                }
                | G::Unfold {
                    inner, expression, ..
                } => stack.extend([
                    (Self::Pattern(inner), depth),
                    (Self::Expr(expression), depth),
                ]),
                G::Values { bindings, .. } => stack.extend(
                    bindings
                        .iter()
                        .flatten()
                        .flatten()
                        .map(|t| (Self::Ground(t), depth)),
                ),
                G::OrderBy { inner, expression } => {
                    stack.push((Self::Pattern(inner), depth));
                    order(expression, stack, depth);
                }
                G::Graph { inner, .. }
                | G::Service { inner, .. }
                | G::Project { inner, .. }
                | G::Distinct { inner }
                | G::Reduced { inner }
                | G::Slice { inner, .. } => stack.push((Self::Pattern(inner), depth)),
                G::Group {
                    inner, aggregates, ..
                } => {
                    stack.push((Self::Pattern(inner), depth));
                    for (_, value) in aggregates {
                        stack.extend(value.args().iter().map(|e| (Self::Expr(e), depth)));
                        order(value.order_by(), stack, depth);
                    }
                }
                G::PropertyFunction(call) => stack.extend(
                    call.subject_args
                        .iter()
                        .chain(&call.object_args)
                        .map(|t| (Self::Term(t), depth)),
                ),
            },
            Self::Expr(expr) => match expr {
                E::NamedNode(_) | E::Literal(_) | E::Variable(_) | E::Bound(_) => {}
                E::Or(operands) | E::And(operands) | E::Coalesce(operands) => {
                    stack.extend(operands.iter().map(|e| (Self::Expr(e), depth)));
                }
                E::Arithmetic(first, steps) => {
                    stack.push((Self::Expr(first), depth));
                    stack.extend(steps.iter().map(|(_, e)| (Self::Expr(e), depth)));
                }
                E::Equal(a, b)
                | E::SameTerm(a, b)
                | E::Greater(a, b)
                | E::GreaterOrEqual(a, b)
                | E::Less(a, b)
                | E::LessOrEqual(a, b) => {
                    stack.extend([(Self::Expr(a), depth), (Self::Expr(b), depth)]);
                }
                E::UnaryPlus(x) | E::UnaryMinus(x) | E::Not(x) => {
                    stack.push((Self::Expr(x), depth));
                }
                E::In(x, args) => {
                    stack.push((Self::Expr(x), depth));
                    stack.extend(args.iter().map(|e| (Self::Expr(e), depth)));
                }
                E::If(a, b, c) => stack.extend([
                    (Self::Expr(a), depth),
                    (Self::Expr(b), depth),
                    (Self::Expr(c), depth),
                ]),
                E::FunctionCall(_, args) => {
                    stack.extend(args.iter().map(|e| (Self::Expr(e), depth)));
                }
                E::Exists(pattern) => stack.push((Self::Pattern(pattern), depth)),
            },
            Self::Path(path) => match path {
                P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => {}
                P::Reverse(x)
                | P::ZeroOrMore(x)
                | P::OneOrMore(x)
                | P::ZeroOrOne(x)
                | P::Range { inner: x, .. } => stack.push((Self::Path(x), depth)),
                P::Sequence(elements) | P::Alternative(elements) => {
                    stack.extend(elements.iter().map(|e| (Self::Path(e), depth)));
                }
            },
            Self::Triple(triple) => stack.extend([
                (Self::Term(&triple.subject), depth),
                (Self::Term(&triple.object), depth),
            ]),
            Self::Term(term) => {
                if let TermPattern::Triple(t) = term {
                    stack.push((Self::Triple(t), depth));
                }
            }
            Self::Ground(term) => {
                if let GroundTerm::Triple(t) = term {
                    stack.extend([
                        (Self::Ground(&t.subject), depth),
                        (Self::Ground(&t.object), depth),
                    ]);
                }
            }
        }
    }
}
