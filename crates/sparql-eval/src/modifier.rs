// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Solution modifiers and the `VALUES` / `GRAPH` graph-pattern nodes:
//! `Project`, `Distinct`, `Reduced`, `OrderBy`, `Slice`, plus inline `VALUES` data
//! and named-graph scoping.
//!
//! # Aggregate semantics: the deterministic reading this crate ships
//!
//! SPARQL 1.1/1.2 leave several corners of aggregate evaluation intentionally
//! underspecified — a conforming engine may pick any answer within the spec's
//! envelope. This crate picks ONE deterministic answer for each and documents
//! it here, so "what does GROUP_CONCAT return" has a single, testable meaning
//! rather than "any order the engine happened to produce."
//!
//! ## Row and group order (§18.6.1 "Aggregate Algebra")
//!
//! `Group` partitions the inner solution sequence into groups; this crate keeps
//! groups in **first-seen order** (the order their key first appears in the
//! inner sequence — see `eval_group`'s `groups` map, ordinal-assigned on first
//! insert) and keeps each group's own rows in **inner-operator order** (the
//! order `eval_group`'s row scan visits them, i.e. the order the ungrouped
//! input produced them). Every order-sensitive fold — `GROUP_CONCAT`'s
//! concatenation, `SAMPLE`'s "first value wins", a custom `OrderDependent`
//! aggregate — folds over rows in exactly that order, and `DISTINCT` (see
//! below) keeps the FIRST occurrence in that same order, never an arbitrary
//! one.
//!
//! ## `DISTINCT`
//!
//! Per §18.6.1's `Aggregation` definition, `DISTINCT` folds `Dedup(M(Ψ))`
//! rather than `M(Ψ)` — an order-preserving, duplicate-free view whose relative
//! order of first occurrences is preserved. This crate's dedup is exactly that:
//! the first occurrence (in row order, per the paragraph above) of an
//! equal-by-value tuple is kept, every later occurrence is discarded before it
//! ever reaches `step` — see `eval_aggregate`'s and `eval_custom_aggregate`'s
//! `seen` sets.
//!
//! ## `GROUP_CONCAT`'s result
//!
//! §18.6.1.7 defines `GroupConcat` as concatenating the sequence's elements
//! with `sep` between them — but leaves the sequence's own order unspecified
//! ("The order of the strings is not specified"), which is exactly the freedom
//! the paragraph above pins down: THIS crate concatenates in the row order
//! stated above (groups first-seen, rows inner-operator order, `DISTINCT`
//! keeping the first occurrence), producing a plain `xsd:string` of the
//! **lexical forms** joined by `sep` (default `" "` per §18.6.1.7, absent an
//! explicit `SEPARATOR`). See [`GroupConcatAccumulator`] and [`lexical_of`]
//! for which terms contribute a lexical form and which poison the fold.
//!
//! ## Aggregate error handling (§18.6.1's `ListEval`, and where this crate's
//! reading diverges)
//!
//! The spec's `ListEval` unifies two causes into one value: an expression that
//! raises a type error, and an expression that evaluates over an unbound
//! variable, BOTH become the single value `error` in the flattened list — and
//! `error` elements are then dropped before any set function runs (`Count`
//! counts "a bound, non-error value"; every other set function's `Flatten`
//! implicitly works over the error-free list the note under `ListEval` states:
//! "solutions containing error values are removed at the end of evaluating the
//! group and any aggregation functions").
//!
//! This crate's `eval_expr` cannot express that: its signature is
//! `Result<Option<SolutionTerm>, EvalError>`, which has only three states — a
//! bound value, an honestly UNBOUND variable (`Ok(None)`), or a HARD evaluation
//! failure (`Err`, e.g. an XPath-function type error) — not the spec's single
//! unified `error`. `eval_aggregate`/`eval_custom_aggregate` map the spec's
//! "error → remove from the list" reading onto exactly ONE of those three: an
//! honestly unbound argument (`Ok(None)`) skips the row, matching the spec's
//! removal for THAT cause. A hard evaluation failure (`Err`) is propagated via
//! `?` and aborts the whole query, NOT removed-and-continued — diverging from
//! the spec's literal reading for that cause, but consistent with this crate's
//! hard-fail doctrine every other expression-evaluation seam already follows
//! (a raised type error is a defect to surface, never a solution to silently
//! vanish). `SUM`/`AVG` layer a THIRD, aggregate-specific error on top: a
//! non-numeric or overflowing running total *poisons the fold* — represented
//! as [`NumericFold`] going from `Some` to `None` inside [`SumAccumulator`]/
//! [`AvgAccumulator`] (see their docs) — rather than raising `Err` — the SPARQL 1.1/1.2
//! aggregate algebra has no notion of a "poisoned" set-function result, but an
//! unbound aggregate OUTPUT is exactly the shape the spec already uses for
//! `MinList`/`MaxList`/`Sample`'s empty-group `error` (see below), so this
//! crate reuses that shape for a mid-fold numeric failure instead of aborting
//! the query over one bad group.
//!
//! ## `MIN`/`MAX` comparison order (§18.6.1.5 Min, §18.6.1.6 Max)
//!
//! Both are defined via the SPARQL `ORDER BY` total order (§15.1): `Min(S)` is
//! the first element of `Flatten(S)` ordered `ASC`; `Max(S)` is the first
//! element ordered `DESC`. This crate's [`fold_extreme`] runs the SAME
//! [`project`]/[`total_order`] relation `ORDER BY` itself runs (unbound < blank
//! < IRI < literal < triple; literals by comparability class, then value space,
//! then a deterministic syntactic fallback) — so a tie (neither strictly
//! less-than the other under
//! that order) keeps the EARLIER occurrence, exactly as `MinList`/`MaxList`'s
//! "first element of the ordered list" reading demands when the ordering does
//! not distinguish them. An empty group's spec answer is `error`
//! (`Card(L) = 0 ⇒ MinList(L) = error`); this crate reports that as unbound
//! (`None`), the same unbound-for-error reading `AVG`/`SAMPLE` already use for
//! their own empty-group `error` case.

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use purrdf_core::{DatasetView, GraphMatch, TermValue, ViewTermId};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, NamedNodePattern,
    OrderExpression, Variable,
};
use purrdf_xsd::{
    BigInt, XsdDatatype, XsdValue, numeric_add, numeric_div, parse_by_iri, value_total_cmp,
};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

use crate::agg_fn::AggregateAccumulator as _;
use crate::convert::{ground_term_to_value, literal_to_value, named_node_to_value};
use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::expr::{eval_expr, xsd_of};
use crate::governor::ChargePoint;
use crate::governor::lift::{Evaluated, Lift, Truncation};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
use crate::user_fn::Volatility;
use crate::{DetHashMap, DetHashSet, DetHasher};

/// Inline `VALUES`: one solution per binding row, each cell an interned ground term
/// (or unbound for `UNDEF`).
pub(crate) fn eval_values<D: DatasetView + Sync>(
    variables: &[Variable],
    bindings: &[Vec<Option<purrdf_sparql_algebra::GroundTerm>>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<SolutionSeq<D::Id>, EvalError> {
    let schema = Arc::new(VarSchema::from_vars(variables.iter().cloned()));
    let width = schema.len();
    // `VALUES` is 1:1 with its inline bindings, in order, so the pushed row ceiling is
    // simply how many of them are worth interning. Interning is not free — every ground
    // term goes through the scratch arena's value hash — so a `VALUES` block of ten
    // thousand rows under a `LIMIT 5` stops after five.
    let semantic_ceiling = ctx.row_ceiling();
    let cell_ceiling = ctx.cell_row_ceiling(width);
    let capacity = semantic_ceiling
        .into_iter()
        .chain(cell_ceiling)
        .min()
        .unwrap_or(bindings.len())
        .min(bindings.len());
    let mut rows = Vec::with_capacity(capacity);
    for binding in bindings {
        if semantic_ceiling.is_some_and(|cap| rows.len() >= cap) {
            break;
        }
        if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
            let _ = ctx.observe_cells(rows.len().saturating_add(1), width);
            break;
        }
        let mut row = smallvec::smallvec![None; width];
        for (i, cell) in binding.iter().enumerate() {
            if let Some(ground) = cell {
                row[i] = Some(
                    ctx.scratch
                        .intern(ctx.dataset, ground_term_to_value(ground)),
                );
            }
        }
        rows.push(row);
    }
    Ok(SolutionSeq { schema, rows })
}

/// `SELECT`-list projection: restrict to `variables` in order. A projected variable
/// absent from the inner solution yields an all-unbound column.
pub(crate) fn eval_project<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    variables: &[Variable],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        let schema = Arc::new(VarSchema::from_vars(variables.iter().cloned()));
        return Ok(lift.finish(SolutionSeq::empty(schema)));
    };
    let out = Arc::new(VarSchema::from_vars(variables.iter().cloned()));
    // For each projected column, the source column in the inner schema (if any).
    let src: Vec<Option<usize>> = out.vars().iter().map(|v| seq.schema.index_of(v)).collect();
    let rows = seq
        .rows
        .iter()
        .map(|row| src.iter().map(|s| s.and_then(|c| row[c])).collect())
        .collect();
    Ok(lift.finish(SolutionSeq { schema: out, rows }))
}

/// `DISTINCT`: drop duplicate whole-solution rows, preserving first-seen order.
pub(crate) fn eval_distinct<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    dedup_lifted(node, inner, ctx)
}

/// `REDUCED`: permitted to drop duplicates; we apply the same dedup as `DISTINCT`
/// (a stronger-but-permitted reduction than the spec's minimum).
pub(crate) fn eval_reduced<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    dedup_lifted(node, inner, ctx)
}

/// The shared `DISTINCT`/`REDUCED` body.
///
/// De-duplication decides each row from the rows already seen, so it depends only on the
/// prefix and commits **per input row**: a prefix of the input dedups to a prefix of the
/// output, which is why a truncation below either operator keeps its bound instead of
/// voiding every `SELECT DISTINCT` in the corpus.
fn dedup_lifted<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        return Ok(lift.withheld());
    };
    Ok(lift.finish(dedup(seq)))
}

/// Drop duplicate rows, preserving first-seen order (SolutionTerm equality is exact
/// RDF-term identity — see the scratch-interner promotion rule).
fn dedup<I: ViewTermId>(seq: SolutionSeq<I>) -> SolutionSeq<I> {
    let mut unique = DetHashMap::with_capacity_and_hasher(seq.rows.len(), DetHasher::default());
    for (ordinal, row) in seq.rows.into_iter().enumerate() {
        if let Entry::Vacant(entry) = unique.entry(row) {
            entry.insert(ordinal);
        }
    }
    let mut rows: Vec<_> = unique
        .into_iter()
        .map(|(row, ordinal)| (ordinal, row))
        .collect();
    rows.sort_unstable_by_key(|(ordinal, _)| *ordinal);
    SolutionSeq {
        schema: seq.schema,
        rows: rows.into_iter().map(|(_, row)| row).collect(),
    }
}

/// `LIMIT`/`OFFSET`: skip `start` solutions then keep at most `length`.
///
/// # Under a truncated child
///
/// A restricting slice selects **by position**, so it needs a genuine prefix: given only
/// a sub-bag it can select rows the true query never returns. The lift enforces that —
/// a truncation that reaches this node having lost positional fidelity anywhere below it
/// yields no rows at all — so the ordinary slice below only ever runs over a prefix.
pub(crate) fn eval_slice<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    start: usize,
    length: Option<usize>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        return Ok(lift.withheld());
    };
    let rows = seq
        .rows
        .into_iter()
        .skip(start)
        .take(length.unwrap_or(usize::MAX))
        .collect();
    Ok(lift.finish(SolutionSeq {
        schema: seq.schema,
        rows,
    }))
}

/// `ORDER BY`: stable-sort by the sort keys under SPARQL ordering (§15.1).
pub(crate) fn eval_order_by<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    exprs: &[OrderExpression],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        return Ok(lift.withheld());
    };
    let schema = seq.schema.clone();

    // Evaluate every row's sort values first, then [`project`] each ONCE: the XSD
    // parse the ordering needs is paid per row instead of re-run inside the
    // O(n log n) comparator. Row `i`'s keys are `keys[i * width..][..width]`, and
    // sorting the PERMUTATION leaves them where their borrows point.
    let width = exprs.len();
    let mut values: Vec<Option<TermValue>> = Vec::with_capacity(seq.rows.len() * width);
    for row in &seq.rows {
        for oe in exprs {
            let (OrderExpression::Asc(e) | OrderExpression::Desc(e)) = oe;
            let term = eval_expr(e, row, &schema, ctx)?;
            values.push(term.map(|t| ctx.scratch.value_of(ctx.dataset, t)));
        }
    }
    let keys: Vec<SortKey<'_>> = values.iter().map(|v| project(v.as_ref())).collect();
    let mut order: Vec<usize> = (0..seq.rows.len()).collect();
    order.sort_by(|a, b| compare_keys(&keys[a * width..], &keys[b * width..], exprs));
    let mut source = seq.rows;
    let rows = order
        .into_iter()
        .map(|i| std::mem::take(&mut source[i]))
        .collect();
    // An `EXISTS` inside a sort key is an opaque edge; see `eval_left_join`.
    if let Some(tripped) = ctx.expression_barrier.observed() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            node,
            tripped,
            schema.clone(),
        )));
    }
    Ok(lift.finish(SolutionSeq { schema, rows }))
}

/// The expression a sort key sorts by, with its `ASC`/`DESC` direction set
/// aside — the read half of the `(direction, expression)` pair
/// [`OrderExpression`] is.
///
/// Every walk that treats a sort key as an ordinary expression (variable
/// collection, substitution, prepare-time planning) wants exactly this and
/// nothing else, and re-spelling the irrefutable
/// `let (Asc(e) | Desc(e)) = oe;` binding at each of them is how one of them
/// eventually diverges from the rest.
pub(crate) const fn order_sort_key(order: &OrderExpression) -> &Expression {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => expr,
    }
}

/// [`order_sort_key`]'s write half: put a rewritten expression back under
/// `order`'s ORIGINAL direction. Pairing the two is what keeps a rewrite from
/// silently turning a `DESC` key into an `ASC` one.
pub(crate) fn rebuild_order(order: &OrderExpression, expr: Expression) -> OrderExpression {
    match order {
        OrderExpression::Asc(_) => OrderExpression::Asc(expr),
        OrderExpression::Desc(_) => OrderExpression::Desc(expr),
    }
}

/// `GRAPH name { ... }`: scope the inner pattern to a named graph (or, for a
/// variable, every named graph in turn, binding the variable to each).
pub(crate) fn eval_graph<D: DatasetView + Sync>(
    node: &GraphPattern,
    name: &NamedNodePattern,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    match name {
        NamedNodePattern::NamedNode(n) => {
            let mut lift = Lift::at(node);
            match ctx.dataset.term_id_by_value(&named_node_to_value(n)) {
                // Addressable only if the active dataset's named set admits it (a
                // `FROM NAMED` / `USING NAMED` may restrict which graphs `GRAPH` sees).
                Some(id) if ctx.active_dataset.named_allows(id) => {
                    let saved = ctx.active_graph;
                    ctx.active_graph = GraphMatch::Named(id);
                    let result = eval_evaluated(inner, ctx);
                    ctx.active_graph = saved;
                    let Some(seq) = lift.absorb(0, result?) else {
                        return Ok(lift.withheld());
                    };
                    Ok(lift.finish(seq))
                }
                // The IRI is not a term (no quads), or not in the named dataset → empty.
                _ => Ok(lift.finish(SolutionSeq::empty(crate::eval::syntactic_schema(inner)))),
            }
        }
        NamedNodePattern::Variable(v) => eval_graph_var(node, v, inner, ctx),
    }
}

/// `GRAPH ?g { ... }`: evaluate the inner pattern once per named graph, binding `?g`
/// to the graph IRI, and union the results.
///
/// Per SPARQL 1.1 §8.3/§18.6, `?g` ranges over **every named graph in the active
/// dataset**, including one that owns zero quads (RDF 1.1 §3 allows an empty named
/// graph — see [`purrdf_core::RdfDataset::named_graphs`]), NOT just graphs a `quads()`
/// scan happens to find. And if `var` is ALREADY bound when this node is entered
/// (e.g. an outer `VALUES (?g ?t) { ... }` nested inside the `GRAPH ?g { }` block,
/// or any other pre-binding), each candidate graph must be JOINED against that
/// existing binding — kept only when compatible — rather than blindly overwritten.
///
/// # Under a truncated child
///
/// The inner pattern is evaluated once per named graph, so a trip inside it is a trip in
/// flight: the graph being scanned contributes nothing (its block is incomplete) while
/// every graph already scanned keeps its complete block — commit granularity at the
/// per-graph boundary, which is this operator's input row.
fn eval_graph_var<D: DatasetView + Sync>(
    node: &GraphPattern,
    var: &Variable,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    // Enumerate every named graph the dataset knows about, restricted to those the
    // active dataset admits (a `FROM NAMED` / `USING NAMED` may limit which graphs
    // `GRAPH ?g` binds to).
    let graphs: Vec<D::Id> = ctx
        .dataset
        .named_graphs()
        .filter(|g| ctx.active_dataset.named_allows(*g))
        .collect();

    let saved = ctx.active_graph;
    let mut out_schema: Option<Arc<VarSchema>> = None;
    let mut rows = Vec::new();
    let mut truncated = false;
    for g in graphs {
        ctx.active_graph = GraphMatch::Named(g);
        let inner_seq = match eval_evaluated(inner, ctx) {
            Ok(Evaluated::Complete(seq)) => seq,
            Ok(truncation @ Evaluated::Truncated(_)) => {
                truncated = true;
                if lift.absorb(0, truncation).is_none() {
                    ctx.active_graph = saved;
                    return Ok(lift.withheld());
                }
                break;
            }
            Err(e) => {
                ctx.active_graph = saved;
                return Err(e);
            }
        };
        let mut sch = (*inner_seq.schema).clone();
        let candidate = SolutionTerm::Existing(g);
        match sch.index_of(var) {
            // `var` is already a column of the inner pattern's own schema (e.g. it
            // came from a `VALUES` clause nested directly inside this `GRAPH ?g`
            // block): JOIN this candidate graph against each row's existing
            // binding instead of overwriting it — unbound rows adopt `g`,
            // rows bound to a DIFFERENT value are rejected, rows already bound to
            // `g` pass through unchanged.
            Some(gcol) => {
                for mut row in inner_seq.rows {
                    let compatible = !matches!(row[gcol], Some(existing) if existing != candidate);
                    if compatible {
                        row[gcol] = Some(candidate);
                        rows.push(row);
                    }
                }
            }
            // `var` is fresh to the inner pattern: append it as a new column.
            None => {
                let gcol = sch.push(var.clone());
                let width = sch.len();
                for mut row in inner_seq.rows {
                    row.resize(width, None);
                    row[gcol] = Some(candidate);
                    rows.push(row);
                }
            }
        }
        out_schema = Some(Arc::new(sch));
    }
    ctx.active_graph = saved;

    // No named graphs (or none matched): still produce the right schema with no rows.
    // A truncation already in hand means the inner pattern must NOT be evaluated again
    // (that would be a fresh scan after the budget is spent), so the schema is taken
    // from the partial result instead.
    let schema = match out_schema {
        Some(s) => s,
        None if truncated => lift.absorbed_schema().map_or_else(
            || {
                let mut schema = (*crate::eval::syntactic_schema(inner)).clone();
                schema.push(var.clone());
                Arc::new(schema)
            },
            |schema| {
                let mut schema = (*schema).clone();
                schema.push(var.clone());
                Arc::new(schema)
            },
        ),
        None => {
            let Some(seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
                return Ok(lift.withheld());
            };
            let mut sch = (*seq.schema).clone();
            sch.push(var.clone());
            Arc::new(sch)
        }
    };
    Ok(lift.finish(SolutionSeq { schema, rows }))
}

// ---------------------------------------------------------------------------
// ordering
// ---------------------------------------------------------------------------

