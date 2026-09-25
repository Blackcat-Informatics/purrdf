// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One blank node label, one non-distinguished variable, across the pieces of
//! the basic graph pattern it is written in.
//!
//! A blank node label in a query pattern is a variable that is never projected,
//! scoped to its basic graph pattern: `{ ?s ex:p _:r . _:r ex:q ?o }` asks for an
//! `?s` and an `?o` joined through SOME node, exactly as the same text with a
//! variable `?r` in both places would, minus the column.
//!
//! The parser does not hand the evaluator a basic graph pattern as one node. A
//! triples block that carries a property-function call or a complex property path
//! becomes a spine of `Join`s and `Lateral`s over several leaves — the data
//! triples as `Bgp` runs, each call as a `PropertyFunction`, each path as a
//! `Path` — and every leaf evaluates its blank nodes as its own and drops their
//! columns before its rows leave. A label written in two of those leaves was
//! therefore two unrelated existentials, and the join between them silently
//! became a cross product.
//!
//! [`join_shared_blanks`] closes that. It runs once, where a query or an UPDATE
//! `WHERE` is admitted (and on the few paths that evaluate algebra without
//! admission: the public raw-algebra entries and the in-process `SERVICE`
//! resolver), and for every such spine it renames each label that
//! occurs in MORE than one leaf into a single variable no query text can spell
//! (NUL-prefixed, and numbered per spine). The leaves then treat it as the
//! variable it is: it gets a column, the spine's joins agree on it, and a call
//! with a shared blank argument is driven with the value its sibling bound — and
//! so is told the position is observed. A label confined to one leaf is left a
//! blank node, so a leaf keeps its own consistency check and its column-less
//! evaluation, and a property-function position written with a blank that occurs
//! nowhere else stays unobserved.
//!
//! # Why the spine, and nothing wider
//!
//! SPARQL forbids one label in two different basic graph patterns, and the parser
//! refuses such a query. A basic graph pattern's pieces are joined by a `Join`,
//! or — for a call — a `Lateral` whose right operand is that call (a `FILTER`
//! between two triples blocks does not end the pattern; it is lifted over the
//! whole group, and the two blocks meet in a `Join`). Every other operator — an
//! `OPTIONAL`, a `UNION`, a `MINUS`, a `GRAPH`, a `LATERAL` group, a `BIND` — sits
//! between two different patterns. So a label shared across the leaves of a spine
//! is exactly a label shared within one basic graph pattern, and renaming stops
//! at every other operator.
//!
//! # Where the renamed columns end
//!
//! The renamed column is carried out of the spine rather than projected away at
//! its root, because a projection node there would read as a sub-`SELECT` to the
//! parameter pushdown and the feasibility planner, both of which stop at one. It
//! is dropped instead wherever a whole solution becomes observable: a query's own
//! projection (whose variable list never names it), `DISTINCT`/`REDUCED`,
//! `COUNT(DISTINCT *)`, `DESCRIBE *`, and a `SELECT` with no projection at all —
//! see [`is_joined_blank`]. Joins elsewhere cannot meet it, because no other
//! basic graph pattern carries that spine's number.

use std::sync::Arc;

use purrdf_core::ViewTermId;
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, GraphPattern, OrderExpression, Query, TermPattern,
    TriplePattern, Variable,
};

use crate::DetHashMap;
use crate::solution::{SolutionSeq, VarSchema};

/// The prefix of a renamed shared blank. NUL cannot occur in a parsed variable
/// name, so nothing a query author writes can collide with it, and it is distinct
/// from the per-leaf blank slot prefix `crate::bgp` uses, so a leaf treats it as a
/// variable with a column rather than as one of its own blanks.
const JOINED_BLANK_PREFIX: &str = "\u{0}bgp";

/// Whether `variable` is a shared blank this pass renamed: a non-distinguished
/// variable that must never be observed as part of a solution.
pub(crate) fn is_joined_blank(variable: &Variable) -> bool {
    variable.as_str().starts_with(JOINED_BLANK_PREFIX)
}

/// The columns of `schema` that are not renamed shared blanks, in order — or `None`
/// when there is no renamed blank to leave out, which is every schema outside a
/// basic graph pattern whose pieces share a label.
pub(crate) fn visible_columns(schema: &VarSchema) -> Option<Vec<usize>> {
    if !schema.vars().iter().any(is_joined_blank) {
        return None;
    }
    Some(
        schema
            .vars()
            .iter()
            .enumerate()
            .filter_map(|(column, variable)| (!is_joined_blank(variable)).then_some(column))
            .collect(),
    )
}

