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
//! A plain-predicate alternative is translated into `UNION` branches inside one
//! source triples block. The parser marks the block's shared blank endpoints and
//! path joints with canonical hidden identities before splitting it. This pass
//! carries those marked identities across the generated branches and reconciles
//! a matching raw label in a sibling block separated by `FILTER`. Raw blanks in
//! independently constructed `UNION` arms remain local to their own leaves.
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

/// Whether `variable` is a shared blank this pass renamed: a non-distinguished
/// variable that must never be observed as part of a solution. Its canonical
/// identity cannot be written in SPARQL and differs from a per-leaf blank slot.
pub(crate) fn is_joined_blank(variable: &Variable) -> bool {
    variable.is_hidden()
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
    let mut next_spine = 0_usize;
    Some(rewrite_pattern(pattern.clone(), &mut next_spine))
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
///
/// A work list stands in for the recursion: a spine node's operands are pushed right
/// first, so the left pops first and the leaves come out left to right, however tall
/// the spine.
fn spine_leaves<'a>(pattern: &'a GraphPattern, out: &mut Vec<&'a GraphPattern>) {
    let mut pending: purrdf_core::SmallVec<[&'a GraphPattern; 8]> = purrdf_core::smallvec![pattern];
    while let Some(node) = pending.pop() {
        match node {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
                if is_spine(node) =>
            {
                pending.push(right);
                pending.push(left);
            }
            _ => out.push(node),
        }
    }
}

/// [`spine_leaves`], mutably.
fn spine_leaves_mut<'a>(pattern: &'a mut GraphPattern, out: &mut Vec<&'a mut GraphPattern>) {
    let mut pending: Vec<&'a mut GraphPattern> = vec![pattern];
    while let Some(node) = pending.pop() {
        if !is_spine(node) {
            out.push(node);
            continue;
        }
        match node {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                pending.push(right);
                pending.push(left);
            }
            _ => unreachable!("is_spine admits only a Join or a Lateral"),
        }
    }
}

/// Push every blank node label a leaf writes: a `Bgp`'s triples, a `Path`'s two
/// endpoints, a call's arguments — quoted triples included. Any other leaf is
/// another basic graph pattern (or none) and contributes nothing.
fn leaf_labels<'a>(leaf: &'a GraphPattern, out: &mut Vec<&'a str>) {
    leaf_labels_with(leaf, out, LabelSource::All);
}

fn leaf_labels_with<'a>(leaf: &'a GraphPattern, out: &mut Vec<&'a str>, source: LabelSource) {
    visit_leaf_labels(leaf, source, &mut |label| {
        out.push(label);
        false
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelSource {
    Raw,
    Carried,
    All,
}

/// Visit a leaf's labels, stopping when the visitor returns true. A UNION exposes
/// only carried parser-block identities, never its arms' local algebra blanks.
fn visit_leaf_labels<'a>(
    leaf: &'a GraphPattern,
    source: LabelSource,
    visit: &mut impl FnMut(&'a str) -> bool,
) -> bool {
    let mut pending: purrdf_core::SmallVec<[_; 8]> = purrdf_core::smallvec![(leaf, source)];
    while let Some((node, source)) = pending.pop() {
        let found = match node {
            GraphPattern::Bgp { patterns } => patterns.iter().any(|triple| {
                visit_term_labels(&triple.subject, source, visit)
                    || visit_term_labels(&triple.object, source, visit)
            }),
            GraphPattern::Path {
                subject, object, ..
            } => {
                visit_term_labels(subject, source, visit)
                    || visit_term_labels(object, source, visit)
            }
            GraphPattern::PropertyFunction(call) => call
                .subject_args
                .iter()
                .chain(&call.object_args)
                .any(|term| visit_term_labels(term, source, visit)),
            GraphPattern::Join { left, right } => {
                pending.push((right, source));
                pending.push((left, source));
                false
            }
            GraphPattern::Union { arms } if source != LabelSource::Raw => {
                pending.extend(arms.iter().rev().map(|arm| (arm, LabelSource::Carried)));
                false
            }
            _ => false,
        };
        if found {
            return true;
        }
    }
    false
}

fn term_labels_with<'a>(term: &'a TermPattern, out: &mut Vec<&'a str>, source: LabelSource) {
    visit_term_labels(term, source, &mut |label| {
        out.push(label);
        false
    });
}

/// The blank labels in a term, subject before object at every quoted level, over
/// an inline work list and with the same early-exit visitor as the leaf walk.
fn visit_term_labels<'a>(
    term: &'a TermPattern,
    source: LabelSource,
    visit: &mut impl FnMut(&'a str) -> bool,
) -> bool {
    let mut pending: purrdf_core::SmallVec<[&'a TermPattern; 8]> = purrdf_core::smallvec![term];
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::BlankNode(blank) if source != LabelSource::Carried => {
                if visit(blank.as_str()) {
                    return true;
                }
            }
            TermPattern::Variable(variable) if source != LabelSource::Raw => {
                if variable.source_blank_label().is_some_and(&mut *visit) {
                    return true;
                }
            }
            TermPattern::Triple(triple) => {
                pending.push(&triple.object);
                pending.push(&triple.subject);
            }
            TermPattern::NamedNode(_)
            | TermPattern::BlankNode(_)
            | TermPattern::Literal(_)
            | TermPattern::Variable(_) => {}
        }
    }
    false
}

/// The labels written in more than one of `leaves`, sorted.
fn shared_labels(leaves: &[&GraphPattern]) -> Vec<String> {
    // A canonical-only spine is already settled. In particular, do not rescan
    // nested UNION subtrees at each enclosing sequence join during preparation.
    if !leaves.iter().any(|leaf| leaf_has_raw_blank(leaf)) {
        return Vec::new();
    }
    let mut per_leaf: Vec<Vec<&str>> = Vec::new();
    let mut raw = crate::DetHashSet::default();
    for leaf in leaves {
        let mut original = Vec::new();
        match leaf {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    term_labels_with(&triple.subject, &mut original, LabelSource::Raw);
                    term_labels_with(&triple.object, &mut original, LabelSource::Raw);
                }
            }
            GraphPattern::Path {
                subject, object, ..
            } => {
                term_labels_with(subject, &mut original, LabelSource::Raw);
                term_labels_with(object, &mut original, LabelSource::Raw);
            }
            GraphPattern::PropertyFunction(call) => {
                for term in call.subject_args.iter().chain(&call.object_args) {
                    term_labels_with(term, &mut original, LabelSource::Raw);
                }
            }
            _ => {}
        }
        raw.extend(original);
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
        .filter(|(label, leaves)| *leaves > 1 && raw.contains(*label))
        .map(|(label, _)| label.to_owned())
        .collect();
    shared.sort_unstable();
    shared
}