/// Compare two rows' projected sort keys, applying each key's `ASC`/`DESC`.
pub(crate) fn compare_keys(
    a: &[SortKey<'_>],
    b: &[SortKey<'_>],
    exprs: &[OrderExpression],
) -> Ordering {
    for ((ka, kb), oe) in a.iter().zip(b).zip(exprs) {
        let mut ord = total_order(ka, kb);
        if matches!(oe, OrderExpression::Desc(_)) {
            ord = ord.reverse();
        }
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

/// A literal's value-space **comparability class** — the coarsest partition under
/// which [`value_total_cmp`] is total throughout a block or undefined throughout it.
///
/// The obvious reading of §15.1 — "by value, else by a deterministic syntactic
/// key" — is NOT a total order; it cycles. `"9"^^xsd:double` <
/// `"P1D"^^xsd:duration` (no value order, so `double` < `duration`) <
/// `"8"^^xsd:float` (again none) < `"9"^^xsd:double` (a value order: 8 < 9), and
/// Rust's sorts may panic on a comparator like that. Ranking the class BEFORE
/// the value breaks every such cycle, because the syntactic fallback then runs
/// only between two literals of ONE class.
///
/// `Opaque` (an unsupported datatype, `rdf:langString` included, or a lexical
/// form its own datatype rejects), `NotANumber`, and `Duration` (the general
/// `xsd:duration` is only partially ordered on `(months, seconds)`; XPath F&O
/// gives `lt`/`gt` to the two SUBTYPES and to nothing else) carry no value order
/// at all. The rest are total inside themselves, which is why the partition is
/// finer than the XSD value spaces: `Temporal` is one block per temporal datatype
/// IRI, split again on whether the lexical carries a timezone (a timezoned and an
/// untimezoned instant are indeterminate within fourteen hours), and `Binary`
/// separates the two byte-sequence value spaces.
///
/// # Why `Numeric` is one block, and why it needs an EXACT comparison to be one
///
/// Splitting the class was never enough for the numbers, because the cycle there
/// is INSIDE one class. SPARQL §17.3 maps a cross-type numeric comparison onto the
/// promotion lattice `integer ⊂ decimal ⊂ float ⊂ double`, which compares an
/// integer against a decimal exactly but routes anything touching a float or a
/// double through IEEE. Mixing an exact sub-relation with a lossy one is not
/// transitive: `"1.000000000000000001"^^xsd:decimal` is exactly greater than
/// `"1"^^xsd:integer`, yet promotion rounds it to `1.0` and calls it equal to
/// `"1.0E0"^^xsd:double`, which the integer also equals. Three ordinary literals,
/// one cycle, and a query supplies them.
///
/// The fix is not another class — a class rank would have to separate values that
/// genuinely compare — but an exact comparison, which is what
/// [`value_total_cmp`] gives this block: every member of the numeric tower except
/// `NaN` (split out as [`ValueClass::NotANumber`]) is exactly a rational, the
/// infinities included as its two ends, and comparing those rationals exactly is
/// transitive by construction. That deliberately DIVERGES from the promotion-based
/// `<` this crate's `FILTER` still evaluates, in exactly the pairs where the
/// promoted relation is intransitive and therefore had no admissible sort order to
/// preserve; see `purrdf_xsd::numeric_total_cmp` for the full argument.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ValueClass {
    Opaque,
    Boolean,
    Numeric,
    NotANumber,
    Text,
    Temporal(&'static str, bool),
    YearMonthDuration,
    DayTimeDuration,
    Duration,
    Binary(&'static str),
}

impl ValueClass {
    /// Classify a literal from its parsed value (`None` when its datatype models
    /// no value space, or its lexical does not parse), answering the class with
    /// the value it orders BY — `None` for the three value-less blocks, which
    /// therefore never reach [`value_total_cmp`].
    fn of(parsed: Option<XsdValue>, lexical: &str) -> (Self, Option<XsdValue>) {
        let Some(value) = parsed else {
            return (Self::Opaque, None);
        };
        // A timezone is a trailing `Z` or the six-byte `(+|-)hh:mm` offset, whose
        // `':'` three from the end tells it from an untimezoned `2000-01-01`.
        let b = lexical.as_bytes();
        let zoned = b.last() == Some(&b'Z')
            || (b.len() >= 6 && matches!(b[b.len() - 6], b'+' | b'-') && b[b.len() - 3] == b':');
        let class = match &value {
            XsdValue::Boolean(_) => Self::Boolean,
            XsdValue::Float(f) if f.is_nan() => Self::NotANumber,
            XsdValue::Double(d) if d.is_nan() => Self::NotANumber,
            XsdValue::Integer { .. }
            | XsdValue::Decimal(_)
            | XsdValue::Float(_)
            | XsdValue::Double(_) => Self::Numeric,
            XsdValue::String(_) => Self::Text,
            XsdValue::DateTime(_)
            | XsdValue::Date(_)
            | XsdValue::Time(_)
            | XsdValue::Gregorian(_) => Self::Temporal(value.datatype().iri(), zoned),
            XsdValue::Duration(_) => match value.datatype() {
                XsdDatatype::YearMonthDuration => Self::YearMonthDuration,
                XsdDatatype::DayTimeDuration => Self::DayTimeDuration,
                _ => Self::Duration,
            },
            XsdValue::Binary { datatype, .. } => Self::Binary(datatype.iri()),
            // `XsdValue` is `#[non_exhaustive]`: an undecided value space is
            // opaque, never guessed at.
            _ => Self::Opaque,
        };
        let ordered = !matches!(class, Self::Opaque | Self::NotANumber | Self::Duration);
        (class, ordered.then_some(value))
    }

    /// The class of a value held OUTSIDE a term, where there is no lexical form
    /// to read a timezone off.
    ///
    /// `MEDIAN`/`PERCENTILE` hold a `Vec<XsdValue>`, not terms, and their sort
    /// needs the SAME block ranking `ORDER BY` uses or it inherits the very
    /// cycles this partition exists to break: within one numeric series a `NaN`
    /// compares to nothing, and within one duration series a `yearMonthDuration`
    /// compares to no `dayTimeDuration` — either way an incomparable pair read as
    /// "equal" sits between two values that are not equal, and Rust's sorts may
    /// panic on that. Ranking the block first breaks both.
    ///
    /// The temporal blocks split on whether the lexical carries a timezone, which
    /// is unknowable from a bare value, so a temporal value answers [`Self::Opaque`]
    /// here. That is the fail-CLOSED answer, not a wrong one: `Opaque` is a
    /// value-less block whose members tie, so it can introduce no cycle. It is
    /// also unreachable today — both callers gate their input through
    /// `is_numeric_or_duration_xsd` — and exists so a future caller gets a tie
    /// rather than a silently-merged pair of indeterminate instants.
    pub(crate) fn of_value(value: &XsdValue) -> Self {
        if matches!(
            value,
            XsdValue::DateTime(_) | XsdValue::Date(_) | XsdValue::Time(_) | XsdValue::Gregorian(_)
        ) {
            return Self::Opaque;
        }
        Self::of(Some(value.clone()), "").0
    }
}

/// The literal arm of [`SortKey`]: its comparability class, the value that class
/// orders by, and the `(datatype, language, lexical)` tiebreak the value-less
/// classes and the value-space ties fall back on (a base direction plays no part,
/// matching §15.1's silence about it).
pub(crate) struct LiteralKey<'a> {
    class: ValueClass,
    value: Option<XsdValue>,
    datatype: &'a str,
    language: Option<&'a str>,
    lexical: &'a str,
}

/// One term's position in the SPARQL ordering (§15.1) — the crate's ONE
/// projection of that relation, paired with the one [`total_order`] over it and
/// shared by `ORDER BY`, `MIN`/`MAX` and the statistical aggregates. A key borrows
/// from the [`TermValue`] it came from, so [`project`] allocates nothing but the
/// box a nested triple term needs and the parsed contents a composite literal
/// carries, and the per-literal XSD parse is paid once per term instead of on
/// every comparison inside an `O(n log n)` sort.
///
/// # Adding a term category
///
/// A new recursive category is one variant here plus one arm each in [`project`],
/// [`SortKey::rank`] and [`total_order`]; its members recurse back through
/// `total_order`, exactly as [`SortKey::Triple`] does, so the category inherits the
/// whole relation instead of restating any of it. [`SortKey::Composite`] is the
/// worked example.
pub(crate) enum SortKey<'a> {
    /// Unbound — sorts before every bound term.
    Unbound,
    /// Blank node, by `(scope ordinal, label)`.
    Blank(u32, &'a str),
    /// IRI, by its string.
    Iri(&'a str),
    /// Literal — see [`LiteralKey`].
    Literal(LiteralKey<'a>),
    /// A SEP-0009 composite literal (`cdt:List` / `cdt:Map`) whose lexical form
    /// parses, ordered by the value it denotes rather than by that lexical form.
    ///
    /// # Where composites sit relative to the other kinds, and why that is a choice
    ///
    /// §15.1 does not mention composite datatypes, and neither does SEP-0009's own
    /// `ORDER BY` corpus: every one of its cases sorts a column of composites
    /// against other composites, so nothing there pins where a `cdt:List` sits
    /// relative to an unbound, a blank node, an IRI or a plain literal. This crate
    /// pins it HERE, by declaration order: after [`SortKey::Literal`] and before
    /// [`SortKey::Triple`]. A composite literal IS an RDF literal, so it belongs on
    /// the literal side of §15.1's kind order rather than below the IRIs; and it is
    /// a container of terms, like a triple term, so it belongs beside the other
    /// container rather than interleaved with the scalars. A composite whose
    /// lexical form does NOT parse never reaches this variant at all — it is an
    /// ordinary [`ValueClass::Opaque`] literal, sorted by its lexical form, one
    /// rank below every composite that does parse.
    Composite(purrdf_cdt::CdtValue),
    /// RDF 1.2 triple term, componentwise over `(s, p, o)`.
    Triple(Box<[Self; 3]>),
}

impl SortKey<'_> {
    /// §15.1's kind order, extended with the composite and triple terms it does not
    /// mention. The ranks are declaration order and nothing else reads them.
    const fn rank(&self) -> u8 {
        match self {
            Self::Unbound => 0,
            Self::Blank(..) => 1,
            Self::Iri(_) => 2,
            Self::Literal(_) => 3,
            Self::Composite(_) => 4,
            Self::Triple(_) => 5,
        }
    }
}

/// Project one (possibly unbound) term onto its sort key.
pub(crate) fn project(value: Option<&TermValue>) -> SortKey<'_> {
    match value {
        None => SortKey::Unbound,
        Some(TermValue::Blank { label, scope }) => SortKey::Blank(scope.ordinal(), label),
        Some(TermValue::Iri(iri)) => SortKey::Iri(iri),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        }) => {
            // The composite datatypes are decided by their datatype IRI FIRST, so a
            // `cdt:List` never pays the XSD parse and an `xsd:string` never pays the
            // composite scan. A composite whose lexical form does not parse falls
            // through to the ordinary literal path and lands in
            // `ValueClass::Opaque`, which is the honest place for a literal that
            // denotes nothing — `ORDER BY` must still be total over it.
            if matches!(
                datatype.as_str(),
                purrdf_cdt::CDT_LIST | purrdf_cdt::CDT_MAP
            ) && let Ok(Some(composite)) = purrdf_cdt::parse_cdt_by_iri(lexical_form, datatype)
            {
                return SortKey::Composite(composite);
            }
            let parsed = parse_by_iri(lexical_form, datatype).ok().flatten();
            let (class, value) = ValueClass::of(parsed, lexical_form);
            SortKey::Literal(LiteralKey {
                class,
                value,
                datatype,
                language: language.as_deref(),
                lexical: lexical_form,
            })
        }
        Some(TermValue::Triple { s, p, o }) => {
            SortKey::Triple(Box::new([s, p, o].map(|t| project(Some(&**t)))))
        }
    }
}

/// The SPARQL ordering relation over two projections, and a genuine total order:
/// by term kind, then — within the literals — by comparability class, value space,
/// and the syntactic tiebreak. A value comparison and a syntactic one never mix
/// inside a class, which is what keeps it transitive (see [`ValueClass`]).
/// `Unbound` needs no arm: it ranks below every bound key, two of them as equals.
///
/// # Composites use the CDT crate's SYNTACTIC order, deliberately
///
/// [`purrdf_cdt::total_value_cmp`] is that crate's exported total order, and it is
/// the one a sort may use. SEP-0009's own value relations
/// (`purrdf_cdt::value_less_than` and friends) are partial and RAISE on a pair they
/// cannot decide, which `ORDER BY` cannot do — a sort must be total and must never
/// error — and the obvious repair, "value order with a structural tie-break", is
/// itself intransitive for exactly the reason [`ValueClass`] documents one level
/// up (`crates/cdt/tests/value_relations.rs` exhibits the cycle). The syntactic
/// order is a lexicographic product of total orders, so it is transitive by
/// construction, and it is what the CDT crate already sorts map entries and renders
/// canonical lexical forms with.
pub(crate) fn total_order(a: &SortKey<'_>, b: &SortKey<'_>) -> Ordering {
    match (a, b) {
        (SortKey::Blank(sa, la), SortKey::Blank(sb, lb)) => (sa, la).cmp(&(sb, lb)),
        (SortKey::Iri(x), SortKey::Iri(y)) => x.cmp(y),
        (SortKey::Literal(x), SortKey::Literal(y)) => {
            if x.class != y.class {
                return x.class.cmp(&y.class);
            }
            if let (Some(av), Some(bv)) = (&x.value, &y.value)
                && let Some(ord) = value_total_cmp(av, bv)
            {
                return ord;
            }
            (x.datatype, x.language, x.lexical).cmp(&(y.datatype, y.language, y.lexical))
        }
        (SortKey::Composite(x), SortKey::Composite(y)) => purrdf_cdt::total_value_cmp(x, y),
        (SortKey::Triple(x), SortKey::Triple(y)) => total_order(&x[0], &y[0])
            .then_with(|| total_order(&x[1], &y[1]))
            .then_with(|| total_order(&x[2], &y[2])),
        _ => a.rank().cmp(&b.rank()),
    }
}

// ---------------------------------------------------------------------------
// value-level entry points
// ---------------------------------------------------------------------------

/// Order `values` by SPARQL `ORDER BY` semantics — the evaluator's own ordering,
/// applied to a bag of already-materialized [`TermValue`]s rather than to
/// solution rows.
///
/// `ORDER BY` over a one-column input is a total order over TERM VALUES, and this
/// exposes exactly that: the same `SortKey` precomputation, the same
/// unbound < blank < IRI < literal < composite < triple kind ranking, the same
/// value-space literal comparison with its deterministic `(datatype, language,
/// lexical)` fallback, and the same componentwise ordering of RDF 1.2 triple
/// terms. A caller with a bag of values in hand therefore does not have to
/// reach the comparator by *writing a query* — splicing operands into `VALUES`
/// as text would restrict them to the terms N-Triples can spell inside that
/// block, which is a property of the string bridge and not of the order.
///
/// The sort is STABLE, so values that compare equal keep their input order and
/// duplicates are preserved (an `ORDER BY` applies no `DISTINCT`).
#[must_use]
pub fn order_values(values: Vec<TermValue>, descending: bool) -> Vec<TermValue> {
    // The keys BORROW their value, so the projection is taken per comparison
    // over an index permutation rather than stored beside the value. A
    // `SortKey<'_>` cannot outlive the `TermValue` it reads, and moving the
    // values into a keyed pair would invalidate every borrow.
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&i, &j| {
        let ord = total_order(&project(Some(&values[i])), &project(Some(&values[j])));
        if descending { ord.reverse() } else { ord }
    });
    let mut slots: Vec<Option<TermValue>> = values.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|i| {
            slots[i]
                .take()
                .expect("each index appears once in a sorted permutation")
        })
        .collect()
}

/// Compare two term values under SPARQL `ORDER BY` semantics.
///
/// The single-pair form of [`order_values`], for a caller that needs the
/// comparator itself (a `MIN`/`MAX` tie-break, a merge) rather than a sorted bag.
#[must_use]
pub fn compare_values(a: &TermValue, b: &TermValue) -> Ordering {
    total_order(&project(Some(a)), &project(Some(b)))
}

/// A SPARQL built-in aggregate that folds a bag of already-evaluated values and
/// takes no further arguments.
///
/// The set is deliberately not all of [`AggregateFunction`]: `GROUP_CONCAT`
/// carries a separator and `AggregateFunction::Custom` is a registry lookup, so
/// neither is determined by a bag of values alone. Every member here is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueAggregate {
    /// `COUNT` — the number of values folded.
    Count,
    /// `SUM` — the numerically-promoted total; `0`^^`xsd:integer` over an empty bag.
    Sum,
    /// `AVG` — the mean; `0`^^`xsd:integer` over an empty bag.
    Avg,
    /// `MIN` — the `ORDER BY`-least value; unbound over an empty bag.
    Min,
    /// `MAX` — the `ORDER BY`-greatest value; unbound over an empty bag.
    Max,
    /// `SAMPLE` — the first value in input order; unbound over an empty bag.
    Sample,
}

/// Fold `values` through a SPARQL built-in aggregate — the evaluator's own
/// accumulator, applied to a bag of already-materialized [`TermValue`]s rather
/// than to a solution group.
///
/// The value-level twin of [`order_values`], and it exists for the same reason:
/// the aggregate's answer is a property of the VALUES, so a caller holding them
/// should not have to reach the fold by writing a query whose text embeds the
/// data. It runs the identical accumulators `eval_aggregate` runs — the exact
/// `NumericFold` promotion ladder for `SUM`/`AVG`, the same total order's running
/// extreme with its earlier-occurrence tie-break for `MIN`/`MAX` — so a
/// value-level fold and a `GROUP BY` fold over the same bag are the same number,
/// by construction rather than by two implementations agreeing.
///
/// `Ok(None)` is the aggregate's UNBOUND answer (`MIN`/`MAX`/`SAMPLE` of an
/// empty bag; a `SUM`/`AVG` fold poisoned by a non-numeric value).
///
/// # Errors
///
/// Any [`EvalError`] the accumulator raises while folding.
pub fn fold_values(
    aggregate: ValueAggregate,
    values: &[TermValue],
) -> Result<Option<TermValue>, EvalError> {
    match aggregate {
        ValueAggregate::Count => fold_builtin(values, CountAccumulator::default, acc_step_one),
        ValueAggregate::Sum => fold_builtin(values, SumAccumulator::default, acc_step_one),
        ValueAggregate::Avg => fold_builtin(values, AvgAccumulator::default, acc_step_one),
        ValueAggregate::Min => fold_builtin(values, MinAccumulator::default, acc_step_one),
        ValueAggregate::Max => fold_builtin(values, MaxAccumulator::default, acc_step_one),
        ValueAggregate::Sample => fold_builtin(values, SampleAccumulator::default, acc_step_one),
    }
}

// ---------------------------------------------------------------------------
// GROUP BY + aggregates
// ---------------------------------------------------------------------------

/// `GROUP BY ... ` with aggregates: partition the inner solutions by the grouping
/// key (term identity), then compute each aggregate per group. One output row per
/// group; the columns are the grouping variables followed by the aggregate outputs.
///
/// With **no** grouping variables but aggregates present, the whole input is a
/// single group — even when empty (so `COUNT(*)` yields one row binding `0`).
///
/// # Under a truncated child
///
/// Nothing crosses. An aggregate over a partial input is a **different number**, not a
/// subset of the true one — `COUNT` under-counts, `SUM` under-sums — so the edge to the
/// grouped input is opaque and the lift withholds every row, carrying the barrier in
/// their place. Computing the aggregates anyway and discarding them would be the same
/// answer at higher cost, so the operator returns before grouping.
pub(crate) fn eval_group<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    variables: &[Variable],
    aggregates: &[(Variable, AggregateExpression)],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        // No rows cross an opaque edge, but the COLUMNS still do: a `GROUP BY`'s output
        // schema is syntactic — the grouping variables followed by the aggregate output
        // variables — so it costs nothing to report the columns this node would have
        // produced rather than the columns of the input it withheld.
        let mut out_schema = VarSchema::from_vars(variables.iter().cloned());
        for (out_var, _) in aggregates {
            out_schema.push(out_var.clone());
        }
        return Ok(lift.finish(SolutionSeq::empty(Arc::new(out_schema))));
    };
    let in_schema = seq.schema.clone();
    let key_cols: Vec<Option<usize>> = variables.iter().map(|v| in_schema.index_of(v)).collect();

    // Partition rows into groups, keeping groups in first-seen order.
    let mut groups: DetHashMap<Solution<D::Id>, (usize, Vec<usize>)> = DetHashMap::default();
    for (idx, row) in seq.rows.iter().enumerate() {
        let key: Solution<D::Id> = key_cols.iter().map(|c| c.and_then(|c| row[c])).collect();
        let next_ordinal = groups.len();
        match groups.entry(key) {
            Entry::Occupied(mut entry) => entry.get_mut().1.push(idx),
            Entry::Vacant(entry) => {
                entry.insert((next_ordinal, vec![idx]));
            }
        }
    }
    // No GROUP BY + empty input + aggregates → a single empty group.
    if groups.is_empty() && variables.is_empty() && !aggregates.is_empty() {
        groups.insert(Solution::new(), (0, Vec::new()));
    }
    let mut groups: Vec<_> = groups
        .into_iter()
        .map(|(key, (ordinal, rows))| (ordinal, key, rows))
        .collect();
    groups.sort_unstable_by_key(|(ordinal, _, _)| *ordinal);

    let mut out_schema = VarSchema::from_vars(variables.iter().cloned());
    for (out_var, _) in aggregates {
        out_schema.push(out_var.clone());
    }
    let out_schema = Arc::new(out_schema);
    let out_width = out_schema.len();
    let var_count = variables.len();

    // Every aggregate expression must be parallel-safe (no RAND/UUID/STRUUID/
    // BNODE/list-mint reachable) for the per-group compute below to run under
    // the fork-join model, AND every `Custom` aggregate reached must resolve to a
    // registered, non-`Volatile` accumulator — `ctx.may_fork_aggregate` checks
    // both; `should_parallelize` (inside `par_chunk_try_map_init`) still gates on
    // group count. `COUNT(*)`'s empty `args` trivially passes (nothing to check),
    // exactly as the prior `CountStar { .. } => true` arm did.
    let safe = aggregates
        .iter()
        .all(|(_, agg)| ctx.may_fork_aggregate(agg));

    let rows = if safe {
        let base = ctx.scratch.computed_count();
        let minted = crate::parallel::par_chunk_try_map_init(
            &groups,
            || ctx.fork_for_worker(),
            |child, acc, (_, key, idxs)| {
                let mut row = smallvec::smallvec![None; out_width];
                for (i, _) in variables.iter().enumerate() {
                    row[i] = key[i];
                }
                for (j, (_, agg)) in aggregates.iter().enumerate() {
                    row[var_count + j] = eval_aggregate(agg, idxs, &seq.rows, &in_schema, child)?;
                }
                acc.push(crate::parallel::minted_row(&child.scratch, base, row));
                Ok(())
            },
        )?;
        minted
            .into_iter()
            .map(|row| crate::parallel::reintern_minted_row(&mut ctx.scratch, ctx.dataset, row))
            .collect()
    } else {
        let mut rows = Vec::with_capacity(groups.len());
        for (_, key, idxs) in &groups {
            let mut row = smallvec::smallvec![None; out_width];
            for (i, _) in variables.iter().enumerate() {
                row[i] = key[i];
            }
            for (j, (_, agg)) in aggregates.iter().enumerate() {
                row[var_count + j] = eval_aggregate(agg, idxs, &seq.rows, &in_schema, ctx)?;
            }
            rows.push(row);
        }
        rows
    };

    // An `EXISTS` inside an aggregate argument is an opaque edge; see `eval_left_join`.
    if let Some(tripped) = ctx.expression_barrier.observed() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            node, tripped, out_schema,
        )));
    }
    Ok(lift.finish(SolutionSeq {
        schema: out_schema,
        rows,
    }))
}