/// `seq` without its renamed shared-blank columns: the solutions as SPARQL defines
/// them, over the pattern's variables only. Rows are kept one for one — dropping a
/// non-distinguished variable never merges two of them — and `seq` is returned as it
/// is when it carries no such column.
pub(crate) fn without_joined_blanks<I: ViewTermId>(seq: SolutionSeq<I>) -> SolutionSeq<I> {
    let Some(keep) = visible_columns(&seq.schema) else {
        return seq;
    };
    let schema = VarSchema::from_vars(keep.iter().map(|&column| seq.schema.vars()[column].clone()));
    let rows = seq
        .rows
        .into_iter()
        .map(|row| keep.iter().map(|&column| row[column]).collect())
        .collect();
    SolutionSeq {
        schema: Arc::new(schema),
        rows,
    }
}

/// `pattern` with every blank node label shared between the leaves of one basic
/// graph pattern renamed to one variable per label, or `None` when no label is
/// shared — every pattern with no blank node in two pieces of one block, which is
/// nearly all of them, and which then costs one read-only walk and no allocation
/// beyond a spine's leaf list.
///
/// Idempotent: the output shares no label between leaves, so a second pass
/// returns `None`.
pub(crate) fn join_shared_blanks(pattern: &GraphPattern) -> Option<GraphPattern> {
    if !pattern_needs(pattern) {
        return None;
    }
    let mut rewritten = pattern.clone();
    let mut next_spine = 0_usize;
    rewrite_pattern(&mut rewritten, &mut next_spine);
    Some(rewritten)
}

/// [`join_shared_blanks`] over a whole query's `WHERE` pattern, for a query that is
/// evaluated without passing through admission (`crate::property_fn_plan`, which
/// applies it to every admitted one). `None` when nothing is shared.
pub(crate) fn join_shared_blanks_in_query(query: &Query) -> Option<Query> {
    let (Query::Select { pattern, .. }
    | Query::Ask { pattern, .. }
    | Query::Construct { pattern, .. }
    | Query::Describe { pattern, .. }) = query;
    let joined = join_shared_blanks(pattern)?;
    let mut query = query.clone();
    let (Query::Select { pattern, .. }
    | Query::Ask { pattern, .. }
    | Query::Construct { pattern, .. }
    | Query::Describe { pattern, .. }) = &mut query;
    *pattern = joined;
    Some(query)
}

// ---------------------------------------------------------------------------
// The spine
// ---------------------------------------------------------------------------

/// Whether `pattern` is a node joining two pieces of one basic graph pattern.
fn is_spine(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Join { .. } => true,
        GraphPattern::Lateral { right, .. } => {
            matches!(&**right, GraphPattern::PropertyFunction(_))
        }
        _ => false,
    }
}

/// The leaves under the spine rooted at `pattern`, in written order.
fn spine_leaves<'a>(pattern: &'a GraphPattern, out: &mut Vec<&'a GraphPattern>) {
    // A walk over the whole query. From an in-process `SERVICE`, which may sit deep in
    // an evaluation, it runs inside a `crate::stack::walk` scope that discards the
    // placeholder; everywhere else no scope is open and this never refuses.
    if crate::stack::walk_is_low("blank node scope") {
        return;
    }
    match pattern {
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
            if is_spine(pattern) =>
        {
            spine_leaves(left, out);
            spine_leaves(right, out);
        }
        _ => out.push(pattern),
    }
}

/// [`spine_leaves`], mutably.
fn spine_leaves_mut<'a>(pattern: &'a mut GraphPattern, out: &mut Vec<&'a mut GraphPattern>) {
    // See `spine_leaves`.
    if crate::stack::walk_is_low("blank node scope") {
        return;
    }
    if !is_spine(pattern) {
        out.push(pattern);
        return;
    }
    match pattern {
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
            spine_leaves_mut(left, out);
            spine_leaves_mut(right, out);
        }
        _ => unreachable!("is_spine admits only a Join or a Lateral"),
    }
}