// ---------------------------------------------------------------------------
// Detection (read-only)
// ---------------------------------------------------------------------------

/// Whether any leaf under the spine rooted at `pattern` satisfies `test` — the
/// leaves visited left to right and the walk stopped at the first that does, over a
/// work list held inline until a spine is taller than it, so a pattern with nothing to
/// rename allocates nothing (this walk runs on every admission, prepared re-runs
/// included).
fn any_spine_leaf<'a>(
    pattern: &'a GraphPattern,
    test: &mut impl FnMut(&'a GraphPattern) -> bool,
) -> bool {
    let mut pending: purrdf_core::SmallVec<[&'a GraphPattern; 8]> = purrdf_core::smallvec![pattern];
    while let Some(node) = pending.pop() {
        match node {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
                if is_spine(node) =>
            {
                pending.push(right);
                pending.push(left);
            }
            _ => {
                if test(node) {
                    return true;
                }
            }
        }
    }
    false
}

/// Whether a leaf writes any blank node at all.
fn leaf_has_blank(leaf: &GraphPattern) -> bool {
    visit_leaf_labels(leaf, LabelSource::All, &mut |_| true)
}

fn leaf_has_raw_blank(leaf: &GraphPattern) -> bool {
    visit_leaf_labels(leaf, LabelSource::Raw, &mut |_| true)
}

/// One node the detection walk has still to look at.
enum Node<'a> {
    Pattern(&'a GraphPattern),
    Expression(&'a Expression),
}

/// Whether `pattern`, or any pattern under it — through every operator and through
/// the `EXISTS` bodies of its expressions — is a spine whose leaves share a label.
fn pattern_needs(pattern: &GraphPattern) -> bool {
    needs(Node::Pattern(pattern))
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
    needs(Node::Expression(expr))
}

/// The detection walk: every node under `root` is examined once, spines by their
/// leaves' labels and everything else by its parts, over a work list. The answer is
/// the same whichever order the nodes are examined in, and the walk stops at the
/// first spine that shares a label.
fn needs(root: Node<'_>) -> bool {
    let mut pending: purrdf_core::SmallVec<[Node<'_>; 16]> = purrdf_core::smallvec![root];
    while let Some(node) = pending.pop() {
        match node {
            Node::Pattern(pattern) if is_spine(pattern) => {
                // Only a spine with blanks in two of its leaves can share a label, and
                // only that one pays for collecting them.
                let mut blank_leaves = 0_usize;
                if any_spine_leaf(pattern, &mut leaf_has_raw_blank) {
                    any_spine_leaf(pattern, &mut |leaf| {
                        blank_leaves += usize::from(leaf_has_blank(leaf));
                        false
                    });
                }
                if blank_leaves > 1 {
                    let mut leaves = Vec::new();
                    spine_leaves(pattern, &mut leaves);
                    if !shared_labels(&leaves).is_empty() {
                        return true;
                    }
                }
                // The leaves go on the work list leftmost on top, so they are examined
                // in written order.
                let first = pending.len();
                any_spine_leaf(pattern, &mut |leaf| {
                    pending.push(Node::Pattern(leaf));
                    false
                });
                pending[first..].reverse();
            }
            Node::Pattern(pattern) => match pattern {
                GraphPattern::Bgp { .. }
                | GraphPattern::Path { .. }
                | GraphPattern::Values { .. }
                | GraphPattern::PropertyFunction(_)
                // A `SERVICE` body is evaluated by the endpoint it names, as written.
                | GraphPattern::Service { .. } => {}
                GraphPattern::Join { left, right }
                | GraphPattern::Lateral { left, right }
                | GraphPattern::Minus { left, right } => {
                    pending.push(Node::Pattern(right));
                    pending.push(Node::Pattern(left));
                }
                GraphPattern::Union { arms } => {
                    pending.extend(arms.iter().rev().map(Node::Pattern));
                }
                GraphPattern::LeftJoin {
                    left,
                    right,
                    expression,
                } => {
                    if let Some(expr) = expression {
                        pending.push(Node::Expression(expr));
                    }
                    pending.push(Node::Pattern(right));
                    pending.push(Node::Pattern(left));
                }
                GraphPattern::Filter { expr, inner } => {
                    pending.push(Node::Pattern(inner));
                    pending.push(Node::Expression(expr));
                }
                GraphPattern::Extend {
                    inner, expression, ..
                }
                | GraphPattern::Unfold {
                    inner, expression, ..
                } => {
                    pending.push(Node::Pattern(inner));
                    pending.push(Node::Expression(expression));
                }
                GraphPattern::Graph { inner, .. }
                | GraphPattern::Project { inner, .. }
                | GraphPattern::Distinct { inner }
                | GraphPattern::Reduced { inner }
                | GraphPattern::Slice { inner, .. } => pending.push(Node::Pattern(inner)),
                GraphPattern::OrderBy { inner, expression } => {
                    pending.extend(
                        expression
                            .iter()
                            .rev()
                            .map(|key| Node::Expression(key.expression())),
                    );
                    pending.push(Node::Pattern(inner));
                }
                GraphPattern::Group {
                    inner, aggregates, ..
                } => {
                    for (_, aggregate) in aggregates.iter().rev() {
                        pending.extend(
                            aggregate
                                .order_by()
                                .iter()
                                .rev()
                                .map(|key| Node::Expression(key.expression())),
                        );
                        pending.extend(aggregate.args().iter().rev().map(Node::Expression));
                    }
                    pending.push(Node::Pattern(inner));
                }
            },
            Node::Expression(expr) => match expr {
                Expression::Exists(pattern) => pending.push(Node::Pattern(pattern)),
                Expression::Or(operands) | Expression::And(operands) => {
                    pending.extend(operands.iter().rev().map(Node::Expression));
                }
                Expression::Arithmetic(first, steps) => {
                    pending.extend(
                        steps
                            .iter()
                            .rev()
                            .map(|(_, operand)| Node::Expression(operand)),
                    );
                    pending.push(Node::Expression(first));
                }
                Expression::Equal(a, b)
                | Expression::SameTerm(a, b)
                | Expression::Greater(a, b)
                | Expression::GreaterOrEqual(a, b)
                | Expression::Less(a, b)
                | Expression::LessOrEqual(a, b) => {
                    pending.push(Node::Expression(b));
                    pending.push(Node::Expression(a));
                }
                Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                    pending.push(Node::Expression(a));
                }
                Expression::If(c, t, e) => {
                    pending.push(Node::Expression(e));
                    pending.push(Node::Expression(t));
                    pending.push(Node::Expression(c));
                }
                Expression::In(needle, haystack) => {
                    pending.extend(haystack.iter().rev().map(Node::Expression));
                    pending.push(Node::Expression(needle));
                }
                Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
                    pending.extend(items.iter().rev().map(Node::Expression));
                }
                Expression::NamedNode(_)
                | Expression::Literal(_)
                | Expression::Variable(_)
                | Expression::Bound(_) => {}
            },
        }
    }
    false
}

// ---------------------------------------------------------------------------
// The rewrite
// ---------------------------------------------------------------------------

/// A child position of a node, mutably: where a rewritten child is put back.
pub(crate) enum ChildMut<'a> {
    Pattern(&'a mut GraphPattern),
    Expression(&'a mut Expression),
}

/// Call `visit` with each child position of `node` — its sub-patterns and the
/// expressions it evaluates — in the order the rewrite numbers spines through them:
/// a `FILTER`'s expression before its pattern, a `BIND`'s and an `UNFOLD`'s
/// expression before their pattern, an `OPTIONAL`'s left operand, right operand and
/// then its condition, an `ORDER BY`'s pattern before its keys, a `GROUP BY`'s
/// pattern only (its aggregates' expressions are reached through
/// [`take_aggregates`], after it). A `SERVICE` body is left as written.
pub(crate) fn for_each_child_mut<'a>(
    node: &'a mut GraphPattern,
    visit: &mut impl FnMut(ChildMut<'a>),
) {
    match node {
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_)
        | GraphPattern::Service { .. } => {}
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Minus { left, right } => {
            visit(ChildMut::Pattern(left));
            visit(ChildMut::Pattern(right));
        }
        GraphPattern::Union { arms } => {
            for arm in arms.iter_mut() {
                visit(ChildMut::Pattern(arm));
            }
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            visit(ChildMut::Pattern(left));
            visit(ChildMut::Pattern(right));
            if let Some(expr) = expression {
                visit(ChildMut::Expression(expr));
            }
        }
        GraphPattern::Filter { expr, inner } => {
            visit(ChildMut::Expression(expr));
            visit(ChildMut::Pattern(inner));
        }
        GraphPattern::Extend {
            inner, expression, ..
        }
        | GraphPattern::Unfold {
            inner, expression, ..
        } => {
            visit(ChildMut::Expression(expression));
            visit(ChildMut::Pattern(inner));
        }
        GraphPattern::Graph { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => visit(ChildMut::Pattern(inner)),
        GraphPattern::OrderBy { inner, expression } => {
            visit(ChildMut::Pattern(inner));
            for key in expression.iter_mut() {
                match key {
                    OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
                        visit(ChildMut::Expression(expr));
                    }
                }
            }
        }
        GraphPattern::Group { inner, .. } => visit(ChildMut::Pattern(inner)),
    }
}

/// [`for_each_child_mut`] for an expression: its operands in written order, and an
/// `EXISTS` body as a pattern.
fn for_each_operand_mut(expr: &mut Expression, visit: &mut impl FnMut(ChildMut<'_>)) {
    match expr {
        Expression::Exists(pattern) => visit(ChildMut::Pattern(pattern)),
        Expression::Or(operands) | Expression::And(operands) => {
            for operand in operands.iter_mut() {
                visit(ChildMut::Expression(operand));
            }
        }
        Expression::Arithmetic(first, steps) => {
            visit(ChildMut::Expression(first));
            for (_, operand) in steps.iter_mut() {
                visit(ChildMut::Expression(operand));
            }
        }
        Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b) => {
            visit(ChildMut::Expression(a));
            visit(ChildMut::Expression(b));
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            visit(ChildMut::Expression(a));
        }
        Expression::If(c, t, e) => {
            visit(ChildMut::Expression(c));
            visit(ChildMut::Expression(t));
            visit(ChildMut::Expression(e));
        }
        Expression::In(needle, haystack) => {
            visit(ChildMut::Expression(needle));
            for item in haystack.iter_mut() {
                visit(ChildMut::Expression(item));
            }
        }
        Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
            for item in items.iter_mut() {
                visit(ChildMut::Expression(item));
            }
        }
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => {}
    }
}