/// Compute one aggregate over a group's rows in two phases: phase 1 evaluates
/// every surviving row's argument(s) and materializes them into a per-group
/// buffer (`survivors`, already `DISTINCT`-resolved and in row order); phase 2
/// folds that buffer through an accumulator, sequentially or in parallel
/// chunks. This is NOT streaming — the whole group's resolved values are held
/// at once before folding starts — which is what lets phase 2 chunk the fold
/// without touching [`EvalCtx`]/the governor at all (every charge already
/// happened in phase 1) and is why phase 2 is safe to parallelize regardless
/// of volatility (see phase 2's own comment below). A
/// built-in accumulator (this module: [`CountAccumulator`], [`SumAccumulator`],
/// [`AvgAccumulator`], [`MinAccumulator`], [`MaxAccumulator`],
/// [`SampleAccumulator`], [`GroupConcatAccumulator`]) and a registered
/// [`crate::agg_fn::CustomAggregate`] both instantiate the exact SAME
/// [`crate::agg_fn::AggregateAccumulator`] trait — `init`/`step`/`combine`/
/// `finish` — the one fold algebra this crate has; only the DISPATCH differs
/// (static, generic-monomorphized for a built-in via [`fold_builtin`]; dynamic,
/// through a boxed trait object, for a host-registered one via
/// `eval_custom_aggregate`, which needs it because the concrete type is not
/// known until the registry resolves an IRI at run time). This function is the
/// ONE place that decides which instance a given [`AggregateExpression`] folds
/// through — and, because every kind is dispatched from here, the ONE place
/// that charges [`ChargePoint::AggregateInvocation`] for any of them.
///
/// # Error-row skipping
///
/// A row whose argument expression evaluates to unbound (`eval_expr` returns
/// `Ok(None)`) never reaches a fold's `step` at all — for the single argument a
/// built-in aggregate other than `COUNT(*)` takes, exactly as for every
/// positional argument a [`AggregateFunction::Custom`] call takes (a `Custom`
/// row needs EVERY position bound, mirroring
/// [`crate::user_fn::eval_native_function`]'s "no per-parameter optionality" for
/// a native scalar function's arguments). An expression that raises an
/// evaluation error still aborts the whole query via `?`, exactly as before this
/// restructuring — only an honestly UNBOUND value is skipped.
///
/// # Governance
///
/// [`ChargePoint::AggregateInvocation`] is charged exactly once here, before any
/// dispatch, so it prices the fold's init/finish overhead uniformly for
/// `COUNT(*)`, every other built-in, and every registered custom aggregate. A
/// refused charge is recorded on [`EvalCtx::expression_barrier`] and answered as
/// unbound — the same doctrine [`crate::user_fn::eval_native_function`]'s
/// invocation charge follows — leaving [`eval_group`] to notice the barrier and
/// withhold the whole grouped output.
fn eval_aggregate<D: DatasetView + Sync>(
    agg: &AggregateExpression,
    idxs: &[usize],
    rows: &[Solution<D::Id>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    if let Err(tripped) = ctx.charge(ChargePoint::AggregateInvocation) {
        ctx.expression_barrier.record(tripped);
        return Ok(None);
    }

    if let AggregateFunction::Custom(iri) = agg.function() {
        return eval_custom_aggregate(iri.as_str(), agg, idxs, rows, schema, ctx);
    }

    // `FOLD` is dispatched away here for the same reason `Custom` is: its PHASE 1
    // is not the one below. Every other built-in skips a row whose argument is
    // unbound; `FOLD` RETAINS it as the SEP-0009 `null` element, takes a second
    // argument in its `cdt:Map` form, and orders its survivors by its own
    // `ORDER BY` before folding any of them. Its phase 2 is nonetheless the
    // ordinary [`fold_builtin`] tail over an ordinary
    // [`crate::agg_fn::AggregateAccumulator`] — one fold algebra, as the rest of
    // this module's dispatch promises. See [`crate::cdt_agg`].
    if matches!(agg.function(), AggregateFunction::Fold) {
        return crate::cdt_agg::eval_fold(agg, idxs, rows, schema, ctx);
    }

    // `COUNT(*)`/`COUNT(DISTINCT *)` is the spec's empty exprlist, and
    // [`AggregateExpression::new`] enforces that `COUNT` is the ONLY function
    // that can ever carry one — so dispatch reads `agg.function()` itself
    // rather than asking whether `args` is empty. A zero-arity custom
    // aggregate cannot reach this arm by accident and fall through to a row
    // count: it cannot be constructed at all, and even if some future
    // registry made one legal, the `Custom` short-circuit above already
    // claimed it.
    //
    // The row loop below (charge, then row-identity `DISTINCT`) is UNCHANGED
    // from before this restructuring — it decides how many rows survive, not
    // how the survivors fold. What changes is the TAIL: rather than
    // hand-incrementing an `i64` outside any accumulator (the third,
    // bolted-on dispatch path this restructuring removes), `COUNT(*)` now
    // folds through the exact same [`CountAccumulator`] `COUNT(?x)` uses,
    // via [`fold_builtin`] — `CountAccumulator::step` ignores its argument
    // entirely, so a run of `survivors` zero-sized units is a faithful
    // (and allocation-free — `Vec<()>` never touches the heap) stand-in for
    // "this many rows survived to be folded."
    if matches!(agg.function(), AggregateFunction::Count) && agg.args().is_empty() {
        // Every row is a value `COUNT(*)` folds, whether or not `DISTINCT` keeps
        // it — see [`ChargePoint::AggregateAccumulation`]'s doc for why the
        // charge precedes the dedup check. An explicit loop, rather than
        // `Iterator::count`, so a refused charge stops the count exactly where
        // the budget ran out instead of after the whole group was scanned.
        let mut seen: Option<DetHashSet<&Solution<D::Id>>> = agg.distinct.then(DetHashSet::default);
        let mut survivors: usize = 0;
        for &i in idxs {
            if let Err(tripped) = ctx.charge(ChargePoint::AggregateAccumulation) {
                ctx.expression_barrier.record(tripped);
                return Ok(None);
            }
            if let Some(seen) = seen.as_mut()
                && !seen.insert(&rows[i])
            {
                continue;
            }
            survivors += 1;
        }
        let value = fold_builtin(
            &vec![(); survivors],
            CountAccumulator::default,
            |acc, ()| acc.step(&[]),
        )?;
        return Ok(value.map(|v| ctx.scratch.intern(ctx.dataset, v)));
    }

    // Every built-in aggregate reaching here is `COUNT(?x)`/`SUM`/`AVG`/`MIN`/
    // `MAX`/`SAMPLE`/`GROUP_CONCAT`, every one of them unary — only this one
    // argument expression is ever evaluated. The `COUNT`+empty-`args` case
    // returned above already, and `AggregateExpression::new` forbids empty
    // `args` for anything else, so `agg.args()` is non-empty for every value
    // that reaches here — an invariant enforced by `purrdf_sparql_algebra`'s
    // constructor, not by a type this crate owns, so rather than panic on a
    // state that should be provably unreachable, an empty `args` here folds
    // zero survivors (the SAME answer a genuinely empty group already gives)
    // instead of trusting the invariant with an `unreachable!()`.
    let mut seen: Option<DetHashSet<SolutionTerm<D::Id>>> = agg.distinct.then(DetHashSet::default);
    let mut survivors: Vec<TermValue> = Vec::new();
    if let Some(first_arg) = agg.args().first() {
        // Phase 1: evaluate every row's argument expression against `ctx`, charge
        // `AggregateAccumulation` for each one, apply `DISTINCT`, and charge
        // `ScratchBytes` for each value actually retained into `survivors` (see
        // below) — otherwise unchanged from before within-group chunking existed,
        // so the REST of what this phase spends is a pure function of `idxs`/`ctx`,
        // identical whether or not phase 2 below ends up chunking. See
        // [`ChargePoint::AggregateAccumulation`]'s doc for why that charge
        // precedes the `DISTINCT` check.
        //
        // DISTINCT dedups by `SolutionTerm` equality, which the scratch interner's
        // Existing/Computed promotion rule (see `crate::scratch`'s module docs) makes
        // exactly equivalent to dedup-by-value: two distinct `SolutionTerm`s never
        // denote the same value. Cheaper than hashing the resolved `TermValue` (an
        // owned-string clone for a literal/IRI), and byte-identical to the prior
        // materializing implementation's `seen: DetHashSet<SolutionTerm>` retain.
        //
        // `ScratchBytes`: `ctx.scratch.value_of` below does not mint anything —
        // it reads an already-interned or already-dataset-resident value back out
        // as an OWNED clone (see [`crate::scratch::ScratchInterner::value_of`]),
        // so [`EvalCtx::charge_scratch_growth`]'s automatic per-node charge, which
        // meters only what [`crate::scratch::ScratchInterner::intern`] mints,
        // never sees this buffer. Retaining `O(survivors)` owned `TermValue`
        // clones for the rest of this group's fold is real, otherwise-uncharged
        // memory — proportional to group cardinality and unbounded by any other
        // dimension — so it is charged here explicitly, the same way a custom
        // aggregate's own accumulator state is (see `eval_custom_aggregate`'s
        // matching charge), through the SAME deterministic per-value proxy
        // [`crate::scratch::value_bytes`] the arena's own automatic charge uses.
        for &i in idxs {
            let Some(term) = eval_expr(first_arg, &rows[i], schema, ctx)? else {
                continue;
            };
            if let Err(tripped) = ctx.charge(ChargePoint::AggregateAccumulation) {
                ctx.expression_barrier.record(tripped);
                return Ok(None);
            }
            if let Some(seen) = seen.as_mut()
                && !seen.insert(term)
            {
                continue;
            }
            let value = ctx.scratch.value_of(ctx.dataset, term);
            if let Err(tripped) = ctx.charge_amount(
                purrdf_core::ResourceDimension::ScratchBytes,
                crate::scratch::value_bytes(&value),
            ) {
                ctx.expression_barrier.record(tripped);
                return Ok(None);
            }
            survivors.push(value);
        }
    }

    // Phase 2: fold the (already `DISTINCT`-resolved, already in row order)
    // survivor list through the built-in's [`crate::agg_fn::AggregateAccumulator`]
    // instance — chunked in parallel for a large enough group, strictly
    // sequential for a small one — see [`fold_builtin`]/
    // `crate::parallel::par_chunk_reduce_init`'s docs for why this phase needs
    // no `EvalCtx`/governor access at all (every charge already happened in
    // phase 1 above) and is therefore always safe to chunk regardless of any
    // volatility concern: nothing here can reach `RAND`/`BNODE`/an `EXISTS`
    // re-entry, because nothing here evaluates an expression.
    let value = match agg.function() {
        AggregateFunction::Count => {
            fold_builtin(&survivors, CountAccumulator::default, acc_step_one)?
        }
        AggregateFunction::Sum => fold_builtin(&survivors, SumAccumulator::default, acc_step_one)?,
        AggregateFunction::Avg => fold_builtin(&survivors, AvgAccumulator::default, acc_step_one)?,
        AggregateFunction::Min => fold_builtin(&survivors, MinAccumulator::default, acc_step_one)?,
        AggregateFunction::Max => fold_builtin(&survivors, MaxAccumulator::default, acc_step_one)?,
        AggregateFunction::Sample => {
            fold_builtin(&survivors, SampleAccumulator::default, acc_step_one)?
        }
        AggregateFunction::GroupConcat => {
            let sep = agg.separator().unwrap_or(" ").to_owned();
            fold_builtin(
                &survivors,
                || GroupConcatAccumulator::new(sep.clone()),
                acc_step_one,
            )?
        }
        // `Custom` was dispatched away at the top of this function, before any
        // charge or row was consulted — this arm cannot be reached by any
        // `AggregateExpression` this crate's own planner ever hands to
        // `eval_aggregate`. `AggregateFunction` is defined in
        // `purrdf_sparql_algebra`, a crate this one does not own, so its
        // `Custom` variant cannot be removed from this match at the type
        // level; a typed, non-panicking internal error is the honest stand-in
        // for that unencodable invariant (never observed by any test or
        // conformance case — see this function's own dispatch order).
        AggregateFunction::Custom(_) => {
            return Err(EvalError::internal(
                "AggregateFunction::Custom reached the built-in dispatch arm; the Custom \
                 short-circuit at the top of eval_aggregate should have already handled it",
            ));
        }
        // Unreachable for exactly the same reason `Custom` is: `FOLD` was
        // dispatched to `crate::cdt_agg::eval_fold` at the top of this function,
        // before any row was consulted, because its phase 1 retains unbound rows
        // that the loop above has already skipped. Reaching here would mean
        // folding a `FOLD` over a survivor list its own semantics never built, so
        // it is a typed internal error rather than a silently wrong answer.
        AggregateFunction::Fold => {
            return Err(EvalError::internal(
                "AggregateFunction::Fold reached the built-in dispatch arm; the FOLD \
                 short-circuit at the top of eval_aggregate should have already handled it",
            ));
        }
    };
    Ok(value.map(|v| ctx.scratch.intern(ctx.dataset, v)))
}

/// [`fold_builtin`]'s per-row step closure for every built-in whose argument
/// list is exactly one expression (every one but `COUNT(*)`, which folds a
/// unit per survivor instead — see `eval_aggregate`): wrap the single already-
/// evaluated `value` as the one-element `&[TermValue]` slice
/// [`crate::agg_fn::AggregateAccumulator::step`] expects, with no allocation
/// (`std::slice::from_ref`).
fn acc_step_one<A: crate::agg_fn::AggregateAccumulator>(
    acc: &mut A,
    value: &TermValue,
) -> Result<(), EvalError> {
    acc.step(std::slice::from_ref(value))
}

/// Fold `survivors` through one instance of a built-in
/// [`crate::agg_fn::AggregateAccumulator`] — the shared TAIL every built-in
/// aggregate in this module runs, whether its own per-row item `T` is a single
/// evaluated [`TermValue`] (every unary built-in) or a zero-sized unit
/// (`COUNT(*)`, which folds a run of survivors none of whose VALUES matter —
/// see `eval_aggregate`).
///
/// Generic over the concrete accumulator type `A`, monomorphized once per
/// built-in function at this function's seven (plus `COUNT(*)`, which shares
/// `COUNT(?x)`'s instantiation) call sites in `eval_aggregate` — "static
/// dispatch via generics": every `step` call in the hot per-row loop below is
/// an ordinary, fully-inlinable call on a concrete type, never a vtable call
/// through `&dyn`/`Box<dyn AggregateAccumulator>`. The trait's `combine`/
/// `finish` DO require a `Box<dyn AggregateAccumulator>` receiver (the shape
/// [`crate::agg_fn::AggregateAccumulator`] needs so a HOST-registered,
/// dynamically-resolved aggregate can implement the exact same trait) — this
/// function pays that box allocation only where the trait forces it: once per
/// CHUNK at `combine` (not per row — `combine` runs `O(chunks)` times, never
/// `O(rows)`, see `crate::parallel::par_chunk_reduce_init`) and once per GROUP
/// at `finish`. Both boxes are immediately consumed by
/// [`crate::agg_fn::downcast_combine_partial`], which always succeeds here:
/// the box handed to `combine` was built one line above from the SAME
/// concrete `A` `par_chunk_reduce_init`'s `init` just produced, in this same
/// function, never crossing a host/dyn boundary the way a registered custom
/// aggregate's accumulator does — reusing the identical, already-contained
/// downcast machinery `crate::agg_fn`'s own doc comments describe for that
/// case, at a strictly narrower and provably-safe use.
pub(crate) fn fold_builtin<A: crate::agg_fn::AggregateAccumulator, T: Sync>(
    survivors: &[T],
    init: impl Fn() -> A + Sync,
    step: impl Fn(&mut A, &T) -> Result<(), EvalError> + Sync,
) -> Result<Option<TermValue>, EvalError> {
    let fold = crate::parallel::par_chunk_reduce_init(
        survivors,
        || Ok(init()),
        step,
        |acc: &mut A, other: A| acc.combine(Box::new(other)),
    )?;
    Box::new(fold).finish()
}

/// Fold a group through a registered [`crate::agg_fn::CustomAggregate`]:
/// resolve the IRI, meter its declared per-accumulator state bound, then stream
/// every row's positional argument tuple through
/// [`crate::agg_fn::step_contained`], deduping on the FULL tuple under
/// `DISTINCT` (fixing the multi-argument case a single-argument built-in never
/// exercises; for an arity-1 call this reduces to exactly the same "first
/// occurrence kept" rule the built-in path applies).
///
/// # Errors
///
/// [`EvalError::Function`] if `iri` is unregistered (a defense-in-depth repeat of
/// `crate::property_fn_plan::plan_query`'s prepare-time refusal — reachable here
/// only when a caller evaluates algebra that bypassed that walk) or if the
/// resolved aggregate panics anywhere in its contained lifecycle.
pub(crate) fn eval_custom_aggregate<D: DatasetView + Sync>(
    iri: &str,
    agg: &AggregateExpression,
    idxs: &[usize],
    rows: &[Solution<D::Id>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let custom = ctx.aggregates.resolve(iri).cloned().ok_or_else(|| {
        EvalError::function(format!("no custom aggregate is registered for <{iri}>"))
    })?;

    // Meter the declared per-accumulator state bound against the scratch-arena
    // ceiling AT ADMISSION (once per group's accumulator), because a custom
    // accumulator's retained state is opaque host Rust the evaluator cannot
    // observe mid-fold the way it observes a minted `TermValue` the instant the
    // scratch interner produces one — see `CustomAggregate::state_bound`'s docs.
    let state_bound = crate::agg_fn::state_bound_contained(custom.as_ref(), iri)?;
    if let Err(tripped) =
        ctx.charge_amount(purrdf_core::ResourceDimension::ScratchBytes, state_bound)
    {
        ctx.expression_barrier.record(tripped);
        return Ok(None);
    }

    // Phase 1: evaluate every row's positional argument tuple against `ctx`,
    // charge `AggregateAccumulation` for each one, apply `DISTINCT` on the full
    // tuple, and charge `ScratchBytes` for each tuple actually retained into
    // `survivors` — otherwise unchanged from before within-group chunking
    // existed. See `eval_aggregate`'s matching phase-1 comment for why the REST
    // of this phase's spend stays a pure function of `idxs`/`ctx` regardless of
    // what phase 2 below does, and for why `survivors`' retained `TermValue`
    // clones need their own `ScratchBytes` charge: `ctx.scratch.value_of` mints
    // nothing, so [`EvalCtx::charge_scratch_growth`]'s automatic arena charge
    // never sees them, and this per-group buffer is otherwise-uncharged memory
    // proportional to the group's cardinality.
    let mut seen: Option<DetHashSet<Vec<TermValue>>> = agg.distinct.then(DetHashSet::default);
    let mut tuple: Vec<TermValue> = Vec::with_capacity(agg.args().len());
    let mut survivors: Vec<Vec<TermValue>> = Vec::new();
    for &i in idxs {
        tuple.clear();
        let mut every_position_bound = true;
        for expression in agg.args() {
            let Some(term) = eval_expr(expression, &rows[i], schema, ctx)? else {
                every_position_bound = false;
                break;
            };
            tuple.push(ctx.scratch.value_of(ctx.dataset, term));
        }
        if !every_position_bound {
            continue;
        }
        // The `aggregate-accumulation` charge point, charged once the row's whole
        // argument tuple is bound — BEFORE the `DISTINCT` check below, exactly as
        // the built-in fold path in `eval_aggregate` charges it: producing and
        // inspecting the tuple is the work this point prices, whether or not
        // `DISTINCT` goes on to discard it.
        if let Err(tripped) = ctx.charge(ChargePoint::AggregateAccumulation) {
            ctx.expression_barrier.record(tripped);
            return Ok(None);
        }
        if let Some(seen) = seen.as_mut()
            && !seen.insert(tuple.clone())
        {
            continue;
        }
        let tuple_bytes: u64 = tuple
            .iter()
            .map(crate::scratch::value_bytes)
            .fold(0u64, u64::saturating_add);
        if let Err(tripped) =
            ctx.charge_amount(purrdf_core::ResourceDimension::ScratchBytes, tuple_bytes)
        {
            ctx.expression_barrier.record(tripped);
            return Ok(None);
        }
        // Move the tuple into `survivors` rather than cloning it a second time: the
        // `seen` clone above (when `DISTINCT` is engaged) is the one genuinely
        // unavoidable copy, because that set and this buffer are two independent
        // owners of the same content. `tuple` is replaced with a fresh, correctly
        // sized buffer so the next iteration's `tuple.clear()` still has the
        // capacity this loop already paid for.
        survivors.push(std::mem::replace(
            &mut tuple,
            Vec::with_capacity(agg.args().len()),
        ));
    }

    // Phase 2: fold the (already `DISTINCT`-resolved, already in row order)
    // survivor tuples — chunked in parallel for a large enough group, but ONLY
    // when `custom` declares `Volatility::Stable` (mirrors
    // `EvalCtx::may_fork_aggregate`'s own per-group gate at the finer,
    // within-group grain: a `Volatile` aggregate's `step`/`combine` may depend
    // on state this evaluator does not control, so it stays on ONE
    // accumulator, sequentially, exactly as before this increment existed).
    //
    // A chunked fold creates MORE than one live accumulator at once (one per
    // chunk, combined down afterward) — [`CustomAggregate::state_bound`]'s
    // admission charge above metered exactly ONE accumulator's declared bound,
    // so chunking must charge the EXTRA accumulators beyond that first one
    // before creating them, using [`crate::parallel::planned_aggregate_chunk_count`]
    // (the exact count [`crate::parallel::par_chunk_reduce_init`] will create, not
    // an estimate) so this stays an honest upper bound on peak retained state. That
    // count is a pure function of `survivors.len()` alone — see
    // [`crate::parallel::aggregate_chunk_size_for`]'s doc comment — so this charge,
    // unlike an earlier increment's, cannot vary with the host's thread count.
    let stable = crate::agg_fn::volatility_contained(custom.as_ref(), iri)? == Volatility::Stable;
    let chunk_count = if stable {
        crate::parallel::planned_aggregate_chunk_count(survivors.len())
    } else {
        1
    };
    if chunk_count > 1 {
        let extra = state_bound.saturating_mul(u64::try_from(chunk_count - 1).unwrap_or(u64::MAX));
        if let Err(tripped) = ctx.charge_amount(purrdf_core::ResourceDimension::ScratchBytes, extra)
        {
            ctx.expression_barrier.record(tripped);
            return Ok(None);
        }
    }
    // The call site's `; NAME=value` scalarval clauses, resolved to `TermValue`
    // ONCE, ahead of every accumulator this fold creates — a scalarval is ONE
    // value for the WHOLE aggregation (see `AggregateExpression::scalarvals`'s
    // docs), never re-evaluated per row or per chunk. Already validated at
    // prepare time (`crate::property_fn_plan::plan_aggregate`) against
    // `custom`'s declared `CustomAggregate::scalarvals`: every name known, no
    // duplicate, every declared name present, every value the right kind — so
    // `CustomAggregate::init` below can trust this slice without re-checking it.
    let scalarvals: Vec<(String, TermValue)> = agg
        .scalarvals()
        .iter()
        .map(|(name, literal)| (name.clone(), literal_to_value(literal)))
        .collect();
    let force_sequential = (!stable).then(crate::parallel::force_sequential_operation);
    let accumulator = crate::parallel::par_chunk_reduce_init(
        &survivors,
        || crate::agg_fn::init_contained(custom.as_ref(), iri, &scalarvals),
        |accumulator, tuple| crate::agg_fn::step_contained(accumulator.as_mut(), iri, tuple),
        |accumulator, other| crate::agg_fn::combine_contained(accumulator.as_mut(), iri, other),
    )?;
    drop(force_sequential);

    let value = crate::agg_fn::finish_contained(accumulator, iri)?;
    Ok(value.map(|v| ctx.scratch.intern(ctx.dataset, v)))
}

/// Whether an [`XsdValue`] belongs to the SPARQL numeric tower (integer / decimal /
/// float / double). Boolean, string, temporal, and binary values are NOT numeric.
pub(crate) fn is_numeric_xsd(v: &XsdValue) -> bool {
    matches!(
        v,
        XsdValue::Integer { .. } | XsdValue::Decimal(_) | XsdValue::Float(_) | XsdValue::Double(_)
    )
}

/// The running numeric fold `SUM`/`AVG` share, wrapped `Option`-poisonable by
/// [`SumAccumulator`]/[`AvgAccumulator`] (see their docs for why the poisoned
/// state lives one level up, as `None`, rather than as a variant here).
///
/// A **pure-integer** running group ([`Self::Int`]) accumulates through
/// [`BigInt`] — arbitrary precision, so it never overflows regardless of how
/// large the true total gets; only a genuinely non-numeric value poisons it.
/// The moment a `decimal`/`float`/`double` value joins the group, the fold
/// promotes to [`Self::Ok`] and continues through `numeric_add`'s ordinary
/// (bounded, spec-defined) promotion tower exactly as before — see
/// [`int_sum_promote_base`] for the promotion step and why it is exact for
/// `decimal` whenever `decimal`'s own `i128`-bounded mantissa could hold the
/// value at all, and lossy-but-never-poisoning for `float`/`double` (IEEE,
/// never exact anyway). [`Self::Ok`]'s own arithmetic is untouched by this
/// module: `xsd:decimal` keeps its documented `i128`-mantissa bound and
/// `xsd:float`/`xsd:double` keep IEEE semantics, inf/NaN included.
///
/// ## `SUM`/`AVG` over `xsd:duration` — a PurRDF extension
///
/// SPARQL 1.1 §18.5.1.3 defines `SUM` as repeated `op:numeric-add`, whose domain
/// is the numeric tower alone; F&O has no `SUM`/`AVG` for `xsd:duration` either.
/// [`Self::Dur`] extends the aggregate algebra to the duration group, which
/// `.goals`' MAXIMAL UTILITY line asks for once nothing in [`is_numeric_xsd`]'s
/// gate has to move to reach it (see [`NumericFold::step`]'s doc for the exact
/// gate). The RAW `(months, seconds)` pair is an abelian group under
/// componentwise `+` unconditionally — see [`Self::Dur`]'s own doc for why the
/// fold accumulates that raw pair (rather than folding through
/// [`purrdf_xsd::temporal::add_durations`]'s validated `Duration` at every
/// step, which is NOT closed under `+` and made an earlier revision of this
/// fold order-dependent) — so the group sum is well-defined for any nonempty
/// multiset, independent of fold order and chunk boundaries; `AVG` is that
/// sum divided by the folded count.
#[derive(Debug, Clone)]
enum NumericFold {
    /// No value has been folded in yet.
    Empty,
    /// Every value folded in so far has been `xsd:integer` (or a derived
    /// integer facet): an exact, unbounded running total, plus the count of
    /// values folded (only `AVG` reads it) and the datatype to report if the
    /// fold never grows past this single value (see [`SumAccumulator`]'s docs
    /// on why a singleton group preserves its one literal's exact subtype,
    /// e.g. `xsd:byte`, while two or more folded values normalize to plain
    /// `xsd:integer` — `datatype` here mirrors that: it starts as the first
    /// value's own datatype and is stamped to [`XsdDatatype::Integer`] the
    /// instant a second value (via `step` OR `combine_owned`) is actually
    /// folded in).
    Int {
        sum: BigInt,
        count: u64,
        datatype: XsdDatatype,
    },
    /// A running sum plus the count of values folded so far (only `AVG` reads
    /// the count) — at `decimal`/`float`/`double` tier or higher; promotion is
    /// monotonic (`integer ⊂ decimal ⊂ float ⊂ double`), so once here the fold
    /// never returns to [`Self::Int`].
    Ok { acc: XsdValue, count: u64 },
    /// A running duration sum, carried as its own RAW `(months, seconds)`
    /// components rather than as an already-validated `XsdValue::Duration` —
    /// this decomposition is the fix for a real nondeterminism defect, not a
    /// stylistic choice; see the paragraph below for why.
    ///
    /// XSD 1.1 Part 2 §3.3.6's duration value space is the pair `(months,
    /// seconds)` with BOTH components non-negative or BOTH non-positive
    /// ([`purrdf_xsd::temporal::Duration::new`] enforces this — sign
    /// coherence). That set is NOT closed under componentwise `+`: e.g.
    /// `(12, 0) + (0, -86400) = (12, -86400)` is mixed-sign and therefore
    /// unrepresentable, even though summing the SAME three durations in a
    /// different order (or after a different chunk boundary) can land on a
    /// representable total instead. An earlier revision of this variant
    /// carried `acc: XsdValue::Duration` and folded through
    /// [`purrdf_xsd::temporal::add_durations`]/`purrdf_xsd::value_add` at every
    /// intermediate row — validating sign coherence at EVERY step, not just
    /// the final one — which therefore poisoned or not depending on fold
    /// order and on chunk boundaries: a genuine nondeterminism bug, since
    /// [`crate::parallel::par_chunk_reduce_init`] chunks a large group
    /// differently than a small one folds sequentially
    /// (`crate::parallel::PARALLEL_MIN_ROWS`), and both are legitimate ways to
    /// fold the identical multiset of rows to the identical answer.
    ///
    /// Sign coherence is a property of the RESULT, not of every partial sum on
    /// the way to it: the (months, seconds) pair sums over the free abelian
    /// group `ℤ × Decimal`, which IS totally order-independent (ordinary
    /// integer/decimal addition — no partiality anywhere in the group itself).
    /// So this variant accumulates the two components raw — `months` as a
    /// `checked_add`ed running `i128`, never validated against sign coherence
    /// mid-fold; `seconds` as an `XsdValue::Decimal` accumulated through
    /// [`numeric_add`] (the SAME function [`Self::Ok`]'s own decimal SUM uses,
    /// so duration seconds and a plain `xsd:decimal` SUM overflow identically)
    /// — and defers the ONE sign-coherence check to [`Self::finish_sum`]/
    /// [`Self::finish_avg`], on the fully-summed total. Because the raw
    /// accumulation itself is now genuinely order-independent, that single
    /// validation is order-independent too: sequential folding and every
    /// chunking agree on the SAME raw total and therefore on the SAME
    /// validation outcome — representable → the same value; unrepresentable →
    /// unbound, deterministically, every time.
    ///
    /// `months` is `i128`, not the `i64` [`purrdf_xsd::temporal::Duration::new`]
    /// itself requires: a running sum of `n` `i64`-bounded values cannot
    /// overflow `i128` until `n` exceeds roughly `2^64` rows, which is not a
    /// group size any real query reaches, so the accumulation is total in
    /// practice — only the narrowing back to `i64` at `finish` can fail
    /// (checked anyway, for honesty, not because it is expected to fire).
    ///
    /// `datatype` is the joined result tag ([`join_duration_datatype`]:
    /// `dayTimeDuration` iff every folded value declared it, likewise
    /// `yearMonthDuration`, else the general `xsd:duration`) — a genuine
    /// semilattice join (associative, commutative, idempotent on its own), so
    /// folding it per step, unlike the sign check, stays safe.
    Dur {
        /// Raw running months total — see this variant's own doc.
        months: i128,
        /// Raw running seconds total, always `XsdValue::Decimal(_)` — see this
        /// variant's own doc.
        seconds: XsdValue,
        /// The joined result tag — see this variant's own doc.
        datatype: XsdDatatype,
        /// The folded row count (`AVG` reads it).
        count: u64,
    },
}

impl NumericFold {
    /// Fold `value` in. `false` means `value` POISONS the fold — a genuinely
    /// non-numeric value, or an arithmetic failure promoting/adding it — and
    /// the caller ([`SumAccumulator`]/[`AvgAccumulator`]) turns that into its
    /// own `None`, discarding this state permanently; `true` means `self` now
    /// reflects `value` folded in.
    ///
    /// This match is exhaustive over `Empty`/`Int`/`Ok`/`Dur`: poisoning has no
    /// variant of its own to (mis)match here, because the wrapper's
    /// `Option<Self>` going from `Some` to `None` — never a fifth variant of
    /// THIS type — is where "poisoned" is expressed. There is therefore no
    /// state this match must defensively refuse to handle.
    ///
    /// The gate below accepts the numeric tower OR a duration, never both in
    /// the same group: the duration check sits entirely on
    /// [`is_numeric_xsd`]'s **failure path** (short-circuit `&&`), so a numeric
    /// value executes exactly the branches it executed before [`Self::Dur`]
    /// existed — [`is_numeric_xsd`] itself is unchanged and untouched by this
    /// widening (see its own doc for why: widening THAT predicate, rather than
    /// gating here, would let a mixed numeric+duration group silently coerce
    /// through whichever other call site trusts it). A group that mixes the
    /// two poisons: `Self::Int`/`Self::Ok` reject a duration value through
    /// their own existing arithmetic (`int_sum_promote_base`'s `None` arm, or
    /// `numeric_add`'s `TypeMismatch`, respectively — neither needed an edit),
    /// and `Self::Dur` rejects a numeric value explicitly below.
    fn step(&mut self, value: &TermValue) -> bool {
        let Some(xv) = xsd_of(value) else {
            return false;
        };
        if !is_numeric_xsd(&xv) && !matches!(xv, XsdValue::Duration(_)) {
            return false;
        }
        match self {
            Self::Empty => {
                *self = match xv {
                    XsdValue::Integer { value, datatype } => Self::Int {
                        sum: BigInt::from_i128(value),
                        count: 1,
                        datatype,
                    },
                    XsdValue::Duration(dur) => Self::Dur {
                        months: i128::from(dur.months()),
                        seconds: XsdValue::Decimal(dur.seconds()),
                        datatype: dur.datatype(),
                        count: 1,
                    },
                    other => Self::Ok {
                        acc: other,
                        count: 1,
                    },
                };
                true
            }
            Self::Int {
                sum,
                count,
                datatype,
            } => match &xv {
                XsdValue::Integer { value, .. } => {
                    sum.add_i128(*value);
                    *count += 1;
                    *datatype = XsdDatatype::Integer;
                    true
                }
                other => match int_sum_promote_base(sum, other) {
                    Some(base) => match numeric_add(&base, other) {
                        Ok(result) => {
                            *self = Self::Ok {
                                acc: result,
                                count: *count + 1,
                            };
                            true
                        }
                        Err(_) => false,
                    },
                    None => false,
                },
            },
            Self::Ok { acc, count } => match numeric_add(acc, &xv) {
                Ok(sum) => {
                    *acc = sum;
                    *count += 1;
                    true
                }
                Err(_) => false,
            },
            Self::Dur {
                months,
                seconds,
                datatype,
                count,
            } => {
                // The top-of-function gate admits only the numeric tower or a
                // duration; a numeric value reaching an already-`Dur` fold is
                // exactly the mixed-group case, and poisons.
                let XsdValue::Duration(dur) = &xv else {
                    return false;
                };
                // Raw componentwise accumulation — no `Duration::new` call, no
                // sign check, here. See `Self::Dur`'s own doc for why: the
                // check belongs once, on the finished total, not on every
                // partial sum along the way.
                let Some(new_months) = months.checked_add(i128::from(dur.months())) else {
                    return false;
                };
                let Ok(new_seconds) = numeric_add(seconds, &XsdValue::Decimal(dur.seconds()))
                else {
                    return false;
                };
                *months = new_months;
                *seconds = new_seconds;
                *datatype = join_duration_datatype(*datatype, dur.datatype());
                *count += 1;
                true
            }
        }
    }

    /// `SUM`'s finish: empty group → `0^^xsd:integer` (SPARQL §18.5.1);
    /// otherwise the running total — exact for a pure-integer group (whatever
    /// its magnitude, via [`BigInt::to_decimal_string`] when it no longer fits
    /// `i128`), the `decimal`/`float`/`double`-tower total, or (PurRDF
    /// extension) the group's duration total, rendered through the same
    /// [`crate::expr::xsd_literal_value`] [`Self::Ok`] uses. A duration-typed
    /// group is never `Self::Empty` at finish: [`Self::step`] only creates
    /// [`Self::Dur`] on the FIRST folded duration, so the empty-group `0` row
    /// above is reached only when literally nothing was folded, exactly as
    /// SPARQL's `SUM(empty) = 0` requires regardless of the group's would-be
    /// type.
    ///
    /// Returns `None` — poisoning to SPARQL unbound — in exactly one case:
    /// [`Self::Dur`]'s raw `(months, seconds)` total fails
    /// [`purrdf_xsd::temporal::Duration::new`]'s validation (mixed-sign
    /// components, or a months total that no longer fits `i64`) — see that
    /// variant's own doc for why this single, order-independent check is
    /// deferred all the way to here rather than applied at every fold step.
    /// `Self::Empty`/`Self::Int`/`Self::Ok` remain unconditionally infallible,
    /// exactly as before [`Self::Dur`]'s raw-component representation existed:
    /// nothing past `step`'s own poisoning (already handled by the wrapper,
    /// see [`SumAccumulator`]) can make one of those three unrepresentable.
    fn finish_sum(self) -> Option<TermValue> {
        match self {
            Self::Empty => Some(integer_value(0)),
            Self::Int { sum, datatype, .. } => Some(int_sum_value(&sum, datatype)),
            Self::Ok { acc, .. } => Some(crate::expr::xsd_literal_value(&acc)),
            Self::Dur {
                months,
                seconds,
                datatype,
                ..
            } => {
                let months = i64::try_from(months).ok()?;
                let XsdValue::Decimal(seconds) = seconds else {
                    unreachable!("NumericFold::Dur's seconds field is always XsdValue::Decimal");
                };
                let dur = purrdf_xsd::temporal::Duration::new(months, seconds, datatype).ok()?;
                Some(crate::expr::xsd_literal_value(&XsdValue::Duration(dur)))
            }
        }
    }

    /// `AVG`'s finish: empty group → `0^^xsd:integer`; otherwise the running
    /// total divided by the folded count.
    ///
    /// A pure-integer running total that still fits `i128` divides exactly as
    /// before (unchanged `numeric_div` call, unchanged truncated-decimal
    /// result). One that no longer fits `i128` divides through
    /// [`purrdf_xsd::bigint_avg_decimal`] instead — an exact `BigInt`-scaled
    /// division by the (always-small) folded row count, mirroring
    /// `numeric_div`'s own truncated-to-18-fractional-digit `Decimal` result
    /// bit for bit. That helper itself answers `None` when the resulting
    /// MANTISSA does not fit `i128` — `xsd:decimal`'s `Decimal` representation
    /// is deliberately `i128`-mantissa-bounded (this crate's documented design,
    /// unmoved by this fold) — but THIS finish does not stop there: it falls
    /// back to [`purrdf_xsd::bigint_avg_decimal_lexical`], which renders the
    /// identical exact scale-18 quotient as raw lexical TEXT with no magnitude
    /// bound at all, the same bypass [`Self::finish_sum`]'s `int_sum_value`
    /// already uses for a pure-integer total that exceeds `i128`. So a `SUM`
    /// that escaped `i128` never has to poison `AVG`, full stop — not only when
    /// the quotient happens to still fit `i128` after scaling, but always. This
    /// makes the `Self::Int` arm infallible; unlike [`Self::finish_sum`] this
    /// function stays `Option`-returning only because [`Self::Ok`]'s
    /// `numeric_div` call can still fail (a `decimal`-tier intermediate
    /// overflow — see `purrdf_xsd::numeric::decimal_div_raw`).
    fn finish_avg(self) -> Option<TermValue> {
        match self {
            Self::Empty => Some(integer_value(0)),
            Self::Int { sum, count, .. } => {
                Some(purrdf_xsd::bigint_avg_decimal(&sum, count).map_or_else(
                    || TermValue::Literal {
                        lexical_form: purrdf_xsd::bigint_avg_decimal_lexical(&sum, count),
                        datatype: XSD_DECIMAL.to_owned(),
                        language: None,
                        direction: None,
                    },
                    |avg| crate::expr::xsd_literal_value(&avg),
                ))
            }
            Self::Ok { acc, count } => {
                let count_val = XsdValue::Integer {
                    value: i128::from(count),
                    datatype: XsdDatatype::Integer,
                };
                numeric_div(&acc, &count_val)
                    .ok()
                    .map(|avg| crate::expr::xsd_literal_value(&avg))
            }
            Self::Dur {
                months,
                seconds,
                datatype,
                count,
            } => {
                // Mirrors `finish_sum`'s deferred-validation shape, but the
                // MEAN — not the sum — is what must pass `Duration::new` here:
                // a group whose raw SUM would be sign-incoherent can still
                // have a representable MEAN (and vice versa), so this cannot
                // simply divide `finish_sum`'s already-summed answer. Months
                // divide with the same ties-toward-positive-infinity rule
                // `purrdf_xsd::temporal::divide_duration` applies internally
                // (its own private `round_decimal_to_i64`), replicated here as
                // `round_i128_div_to_i64` because the numerator is the fold's
                // raw `i128` accumulator, not a `Decimal` `divide_duration`
                // could be handed directly. Seconds divide through
                // `numeric_div` — the same function [`Self::Ok`]'s own AVG
                // uses — for the identical truncated-to-18-fractional-digit
                // `Decimal` result plain decimal AVG gets.
                let divisor = i128::from(count);
                let mean_months = round_i128_div_to_i64(months, divisor)?;
                let count_val = XsdValue::Integer {
                    value: divisor,
                    datatype: XsdDatatype::Integer,
                };
                let XsdValue::Decimal(mean_seconds) = numeric_div(&seconds, &count_val).ok()?
                else {
                    unreachable!("numeric_div(Decimal, Integer) always answers XsdValue::Decimal");
                };
                let dur = purrdf_xsd::temporal::Duration::new(mean_months, mean_seconds, datatype)
                    .ok()?;
                Some(crate::expr::xsd_literal_value(&XsdValue::Duration(dur)))
            }
        }
    }

    /// Merge `a` and `b` — `a` the earlier (in source/chunk order) partial
    /// fold, `b` the later one — returning `None` when the merge itself
    /// poisons (a `decimal`-tier promotion or `numeric_add` failure; see
    /// [`int_sum_promote_base`]). `Commutative` per
    /// [`crate::agg_fn::AlgebraicClass`]: `BigInt` addition is exact and
    /// associative/commutative with no caveat at all — a pure-integer group's
    /// chunked total agrees with its sequential total, and with every OTHER
    /// chunking of the same group, byte for byte, because [`BigInt`] cannot
    /// overflow. `numeric_add` (once the fold is at `decimal`/`float`/`double`
    /// tier) is likewise associative/commutative in the real-number sense, so
    /// combining chunk partials in chunk order produces the same total
    /// `op:numeric-add` would folding the whole group sequentially — modulo
    /// `decimal`'s own documented `i128`-mantissa bound and `float`/`double`'s
    /// IEEE rounding, neither of which this module changes.
    ///
    /// [`Self::Dur`] is `Commutative` too, and — thanks to the raw-component
    /// representation [`Self::Dur`]'s own doc describes — WITHOUT caveat:
    /// componentwise `+` over the raw `(months, seconds)` pair is ordinary
    /// integer/decimal addition over the free abelian group `ℤ × Decimal`,
    /// genuinely associative/commutative with no partiality anywhere in the
    /// accumulation itself (an earlier revision of this fold validated sign
    /// coherence at every intermediate `step`/`combine`, which made the
    /// answer depend on chunk boundaries — exactly the nondeterminism this
    /// representation exists to rule out). Combining chunk partials in chunk
    /// order therefore agrees with folding the whole group sequentially on
    /// the RAW total, byte for byte, before the one sign-coherence check
    /// either path defers to `finish`.
    ///
    /// [`crate::parallel::par_chunk_reduce_init`] chunks through
    /// [`crate::parallel::aggregate_chunk_size_for`], which is a pure function of the
    /// group's row count — never of `rayon::current_num_threads()` — so for a given
    /// (query, data) pair there is exactly ONE chunking in production, reproduced
    /// identically on every host and every run, on top of the exactness above.
    fn combine_owned(a: Self, b: Self) -> Option<Self> {
        match (a, b) {
            (Self::Empty, other) | (other, Self::Empty) => Some(other),
            (
                Self::Int {
                    sum: mut sum1,
                    count: count1,
                    ..
                },
                Self::Int {
                    sum: sum2,
                    count: count2,
                    ..
                },
            ) => {
                sum1.add_assign(&sum2);
                Some(Self::Int {
                    sum: sum1,
                    count: count1 + count2,
                    datatype: XsdDatatype::Integer,
                })
            }
            (
                Self::Ok {
                    acc: acc1,
                    count: count1,
                },
                Self::Ok {
                    acc: acc2,
                    count: count2,
                },
            ) => numeric_add(&acc1, &acc2).ok().map(|sum| Self::Ok {
                acc: sum,
                count: count1 + count2,
            }),
            (
                Self::Int {
                    sum, count: count1, ..
                },
                Self::Ok { acc, count: count2 },
            )
            | (
                Self::Ok { acc, count: count2 },
                Self::Int {
                    sum, count: count1, ..
                },
            ) => int_sum_promote_base(&sum, &acc)
                .and_then(|base| numeric_add(&base, &acc).ok())
                .map(|result| Self::Ok {
                    acc: result,
                    count: count1 + count2,
                }),
            (
                Self::Dur {
                    months: months1,
                    seconds: seconds1,
                    datatype: datatype1,
                    count: count1,
                },
                Self::Dur {
                    months: months2,
                    seconds: seconds2,
                    datatype: datatype2,
                    count: count2,
                },
            ) => months1.checked_add(months2).and_then(|months| {
                numeric_add(&seconds1, &seconds2)
                    .ok()
                    .map(|seconds| Self::Dur {
                        months,
                        seconds,
                        datatype: join_duration_datatype(datatype1, datatype2),
                        count: count1 + count2,
                    })
            }),
            // A duration chunk merged with a numeric chunk (either order) is
            // the cross-group mixing `step` already refuses within one
            // accumulator — a chunk boundary must not let it back in.
            (Self::Dur { .. }, Self::Int { .. } | Self::Ok { .. })
            | (Self::Int { .. } | Self::Ok { .. }, Self::Dur { .. }) => None,
        }
    }
}

/// The joined result tag for two duration operands' declared datatypes, used
/// per fold step by [`NumericFold::Dur`]. Mirrors `purrdf_xsd::temporal`'s own
/// (private) `duration_result_datatype`: `dayTimeDuration` iff both declare
/// it, `yearMonthDuration` iff both declare it, else the general
/// `xsd:duration`. A plain match over the pair, never a derived `Ord` + `max`
/// — `purrdf_xsd::temporal`'s own `Shape` doc explains why a duration tag has
/// no total order for `max` to invent one from. This join is a genuine
/// semilattice operation (associative, commutative, idempotent), unlike the
/// sign-coherence check [`NumericFold::Dur`] defers to `finish` — see that
/// variant's own doc.
fn join_duration_datatype(a: XsdDatatype, b: XsdDatatype) -> XsdDatatype {
    match (a, b) {
        (XsdDatatype::YearMonthDuration, XsdDatatype::YearMonthDuration) => {
            XsdDatatype::YearMonthDuration
        }
        (XsdDatatype::DayTimeDuration, XsdDatatype::DayTimeDuration) => {
            XsdDatatype::DayTimeDuration
        }
        _ => XsdDatatype::Duration,
    }
}

/// Round `numerator / denominator` (`denominator > 0`) to the nearest `i64`,
/// ties toward positive infinity — replicates
/// `purrdf_xsd::temporal::divide_duration`'s internal (private)
/// `round_decimal_to_i64` rounding rule, used by [`NumericFold::finish_avg`]'s
/// `Dur` arm to round the duration-AVG months MEAN. Reimplemented here rather
/// than reused because the numerator is the fold's raw `i128` accumulator
/// (see [`NumericFold::Dur`]'s own doc for why it stays raw through `finish`),
/// not a `Decimal` `divide_duration` could be handed directly.
///
/// Derivation: round-half-up-toward-`+∞` of `N / D` is `floor(N/D + 1/2) =
/// floor((2N + D) / (2D))`; `div_euclid` on a POSITIVE divisor is exactly
/// floor division, which is why `denominator > 0` (always true here — the
/// divisor is a nonzero folded row count) is load-bearing. `None` only on an
/// `i128` overflow computing `2 × numerator` (unreachable for any realistic
/// row count — see [`NumericFold::Dur`]'s own doc on why the raw accumulation
/// itself cannot overflow `i128` in practice) or an out-of-`i64`-range mean.
fn round_i128_div_to_i64(numerator: i128, denominator: i128) -> Option<i64> {
    debug_assert!(
        denominator > 0,
        "duration AVG divisor is a nonzero folded row count"
    );
    let doubled_numerator = numerator.checked_mul(2)?;
    let doubled_denominator = denominator.checked_mul(2)?;
    let biased = doubled_numerator.checked_add(denominator)?;
    i64::try_from(biased.div_euclid(doubled_denominator)).ok()
}

/// Convert a pure-integer running sum into the `XsdValue` `numeric_add` needs
/// once a `decimal`/`float`/`double` value `joining` the fold promotes it out
/// of [`NumericFold::Int`].
///
/// Exact (`XsdValue::Integer`) whenever the running sum still fits `i128` —
/// the overwhelmingly common case, and identical to what the fold already did
/// before it could exceed `i128` at all. Beyond that: `decimal`'s own mantissa
/// is `i128`-bounded too (see `crates/xsd`'s module docs), so a `joining`
/// decimal cannot be represented as a `Decimal` either — `None` (the caller
/// poisons), exactly as today's overflow behavior already would have, just
/// reached later. A `joining` float/double, however, is IEEE and never exact
/// regardless of magnitude, so the sum can be cast (lossily, precisely as the
/// existing `i128 → f64`/`f32` promotion already is — see `purrdf_xsd::numeric`)
/// with no representability question at all: this is the one case where a
/// running total that has escaped `i128` still avoids poisoning.
fn int_sum_promote_base(sum: &BigInt, joining: &XsdValue) -> Option<XsdValue> {
    if let Some(value) = sum.to_i128() {
        return Some(XsdValue::Integer {
            value,
            datatype: XsdDatatype::Integer,
        });
    }
    match joining {
        XsdValue::Float(_) => Some(XsdValue::Float(sum.to_f64() as f32)),
        XsdValue::Double(_) => Some(XsdValue::Double(sum.to_f64())),
        _ => None,
    }
}

/// Render a [`NumericFold::Int`]'s running sum as a `TermValue`: the exact
/// canonical lexical form either way — [`crate::expr::xsd_literal_value`]'s
/// usual `XsdValue::Integer` path when `sum` still fits `i128` (preserving
/// `datatype`, e.g. a singleton `xsd:byte` group — see [`NumericFold::Int`]'s
/// docs), or a directly-built `xsd:integer` literal from
/// [`BigInt::to_decimal_string`] when it does not: `XsdValue::Integer`'s
/// `i128` field cannot hold that magnitude, but `xsd:integer`'s LEXICAL space
/// is unbounded, so the term is built straight from the exact decimal string
/// rather than forced through a value representation that would have to lose
/// precision to accept it. Interning (via [`ScratchInterner`](crate::scratch::ScratchInterner))
/// happens once, at the caller — see [`fold_builtin`] — not here.
fn int_sum_value(sum: &BigInt, datatype: XsdDatatype) -> TermValue {
    match sum.to_i128() {
        Some(value) => crate::expr::xsd_literal_value(&XsdValue::Integer { value, datatype }),
        None => TermValue::Literal {
            lexical_form: sum.to_decimal_string(),
            datatype: XSD_INTEGER.to_owned(),
            language: None,
            direction: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Built-in aggregate accumulators — instances of `crate::agg_fn::AggregateAccumulator`
// ---------------------------------------------------------------------------
//
// Every type below is a genuine `impl crate::agg_fn::AggregateAccumulator`: the SAME
// trait a host-registered `crate::agg_fn::CustomAggregate` produces from its own
// `init`. [`fold_builtin`] is the single generic driver both this module's built-ins
// and `eval_custom_aggregate`'s registered ones ultimately bottom out in (directly
// here; through `crate::parallel::par_chunk_reduce_init` with a boxed trait object
// there) — one fold algebra, two ways of reaching a concrete type: statically, by
// generic monomorphization, when the type is known at compile time (every built-in);
// dynamically, through `Box<dyn AggregateAccumulator>`, when it is resolved from an
// IRI at run time (a registered custom aggregate). See [`eval_aggregate`]'s doc
// comment for the dispatch site, and [`fold_builtin`]'s for why the static path never
// pays a per-row vtable cost despite implementing the identical trait a dynamically
// dispatched aggregate uses.
//
// Each type below is its OWN concrete Rust type (not one enum-of-variants covering
// every built-in) specifically so a [`crate::agg_fn::AggregateAccumulator::combine`]
// mismatch is a COMPILE-TIME impossibility for the within-crate fold in
// [`fold_builtin`], never a runtime `Err` this crate's own built-in fold has to
// handle: `combine`'s trait signature takes `Box<dyn AggregateAccumulator>`, so the
// ONLY way to recover a typed value is [`crate::agg_fn::downcast_combine_partial`]'s
// `downcast::<Self>()` — for `SumAccumulator`, say, that can only ever produce a
// `SumAccumulator`, because no OTHER built-in type is ever boxed and handed to
// `SumAccumulator::combine` (`fold_builtin`'s `combine` closure boxes the SAME
// concrete `A` its own `init` just produced, one line above, every time — see that
// function's doc comment) — so every `?` below on `downcast_combine_partial`'s
// `Result` is unreachable in practice for a built-in, exactly as unreachable as the
// old `BuiltinFold::combine`'s mismatched-variant `unreachable!()` this design
// replaced, just expressed as a typed `Err` a HOST-registered aggregate's own
// mismatch can actually hit (see [`crate::agg_fn::AggregateAccumulator::combine`]'s
// trait docs) instead of as a panic no target could safely rely on containing.

/// `COUNT` (and, via [`eval_aggregate`], `COUNT(*)`) — folds a survivor count.
/// `step` ignores its argument entirely, which is exactly why `COUNT(*)` (whose
/// survivors carry no value at all, only the fact that a row survived) can share
/// this SAME type with `COUNT(?x)`/`COUNT(DISTINCT ?x)`.
#[derive(Default)]
struct CountAccumulator(i64);

impl crate::agg_fn::AggregateAccumulator for CountAccumulator {
    fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
        self.0 += 1;
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        self.0 += other.0;
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(integer_value(self.0)))
    }
}

/// `SUM` — wraps [`NumericFold`], `None` meaning the fold has poisoned (a
/// non-numeric value, or an arithmetic failure — see [`NumericFold::step`]) and
/// stays `None` from then on (`step`/`combine` on an already-poisoned
/// accumulator are no-ops, mirroring the prior materializing implementation's
/// "poisoned state ignores every further step"). `Default` seeds
/// `Some(NumericFold::Empty)`, NOT `None`: a fresh accumulator has folded
/// nothing, which is a valid (zero-answering) state, not a poisoned one.
struct SumAccumulator(Option<NumericFold>);

impl Default for SumAccumulator {
    fn default() -> Self {
        Self(Some(NumericFold::Empty))
    }
}

impl crate::agg_fn::AggregateAccumulator for SumAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let Some(value) = args.first() else {
            return Ok(());
        };
        if let Some(state) = self.0.as_mut()
            && !state.step(value)
        {
            self.0 = None;
        }
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        self.0 = match (self.0.take(), other.0) {
            (Some(a), Some(b)) => NumericFold::combine_owned(a, b),
            _ => None,
        };
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.0.and_then(NumericFold::finish_sum))
    }
}

