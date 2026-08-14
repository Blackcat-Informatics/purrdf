// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Solution modifiers and the `VALUES` / `GRAPH` graph-pattern nodes:
//! `Project`, `Distinct`, `Reduced`, `OrderBy`, `Slice`, plus inline `VALUES` data
//! and named-graph scoping.

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use purrdf_core::{DatasetView, GraphMatch, TermValue, ViewTermId};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, NamedNodePattern,
    OrderExpression, Variable,
};
use purrdf_xsd::{XsdDatatype, XsdValue, numeric_add, numeric_div, parse_by_iri, value_cmp};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

use crate::convert::{ground_term_to_value, named_node_to_value};
use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::expr::{eval_expr, xsd_of, xsd_to_term};
use crate::governor::ChargePoint;
use crate::governor::lift::{Evaluated, Lift, Truncation};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
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

    // Precompute each row's typed sort keys — including the one-time XSD parse
    // that `term_value_order` would otherwise re-run inside the O(n log n)
    // comparator — so the sort comparator is a cheap pure function (no `ctx`
    // borrow, no re-parsing during the sort).
    let mut keyed: Vec<(Vec<SortKey>, Solution<D::Id>)> = Vec::with_capacity(seq.rows.len());
    for row in seq.rows {
        let mut keys = Vec::with_capacity(exprs.len());
        for oe in exprs {
            let term = eval_expr(order_expr(oe), &row, &schema, ctx)?;
            keys.push(sort_key(term.map(|t| ctx.scratch.value_of(ctx.dataset, t))));
        }
        keyed.push((keys, row));
    }

    keyed.sort_by(|(ka, _), (kb, _)| compare_keys(ka, kb, exprs));
    let rows = keyed.into_iter().map(|(_, row)| row).collect();
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

fn order_expr(oe: &OrderExpression) -> &Expression {
    match oe {
        OrderExpression::Asc(e) | OrderExpression::Desc(e) => e,
    }
}

fn is_descending(oe: &OrderExpression) -> bool {
    matches!(oe, OrderExpression::Desc(_))
}

/// Compare two rows' precomputed sort keys, applying each key's `ASC`/`DESC`.
fn compare_keys(a: &[SortKey], b: &[SortKey], exprs: &[OrderExpression]) -> Ordering {
    for (i, oe) in exprs.iter().enumerate() {
        let mut ord = compare_sort_keys(&a[i], &b[i]);
        if is_descending(oe) {
            ord = ord.reverse();
        }
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

/// A per-row precomputed ORDER BY sort key. The XSD parse (`parse_by_iri`) that
/// the SPARQL ordering would otherwise re-run for every literal comparison is
/// hoisted to key-build time; [`compare_sort_keys`] then mirrors the
/// unbound-first / kind-rank / value-space-with-deterministic-fallback semantics
/// of `sparql_order`/[`term_value_order`] EXACTLY.
enum SortKey {
    /// Unbound sorts before any bound term.
    Unbound,
    /// Blank node, ordered by `(scope ordinal, label)` — kind rank 0.
    Blank(u32, String),
    /// IRI, ordered by its string — kind rank 1.
    Iri(String),
    /// Literal — kind rank 2. `xsd` is the one-time parse for the value-space
    /// compare; the remaining fields are the deterministic `(datatype, language,
    /// lexical)` fallback tuple (`direction` is ignored, as in `literal_order`).
    Literal {
        xsd: Option<XsdValue>,
        datatype: String,
        language: Option<String>,
        lexical: String,
    },
    /// Triple term — kind rank 3 (rare). Its `(s, p, o)` components are themselves
    /// precomputed sort keys, so the literal XSD parse of a nested component is paid
    /// once at build time (not re-run per comparison, as `term_value_order` would);
    /// [`compare_sort_keys`] recurses over them componentwise.
    Triple(Box<[Self; 3]>),
}

/// The kind rank of a bound sort key: blank < IRI < literal < triple
/// (mirrors `kind_rank`; `Unbound` is handled before ranks are consulted).
fn sort_key_rank(k: &SortKey) -> u8 {
    match k {
        SortKey::Unbound | SortKey::Blank(..) => 0,
        SortKey::Iri(_) => 1,
        SortKey::Literal { .. } => 2,
        SortKey::Triple(_) => 3,
    }
}

/// Build the typed sort key for one (possibly unbound) ORDER BY value.
fn sort_key(value: Option<TermValue>) -> SortKey {
    match value {
        None => SortKey::Unbound,
        Some(TermValue::Blank { label, scope }) => SortKey::Blank(scope.ordinal(), label),
        Some(TermValue::Iri(iri)) => SortKey::Iri(iri),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        }) => SortKey::Literal {
            xsd: parse_by_iri(&lexical_form, &datatype).ok().flatten(),
            datatype,
            language,
            lexical: lexical_form,
        },
        Some(TermValue::Triple { s, p, o }) => SortKey::Triple(Box::new([
            sort_key(Some(*s)),
            sort_key(Some(*p)),
            sort_key(Some(*o)),
        ])),
    }
}

