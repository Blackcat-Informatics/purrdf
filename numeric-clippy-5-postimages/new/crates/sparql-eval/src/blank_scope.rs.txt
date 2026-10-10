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

use purrdf_core::ViewTermId;
use purrdf_sparql_algebra::scope::{LabelSource, joins_blank_scope as is_spine};
use purrdf_sparql_algebra::{
    AggregateExpression, Expression, GraphPattern, OrderExpression, Query, TermPattern,
    TriplePattern, Variable,
};

use crate::property_fn_plan::{PrepResult, PreparationError};
use crate::solution::{SolutionSeq, VarSchema};
use crate::workspace::{AdmittedMap, AdmittedVec, LexicalFrame};
use purrdf_lex::allocation::Memory;
#[cfg(test)]
use purrdf_sparql_algebra::scope::{
    spine_leaves, visit_leaf_labels, visit_spine_leaves as any_spine_leaf,
};
use purrdf_sparql_algebra::scope::{
    visit_leaf_labels_with_memory, visit_spine_leaves_with_memory, visit_term_labels_with_memory,
};

/// Whether `variable` is a shared blank this pass renamed: a non-distinguished
/// variable that must never be observed as part of a solution. Its canonical
/// identity cannot be written in SPARQL and differs from a per-leaf blank slot.
pub(crate) fn is_joined_blank(variable: &Variable) -> bool {
    variable.is_hidden()
}

