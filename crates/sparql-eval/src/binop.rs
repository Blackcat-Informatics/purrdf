// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Binary graph-pattern operators: `Join` and `Union` (multiset semantics).
//!
//! Both produce a result over the **ordered union** of the operand schemas (left
//! columns first), and both preserve multiset cardinality — duplicate solutions
//! are kept (no implicit `DISTINCT`).
//!
//! `Join` is a hash join on the shared variables. The wrinkle is **unbound shared
//! columns**: a solution may leave a shared variable unbound (`None`), which is
//! compatible with any value (SPARQL §17.5 / §18.2.2). A pure hash-on-key join is
//! correct only when every shared column is bound, so the build side is split into
//! a key-indexed set (all shared columns bound) and a `wild` list (≥1 shared column
//! unbound), and a probe row that itself has an unbound shared column falls back to
//! a compatibility scan over all build rows. The common case — two fully-bound BGPs
//! — stays an O(n+m) hash join.

use std::sync::Arc;

use purrdf_sparql_algebra::{Expression, GraphPattern};

use crate::error::EvalError;
use crate::eval::{eval, EvalCtx};
use crate::scratch::SolutionTerm;
use crate::solution::{compatible, Solution, SolutionSeq, VarSchema};
use crate::{DetHashMap, DetHasher};

/// A hash-join key over the shared columns of two solutions.
///
/// The overwhelmingly common case is a **single** shared variable (a star join on
/// `?p`), so that case is specialized to a `Copy` `u64` ([`SolutionTerm::join_key_u64`])
/// — no per-row heap allocation on either the build or probe side. Joins on zero or
/// ≥2 shared columns keep the general owned-vector key. Within one join the shared
/// column count is fixed, so every key is the same variant and the two never compare
/// across variants.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum JoinKey {
    /// Exactly one shared column: the term encoded as a collision-free `u64`.
    Single(u64),
    /// Zero or ≥2 shared columns: the bound terms in shared-column order.
    Multi(Vec<SolutionTerm>),
}

/// Evaluate `left . right` (algebra `Join`) as a hash join on shared variables.
pub(crate) fn eval_join(
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let l = eval(left, ctx)?;
    let r = eval(right, ctx)?;
    Ok(hash_join(&l, &r))
}

/// Evaluate `left UNION right` as a multiset concatenation over the union schema.
pub(crate) fn eval_union(
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let l = eval(left, ctx)?;
    let r = eval(right, ctx)?;

    let out = l.schema.union(&r.schema);
    let out_len = out.len();
    let left_len = l.schema.len();
    let right_to_out = right_to_out_map(&r.schema, &out);

    let mut rows = Vec::with_capacity(l.rows.len() + r.rows.len());
    for lrow in &l.rows {
        // Left columns are out[0..left_len] in order; pad the rest with None.
        let mut row = vec![None; out_len];
        row[..left_len].copy_from_slice(lrow);
        rows.push(row);
    }
    for rrow in &r.rows {
        let mut row = vec![None; out_len];
        for (j, &cell) in rrow.iter().enumerate() {
            row[right_to_out[j]] = cell;
        }
        rows.push(row);
    }

    Ok(SolutionSeq {
        schema: Arc::new(out),
        rows,
    })
}

/// The mapping from a right operand's column ordinal to its ordinal in `out`.
fn right_to_out_map(right: &VarSchema, out: &VarSchema) -> Vec<usize> {
    right
        .vars()
        .iter()
        .map(|v| {
            out.index_of(v)
                .expect("union schema contains every right variable")
        })
        .collect()
}

/// Build the right-side join index: rows whose shared columns are all bound are
/// grouped by their key; rows with an unbound shared column are returned separately
/// (`wild`), since they are compatible with any probe value on that column.
///
/// Exposed `pub(crate)` so an `EXISTS` site can build the index over its inner
/// result **once** and reuse it across every outer row (see [`probe_has_match`]),
/// rather than rebuilding it per probe.
pub(crate) fn build_index(
    r: &SolutionSeq,
    shared: &[(usize, usize)],
) -> (DetHashMap<JoinKey, Vec<usize>>, Vec<usize>) {
    // Pre-size to the build-row count: the exact upper bound on distinct keys, so a
    // large build side is filled without incremental rehash-and-reallocate churn.
    let mut keyed: DetHashMap<JoinKey, Vec<usize>> =
        DetHashMap::with_capacity_and_hasher(r.rows.len(), DetHasher::default());
    let mut wild: Vec<usize> = Vec::new();
    for (idx, rrow) in r.rows.iter().enumerate() {
        match bound_key(rrow, shared, KeySide::Right) {
            Some(key) => keyed.entry(key).or_default().push(idx),
            None => wild.push(idx),
        }
    }
    (keyed, wild)
}