/// SPARQL ORDER BY total order over precomputed keys: unbound sorts before any
/// bound term; otherwise by term kind (blank < IRI < literal < triple) and then
/// within the kind — identical ordering to `sparql_order` over the raw values,
/// with the literal XSD parse already paid at key-build time.
fn compare_sort_keys(a: &SortKey, b: &SortKey) -> Ordering {
    match (a, b) {
        (SortKey::Unbound, SortKey::Unbound) => Ordering::Equal,
        (SortKey::Unbound, _) => Ordering::Less,
        (_, SortKey::Unbound) => Ordering::Greater,
        (SortKey::Blank(sa, la), SortKey::Blank(sb, lb)) => (sa, la).cmp(&(sb, lb)),
        (SortKey::Iri(x), SortKey::Iri(y)) => x.cmp(y),
        (
            SortKey::Literal {
                xsd: ax,
                datatype: dx,
                language: gx,
                lexical: lx,
            },
            SortKey::Literal {
                xsd: bx,
                datatype: dy,
                language: gy,
                lexical: ly,
            },
        ) => {
            // Value space where both parse AND compare; else the deterministic
            // (datatype, language, lexical) fallback — exactly `literal_order`.
            if let (Some(av), Some(bv)) = (ax, bx)
                && let Some(ord) = value_cmp(av, bv)
            {
                return ord;
            }
            (dx, gx, lx).cmp(&(dy, gy, ly))
        }
        (SortKey::Triple(x), SortKey::Triple(y)) => compare_triple_keys(x, y),
        _ => sort_key_rank(a).cmp(&sort_key_rank(b)),
    }
}

/// Compare two triple-term sort keys componentwise (`s`, then `p`, then `o`) — the
/// precomputed-key analogue of [`term_value_order`]'s triple arm, with each
/// component already parsed at build time.
fn compare_triple_keys(a: &[SortKey; 3], b: &[SortKey; 3]) -> Ordering {
    compare_sort_keys(&a[0], &b[0])
        .then_with(|| compare_sort_keys(&a[1], &b[1]))
        .then_with(|| compare_sort_keys(&a[2], &b[2]))
}

fn kind_rank(v: &TermValue) -> u8 {
    match v {
        TermValue::Blank { .. } => 0,
        TermValue::Iri(_) => 1,
        TermValue::Literal { .. } => 2,
        TermValue::Triple { .. } => 3,
    }
}

fn term_value_order(a: &TermValue, b: &TermValue) -> Ordering {
    match (a, b) {
        (
            TermValue::Blank {
                label: la,
                scope: sa,
            },
            TermValue::Blank {
                label: lb,
                scope: sb,
            },
        ) => (sa.ordinal(), la).cmp(&(sb.ordinal(), lb)),
        (TermValue::Iri(x), TermValue::Iri(y)) => x.cmp(y),
        (
            TermValue::Literal {
                lexical_form: lx,
                datatype: dx,
                language: gx,
                ..
            },
            TermValue::Literal {
                lexical_form: ly,
                datatype: dy,
                language: gy,
                ..
            },
        ) => literal_order((lx, dx, gx), (ly, dy, gy)),
        (
            TermValue::Triple {
                s: sa,
                p: pa,
                o: oa,
            },
            TermValue::Triple {
                s: sb,
                p: pb,
                o: ob,
            },
        ) => term_value_order(sa, sb)
            .then_with(|| term_value_order(pa, pb))
            .then_with(|| term_value_order(oa, ob)),
        _ => kind_rank(a).cmp(&kind_rank(b)),
    }
}