/// `AVG` — the [`SumAccumulator`] twin, differing only in which
/// [`NumericFold`] finish it calls (`finish_avg`, which — like `finish_sum`
/// — can still answer unbound at FINISH time even over a fold that never
/// poisoned in `step`; see that method's docs).
struct AvgAccumulator(Option<NumericFold>);

impl Default for AvgAccumulator {
    fn default() -> Self {
        Self(Some(NumericFold::Empty))
    }
}

impl crate::agg_fn::AggregateAccumulator for AvgAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let Some(value) = args.first() else {
            return Ok(());
        };
        if let Some(state) = self.0.as_mut()
            && !state.step(value)
        {
            self.0 = None;
        }
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        self.0 = match (self.0.take(), other.0) {
            (Some(a), Some(b)) => NumericFold::combine_owned(a, b),
            _ => None,
        };
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.0.and_then(NumericFold::finish_avg))
    }
}

/// One step of `MIN`/`MAX`'s running extreme: `want` is `Ordering::Less` for
/// `MIN`, `Greater` for `MAX`. Ties ([`total_order`] returns anything other
/// than `want`) keep the EARLIER occurrence — the same left-fold tie-break the
/// original `values.iter().reduce(..)` implementation had, since `reduce` seeds
/// its accumulator with the first element and only replaces it when a later
/// element compares strictly better. Shared by [`MinAccumulator`] and
/// [`MaxAccumulator`]'s `step`/`combine`, parameterized by `want`, rather than
/// duplicated per direction.
fn fold_extreme(current: Option<TermValue>, value: TermValue, want: Ordering) -> TermValue {
    match current {
        None => value,
        Some(current_value) => {
            if total_order(&project(Some(&value)), &project(Some(&current_value))) == want {
                value
            } else {
                current_value
            }
        }
    }
}