/// Existence-only probe against a **prebuilt** right-side index: whether any row of
/// `r_rows` is join-compatible with `probe` on the `shared` columns, without
/// materializing the join. This is the `EXISTS` primitive — it short-circuits on the
/// first match and reuses the index ([`build_index`]) built once per `EXISTS` site, so
/// a `FILTER (NOT) EXISTS` over N outer rows is O(N) probes, not N index rebuilds.
///
/// `keyed`/`wild`/`r_rows` must come from the same `build_index(r, shared)` call.
///
/// Cliff note: when `probe` has an **unbound** shared column, no exact key exists, so
/// this falls back to a per-row compatibility scan over the full inner result — that
/// case is O(|inner|) per probe. A probe fully bound on its shared columns (the common
/// anti-join shape) hits the keyed bucket in O(1).
pub(crate) fn probe_has_match(
    probe: &[Option<SolutionTerm>],
    shared: &[(usize, usize)],
    keyed: &DetHashMap<JoinKey, Vec<usize>>,
    wild: &[usize],
    r_rows: &[Solution],
) -> bool {
    match bound_key(probe, shared, KeySide::Left) {
        // Fully bound on shared columns: a present exact-key bucket is a match
        // (`build_index` only inserts non-empty buckets via `or_default().push`),
        // else any compatible wild build row (its `None` shared column matches).
        Some(key) => {
            keyed.contains_key(&key) || wild.iter().any(|&i| compatible(probe, &r_rows[i], shared))
        }
        // Unbound shared column ⇒ wildcard probe: scan for any compatible build row.
        None => r_rows.iter().any(|rrow| compatible(probe, rrow, shared)),
    }
}

/// Hash-join two solution sequences on their shared variables.
fn hash_join(l: &SolutionSeq, r: &SolutionSeq) -> SolutionSeq {
    let out = l.schema.union(&r.schema);
    let out_len = out.len();
    let left_len = l.schema.len();
    let right_to_out = right_to_out_map(&r.schema, &out);
    // Shared columns as (left_ordinal, right_ordinal) pairs, in left order.
    let shared = l.schema.shared_columns(&r.schema);

    // Build side = right (split into key-indexed + wild rows).
    let (keyed, wild) = build_index(r, &shared);

    // Each left row's worker returns its merged matches in the same order as the
    // sequential path (keyed-bucket matches in `idxs` order, then wild matches; or,
    // for the unbound-shared-column case, the compatibility scan over `r.rows` in
    // order); flattening across rows in index order reproduces the exact sequential
    // row sequence. Captures only read-only borrows: `keyed`/`wild`/`r.rows` (the
    // prebuilt index), `right_to_out`/`shared` (pure layout), `left_len`/`out_len`
    // (`Copy`), and `merge`/`compatible` (pure fns).
    let rows = crate::parallel::par_flat_map(&l.rows, |_, lrow| {
        let mut out_rows = Vec::new();
        match bound_key(lrow, &shared, KeySide::Left) {
            // Probe is fully bound on shared columns: hit the matching bucket
            // (exact key ⇒ compatible) plus any wild build rows it is compatible
            // with (a wild row's None shared column matches anything).
            Some(key) => {
                if let Some(idxs) = keyed.get(&key) {
                    for &idx in idxs {
                        out_rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                    }
                }
                for &idx in &wild {
                    if compatible(lrow, &r.rows[idx], &shared) {
                        out_rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                    }
                }
            }
            // Probe has an unbound shared column: it can match any build row, so
            // fall back to a compatibility scan over all of them.
            None => {
                for rrow in &r.rows {
                    if compatible(lrow, rrow, &shared) {
                        out_rows.push(merge(lrow, rrow, left_len, &right_to_out, out_len));
                    }
                }
            }
        }
        out_rows
    });

    SolutionSeq {
        schema: Arc::new(out),
        rows,
    }
}

/// Which side's ordinal a shared-column pair addresses.
#[derive(Clone, Copy)]
enum KeySide {
    Left,
    Right,
}