/// Push every blank node label a leaf writes: a `Bgp`'s triples, a `Path`'s two
/// endpoints, a call's arguments — quoted triples included. Any other leaf is
/// another basic graph pattern (or none) and contributes nothing.
fn leaf_labels<'a>(leaf: &'a GraphPattern, out: &mut Vec<&'a str>) {
    match leaf {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                triple_labels(triple, out);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            term_labels(subject, out);
            term_labels(object, out);
        }
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter().chain(&call.object_args) {
                term_labels(term, out);
            }
        }
        _ => {}
    }
}

fn triple_labels<'a>(triple: &'a TriplePattern, out: &mut Vec<&'a str>) {
    term_labels(&triple.subject, out);
    term_labels(&triple.object, out);
}

fn term_labels<'a>(term: &'a TermPattern, out: &mut Vec<&'a str>) {
    match term {
        TermPattern::BlankNode(blank) => out.push(blank.as_str()),
        TermPattern::Triple(triple) => triple_labels(triple, out),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Variable(_) => {}
    }
}

/// The labels written in more than one of `leaves`, sorted.
fn shared_labels(leaves: &[&GraphPattern]) -> Vec<String> {
    let mut per_leaf: Vec<Vec<&str>> = Vec::new();
    for leaf in leaves {
        let mut labels = Vec::new();
        leaf_labels(leaf, &mut labels);
        if !labels.is_empty() {
            labels.sort_unstable();
            labels.dedup();
            per_leaf.push(labels);
        }
    }
    // A label can only be shared if two leaves write blanks at all.
    if per_leaf.len() < 2 {
        return Vec::new();
    }
    let mut leaf_counts: DetHashMap<&str, usize> = DetHashMap::default();
    for labels in &per_leaf {
        for label in labels {
            *leaf_counts.entry(label).or_insert(0) += 1;
        }
    }
    let mut shared: Vec<String> = leaf_counts
        .into_iter()
        .filter(|(_, leaves)| *leaves > 1)
        .map(|(label, _)| label.to_owned())
        .collect();
    shared.sort_unstable();
    shared
}

// ---------------------------------------------------------------------------
// Detection (read-only)
// ---------------------------------------------------------------------------

/// Whether any leaf under the spine rooted at `pattern` satisfies `test` — the
/// spine walked in place, so a pattern with nothing to rename allocates nothing
/// (this walk runs on every admission, prepared re-runs included).
fn any_spine_leaf(pattern: &GraphPattern, test: &mut impl FnMut(&GraphPattern) -> bool) -> bool {
    // See `spine_leaves`.
    if crate::stack::walk_is_low("blank node scope") {
        return false;
    }
    match pattern {
        GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
            if is_spine(pattern) =>
        {
            any_spine_leaf(left, test) || any_spine_leaf(right, test)
        }
        _ => test(pattern),
    }
}

/// Whether a leaf writes any blank node at all.
fn leaf_has_blank(leaf: &GraphPattern) -> bool {
    fn term(term: &TermPattern) -> bool {
        match term {
            TermPattern::BlankNode(_) => true,
            TermPattern::Triple(t) => triple(t),
            TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Variable(_) => false,
        }
    }
    fn triple(t: &TriplePattern) -> bool {
        term(&t.subject) || term(&t.object)
    }
    match leaf {
        GraphPattern::Bgp { patterns } => patterns.iter().any(triple),
        GraphPattern::Path {
            subject, object, ..
        } => term(subject) || term(object),
        GraphPattern::PropertyFunction(call) => {
            call.subject_args.iter().chain(&call.object_args).any(term)
        }
        _ => false,
    }
}