/// Order two literals: by XSD value where both are value-comparable, otherwise a
/// deterministic fall-back by (datatype, language, lexical form).
fn literal_order(a: (&str, &str, &Option<String>), b: (&str, &str, &Option<String>)) -> Ordering {
    let (lx, dx, gx) = a;
    let (ly, dy, gy) = b;
    if let (Ok(Some(ax)), Ok(Some(bx))) = (parse_by_iri(lx, dx), parse_by_iri(ly, dy))
        && let Some(ord) = value_cmp(&ax, &bx)
    {
        return ord;
    }
    (dx, gx, lx).cmp(&(dy, gy, ly))
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

/// Compute one aggregate over a group's rows — streaming: every value is folded
/// as it is produced, never materialized into a per-group buffer first. A
/// built-in and a registered [`crate::agg_fn::CustomAggregate`] both instantiate
/// the same init/step/finish shape ([`BuiltinFold`] for the former,
/// [`crate::agg_fn`]'s trait pair for the latter); this function is the ONE place
/// that decides which of the two a given [`AggregateExpression`] folds through —
/// and, because both kinds are dispatched from here, the ONE place that charges
/// [`ChargePoint::AggregateInvocation`] for either of them.
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

    // `COUNT(*)` is the spec's empty exprlist: count rows (or distinct rows),
    // never evaluating an expression at all — the only aggregate an empty
    // `exprlist` can name (see `AggregateExpression::args`'s docs).
    let Some(first_arg) = agg.args.first() else {
        // Every row is a value `COUNT(*)` folds, whether or not `DISTINCT` keeps
        // it — see [`ChargePoint::AggregateAccumulation`]'s doc for why the
        // charge precedes the dedup check. An explicit loop, rather than
        // `Iterator::count`, so a refused charge stops the count exactly where
        // the budget ran out instead of after the whole group was scanned.
        let mut seen: Option<DetHashSet<&Solution<D::Id>>> = agg.distinct.then(DetHashSet::default);
        let mut count: i64 = 0;
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
            count += 1;
        }
        return Ok(Some(integer_term(ctx, count)));
    };

    if let AggregateFunction::Custom(iri) = &agg.function {
        return eval_custom_aggregate(iri.as_str(), agg, idxs, rows, schema, ctx);
    }

    // Every built-in aggregate other than `COUNT(*)` is unary — only this one
    // argument expression is ever evaluated.
    let mut fold: BuiltinFold<'_, D> = BuiltinFold::init(&agg.function, agg.separator());
    // DISTINCT dedups by `SolutionTerm` equality, which the scratch interner's
    // Existing/Computed promotion rule (see `crate::scratch`'s module docs) makes
    // exactly equivalent to dedup-by-value: two distinct `SolutionTerm`s never
    // denote the same value. Cheaper than hashing the resolved `TermValue` (an
    // owned-string clone for a literal/IRI), and byte-identical to the prior
    // materializing implementation's `seen: DetHashSet<SolutionTerm>` retain.
    let mut seen: Option<DetHashSet<SolutionTerm<D::Id>>> = agg.distinct.then(DetHashSet::default);
    for &i in idxs {
        let Some(term) = eval_expr(first_arg, &rows[i], schema, ctx)? else {
            continue;
        };
        // The `aggregate-accumulation` charge point, charged for every value the
        // argument expression produced — BEFORE the `DISTINCT` check below, per
        // [`ChargePoint::AggregateAccumulation`]'s documented reading: producing
        // and inspecting the value is the work this point prices, and that work
        // already happened by the time a duplicate is discarded.
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
        fold.step(term, &value);
    }
    Ok(fold.finish(ctx))
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
    let custom = ctx
        .aggregates
        .and_then(|registry| registry.resolve(iri))
        .cloned()
        .ok_or_else(|| {
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

    let mut accumulator = crate::agg_fn::init_contained(custom.as_ref(), iri)?;
    let mut seen: Option<DetHashSet<Vec<TermValue>>> = agg.distinct.then(DetHashSet::default);
    let mut tuple: Vec<TermValue> = Vec::with_capacity(agg.args.len());
    for &i in idxs {
        tuple.clear();
        let mut every_position_bound = true;
        for expression in &agg.args {
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
        crate::agg_fn::step_contained(accumulator.as_mut(), iri, &tuple)?;
    }
    let value = crate::agg_fn::finish_contained(accumulator, iri)?;
    Ok(value.map(|v| ctx.scratch.intern(ctx.dataset, v)))
}

/// Whether an [`XsdValue`] belongs to the SPARQL numeric tower (integer / decimal /
/// float / double). Boolean, string, temporal, and binary values are NOT numeric.
fn is_numeric_xsd(v: &XsdValue) -> bool {
    matches!(
        v,
        XsdValue::Integer { .. } | XsdValue::Decimal(_) | XsdValue::Float(_) | XsdValue::Double(_)
    )
}

/// The running numeric fold `SUM`/`AVG` share: a value is folded in with
/// `numeric_add`, seeded by the FIRST folded value (never `0 + first`, so the
/// result datatype matches the pre-restructuring "remove the first element,
/// then fold the rest" implementation exactly). A non-numeric value or an
/// overflow POISONS the fold permanently — the answer is unbound regardless of
/// what would have followed — mirroring the materializing implementation's
/// immediate `return Ok(None)`, just without needing to abort the row loop
/// early (the poisoned state simply ignores every further `step`).
#[derive(Debug, Clone)]
enum NumericFold {
    /// No value has been folded in yet.
    Empty,
    /// A running sum plus the count of values folded so far (only `AVG` reads
    /// the count).
    Ok { acc: XsdValue, count: u64 },
    /// A non-numeric value or an overflow was seen.
    Poisoned,
}

impl NumericFold {
    fn step(&mut self, value: &TermValue) {
        if matches!(self, Self::Poisoned) {
            return;
        }
        let Some(xv) = xsd_of(value).filter(is_numeric_xsd) else {
            *self = Self::Poisoned;
            return;
        };
        match self {
            Self::Empty => *self = Self::Ok { acc: xv, count: 1 },
            Self::Ok { acc, count } => match numeric_add(acc, &xv) {
                Ok(sum) => {
                    *acc = sum;
                    *count += 1;
                }
                Err(_) => *self = Self::Poisoned,
            },
            Self::Poisoned => unreachable!("poisoned state returns above"),
        }
    }

    /// `SUM`'s finish: empty group → `0^^xsd:integer` (SPARQL §18.5.1); poisoned
    /// → unbound; otherwise the running total.
    fn finish_sum<D: DatasetView + Sync>(
        self,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Option<SolutionTerm<D::Id>> {
        match self {
            Self::Empty => Some(integer_term(ctx, 0)),
            Self::Ok { acc, .. } => Some(xsd_to_term(ctx, &acc)),
            Self::Poisoned => None,
        }
    }

    /// `AVG`'s finish: empty group → `0^^xsd:integer`; poisoned → unbound;
    /// otherwise the running total divided by the folded count.
    fn finish_avg<D: DatasetView + Sync>(
        self,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Option<SolutionTerm<D::Id>> {
        match self {
            Self::Empty => Some(integer_term(ctx, 0)),
            Self::Ok { acc, count } => {
                let count_val = XsdValue::Integer {
                    value: i128::from(count),
                    datatype: XsdDatatype::Integer,
                };
                match numeric_div(&acc, &count_val) {
                    Ok(avg) => Some(xsd_to_term(ctx, &avg)),
                    Err(_) => None,
                }
            }
            Self::Poisoned => None,
        }
    }
}

/// The streaming fold state of one built-in aggregate over one group — the
/// internal, statically-dispatched instance of the same init/step/finish shape
/// [`crate::agg_fn::CustomAggregate`]/[`crate::agg_fn::AggregateAccumulator`]
/// give a HOST-registered aggregate. Kept as a plain enum-of-states rather than a
/// `Box<dyn>` so a built-in's per-row fold costs no allocation and no vtable
/// dispatch — the trait in `crate::agg_fn` is the seam for a caller-supplied
/// aggregate; this is the shape every built-in already had before it needed one.
///
/// Algebraic class (informational, matching `AlgebraicClass`'s documentation of
/// the built-ins in `crate::agg_fn`): `Count`/`Sum`/`Avg` are `Commutative`
/// (row order never changes the answer); `Min`/`Max` are `Commutative` under
/// SPARQL term ordering (ties keep the earliest occurrence, which does not
/// change the VALUE, only which term instance is returned — see
/// [`fold_extreme`]); `Sample`/`GroupConcat` are `OrderDependent` ("first value
/// wins" / ordered concatenation).
enum BuiltinFold<'a, D: DatasetView + Sync> {
    Count(i64),
    Sample(Option<SolutionTerm<D::Id>>),
    Min(Option<(SolutionTerm<D::Id>, TermValue)>),
    Max(Option<(SolutionTerm<D::Id>, TermValue)>),
    GroupConcat {
        sep: &'a str,
        buf: String,
        started: bool,
    },
    Sum(NumericFold),
    Avg(NumericFold),
}

impl<'a, D: DatasetView + Sync> BuiltinFold<'a, D> {
    /// The empty-group answer for `function` is exactly `Self::init(function,
    /// ..).finish(ctx)` with no `step` ever called — every built-in answers
    /// explicitly (see the module-level fold-algebra contract): `Count` → `0`,
    /// `Sum` → `0`, `Avg`/`Min`/`Max`/`Sample` → unbound, `GroupConcat` → `""`.
    fn init(function: &AggregateFunction, separator: Option<&'a str>) -> Self {
        match function {
            AggregateFunction::Count => Self::Count(0),
            AggregateFunction::Sample => Self::Sample(None),
            AggregateFunction::Min => Self::Min(None),
            AggregateFunction::Max => Self::Max(None),
            AggregateFunction::GroupConcat => Self::GroupConcat {
                sep: separator.unwrap_or(" "),
                buf: String::new(),
                started: false,
            },
            AggregateFunction::Sum => Self::Sum(NumericFold::Empty),
            AggregateFunction::Avg => Self::Avg(NumericFold::Empty),
            AggregateFunction::Custom(_) => unreachable!(
                "a Custom aggregate dispatches through eval_custom_aggregate before a \
                 BuiltinFold is ever constructed"
            ),
        }
    }

    /// Fold one row's already-evaluated `(term, value)` pair in.
    fn step(&mut self, term: SolutionTerm<D::Id>, value: &TermValue) {
        match self {
            Self::Count(n) => *n += 1,
            Self::Sample(slot) => {
                if slot.is_none() {
                    *slot = Some(term);
                }
            }
            Self::Min(slot) => *slot = Some(fold_extreme(slot.take(), term, value, Ordering::Less)),
            Self::Max(slot) => {
                *slot = Some(fold_extreme(slot.take(), term, value, Ordering::Greater));
            }
            Self::GroupConcat { sep, buf, started } => {
                if let Some(lexical) = lexical_of(value) {
                    if *started {
                        buf.push_str(sep);
                    } else {
                        *started = true;
                    }
                    buf.push_str(&lexical);
                }
            }
            Self::Sum(state) | Self::Avg(state) => state.step(value),
        }
    }

    /// Produce the group's answer, consuming the fold.
    fn finish(self, ctx: &mut EvalCtx<'_, D>) -> Option<SolutionTerm<D::Id>> {
        match self {
            Self::Count(n) => Some(integer_term(ctx, n)),
            Self::Sample(slot) => slot,
            Self::Min(slot) | Self::Max(slot) => slot.map(|(t, _)| t),
            Self::GroupConcat { buf, .. } => Some(string_term(ctx, buf)),
            Self::Sum(state) => state.finish_sum(ctx),
            Self::Avg(state) => state.finish_avg(ctx),
        }
    }
}

/// One step of `MIN`/`MAX`'s running extreme: `want` is `Ordering::Less` for
/// `MIN`, `Greater` for `MAX`. Ties (`term_value_order` returns anything other
/// than `want`) keep the EARLIER occurrence — the same left-fold tie-break the
/// prior `values.iter().reduce(..)` implementation had, since `reduce` seeds its
/// accumulator with the first element and only replaces it when a later element
/// compares strictly better.
fn fold_extreme<I: ViewTermId>(
    current: Option<(SolutionTerm<I>, TermValue)>,
    term: SolutionTerm<I>,
    value: &TermValue,
    want: Ordering,
) -> (SolutionTerm<I>, TermValue) {
    match current {
        None => (term, value.clone()),
        Some((current_term, current_value)) => {
            if term_value_order(value, &current_value) == want {
                (term, value.clone())
            } else {
                (current_term, current_value)
            }
        }
    }
}

/// The lexical string of a term for GROUP_CONCAT (literal lexical / IRI string).
fn lexical_of(value: &TermValue) -> Option<String> {
    match value {
        TermValue::Literal { lexical_form, .. } => Some(lexical_form.clone()),
        TermValue::Iri(iri) => Some(iri.clone()),
        _ => None,
    }
}

/// Intern an `xsd:integer` literal.
fn integer_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    value: i64,
) -> SolutionTerm<D::Id> {
    ctx.scratch.intern(
        ctx.dataset,
        TermValue::Literal {
            lexical_form: value.to_string(),
            datatype: XSD_INTEGER.to_owned(),
            language: None,
            direction: None,
        },
    )
}

/// Intern an `xsd:string` literal.
fn string_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    lexical: String,
) -> SolutionTerm<D::Id> {
    ctx.scratch.intern(
        ctx.dataset,
        TermValue::Literal {
            lexical_form: lexical,
            datatype: XSD_STRING.to_owned(),
            language: None,
            direction: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::eval;

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
                AggregateExpression {
                    function: AggregateFunction::Count,
                    args: Vec::new(),
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Count,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: true,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Count,
                    args: Vec::new(),
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Min,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Sum,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Sum,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Sum,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Sum,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Avg,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Avg,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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
                AggregateExpression {
                    function: AggregateFunction::Sum,
                    args: vec![Expression::Variable(Variable::new("n"))],
                    scalarvals: Vec::new(),
                    distinct: false,
                },
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

    /// The precomputed triple sort key (`sort_key` + `compare_sort_keys`) must order
    /// quoted-triple terms **identically** to the reference `term_value_order` over
    /// the raw values — the only difference is that the nested literals' XSD parse is
    /// paid once at key-build time instead of on every comparison. Includes cases
    /// where value-space and lexical order disagree (integer `9` < `30` by value but
    /// `"30"` < `"9"` lexically), cross-kind components, and a nested triple.
    #[test]
    fn triple_sort_keys_match_term_value_order() {
        let lit = |n: &str| TermValue::Literal {
            lexical_form: n.to_owned(),
            datatype: XINT.to_owned(),
            language: None,
            direction: None,
        };
        let iri = |s: &str| TermValue::Iri(s.to_owned());
        let triple = |s: TermValue, p: TermValue, o: TermValue| TermValue::Triple {
            s: Box::new(s),
            p: Box::new(p),
            o: Box::new(o),
        };
        let samples = [
            triple(iri("http://ex/a"), iri("http://ex/p"), lit("30")),
            triple(iri("http://ex/a"), iri("http://ex/p"), lit("9")),
            triple(iri("http://ex/a"), iri("http://ex/p"), iri("http://ex/z")),
            triple(iri("http://ex/b"), iri("http://ex/p"), lit("9")),
            triple(
                triple(iri("http://ex/a"), iri("http://ex/p"), lit("9")),
                iri("http://ex/q"),
                lit("30"),
            ),
        ];
        for a in &samples {
            for b in &samples {
                let via_keys =
                    compare_sort_keys(&sort_key(Some(a.clone())), &sort_key(Some(b.clone())));
                let via_ref = term_value_order(a, b);
                assert_eq!(
                    via_keys, via_ref,
                    "ordering mismatch:\n  a={a:?}\n  b={b:?}"
                );
            }
        }
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
                    AggregateExpression {
                        function: AggregateFunction::Count,
                        args: Vec::new(),
                        scalarvals: Vec::new(),
                        distinct: false,
                    },
                ),
                (
                    Variable::new("avg"),
                    AggregateExpression {
                        function: AggregateFunction::Avg,
                        args: vec![Expression::Variable(Variable::new("val"))],
                        scalarvals: Vec::new(),
                        distinct: false,
                    },
                ),
                (
                    Variable::new("mx"),
                    AggregateExpression {
                        function: AggregateFunction::Max,
                        args: vec![Expression::Variable(Variable::new("val"))],
                        scalarvals: Vec::new(),
                        distinct: false,
                    },
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
}