/// The shared-column key of `row`, or `None` if any shared column is unbound.
///
/// Both sides build the key in the same `shared` order, so a left key equals a
/// right key iff the two rows agree on every (bound) shared column. A single shared
/// column — the common star-join shape — produces an allocation-free
/// [`JoinKey::Single`]; zero or ≥2 columns fall back to an owned [`JoinKey::Multi`].
fn bound_key(
    row: &[Option<SolutionTerm>],
    shared: &[(usize, usize)],
    side: KeySide,
) -> Option<JoinKey> {
    let col_of = |ia: usize, ib: usize| match side {
        KeySide::Left => ia,
        KeySide::Right => ib,
    };
    if let [(ia, ib)] = *shared {
        // Single shared column: no heap allocation for the key.
        return Some(JoinKey::Single(row[col_of(ia, ib)]?.join_key_u64()));
    }
    let mut key = Vec::with_capacity(shared.len());
    for &(ia, ib) in shared {
        key.push(row[col_of(ia, ib)]?);
    }
    Some(JoinKey::Multi(key))
}

/// Merge a compatible `(left_row, right_row)` pair into one solution over the output
/// layout. Left columns occupy `out[0..left_len]`; each right column fills its
/// output slot only if still unbound, so a shared column unbound on the left is
/// filled from the right (and an already-bound shared column — equal by
/// compatibility — is left intact).
fn merge(
    left_row: &Solution,
    right_row: &Solution,
    left_len: usize,
    right_to_out: &[usize],
    out_len: usize,
) -> Solution {
    debug_assert_eq!(left_row.len(), left_len);
    // One exact-size allocation, initialized from the left row directly (no
    // write-None-then-overwrite pass over the left prefix).
    let mut merged = Vec::with_capacity(out_len);
    merged.extend_from_slice(left_row);
    merged.resize(out_len, None);
    for (j, &cell) in right_row.iter().enumerate() {
        let p = right_to_out[j];
        if merged[p].is_none() {
            merged[p] = cell;
        }
    }
    merged
}

/// Evaluate `left OPTIONAL { right }` (algebra `LeftJoin`) as a left outer join,
/// with an optional inline `FILTER` condition evaluated on the merged solution.
pub(crate) fn eval_left_join(
    left: &GraphPattern,
    right: &GraphPattern,
    expression: Option<&Expression>,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let l = eval(left, ctx)?;
    let r = eval(right, ctx)?;
    match expression {
        None => Ok(left_outer_join(&l, &r)),
        Some(expr) => left_outer_join_filtered(&l, &r, expr, ctx),
    }
}

/// A left outer join whose right-side pairings must additionally satisfy `expr`
/// (the inline `OPTIONAL { ... FILTER expr }` condition, §18.6). A left solution
/// with no pairing that is both compatible and passes the filter is emitted alone.
fn left_outer_join_filtered(
    l: &SolutionSeq,
    r: &SolutionSeq,
    expr: &Expression,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let out = Arc::new(l.schema.union(&r.schema));
    let out_len = out.len();
    let left_len = l.schema.len();
    let right_to_out = right_to_out_map(&r.schema, &out);
    let shared = l.schema.shared_columns(&r.schema);

    // A left outer join emits at least one row per left row.
    let mut rows = Vec::with_capacity(l.rows.len());
    for lrow in &l.rows {
        let mut matched = false;
        for rrow in &r.rows {
            if !compatible(lrow, rrow, &shared) {
                continue;
            }
            let merged = merge(lrow, rrow, left_len, &right_to_out, out_len);
            if crate::expr::eval_ebv(expr, &merged, &out, ctx)? == Some(true) {
                rows.push(merged);
                matched = true;
            }
        }
        if !matched {
            let mut row = vec![None; out_len];
            row[..left_len].copy_from_slice(lrow);
            rows.push(row);
        }
    }
    Ok(SolutionSeq { schema: out, rows })
}