fn expression_hole() -> Expression {
    Expression::Coalesce(purrdf_sparql_algebra::Args::new())
}

/// `COUNT(*)`, the one aggregate with no argument, which stands in for an aggregate
/// whose expressions are away being rewritten.
fn count_star() -> AggregateExpression {
    AggregateExpression::new(
        purrdf_sparql_algebra::AggregateFunction::Count,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("COUNT(*) is the one aggregate with an empty argument list")
}

/// An aggregate taken apart for the rewrite: what it is rebuilt from, once its
/// arguments and sort keys — `args` expressions, then one per entry of `descending`
/// — come back.
struct TakenAggregate {
    /// Its index in the `GROUP BY`'s aggregate list.
    index: usize,
    function: purrdf_sparql_algebra::AggregateFunction,
    scalarvals: Vec<(String, purrdf_sparql_algebra::Literal)>,
    distinct: bool,
    /// How many arguments it has.
    args: usize,
    /// Its own `ORDER BY` keys' directions, in order.
    descending: Vec<bool>,
}

/// Take the expressions of every aggregate of a `GROUP BY` node that reaches a
/// shared blank out of the node, in aggregate order (each one's arguments, then its
/// sort keys), leaving `COUNT(*)` in its place. An aggregate's argument list is not
/// mutable in place, so one that reaches a shared blank is rebuilt with the same
/// function, scalar values, sort keys and `DISTINCT` flag. Only expressions change,
/// never their count, so the rebuilt aggregate is as valid as the original.
fn take_aggregates(
    node: &mut GraphPattern,
    out: &mut Vec<Expression>,
    all: bool,
) -> Vec<TakenAggregate> {
    let GraphPattern::Group { aggregates, .. } = node else {
        return Vec::new();
    };
    let mut taken = Vec::new();
    for (index, (_, aggregate)) in aggregates.iter_mut().enumerate() {
        if !all && !aggregate_needs(aggregate) {
            continue;
        }
        let (function, args, scalarvals, order_by, distinct) =
            std::mem::replace(aggregate, count_star()).into_parts();
        taken.push(TakenAggregate {
            index,
            function,
            scalarvals,
            distinct,
            args: args.len(),
            descending: order_by
                .iter()
                .map(|key| matches!(key, OrderExpression::Desc(_)))
                .collect(),
        });
        out.extend(args);
        out.extend(order_by.into_iter().map(|key| match key {
            OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => expr,
        }));
    }
    taken
}

/// Put the rewritten expressions of [`take_aggregates`]'s aggregates back, rebuilding
/// each aggregate in its place.
fn restore_aggregates(
    node: &mut GraphPattern,
    taken: Vec<TakenAggregate>,
    rewritten: &mut impl Iterator<Item = Expression>,
) {
    let GraphPattern::Group { aggregates, .. } = node else {
        debug_assert!(taken.is_empty(), "only a GROUP BY has aggregates taken");
        return;
    };
    for aggregate in taken {
        let args: Vec<Expression> = rewritten.take(aggregate.args).collect();
        let order_by: Vec<OrderExpression> = aggregate
            .descending
            .iter()
            .map(|&descending| {
                let expr = rewritten
                    .next()
                    .expect("every sort key taken from an aggregate comes back");
                if descending {
                    OrderExpression::Desc(expr)
                } else {
                    OrderExpression::Asc(expr)
                }
            })
            .collect();
        aggregates[aggregate.index].1 = AggregateExpression::new(
            aggregate.function,
            args,
            aggregate.scalarvals,
            order_by,
            aggregate.distinct,
        )
        .expect("rewriting an argument's patterns keeps the argument count");
    }
}

/// A rewritten subtree, waiting for its parent to be reassembled.
enum Rewritten {
    Pattern(GraphPattern),
    Expression(Expression),
}

/// A node whose children are away being rewritten, and how many of them there are.
enum Shell {
    Pattern {
        node: GraphPattern,
        children: usize,
        aggregates: Vec<TakenAggregate>,
    },
    Expression {
        node: Expression,
        children: usize,
    },
}

/// One step of the rewrite.
enum Step {
    /// Rewrite a pattern. `in_spine` is whether it is an operand of a spine already
    /// renamed at its root, whose own shared labels are settled.
    Pattern(GraphPattern, bool),
    Expression(Expression),
    /// Put a node's rewritten children back and hand the node up.
    Assemble(Shell),
}

/// `pattern` with every spine's shared labels renamed, spines numbered in the order
/// a reading of the pattern reaches them.
///
/// The rewrite consumes the pattern and rebuilds it over a work list: a node is
/// taken apart into a shell with holes where its children were, the children are
/// rewritten in turn, and the shell is refilled once the last of them is done. A
/// spine is renamed at its root before its operands are entered — its leaves
/// renamed left to right — and its operands are then walked as any other node, its
/// leaves rewritten in written order. A `GROUP BY`'s aggregates cannot be written
/// through, so their expressions are taken out with the node's other children and
/// each aggregate is rebuilt as they come back.
fn rewrite_pattern(pattern: GraphPattern, next_spine: &mut usize) -> GraphPattern {
    map_patterns(pattern, false, &mut |node, in_spine| {
        let spine = is_spine(node);
        if spine && !in_spine && any_spine_leaf(node, &mut leaf_has_raw_blank) {
            let mut leaves = Vec::new();
            spine_leaves(node, &mut leaves);
            let shared = shared_labels(&leaves);
            if !shared.is_empty() {
                let mut carried = Vec::new();
                for leaf in &leaves {
                    leaf_labels_with(leaf, &mut carried, LabelSource::Carried);
                }
                let mut canonical = crate::DetHashSet::default();
                for label in carried {
                    if !canonical.contains(label) {
                        canonical.insert(label.to_owned());
                    }
                }
                let number = *next_spine;
                *next_spine += 1;
                let mut leaves = Vec::new();
                spine_leaves_mut(node, &mut leaves);
                for leaf in leaves {
                    rename_labels(leaf, &shared, &mut |label| {
                        if canonical.contains(label) {
                            Variable::hidden_blank(label)
                        } else {
                            Variable::hidden(format!("{number}:{label}"))
                        }
                    });
                }
            }
        }
        spine
    })
}

/// Map every pattern, including expression and aggregate EXISTS bodies, before
/// its children, using the same iterative reconstruction as blank scoping.
/// The callback's returned context is passed to its immediate pattern children.
pub(crate) fn map_patterns(
    pattern: GraphPattern,
    all_aggregates: bool,
    transform: &mut impl FnMut(&mut GraphPattern, bool) -> bool,
) -> GraphPattern {
    let mut steps = vec![Step::Pattern(pattern, false)];
    let mut done: Vec<Rewritten> = Vec::new();
    // The children of the node being taken apart, in visit order, before they are
    // pushed onto `steps` in reverse.
    let mut children: Vec<Step> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Pattern(mut node, in_spine) => {
                let spine = transform(&mut node, in_spine);
                for_each_child_mut(&mut node, &mut |child| {
                    children.push(match child {
                        ChildMut::Pattern(inner) => Step::Pattern(
                            std::mem::replace(inner, GraphPattern::empty_bgp()),
                            spine,
                        ),
                        ChildMut::Expression(expr) => {
                            Step::Expression(std::mem::replace(expr, expression_hole()))
                        }
                    });
                });
                let mut aggregate_exprs = Vec::new();
                let aggregates = take_aggregates(&mut node, &mut aggregate_exprs, all_aggregates);
                children.extend(aggregate_exprs.into_iter().map(Step::Expression));
                steps.push(Step::Assemble(Shell::Pattern {
                    node,
                    children: children.len(),
                    aggregates,
                }));
                steps.extend(std::iter::from_fn(|| children.pop()));
            }
            Step::Expression(mut node) => {
                for_each_operand_mut(&mut node, &mut |child| {
                    children.push(match child {
                        ChildMut::Pattern(inner) => Step::Pattern(
                            std::mem::replace(inner, GraphPattern::empty_bgp()),
                            false,
                        ),
                        ChildMut::Expression(expr) => {
                            Step::Expression(std::mem::replace(expr, expression_hole()))
                        }
                    });
                });
                steps.push(Step::Assemble(Shell::Expression {
                    node,
                    children: children.len(),
                }));
                steps.extend(std::iter::from_fn(|| children.pop()));
            }
            Step::Assemble(shell) => {
                let count = match &shell {
                    Shell::Pattern { children, .. } | Shell::Expression { children, .. } => {
                        *children
                    }
                };
                let mut rewritten = done.drain(done.len() - count..);
                let mut refill = |child: ChildMut<'_>| match (child, rewritten.next()) {
                    (ChildMut::Pattern(slot), Some(Rewritten::Pattern(value))) => *slot = value,
                    (ChildMut::Expression(slot), Some(Rewritten::Expression(value))) => {
                        *slot = value;
                    }
                    _ => unreachable!("every child comes back as the kind it was taken as"),
                };
                match shell {
                    Shell::Pattern {
                        mut node,
                        aggregates,
                        ..
                    } => {
                        for_each_child_mut(&mut node, &mut refill);
                        drop(refill);
                        let mut expressions = rewritten.map(|child| match child {
                            Rewritten::Expression(expr) => expr,
                            Rewritten::Pattern(_) => {
                                unreachable!("an aggregate's expressions come back as expressions")
                            }
                        });
                        restore_aggregates(&mut node, aggregates, &mut expressions);
                        let leftover = expressions.next();
                        debug_assert!(leftover.is_none(), "every child taken comes back");
                        drop(expressions);
                        done.push(Rewritten::Pattern(node));
                    }
                    Shell::Expression { mut node, .. } => {
                        for_each_operand_mut(&mut node, &mut refill);
                        drop(refill);
                        let leftover = rewritten.next();
                        debug_assert!(leftover.is_none(), "every operand taken comes back");
                        drop(rewritten);
                        done.push(Rewritten::Expression(node));
                    }
                }
            }
        }
    }
    match done.pop() {
        Some(Rewritten::Pattern(pattern)) if done.is_empty() => pattern,
        _ => unreachable!("the root's rewrite is the last value assembled"),
    }
}