/// `MIN` — the running SPARQL-order minimum; see [`fold_extreme`].
#[derive(Default)]
struct MinAccumulator(Option<TermValue>);

impl crate::agg_fn::AggregateAccumulator for MinAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(value) = args.first() {
            self.0 = Some(fold_extreme(self.0.take(), value.clone(), Ordering::Less));
        }
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        if let Some(value) = other.0 {
            self.0 = Some(fold_extreme(self.0.take(), value, Ordering::Less));
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.0)
    }
}

/// `MAX` — the running SPARQL-order maximum; see [`fold_extreme`].
#[derive(Default)]
struct MaxAccumulator(Option<TermValue>);

impl crate::agg_fn::AggregateAccumulator for MaxAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(value) = args.first() {
            self.0 = Some(fold_extreme(
                self.0.take(),
                value.clone(),
                Ordering::Greater,
            ));
        }
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        if let Some(value) = other.0 {
            self.0 = Some(fold_extreme(self.0.take(), value, Ordering::Greater));
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.0)
    }
}

/// `SAMPLE` — "first value wins" in row order; `combine` keeps `self`'s value
/// (the earlier chunk's) whenever it already has one, exactly mirroring
/// `step`'s own "only fill an empty slot" rule at chunk-merge granularity.
#[derive(Default)]
struct SampleAccumulator(Option<TermValue>);

impl crate::agg_fn::AggregateAccumulator for SampleAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if self.0.is_none()
            && let Some(value) = args.first()
        {
            self.0 = Some(value.clone());
        }
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        if self.0.is_none() {
            self.0 = other.0;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.0)
    }
}

/// `GROUP_CONCAT` — orders concatenation by row order (see the module-level
/// "Aggregate semantics" docs), joined by `sep` (owned: [`crate::agg_fn::AggregateAccumulator`]
/// requires `'static`, so this cannot borrow [`AggregateExpression::separator`]
/// the way the pre-unification fold did — cloned once per group, not per row).
struct GroupConcatAccumulator {
    sep: String,
    buf: String,
    started: bool,
    /// Set once a folded value has no lexical form ([`lexical_of`] returns
    /// `None` — a blank node or a triple term; `STR()` of either is a SPARQL
    /// type error). Poisons the fold exactly as [`NumericFold`] poisons
    /// `SUM`/`AVG` on a non-numeric value: the group's answer becomes unbound
    /// regardless of what would have followed, and no further value is
    /// appended to `buf` (which is discarded, not returned) once poisoned.
    poisoned: bool,
}

impl GroupConcatAccumulator {
    fn new(sep: String) -> Self {
        Self {
            sep,
            buf: String::new(),
            started: false,
            poisoned: false,
        }
    }
}

impl crate::agg_fn::AggregateAccumulator for GroupConcatAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if self.poisoned {
            return Ok(());
        }
        let Some(value) = args.first() else {
            return Ok(());
        };
        match lexical_of(value) {
            Some(lexical) => {
                if self.started {
                    self.buf.push_str(&self.sep);
                } else {
                    self.started = true;
                }
                self.buf.push_str(&lexical);
            }
            // A blank node or a triple term has no lexical form — `STR()`
            // of either is a SPARQL type error (§17.4.2.2/§21 of the
            // relevant Query spec), so GROUP_CONCAT poisons rather than
            // silently dropping the value, mirroring how SUM/AVG poison
            // on a non-numeric value (see `NumericFold::step`).
            None => {
                self.poisoned = true;
                self.buf.clear();
            }
        }
        Ok(())
    }

    fn combine(
        &mut self,
        other: Box<dyn crate::agg_fn::AggregateAccumulator>,
    ) -> Result<(), EvalError> {
        let other = crate::agg_fn::downcast_combine_partial::<Self>(other)?;
        if self.poisoned {
            // Already poisoned: nothing `other` holds can un-poison it.
        } else if other.poisoned {
            self.poisoned = true;
            self.buf.clear();
        } else if other.started {
            if self.started {
                self.buf.push_str(&self.sep);
            }
            self.buf.push_str(&other.buf);
            self.started = true;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        if self.poisoned {
            Ok(None)
        } else {
            Ok(Some(string_value(self.buf)))
        }
    }
}

/// The lexical string of a term for GROUP_CONCAT: a literal's lexical form, or
/// an IRI's string — the same two kinds `STR()` accepts without error (an IRI's
/// "lexical form" here is its IRI string, `STR`-consistent). `None` for a blank
/// node or a triple term: `STR()` of either is a SPARQL type error, and
/// [`GroupConcatAccumulator::step`] poisons the fold on `None` rather than
/// silently dropping the value — see the module-level "Aggregate semantics"
/// docs' `GROUP_CONCAT` section for the full reading.
pub(crate) fn lexical_of(value: &TermValue) -> Option<String> {
    match value {
        TermValue::Literal { lexical_form, .. } => Some(lexical_form.clone()),
        TermValue::Iri(iri) => Some(iri.clone()),
        TermValue::Blank { .. } | TermValue::Triple { .. } => None,
    }
}

/// Build an `xsd:integer` literal value (not interned — see [`fold_builtin`]).
fn integer_value(value: i64) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_string(),
        datatype: XSD_INTEGER.to_owned(),
        language: None,
        direction: None,
    }
}

