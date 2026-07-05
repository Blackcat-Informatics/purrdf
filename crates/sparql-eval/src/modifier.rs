// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Solution modifiers and the `VALUES` / `GRAPH` graph-pattern nodes:
//! `Project`, `Distinct`, `Reduced`, `OrderBy`, `Slice`, plus inline `VALUES` data
//! and named-graph scoping.

use std::cmp::Ordering;
use std::sync::Arc;

use purrdf_core::{GraphMatch, TermId, TermValue};
use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, NamedNodePattern,
    OrderExpression, Variable,
};
use purrdf_xsd::{numeric_add, numeric_div, parse_by_iri, value_cmp, XsdDatatype, XsdValue};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

use crate::convert::{ground_term_to_value, named_node_to_value};
use crate::error::EvalError;
use crate::eval::{eval, EvalCtx};
use crate::expr::{eval_expr, xsd_of, xsd_to_term};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
use crate::DetHashSet;

/// Inline `VALUES`: one solution per binding row, each cell an interned ground term
/// (or unbound for `UNDEF`).
pub(crate) fn eval_values(
    variables: &[Variable],
    bindings: &[Vec<Option<purrdf_sparql_algebra::GroundTerm>>],
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let schema = Arc::new(VarSchema::from_vars(variables.iter().cloned()));
    let width = schema.len();
    let mut rows = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let mut row = vec![None; width];
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
pub(crate) fn eval_project(
    inner: &GraphPattern,
    variables: &[Variable],
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let seq = eval(inner, ctx)?;
    let out = Arc::new(VarSchema::from_vars(variables.iter().cloned()));
    // For each projected column, the source column in the inner schema (if any).
    let src: Vec<Option<usize>> = out.vars().iter().map(|v| seq.schema.index_of(v)).collect();
    let rows = seq
        .rows
        .iter()
        .map(|row| src.iter().map(|s| s.and_then(|c| row[c])).collect())
        .collect();
    Ok(SolutionSeq { schema: out, rows })
}

/// `DISTINCT`: drop duplicate whole-solution rows, preserving first-seen order.
pub(crate) fn eval_distinct(
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    Ok(dedup(eval(inner, ctx)?))
}

/// `REDUCED`: permitted to drop duplicates; we apply the same dedup as `DISTINCT`
/// (a stronger-but-permitted reduction than the spec's minimum).
pub(crate) fn eval_reduced(
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    Ok(dedup(eval(inner, ctx)?))
}

/// Drop duplicate rows, preserving first-seen order (SolutionTerm equality is exact
/// RDF-term identity — see the scratch-interner promotion rule).
fn dedup(seq: SolutionSeq) -> SolutionSeq {
    let mut seen: DetHashSet<Solution> = DetHashSet::default();
    let mut rows = Vec::new();
    for row in seq.rows {
        if seen.insert(row.clone()) {
            rows.push(row);
        }
    }
    SolutionSeq {
        schema: seq.schema,
        rows,
    }
}

/// `LIMIT`/`OFFSET`: skip `start` solutions then keep at most `length`.
pub(crate) fn eval_slice(
    inner: &GraphPattern,
    start: usize,
    length: Option<usize>,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let seq = eval(inner, ctx)?;
    let rows = seq
        .rows
        .into_iter()
        .skip(start)
        .take(length.unwrap_or(usize::MAX))
        .collect();
    Ok(SolutionSeq {
        schema: seq.schema,
        rows,
    })
}

/// `ORDER BY`: stable-sort by the sort keys under SPARQL ordering (§15.1).
pub(crate) fn eval_order_by(
    inner: &GraphPattern,
    exprs: &[OrderExpression],
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let seq = eval(inner, ctx)?;
    let schema = seq.schema.clone();

    // Precompute each row's typed sort keys — including the one-time XSD parse
    // that `term_value_order` would otherwise re-run inside the O(n log n)
    // comparator — so the sort comparator is a cheap pure function (no `ctx`
    // borrow, no re-parsing during the sort).
    let mut keyed: Vec<(Vec<SortKey>, Solution)> = Vec::with_capacity(seq.rows.len());
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
    Ok(SolutionSeq { schema, rows })
}

/// `GRAPH name { ... }`: scope the inner pattern to a named graph (or, for a
/// variable, every named graph in turn, binding the variable to each).
pub(crate) fn eval_graph(
    name: &NamedNodePattern,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    match name {
        NamedNodePattern::NamedNode(n) => {
            match ctx.dataset.term_id_by_value(&named_node_to_value(n)) {
                // Addressable only if the active dataset's named set admits it (a
                // `FROM NAMED` / `USING NAMED` may restrict which graphs `GRAPH` sees).
                Some(id) if ctx.active_dataset.named_allows(id) => {
                    let saved = ctx.active_graph;
                    ctx.active_graph = GraphMatch::Named(id);
                    let result = eval(inner, ctx);
                    ctx.active_graph = saved;
                    result
                }
                // The IRI is not a term (no quads), or not in the named dataset → empty.
                _ => {
                    let seq = eval(inner, ctx)?;
                    Ok(SolutionSeq::empty(seq.schema))
                }
            }
        }
        NamedNodePattern::Variable(v) => eval_graph_var(v, inner, ctx),
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
fn eval_graph_var(
    var: &Variable,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    // Enumerate every named graph the dataset knows about, restricted to those the
    // active dataset admits (a `FROM NAMED` / `USING NAMED` may limit which graphs
    // `GRAPH ?g` binds to).
    let graphs: Vec<TermId> = ctx
        .dataset
        .named_graphs()
        .filter(|g| ctx.active_dataset.named_allows(*g))
        .collect();

    let saved = ctx.active_graph;
    let mut out_schema: Option<Arc<VarSchema>> = None;
    let mut rows = Vec::new();
    for g in graphs {
        ctx.active_graph = GraphMatch::Named(g);
        let inner_seq = eval(inner, ctx)?;
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
    let schema = match out_schema {
        Some(s) => s,
        None => {
            let seq = eval(inner, ctx)?;
            let mut sch = (*seq.schema).clone();
            sch.push(var.clone());
            Arc::new(sch)
        }
    };
    Ok(SolutionSeq { schema, rows })
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
            if let (Some(av), Some(bv)) = (ax, bx) {
                if let Some(ord) = value_cmp(av, bv) {
                    return ord;
                }
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
    if let (Ok(Some(ax)), Ok(Some(bx))) = (parse_by_iri(lx, dx), parse_by_iri(ly, dy)) {
        if let Some(ord) = value_cmp(&ax, &bx) {
            return ord;
        }
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
pub(crate) fn eval_group(
    inner: &GraphPattern,
    variables: &[Variable],
    aggregates: &[(Variable, AggregateExpression)],
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let seq = eval(inner, ctx)?;
    let in_schema = seq.schema.clone();
    let key_cols: Vec<Option<usize>> = variables.iter().map(|v| in_schema.index_of(v)).collect();

    // Partition rows into groups, keeping groups in first-seen order.
    let mut order: Vec<Vec<Option<SolutionTerm>>> = Vec::new();
    let mut groups: crate::DetHashMap<Vec<Option<SolutionTerm>>, Vec<usize>> =
        crate::DetHashMap::default();
    for (idx, row) in seq.rows.iter().enumerate() {
        let key: Vec<Option<SolutionTerm>> =
            key_cols.iter().map(|c| c.and_then(|c| row[c])).collect();
        if !groups.contains_key(&key) {
            order.push(key.clone());
            groups.insert(key.clone(), Vec::new());
        }
        groups.get_mut(&key).unwrap().push(idx);
    }
    // No GROUP BY + empty input + aggregates → a single empty group.
    if order.is_empty() && variables.is_empty() && !aggregates.is_empty() {
        order.push(Vec::new());
        groups.insert(Vec::new(), Vec::new());
    }

    let mut out_schema = VarSchema::from_vars(variables.iter().cloned());
    for (out_var, _) in aggregates {
        out_schema.push(out_var.clone());
    }
    let out_schema = Arc::new(out_schema);
    let out_width = out_schema.len();
    let var_count = variables.len();

    // Every aggregate expression must be parallel-safe (no RAND/UUID/STRUUID/
    // BNODE/list-mint reachable) for the per-group compute below to run under
    // the fork-join model; `should_parallelize` (inside `par_chunk_try_map_init`)
    // still gates on group count.
    let safe = aggregates.iter().all(|(_, agg)| match agg {
        AggregateExpression::CountStar { .. } => true,
        AggregateExpression::FunctionCall { expression, .. } => {
            crate::parallel::is_parallel_safe(expression)
        }
    });

    let rows = if safe {
        let base = ctx.scratch.computed_count();
        let minted = crate::parallel::par_chunk_try_map_init(
            &order,
            || ctx.fork_for_worker(),
            |child, acc, key| {
                let idxs = &groups[key];
                let mut row = vec![None; out_width];
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
        let mut rows = Vec::with_capacity(order.len());
        for key in &order {
            let idxs = &groups[key];
            let mut row = vec![None; out_width];
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

    Ok(SolutionSeq {
        schema: out_schema,
        rows,
    })
}

/// Compute one aggregate over a group's rows.
fn eval_aggregate(
    agg: &AggregateExpression,
    idxs: &[usize],
    rows: &[Solution],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
    match agg {
        AggregateExpression::CountStar { distinct } => {
            let count = if *distinct {
                let mut seen: DetHashSet<&Solution> = DetHashSet::default();
                idxs.iter().filter(|&&i| seen.insert(&rows[i])).count()
            } else {
                idxs.len()
            };
            Ok(Some(integer_term(ctx, count as i64)))
        }
        AggregateExpression::FunctionCall {
            function,
            expression,
            distinct,
        } => {
            // Collect the bound values of the expression over the group.
            let mut values: Vec<(SolutionTerm, TermValue)> = Vec::new();
            for &i in idxs {
                if let Some(term) = eval_expr(expression, &rows[i], schema, ctx)? {
                    let value = ctx.scratch.value_of(ctx.dataset, term);
                    values.push((term, value));
                }
            }
            if *distinct {
                let mut seen: DetHashSet<SolutionTerm> = DetHashSet::default();
                values.retain(|(t, _)| seen.insert(*t));
            }
            apply_aggregate(function, &values, ctx)
        }
    }
}

/// Apply a named aggregate to the collected group values.
fn apply_aggregate(
    function: &AggregateFunction,
    values: &[(SolutionTerm, TermValue)],
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
    match function {
        AggregateFunction::Count => Ok(Some(integer_term(ctx, values.len() as i64))),
        AggregateFunction::Sample => Ok(values.first().map(|(t, _)| *t)),
        AggregateFunction::Min => Ok(extreme(values, Ordering::Less)),
        AggregateFunction::Max => Ok(extreme(values, Ordering::Greater)),
        AggregateFunction::GroupConcat { separator } => {
            let sep = separator.as_deref().unwrap_or(" ");
            let joined = values
                .iter()
                .filter_map(|(_, v)| lexical_of(v))
                .collect::<Vec<_>>()
                .join(sep);
            Ok(Some(string_term(ctx, joined)))
        }
        AggregateFunction::Sum => {
            // Empty group → 0^^xsd:integer (SPARQL §18.5.1).
            if values.is_empty() {
                return Ok(Some(integer_term(ctx, 0)));
            }
            // Extract numeric XsdValues; any non-numeric → unbound.
            let mut numerics: Vec<XsdValue> = Vec::with_capacity(values.len());
            for (_, v) in values {
                match xsd_of(v) {
                    Some(xv) if is_numeric_xsd(&xv) => numerics.push(xv),
                    _ => return Ok(None),
                }
            }
            // Fold left with numeric_add; any error (overflow) → unbound.
            let mut acc = numerics.remove(0);
            for xv in numerics {
                match numeric_add(&acc, &xv) {
                    Ok(sum) => acc = sum,
                    Err(_) => return Ok(None),
                }
            }
            Ok(Some(xsd_to_term(ctx, &acc)))
        }
        AggregateFunction::Avg => {
            // Empty group → 0^^xsd:integer.
            if values.is_empty() {
                return Ok(Some(integer_term(ctx, 0)));
            }
            let n = values.len();
            // Extract numeric XsdValues; any non-numeric → unbound.
            let mut numerics: Vec<XsdValue> = Vec::with_capacity(n);
            for (_, v) in values {
                match xsd_of(v) {
                    Some(xv) if is_numeric_xsd(&xv) => numerics.push(xv),
                    _ => return Ok(None),
                }
            }
            // Sum.
            let mut acc = numerics.remove(0);
            for xv in numerics {
                match numeric_add(&acc, &xv) {
                    Ok(sum) => acc = sum,
                    Err(_) => return Ok(None),
                }
            }
            // Divide by count to get average.
            let count_val = XsdValue::Integer {
                value: n as i128,
                datatype: XsdDatatype::Integer,
            };
            match numeric_div(&acc, &count_val) {
                Ok(avg) => Ok(Some(xsd_to_term(ctx, &avg))),
                Err(_) => Ok(None),
            }
        }
        AggregateFunction::Custom(iri) => Err(EvalError::unsupported(format!(
            "custom aggregate <{}>",
            iri.as_str()
        ))),
    }
}

/// Whether an [`XsdValue`] belongs to the SPARQL numeric tower (integer / decimal /
/// float / double). Boolean, string, temporal, and binary values are NOT numeric.
fn is_numeric_xsd(v: &XsdValue) -> bool {
    matches!(
        v,
        XsdValue::Integer { .. } | XsdValue::Decimal(_) | XsdValue::Float(_) | XsdValue::Double(_)
    )
}

/// The group's extreme value (`Ordering::Less` = MIN, `Greater` = MAX) under SPARQL
/// term ordering, returning its solution term; `None` for an empty group.
fn extreme(values: &[(SolutionTerm, TermValue)], want: Ordering) -> Option<SolutionTerm> {
    values
        .iter()
        .reduce(|acc, cur| {
            if term_value_order(&cur.1, &acc.1) == want {
                cur
            } else {
                acc
            }
        })
        .map(|(t, _)| *t)
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
fn integer_term(ctx: &mut EvalCtx<'_>, value: i64) -> SolutionTerm {
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
fn string_term(ctx: &mut EvalCtx<'_>, lexical: String) -> SolutionTerm {
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
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/type")),
                    object: TermPattern::Variable(Variable::new("t")),
                },
                TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked("http://ex/age")),
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
                AggregateExpression::CountStar { distinct: false },
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Count,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::CountStar { distinct: false },
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Min,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
    fn agg_lex(ds: &Arc<RdfDataset>, ctx: &EvalCtx<'_>, seq: &SolutionSeq, var: &str) -> String {
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Sum,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Sum,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Sum,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Sum,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Avg,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Avg,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Sum,
                    expression: Box::new(Expression::Variable(Variable::new("n"))),
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

    /// Determinism smoke test (Task 6): `GROUP BY ?cat` with `COUNT(*)`/`AVG(?val)`/
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
                    AggregateExpression::CountStar { distinct: false },
                ),
                (
                    Variable::new("avg"),
                    AggregateExpression::FunctionCall {
                        function: AggregateFunction::Avg,
                        expression: Box::new(Expression::Variable(Variable::new("val"))),
                        distinct: false,
                    },
                ),
                (
                    Variable::new("mx"),
                    AggregateExpression::FunctionCall {
                        function: AggregateFunction::Max,
                        expression: Box::new(Expression::Variable(Variable::new("val"))),
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