/// Rename, inside one leaf, every blank whose label is in `shared` to that label's
/// variable for spine number `spine`.
#[cfg(test)]
fn rename_leaf(leaf: &mut GraphPattern, shared: &[String], spine: usize) {
    rename_labels(leaf, shared, &mut |label| {
        Variable::hidden(format!("{spine}:{label}"))
    });
}

fn rename_labels(
    leaf: &mut GraphPattern,
    shared: &[String],
    mint: &mut impl FnMut(&str) -> Variable,
) {
    match leaf {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                rename_term(&mut triple.subject, shared, mint);
                rename_term(&mut triple.object, shared, mint);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            rename_term(subject, shared, mint);
            rename_term(object, shared, mint);
        }
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter_mut().chain(&mut call.object_args) {
                rename_term(term, shared, mint);
            }
        }
        _ => {}
    }
}

/// [`rename_leaf`] for one term position, a quoted triple's subject and object
/// included at every level, over a work list.
fn rename_term(term: &mut TermPattern, shared: &[String], mint: &mut impl FnMut(&str) -> Variable) {
    let mut pending: purrdf_core::SmallVec<[_; 8]> = purrdf_core::smallvec![term];
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::BlankNode(blank)
                if shared
                    .binary_search_by(|label| label.as_str().cmp(blank.as_str()))
                    .is_ok() =>
            {
                *term = TermPattern::Variable(mint(blank.as_str()));
            }
            TermPattern::Triple(triple) => {
                let triple: &mut TriplePattern = triple;
                pending.push(&mut triple.object);
                pending.push(&mut triple.subject);
            }
            TermPattern::BlankNode(_)
            | TermPattern::NamedNode(_)
            | TermPattern::Literal(_)
            | TermPattern::Variable(_) => {}
        }
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
        assert!(text.contains("\\0bgp0:a"), "{text}");
        assert!(!text.contains("BlankNode"), "{text}");
        assert!(join_shared_blanks(&rewritten).is_none());
    }
}