fn pattern_needs(pattern: &GraphPattern) -> bool {
    // See `spine_leaves`.
    if crate::stack::walk_is_low("blank node scope") {
        return false;
    }
    if is_spine(pattern) {
        // Only a spine with blanks in two of its leaves can share a label, and only
        // that one pays for collecting them.
        let mut blank_leaves = 0_usize;
        any_spine_leaf(pattern, &mut |leaf| {
            blank_leaves += usize::from(leaf_has_blank(leaf));
            false
        });
        if blank_leaves > 1 {
            let mut leaves = Vec::new();
            spine_leaves(pattern, &mut leaves);
            if !shared_labels(&leaves).is_empty() {
                return true;
            }
        }
        return any_spine_leaf(pattern, &mut pattern_needs);
    }
    match pattern {
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_)
        // A `SERVICE` body is evaluated by the endpoint it names, as written.
        | GraphPattern::Service { .. } => false,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => pattern_needs(left) || pattern_needs(right),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            pattern_needs(left)
                || pattern_needs(right)
                || expression.as_ref().is_some_and(expression_needs)
        }
        GraphPattern::Filter { expr, inner } => expression_needs(expr) || pattern_needs(inner),
        GraphPattern::Extend {
            inner, expression, ..
        }
        | GraphPattern::Unfold {
            inner, expression, ..
        } => expression_needs(expression) || pattern_needs(inner),
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => pattern_needs(inner),
        GraphPattern::OrderBy { inner, expression } => {
            pattern_needs(inner) || expression.iter().any(order_needs)
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            pattern_needs(inner)
                || aggregates
                    .iter()
                    .any(|(_, aggregate)| aggregate_needs(aggregate))
        }
    }
}

fn order_needs(order: &OrderExpression) -> bool {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => expression_needs(expr),
    }
}

fn aggregate_needs(aggregate: &AggregateExpression) -> bool {
    aggregate.args().iter().any(expression_needs) || aggregate.order_by().iter().any(order_needs)
}

fn expression_needs(expr: &Expression) -> bool {
    // See `spine_leaves`.
    if crate::stack::walk_is_low("blank node scope") {
        return false;
    }
    match expr {
        Expression::Exists(pattern) => pattern_needs(pattern),
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
        | Expression::Divide(a, b) => expression_needs(a) || expression_needs(b),
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            expression_needs(a)
        }
        Expression::If(c, t, e) => {
            expression_needs(c) || expression_needs(t) || expression_needs(e)
        }
        Expression::In(needle, haystack) => {
            expression_needs(needle) || haystack.iter().any(expression_needs)
        }
        Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
            items.iter().any(expression_needs)
        }
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => false,
    }
}

// ---------------------------------------------------------------------------
// The rewrite
// ---------------------------------------------------------------------------

fn rewrite_pattern(pattern: &mut GraphPattern, next_spine: &mut usize) {
    // See `spine_leaves`.
    if crate::stack::walk_is_low("blank node scope") {
        return;
    }
    if is_spine(pattern) {
        let shared = {
            let mut leaves = Vec::new();
            spine_leaves(pattern, &mut leaves);
            shared_labels(&leaves)
        };
        let mut leaves = Vec::new();
        spine_leaves_mut(pattern, &mut leaves);
        if !shared.is_empty() {
            let spine = *next_spine;
            *next_spine += 1;
            for leaf in &mut leaves {
                rename_leaf(leaf, &shared, spine);
            }
        }
        for leaf in leaves {
            rewrite_pattern(leaf, next_spine);
        }
        return;
    }
    match pattern {
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_)
        | GraphPattern::Service { .. } => {}
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            rewrite_pattern(left, next_spine);
            rewrite_pattern(right, next_spine);
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            rewrite_pattern(left, next_spine);
            rewrite_pattern(right, next_spine);
            if let Some(expr) = expression {
                rewrite_expression(expr, next_spine);
            }
        }
        GraphPattern::Filter { expr, inner } => {
            rewrite_expression(expr, next_spine);
            rewrite_pattern(inner, next_spine);
        }
        GraphPattern::Extend {
            inner, expression, ..
        }
        | GraphPattern::Unfold {
            inner, expression, ..
        } => {
            rewrite_expression(expression, next_spine);
            rewrite_pattern(inner, next_spine);
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => rewrite_pattern(inner, next_spine),
        GraphPattern::OrderBy { inner, expression } => {
            rewrite_pattern(inner, next_spine);
            for order in expression {
                rewrite_order(order, next_spine);
            }
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            rewrite_pattern(inner, next_spine);
            for (_, aggregate) in aggregates {
                if aggregate_needs(aggregate) {
                    *aggregate = rewrite_aggregate(aggregate, next_spine);
                }
            }
        }
    }
}

fn rewrite_order(order: &mut OrderExpression, next_spine: &mut usize) {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
            rewrite_expression(expr, next_spine);
        }
    }
}