/// Left outer join: every left solution merged with each compatible right
/// solution, or emitted alone (right columns unbound) when none is compatible.
fn left_outer_join(l: &SolutionSeq, r: &SolutionSeq) -> SolutionSeq {
    let out = l.schema.union(&r.schema);
    let out_len = out.len();
    let left_len = l.schema.len();
    let right_to_out = right_to_out_map(&r.schema, &out);
    let shared = l.schema.shared_columns(&r.schema);

    let (keyed, wild) = build_index(r, &shared);

    // A left outer join emits at least one row per left row. Each worker returns the
    // matched merges (keyed then wild, same order as the sequential path) or, when
    // none match, the single padded left-alone row — reproducing the existing "emit
    // alone iff no match" per-row semantics inside the worker so flattening in index
    // order is byte-identical to the sequential path.
    let rows = crate::parallel::par_flat_map(&l.rows, |_, lrow| {
        let mut out_rows = Vec::new();
        match bound_key(lrow, &shared, KeySide::Left) {
            Some(key) => {
                if let Some(idxs) = keyed.get(&key) {
                    for &idx in idxs {
                        out_rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                    }
                }
                for &idx in &wild {
                    if compatible(lrow, &r.rows[idx], &shared) {
                        out_rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                    }
                }
            }
            None => {
                for rrow in &r.rows {
                    if compatible(lrow, rrow, &shared) {
                        out_rows.push(merge(lrow, rrow, left_len, &right_to_out, out_len));
                    }
                }
            }
        }
        // No compatible right solution → keep the left solution alone (the OPTIONAL
        // contributed nothing, its variables stay unbound).
        if out_rows.is_empty() {
            let mut row = vec![None; out_len];
            row[..left_len].copy_from_slice(lrow);
            out_rows.push(row);
        }
        out_rows
    });

    SolutionSeq {
        schema: Arc::new(out),
        rows,
    }
}