/// The work-list walks against recursive readings of the same rules, over generated
/// shapes, and at a depth no recursive reading could reach on a small stack.
#[cfg(test)]
mod walk_tests {
    use purrdf_sparql_algebra::{
        AggregateFunction, ArithmeticOperator, BlankNode, Chain, Child, GroundTerm, Literal,
        NamedNode, NamedNodePattern, NonEmpty, PropertyFunctionCall, PropertyPathExpression,
    };

    use super::*;

    // ── The recursive references ───────────────────────────────────────────────────

    fn reference_spine_leaves<'a>(pattern: &'a GraphPattern, out: &mut Vec<&'a GraphPattern>) {
        match pattern {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
                if is_spine(pattern) =>
            {
                reference_spine_leaves(left, out);
                reference_spine_leaves(right, out);
            }
            _ => out.push(pattern),
        }
    }

    fn reference_any_spine_leaf(
        pattern: &GraphPattern,
        test: &mut impl FnMut(&GraphPattern) -> bool,
    ) -> bool {
        match pattern {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right }
                if is_spine(pattern) =>
            {
                reference_any_spine_leaf(left, test) || reference_any_spine_leaf(right, test)
            }
            _ => test(pattern),
        }
    }

    fn reference_pattern_needs(pattern: &GraphPattern) -> bool {
        if is_spine(pattern) {
            let mut blank_leaves = 0_usize;
            reference_any_spine_leaf(pattern, &mut |leaf| {
                blank_leaves += usize::from(leaf_has_blank(leaf));
                false
            });
            if blank_leaves > 1 {
                let mut leaves = Vec::new();
                reference_spine_leaves(pattern, &mut leaves);
                if !shared_labels(&leaves).is_empty() {
                    return true;
                }
            }
            return reference_any_spine_leaf(pattern, &mut reference_pattern_needs);
        }
        match pattern {
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_)
            | GraphPattern::Service { .. } => false,
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Minus { left, right } => {
                reference_pattern_needs(left) || reference_pattern_needs(right)
            }
            GraphPattern::Union { arms } => arms.iter().any(reference_pattern_needs),
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                reference_pattern_needs(left)
                    || reference_pattern_needs(right)
                    || expression.as_ref().is_some_and(reference_expression_needs)
            }
            GraphPattern::Filter { expr, inner } => {
                reference_expression_needs(expr) || reference_pattern_needs(inner)
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => reference_expression_needs(expression) || reference_pattern_needs(inner),
            GraphPattern::Graph { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => reference_pattern_needs(inner),
            GraphPattern::OrderBy { inner, expression } => {
                reference_pattern_needs(inner)
                    || expression
                        .iter()
                        .any(|key| reference_expression_needs(key.expression()))
            }
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                reference_pattern_needs(inner)
                    || aggregates
                        .iter()
                        .any(|(_, aggregate)| reference_aggregate_needs(aggregate))
            }
        }
    }

    fn reference_aggregate_needs(aggregate: &AggregateExpression) -> bool {
        aggregate.args().iter().any(reference_expression_needs)
            || aggregate
                .order_by()
                .iter()
                .any(|key| reference_expression_needs(key.expression()))
    }

    fn reference_expression_needs(expr: &Expression) -> bool {
        match expr {
            Expression::Exists(pattern) => reference_pattern_needs(pattern),
            Expression::Or(operands) | Expression::And(operands) => {
                operands.iter().any(reference_expression_needs)
            }
            Expression::Arithmetic(first, steps) => {
                reference_expression_needs(first)
                    || steps
                        .iter()
                        .any(|(_, operand)| reference_expression_needs(operand))
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                reference_expression_needs(a) || reference_expression_needs(b)
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                reference_expression_needs(a)
            }
            Expression::If(c, t, e) => {
                reference_expression_needs(c)
                    || reference_expression_needs(t)
                    || reference_expression_needs(e)
            }
            Expression::In(needle, haystack) => {
                reference_expression_needs(needle)
                    || haystack.iter().any(reference_expression_needs)
            }
            Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
                items.iter().any(reference_expression_needs)
            }
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_) => false,
        }
    }

    fn reference_spine_leaves_mut<'a>(
        pattern: &'a mut GraphPattern,
        out: &mut Vec<&'a mut GraphPattern>,
    ) {
        if !is_spine(pattern) {
            out.push(pattern);
            return;
        }
        match pattern {
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                reference_spine_leaves_mut(left, out);
                reference_spine_leaves_mut(right, out);
            }
            _ => unreachable!("is_spine admits only a Join or a Lateral"),
        }
    }

    fn reference_rewrite_pattern(pattern: &mut GraphPattern, next_spine: &mut usize) {
        if is_spine(pattern) {
            let shared = {
                let mut leaves = Vec::new();
                reference_spine_leaves(pattern, &mut leaves);
                shared_labels(&leaves)
            };
            let mut leaves = Vec::new();
            reference_spine_leaves_mut(pattern, &mut leaves);
            if !shared.is_empty() {
                let spine = *next_spine;
                *next_spine += 1;
                for leaf in &mut leaves {
                    rename_leaf(leaf, &shared, spine);
                }
            }
            for leaf in leaves {
                reference_rewrite_pattern(leaf, next_spine);
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
            | GraphPattern::Minus { left, right } => {
                reference_rewrite_pattern(left, next_spine);
                reference_rewrite_pattern(right, next_spine);
            }
            GraphPattern::Union { arms } => {
                for arm in arms.iter_mut() {
                    reference_rewrite_pattern(arm, next_spine);
                }
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                reference_rewrite_pattern(left, next_spine);
                reference_rewrite_pattern(right, next_spine);
                if let Some(expr) = expression {
                    reference_rewrite_expression(expr, next_spine);
                }
            }
            GraphPattern::Filter { expr, inner } => {
                reference_rewrite_expression(expr, next_spine);
                reference_rewrite_pattern(inner, next_spine);
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => {
                reference_rewrite_expression(expression, next_spine);
                reference_rewrite_pattern(inner, next_spine);
            }
            GraphPattern::Graph { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. } => reference_rewrite_pattern(inner, next_spine),
            GraphPattern::OrderBy { inner, expression } => {
                reference_rewrite_pattern(inner, next_spine);
                for key in expression.iter_mut() {
                    match key {
                        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
                            reference_rewrite_expression(expr, next_spine);
                        }
                    }
                }
            }
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                reference_rewrite_pattern(inner, next_spine);
                for (_, aggregate) in aggregates.iter_mut() {
                    if reference_aggregate_needs(aggregate) {
                        let mut args = aggregate.args().to_vec();
                        for arg in &mut args {
                            reference_rewrite_expression(arg, next_spine);
                        }
                        let mut order_by = aggregate.order_by().to_vec();
                        for key in &mut order_by {
                            match key {
                                OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
                                    reference_rewrite_expression(expr, next_spine);
                                }
                            }
                        }
                        *aggregate = AggregateExpression::new(
                            aggregate.function().clone(),
                            args,
                            aggregate.scalarvals().to_vec(),
                            order_by,
                            aggregate.distinct,
                        )
                        .expect("rewriting an argument's patterns keeps the argument count");
                    }
                }
            }
        }
    }

    fn reference_rewrite_expression(expr: &mut Expression, next_spine: &mut usize) {
        match expr {
            Expression::Exists(pattern) => reference_rewrite_pattern(pattern, next_spine),
            Expression::Or(operands) | Expression::And(operands) => {
                for operand in operands.iter_mut() {
                    reference_rewrite_expression(operand, next_spine);
                }
            }
            Expression::Arithmetic(first, steps) => {
                reference_rewrite_expression(first, next_spine);
                for (_, operand) in steps.iter_mut() {
                    reference_rewrite_expression(operand, next_spine);
                }
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                reference_rewrite_expression(a, next_spine);
                reference_rewrite_expression(b, next_spine);
            }
            Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                reference_rewrite_expression(a, next_spine);
            }
            Expression::If(c, t, e) => {
                reference_rewrite_expression(c, next_spine);
                reference_rewrite_expression(t, next_spine);
                reference_rewrite_expression(e, next_spine);
            }
            Expression::In(needle, haystack) => {
                reference_rewrite_expression(needle, next_spine);
                for item in haystack.iter_mut() {
                    reference_rewrite_expression(item, next_spine);
                }
            }
            Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
                for item in items.iter_mut() {
                    reference_rewrite_expression(item, next_spine);
                }
            }
            Expression::NamedNode(_)
            | Expression::Literal(_)
            | Expression::Variable(_)
            | Expression::Bound(_) => {}
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
                budget: 48,
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

    /// One of three labels, so leaves share labels often, or one of three variables.
    fn term(choices: &mut Choices) -> TermPattern {
        match choices.choose(9) {
            0..=2 => TermPattern::BlankNode(BlankNode::new(format!("b{}", choices.choose(3)))),
            3 | 4 => TermPattern::Variable(Variable::new(format!("v{}", choices.choose(3)))),
            5 => TermPattern::NamedNode(iri("n")),
            6 => TermPattern::Literal(Literal::new_simple("lit")),
            _ if choices.spend() => TermPattern::Triple(Child::new(TriplePattern {
                subject: term(choices),
                predicate: NamedNodePattern::NamedNode(iri("p")),
                object: term(choices),
            })),
            _ => TermPattern::Variable(Variable::new("v0")),
        }
    }

    fn triple(choices: &mut Choices) -> TriplePattern {
        TriplePattern {
            subject: term(choices),
            predicate: NamedNodePattern::NamedNode(iri("p")),
            object: term(choices),
        }
    }

    /// A leaf of a spine: a triples block, a path, or a call.
    fn leaf(choices: &mut Choices) -> GraphPattern {
        match choices.choose(3) {
            0 => GraphPattern::Bgp {
                patterns: (0..=choices.choose(2)).map(|_| triple(choices)).collect(),
            },
            1 => GraphPattern::Path {
                subject: term(choices),
                path: PropertyPathExpression::OneOrMore(Child::new(
                    PropertyPathExpression::NamedNode(iri("p")),
                )),
                object: term(choices),
            },
            _ => call(choices),
        }
    }

    fn call(choices: &mut Choices) -> GraphPattern {
        GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: "http://example.org/rel".to_owned(),
            subject_args: vec![term(choices)],
            object_args: vec![term(choices)],
        })
    }

    /// A spine: a left-deep run of `Join`s over leaves, with a call joined through a
    /// `Lateral` one time in three, and one time in four a non-leaf operand — the
    /// shape a group with a nested block or a call would parse to.
    fn spine(choices: &mut Choices) -> GraphPattern {
        let mut built = leaf(choices);
        for _ in 0..=choices.choose(3) {
            if !choices.spend() {
                break;
            }
            built = match choices.choose(4) {
                0 => GraphPattern::Lateral {
                    left: Child::new(built),
                    right: Child::new(call(choices)),
                },
                1 => GraphPattern::Join {
                    left: Child::new(built),
                    right: Child::new(pattern(choices)),
                },
                _ => GraphPattern::Join {
                    left: Child::new(built),
                    right: Child::new(leaf(choices)),
                },
            };
        }
        built
    }

    fn expression(choices: &mut Choices) -> Expression {
        if !choices.spend() {
            return Expression::Variable(Variable::new("v0"));
        }
        match choices.choose(9) {
            0 => Expression::Variable(Variable::new("v1")),
            1 => Expression::Bound(Variable::new("v2")),
            2 => Expression::Or(
                Chain::try_from(vec![expression(choices), expression(choices)])
                    .expect("two operands"),
            ),
            3 => Expression::Arithmetic(
                Child::new(expression(choices)),
                NonEmpty::try_from(vec![(ArithmeticOperator::Add, expression(choices))])
                    .expect("one step"),
            ),
            4 => Expression::Not(Child::new(expression(choices))),
            5 => Expression::If(
                Child::new(expression(choices)),
                Child::new(expression(choices)),
                Child::new(expression(choices)),
            ),
            6 => Expression::In(
                Child::new(expression(choices)),
                vec![expression(choices)].into(),
            ),
            7 => Expression::FunctionCall(
                purrdf_sparql_algebra::Function::Str,
                vec![expression(choices)].into(),
            ),
            _ => Expression::Exists(Child::new(pattern(choices))),
        }
    }

    fn aggregate(choices: &mut Choices) -> (Variable, AggregateExpression) {
        let (function, order_by) = if choices.choose(2) == 0 {
            (AggregateFunction::Sample, Vec::new())
        } else {
            (
                AggregateFunction::Fold,
                vec![
                    OrderExpression::Desc(expression(choices)),
                    OrderExpression::Asc(expression(choices)),
                ],
            )
        };
        (
            Variable::new("agg"),
            AggregateExpression::new(
                function,
                vec![expression(choices)],
                Vec::new(),
                order_by,
                false,
            )
            .expect("one argument"),
        )
    }

    fn pattern(choices: &mut Choices) -> GraphPattern {
        if !choices.spend() {
            return leaf(choices);
        }
        match choices.choose(15) {
            0..=2 => spine(choices),
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
                variable: Variable::new("x"),
                expression: expression(choices),
            },
            7 => GraphPattern::Minus {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            8 => GraphPattern::Values {
                variables: vec![Variable::new("a")],
                bindings: vec![vec![Some(GroundTerm::NamedNode(iri("g")))]],
            },
            9 => GraphPattern::OrderBy {
                inner: Child::new(pattern(choices)),
                expression: vec![OrderExpression::Asc(expression(choices))],
            },
            10 => GraphPattern::Project {
                inner: Child::new(pattern(choices)),
                variables: vec![Variable::new("v0")],
            },
            11 => GraphPattern::Group {
                inner: Child::new(pattern(choices)),
                variables: vec![Variable::new("v0")],
                aggregates: vec![aggregate(choices), aggregate(choices)],
            },
            12 => GraphPattern::Unfold {
                inner: Child::new(pattern(choices)),
                expression: expression(choices),
                element: Variable::new("e"),
                companion: None,
            },
            13 => GraphPattern::Service {
                name: NamedNodePattern::NamedNode(iri("svc")),
                inner: Child::new(spine(choices)),
                silent: false,
            },
            _ => GraphPattern::Lateral {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
        }
    }

    // ── The tests ──────────────────────────────────────────────────────────────────

    /// The detection walk answers as the recursive reading does, and the rewrite
    /// produces the same tree with the same spine numbering, for every generated shape.
    #[test]
    fn the_walks_agree_with_their_recursive_references_on_generated_shapes() {
        let mut rewritten = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let shape = pattern(&mut choices);

            let mut leaves = Vec::new();
            spine_leaves(&shape, &mut leaves);
            let mut expected_leaves = Vec::new();
            reference_spine_leaves(&shape, &mut expected_leaves);
            assert_eq!(leaves, expected_leaves, "seed {seed}: spine leaves");

            let mut seen = Vec::new();
            any_spine_leaf(&shape, &mut |leaf| {
                seen.push(leaf.clone());
                false
            });
            let mut expected_seen = Vec::new();
            reference_any_spine_leaf(&shape, &mut |leaf| {
                expected_seen.push(leaf.clone());
                false
            });
            assert_eq!(seen, expected_seen, "seed {seed}: spine leaves visited");

            let needs = pattern_needs(&shape);
            assert_eq!(needs, reference_pattern_needs(&shape), "seed {seed}: needs");
            rewritten += usize::from(needs);

            let mut next_spine = 0;
            let ours = rewrite_pattern(shape.clone(), &mut next_spine);
            let mut expected = shape.clone();
            let mut expected_next_spine = 0;
            reference_rewrite_pattern(&mut expected, &mut expected_next_spine);
            assert_eq!(ours, expected, "seed {seed}: rewritten tree");
            assert_eq!(
                next_spine, expected_next_spine,
                "seed {seed}: spines numbered"
            );
            assert_eq!(
                needs,
                ours != shape,
                "seed {seed}: the rewrite changes exactly the shapes detection reports"
            );
            assert!(
                !pattern_needs(&ours),
                "seed {seed}: the rewrite settles every shared label"
            );
        }
        assert!(
            rewritten > 40 && rewritten < 360,
            "the generator produces shapes with and without shared labels ({rewritten} of 400)"
        );
    }

    /// A left-deep spine of `depth` one-triple blocks, every block writing the blank
    /// `_:x` as its subject.
    fn deep_spine(depth: usize) -> GraphPattern {
        let block = |n: usize| GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::BlankNode(BlankNode::new("x")),
                predicate: NamedNodePattern::NamedNode(iri("p")),
                object: TermPattern::Variable(Variable::new(format!("o{n}"))),
            }],
        };
        let mut shape = block(0);
        for n in 1..depth {
            shape = GraphPattern::Join {
                left: Child::new(shape),
                right: Child::new(block(n)),
            };
        }
        shape
    }

    /// `depth` levels of `DISTINCT` over a two-block spine sharing `_:x`, so the
    /// spine is reached, detected and renamed through a hundred thousand wrappers.
    fn deep_wrappers(depth: usize) -> GraphPattern {
        let mut shape = deep_spine(2);
        for _ in 0..depth {
            shape = GraphPattern::Distinct {
                inner: Child::new(shape),
            };
        }
        shape
    }

    /// The shared label is joined across a spine a hundred thousand leaves wide, and
    /// through a hundred thousand wrappers, on a 128 KiB stack: every leaf is renamed
    /// to spine 0's variable and nothing is left to detect.
    #[test]
    fn a_hundred_thousand_level_pattern_is_joined_on_a_128_kib_thread() {
        let (spine_leaves_renamed, wrapped_ok) = purrdf_stack::on_stack(128 * 1024, || {
            let spine = deep_spine(100_000);
            assert!(pattern_needs(&spine));
            let joined = join_shared_blanks(&spine).expect("the label is shared");
            let mut leaves = Vec::new();
            spine_leaves(&joined, &mut leaves);
            let renamed = leaves
                .iter()
                .filter(|leaf| match leaf {
                    GraphPattern::Bgp { patterns } => matches!(
                        &patterns[0].subject,
                        TermPattern::Variable(v) if v.as_str() == "\u{0}bgp0:x"
                    ),
                    _ => false,
                })
                .count();
            assert!(!pattern_needs(&joined));
            assert!(join_shared_blanks(&joined).is_none());

            let wrapped = deep_wrappers(100_000);
            assert!(pattern_needs(&wrapped));
            let joined = join_shared_blanks(&wrapped).expect("the label is shared");
            let wrapped_ok =
                !pattern_needs(&joined) && is_joined_blank(&Variable::new("\u{0}bgp0:x"));
            (renamed, wrapped_ok)
        })
        .expect("spawn");
        assert_eq!(spine_leaves_renamed, 100_000);
        assert!(wrapped_ok);
    }

    /// A canonical-only alternative spine needs no rewrite. Its deeply nested
    /// joins must not rescan their descendant UNIONs, and a second preparation
    /// keeps the same reserved identities without cloning the tree.
    #[test]
    fn canonical_unions_a_hundred_thousand_deep_are_already_scoped() {
        purrdf_stack::on_stack(128 * 1024, || {
            let block = || GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::hidden_blank("witness")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                        "http://example.org/p",
                    )),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            };
            let mut shape = block();
            for _ in 0..100_000 {
                shape = GraphPattern::union(
                    GraphPattern::Join {
                        left: Child::new(block()),
                        right: Child::new(shape),
                    },
                    block(),
                );
            }
            assert!(join_shared_blanks(&shape).is_none());
            assert!(join_shared_blanks(&shape).is_none());
        })
        .expect("spawn a small-stack thread");
    }
}