/// Build an `xsd:string` literal value (not interned — see [`fold_builtin`]).
fn string_value(lexical: String) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical,
        datatype: XSD_STRING.to_owned(),
        language: None,
        direction: None,
    }
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_algebra::Expression;

    use super::*;
    use crate::eval::eval;

    // ── value-level entry points ────────────────────────────────────────────

    fn iri(s: &str) -> TermValue {
        TermValue::Iri(s.to_owned())
    }

    fn integer(n: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: n.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        }
    }

    fn blank(label: &str) -> TermValue {
        TermValue::Blank {
            label: label.to_owned(),
            scope: purrdf_core::BlankScope::default(),
        }
    }

    fn triple(s: TermValue, p: TermValue, o: TermValue) -> TermValue {
        TermValue::Triple {
            s: Box::new(s),
            p: Box::new(p),
            o: Box::new(o),
        }
    }

    /// [`order_values`] applies the SPARQL kind ranking (blank < IRI < literal <
    /// triple) to a bag that mixes every RDF 1.2 term kind.
    #[test]
    fn order_values_ranks_every_rdf12_term_kind() {
        let t = triple(iri("http://ex/s"), iri("http://ex/p"), iri("http://ex/o"));
        let input = vec![t.clone(), integer("7"), iri("http://ex/i"), blank("b0")];
        assert_eq!(
            order_values(input.clone(), false),
            vec![blank("b0"), iri("http://ex/i"), integer("7"), t.clone()]
        );
        assert_eq!(
            order_values(input, true),
            vec![t, integer("7"), iri("http://ex/i"), blank("b0")]
        );
    }

    /// Literals order by VALUE, not lexically: `2` precedes `10`.
    #[test]
    fn order_values_orders_literals_by_value() {
        assert_eq!(
            order_values(vec![integer("10"), integer("2")], false),
            vec![integer("2"), integer("10")]
        );
    }

    /// The sort is stable, so duplicates survive and equal keys keep input order —
    /// an `ORDER BY` applies no `DISTINCT`.
    #[test]
    fn order_values_is_stable_and_keeps_duplicates() {
        assert_eq!(
            order_values(vec![integer("2"), integer("2"), integer("1")], false),
            vec![integer("1"), integer("2"), integer("2")]
        );
    }

    /// Triple terms compare componentwise, subject first.
    #[test]
    fn order_values_compares_triple_terms_componentwise() {
        let a = triple(iri("http://ex/s"), iri("http://ex/p"), iri("http://ex/a"));
        let b = triple(iri("http://ex/s"), iri("http://ex/p"), iri("http://ex/b"));
        assert_eq!(order_values(vec![b.clone(), a.clone()], false), vec![a, b]);
    }

    /// [`fold_values`] runs the same accumulators a `GROUP BY` runs, including the
    /// numeric promotion ladder.
    #[test]
    fn fold_values_matches_the_group_by_accumulators() {
        let vals = [integer("1"), integer("2"), integer("3")];
        assert_eq!(
            fold_values(ValueAggregate::Sum, &vals).expect("sum"),
            Some(integer("6"))
        );
        assert_eq!(
            fold_values(ValueAggregate::Min, &vals).expect("min"),
            Some(integer("1"))
        );
        assert_eq!(
            fold_values(ValueAggregate::Max, &vals).expect("max"),
            Some(integer("3"))
        );
        assert_eq!(
            fold_values(ValueAggregate::Count, &vals).expect("count"),
            Some(integer("3"))
        );
    }

    /// `MIN`/`MAX` rank a blank node and a triple term rather than refusing them.
    #[test]
    fn fold_values_ranks_blank_nodes_and_triple_terms() {
        let t = triple(iri("http://ex/s"), iri("http://ex/p"), iri("http://ex/o"));
        let vals = [t.clone(), iri("http://ex/i"), blank("b0")];
        assert_eq!(
            fold_values(ValueAggregate::Min, &vals).expect("min"),
            Some(blank("b0"))
        );
        assert_eq!(
            fold_values(ValueAggregate::Max, &vals).expect("max"),
            Some(t)
        );
    }

    /// The empty bag: `SUM` is `0`^^`xsd:integer`, `MIN`/`MAX`/`SAMPLE` unbound.
    #[test]
    fn fold_values_over_the_empty_bag() {
        assert_eq!(
            fold_values(ValueAggregate::Sum, &[]).expect("sum"),
            Some(integer("0"))
        );
        assert_eq!(fold_values(ValueAggregate::Min, &[]).expect("min"), None);
        assert_eq!(fold_values(ValueAggregate::Max, &[]).expect("max"), None);
        assert_eq!(
            fold_values(ValueAggregate::Sample, &[]).expect("sample"),
            None
        );
    }

    /// [`compare_values`] is the single-pair form of the same order.
    #[test]
    fn compare_values_agrees_with_order_values() {
        assert_eq!(
            compare_values(&integer("2"), &integer("10")),
            Ordering::Less
        );
        assert_eq!(
            compare_values(&blank("b0"), &iri("http://ex/i")),
            Ordering::Less
        );
        assert_eq!(
            compare_values(&iri("http://ex/i"), &iri("http://ex/i")),
            Ordering::Equal
        );
    }

    // These modifiers take the algebra node itself (it names the barrier and supplies the
    // child edge classification), so the tests build the node and drive the ordinary
    // dispatch — testing the wiring rather than an entry point no query reaches.
    fn eval_order_by<D: DatasetView<Id = purrdf_core::TermId> + Sync>(
        inner: &GraphPattern,
        exprs: &[OrderExpression],
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::OrderBy {
                inner: Box::new(inner.clone()),
                expression: exprs.to_vec(),
            },
            ctx,
        )
    }

    fn eval_distinct<D: DatasetView<Id = purrdf_core::TermId> + Sync>(
        inner: &GraphPattern,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::Distinct {
                inner: Box::new(inner.clone()),
            },
            ctx,
        )
    }

    fn eval_project<D: DatasetView<Id = purrdf_core::TermId> + Sync>(
        inner: &GraphPattern,
        variables: &[Variable],
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::Project {
                inner: Box::new(inner.clone()),
                variables: variables.to_vec(),
            },
            ctx,
        )
    }

    fn eval_slice<D: DatasetView<Id = purrdf_core::TermId> + Sync>(
        inner: &GraphPattern,
        start: usize,
        length: Option<usize>,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::Slice {
                inner: Box::new(inner.clone()),
                start,
                length,
            },
            ctx,
        )
    }

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
    use purrdf_sparql_algebra::{NamedNode, NamedNodePattern, TermPattern, TriplePattern};

    const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";

    fn ages() -> Arc<RdfDataset> {
        // :a :age 30 ; :b :age 17 ; :c :age 30  (duplicate age 30)
        let mut b = RdfDatasetBuilder::new();
        let age = b.intern_iri("http://ex/age");
        for (s, n) in [("a", "30"), ("b", "17"), ("c", "30")] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: n.to_owned(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, age, val, None);
        }
        b.freeze().expect("freeze")
    }

    fn age_bgp() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/age")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        }
    }

    fn ints(ds: &RdfDataset, seq: &SolutionSeq, var: &str) -> Vec<String> {
        let scratch = crate::scratch::ScratchInterner::new();
        let col = seq.schema.index_of(&Variable::new(var)).unwrap();
        seq.rows
            .iter()
            .filter_map(|r| r[col])
            .map(|t| match scratch.value_of(ds, t) {
                TermValue::Literal { lexical_form, .. } => lexical_form,
                other => format!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn order_by_ascending_value_space() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval_order_by(
            &age_bgp(),
            &[OrderExpression::Asc(Expression::Variable(Variable::new(
                "n",
            )))],
            &mut ctx,
        )
        .expect("order");
        // 17, 30, 30 — numeric (value-space) ascending.
        assert_eq!(ints(&ds, &seq, "n"), vec!["17", "30", "30"]);
    }

    #[test]
    fn order_by_descending() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval_order_by(
            &age_bgp(),
            &[OrderExpression::Desc(Expression::Variable(Variable::new(
                "n",
            )))],
            &mut ctx,
        )
        .expect("order");
        assert_eq!(ints(&ds, &seq, "n"), vec!["30", "30", "17"]);
    }

    #[test]
    fn distinct_drops_duplicate_rows() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        // Project to ?n only → {30, 17, 30}; DISTINCT → {30, 17}.
        let project = GraphPattern::Project {
            inner: Box::new(age_bgp()),
            variables: vec![Variable::new("n")],
        };
        let seq = eval_distinct(&project, &mut ctx).expect("distinct");
        assert_eq!(seq.len(), 2);
    }

    #[test]
    fn slice_offset_and_limit() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let ordered = GraphPattern::OrderBy {
            inner: Box::new(age_bgp()),
            expression: vec![OrderExpression::Asc(Expression::Variable(Variable::new(
                "n",
            )))],
        };
        // OFFSET 1 LIMIT 1 over [17,30,30] → [30].
        let seq = eval_slice(&ordered, 1, Some(1), &mut ctx).expect("slice");
        assert_eq!(ints(&ds, &seq, "n"), vec!["30"]);
    }

    #[test]
    fn project_keeps_only_listed_vars_in_order() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval_project(&age_bgp(), &[Variable::new("n")], &mut ctx).expect("project");
        assert_eq!(seq.schema.vars(), &[Variable::new("n")]);
        assert_eq!(seq.len(), 3);
    }

    fn typed_ages() -> Arc<RdfDataset> {
        // :a :type :T ; :age 30
        // :b :type :T ; :age 30
        // :c :type :T ; :age 17
        // :d :type :U ; :age 42
        let mut b = RdfDatasetBuilder::new();
        let age = b.intern_iri("http://ex/age");
        let ty = b.intern_iri("http://ex/type");
        let t = b.intern_iri("http://ex/T");
        let u = b.intern_iri("http://ex/U");
        for (s, n, g) in [
            ("a", "30", t),
            ("b", "30", t),
            ("c", "17", t),
            ("d", "42", u),
        ] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: n.to_owned(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, ty, g, None);
            b.push_quad(subj, age, val, None);
        }
        b.freeze().expect("freeze")
    }

    fn typed_age_bgp() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![
                TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                        "http://ex/type",
                    )),
                    object: TermPattern::Variable(Variable::new("t")),
                },
                TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                        "http://ex/age",
                    )),
                    object: TermPattern::Variable(Variable::new("n")),
                },
            ],
        }
    }

    #[test]
    fn group_by_with_count() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        // GROUP BY ?n COUNT(*) — group by age: {30→2, 17→1}.
        let group = GraphPattern::Group {
            inner: Box::new(age_bgp()),
            variables: vec![Variable::new("n")],
            aggregates: vec![(
                Variable::new("c"),
                AggregateExpression::new(
                    AggregateFunction::Count,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group");
        assert_eq!(seq.len(), 2);
        let ncol = seq.schema.index_of(&Variable::new("n")).unwrap();
        let ccol = seq.schema.index_of(&Variable::new("c")).unwrap();
        let scratch = crate::scratch::ScratchInterner::new();
        let mut pairs: Vec<(String, String)> = seq
            .rows
            .iter()
            .map(|r| {
                let n = match scratch.value_of(&ds, r[ncol].unwrap()) {
                    TermValue::Literal { lexical_form, .. } => lexical_form,
                    o => format!("{o:?}"),
                };
                // The count is a computed term — resolve via the eval scratch.
                let c = match ctx.scratch.value_of(&ds, r[ccol].unwrap()) {
                    TermValue::Literal { lexical_form, .. } => lexical_form,
                    o => format!("{o:?}"),
                };
                (n, c)
            })
            .collect();
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("17".to_owned(), "1".to_owned()),
                ("30".to_owned(), "2".to_owned())
            ]
        );
    }

    #[test]
    fn group_by_with_count_distinct() {
        // GROUP BY ?t COUNT(DISTINCT ?n) — T has ages {30,30,17} → 2 distinct,
        // U has ages {42} → 1.
        let ds = typed_ages();
        let mut ctx = EvalCtx::new(&ds);
        let group = GraphPattern::Group {
            inner: Box::new(typed_age_bgp()),
            variables: vec![Variable::new("t")],
            aggregates: vec![(
                Variable::new("c"),
                AggregateExpression::new(
                    AggregateFunction::Count,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    true,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group");
        assert_eq!(seq.len(), 2);
        let tcol = seq.schema.index_of(&Variable::new("t")).unwrap();
        let ccol = seq.schema.index_of(&Variable::new("c")).unwrap();
        let scratch = crate::scratch::ScratchInterner::new();
        let mut pairs: Vec<(String, String)> = seq
            .rows
            .iter()
            .map(|r| {
                let t = match scratch.value_of(&ds, r[tcol].unwrap()) {
                    TermValue::Iri(iri) => iri,
                    o => format!("{o:?}"),
                };
                let c = match ctx.scratch.value_of(&ds, r[ccol].unwrap()) {
                    TermValue::Literal { lexical_form, .. } => lexical_form,
                    o => format!("{o:?}"),
                };
                (t, c)
            })
            .collect();
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("http://ex/T".to_owned(), "2".to_owned()),
                ("http://ex/U".to_owned(), "1".to_owned())
            ]
        );
    }

    #[test]
    fn count_star_over_empty_is_one_group_zero() {
        // No GROUP BY, COUNT(*) over an empty result → one row binding 0.
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        // A BGP that matches nothing.
        let empty_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/none")),
                object: TermPattern::Variable(Variable::new("o")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(empty_bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("c"),
                AggregateExpression::new(
                    AggregateFunction::Count,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group");
        assert_eq!(seq.len(), 1);
        let ccol = seq.schema.index_of(&Variable::new("c")).unwrap();
        let c = match ctx.scratch.value_of(&ds, seq.rows[0][ccol].unwrap()) {
            TermValue::Literal { lexical_form, .. } => lexical_form,
            o => format!("{o:?}"),
        };
        assert_eq!(c, "0");
    }

    #[test]
    fn group_min() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        // MIN(?n) over the whole input → 17.
        let group_min = GraphPattern::Group {
            inner: Box::new(age_bgp()),
            variables: vec![],
            aggregates: vec![(
                Variable::new("m"),
                AggregateExpression::new(
                    AggregateFunction::Min,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group_min, &mut ctx).expect("min");
        let mcol = seq.schema.index_of(&Variable::new("m")).unwrap();
        let m = match ctx.scratch.value_of(&ds, seq.rows[0][mcol].unwrap()) {
            TermValue::Literal { lexical_form, .. } => lexical_form,
            o => format!("{o:?}"),
        };
        assert_eq!(m, "17");
    }

    /// Helper: resolve an aggregate column via the eval scratch.
    fn agg_lex(
        ds: &Arc<RdfDataset>,
        ctx: &EvalCtx<'_, Arc<RdfDataset>>,
        seq: &SolutionSeq,
        var: &str,
    ) -> String {
        let col = seq.schema.index_of(&Variable::new(var)).unwrap();
        match ctx.scratch.value_of(ds, seq.rows[0][col].unwrap()) {
            TermValue::Literal { lexical_form, .. } => lexical_form,
            o => format!("{o:?}"),
        }
    }

    #[test]
    fn sum_integers() {
        // SUM(?n) over {30, 17, 30} → 77.
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let group = GraphPattern::Group {
            inner: Box::new(age_bgp()),
            variables: vec![],
            aggregates: vec![(
                Variable::new("s"),
                AggregateExpression::new(
                    AggregateFunction::Sum,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("sum");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "s"), "77");
    }

    #[test]
    fn sum_with_decimal() {
        // Dataset: {1^^xsd:integer, 0.5^^xsd:decimal} → SUM = 1.5 (decimal).
        use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
        const XDEC: &str = "http://www.w3.org/2001/XMLSchema#decimal";
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/v");
        for (s, lex, dt) in [("a", "1", XINT), ("b", "0.5", XDEC)] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: lex.to_owned(),
                datatype: Some(dt.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, p, val, None);
        }
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);
        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/v")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("s"),
                AggregateExpression::new(
                    AggregateFunction::Sum,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("sum decimal");
        let result = agg_lex(&ds, &ctx, &seq, "s");
        assert!(
            result.starts_with("1.5"),
            "SUM(1, 0.5) should be 1.5…, got {result}"
        );
    }

    #[test]
    fn sum_empty_group_is_zero() {
        // SUM over an empty group with no GROUP BY → one row with 0^^xsd:integer.
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let empty_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/none")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(empty_bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("s"),
                AggregateExpression::new(
                    AggregateFunction::Sum,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("sum empty");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "s"), "0");
    }

    #[test]
    fn sum_non_numeric_is_unbound() {
        // SUM over a string value → unbound (Ok(None) in the aggregate output).
        use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/label");
        let subj = b.intern_iri("http://ex/x");
        let val = b.intern_literal(RdfLiteral::simple("hello"));
        b.push_quad(subj, p, val, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);
        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/label")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("agg"),
                AggregateExpression::new(
                    AggregateFunction::Sum,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("sum non-numeric");
        assert_eq!(seq.len(), 1);
        let col = seq.schema.index_of(&Variable::new("agg")).unwrap();
        // Non-numeric → unbound (None).
        assert!(
            seq.rows[0][col].is_none(),
            "SUM of non-numeric must be unbound"
        );
    }

    #[test]
    fn avg_integers() {
        // AVG(?n) over {2, 4} → 3.0 (decimal, NOT integer).
        use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/v");
        for (s, n) in [("a", "2"), ("b", "4")] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: n.to_owned(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, p, val, None);
        }
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);
        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/v")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("avg"),
                AggregateExpression::new(
                    AggregateFunction::Avg,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("avg");
        let result = agg_lex(&ds, &ctx, &seq, "avg");
        // AVG(2, 4) = 6 / 2 = 3 — result is decimal (integer ÷ integer → decimal);
        // XSD 1.1 whole-decimal lexical has no point ("3", not "3.0").
        assert_eq!(result, "3", "AVG(2,4) should be 3, got {result}");
    }

    #[test]
    fn avg_empty_group_is_zero() {
        // AVG over an empty group → 0^^xsd:integer.
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let empty_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/none")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(empty_bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("avg"),
                AggregateExpression::new(
                    AggregateFunction::Avg,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("avg empty");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "avg"), "0");
    }

    #[test]
    fn sum_group_by_integration() {
        // GROUP BY ?s, SUM(?n) per group: dataset has two subjects each with two values.
        use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/score");
        // :alice → 10, 20 ; :bob → 5, 15
        for (s, vals) in [("alice", vec!["10", "20"]), ("bob", vec!["5", "15"])] {
            for v in vals {
                let subj = b.intern_iri(&format!("http://ex/{s}"));
                let val = b.intern_literal(RdfLiteral {
                    lexical_form: v.to_owned(),
                    datatype: Some(XINT.to_owned()),
                    language: None,
                    direction: None,
                });
                b.push_quad(subj, p, val, None);
            }
        }
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);
        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("who")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/score")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(bgp),
            variables: vec![Variable::new("who")],
            aggregates: vec![(
                Variable::new("total"),
                AggregateExpression::new(
                    AggregateFunction::Sum,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group sum");
        assert_eq!(seq.len(), 2);
        let who_col = seq.schema.index_of(&Variable::new("who")).unwrap();
        let total_col = seq.schema.index_of(&Variable::new("total")).unwrap();
        let scratch = crate::scratch::ScratchInterner::new();
        let mut pairs: Vec<(String, String)> = seq
            .rows
            .iter()
            .map(|r| {
                let who = match scratch.value_of(&ds, r[who_col].unwrap()) {
                    TermValue::Iri(iri) => iri.split('/').next_back().unwrap_or("").to_owned(),
                    o => format!("{o:?}"),
                };
                let total = match ctx.scratch.value_of(&ds, r[total_col].unwrap()) {
                    TermValue::Literal { lexical_form, .. } => lexical_form,
                    o => format!("{o:?}"),
                };
                (who, total)
            })
            .collect();
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("alice".to_owned(), "30".to_owned()),
                ("bob".to_owned(), "20".to_owned()),
            ]
        );
    }

    /// Build a single-group `SUM(?n)`/`AVG(?n)` dataset from `(subject-suffix,
    /// lexical, datatype-iri)` triples on `ex:v`, in the given ROW order (insertion
    /// order — the BGP scan visits rows in exactly this order, so callers proving
    /// row-order independence pass the SAME multiset in different orders).
    fn numeric_fold_dataset(values: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/v");
        for (s, lex, dt) in values {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: (*lex).to_owned(),
                datatype: Some((*dt).to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, p, val, None);
        }
        b.freeze().expect("freeze")
    }

    /// Evaluate a bare (no `GROUP BY`) `function(?n)` over `ex:v` and return the
    /// result variable's lexical form, or `None` if unbound. A thin lexical-only
    /// projection of [`eval_numeric_fold_term`], which does the actual BGP +
    /// `Group` evaluation and additionally returns the result's datatype IRI.
    fn eval_numeric_fold(ds: &Arc<RdfDataset>, function: AggregateFunction) -> Option<String> {
        eval_numeric_fold_term(ds, function).map(|(lexical_form, _datatype)| lexical_form)
    }

    /// The exact reproduction from the reported gap: `SUM` over
    /// `{i128::MAX, 1, -i128::MAX}` must answer `"1"^^xsd:integer`, not unbound —
    /// the running total visits `i128::MAX` (fits) then `i128::MAX + 1` (does NOT
    /// fit `i128`) along the way, but the true mathematical total is a perfectly
    /// ordinary, representable `xsd:integer`. Checked in three different row
    /// orders (the multiset is identical; only insertion order — hence fold
    /// order — differs), proving the fix is not an artifact of one particular
    /// order visiting the overflow at a convenient moment.
    #[test]
    fn sum_overflow_cancelling_values_answers_exact_total() {
        let max = i128::MAX.to_string();
        let neg_max = format!("-{max}");
        let orders: [[(&str, &str, &str); 3]; 3] = [
            [("a", &max, XINT), ("b", "1", XINT), ("c", &neg_max, XINT)],
            [("c", &neg_max, XINT), ("a", &max, XINT), ("b", "1", XINT)],
            [("b", "1", XINT), ("c", &neg_max, XINT), ("a", &max, XINT)],
        ];
        for order in &orders {
            let ds = numeric_fold_dataset(order);
            let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
            assert_eq!(
                result.as_deref(),
                Some("1"),
                "SUM over {{i128::MAX, 1, -i128::MAX}} must be exact regardless of row \
                 order, got {result:?} for order {order:?}"
            );
        }
    }

    /// `AVG` over the same reproduction data as
    /// [`sum_overflow_cancelling_values_answers_exact_total`] must be exact too:
    /// the running SUM no longer poisons on the intermediate overflow, so `AVG`
    /// reaches its ordinary `numeric_div(sum, count)` finish exactly as it would
    /// for any other 3-row group. The expected value is computed through the
    /// SAME `numeric_div` the fold itself calls, so this pins "AVG uses the exact
    /// sum" rather than hand-duplicating `numeric_div`'s truncation behavior.
    #[test]
    fn avg_overflow_cancelling_values_is_exact() {
        let max = i128::MAX.to_string();
        let neg_max = format!("-{max}");
        let ds =
            numeric_fold_dataset(&[("a", &max, XINT), ("b", "1", XINT), ("c", &neg_max, XINT)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Avg);
        let one = XsdValue::Integer {
            value: 1,
            datatype: XsdDatatype::Integer,
        };
        let three = XsdValue::Integer {
            value: 3,
            datatype: XsdDatatype::Integer,
        };
        let expected = numeric_div(&one, &three).expect("1/3").canonical_lexical();
        assert_eq!(result.as_deref(), Some(expected.as_str()));
    }

    /// A total that genuinely exceeds `i128` (not merely visits an out-of-range
    /// INTERMEDIATE, but is truly wider than `i128` itself) must still answer
    /// exactly: `SUM({i128::MAX, i128::MAX})` = `2 × i128::MAX`, an entirely
    /// ordinary `xsd:integer` no `i128` field can hold, rendered from
    /// [`BigInt::to_decimal_string`] directly.
    #[test]
    fn sum_overflow_exceeding_i128_answers_exact_total() {
        let max = i128::MAX.to_string();
        let ds = numeric_fold_dataset(&[("a", &max, XINT), ("b", &max, XINT)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
        assert_eq!(
            result.as_deref(),
            Some("340282366920938463463374607431768211454"),
            "2 * i128::MAX must answer exactly, not unbound"
        );
    }

    /// `AVG` over a total that genuinely exceeds `i128`, where the scale-18
    /// quotient mantissa ALSO exceeds `i128` (`bigint_avg_decimal` alone would
    /// answer `None` here — see `purrdf_xsd::numeric`'s doc on it): must still
    /// answer exactly rather than go unbound.
    /// `AVG({i128::MAX, i128::MAX})` = `i128::MAX` exactly (the two `i128::MAX`
    /// values sum to `2 × i128::MAX`, divided by a count of 2), rendered through
    /// [`purrdf_xsd::bigint_avg_decimal_lexical`]'s TEXT bypass — the same shape
    /// [`sum_overflow_exceeding_i128_answers_exact_total`] pins for `SUM`. This
    /// is the exact fixture the crate's public rustdoc worked example describes
    /// (`(i128::MAX + i128::MAX) / 2 == i128::MAX`); before the lexical-text
    /// fallback existed this answered unbound instead, because `Decimal`'s own
    /// `i128`-mantissa bound rejects the scale-18 quotient even though the
    /// value itself is an ordinary integer.
    #[test]
    fn avg_overflow_exceeding_i128_answers_exact_total() {
        let max = i128::MAX.to_string();
        let ds = numeric_fold_dataset(&[("a", &max, XINT), ("b", &max, XINT)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Avg);
        assert_eq!(
            result.as_deref(),
            Some(max.as_str()),
            "AVG({{i128::MAX, i128::MAX}}) must answer i128::MAX exactly, not unbound"
        );
    }

    /// `xsd:double`'s IEEE semantics are untouched by the `AVG` lexical-text
    /// fallback: an overflowing double running total still saturates to IEEE
    /// infinity divided by the row count, which is still infinity — the
    /// spec-correct IEEE answer, never routed through the integer/`BigInt` path
    /// at all (`NumericFold::Ok`, not `NumericFold::Int` — see `finish_avg`
    /// above in this module).
    #[test]
    fn avg_double_overflow_is_ieee_infinity_not_poisoned_or_exact() {
        const XDOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        let huge = format!("{:e}", f64::MAX);
        let ds = numeric_fold_dataset(&[("a", &huge, XDOUBLE), ("b", &huge, XDOUBLE)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Avg);
        assert_eq!(result.as_deref(), Some("INF"));
    }

    /// `NaN` still propagates through a `double` `AVG`, per IEEE 754 — identical
    /// to [`sum_double_nan_propagates`], unaffected by the lexical-text fallback
    /// (which only ever fires for the pure-integer `NumericFold::Int` case).
    #[test]
    fn avg_double_nan_propagates() {
        const XDOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        let ds = numeric_fold_dataset(&[("a", "NaN", XDOUBLE), ("b", "5.0", XDOUBLE)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Avg);
        assert_eq!(result.as_deref(), Some("NaN"));
    }

    /// Once a pure-integer running sum has escaped `i128` (see
    /// [`sum_overflow_exceeding_i128_answers_exact_total`]), a `float`/`double`
    /// value joining the group must still promote the fold rather than poison
    /// it — `float`/`double` are IEEE and never exact regardless of magnitude, so
    /// there is no representability question the way there is for `decimal`
    /// (see the next test). The result is `xsd:double` (Double is the higher
    /// promotion tier once it appears), finite (`2 × i128::MAX ≈ 3.4e38` is far
    /// below `f64::MAX ≈ 1.8e308`).
    #[test]
    fn sum_overflow_then_double_promotes_without_poisoning() {
        const XDOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        let max = i128::MAX.to_string();
        let ds =
            numeric_fold_dataset(&[("a", &max, XINT), ("b", &max, XINT), ("c", "0.0", XDOUBLE)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
        let value = result
            .expect("must not be poisoned")
            .parse::<f64>()
            .expect("a double lexical");
        assert!(value.is_finite());
        let expected = 2.0_f64 * (i128::MAX as f64);
        assert!((value - expected).abs() / expected < 1e-9);
    }

    /// Once a pure-integer running sum has escaped `i128`, a `decimal` value
    /// joining the group DOES poison — `xsd:decimal`'s mantissa is `i128`-bounded
    /// by this crate's own documented design (`crates/xsd`'s module docs), so an
    /// out-of-`i128`-range integer sum cannot be represented as a `Decimal`
    /// either. This is the "genuinely unrepresentable in the result type" case
    /// the fix explicitly does not claim to have closed, and it is no worse than
    /// before: this exact group already poisoned prior to this change (on the
    /// very first `i128` overflow), just for a different proximate reason.
    #[test]
    fn sum_overflow_then_decimal_poisons_on_decimals_own_bound() {
        const XDEC: &str = "http://www.w3.org/2001/XMLSchema#decimal";
        let max = i128::MAX.to_string();
        let ds = numeric_fold_dataset(&[("a", &max, XINT), ("b", &max, XINT), ("c", "0.5", XDEC)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
        assert_eq!(
            result, None,
            "decimal cannot hold an out-of-i128 integer sum"
        );
    }

    /// `xsd:double`'s IEEE semantics are untouched by the integer-accumulator
    /// fix: an overflowing double `SUM` still saturates to IEEE infinity — the
    /// spec-correct IEEE answer — rather than becoming exact the way the integer
    /// tower now is. Mirrors `f64::MAX + f64::MAX == f64::INFINITY`.
    #[test]
    fn sum_double_overflow_is_ieee_infinity_not_poisoned_or_exact() {
        const XDOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        let huge = format!("{:e}", f64::MAX);
        let ds = numeric_fold_dataset(&[("a", &huge, XDOUBLE), ("b", &huge, XDOUBLE)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
        assert_eq!(result.as_deref(), Some("INF"));
    }

    /// `NaN` still propagates through a `double` `SUM`, per IEEE 754 — a `NaN`
    /// operand poisons every subsequent IEEE add, and `NaN`'s canonical
    /// `xsd:double` lexical form is `"NaN"`, never unbound.
    #[test]
    fn sum_double_nan_propagates() {
        const XDOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        let ds = numeric_fold_dataset(&[("a", "NaN", XDOUBLE), ("b", "5.0", XDOUBLE)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
        assert_eq!(result.as_deref(), Some("NaN"));
    }

    /// Mixed-type promotion is unchanged by the integer-accumulator fix: an
    /// `xsd:integer` `SUM`'d with an `xsd:float` still promotes to `xsd:float`
    /// (the same tier `numeric_add` always promoted mixed integer/float pairs
    /// to), whether or not the integer running total ever left `i128` along the
    /// way.
    #[test]
    fn sum_mixed_integer_and_float_promotes_to_float_as_before() {
        const XFLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
        let ds = numeric_fold_dataset(&[("a", "40", XINT), ("b", "2.5", XFLOAT)]);
        let result = eval_numeric_fold(&ds, AggregateFunction::Sum);
        assert_eq!(result.as_deref(), Some("4.25E1"));
    }

    /// The same overflow-cancelling total from
    /// [`sum_overflow_cancelling_values_answers_exact_total`], but folded across
    /// a genuinely large group (well above `crate::parallel::PARALLEL_MIN_ROWS`)
    /// on rayon pools of 1, 2, 8, and 32 workers — the multi-thread-pool harness
    /// pattern from `tests/governor_correctness.rs`'s
    /// `filter_exists_fuel_is_invariant_under_worker_count`. Chunk PLANNING is
    /// already a pure function of row count, never of `rayon::current_num_threads()`
    /// (see `crate::parallel::aggregate_chunk_size_for`), so this does not probe a
    /// different chunk boundary per pool; it proves the `BigInt` accumulation
    /// itself is race-free and produces the identical exact answer however many
    /// OS threads actually execute the (fixed) chunk plan concurrently.
    #[test]
    fn sum_overflow_agrees_across_thread_pool_sizes() {
        const ROWS: usize = 2000;
        let max = i128::MAX.to_string();
        let neg_max = format!("-{max}");
        let mut values: Vec<(String, String, &str)> = Vec::with_capacity(ROWS);
        values.push(("r0".to_owned(), max, XINT));
        values.push(("r1".to_owned(), neg_max, XINT));
        for i in 2..ROWS {
            values.push((format!("r{i}"), "1".to_owned(), XINT));
        }
        let borrowed: Vec<(&str, &str, &str)> = values
            .iter()
            .map(|(s, lex, dt)| (s.as_str(), lex.as_str(), *dt))
            .collect();
        let ds = numeric_fold_dataset(&borrowed);
        // 1998 rows of `1` plus the cancelling MAX/-MAX pair.
        let expected = (ROWS - 2).to_string();

        let sequential = {
            let _guard = crate::parallel::force_sequential_operation();
            eval_numeric_fold(&ds, AggregateFunction::Sum)
        };
        assert_eq!(sequential.as_deref(), Some(expected.as_str()));

        for threads in [1_usize, 2, 8, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("building a fixed-size pool");
            let observed = pool.install(|| {
                assert_eq!(rayon::current_num_threads(), threads);
                let _guard = crate::parallel::force_parallel_for_test(true);
                eval_numeric_fold(&ds, AggregateFunction::Sum)
            });
            assert_eq!(
                observed, sequential,
                "SUM disagreed at {threads} worker(s): expected {sequential:?}, got {observed:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // `SUM`/`AVG` over `xsd:duration` — PurRDF extension (`NumericFold::Dur`)
    // -----------------------------------------------------------------------

    const XSD_YEAR_MONTH_DURATION: &str = "http://www.w3.org/2001/XMLSchema#yearMonthDuration";
    const XSD_DAY_TIME_DURATION: &str = "http://www.w3.org/2001/XMLSchema#dayTimeDuration";
    const XSD_DURATION: &str = "http://www.w3.org/2001/XMLSchema#duration";

    /// [`eval_numeric_fold`]'s underlying twin: returns the result's lexical form
    /// AND its datatype IRI, or `None` if unbound — [`eval_numeric_fold`] is a
    /// lexical-only projection of this function. The duration tests below pin
    /// the datatype half as much as the lexical half — the tag-join rule is
    /// exactly what [`sum_over_mixed_subtype_durations_is_the_general_duration`]
    /// exists to catch, and a lexical-only assertion cannot see it.
    fn eval_numeric_fold_term(
        ds: &Arc<RdfDataset>,
        function: AggregateFunction,
    ) -> Option<(String, String)> {
        let mut ctx = EvalCtx::new(ds);
        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/v")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("agg"),
                AggregateExpression::new(
                    function,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("eval");
        let col = seq.schema.index_of(&Variable::new("agg")).unwrap();
        seq.rows[0][col].map(|t| match ctx.scratch.value_of(ds, t) {
            TermValue::Literal {
                lexical_form,
                datatype,
                ..
            } => (lexical_form, datatype),
            o => (format!("{o:?}"), String::new()),
        })
    }

    /// PurRDF extension: `SUM` over a pure-`yearMonthDuration` group is the
    /// group's duration total. SPARQL 1.1 §18.5.1.3 defines `SUM` by repeated
    /// `op:numeric-add`, whose domain is the numeric tower alone — this widens
    /// the aggregate algebra to the duration group durations already form under
    /// `+` (`purrdf_xsd::temporal::add_durations`). `P1M + P2M = P3M`, still
    /// tagged `yearMonthDuration` since every folded value declared it.
    #[test]
    fn sum_over_durations_is_the_group_sum() {
        let ds = numeric_fold_dataset(&[
            ("a", "P1M", XSD_YEAR_MONTH_DURATION),
            ("b", "P2M", XSD_YEAR_MONTH_DURATION),
        ]);
        let (lex, dt) =
            eval_numeric_fold_term(&ds, AggregateFunction::Sum).expect("sum of durations");
        assert_eq!(lex, "P3M");
        assert_eq!(dt, XSD_YEAR_MONTH_DURATION);
    }

    /// A group mixing a numeric value and a duration poisons — `NumericFold`
    /// never coerces across the numeric/duration boundary, regardless of which
    /// kind the fold saw FIRST. The two orders drive different match arms:
    /// numeric-first exercises `NumericFold::Int`'s existing promote-base path
    /// (unedited by this change), duration-first exercises `NumericFold::Dur`'s
    /// new, explicit numeric rejection — so only the pair together proves both
    /// poisoning paths actually work, not just one of them.
    #[test]
    fn sum_over_mixed_numeric_and_duration_is_unbound() {
        let numeric_first =
            numeric_fold_dataset(&[("a", "1", XINT), ("b", "P1D", XSD_DAY_TIME_DURATION)]);
        assert_eq!(
            eval_numeric_fold(&numeric_first, AggregateFunction::Sum),
            None,
            "numeric-then-duration must poison to unbound"
        );

        let duration_first =
            numeric_fold_dataset(&[("a", "P1D", XSD_DAY_TIME_DURATION), ("b", "1", XINT)]);
        assert_eq!(
            eval_numeric_fold(&duration_first, AggregateFunction::Sum),
            None,
            "duration-then-numeric must poison to unbound"
        );
    }

    /// `AVG` over durations rounds the months component toward positive
    /// infinity (`purrdf_xsd::temporal::divide_duration`'s existing
    /// `round_decimal_to_i64` rule — see `NumericFold::Dur`'s `finish_avg` arm)
    /// and divides seconds exactly.
    #[test]
    fn avg_over_durations_rounds_months() {
        let months = numeric_fold_dataset(&[
            ("a", "P1M", XSD_YEAR_MONTH_DURATION),
            ("b", "P2M", XSD_YEAR_MONTH_DURATION),
        ]);
        let (lex, dt) =
            eval_numeric_fold_term(&months, AggregateFunction::Avg).expect("avg of ym durations");
        assert_eq!(lex, "P2M", "1.5 months rounds toward +infinity to 2");
        assert_eq!(dt, XSD_YEAR_MONTH_DURATION);

        let days = numeric_fold_dataset(&[
            ("a", "P1D", XSD_DAY_TIME_DURATION),
            ("b", "P2D", XSD_DAY_TIME_DURATION),
        ]);
        let (lex, dt) =
            eval_numeric_fold_term(&days, AggregateFunction::Avg).expect("avg of dt durations");
        assert_eq!(lex, "P1DT12H");
        assert_eq!(dt, XSD_DAY_TIME_DURATION);
    }

    /// The aggregate-level result-tag rule mirrors the value-space one exactly:
    /// a group mixing `yearMonthDuration` and `dayTimeDuration` operands sums
    /// fine (durations form ONE group under `+`) but the result is stamped the
    /// general `xsd:duration`, never either subtype —
    /// `purrdf_xsd::temporal::add_durations`'s "both declare X, else general"
    /// join rule, reached here through the fold's repeated calls rather than a
    /// single one.
    #[test]
    fn sum_over_mixed_subtype_durations_is_the_general_duration() {
        let ds = numeric_fold_dataset(&[
            ("a", "P1M", XSD_YEAR_MONTH_DURATION),
            ("b", "PT1H", XSD_DAY_TIME_DURATION),
        ]);
        let (lex, dt) = eval_numeric_fold_term(&ds, AggregateFunction::Sum)
            .expect("sum of mixed-subtype durations");
        assert_eq!(lex, "P1MT1H");
        assert_eq!(dt, XSD_DURATION);
    }

    /// The overflow suite's thread-pool-size pattern
    /// ([`sum_overflow_agrees_across_thread_pool_sizes`]), reproduced for
    /// durations: a group large enough to force
    /// `crate::parallel::par_chunk_reduce_init` into MANY chunks, summed
    /// sequentially and under forced-parallel pools of 1/2/8/32 workers.
    /// `NumericFold::Dur`'s `combine_owned` arm is `Commutative` (duration `+`
    /// is associative/commutative — see that method's doc), so every pool size
    /// must agree with the sequential answer byte for byte.
    #[test]
    fn sum_over_durations_agrees_across_thread_pool_sizes() {
        const ROWS: usize = 2000;
        let mut values: Vec<(String, &str, &str)> = Vec::with_capacity(ROWS);
        for i in 0..ROWS {
            values.push((format!("r{i}"), "P1D", XSD_DAY_TIME_DURATION));
        }
        let borrowed: Vec<(&str, &str, &str)> = values
            .iter()
            .map(|(s, lex, dt)| (s.as_str(), *lex, *dt))
            .collect();
        let ds = numeric_fold_dataset(&borrowed);

        let sequential = {
            let _guard = crate::parallel::force_sequential_operation();
            eval_numeric_fold(&ds, AggregateFunction::Sum)
        };
        assert_eq!(sequential.as_deref(), Some("P2000D"));

        for threads in [1_usize, 2, 8, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("building a fixed-size pool");
            let observed = pool.install(|| {
                assert_eq!(rayon::current_num_threads(), threads);
                let _guard = crate::parallel::force_parallel_for_test(true);
                eval_numeric_fold(&ds, AggregateFunction::Sum)
            });
            assert_eq!(
                observed, sequential,
                "duration SUM disagreed at {threads} worker(s): expected {sequential:?}, got {observed:?}"
            );
        }
    }

    /// The exact defect this crate's `NumericFold::Dur` decomposition fixes:
    /// `{P1Y^^yearMonthDuration, -P1D^^dayTimeDuration, P1D^^dayTimeDuration}`'s
    /// true total, `(12 months, 0 s)`, is representable ("P1Y") — but a fold
    /// that validates sign coherence at every intermediate `step`/`combine`
    /// (rather than once, on the finished total) sees a mixed-sign
    /// INTERMEDIATE the moment `P1Y` and `-P1D` combine, regardless of
    /// whether that combination happens via a sequential `step` or via
    /// `combine_owned` merging a chunk that has already folded `-P1D` and
    /// `P1D` together. Forcing every row into its own chunk
    /// (`force_chunk_size_for_test(1)`) drives `combine_owned` through every
    /// possible adjacent pairing of this three-row group, so this is a
    /// genuine stress of the fix, not merely a single lucky chunk boundary.
    #[test]
    fn sum_over_mixed_sign_durations_is_order_independent() {
        let ds = numeric_fold_dataset(&[
            ("a", "P1Y", XSD_YEAR_MONTH_DURATION),
            ("b", "-P1D", XSD_DAY_TIME_DURATION),
            ("c", "P1D", XSD_DAY_TIME_DURATION),
        ]);

        let sequential = {
            let _guard = crate::parallel::force_sequential_operation();
            eval_numeric_fold_term(&ds, AggregateFunction::Sum)
        };
        assert_eq!(
            sequential,
            Some(("P1Y".to_owned(), XSD_DURATION.to_owned())),
            "the group's true total is representable regardless of fold order"
        );

        let chunked = {
            let _parallel_guard = crate::parallel::force_parallel_for_test(true);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(1);
            eval_numeric_fold_term(&ds, AggregateFunction::Sum)
        };
        assert_eq!(
            sequential, chunked,
            "duration SUM over a mixed-sign group must not depend on chunk boundaries"
        );
    }

    /// [`sum_over_mixed_sign_durations_is_order_independent`]'s three-row
    /// fixture, reproduced at a scale that crosses
    /// `crate::parallel::PARALLEL_MIN_ROWS` (1024) so the REAL
    /// `crate::parallel::par_chunk_reduce_init` combine path — not merely the
    /// sequential fallback — is exercised: 1 `P1Y`, then 511 `(P1D, -P1D)`
    /// pairs (each returns the running seconds to exactly zero before the
    /// next pair starts, so this whole stretch never goes mixed-sign no
    /// matter how it is chunked), then one final REVERSED pair
    /// `(-P1D, P1D)` — 1025 rows total.
    ///
    /// A forced chunk size of 1023 splits this at exactly the reversed
    /// pair's boundary: chunk 0 is the leading 1023 rows (`P1Y` plus all 511
    /// ordinary pairs), which stays representable the entire time it folds;
    /// chunk 1 is the trailing `(-P1D, P1D)` pair alone, folded from a FRESH
    /// accumulator that never sees `P1Y`'s months at all — its own transient
    /// negative (`-P1D` applied while `months` is still zero) never
    /// interacts with a nonzero months component, so a pre-fix, per-step
    /// sign-coherence check never fires there either. Sequentially, by
    /// contrast, that same `-P1D` lands on the already-`P1Y`-bearing running
    /// total inherited from the preceding 1023 rows — the exact
    /// chunk-boundary-dependent poisoning [`NumericFold::Dur`]'s own doc
    /// describes.
    #[test]
    fn sum_over_mixed_sign_durations_agrees_across_a_forced_chunk_boundary() {
        const PAIRS: usize = 511;
        let mut values: Vec<(String, &str, &str)> = Vec::with_capacity(2 + 2 * PAIRS);
        values.push(("y".to_owned(), "P1Y", XSD_YEAR_MONTH_DURATION));
        for i in 0..PAIRS {
            values.push((format!("p{i}"), "P1D", XSD_DAY_TIME_DURATION));
            values.push((format!("n{i}"), "-P1D", XSD_DAY_TIME_DURATION));
        }
        values.push(("nf".to_owned(), "-P1D", XSD_DAY_TIME_DURATION));
        values.push(("pf".to_owned(), "P1D", XSD_DAY_TIME_DURATION));
        assert_eq!(
            values.len(),
            1025,
            "fixture must cross PARALLEL_MIN_ROWS (1024) for the real chunking path"
        );

        let borrowed: Vec<(&str, &str, &str)> = values
            .iter()
            .map(|(s, lex, dt)| (s.as_str(), *lex, *dt))
            .collect();
        let ds = numeric_fold_dataset(&borrowed);

        let sequential = {
            let _guard = crate::parallel::force_sequential_operation();
            eval_numeric_fold_term(&ds, AggregateFunction::Sum)
        };
        let chunked = {
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(1023);
            eval_numeric_fold_term(&ds, AggregateFunction::Sum)
        };
        assert_eq!(
            sequential, chunked,
            "duration SUM must not depend on where a chunk boundary falls"
        );
        assert_eq!(
            sequential,
            Some(("P1Y".to_owned(), XSD_DURATION.to_owned())),
            "the true total is representable regardless of fold order"
        );
    }

    /// The other side of the same coin: a group whose true total genuinely
    /// IS mixed-sign — `{P1Y^^yearMonthDuration, -P1D^^dayTimeDuration}` sums
    /// to `(12 months, -86400 s)`, which XSD 1.1 Part 2 §3.3.6 places outside
    /// the duration value space — must poison to unbound DETERMINISTICALLY,
    /// never depending on whether the fold happened to run sequentially or
    /// under a forced-parallel path.
    #[test]
    fn sum_whose_final_value_is_mixed_sign_is_unbound() {
        let ds = numeric_fold_dataset(&[
            ("a", "P1Y", XSD_YEAR_MONTH_DURATION),
            ("b", "-P1D", XSD_DAY_TIME_DURATION),
        ]);

        let sequential = {
            let _guard = crate::parallel::force_sequential_operation();
            eval_numeric_fold(&ds, AggregateFunction::Sum)
        };
        assert_eq!(
            sequential, None,
            "the group's true total, (12 months, -86400 s), is mixed-sign and unrepresentable"
        );

        let parallel = {
            let _guard = crate::parallel::force_parallel_for_test(true);
            eval_numeric_fold(&ds, AggregateFunction::Sum)
        };
        assert_eq!(
            sequential, parallel,
            "an unrepresentable total must poison to unbound identically in both paths"
        );
    }

    /// `AVG`'s raw-component mean is validated independently of `SUM`'s raw
    /// total (see `NumericFold::finish_avg`'s `Dur` arm doc) — proven here in
    /// both directions: a group whose mean IS representable answers that
    /// mean (reusing [`sum_over_mixed_sign_durations_is_order_independent`]'s
    /// fixture, whose total `(12, 0)` divided by its count of 3 is `(4, 0)` =
    /// `"P4M"`), and a group whose mean is NOT representable poisons to
    /// unbound, deterministically, in both the sequential and the
    /// forced-parallel path.
    #[test]
    fn avg_of_mixed_sign_durations() {
        let representable_mean = numeric_fold_dataset(&[
            ("a", "P1Y", XSD_YEAR_MONTH_DURATION),
            ("b", "-P1D", XSD_DAY_TIME_DURATION),
            ("c", "P1D", XSD_DAY_TIME_DURATION),
        ]);
        let (lex, dt) = eval_numeric_fold_term(&representable_mean, AggregateFunction::Avg)
            .expect("avg of a mixed-sign group whose mean is representable");
        assert_eq!(lex, "P4M");
        assert_eq!(dt, XSD_DURATION);

        // `(24 months, 0 s) + (0 months, -86400 s)`, divided by a count of
        // 2, means `(12 months, -43200 s)` — itself mixed-sign, even though
        // computing it never routes through `finish_sum` at all.
        let unrepresentable_mean = numeric_fold_dataset(&[
            ("a", "P2Y", XSD_YEAR_MONTH_DURATION),
            ("b", "-P1D", XSD_DAY_TIME_DURATION),
        ]);

        let sequential = {
            let _guard = crate::parallel::force_sequential_operation();
            eval_numeric_fold(&unrepresentable_mean, AggregateFunction::Avg)
        };
        assert_eq!(
            sequential, None,
            "the group's mean, (12 months, -43200 s), is mixed-sign and unrepresentable"
        );

        let parallel = {
            let _guard = crate::parallel::force_parallel_for_test(true);
            eval_numeric_fold(&unrepresentable_mean, AggregateFunction::Avg)
        };
        assert_eq!(
            sequential, parallel,
            "an unrepresentable mean must poison to unbound identically in both paths"
        );
    }

    #[test]
    fn values_seeds_solutions() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        use purrdf_sparql_algebra::GroundTerm;
        // VALUES ?x { :a UNDEF }
        let vars = vec![Variable::new("x")];
        let bindings = vec![
            vec![Some(GroundTerm::NamedNode(NamedNode::new_unchecked(
                "http://ex/a",
            )))],
            vec![None],
        ];
        let seq = eval_values(&vars, &bindings, &mut ctx).expect("values");
        assert_eq!(seq.len(), 2);
        let x = seq.schema.index_of(&Variable::new("x")).unwrap();
        assert!(seq.rows[0][x].is_some());
        assert!(seq.rows[1][x].is_none()); // UNDEF.
    }

    /// One `?v` literal per `(lexical, datatype-local-name)`, so a test can drive
    /// a chosen literal set through the real `ORDER BY` operator.
    fn typed_values(values: &[(&str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/v");
        for (i, (lexical, local)) in values.iter().enumerate() {
            let s = b.intern_iri(&format!("http://ex/s{i}"));
            let o = b.intern_literal(RdfLiteral {
                lexical_form: (*lexical).to_owned(),
                datatype: Some(format!("http://www.w3.org/2001/XMLSchema#{local}")),
                language: None,
                direction: None,
            });
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn value_bgp() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/v")),
                object: TermPattern::Variable(Variable::new("v")),
            }],
        }
    }

    /// THE CYCLE, driven through the production `ORDER BY` path. Under the naive
    /// reading of §15.1 — "compare by value, else by a deterministic syntactic
    /// key" — these three literals cycle: `"9"^^xsd:double` < `"P1D"^^xsd:duration`
    /// (no value order, so datatype `double` < `duration`) < `"8"^^xsd:float`
    /// (`duration` < `float`) < `"9"^^xsd:double` (a value order: 8 < 9).
    /// `slice::sort_by` is entitled to PANIC on a comparator like that, so the
    /// cycle must not exist at all: [`ValueClass`] ranks ahead of the syntactic
    /// fallback, which puts both numerics (one class) before the duration
    /// (another) and orders the two of them 8 < 9 in value space.
    #[test]
    fn order_by_has_no_value_versus_syntactic_cycle() {
        let ds = typed_values(&[("9", "double"), ("P1D", "duration"), ("8", "float")]);
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval_order_by(
            &value_bgp(),
            &[OrderExpression::Asc(Expression::Variable(Variable::new(
                "v",
            )))],
            &mut ctx,
        )
        .expect("order");
        assert_eq!(ints(&ds, &seq, "v"), vec!["8", "9", "P1D"]);
    }

    /// [`project`] + [`total_order`] must be a genuine TOTAL ORDER over every term
    /// kind — antisymmetric AND transitive — since that is exactly the contract
    /// `slice::sort_by` (hence `ORDER BY`, `MIN`/`MAX` and the statistical
    /// aggregates) may panic over when it is broken. The sample mixes every shape
    /// that used to cycle: incomparable value spaces beside comparable ones, an
    /// ill-typed literal, `NaN`, a timezoned against an untimezoned `dateTime`,
    /// the two duration subtypes against the general one, the two binary value
    /// spaces, unbound/blank/IRI, and triple terms whose components are
    /// themselves such literals (the recursive arm, which inherits the relation
    /// rather than restating it).
    #[test]
    fn the_sparql_ordering_is_a_total_order_over_every_term_kind() {
        let xsd = |lexical: &str, local: &str| {
            TermValue::typed_literal(lexical, format!("http://www.w3.org/2001/XMLSchema#{local}"))
        };
        let triple = |s: TermValue, p: TermValue, o: TermValue| TermValue::Triple {
            s: Box::new(s),
            p: Box::new(p),
            o: Box::new(o),
        };
        let samples: Vec<Option<TermValue>> = vec![
            None,
            Some(TermValue::blank("b0")),
            Some(TermValue::blank("b1")),
            Some(TermValue::iri("http://ex/a")),
            Some(TermValue::iri("http://ex/z")),
            Some(xsd("9", "double")),
            Some(xsd("8", "float")),
            Some(xsd("9", "integer")),
            Some(xsd("30", "integer")),
            Some(xsd("NaN", "double")),
            // The promotion cycle, in the shape a query can supply: a decimal with
            // nineteen significant digits beside the IEEE value of the same
            // magnitude, and the integer that value rounds to. Under §17.3's
            // promotion the decimal is GREATER than the integer (compared exactly)
            // and EQUAL to both the double and the float (compared through IEEE),
            // which is a cycle a Rust sort may abort on. See `ValueClass::Numeric`.
            Some(xsd("1.000000000000000001", "decimal")),
            Some(xsd("1", "integer")),
            Some(xsd("1.0E0", "double")),
            Some(xsd("1.0E0", "float")),
            Some(xsd("-1.000000000000000001", "decimal")),
            Some(xsd("-1", "integer")),
            Some(xsd("-1.0E0", "double")),
            // The same failure one type up: 2^53 + 1 is an ordinary integer and an
            // unrepresentable double, so the promotion rounds it onto 2^53.
            Some(xsd("9007199254740993", "integer")),
            Some(xsd("9.007199254740992E15", "double")),
            Some(xsd("abc", "integer")),
            Some(xsd("P1D", "duration")),
            Some(xsd("P1D", "dayTimeDuration")),
            Some(xsd("P1Y", "yearMonthDuration")),
            Some(xsd("2000-01-01T00:00:00", "dateTime")),
            Some(xsd("2000-01-01T00:00:00Z", "dateTime")),
            Some(xsd("2000-01-01", "date")),
            Some(xsd("0F", "hexBinary")),
            Some(xsd("Dw==", "base64Binary")),
            Some(xsd("true", "boolean")),
            Some(TermValue::simple_literal("abc")),
            Some(TermValue::lang_literal("abc", "en")),
            Some(triple(
                TermValue::iri("http://ex/a"),
                TermValue::iri("http://ex/p"),
                xsd("9", "integer"),
            )),
            Some(triple(
                TermValue::iri("http://ex/a"),
                TermValue::iri("http://ex/p"),
                xsd("30", "integer"),
            )),
            Some(triple(
                triple(
                    TermValue::iri("http://ex/a"),
                    TermValue::iri("http://ex/p"),
                    xsd("9", "integer"),
                ),
                TermValue::iri("http://ex/q"),
                xsd("P1D", "duration"),
            )),
            // The composite category, including the two spellings of one value
            // (which must tie), a nested composite, and two lexical forms that do
            // NOT parse (which must fall back to the opaque-literal block instead
            // of raising or of ranking with the composites).
            Some(TermValue::typed_literal("[]", purrdf_cdt::CDT_LIST)),
            Some(TermValue::typed_literal("[  ]", purrdf_cdt::CDT_LIST)),
            Some(TermValue::typed_literal("[1]", purrdf_cdt::CDT_LIST)),
            Some(TermValue::typed_literal("[ 1, null]", purrdf_cdt::CDT_LIST)),
            Some(TermValue::typed_literal(
                "[[1],{ 1:2 }]",
                purrdf_cdt::CDT_LIST,
            )),
            Some(TermValue::typed_literal("{}", purrdf_cdt::CDT_MAP)),
            Some(TermValue::typed_literal(
                "{ 3:1, 1:2 }",
                purrdf_cdt::CDT_MAP,
            )),
            Some(TermValue::typed_literal("[1,", purrdf_cdt::CDT_LIST)),
            Some(TermValue::typed_literal("nonsense", purrdf_cdt::CDT_MAP)),
        ];
        let keys: Vec<_> = samples.iter().map(|v| project(v.as_ref())).collect();

        for (i, a) in keys.iter().enumerate() {
            for (j, b) in keys.iter().enumerate() {
                assert_eq!(
                    total_order(a, b),
                    total_order(b, a).reverse(),
                    "not antisymmetric at ({i}, {j}): {:?} vs {:?}",
                    samples[i],
                    samples[j]
                );
            }
        }
        for (i, a) in keys.iter().enumerate() {
            for (j, b) in keys.iter().enumerate() {
                if total_order(a, b) == Ordering::Greater {
                    continue;
                }
                for (k, c) in keys.iter().enumerate() {
                    if total_order(b, c) == Ordering::Greater {
                        continue;
                    }
                    assert_ne!(
                        total_order(a, c),
                        Ordering::Greater,
                        "not transitive at ({i}, {j}, {k}): {:?} <= {:?} <= {:?}",
                        samples[i],
                        samples[j],
                        samples[k]
                    );
                }
            }
        }

        // And the whole sample sorts through the production comparator without
        // tripping `sort_by`'s total-order check, to the SAME sequence every run.
        let sorted_once = sorted_sample_lexicals(&samples);
        assert_eq!(sorted_once, sorted_sample_lexicals(&samples));
        assert_eq!(sorted_once.len(), samples.len());
    }

    /// Sort a projected sample with the production comparator and render it, so a
    /// caller can compare two runs for determinism.
    fn sorted_sample_lexicals(samples: &[Option<TermValue>]) -> Vec<String> {
        let keys: Vec<_> = samples.iter().map(|v| project(v.as_ref())).collect();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        order.sort_by(|a, b| total_order(&keys[*a], &keys[*b]));
        order
            .into_iter()
            .map(|i| format!("{:?}", samples[i]))
            .collect()
    }

    /// Determinism smoke test: `GROUP BY ?cat` with `COUNT(*)`/`AVG(?val)`/
    /// `MAX(?val)` over 220 groups (the `e_group_aggregate` bench shape) evaluated
    /// once with the parallel per-group path FORCED and once with the sequential
    /// path FORCED must produce byte-identical rows — group ORDER (first-seen) is
    /// always computed sequentially, but the per-group `AVG`/`MAX` compute (which
    /// mints fresh `Computed` terms that must escape a forked child via
    /// [`crate::parallel::portable_row`]/[`crate::parallel::reintern_portable_row`])
    /// runs under fork-join when FORCED.
    #[test]
    fn group_aggregate_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;

        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const GROUPS: i64 = 220;
        const ROWS: i64 = 260;

        let mut b = RdfDatasetBuilder::new();
        let cat_pred = b.intern_iri("http://ex/cat");
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let cat = b.intern_literal(RdfLiteral {
                lexical_form: format!("cat{}", i % GROUPS),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            let val = b.intern_literal(RdfLiteral {
                lexical_form: i.to_string(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, cat_pred, cat, None);
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        let inner = GraphPattern::Join {
            left: Box::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                        "http://ex/cat",
                    )),
                    object: TermPattern::Variable(Variable::new("cat")),
                }],
            }),
            right: Box::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                        "http://ex/val",
                    )),
                    object: TermPattern::Variable(Variable::new("val")),
                }],
            }),
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![Variable::new("cat")],
            aggregates: vec![
                (
                    Variable::new("cnt"),
                    AggregateExpression::new(
                        AggregateFunction::Count,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("avg"),
                    AggregateExpression::new(
                        AggregateFunction::Avg,
                        vec![Expression::Variable(Variable::new("val"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("mx"),
                    AggregateExpression::new(
                        AggregateFunction::Max,
                        vec![Expression::Variable(Variable::new("val"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
            ],
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&group, &mut ctx).expect("eval");
            (seq.schema.vars().to_vec(), seq.rows)
        };

        let (schema_par, rows_par) = run(true);
        let (schema_seq, rows_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential per-group aggregate paths must produce byte-identical rows"
        );
        assert_eq!(rows_seq.len() as i64, GROUPS);
    }

    /// Within-group chunked partial aggregation (this increment): a SINGLE huge
    /// group — no `GROUP BY` at all, so `eval_group`'s single-implicit-group rule
    /// applies — whose row count crosses `PARALLEL_MIN_ROWS`, aggregated by
    /// `GROUP_CONCAT` (`OrderDependent`: string concatenation, the case an
    /// across-groups-only fork could never parallelize since there is only ONE
    /// group here) and `MAX`. Forced-parallel (with a small forced chunk size, so
    /// the fold actually spans MANY chunks and `combine` runs many times) and
    /// forced-sequential must agree byte-for-byte — proving
    /// `crate::parallel::par_chunk_reduce_init`'s chunk-order `combine` contract
    /// holds for a non-commutative fold, not just the commutative ones a weaker
    /// test could pass by accident.
    #[test]
    fn within_group_chunked_fold_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const ROWS: i64 = 3000;

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: i.to_string(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/val")),
                object: TermPattern::Variable(Variable::new("val")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![
                (
                    Variable::new("g"),
                    AggregateExpression::new(
                        AggregateFunction::GroupConcat,
                        vec![Expression::Variable(Variable::new("val"))],
                        vec![(
                            "separator".to_owned(),
                            purrdf_sparql_algebra::Literal::new_simple(","),
                        )],
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("mx"),
                    AggregateExpression::new(
                        AggregateFunction::Max,
                        vec![Expression::Variable(Variable::new("val"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
            ],
        };

        let run = |forced: bool, chunk: usize| {
            let _parallel_guard = crate::parallel::force_parallel_for_test(forced);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(chunk);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&group, &mut ctx).expect("eval");
            let g = agg_lex(&ds, &ctx, &seq, "g");
            let mx = agg_lex(&ds, &ctx, &seq, "mx");
            (seq.schema.vars().to_vec(), g, mx)
        };

        let sequential = run(false, 64);
        let parallel = run(true, 37); // a deliberately ragged chunk size

        assert_eq!(
            sequential, parallel,
            "within-group chunked fold must be byte-identical"
        );
        // And pin the actual value: input row order is s0..s2999, so GROUP_CONCAT
        // is exactly "0,1,2,...,2999", never re-ordered by chunking.
        let expected = (0..ROWS)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(sequential.1, expected);
        assert_eq!(sequential.2, (ROWS - 1).to_string());
    }

    /// `DISTINCT` correctness under within-group chunking: a duplicate value whose
    /// two occurrences straddle where chunk boundaries fall must still resolve to
    /// its INPUT-ORDER-FIRST occurrence — proven by [`crate::modifier`]'s "dedup
    /// pre-chunking" design (`eval_aggregate`'s phase 1 dedups sequentially,
    /// BEFORE phase 2 ever chunks anything), so no chunk can ever see a value its
    /// group already resolved as a duplicate. A small forced chunk size (so the
    /// large row count spans many chunks) plus forced-parallel vs forced-
    /// sequential agreement is the proof: if chunking could reintroduce a
    /// duplicate, this is exactly the shape that would expose it.
    #[test]
    fn within_group_distinct_dedup_survives_chunking_keeping_input_order_first_occurrence() {
        use purrdf_core::RdfLiteral;
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const ROWS: i64 = 3000;

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            // Row 5 and row 2900 (far apart — different chunks under any
            // reasonable chunk size) share the SAME value "5"; every other row
            // is unique. The DISTINCT-kept representative must be row 5's
            // occurrence (position 5 in row order), not row 2900's.
            let lex = if i == 2900 {
                "5".to_owned()
            } else {
                i.to_string()
            };
            let val = b.intern_literal(RdfLiteral {
                lexical_form: lex,
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/val")),
                object: TermPattern::Variable(Variable::new("val")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::GroupConcat,
                    vec![Expression::Variable(Variable::new("val"))],
                    vec![(
                        "separator".to_owned(),
                        purrdf_sparql_algebra::Literal::new_simple(","),
                    )],
                    Vec::new(),
                    true,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };

        let run = |forced: bool, chunk: usize| {
            let _parallel_guard = crate::parallel::force_parallel_for_test(forced);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(chunk);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&group, &mut ctx).expect("eval");
            agg_lex(&ds, &ctx, &seq, "g")
        };

        let sequential = run(false, 64);
        let parallel = run(true, 41);
        assert_eq!(sequential, parallel);

        // Expected: every i in 0..3000 in order EXCEPT row 2900 — its duplicate
        // "5" is dropped, keeping row 5's earlier occurrence exactly where it
        // already was.
        let expected: String = (0..ROWS)
            .filter(|&i| i != 2900)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(sequential, expected);
    }

    /// `MAX(?n)` over {30, 17, 30} → 30 — standalone pin, alongside
    /// [`group_min`], for the built-in accumulator that shares
    /// [`fold_extreme`] with `MIN`.
    #[test]
    fn max_integers() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let group = GraphPattern::Group {
            inner: Box::new(age_bgp()),
            variables: vec![],
            aggregates: vec![(
                Variable::new("m"),
                AggregateExpression::new(
                    AggregateFunction::Max,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("max");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "m"), "30");
    }

    /// `MAX` over an empty group → unbound (`error` in the spec's `MaxList`
    /// reading — see the module-level "Aggregate semantics" docs).
    #[test]
    fn max_empty_group_is_unbound() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let empty_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/none")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(empty_bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("m"),
                AggregateExpression::new(
                    AggregateFunction::Max,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("max empty");
        assert_eq!(seq.len(), 1);
        let col = seq.schema.index_of(&Variable::new("m")).unwrap();
        assert!(
            seq.rows[0][col].is_none(),
            "MAX of an empty group must be unbound"
        );
    }

    /// `SAMPLE(?n)` picks the FIRST row-order value — `ages()`'s BGP scan
    /// visits `:a`(30), `:b`(17), `:c`(30) in insertion order, so `SAMPLE`
    /// must answer `:a`'s value, `30`, never `17` or an arbitrary pick.
    #[test]
    fn sample_returns_first_row_order_value() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let group = GraphPattern::Group {
            inner: Box::new(age_bgp()),
            variables: vec![],
            aggregates: vec![(
                Variable::new("smp"),
                AggregateExpression::new(
                    AggregateFunction::Sample,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("sample");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "smp"), "30");
    }

    /// `SAMPLE` over an empty group → unbound.
    #[test]
    fn sample_empty_group_is_unbound() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let empty_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/none")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(empty_bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("smp"),
                AggregateExpression::new(
                    AggregateFunction::Sample,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("sample empty");
        assert_eq!(seq.len(), 1);
        let col = seq.schema.index_of(&Variable::new("smp")).unwrap();
        assert!(
            seq.rows[0][col].is_none(),
            "SAMPLE of an empty group must be unbound"
        );
    }

    /// `GROUP_CONCAT(?n)` with NO `SEPARATOR` clause defaults to a single
    /// space per §18.6.1.7 — `scalarvals` empty, exactly the shape a bare
    /// `GROUP_CONCAT(?x)` (no `; SEPARATOR="…"`) parses to.
    #[test]
    fn group_concat_without_separator_uses_default_space() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let group = GraphPattern::Group {
            inner: Box::new(age_bgp()),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::GroupConcat,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group_concat");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "g"), "30 17 30");
    }

    /// `GROUP_CONCAT` over an empty group → `""` (SPARQL's explicit
    /// empty-group answer, not unbound).
    #[test]
    fn group_concat_empty_group_is_empty_string() {
        let ds = ages();
        let mut ctx = EvalCtx::new(&ds);
        let empty_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/none")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(empty_bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::GroupConcat,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group_concat empty");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "g"), "");
    }

    /// `GROUP_CONCAT` poisons — answers unbound, not a hard error — when a
    /// folded value has no lexical form: a blank node's `STR()` is a SPARQL
    /// type error (§17.4.2.2), which [`GroupConcatAccumulator::step`] turns
    /// into the same "poisoned" unbound answer [`NumericFold`] gives `SUM`/
    /// `AVG` over a non-numeric value — this crate's "groups with error
    /// values" reading (see the module-level "Aggregate semantics" docs).
    #[test]
    fn group_concat_poisons_on_a_blank_node_value() {
        use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/v");
        let s1 = b.intern_iri("http://ex/s1");
        let v1 = b.intern_literal(RdfLiteral::simple("first"));
        b.push_quad(s1, p, v1, None);
        let s2 = b.intern_iri("http://ex/s2");
        let blank = b.intern_blank("b0", purrdf_core::BlankScope::DEFAULT);
        b.push_quad(s2, p, blank, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);
        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/v")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(bgp),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::GroupConcat,
                    vec![Expression::Variable(Variable::new("n"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let seq = eval(&group, &mut ctx).expect("group_concat blank");
        assert_eq!(seq.len(), 1);
        let col = seq.schema.index_of(&Variable::new("g")).unwrap();
        assert!(
            seq.rows[0][col].is_none(),
            "GROUP_CONCAT over a blank-node value must poison to unbound"
        );
    }

    /// A row whose aggregate ARGUMENT is honestly unbound (an `OPTIONAL` that
    /// did not match) is skipped by every unary built-in — never counted,
    /// never folded, never an error — while `COUNT(*)` (which folds the whole
    /// SOLUTION, not this one expression) still counts that row. Dataset:
    /// `:a`/`:b`/`:c` each `rdf:type :Thing`; only `:a`(30)/`:b`(17) also carry
    /// `:n`; `:c` has none, so `OPTIONAL { ?s :n ?n }` leaves `?n` unbound on
    /// `:c`'s row.
    #[test]
    fn aggregates_skip_unbound_argument_rows_but_count_star_does_not() {
        use purrdf_core::RdfLiteral;
        let mut b = RdfDatasetBuilder::new();
        let ty = b.intern_iri("http://ex/type");
        let thing = b.intern_iri("http://ex/Thing");
        let n = b.intern_iri("http://ex/n");
        for s in ["a", "b", "c"] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            b.push_quad(subj, ty, thing, None);
        }
        for (s, lex) in [("a", "30"), ("b", "17")] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: lex.to_owned(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, n, val, None);
        }
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);

        let required = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/type")),
                object: TermPattern::NamedNode(NamedNode::new_unchecked("http://ex/Thing")),
            }],
        };
        let optional = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/n")),
                object: TermPattern::Variable(Variable::new("n")),
            }],
        };
        let inner = GraphPattern::LeftJoin {
            left: Box::new(required),
            right: Box::new(optional),
            expression: None,
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![
                (
                    Variable::new("cnt_star"),
                    AggregateExpression::new(
                        AggregateFunction::Count,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("cnt_n"),
                    AggregateExpression::new(
                        AggregateFunction::Count,
                        vec![Expression::Variable(Variable::new("n"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("s"),
                    AggregateExpression::new(
                        AggregateFunction::Sum,
                        vec![Expression::Variable(Variable::new("n"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("mn"),
                    AggregateExpression::new(
                        AggregateFunction::Min,
                        vec![Expression::Variable(Variable::new("n"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
                (
                    Variable::new("mx"),
                    AggregateExpression::new(
                        AggregateFunction::Max,
                        vec![Expression::Variable(Variable::new("n"))],
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("fixture: valid AggregateExpression"),
                ),
            ],
        };
        let seq = eval(&group, &mut ctx).expect("group over optional");
        assert_eq!(seq.len(), 1);
        assert_eq!(
            agg_lex(&ds, &ctx, &seq, "cnt_star"),
            "3",
            "COUNT(*) counts every solution, including :c's unbound-?n row"
        );
        assert_eq!(
            agg_lex(&ds, &ctx, &seq, "cnt_n"),
            "2",
            "COUNT(?n) skips the unbound row"
        );
        assert_eq!(
            agg_lex(&ds, &ctx, &seq, "s"),
            "47",
            "SUM(?n) skips the unbound row (30 + 17)"
        );
        assert_eq!(agg_lex(&ds, &ctx, &seq, "mn"), "17");
        assert_eq!(agg_lex(&ds, &ctx, &seq, "mx"), "30");
    }

    /// [`sum_overflow_agrees_across_thread_pool_sizes`]'s multi-thread-pool
    /// harness (1/2/8/32 workers), reused for every OTHER unary built-in
    /// (`COUNT`, `AVG`, `MIN`, `MAX`, `SAMPLE`, `GROUP_CONCAT`) — not just
    /// `SUM` — over a group large enough to cross `PARALLEL_MIN_ROWS` so each
    /// pool size really does split the within-group fold into a different
    /// chunk count. Forced-sequential is the oracle; every pool size must
    /// reproduce it byte for byte.
    #[test]
    fn every_builtin_agrees_across_thread_pool_sizes() {
        use purrdf_core::RdfLiteral;
        const ROWS: i64 = 2000;
        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: i.to_string(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        for function in [
            AggregateFunction::Count,
            AggregateFunction::Avg,
            AggregateFunction::Min,
            AggregateFunction::Max,
            AggregateFunction::Sample,
            AggregateFunction::GroupConcat,
        ] {
            let run = || eval_numeric_fold(&ds, function.clone());
            let sequential = {
                let _guard = crate::parallel::force_sequential_operation();
                run()
            };
            for threads in [1_usize, 2, 8, 32] {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .expect("building a fixed-size pool");
                let observed = pool.install(|| {
                    assert_eq!(rayon::current_num_threads(), threads);
                    let _guard = crate::parallel::force_parallel_for_test(true);
                    run()
                });
                assert_eq!(
                    observed, sequential,
                    "{function:?} disagreed at {threads} worker(s): expected {sequential:?}, \
                     got {observed:?}"
                );
            }
        }
    }

    /// A minimal `OrderDependent` custom aggregate: collects each row's single
    /// argument's lexical form into an ordered `Vec<String>`, `combine` appending
    /// the LATER partial's list after the earlier one's (never sorted, never
    /// deduped) — a "list collector", exactly the shape
    /// `crate::agg_fn`'s module docs use as the running example of a fold whose
    /// answer depends on row order. `finish` joins with `;` so the test can
    /// compare a single string.
    struct ListCollector {
        items: Vec<String>,
    }

    impl crate::agg_fn::AggregateAccumulator for ListCollector {
        fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
            if let Some(TermValue::Literal { lexical_form, .. }) = args.first() {
                self.items.push(lexical_form.clone());
            }
            Ok(())
        }
        fn combine(
            &mut self,
            other: Box<dyn crate::agg_fn::AggregateAccumulator>,
        ) -> Result<(), EvalError> {
            // An ordered list join IS its own sufficient merge state (see
            // `crate::agg_fn`'s "Merging structural state" module docs), so —
            // exactly like `agg_fn`'s own `SumAccumulator` test fixture — this
            // finishes the other partial through the same public surface a
            // caller has and re-derives its item list from that, rather than
            // reaching for `into_any`'s downcast escape hatch.
            if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
                && !lexical_form.is_empty()
            {
                self.items
                    .extend(lexical_form.split(';').map(str::to_owned));
            }
            Ok(())
        }
        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            Ok(Some(TermValue::typed_literal(
                self.items.join(";"),
                "http://www.w3.org/2001/XMLSchema#string",
            )))
        }
    }

    struct ListCollectorAggregate;

    impl crate::agg_fn::CustomAggregate for ListCollectorAggregate {
        fn arity(&self) -> crate::user_fn::Arity {
            crate::user_fn::Arity::Exact(1)
        }
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }
        fn algebraic_class(&self) -> crate::agg_fn::AlgebraicClass {
            crate::agg_fn::AlgebraicClass::OrderDependent
        }
        fn state_bound(&self) -> u64 {
            256
        }
        fn init(
            &self,
            _scalarvals: &[(String, TermValue)],
        ) -> Box<dyn crate::agg_fn::AggregateAccumulator> {
            Box::new(ListCollector { items: Vec::new() })
        }
    }

    const LIST_COLLECTOR_IRI: &str = "http://example.org/agg#listCollector";

    /// A large single-group `OrderDependent` CUSTOM aggregate, folded via
    /// `eval_custom_aggregate`'s within-group chunked path
    /// ([`crate::agg_fn::AggregateAccumulator::combine`], driven by
    /// `crate::parallel::par_chunk_reduce_init`), must agree byte-for-byte
    /// between forced-parallel and forced-sequential — the custom-aggregate
    /// counterpart of [`within_group_chunked_fold_forced_parallel_and_sequential_agree`]
    /// above, proving `combine_contained`'s chunk-order contract holds through
    /// the FULL host-extension seam (panic containment included), not just the
    /// crate's own built-in fold.
    #[test]
    fn within_group_custom_aggregate_chunked_fold_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        const ROWS: i64 = 3000;

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let val = b.intern_literal(RdfLiteral::simple(i.to_string()));
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register(LIST_COLLECTOR_IRI, Arc::new(ListCollectorAggregate));

        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/val")),
                object: TermPattern::Variable(Variable::new("val")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::Custom(NamedNode::new_unchecked(LIST_COLLECTOR_IRI)),
                    vec![Expression::Variable(Variable::new("val"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };

        let run = |forced: bool, chunk: usize| {
            let _parallel_guard = crate::parallel::force_parallel_for_test(forced);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(chunk);
            let mut ctx = EvalCtx::new(&ds).with_aggregates(&registry);
            let seq = eval(&group, &mut ctx).expect("eval");
            agg_lex(&ds, &ctx, &seq, "g")
        };

        let sequential = run(false, 64);
        let parallel = run(true, 53);
        assert_eq!(sequential, parallel);
        let expected = (0..ROWS)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(";");
        assert_eq!(sequential, expected);
    }

    /// `crate::stat_agg::FIRST` (`Volatility::Stable`) over a single implicit
    /// group large enough to cross the within-group chunk threshold: forced
    /// parallel and forced sequential must agree byte-for-byte, proving
    /// `FirstAccumulator::combine`'s "earlier chunk wins" merge is correct
    /// under `crate::parallel::par_chunk_reduce_init`'s fixed chunk-order
    /// reduce — the custom-aggregate-set counterpart of
    /// `within_group_custom_aggregate_chunked_fold_forced_parallel_and_sequential_agree`
    /// above, using a REAL first-party member instead of a test fixture.
    #[test]
    fn stat_agg_first_chunked_fold_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        const ROWS: i64 = 3000;
        const NS: &str = "http://example.org/agg/";

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let val = b.intern_literal(RdfLiteral::simple(i.to_string()));
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/val")),
                object: TermPattern::Variable(Variable::new("val")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::Custom(NamedNode::new_unchecked(format!("{NS}FIRST"))),
                    vec![Expression::Variable(Variable::new("val"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };

        let run = |forced: bool, chunk: usize| {
            let _parallel_guard = crate::parallel::force_parallel_for_test(forced);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(chunk);
            let mut ctx = EvalCtx::new(&ds).with_aggregates(&registry);
            let seq = eval(&group, &mut ctx).expect("eval");
            agg_lex(&ds, &ctx, &seq, "g")
        };

        let sequential = run(false, 64);
        let parallel = run(true, 53);
        assert_eq!(sequential, parallel);
        assert_eq!(
            sequential, "0",
            "FIRST over row order s0..s2999 is s0's value"
        );
    }

    /// Build a large single-implicit-group dataset of `ROWS` `xsd:integer`
    /// `ex:val` values (`?s0 ex:val 0`, …, `?s{ROWS-1} ex:val (ROWS-1)`) — the
    /// common fixture shape every `stat_agg` within-group chunked-fold
    /// determinism pin below shares with [`stat_agg_first_chunked_fold_forced_parallel_and_sequential_agree`].
    fn stat_agg_integer_sequence_dataset(rows: i64) -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..rows {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: i.to_string(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val_pred, val, None);
        }
        b.freeze().expect("freeze")
    }

    /// Evaluate a single-argument `AGG(<{NS}{local}>, ?val)` over `ds`'s implicit
    /// group, once forced sequential and once forced parallel with a small chunk
    /// size, returning `(sequential, forced_parallel)` — the shared driver every
    /// `stat_agg_*_chunked_fold_forced_parallel_and_sequential_agree` test below
    /// uses.
    fn stat_agg_run_single_arg(
        ds: &Arc<RdfDataset>,
        registry: &crate::agg_fn::AggregateRegistry,
        iri: &str,
    ) -> (String, String) {
        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/val")),
                object: TermPattern::Variable(Variable::new("val")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::Custom(NamedNode::new_unchecked(iri)),
                    vec![Expression::Variable(Variable::new("val"))],
                    Vec::new(),
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };
        let run = |forced: bool, chunk: usize| {
            let _parallel_guard = crate::parallel::force_parallel_for_test(forced);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(chunk);
            let mut ctx = EvalCtx::new(ds).with_aggregates(registry);
            let seq = eval(&group, &mut ctx).expect("eval");
            agg_lex(ds, &ctx, &seq, "g")
        };
        (run(false, 64), run(true, 53))
    }

    /// `crate::stat_agg::MEDIAN` — now `Volatility::Stable` (see that module's
    /// "Real merges via `AggregateAccumulator::into_any`" docs) — over a large
    /// single group must agree byte-for-byte between forced-parallel and
    /// forced-sequential: the determinism pin that a finish-only `combine`
    /// could never support, now exercising `MedianAccumulator::combine`'s real
    /// value-list merge through the within-group chunked fold.
    #[test]
    fn stat_agg_median_chunked_fold_forced_parallel_and_sequential_agree() {
        const ROWS: i64 = 3000;
        const NS: &str = "http://example.org/agg/";

        let ds = stat_agg_integer_sequence_dataset(ROWS);
        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let (sequential, forced_parallel) =
            stat_agg_run_single_arg(&ds, &registry, &format!("{NS}MEDIAN"));
        assert_eq!(
            sequential, forced_parallel,
            "MEDIAN's chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            sequential, "1499.5",
            "median of 0..2999 is the mean of the two middle values 1499 and 1500"
        );
    }

    /// `rows` `xsd:dayTimeDuration` literals `P0D, P2D, P4D, ..., P(2*(rows-1))D`
    /// — [`stat_agg_integer_sequence_dataset`]'s duration counterpart, used by
    /// the `crate::stat_agg`'s duration-extension forced-parallel test below.
    /// The `2 *` step keeps the eventual `MEDIAN` midpoint an EXACT whole-day
    /// value (see `crate::stat_agg`'s own
    /// `median_over_pure_day_time_duration_even_group_is_the_midpoint` unit
    /// test for the same reasoning at unit-test scale), so the pinned answer
    /// does not depend on sub-day duration canonicalization.
    fn stat_agg_dt_duration_sequence_dataset(rows: i64) -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        const XSD_DAY_TIME_DURATION: &str = "http://www.w3.org/2001/XMLSchema#dayTimeDuration";

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..rows {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let val = b.intern_literal(RdfLiteral {
                lexical_form: format!("P{}D", 2 * i),
                datatype: Some(XSD_DAY_TIME_DURATION.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val_pred, val, None);
        }
        b.freeze().expect("freeze")
    }

    /// `crate::stat_agg::MEDIAN`'s `xsd:duration` extension (see that module's
    /// "The `xsd:duration` extension" doc section) over a group large enough
    /// to cross `crate::parallel::PARALLEL_MIN_ROWS`, exercising the REAL
    /// `crate::parallel::par_chunk_reduce_init` chunked fold — forced-parallel
    /// and forced-sequential must agree byte-for-byte, the same determinism
    /// pin [`stat_agg_median_chunked_fold_forced_parallel_and_sequential_agree`]
    /// makes for the numeric case.
    #[test]
    fn stat_agg_median_duration_chunked_fold_forced_parallel_and_sequential_agree() {
        const ROWS: i64 = 3000;
        const NS: &str = "http://example.org/agg/";

        let ds = stat_agg_dt_duration_sequence_dataset(ROWS);
        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let (sequential, forced_parallel) =
            stat_agg_run_single_arg(&ds, &registry, &format!("{NS}MEDIAN"));
        assert_eq!(
            sequential, forced_parallel,
            "MEDIAN's duration chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            sequential, "P2999D",
            "median of P0D, P2D, .., P5998D is the midpoint of the two middle values P2998D and P3000D"
        );
    }

    /// `crate::stat_agg::LAST`'s counterpart of
    /// [`stat_agg_first_chunked_fold_forced_parallel_and_sequential_agree`]: over a
    /// large single group, forced-parallel and forced-sequential must agree, AND
    /// the answer must be the group's LAST row, not its first — the one outcome
    /// that would be silently wrong (with no local symptom — see
    /// `crate::stat_agg`'s "FIRST/LAST" doc section) if
    /// `crate::parallel::par_chunk_reduce_init`'s chunk-index-order reduce were
    /// ever reversed, or if `LastAccumulator::combine`'s "`other` is always the
    /// later chunk" assumption stopped holding.
    #[test]
    fn stat_agg_last_chunked_fold_forced_parallel_and_sequential_agree() {
        const ROWS: i64 = 3000;
        const NS: &str = "http://example.org/agg/";

        let ds = stat_agg_integer_sequence_dataset(ROWS);
        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let (sequential, forced_parallel) =
            stat_agg_run_single_arg(&ds, &registry, &format!("{NS}LAST"));
        assert_eq!(
            sequential, forced_parallel,
            "LAST's chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            sequential, "2999",
            "LAST over row order s0..s2999 is s2999's value"
        );
    }

    /// `crate::stat_agg::MODE` over a large single group where 1000 of the 3000
    /// rows share one value (`7`) and the rest are each unique: no matter how
    /// the group is chunked, `ModeAccumulator::combine`'s count-map-by-
    /// concatenation merge must recover the SAME winning value with the SAME
    /// count, so forced-parallel and forced-sequential must agree.
    #[test]
    fn stat_agg_mode_chunked_fold_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        const ROWS: i64 = 3000;
        const REPEATED_UP_TO: i64 = 1000;
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const NS: &str = "http://example.org/agg/";

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        for i in 0..ROWS {
            let subj = b.intern_iri(&format!("http://ex/s{i}"));
            let v = if i < REPEATED_UP_TO { 7 } else { i };
            let val = b.intern_literal(RdfLiteral {
                lexical_form: v.to_string(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val_pred, val, None);
        }
        let ds = b.freeze().expect("freeze");

        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let (sequential, forced_parallel) =
            stat_agg_run_single_arg(&ds, &registry, &format!("{NS}MODE"));
        assert_eq!(
            sequential, forced_parallel,
            "MODE's chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            sequential, "7",
            "7 occurs 1000 times, every other value exactly once"
        );
    }

    /// `crate::stat_agg::VAR_POP`/`STDDEV_POP` over a large single group built
    /// by repeating the module's own known 8-value dataset (population variance
    /// exactly `4`, population stddev exactly `2`) 400 times: repetition does
    /// not change either statistic, so this pins BOTH that
    /// `MomentsAccumulator::combine`'s `(n, Σx, Σx²)` merge agrees between
    /// forced-parallel and forced-sequential AND that it stays byte-exact —
    /// the moments family's own chunked-fold counterpart to `stat_agg.rs`'s
    /// `var_pop_matches_the_known_dataset` test.
    #[test]
    fn stat_agg_moments_chunked_fold_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        const KNOWN: [i64; 8] = [2, 4, 4, 4, 5, 5, 7, 9];
        const REPEATS: i64 = 400;
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const NS: &str = "http://example.org/agg/";

        let mut b = RdfDatasetBuilder::new();
        let val_pred = b.intern_iri("http://ex/val");
        let mut i = 0i64;
        for _ in 0..REPEATS {
            for &v in &KNOWN {
                let subj = b.intern_iri(&format!("http://ex/s{i}"));
                let val = b.intern_literal(RdfLiteral {
                    lexical_form: v.to_string(),
                    datatype: Some(XINT.to_owned()),
                    language: None,
                    direction: None,
                });
                b.push_quad(subj, val_pred, val, None);
                i += 1;
            }
        }
        let ds = b.freeze().expect("freeze");

        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let (var_sequential, var_forced_parallel) =
            stat_agg_run_single_arg(&ds, &registry, &format!("{NS}VAR_POP"));
        assert_eq!(
            var_sequential, var_forced_parallel,
            "VAR_POP's chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            var_sequential, "4",
            "population variance of REPEATS copies of the known dataset is still exactly 4"
        );

        let (stddev_sequential, stddev_forced_parallel) =
            stat_agg_run_single_arg(&ds, &registry, &format!("{NS}STDDEV_POP"));
        assert_eq!(
            stddev_sequential, stddev_forced_parallel,
            "STDDEV_POP's chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            stddev_sequential, "2.0E0",
            "population stddev of REPEATS copies of the known dataset is still exactly 2"
        );
    }

    /// `crate::stat_agg::TOPK` over the same 3000-row unique-value sequence
    /// [`stat_agg_median_chunked_fold_forced_parallel_and_sequential_agree`]
    /// uses, asking for the top 3: the true top-3 values (`2999`, `2998`,
    /// `2997`) may fall in different chunks depending on the (forced) chunk
    /// size, so this specifically exercises `TopKAccumulator::combine`'s
    /// bounded-structure merge (`insert_bounded` applied across chunk
    /// boundaries, not just within one chunk's `step` sequence).
    #[test]
    fn stat_agg_topk_chunked_fold_forced_parallel_and_sequential_agree() {
        const ROWS: i64 = 3000;
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const NS: &str = "http://example.org/agg/";

        let ds = stat_agg_integer_sequence_dataset(ROWS);
        let mut registry = crate::agg_fn::AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);

        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/val")),
                object: TermPattern::Variable(Variable::new("val")),
            }],
        };
        let group = GraphPattern::Group {
            inner: Box::new(inner),
            variables: vec![],
            aggregates: vec![(
                Variable::new("g"),
                AggregateExpression::new(
                    AggregateFunction::Custom(NamedNode::new_unchecked(format!("{NS}TOPK"))),
                    vec![Expression::Variable(Variable::new("val"))],
                    vec![(
                        "K".to_owned(),
                        purrdf_sparql_algebra::Literal::new_typed(
                            "3",
                            NamedNode::new_unchecked(XINT),
                        ),
                    )],
                    Vec::new(),
                    false,
                )
                .expect("fixture: valid AggregateExpression"),
            )],
        };

        let run = |forced: bool, chunk: usize| {
            let _parallel_guard = crate::parallel::force_parallel_for_test(forced);
            let _chunk_guard = crate::parallel::force_chunk_size_for_test(chunk);
            let mut ctx = EvalCtx::new(&ds).with_aggregates(&registry);
            let seq = eval(&group, &mut ctx).expect("eval");
            agg_lex(&ds, &ctx, &seq, "g")
        };

        let sequential = run(false, 64);
        let forced_parallel = run(true, 53);
        assert_eq!(
            sequential, forced_parallel,
            "TOPK's chunked within-group fold must agree with the sequential one"
        );
        assert_eq!(
            sequential, "2999 2998 2997",
            "the true top 3 of 0..2999, descending"
        );
    }
}