/// An aggregate's argument list is not mutable in place, so one that reaches a
/// shared blank is rebuilt with the same function, scalar values, sort keys and
/// `DISTINCT` flag. Only expressions change, never their count, so the rebuilt
/// aggregate is as valid as the original.
fn rewrite_aggregate(
    aggregate: &AggregateExpression,
    next_spine: &mut usize,
) -> AggregateExpression {
    let mut args = aggregate.args().to_vec();
    for arg in &mut args {
        rewrite_expression(arg, next_spine);
    }
    let mut order_by = aggregate.order_by().to_vec();
    for order in &mut order_by {
        rewrite_order(order, next_spine);
    }
    AggregateExpression::new(
        aggregate.function().clone(),
        args,
        aggregate.scalarvals().to_vec(),
        order_by,
        aggregate.distinct,
    )
    .expect("rewriting an argument's patterns keeps the argument count")
}

fn rewrite_expression(expr: &mut Expression, next_spine: &mut usize) {
    // See `spine_leaves`.
    if crate::stack::walk_is_low("blank node scope") {
        return;
    }
    match expr {
        Expression::Exists(pattern) => rewrite_pattern(pattern, next_spine),
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
            rewrite_expression(a, next_spine);
            rewrite_expression(b, next_spine);
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            rewrite_expression(a, next_spine);
        }
        Expression::If(c, t, e) => {
            rewrite_expression(c, next_spine);
            rewrite_expression(t, next_spine);
            rewrite_expression(e, next_spine);
        }
        Expression::In(needle, haystack) => {
            rewrite_expression(needle, next_spine);
            for item in haystack {
                rewrite_expression(item, next_spine);
            }
        }
        Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
            for item in items {
                rewrite_expression(item, next_spine);
            }
        }
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => {}
    }
}

/// Rename, inside one leaf, every blank whose label is in `shared` to that label's
/// variable for spine number `spine`.
fn rename_leaf(leaf: &mut GraphPattern, shared: &[String], spine: usize) {
    match leaf {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                rename_triple(triple, shared, spine);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            rename_term(subject, shared, spine);
            rename_term(object, shared, spine);
        }
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter_mut().chain(&mut call.object_args) {
                rename_term(term, shared, spine);
            }
        }
        _ => {}
    }
}

fn rename_triple(triple: &mut TriplePattern, shared: &[String], spine: usize) {
    rename_term(&mut triple.subject, shared, spine);
    rename_term(&mut triple.object, shared, spine);
}

fn rename_term(term: &mut TermPattern, shared: &[String], spine: usize) {
    match term {
        TermPattern::BlankNode(blank)
            if shared
                .binary_search_by(|label| label.as_str().cmp(blank.as_str()))
                .is_ok() =>
        {
            *term = TermPattern::Variable(Variable::new(format!(
                "{JOINED_BLANK_PREFIX}{spine}:{}",
                blank.as_str()
            )));
        }
        TermPattern::Triple(triple) => rename_triple(triple, shared, spine),
        TermPattern::BlankNode(_)
        | TermPattern::NamedNode(_)
        | TermPattern::Literal(_)
        | TermPattern::Variable(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_algebra::SparqlParser;

    use super::*;

    fn where_of(text: &str) -> GraphPattern {
        let query = SparqlParser::new().parse_query(text).expect("parses");
        let Query::Select { pattern, .. } = query else {
            panic!("a SELECT");
        };
        pattern
    }

    #[test]
    fn a_pattern_with_no_shared_label_is_left_alone() {
        for text in [
            "SELECT * WHERE { ?s <http://example.org/p> ?o }",
            "SELECT * WHERE { ?s <http://example.org/p> _:a . _:a <http://example.org/q> ?o }",
            "SELECT * WHERE { ?s <http://example.org/p> _:a . ?s <http://example.org/q>+ _:b }",
        ] {
            assert!(join_shared_blanks(&where_of(text)).is_none(), "{text}");
        }
    }

    #[test]
    fn a_label_shared_between_a_path_and_a_triple_becomes_one_variable_and_the_pass_is_idempotent()
    {
        let pattern = where_of(
            "SELECT ?s WHERE { ?s <http://example.org/p> _:a . _:a <http://example.org/q>+ ?o }",
        );
        let rewritten = join_shared_blanks(&pattern).expect("the label is shared");
        let text = format!("{rewritten:?}");
        assert!(text.contains("\u{0}bgp0:a"), "{text}");
        assert!(!text.contains("BlankNode"), "{text}");
        assert!(join_shared_blanks(&rewritten).is_none());
    }
}