/// Evaluate `left MINUS { right }` (algebra `Minus`).
///
/// A left solution is removed iff some right solution is **both** compatible **and**
/// shares at least one actually-bound variable (the domain-intersection guard,
/// SPARQL §18.5): solutions with disjoint domains never remove, so `MINUS` over
/// patterns with no common variable is a no-op. The result schema is the left
/// schema (MINUS introduces no right columns) and left multiplicity is preserved.
pub(crate) fn eval_minus(
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let l = eval(left, ctx)?;
    let r = eval(right, ctx)?;
    let shared = l.schema.shared_columns(&r.schema);

    let rows = crate::parallel::par_retain(&l.rows, |lrow| {
        // Keep the left row unless some right row removes it.
        !r.rows.iter().any(|rrow| {
            compatible(lrow, rrow, &shared)
                && shared
                    .iter()
                    .any(|&(la, ra)| lrow[la].is_some() && rrow[ra].is_some())
        })
    });

    Ok(SolutionSeq {
        schema: l.schema,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EvalCtx;
    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder};
    use purrdf_sparql_algebra::{
        NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
    };

    fn graph() -> Arc<RdfDataset> {
        // :a :knows :b ; :likes :cake .
        // :b :likes :tea .
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let likes = b.intern_iri("http://ex/likes");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let cake = b.intern_iri("http://ex/cake");
        let tea = b.intern_iri("http://ex/tea");
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, likes, cake, None);
        b.push_quad(bb, likes, tea, None);
        b.freeze().expect("freeze")
    }

    fn vp(n: &str) -> TermPattern {
        TermPattern::Variable(Variable::new(n))
    }
    fn pred(iri: &str) -> NamedNodePattern {
        NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri))
    }
    fn bgp(s: TermPattern, p: NamedNodePattern, o: TermPattern) -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: s,
                predicate: p,
                object: o,
            }],
        }
    }

    fn render(ds: &RdfDataset, seq: &SolutionSeq, vars: &[&str]) -> Vec<Vec<Option<String>>> {
        let scratch = crate::scratch::ScratchInterner::new();
        let cols: Vec<usize> = vars
            .iter()
            .map(|v| seq.schema.index_of(&Variable::new(*v)).expect("var"))
            .collect();
        let mut out: Vec<Vec<Option<String>>> = seq
            .rows
            .iter()
            .map(|row| {
                cols.iter()
                    .map(|&c| {
                        row[c].map(|t| match scratch.value_of(ds, t) {
                            purrdf_core::TermValue::Iri(s) => s,
                            other => format!("{other:?}"),
                        })
                    })
                    .collect()
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn join_on_shared_variable() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?x :knows ?y } JOIN { ?y :likes ?z }
        let left = bgp(vp("x"), pred("http://ex/knows"), vp("y"));
        let right = bgp(vp("y"), pred("http://ex/likes"), vp("z"));
        let seq = eval_join(&left, &right, &mut ctx).expect("join");
        // a knows b; b likes tea → (x=a, y=b, z=tea).
        assert_eq!(
            render(&ds, &seq, &["x", "y", "z"]),
            vec![vec![
                Some("http://ex/a".to_owned()),
                Some("http://ex/b".to_owned()),
                Some("http://ex/tea".to_owned()),
            ]]
        );
    }

    #[test]
    fn join_with_no_shared_vars_is_cross_product() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?x :knows ?y } JOIN { ?p :likes ?q } — disjoint vars → cross product.
        let left = bgp(vp("x"), pred("http://ex/knows"), vp("y")); // 1 row
        let right = bgp(vp("p"), pred("http://ex/likes"), vp("q")); // 2 rows
        let seq = eval_join(&left, &right, &mut ctx).expect("join");
        assert_eq!(seq.len(), 2); // 1 × 2.
    }

    #[test]
    fn join_with_no_overlap_is_empty() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?y :likes ?z } JOIN { ?y :knows ?w } — y=b likes tea, but b knows
        // nothing; y=a likes cake, a knows b. Shared y: a(likes cake)+a(knows b).
        let left = bgp(vp("y"), pred("http://ex/likes"), vp("z")); // y∈{a,b}
        let right = bgp(vp("y"), pred("http://ex/knows"), vp("w")); // y∈{a}
        let seq = eval_join(&left, &right, &mut ctx).expect("join");
        // Only y=a survives: (y=a, z=cake, w=b).
        assert_eq!(
            render(&ds, &seq, &["y", "z", "w"]),
            vec![vec![
                Some("http://ex/a".to_owned()),
                Some("http://ex/cake".to_owned()),
                Some("http://ex/b".to_owned()),
            ]]
        );
    }

    #[test]
    fn union_concatenates_preserving_multiset() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :knows ?o } UNION { ?s :likes ?o }  → 1 + 2 = 3 rows.
        let left = bgp(vp("s"), pred("http://ex/knows"), vp("o"));
        let right = bgp(vp("s"), pred("http://ex/likes"), vp("o"));
        let seq = eval_union(&left, &right, &mut ctx).expect("union");
        assert_eq!(seq.len(), 3);
        // Same var names on both sides → schema is exactly [s, o].
        assert_eq!(seq.schema.vars(), &[Variable::new("s"), Variable::new("o")]);
    }

    #[test]
    fn union_of_disjoint_schemas_widens_and_pads() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?a :knows ?b } UNION { ?c :likes ?d } → schema [a,b,c,d]; each row binds
        // only its own side's two columns, the other two are None.
        let left = bgp(vp("a"), pred("http://ex/knows"), vp("b")); // 1
        let right = bgp(vp("c"), pred("http://ex/likes"), vp("d")); // 2
        let seq = eval_union(&left, &right, &mut ctx).expect("union");
        assert_eq!(seq.len(), 3);
        assert_eq!(
            seq.schema.vars(),
            &[
                Variable::new("a"),
                Variable::new("b"),
                Variable::new("c"),
                Variable::new("d"),
            ]
        );
        // The left row has c,d unbound; a right row has a,b unbound.
        let left_rows = seq.rows.iter().filter(|r| r[0].is_some()).count();
        let right_rows = seq.rows.iter().filter(|r| r[2].is_some()).count();
        assert_eq!((left_rows, right_rows), (1, 2));
    }

    #[test]
    fn optional_keeps_unmatched_left_with_unbound_right() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :likes ?o } OPTIONAL { ?s :knows ?f }
        // s∈{a,b}: a knows b (match), b knows nothing (unmatched → ?f unbound).
        let left = bgp(vp("s"), pred("http://ex/likes"), vp("o"));
        let right = bgp(vp("s"), pred("http://ex/knows"), vp("f"));
        let seq = eval_left_join(&left, &right, None, &mut ctx).expect("optional");
        assert_eq!(seq.len(), 2);
        let f = seq.schema.index_of(&Variable::new("f")).unwrap();
        // Exactly one row leaves ?f bound (s=a) and one leaves it unbound (s=b).
        let bound = seq.rows.iter().filter(|r| r[f].is_some()).count();
        assert_eq!(bound, 1);
    }

    #[test]
    fn optional_inline_filter_excludes_failing_pairings() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :likes ?o } OPTIONAL { ?s :knows ?f } with an always-false condition
        // sameTerm(?s, ?f): no pairing passes, so every left row is emitted alone
        // (?f unbound), exercising the filtered left-outer path.
        let left = bgp(vp("s"), pred("http://ex/likes"), vp("o"));
        let right = bgp(vp("s"), pred("http://ex/knows"), vp("f"));
        let cond = Some(Expression::SameTerm(
            Box::new(Expression::Variable(Variable::new("s"))),
            Box::new(Expression::Variable(Variable::new("f"))),
        ));
        let seq =
            eval_left_join(&left, &right, cond.as_ref(), &mut ctx).expect("filtered optional");
        assert_eq!(seq.len(), 2);
        let f = seq.schema.index_of(&Variable::new("f")).unwrap();
        assert!(seq.rows.iter().all(|r| r[f].is_none()));
    }

    #[test]
    fn minus_removes_compatible_overlapping_rows() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :likes ?o } MINUS { ?s :knows ?f }
        // s∈{a,b} on the left; s=a is also a knows-subject (compatible + shares ?s),
        // so the a-row is removed, leaving only s=b.
        let left = bgp(vp("s"), pred("http://ex/likes"), vp("o"));
        let right = bgp(vp("s"), pred("http://ex/knows"), vp("f"));
        let seq = eval_minus(&left, &right, &mut ctx).expect("minus");
        assert_eq!(
            render(&ds, &seq, &["s", "o"]),
            vec![vec![
                Some("http://ex/b".to_owned()),
                Some("http://ex/tea".to_owned()),
            ]]
        );
        // Result schema is the left schema (no right columns introduced).
        assert_eq!(seq.schema.vars(), &[Variable::new("s"), Variable::new("o")]);
    }

    #[test]
    fn probe_has_match_hits_index_then_scans_wild() {
        use crate::scratch::SolutionTerm;
        use purrdf_core::TermId;
        let t = |i: u32| Some(SolutionTerm::Existing(TermId::from_index(i)));

        // Inner over schema [x]: x=1, x=2, and one wild row (x unbound).
        let inner = SolutionSeq {
            schema: Arc::new(VarSchema::from_vars([Variable::new("x")])),
            rows: vec![vec![t(1)], vec![t(2)], vec![None]],
        };
        // Probe layout is the FULL outer schema [x, y]; shared = {x} → [(0, 0)].
        let outer = VarSchema::from_vars([Variable::new("x"), Variable::new("y")]);
        let shared = outer.shared_columns(&inner.schema);
        assert_eq!(shared, vec![(0, 0)]);
        let (keyed, wild) = build_index(&inner, &shared);
        assert_eq!(wild.len(), 1, "the x-unbound inner row is wild");

        // Bound probe x=1: exact keyed bucket → match.
        assert!(probe_has_match(
            &[t(1), None],
            &shared,
            &keyed,
            &wild,
            &inner.rows
        ));
        // Bound probe x=9: no keyed bucket, but the wild inner row matches anything.
        assert!(probe_has_match(
            &[t(9), None],
            &shared,
            &keyed,
            &wild,
            &inner.rows
        ));
        // Unbound probe (x = None): wildcard → scan branch finds a compatible row.
        assert!(probe_has_match(
            &[None, t(5)],
            &shared,
            &keyed,
            &wild,
            &inner.rows
        ));

        // Same shape but NO wild inner row, so a keyed miss is a true non-match.
        let inner2 = SolutionSeq {
            schema: Arc::new(VarSchema::from_vars([Variable::new("x")])),
            rows: vec![vec![t(1)], vec![t(2)]],
        };
        let (keyed2, wild2) = build_index(&inner2, &shared);
        assert!(wild2.is_empty());
        assert!(
            !probe_has_match(&[t(9), None], &shared, &keyed2, &wild2, &inner2.rows),
            "bound probe with no keyed bucket and no wild row does not match"
        );
        // Unbound probe scans a non-empty inner → match; an empty inner → no match.
        assert!(probe_has_match(
            &[None, t(5)],
            &shared,
            &keyed2,
            &wild2,
            &inner2.rows
        ));
        let empty = SolutionSeq {
            schema: Arc::new(VarSchema::from_vars([Variable::new("x")])),
            rows: vec![],
        };
        let (ek, ew) = build_index(&empty, &shared);
        assert!(!probe_has_match(
            &[None, t(5)],
            &shared,
            &ek,
            &ew,
            &empty.rows
        ));
    }

    #[test]
    fn minus_with_disjoint_domains_removes_nothing() {
        let ds = graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :likes ?o } MINUS { ?x :knows ?y } — no shared variable, so the
        // domain-intersection guard keeps every left row (the classic MINUS trap).
        let left = bgp(vp("s"), pred("http://ex/likes"), vp("o")); // 2 rows
        let right = bgp(vp("x"), pred("http://ex/knows"), vp("y")); // 1 row
        let seq = eval_minus(&left, &right, &mut ctx).expect("minus");
        assert_eq!(seq.len(), 2);
    }
}
