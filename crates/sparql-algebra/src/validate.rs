// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed structural admission for compiler-built query algebra.

use std::collections::BTreeSet;

use crate::walk::NodeRef;
use crate::{
    AggregateExpression, Expression, Function, GraphPattern, GroundTerm, Literal, NamedNodePattern,
    ParseError, PropertyPathExpression, Query, Result, TermPattern, Variable,
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

    /// The depth of `node`'s children. An `ORDER BY` key and an aggregate are counted
    /// as part of the node that holds them, not as a level of their own.
    fn below(self, node: NodeRef<'_>) -> Self {
        Self {
            structural: self.structural
                + usize::from(!matches!(node, NodeRef::Order(_) | NodeRef::Aggregate(_))),
            values: self.values + usize::from(matches!(node, NodeRef::Expr(_) | NodeRef::Path(_))),
            terms: self.terms + usize::from(node.is_triple_term()),
        }
    }

    /// Push `node`'s children onto `stack`, each at `self`.
    fn push_children<'a>(self, node: NodeRef<'a>, stack: &mut Vec<(NodeRef<'a>, Self)>) {
        node.for_each_child(|child| stack.push((child, self)));
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
        let mut stack = vec![(NodeRef::Pattern(self), Depth::ROOT)];
        let mut fits = 0;
        let mut terms = 0;
        while let Some((node, depth)) = stack.pop() {
            depth.admit(&mut fits)?;
            let below = depth.below(node);
            terms = terms.max(below.terms);
            below.push_children(node, &mut stack);
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
    /// collisions, and malformed typed calls or ranges; and,
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
        let mut stack = vec![(NodeRef::Pattern(pattern), Depth::ROOT)];
        match self {
            Self::Construct { template, .. } => {
                for quad in template {
                    if let Some(graph) = &quad.graph {
                        named(graph)?;
                    }
                    stack.push((NodeRef::Triple(&quad.triple), Depth::ROOT));
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
            check(node)?;
            depth.below(node).push_children(node, &mut stack);
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
fn check(node: NodeRef<'_>) -> Result<()> {
    match node {
        NodeRef::Pattern(pattern) => check_pattern(pattern),
        NodeRef::Expr(expr) => check_expression(expr),
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

fn check_expression(expr: &Expression) -> Result<()> {
    use Expression as E;
    match expr {
        E::NamedNode(n) => iri(n.as_str())?,
        E::Literal(l) => literal(l)?,
        E::Variable(v) | E::Bound(v) => variable(v)?,
        E::FunctionCall(function, args) => match function {
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