/// The columns of `schema` that are not renamed shared blanks, in order — or `None`
/// when there is no renamed blank to leave out, which is every schema outside a
/// basic graph pattern whose pieces share a label.
/// The same hidden-column identity law, with actual admitted array growth.
pub(crate) fn visible_columns_admitted(
    schema: &VarSchema,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<AdmittedVec<usize>>, crate::EvalError> {
    if !schema.vars().iter().any(is_joined_blank) {
        return Ok(None);
    }
    let mut columns = AdmittedVec::new(workspace);
    for (column, variable) in schema.vars().iter().enumerate() {
        if !is_joined_blank(variable) {
            columns.push(column)?;
        }
    }
    Ok(Some(columns))
}

/// `seq` without its renamed shared-blank columns: the solutions as SPARQL defines
/// them, over the pattern's variables only. Rows are kept one for one — dropping a
/// non-distinguished variable never merges two of them — and `seq` is returned as it
/// is when it carries no such column.
pub(crate) fn without_joined_blanks_admitted<I: ViewTermId>(
    seq: SolutionSeq<I>,
    workspace: &crate::WorkspaceCapability,
) -> Result<SolutionSeq<I>, crate::EvalError> {
    if !seq.schema.vars().iter().any(is_joined_blank) {
        return Ok(seq);
    }
    let mut keep = AdmittedVec::new(workspace);
    for (column, variable) in seq.schema.vars().iter().enumerate() {
        if !is_joined_blank(variable) {
            keep.push(column)?;
        }
    }
    let schema = VarSchema::from_vars_admitted(
        keep.iter().map(|&column| seq.schema.vars()[column].clone()),
        workspace,
    )?
    .shared_admitted(workspace)?;
    let mut rows = crate::solution::RowsBuilder::new(workspace);
    for row in seq.rows {
        rows.push_cells(keep.len(), keep.iter().map(|&column| row[column]))?;
    }
    Ok(SolutionSeq {
        schema,
        rows: rows.finish()?,
    })
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
    let capability = crate::WorkspaceCapability::resident();
    join_shared_blanks_with_memory(
        pattern,
        &mut Memory::new(&mut LexicalFrame::new(&capability)),
    )
    .expect("resident blank-scope preparation")
}

pub(crate) fn join_shared_blanks_with_memory(
    pattern: &GraphPattern,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<Option<GraphPattern>> {
    if !needs_with_memory(Node::Pattern(pattern), memory)? {
        return Ok(None);
    }
    let owned = pattern.clone_with_memory(memory)?;
    rewrite_pattern_with_memory(owned, &mut 0usize, memory).map(Some)
}

/// The shared-blank pass over a whole resident query.
pub(crate) fn join_shared_blanks_in_query(query: &Query) -> Option<Query> {
    let capability = crate::WorkspaceCapability::resident();
    let mut frame = LexicalFrame::new(&capability);
    let mut memory = Memory::new(&mut frame);
    let joined = join_shared_blanks_with_memory(query.pattern(), &mut memory)
        .expect("resident shared-blank traversal")?;
    Some(
        query_with_pattern_with_memory(query, joined, &mut memory)
            .expect("resident query head copy"),
    )
}

/// Carry a replacement WHERE without cloning the discarded original.
pub(crate) fn query_with_pattern_with_memory(
    query: &Query,
    pattern: GraphPattern,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<Query> {
    Ok(query.clone_with_pattern_with_memory(pattern, memory)?)
}

// ---------------------------------------------------------------------------
// The spine
// ---------------------------------------------------------------------------

/// Mutable leaves in the same written order.
fn spine_leaves_mut_with_memory<'a>(
    pattern: &'a mut GraphPattern,
    out: &mut AdmittedVec<&'a mut GraphPattern>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<()> {
    let mut pending = purrdf_lex::walk::WorkList::<&'a mut GraphPattern, 8>::with(pattern);
    while let Some(node) = pending.pop() {
        if !is_spine(node) {
            out.push(node)?;
            continue;
        }
        match node {
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Apply { left, right, .. } => {
                pending.try_push_admitted(&mut **right, memory)?;
                pending.try_push_admitted(&mut **left, memory)?;
            }
            _ => unreachable!("is_spine admits only a Join or Lateral"),
        }
    }
    pending.release_admitted(memory)?;
    Ok(())
}

fn leaf_has_blank_with_memory(
    leaf: &GraphPattern,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<bool> {
    visit_leaf_labels_with_memory(
        leaf,
        LabelSource::All,
        &mut |_, _| Ok::<_, PreparationError>(true),
        memory,
    )
}
fn leaf_has_raw_blank_with_memory(
    leaf: &GraphPattern,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<bool> {
    visit_leaf_labels_with_memory(
        leaf,
        LabelSource::Raw,
        &mut |_, _| Ok::<_, PreparationError>(true),
        memory,
    )
}

/// One original shared-label counting law; borrowed labels avoid copying during detection.
fn shared_label_names<'a>(
    leaves: &[&'a GraphPattern],
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<AdmittedVec<&'a str>> {
    let workspace = memory.admission_mut().workspace().clone();
    let mut output = AdmittedVec::new(&workspace);
    let mut any_raw = false;
    for leaf in leaves {
        if leaf_has_raw_blank_with_memory(leaf, memory)? {
            any_raw = true;
            break;
        }
    }
    if !any_raw {
        return Ok(output);
    }
    let mut per_leaf = AdmittedVec::new(&workspace);
    let mut raw = AdmittedMap::default();
    for leaf in leaves {
        let mut original = AdmittedVec::new(&workspace);
        match leaf {
            GraphPattern::Bgp { patterns } => {
                for triple in patterns {
                    for term in [&triple.subject, &triple.object] {
                        visit_term_labels_with_memory(
                            term,
                            LabelSource::Raw,
                            &mut |label, _| {
                                original.push(label)?;
                                Ok::<_, PreparationError>(false)
                            },
                            memory,
                        )?;
                    }
                }
            }
            GraphPattern::Path {
                subject, object, ..
            } => {
                for term in [subject, object] {
                    visit_term_labels_with_memory(
                        term,
                        LabelSource::Raw,
                        &mut |label, _| {
                            original.push(label)?;
                            Ok::<_, PreparationError>(false)
                        },
                        memory,
                    )?;
                }
            }
            GraphPattern::PropertyFunction(call) => {
                for term in call.subject_args.iter().chain(&call.object_args) {
                    visit_term_labels_with_memory(
                        term,
                        LabelSource::Raw,
                        &mut |label, _| {
                            original.push(label)?;
                            Ok::<_, PreparationError>(false)
                        },
                        memory,
                    )?;
                }
            }
            _ => {}
        }
        for label in original.iter().copied() {
            raw.insert_admitted(label, (), &workspace)?;
        }
        let mut labels = AdmittedVec::new(&workspace);
        visit_leaf_labels_with_memory(
            leaf,
            LabelSource::All,
            &mut |label, _| {
                labels.push(label)?;
                Ok::<_, PreparationError>(false)
            },
            memory,
        )?;
        if !labels.is_empty() {
            labels.as_mut_slice().sort_unstable();
            // De-duplicate borrowed labels without changing original array ownership.
            let mut previous = None;
            labels.retain(|label| {
                let keep = previous != Some(*label);
                previous = Some(*label);
                keep
            });
            per_leaf.push(labels)?;
        }
    }
    if per_leaf.len() < 2 {
        return Ok(output);
    }
    let mut leaf_counts = AdmittedMap::<&str, usize>::default();
    for labels in &per_leaf {
        for &label in labels {
            if let Some(count) = leaf_counts.get_mut(label) {
                *count = count
                    .checked_add(1)
                    .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
            } else {
                leaf_counts.insert_admitted(label, 1, &workspace)?;
            }
        }
    }
    for (&label, &leaves) in leaf_counts.iter() {
        if leaves > 1 && raw.get(label).is_some() {
            output.push(label)?;
        }
    }
    output.as_mut_slice().sort_unstable();
    Ok(output)
}

#[cfg(test)]
fn shared_labels(leaves: &[&GraphPattern]) -> Vec<String> {
    let capability = crate::WorkspaceCapability::resident();
    shared_label_names(
        leaves,
        &mut Memory::new(&mut LexicalFrame::new(&capability)),
    )
    .expect("resident shared-label counting")
    .iter()
    .map(|name| (*name).to_owned())
    .collect()
}

#[cfg(test)]
fn leaf_has_blank(leaf: &GraphPattern) -> bool {
    visit_leaf_labels(leaf, LabelSource::All, &mut |_| true)
}
// ---------------------------------------------------------------------------
// Detection (read-only)
// ---------------------------------------------------------------------------

enum Node<'a> {
    Pattern(&'a GraphPattern),
    Expression(&'a Expression),
}

fn extend_pending<'a>(
    pending: &mut purrdf_lex::walk::WorkList<Node<'a>, 16>,
    nodes: impl IntoIterator<Item = Node<'a>>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<()> {
    for node in nodes {
        pending.try_push_admitted(node, memory)?;
    }
    Ok(())
}

fn aggregate_needs_with_memory(
    aggregate: &AggregateExpression,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<bool> {
    for expression in aggregate.args() {
        if needs_with_memory(Node::Expression(expression), memory)? {
            return Ok(true);
        }
    }
    for order in aggregate.order_by() {
        if needs_with_memory(Node::Expression(order.expression()), memory)? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
fn pattern_needs(pattern: &GraphPattern) -> bool {
    let capability = crate::WorkspaceCapability::resident();
    needs_with_memory(
        Node::Pattern(pattern),
        &mut Memory::new(&mut LexicalFrame::new(&capability)),
    )
    .expect("resident blank-scope detection")
}

fn needs_with_memory(root: Node<'_>, memory: &mut Memory<'_, LexicalFrame>) -> PrepResult<bool> {
    let mut pending = purrdf_lex::walk::WorkList::<Node<'_>, 16>::with(root);
    while let Some(node) = pending.pop() {
        match node {
            Node::Pattern(pattern) if is_spine(pattern) => {
                let mut blank_leaves = 0usize;
                if visit_spine_leaves_with_memory(
                    pattern,
                    &mut |leaf, memory| leaf_has_raw_blank_with_memory(leaf, memory),
                    memory,
                )? {
                    visit_spine_leaves_with_memory(
                        pattern,
                        &mut |leaf, memory| {
                            blank_leaves = blank_leaves
                                .checked_add(usize::from(leaf_has_blank_with_memory(leaf, memory)?))
                                .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                            Ok::<_, PreparationError>(false)
                        },
                        memory,
                    )?;
                }
                if blank_leaves > 1 {
                    let capability = memory.admission_mut().workspace().clone();
                    let mut leaves = AdmittedVec::new(&capability);
                    visit_spine_leaves_with_memory(
                        pattern,
                        &mut |leaf, _| {
                            leaves.push(leaf)?;
                            Ok::<_, PreparationError>(false)
                        },
                        memory,
                    )?;
                    if !shared_label_names(&leaves, memory)?.is_empty() {
                        pending.release_admitted(memory)?;
                        return Ok(true);
                    }
                }
                let first = pending.len();
                visit_spine_leaves_with_memory(
                    pattern,
                    &mut |leaf, memory| {
                        pending.try_push_admitted(Node::Pattern(leaf), memory)?;
                        Ok::<_, PreparationError>(false)
                    },
                    memory,
                )?;
                pending.reverse_top(pending.len() - first);
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
                    pending.try_push_admitted(Node::Pattern(right), memory)?;
                    pending.try_push_admitted(Node::Pattern(left), memory)?;
                }
                GraphPattern::Apply { left, right, policy: _ } => {
                    pending.try_push_admitted(Node::Pattern(right), memory)?; pending.try_push_admitted(Node::Pattern(left), memory)?;
                }
                GraphPattern::Union { arms } => {
                    extend_pending(&mut pending, arms.iter().rev().map(Node::Pattern), memory)?;
                }
                GraphPattern::LeftJoin {
                    left,
                    right,
                    expression,
                } => {
                    if let Some(expr) = expression {
                        pending.try_push_admitted(Node::Expression(expr), memory)?;
                    }
                    pending.try_push_admitted(Node::Pattern(right), memory)?;
                    pending.try_push_admitted(Node::Pattern(left), memory)?;
                }
                GraphPattern::Filter { expr, inner } => {
                    pending.try_push_admitted(Node::Pattern(inner), memory)?;
                    pending.try_push_admitted(Node::Expression(expr), memory)?;
                }
                GraphPattern::Extend {
                    inner, expression, ..
                }
                | GraphPattern::Unfold {
                    inner, expression, ..
                } => {
                    pending.try_push_admitted(Node::Pattern(inner), memory)?;
                    pending.try_push_admitted(Node::Expression(expression), memory)?;
                }
                GraphPattern::Graph { inner, .. }
                | GraphPattern::Project { inner, .. }
                | GraphPattern::Distinct { inner }
                | GraphPattern::Reduced { inner }
                | GraphPattern::Slice { inner, .. } => { pending.try_push_admitted(Node::Pattern(inner), memory)?; },
                GraphPattern::OrderBy { inner, expression } => {
                    extend_pending(&mut pending,
                        expression
                            .iter()
                            .rev()
                            .map(|key| Node::Expression(key.expression())), memory)?;
                    pending.try_push_admitted(Node::Pattern(inner), memory)?;
                }
                GraphPattern::Group {
                    inner, aggregates, ..
                } => {
                    for (_, aggregate) in aggregates.iter().rev() {
                        extend_pending(&mut pending,
                            aggregate
                                .order_by()
                                .iter()
                                .rev()
                                .map(|key| Node::Expression(key.expression())), memory)?;
                        extend_pending(&mut pending, aggregate.args().iter().rev().map(Node::Expression), memory)?;
                    }
                    pending.try_push_admitted(Node::Pattern(inner), memory)?;
                }
            },
            Node::Expression(expr) => match expr {
                Expression::Exists(pattern) => {
                    pending.try_push_admitted(Node::Pattern(pattern), memory)?;
                }
                Expression::Or(operands) | Expression::And(operands) => {
                    extend_pending(
                        &mut pending,
                        operands.iter().rev().map(Node::Expression),
                        memory,
                    )?;
                }
                Expression::Arithmetic(first, steps) => {
                    extend_pending(
                        &mut pending,
                        steps
                            .iter()
                            .rev()
                            .map(|(_, operand)| Node::Expression(operand)),
                        memory,
                    )?;
                    pending.try_push_admitted(Node::Expression(first), memory)?;
                }
                Expression::Equal(a, b)
                | Expression::SameTerm(a, b)
                | Expression::Greater(a, b)
                | Expression::GreaterOrEqual(a, b)
                | Expression::Less(a, b)
                | Expression::LessOrEqual(a, b) => {
                    pending.try_push_admitted(Node::Expression(b), memory)?;
                    pending.try_push_admitted(Node::Expression(a), memory)?;
                }
                Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
                    pending.try_push_admitted(Node::Expression(a), memory)?;
                }
                Expression::If(c, t, e) => {
                    pending.try_push_admitted(Node::Expression(e), memory)?;
                    pending.try_push_admitted(Node::Expression(t), memory)?;
                    pending.try_push_admitted(Node::Expression(c), memory)?;
                }
                Expression::In(needle, haystack) => {
                    extend_pending(
                        &mut pending,
                        haystack.iter().rev().map(Node::Expression),
                        memory,
                    )?;
                    pending.try_push_admitted(Node::Expression(needle), memory)?;
                }
                Expression::Coalesce(items) | Expression::FunctionCall(_, items) => {
                    extend_pending(
                        &mut pending,
                        items.iter().rev().map(Node::Expression),
                        memory,
                    )?;
                }
                Expression::NamedNode(_)
                | Expression::Literal(_)
                | Expression::Variable(_)
                | Expression::Bound(_) => {}
            },
        }
    }
    pending.release_admitted(memory)?;
    Ok(false)
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
        GraphPattern::Apply {
            left,
            right,
            policy: _,
        } => {
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
    descending: AdmittedVec<bool>,
}

/// Take the expressions of every aggregate of a `GROUP BY` node that reaches a
/// shared blank out of the node, in aggregate order (each one's arguments, then its
/// sort keys), leaving `COUNT(*)` in its place. An aggregate's argument list is not
/// mutable in place, so one that reaches a shared blank is rebuilt with the same
/// function, scalar values, sort keys and `DISTINCT` flag. Only expressions change,
/// never their count, so the rebuilt aggregate is as valid as the original.
fn take_aggregates_with_memory(
    node: &mut GraphPattern,
    out: &mut AdmittedVec<Expression>,
    all: bool,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<AdmittedVec<TakenAggregate>> {
    let capability = memory.admission_mut().workspace().clone();
    let mut taken = AdmittedVec::new(&capability);
    let GraphPattern::Group { aggregates, .. } = node else {
        return Ok(taken);
    };
    for (index, (_, aggregate)) in aggregates.iter_mut().enumerate() {
        if !all && !aggregate_needs_with_memory(aggregate, memory)? {
            continue;
        }
        let (function, mut args, scalarvals, mut order_by, distinct) =
            std::mem::replace(aggregate, count_star()).into_parts();
        let mut descending = AdmittedVec::with_capacity(order_by.len(), &capability)?;
        for order in &order_by {
            descending.push(matches!(order, OrderExpression::Desc(_)))?;
        }
        let args_len = args.len();
        taken.push(TakenAggregate {
            index,
            function,
            scalarvals,
            distinct,
            args: args_len,
            descending,
        })?;
        #[expect(
            clippy::iter_with_drain,
            reason = "The original vector stays allocated until Memory destroys it and refunds its grant; drain preserves authored transfer order."
        )]
        for argument in args.drain(..) {
            out.push(argument)?;
        }
        memory.release_vec(args)?;
        #[expect(
            clippy::iter_with_drain,
            reason = "The original vector stays allocated until Memory destroys it and refunds its grant; drain preserves authored transfer order."
        )]
        for order in order_by.drain(..) {
            let (OrderExpression::Asc(expression) | OrderExpression::Desc(expression)) = order;
            out.push(expression)?;
        }
        memory.release_vec(order_by)?;
    }
    Ok(taken)
}

fn restore_aggregates_with_memory(
    node: &mut GraphPattern,
    taken: AdmittedVec<TakenAggregate>,
    rewritten: &mut impl Iterator<Item = Expression>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<()> {
    let GraphPattern::Group { aggregates, .. } = node else {
        debug_assert!(taken.is_empty(), "only GROUP BY has taken aggregates");
        return Ok(());
    };
    for aggregate in taken {
        let args = memory.collect(rewritten.by_ref().take(aggregate.args))?;
        let mut order_by = Vec::new();
        memory.reserve(&mut order_by, aggregate.descending.len())?;
        for &descending in &aggregate.descending {
            let expression = rewritten.next().expect("each taken sort key comes back");
            order_by.push(if descending {
                OrderExpression::Desc(expression)
            } else {
                OrderExpression::Asc(expression)
            });
        }
        aggregates[aggregate.index].1 = AggregateExpression::new(
            aggregate.function,
            args,
            aggregate.scalarvals,
            order_by,
            aggregate.distinct,
        )
        .expect("rewriting preserves original argument count");
    }
    Ok(())
}

/// A rewritten subtree, waiting for its parent to be reassembled.
enum Rewritten {
    Pattern(GraphPattern),
    Expression(Expression),
}

/// A node whose children are away being rewritten, and how many of them there are.
struct Shell {
    node: Rewritten,
    children: usize,
    aggregates: AdmittedVec<TakenAggregate>,
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
#[cfg(test)]
fn rewrite_pattern(pattern: GraphPattern, next_spine: &mut usize) -> GraphPattern {
    resident_owned_pattern(pattern, |pattern, memory| {
        rewrite_pattern_with_memory(pattern, next_spine, memory)
    })
}

fn copy_label_names(
    names: &[&str],
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<Vec<String>> {
    let mut copied = Vec::new();
    memory.reserve(&mut copied, names.len())?;
    for name in names {
        let text = memory.string(name)?;
        copied.push(text);
    }
    Ok(copied)
}
fn release_label_names(
    mut names: Vec<String>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<()> {
    #[expect(
        clippy::iter_with_drain,
        reason = "The original vector stays allocated until Memory destroys it and refunds its grant; drain preserves authored transfer order."
    )]
    for text in names.drain(..) {
        memory.release_string(text)?;
    }
    memory.release_vec(names)?;
    Ok(())
}

fn rewrite_pattern_with_memory(
    pattern: GraphPattern,
    next_spine: &mut usize,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<GraphPattern> {
    map_patterns_with_memory(
        pattern,
        false,
        &mut |node, in_spine, memory| {
            let spine = is_spine(node);
            if spine
                && !in_spine
                && visit_spine_leaves_with_memory(
                    node,
                    &mut |leaf, memory| leaf_has_raw_blank_with_memory(leaf, memory),
                    memory,
                )?
            {
                let capability = memory.admission_mut().workspace().clone();
                let (shared, canonical) = {
                    let mut leaves = AdmittedVec::new(&capability);
                    visit_spine_leaves_with_memory(
                        node,
                        &mut |leaf, _| {
                            leaves.push(leaf)?;
                            Ok::<_, PreparationError>(false)
                        },
                        memory,
                    )?;
                    let shared = shared_label_names(&leaves, memory)?;
                    let mut carried = AdmittedVec::new(&capability);
                    if !shared.is_empty() {
                        for leaf in leaves.iter().copied() {
                            visit_leaf_labels_with_memory(
                                leaf,
                                LabelSource::Carried,
                                &mut |label, _| {
                                    carried.push(label)?;
                                    Ok::<_, PreparationError>(false)
                                },
                                memory,
                            )?;
                        }
                        carried.as_mut_slice().sort_unstable();
                        let mut previous = None;
                        carried.retain(|label| {
                            let keep = previous != Some(*label);
                            previous = Some(*label);
                            keep
                        });
                    }
                    (
                        copy_label_names(&shared, memory)?,
                        copy_label_names(&carried, memory)?,
                    )
                };
                if !shared.is_empty() {
                    let number = *next_spine;
                    *next_spine = next_spine
                        .checked_add(1)
                        .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                    let mut leaves = AdmittedVec::new(&capability);
                    spine_leaves_mut_with_memory(node, &mut leaves, memory)?;
                    for leaf in leaves {
                        rename_labels_with_memory(
                            leaf,
                            &shared,
                            &mut |label, memory| {
                                let workspace = memory.admission_mut().workspace();
                                if canonical
                                    .binary_search_by(|name| name.as_str().cmp(label))
                                    .is_ok()
                                {
                                    Ok(Variable::try_hidden_blank(label, |name| {
                                        workspace.authored_text(name)
                                    })?)
                                } else {
                                    Ok(Variable::try_hidden(
                                        &format_args!("{number}:{label}"),
                                        |name| workspace.authored_text(name),
                                    )?)
                                }
                            },
                            memory,
                        )?;
                    }
                }
                release_label_names(shared, memory)?;
                release_label_names(canonical, memory)?;
            }
            Ok(spine)
        },
        memory,
    )
}

/// Only the explicitly resident raw adapter adopts already caller-owned storage.
#[cfg(test)]
fn resident_owned_pattern(
    pattern: GraphPattern,
    body: impl FnOnce(GraphPattern, &mut Memory<'_, LexicalFrame>) -> PrepResult<GraphPattern>,
) -> GraphPattern {
    let capability = crate::WorkspaceCapability::resident();
    let mut frame = LexicalFrame::new(&capability);
    let mut memory = Memory::new(&mut frame);
    let query = Query::Ask {
        pattern,
        dataset: purrdf_sparql_algebra::QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    let bytes = query
        .raw_owned_bytes_with_memory(&mut memory)
        .expect("resident destruction layout");
    memory
        .add_bytes(bytes)
        .expect("resident ownership adoption");
    let Query::Ask { pattern, .. } = query else {
        unreachable!()
    };
    body(pattern, &mut memory).expect("resident pattern rewrite")
}

/// Map every pattern, including expression and aggregate EXISTS bodies, before
/// its children, using the same iterative reconstruction as blank scoping.
/// The callback's returned context is passed to its immediate pattern children.
pub(crate) fn map_patterns_with_memory(
    pattern: GraphPattern,
    all_aggregates: bool,
    transform: &mut impl FnMut(
        &mut GraphPattern,
        bool,
        &mut Memory<'_, LexicalFrame>,
    ) -> PrepResult<bool>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<GraphPattern> {
    let capability = memory.admission_mut().workspace().clone();
    let mut steps = AdmittedVec::new(&capability);
    steps.push(Step::Pattern(pattern, false))?;
    let mut done: AdmittedVec<Rewritten> = AdmittedVec::new(&capability);
    // The children of the node being taken apart, in visit order, before they are
    // pushed onto `steps` in reverse.
    let mut children: AdmittedVec<Step> = AdmittedVec::new(&capability);
    while let Some(step) = steps.pop() {
        match step {
            Step::Pattern(mut node, in_spine) => {
                let spine = transform(&mut node, in_spine, memory)?;
                let mut failure = None;
                for_each_child_mut(&mut node, &mut |child| {
                    if failure.is_some() {
                        return;
                    }
                    if let Err(error) = children.push(match child {
                        ChildMut::Pattern(inner) => Step::Pattern(
                            std::mem::replace(inner, GraphPattern::empty_bgp()),
                            spine,
                        ),
                        ChildMut::Expression(expr) => {
                            Step::Expression(std::mem::replace(expr, expression_hole()))
                        }
                    }) {
                        failure = Some(error);
                    }
                });
                if let Some(error) = failure {
                    return Err(error.into());
                }
                let mut aggregate_exprs = AdmittedVec::new(&capability);
                let aggregates = take_aggregates_with_memory(
                    &mut node,
                    &mut aggregate_exprs,
                    all_aggregates,
                    memory,
                )?;
                for expression in aggregate_exprs {
                    children.push(Step::Expression(expression))?;
                }
                steps.push(Step::Assemble(Shell {
                    node: Rewritten::Pattern(node),
                    children: children.len(),
                    aggregates,
                }))?;
                while let Some(child) = children.pop() {
                    steps.push(child)?;
                }
            }
            Step::Expression(mut node) => {
                let mut failure = None;
                for_each_operand_mut(&mut node, &mut |child| {
                    if failure.is_some() {
                        return;
                    }
                    if let Err(error) = children.push(match child {
                        ChildMut::Pattern(inner) => Step::Pattern(
                            std::mem::replace(inner, GraphPattern::empty_bgp()),
                            false,
                        ),
                        ChildMut::Expression(expr) => {
                            Step::Expression(std::mem::replace(expr, expression_hole()))
                        }
                    }) {
                        failure = Some(error);
                    }
                });
                if let Some(error) = failure {
                    return Err(error.into());
                }
                steps.push(Step::Assemble(Shell {
                    node: Rewritten::Expression(node),
                    children: children.len(),
                    aggregates: AdmittedVec::new(&capability),
                }))?;
                while let Some(child) = children.pop() {
                    steps.push(child)?;
                }
            }
            Step::Assemble(shell) => {
                let Shell {
                    node,
                    children: count,
                    aggregates,
                } = shell;
                let start = done.len() - count;
                let mut rewritten = done.drain_from(start);
                let mut refill = |child: ChildMut<'_>| match (child, rewritten.next()) {
                    (ChildMut::Pattern(slot), Some(Rewritten::Pattern(value))) => *slot = value,
                    (ChildMut::Expression(slot), Some(Rewritten::Expression(value))) => {
                        *slot = value;
                    }
                    _ => unreachable!("every child comes back as the kind it was taken as"),
                };
                match node {
                    Rewritten::Pattern(mut node) => {
                        for_each_child_mut(&mut node, &mut refill);
                        drop(refill);
                        let mut expressions = rewritten.map(|child| match child {
                            Rewritten::Expression(expr) => expr,
                            Rewritten::Pattern(_) => {
                                unreachable!("an aggregate's expressions come back as expressions")
                            }
                        });
                        restore_aggregates_with_memory(
                            &mut node,
                            aggregates,
                            &mut expressions,
                            memory,
                        )?;
                        let leftover = expressions.next();
                        debug_assert!(leftover.is_none(), "every child taken comes back");
                        drop(expressions);
                        done.push(Rewritten::Pattern(node))?;
                    }
                    Rewritten::Expression(mut node) => {
                        for_each_operand_mut(&mut node, &mut refill);
                        drop(refill);
                        let leftover = rewritten.next();
                        debug_assert!(leftover.is_none(), "every operand taken comes back");
                        drop(rewritten);
                        done.push(Rewritten::Expression(node))?;
                    }
                }
            }
        }
    }
    match done.pop() {
        Some(Rewritten::Pattern(pattern)) if done.is_empty() => Ok(pattern),
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

#[cfg(test)]
fn rename_labels(
    leaf: &mut GraphPattern,
    shared: &[String],
    mint: &mut impl FnMut(&str) -> Variable,
) {
    let capability = crate::WorkspaceCapability::resident();
    rename_labels_with_memory(
        leaf,
        shared,
        &mut |label, _| Ok(mint(label)),
        &mut Memory::new(&mut LexicalFrame::new(&capability)),
    )
    .expect("resident blank rename");
}

fn rename_labels_with_memory(
    leaf: &mut GraphPattern,
    shared: &[String],
    mint: &mut impl FnMut(&str, &mut Memory<'_, LexicalFrame>) -> PrepResult<Variable>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<()> {
    match leaf {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                rename_term_with_memory(&mut triple.subject, shared, mint, memory)?;
                rename_term_with_memory(&mut triple.object, shared, mint, memory)?;
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            rename_term_with_memory(subject, shared, mint, memory)?;
            rename_term_with_memory(object, shared, mint, memory)?;
        }
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter_mut().chain(&mut call.object_args) {
                rename_term_with_memory(term, shared, mint, memory)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// [`rename_leaf`] for one term position, a quoted triple's subject and object
/// included at every level, over a work list.
fn rename_term_with_memory(
    term: &mut TermPattern,
    shared: &[String],
    mint: &mut impl FnMut(&str, &mut Memory<'_, LexicalFrame>) -> PrepResult<Variable>,
    memory: &mut Memory<'_, LexicalFrame>,
) -> PrepResult<()> {
    let mut pending = purrdf_lex::walk::WorkList::<_, 8>::with(term);
    while let Some(term) = pending.pop() {
        match term {
            TermPattern::BlankNode(blank)
                if shared
                    .binary_search_by(|label| label.as_str().cmp(blank.as_str()))
                    .is_ok() =>
            {
                *term = TermPattern::Variable(mint(blank.as_str(), memory)?);
            }
            TermPattern::Triple(triple) => {
                let triple: &mut TriplePattern = triple;
                pending.try_push_admitted(&mut triple.object, memory)?;
                pending.try_push_admitted(&mut triple.subject, memory)?;
            }
            TermPattern::BlankNode(_)
            | TermPattern::NamedNode(_)
            | TermPattern::Literal(_)
            | TermPattern::Variable(_) => {}
        }
    }
    pending.release_admitted(memory)?;
    Ok(())
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
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Apply { left, right, .. }
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
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Apply { left, right, .. }
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
            GraphPattern::Apply {
                left,
                right,
                policy: _,
            } => reference_pattern_needs(left) || reference_pattern_needs(right),
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
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Apply { left, right, .. } => {
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
            GraphPattern::Apply {
                left,
                right,
                policy: _,
            } => {
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
