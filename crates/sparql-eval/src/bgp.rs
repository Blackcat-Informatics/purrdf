// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Basic-graph-pattern (BGP) evaluation — the TermId hot path.
//!
//! A BGP is a conjunction of triple patterns. Evaluation stays entirely in
//! interned [`TermId`](purrdf_core::TermId) space:
//!
//! 1. **Compile** each triple pattern's three positions to either a [`Pos::Slot`]
//!    (a variable column) or a [`Pos::Bound`] (a ground constant resolved once via
//!    `term_id_by_value`, the P4 reverse index). If a ground constant is absent from
//!    the dataset the whole BGP is empty — that constant cannot match.
//! 2. **Order** the patterns cheapest-first with a cost-based join planner
//!    ([`cost_based_order`]): probe each pattern's real cardinality through the P4
//!    lazy permutation index and search join orders (exhaustive left-deep DP for a
//!    small BGP, greedy beyond) to minimise the estimated total intermediate
//!    cardinality, keeping the join connected.
//! 3. **Index-nested-loop join** in that order; for each partial solution, substitute
//!    its already-bound variables into the next pattern's positions and call the
//!    indexed P4 `quads_for_pattern`, then extend. Repeated variables (`?x p ?x`) and
//!    previously-bound variables are enforced at bind time.
//!
//! ## Blank nodes are non-distinguished variables
//!
//! A blank node in a query BGP (`_:b`) is *not* a request to match a specific
//! dataset blank by label — it is an anonymous variable that matches any term and
//! co-refers (by label) **only within this BGP** (SPARQL §4.1.4 / §18.2.1). So a
//! blank position compiles to a synthetic slot variable whose name carries a `NUL`
//! prefix (which the SPARQL grammar can never produce, so it cannot collide with a
//! real `?var`). After the BGP is evaluated these synthetic columns are
//! **projected away**, so two independent BGPs that happen to reuse the label `_:b`
//! never accidentally share a join variable.

use purrdf_core::{DatasetView, GraphMatch, QuadIds, QuadProbePlan, RdfDataset, TermId, TermRef};
use purrdf_sparql_algebra::{NamedNodePattern, TermPattern, TriplePattern, Variable};

use crate::convert::{ground_term_pattern_to_value, named_node_to_value};
use crate::dataset_spec::GraphScope;
use crate::error::EvalError;
use crate::eval::{BgpOrderCache, EvalCtx};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
use crate::DetHashSet;
use std::rc::Rc;
use std::sync::Arc;

/// The `rdf:reifies` predicate IRI — the indirection edge of the RDF 1.2 reification
/// layer. A triple pattern whose predicate is bound to this IRI (and whose object is a
/// quoted-triple pattern) draws candidates from the dataset's reifier side-table via
/// [`RdfDataset::reifier_quads`], which is invisible to the `quads` table.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// The `NUL`-prefixed marker that distinguishes a synthetic blank-node slot
/// variable from a real, projectable SPARQL variable.
const BLANK_VAR_PREFIX: char = '\u{0}';

/// A compiled triple-pattern position.
enum Pos {
    /// A variable (or blank-node) column index into the working schema.
    Slot(usize),
    /// A ground constant resolved to its dataset id.
    Bound(TermId),
    /// A nested RDF 1.2 quoted-triple pattern `<<( s p o )>>` that contains at least
    /// one variable (a fully-ground quoted triple resolves to a single [`Pos::Bound`]
    /// id instead). Binding descends into the candidate row's triple-term value,
    /// unifying the inner positions and enforcing repeated-variable consistency.
    Triple(Box<TriplePos>),
}

/// A compiled nested quoted-triple position: its three component positions, each
/// itself a [`Pos`] (so quoted triples may nest, and any component may be a variable,
/// a ground constant, or a further nested triple).
struct TriplePos {
    s: Pos,
    p: Pos,
    o: Pos,
}

/// One compiled triple pattern: its three positions in `(s, p, o)` order.
struct CompiledPattern {
    s: Pos,
    p: Pos,
    o: Pos,
}

/// Evaluate a basic graph pattern to a multiset of solutions over its real
/// (non-blank) variables.
pub(crate) fn eval_bgp(
    patterns: &[TriplePattern],
    ctx: &EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    // The empty BGP is the identity table Z: one solution binding nothing.
    if patterns.is_empty() {
        return Ok(SolutionSeq::unit());
    }

    // Pass 1: collect every slot variable (real + synthetic blank) in first-seen
    // (subject, predicate, object) order — the working column layout.
    let mut working = VarSchema::new();
    for pattern in patterns {
        for key in slot_keys(pattern) {
            working.push(key);
        }
    }

    // Pass 2: compile each pattern; a ground constant absent from the dataset makes
    // the whole BGP empty.
    let mut compiled = Vec::with_capacity(patterns.len());
    for pattern in patterns {
        match compile_pattern(pattern, &working, ctx.dataset)? {
            Some(cp) => compiled.push(cp),
            None => return Ok(empty_over_real_vars(&working)),
        }
    }

    // The graph scope for this BGP (resolved once — `active_graph` is fixed across a
    // single BGP; `GRAPH` wrapping is applied by `eval_graph` before recursing in).
    // Needed before planning: a pattern's cardinality is scope-dependent.
    let scope = ctx.active_dataset.scope_for(ctx.active_graph);

    // Reorder the patterns cheapest-first with the cost-based planner (memoised by the
    // engine's dataset-aware order cache when present). This is a pure permutation of a
    // commutative join: `Pos::Slot` is an absolute column index into `working`, so
    // reordering cannot change which columns bind — the multiset result is identical,
    // only the join shape (and so the cost) changes.
    let order = plan_or_cached_order(&compiled, ctx.dataset, &scope, ctx.bgp_order_cache);

    // The interned id of `rdf:reifies`, resolved once. `None` ⇒ the dataset has no
    // reifier layer at all (the predicate was never interned), so no virtual reifier
    // candidates exist for any pattern.
    let reifies_id = ctx
        .dataset
        .term_id_by_value(&purrdf_core::TermValue::Iri(RDF_REIFIES.to_owned()));

    // Index-nested-loop evaluation. Rows start as a single all-unbound solution.
    let mut rows: Vec<Solution> = vec![vec![None; working.len()]];
    for &i in order.iter() {
        let cp = &compiled[i];
        let mut next = Vec::new();
        // The probe's bound-axis shape is fixed across this slot's rows (a variable is
        // bound by an earlier pattern for every row or for none), so the permutation
        // choice is loop-invariant: compute the `QuadProbePlan` once (on the first
        // single-graph probe) and reuse it, instead of re-selecting per row.
        let mut probe_plan: Option<QuadProbePlan> = None;
        for row in &rows {
            let s = query_id(&cp.s, row);
            let p = query_id(&cp.p, row);
            let o = query_id(&cp.o, row);
            match &scope {
                // Single-graph scope (store default / a named graph): the indexed
                // partition_point read, unchanged — no de-dup overhead.
                GraphScope::One(gm) => {
                    let plan = *probe_plan.get_or_insert_with(|| {
                        RdfDataset::probe_plan(s.is_some(), p.is_some(), o.is_some(), *gm)
                    });
                    for quad in ctx.dataset.quads_for_pattern_with_plan(&plan, s, p, o, *gm) {
                        if let Some(extended) = bind_row(row, cp, &quad, ctx.dataset) {
                            next.push(extended);
                        }
                    }
                    // The RDF 1.2 reification layer is a dataset-level (default-graph)
                    // side-table outside `quads`, so fold its virtual triples in here
                    // — additively (no double counting) — whenever this scope includes
                    // the default graph. A `GRAPH ?g`/named scope (`gm` matching only a
                    // named graph) never sees it, matching the store-default treatment.
                    if gm.matches(None) {
                        emit_virtual_candidates(ctx.dataset, cp, s, p, o, reifies_id, |quad| {
                            if let Some(extended) = bind_row(row, cp, &quad, ctx.dataset) {
                                next.push(extended);
                            }
                        });
                    }
                }
                // A FROM/USING-merged default graph: union the per-graph reads, but
                // RDF-merge unions *triples*, so a triple present in two merged graphs
                // must bind once — de-dupe by (s, p, o) for this pattern+row. The
                // reification layer is store-default content (not part of an explicitly
                // FROM-named merge), so it is not folded into a merged scope.
                GraphScope::Merge(gs) => {
                    let mut seen: DetHashSet<(TermId, TermId, TermId)> = DetHashSet::default();
                    for &g in gs {
                        for quad in ctx.dataset.quads_for_pattern(s, p, o, GraphMatch::Named(g)) {
                            if !seen.insert((quad.s, quad.p, quad.o)) {
                                continue;
                            }
                            if let Some(extended) = bind_row(row, cp, &quad, ctx.dataset) {
                                next.push(extended);
                            }
                        }
                    }
                }
            }
        }
        rows = next;
        if rows.is_empty() {
            break;
        }
    }

    Ok(project_out_blanks(&working, rows))
}

/// The BGP-size ceiling for exhaustive join-order search. At or below this many
/// patterns the planner runs a left-deep Selinger DP over all `2^n` subsets
/// (`n ≤ 8 ⇒ ≤ 256` states — trivial); above it, a greedy minimum-cardinality walk.
/// Both minimise the same estimated-intermediate-cardinality cost.
const COST_DP_MAX_PATTERNS: usize = 8;

/// Return the join order for `compiled`, served from the engine's dataset-aware cache
/// when one is present, planning + inserting on a miss. Without a cache (a directly
/// built [`EvalCtx`], e.g. a unit test) every BGP is planned afresh — identical result,
/// just not memoised. The cache key is `(dataset stats fingerprint, BGP shape key)`; on
/// a hit the cached order's length is asserted against `compiled` so a (vanishingly
/// unlikely) shape-key collision re-plans rather than indexing out of bounds.
fn plan_or_cached_order(
    compiled: &[CompiledPattern],
    dataset: &RdfDataset,
    scope: &GraphScope,
    cache: Option<&BgpOrderCache>,
) -> Arc<[usize]> {
    let Some(cache) = cache else {
        return Arc::from(cost_based_order(compiled, dataset, scope));
    };
    let key = (dataset.stats_fingerprint(), bgp_shape_key(compiled, scope));
    if let Some(order) = cache.borrow().get(&key) {
        // A shape-key collision is NOT licensed by the stats-fingerprint safety
        // argument: a wrong-length order would index out of bounds in the join loop.
        // Guard it — on a length mismatch fall through and re-plan.
        if order.len() == compiled.len() {
            return Arc::clone(order);
        }
    }
    let order: Arc<[usize]> = Arc::from(cost_based_order(compiled, dataset, scope));
    cache.borrow_mut().insert(key, Arc::clone(&order));
    order
}

/// A deterministic hash of a BGP's *shape* — its pattern count, every position's
/// structure (slot column / bound id / nested quoted-triple), and the graph scope — for
/// the order cache key. Encodes `compiled.len()` and the full positional structure so two
/// structurally distinct BGPs cannot collide to one cached order, and folds in the scope
/// because a pattern's cardinality (hence its best order) is scope-dependent.
fn bgp_shape_key(compiled: &[CompiledPattern], scope: &GraphScope) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    compiled.len().hash(&mut h);
    for cp in compiled {
        hash_pos(&cp.s, &mut h);
        hash_pos(&cp.p, &mut h);
        hash_pos(&cp.o, &mut h);
    }
    match scope {
        GraphScope::One(gm) => {
            0u8.hash(&mut h);
            match gm {
                GraphMatch::Any => 0u8.hash(&mut h),
                GraphMatch::Default => 1u8.hash(&mut h),
                GraphMatch::Named(id) => {
                    2u8.hash(&mut h);
                    id.index().hash(&mut h);
                }
            }
        }
        GraphScope::Merge(gs) => {
            1u8.hash(&mut h);
            gs.len().hash(&mut h);
            for g in gs {
                g.index().hash(&mut h);
            }
        }
    }
    h.finish()
}

/// Hash one compiled position structurally (tag + payload), descending into nested
/// quoted triples — a helper for [`bgp_shape_key`].
fn hash_pos<H: std::hash::Hasher>(pos: &Pos, h: &mut H) {
    use std::hash::Hash;
    match pos {
        Pos::Slot(c) => {
            0u8.hash(h);
            c.hash(h);
        }
        Pos::Bound(id) => {
            1u8.hash(h);
            id.index().hash(h);
        }
        Pos::Triple(t) => {
            2u8.hash(h);
            hash_pos(&t.s, h);
            hash_pos(&t.p, h);
            hash_pos(&t.o, h);
        }
    }
}

/// Order compiled BGP patterns cheapest-first with a cost-based join planner — the
/// native `sparopt` role. Unlike a structural heuristic, this probes the dataset's
/// real per-pattern cardinalities (the P4 lazy permutation index, via
/// [`RdfDataset::cardinality_estimate`]) and searches join orders to minimise the
/// estimated total intermediate cardinality.
///
/// Cost model (left-deep, uniform-independence): a pattern's base size is its
/// constants-only cardinality `|p|`; appending one that shares `j` already-bound
/// positions multiplies the running estimate by `|p| / T^j`, where `T` is the
/// distinct-term count (the standard `1/T` equality-join selectivity). An order's
/// cost is the sum of the running estimates (the Selinger proxy); lower is better.
/// The connectivity rule is preserved — a pattern is only scheduled once it shares a
/// bound variable with the prefix (no accidental Cartesian product) unless no
/// connected pattern remains.
///
/// Evaluation order is COMPUTED here, never asserted or materialised as triples
/// (Principle 12). The reorder is a permutation of a commutative join (`Pos::Slot` is
/// an absolute column index), so it preserves the result *multiset* exactly — a worse
/// order is only slower, never wrong. It does not preserve the observable row
/// *sequence* of a `SELECT` without `ORDER BY`, which is spec-permitted (SPARQL §11
/// leaves solution order unspecified absent `ORDER BY`), so any golden over an
/// un-`ORDER BY`-ed query must be order-tolerant. Determinism: cardinality probes are
/// pure, the cost arithmetic is order-stable `f64` (compared via `total_cmp`), and
/// ties break on the lexicographically smallest order (lowest original index first) —
/// identical run to run, no hash-iteration leak.
///
/// Returns a permutation of `0..compiled.len()`.
fn cost_based_order(
    compiled: &[CompiledPattern],
    dataset: &RdfDataset,
    scope: &GraphScope,
) -> Vec<usize> {
    let n = compiled.len();
    if n <= 1 {
        return (0..n).collect();
    }

    // Per-pattern base cardinality (constants only — slots/quoted-triples are free),
    // the exact stat a structural heuristic ignores. `f64` so the multiplicative join
    // selectivities never truncate to zero mid-estimate.
    let base: Vec<f64> = compiled
        .iter()
        .map(|cp| base_cardinality(dataset, cp, scope) as f64)
        .collect();
    // Distinct-term count as the equality-join domain size (`1/T` per join axis).
    let t = dataset.term_count().max(1) as f64;

    // The dense bound-mask width: the highest slot column across all patterns. A
    // position may be a nested triple, so descend through it to find every slot.
    let mut n_cols = 0usize;
    for cp in compiled {
        for pos in [&cp.s, &cp.p, &cp.o] {
            for_each_slot(pos, &mut |c| n_cols = n_cols.max(c + 1));
        }
    }

    if n <= COST_DP_MAX_PATTERNS {
        cost_order_dp(compiled, &base, t, n_cols)
    } else {
        cost_order_greedy(compiled, &base, t, n_cols)
    }
}

/// The constants-only cardinality of a compiled pattern under `scope`: pass each
/// ground position through, leave slots and quoted-triple positions free. A
/// quoted-triple position is treated as unconstrained — a conservative over-estimate,
/// safe by the multiset invariant. For a FROM/USING merge, sum the per-graph estimates.
fn base_cardinality(dataset: &RdfDataset, cp: &CompiledPattern, scope: &GraphScope) -> usize {
    let s = constant_of(&cp.s);
    let p = constant_of(&cp.p);
    let o = constant_of(&cp.o);
    match scope {
        GraphScope::One(gm) => dataset.cardinality_estimate(s, p, o, *gm),
        GraphScope::Merge(gs) => gs
            .iter()
            .map(|&g| dataset.cardinality_estimate(s, p, o, GraphMatch::Named(g)))
            .sum(),
    }
}

/// The ground id of a position, or `None` for a slot or (conservatively) a nested
/// quoted-triple position.
fn constant_of(pos: &Pos) -> Option<TermId> {
    match pos {
        Pos::Bound(id) => Some(*id),
        Pos::Slot(_) | Pos::Triple(_) => None,
    }
}

/// How many of a pattern's three top-level positions are already bound by an
/// earlier-scheduled pattern — each is an equality-join axis (selectivity ~`1/T`).
/// Ground constants are excluded: their selectivity is already folded into the
/// pattern's base cardinality.
fn join_positions(cp: &CompiledPattern, bound: &[bool]) -> usize {
    [&cp.s, &cp.p, &cp.o]
        .into_iter()
        .filter(|pos| pos_has_bound_slot(pos, bound))
        .count()
}

/// The running intermediate-size estimate after appending a pattern: scale by its
/// base size, divide by `T` for each already-bound join axis.
fn step_size(running: f64, base_p: f64, joins: usize, t: f64) -> f64 {
    running * base_p / t.powi(joins as i32)
}

/// Greedy minimum-cardinality join order for a large BGP (`n > COST_DP_MAX_PATTERNS`):
/// repeatedly schedule the connected pattern whose appended intermediate-size estimate
/// is smallest, lowest-index on ties. Connectivity is enforced exactly as in the DP.
fn cost_order_greedy(
    compiled: &[CompiledPattern],
    base: &[f64],
    t: f64,
    n_cols: usize,
) -> Vec<usize> {
    let n = compiled.len();
    let mut bound = vec![false; n_cols];
    let mut scheduled = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut running = 1.0f64;

    for _ in 0..n {
        // While a connected pattern remains, only such patterns are eligible — never
        // force a Cartesian product. (Round 1: nothing bound, nothing connected, so
        // every pattern is eligible and the lowest-cardinality one seeds the join.)
        let any_connected =
            (0..n).any(|i| !scheduled[i] && pattern_connected(&compiled[i], &bound));
        let mut best: Option<usize> = None;
        let mut best_size = f64::INFINITY;
        for i in 0..n {
            if scheduled[i] {
                continue;
            }
            if any_connected && !pattern_connected(&compiled[i], &bound) {
                continue;
            }
            let joins = join_positions(&compiled[i], &bound);
            let size = step_size(running, base[i], joins, t);
            // Strict `<` over an index-order scan ⇒ lowest original index wins ties.
            if best.is_none() || size < best_size {
                best = Some(i);
                best_size = size;
            }
        }
        let chosen = best.expect("an unscheduled pattern always remains");
        scheduled[chosen] = true;
        mark_bound(&compiled[chosen], &mut bound);
        running = best_size;
        order.push(chosen);
    }
    order
}

/// One left-deep plan in the subset DP: its accumulated cost (sum of intermediate
/// sizes), the running size of its last stage, and the pattern order encoded as a
/// nibble-packed `u64`.
///
/// The order is stored as a sequence of 4-bit nibbles packed into `order_bits`, with
/// the first-scheduled pattern index occupying the most-significant occupied nibble.
/// Each nibble stores `index + 1` (1-based) so that index 0 is distinguishable from
/// the empty zero-padding in less-significant positions. Appending pattern index `i`
/// shifts left by 4 and OR-s in `i + 1`:
///
/// ```text
/// order_bits = (order_bits << 4) | (i as u64 + 1)
/// len += 1
/// ```
///
/// This struct is `Copy` — no heap allocation per DP transition. The maximum supported
/// pattern count is [`COST_DP_MAX_PATTERNS`] = 8, so indices 0–7 fit in a single nibble
/// (values 1–8), and 8 nibbles occupy exactly 32 bits of the 64-bit word — well within
/// capacity. An index ≥ 16 would overflow a nibble; this is statically prevented by the
/// `COST_DP_MAX_PATTERNS ≤ 15` invariant documented on that constant.
///
/// Tie-breaking: when two candidate plans for the same `next` mask have equal `cost`,
/// the winner is the one with the smaller `order_bits`. Because both candidates have
/// identical `len` (same number of bits set in `next`), their packed words are the same
/// width, so a simple `u64` comparison reads them left-to-right — exactly equivalent to
/// the lexicographic `Vec<usize>` comparison it replaces.
#[derive(Clone, Copy)]
struct DpPlan {
    cost: f64,
    size: f64,
    /// Nibble-packed join order: MSB nibble = first scheduled pattern (1-based index).
    order_bits: u64,
    /// Number of patterns scheduled so far (= number of occupied nibbles).
    len: u8,
}

/// Exhaustive left-deep Selinger DP for a small BGP (`n ≤ COST_DP_MAX_PATTERNS`):
/// `dp[mask]` is the minimum-cost connected plan covering exactly the patterns in
/// `mask`. Transitions append one connected pattern (or, when none is connected, any —
/// a forced cross product for a genuinely disconnected BGP). Ties break on the
/// lexicographically smallest order (lowest original index first), so the result is
/// deterministic. The `2^n` state table is a dense `Vec` indexed by the subset
/// bitmask — never a hash map, so no iteration-order nondeterminism can leak in.
///
/// The order is carried as a nibble-packed `u64` (see [`DpPlan`]): each DP transition
/// copies the `Copy` struct and shifts in one nibble — no heap allocation per step.
/// The final order is decoded MSB→LSB into a `Vec<usize>` for the caller.
fn cost_order_dp(compiled: &[CompiledPattern], base: &[f64], t: f64, n_cols: usize) -> Vec<usize> {
    let n = compiled.len();
    // Safety invariant: each pattern index must fit in a 4-bit nibble (values 1–15
    // after the 1-based offset). COST_DP_MAX_PATTERNS == 8 satisfies this with margin.
    debug_assert!(
        n <= COST_DP_MAX_PATTERNS,
        "cost_order_dp called with n={n} > COST_DP_MAX_PATTERNS={COST_DP_MAX_PATTERNS}"
    );
    const {
        assert!(
            COST_DP_MAX_PATTERNS <= 15,
            "COST_DP_MAX_PATTERNS must be ≤ 15 so every index fits in a 4-bit nibble"
        );
    };

    let full: usize = (1usize << n) - 1;
    let mut dp: Vec<Option<DpPlan>> = vec![None; full + 1];
    dp[0] = Some(DpPlan {
        cost: 0.0,
        size: 1.0,
        order_bits: 0,
        len: 0,
    });

    // Masks ascend, and every transition sets one more bit (a strictly larger mask),
    // so `dp[mask]` is final by the time the loop reaches it.
    for mask in 0..=full {
        let Some(plan) = dp[mask] else {
            continue;
        };
        // The slots bound after this prefix (the union of the set's slots).
        // Decode order_bits MSB→LSB to recover the scheduled indices.
        let mut bound = vec![false; n_cols];
        for k in 0..plan.len {
            let nibble_pos = plan.len - 1 - k; // 0 = least-significant occupied nibble
            let idx = ((plan.order_bits >> (4 * nibble_pos)) & 0xF) as usize - 1;
            mark_bound(&compiled[idx], &mut bound);
        }
        let any_connected = mask != 0
            && (0..n).any(|i| mask & (1usize << i) == 0 && pattern_connected(&compiled[i], &bound));

        for i in 0..n {
            if mask & (1usize << i) != 0 {
                continue;
            }
            // Seed (mask == 0) is free; afterwards prefer a connected pattern while one
            // exists (no Cartesian product unless the BGP is genuinely disconnected).
            if any_connected && !pattern_connected(&compiled[i], &bound) {
                continue;
            }
            let joins = if mask == 0 {
                0
            } else {
                join_positions(&compiled[i], &bound)
            };
            let size = step_size(plan.size, base[i], joins, t);
            let cost = plan.cost + size;
            // Append pattern index `i` as a new LSB nibble (1-based so index 0 ≠ empty).
            let order_bits = (plan.order_bits << 4) | (i as u64 + 1);
            let len = plan.len + 1;
            let next = mask | (1usize << i);
            let better = match &dp[next] {
                None => true,
                // `total_cmp` is a deterministic total order (and avoids comparing
                // floats with `==`); ties fall through to the nibble-packed order
                // comparison. Both candidates have the same `len` (identical popcount
                // of `next`), so their packed words are the same width and `u64`
                // comparison reads them left-to-right — exactly lexicographic order
                // on the pattern-index sequence, lowest-index first.
                Some(cur) => match cost.total_cmp(&cur.cost) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Greater => false,
                    std::cmp::Ordering::Equal => order_bits < cur.order_bits,
                },
            };
            if better {
                dp[next] = Some(DpPlan {
                    cost,
                    size,
                    order_bits,
                    len,
                });
            }
        }
    }

    let best = dp[full].expect("the DP always reaches the full set");
    // Decode MSB→LSB: the first-scheduled pattern is in the most-significant nibble.
    (0..best.len)
        .map(|k| {
            let nibble_pos = best.len - 1 - k;
            ((best.order_bits >> (4 * nibble_pos)) & 0xF) as usize - 1
        })
        .collect()
}

/// Whether a pattern shares at least one already-bound variable with the bindings
/// produced so far (so joining it cannot be a Cartesian product). Descends into nested
/// quoted triples: a triple position is connected if any of its inner slots is bound.
fn pattern_connected(cp: &CompiledPattern, bound: &[bool]) -> bool {
    [&cp.s, &cp.p, &cp.o]
        .into_iter()
        .any(|pos| pos_has_bound_slot(pos, bound))
}

/// Whether a position contains an already-bound slot anywhere (recursively).
fn pos_has_bound_slot(pos: &Pos, bound: &[bool]) -> bool {
    match pos {
        Pos::Bound(_) => false,
        Pos::Slot(c) => bound[*c],
        Pos::Triple(t) => [&t.s, &t.p, &t.o]
            .into_iter()
            .any(|p| pos_has_bound_slot(p, bound)),
    }
}

/// Record a scheduled pattern's slot columns as now-bound (descending into nested
/// quoted triples).
fn mark_bound(cp: &CompiledPattern, bound: &mut [bool]) {
    for pos in [&cp.s, &cp.p, &cp.o] {
        for_each_slot(pos, &mut |c| bound[c] = true);
    }
}

/// Visit every slot column reachable from a position (itself, or the inner positions
/// of a nested quoted triple).
fn for_each_slot(pos: &Pos, f: &mut impl FnMut(usize)) {
    match pos {
        Pos::Bound(_) => {}
        Pos::Slot(c) => f(*c),
        Pos::Triple(t) => {
            for inner in [&t.s, &t.p, &t.o] {
                for_each_slot(inner, f);
            }
        }
    }
}

/// The slot variables a triple pattern introduces, in `(s, p, o)` order — descending
/// into any nested quoted-triple position so its inner variables become columns too. A
/// ground position yields nothing; a blank node yields a synthetic slot variable.
fn slot_keys(pattern: &TriplePattern) -> Vec<Variable> {
    let mut keys = Vec::new();
    collect_triple_slot_keys(pattern, &mut keys);
    keys
}

/// Append a triple pattern's slot variables (recursively through nested quoted
/// triples) in `(s, p, o)` order.
fn collect_triple_slot_keys(pattern: &TriplePattern, keys: &mut Vec<Variable>) {
    collect_term_slot_keys(&pattern.subject, keys);
    if let NamedNodePattern::Variable(v) = &pattern.predicate {
        keys.push(v.clone());
    }
    collect_term_slot_keys(&pattern.object, keys);
}

/// Append a term position's slot variables: a real variable, a synthetic blank-node
/// variable, or — for a quoted triple — its inner variables (recursively). Ground
/// terms yield nothing.
fn collect_term_slot_keys(term: &TermPattern, keys: &mut Vec<Variable>) {
    match term {
        TermPattern::Variable(v) => keys.push(v.clone()),
        TermPattern::BlankNode(b) => keys.push(blank_var(b.as_str())),
        TermPattern::Triple(t) => collect_triple_slot_keys(t, keys),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => {}
    }
}

/// The synthetic slot variable for a blank-node label (NUL-prefixed; cannot collide
/// with a parser-produced `?var`).
fn blank_var(label: &str) -> Variable {
    Variable::new(format!("{BLANK_VAR_PREFIX}bnode:{label}"))
}

/// Whether a schema variable is a synthetic blank-node slot (vs. a real variable).
fn is_blank_var(var: &Variable) -> bool {
    var.as_str().starts_with(BLANK_VAR_PREFIX)
}

/// Compile a triple pattern's positions. Returns `Ok(None)` if a ground constant is
/// absent from the dataset (the pattern — and hence the BGP — cannot match).
fn compile_pattern(
    pattern: &TriplePattern,
    schema: &VarSchema,
    dataset: &RdfDataset,
) -> Result<Option<CompiledPattern>, EvalError> {
    let Some(s) = compile_term(&pattern.subject, schema, dataset)? else {
        return Ok(None);
    };
    let Some(p) = compile_predicate(&pattern.predicate, schema, dataset) else {
        return Ok(None);
    };
    let Some(o) = compile_term(&pattern.object, schema, dataset)? else {
        return Ok(None);
    };
    Ok(Some(CompiledPattern { s, p, o }))
}

/// Compile a subject/object term position. `Ok(None)` = an absent ground constant
/// (the pattern — and hence the BGP — cannot match).
fn compile_term(
    term: &TermPattern,
    schema: &VarSchema,
    dataset: &RdfDataset,
) -> Result<Option<Pos>, EvalError> {
    match term {
        TermPattern::Variable(v) => Ok(Some(Pos::Slot(slot_col(schema, v)))),
        TermPattern::BlankNode(b) => Ok(Some(Pos::Slot(slot_col(schema, &blank_var(b.as_str()))))),
        // A quoted-triple position: if it contains a variable it is a STRUCTURAL match
        // that binds inner columns (`Pos::Triple`); a fully-ground quoted triple
        // resolves to a single interned id (`Pos::Bound`) exactly like any constant.
        TermPattern::Triple(t) => {
            if triple_has_variable(t) {
                match compile_triple_pos(t, schema, dataset)? {
                    Some(tp) => Ok(Some(Pos::Triple(Box::new(tp)))),
                    None => Ok(None),
                }
            } else {
                let value = ground_term_pattern_to_value(term)?;
                Ok(dataset.term_id_by_value(&value).map(Pos::Bound))
            }
        }
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => {
            let value = ground_term_pattern_to_value(term)?;
            Ok(dataset.term_id_by_value(&value).map(Pos::Bound))
        }
    }
}

/// Compile a nested quoted-triple pattern's three positions. `Ok(None)` if any
/// ground component is absent from the dataset (so the whole pattern cannot match).
fn compile_triple_pos(
    triple: &TriplePattern,
    schema: &VarSchema,
    dataset: &RdfDataset,
) -> Result<Option<TriplePos>, EvalError> {
    let Some(s) = compile_term(&triple.subject, schema, dataset)? else {
        return Ok(None);
    };
    let Some(p) = compile_predicate(&triple.predicate, schema, dataset) else {
        return Ok(None);
    };
    let Some(o) = compile_term(&triple.object, schema, dataset)? else {
        return Ok(None);
    };
    Ok(Some(TriplePos { s, p, o }))
}

/// The working-schema column of a slot variable (registered in pass 1).
fn slot_col(schema: &VarSchema, var: &Variable) -> usize {
    schema
        .index_of(var)
        .expect("every slot key was registered in pass 1")
}

/// Whether a quoted-triple pattern contains at least one variable anywhere (including
/// nested quoted triples). A variable-free quoted triple is a ground constant.
fn triple_has_variable(triple: &TriplePattern) -> bool {
    term_has_variable(&triple.subject)
        || matches!(triple.predicate, NamedNodePattern::Variable(_))
        || term_has_variable(&triple.object)
}

/// Whether a term position contains a variable (recursively through quoted triples).
/// A blank node is a non-distinguished variable, so it counts.
fn term_has_variable(term: &TermPattern) -> bool {
    match term {
        TermPattern::Variable(_) | TermPattern::BlankNode(_) => true,
        TermPattern::Triple(t) => triple_has_variable(t),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => false,
    }
}

/// Compile a predicate position (IRI or variable). `None` = an absent ground IRI.
fn compile_predicate(
    predicate: &NamedNodePattern,
    schema: &VarSchema,
    dataset: &RdfDataset,
) -> Option<Pos> {
    match predicate {
        NamedNodePattern::Variable(v) => Some(Pos::Slot(
            schema
                .index_of(v)
                .expect("every slot key was registered in pass 1"),
        )),
        NamedNodePattern::NamedNode(n) => dataset
            .term_id_by_value(&named_node_to_value(n))
            .map(Pos::Bound),
    }
}

/// The id to query a position with, given the current partial solution: a bound
/// constant, an already-bound variable's id, or `None` (a wildcard / a variable not
/// yet bound). A `Computed` binding (never produced inside a BGP) degrades to a
/// wildcard and is rejected by [`bind_row`].
fn query_id(pos: &Pos, row: &Solution) -> Option<TermId> {
    match pos {
        Pos::Bound(id) => Some(*id),
        Pos::Slot(col) => match row[*col] {
            Some(SolutionTerm::Existing(id)) => Some(id),
            _ => None,
        },
        // A structural quoted-triple position is not addressable as a single id probe
        // key for the candidate scan; it degrades to a wildcard and is unified
        // structurally in `bind_row` (which descends into the candidate's triple term).
        Pos::Triple(_) => None,
    }
}

/// Try to extend `row` by binding `cp`'s positions from `quad`. Returns `None` if a
/// repeated or previously-bound variable disagrees with the quad, if a nested
/// quoted-triple position fails to unify, or if a ground constant disagrees (the
/// virtual reification candidates are NOT pre-filtered by `quads_for_pattern`, so a
/// `Pos::Bound` mismatch must be rejected here).
fn bind_row(
    row: &Solution,
    cp: &CompiledPattern,
    quad: &QuadIds,
    dataset: &RdfDataset,
) -> Option<Solution> {
    let mut out = row.clone();
    for (pos, id) in [(&cp.s, quad.s), (&cp.p, quad.p), (&cp.o, quad.o)] {
        if !bind_pos(&mut out, pos, id, dataset) {
            return None;
        }
    }
    Some(out)
}

/// Unify one compiled position against a candidate term id, mutating `out` with any
/// newly bound slots. Returns `false` (caller rejects the row) on any disagreement:
/// - a `Pos::Bound` constant that does not equal the candidate id;
/// - a `Pos::Slot` repeated/previously-bound variable that disagrees;
/// - a `Pos::Triple` whose candidate id is not a triple term, or whose components fail
///   to unify recursively.
fn bind_pos(out: &mut Solution, pos: &Pos, id: TermId, dataset: &RdfDataset) -> bool {
    match pos {
        Pos::Bound(want) => *want == id,
        Pos::Slot(col) => {
            let value = SolutionTerm::Existing(id);
            match out[*col] {
                Some(existing) => existing == value,
                None => {
                    out[*col] = Some(value);
                    true
                }
            }
        }
        Pos::Triple(t) => match dataset.resolve(id) {
            TermRef::Triple { s, p, o } => {
                bind_pos(out, &t.s, s, dataset)
                    && bind_pos(out, &t.p, p, dataset)
                    && bind_pos(out, &t.o, o, dataset)
            }
            // The candidate term is not a quoted triple, so a structural triple pattern
            // cannot match it.
            _ => false,
        },
    }
}

/// Emit the virtual triple candidates from the RDF 1.2 reification layer that match
/// a pattern's bound `(s, p, o)` probe (reifier rows first, then annotation rows —
/// each in the side-tables' frozen sorted order). The layer is NOT in `quads`, so
/// these are strictly additive (no double counting).
///
/// Two layers contribute:
/// - **Reifier rows** `(reifier, rdf:reifies, triple-term)` — included only when the
///   pattern's predicate *can* be `rdf:reifies` (unbound, or bound exactly to it).
///   When the predicate is bound to some other IRI, no reifier row can match, so the
///   layer is skipped entirely.
/// - **Annotation rows** `(reifier, annPred, annObj)` — a reifier's statement
///   annotations look like ordinary triples whose subject is a reifier. When the
///   pattern's subject is bound, [`RdfDataset::annotations_of`] indexes straight to
///   that reifier's run; otherwise the whole annotation table is scanned.
///
/// Every candidate is residually filtered by the same id-equality the default scan
/// applies (`quads_for_pattern`), because — unlike `quads_for_pattern` — the virtual
/// side-table walks are not pre-narrowed by the probe. A callback keeps this hot path
/// lazy without boxing or allocating an intermediate candidate buffer.
fn emit_virtual_candidates(
    dataset: &RdfDataset,
    cp: &CompiledPattern,
    s: Option<TermId>,
    p: Option<TermId>,
    o: Option<TermId>,
    reifies_id: Option<TermId>,
    mut emit: impl FnMut(QuadIds),
) {
    // Reifier layer: only when the predicate can be `rdf:reifies`. The object must also
    // be triple-term-shaped to be worth scanning — a quoted-triple pattern position
    // (`Pos::Triple`), a quoted-triple constant (`Pos::Bound` of a triple id), or a
    // free variable (`Pos::Slot`). A literal/IRI object constant can never be a triple
    // term, so the reifier scan is skipped. The residual `bind_row` enforces the exact
    // object match.
    if let Some(reifies) = reifies_id {
        let predicate_can_reify = match &cp.p {
            Pos::Slot(_) => true,
            Pos::Bound(id) => *id == reifies,
            // A quoted triple is never a predicate position.
            Pos::Triple(_) => false,
        };
        if predicate_can_reify && object_can_be_triple_term(&cp.o, dataset) {
            for quad in dataset.reifier_quads().filter(move |q| {
                s.is_none_or(|id| q.s == id)
                    && p.is_none_or(|id| q.p == id)
                    && o.is_none_or(|id| q.o == id)
            }) {
                emit(quad);
            }
        }
    }

    // Annotation layer: index by the bound reifier subject when possible, else scan.
    match s {
        Some(reifier) => {
            for quad in dataset
                .annotations_of(reifier)
                .map(|(pred, obj)| QuadIds {
                    s: reifier,
                    p: pred,
                    o: obj,
                    g: None,
                })
                .filter(|q| p.is_none_or(|id| q.p == id) && o.is_none_or(|id| q.o == id))
            {
                emit(quad);
            }
        }
        None => {
            for quad in dataset
                .annotation_quads()
                .filter(|q| p.is_none_or(|id| q.p == id) && o.is_none_or(|id| q.o == id))
            {
                emit(quad);
            }
        }
    }
}

/// Whether an object position could resolve to a quoted-triple term (so the reifier
/// layer — whose object is always a triple term — is worth scanning for it). An IRI or
/// literal constant never is.
fn object_can_be_triple_term(pos: &Pos, dataset: &RdfDataset) -> bool {
    match pos {
        // A free variable or a structural quoted-triple pattern can match a triple term.
        Pos::Slot(_) | Pos::Triple(_) => true,
        // A bound constant is worth scanning only if the constant is itself a triple
        // term; an IRI/literal/blank object can never match a reifier row.
        Pos::Bound(id) => matches!(dataset.resolve(*id), TermRef::Triple { .. }),
    }
}

/// An empty solution sequence over only the real (non-blank) variables of `working`.
fn empty_over_real_vars(working: &VarSchema) -> SolutionSeq {
    let real = real_var_schema(working);
    SolutionSeq::empty(Rc::new(real))
}

/// The schema of `working` restricted to its real variables, in order.
fn real_var_schema(working: &VarSchema) -> VarSchema {
    VarSchema::from_vars(working.vars().iter().filter(|v| !is_blank_var(v)).cloned())
}

/// Project the working rows onto only the real variables, dropping the synthetic
/// blank-node columns (which are scoped to this BGP and must not leak into joins).
/// Multiset cardinality is preserved (no dedup).
fn project_out_blanks(working: &VarSchema, rows: Vec<Solution>) -> SolutionSeq {
    // The working columns that survive, in order.
    let keep: Vec<usize> = working
        .vars()
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (!is_blank_var(v)).then_some(i))
        .collect();

    // Fast path: no blank columns — reuse rows as-is.
    if keep.len() == working.len() {
        return SolutionSeq {
            schema: Rc::new(real_var_schema(working)),
            rows,
        };
    }

    let schema = Rc::new(real_var_schema(working));
    let projected = rows
        .into_iter()
        .map(|row| keep.iter().map(|&i| row[i]).collect())
        .collect();
    SolutionSeq {
        schema,
        rows: projected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::ScratchInterner;
    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral, TermValue};
    use purrdf_sparql_algebra::{Literal, NamedNode};
    use std::sync::Arc;

    /// A small graph:
    ///   :alice :knows :bob ; :name "Alice" .
    ///   :bob   :knows :carol .
    ///   :carol :knows :alice .
    fn social_graph() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let name = b.intern_iri("http://ex/name");
        let alice = b.intern_iri("http://ex/alice");
        let bob = b.intern_iri("http://ex/bob");
        let carol = b.intern_iri("http://ex/carol");
        let alice_name = b.intern_literal(RdfLiteral::simple("Alice"));
        b.push_quad(alice, knows, bob, None);
        b.push_quad(bob, knows, carol, None);
        b.push_quad(carol, knows, alice, None);
        b.push_quad(alice, name, alice_name, None);
        b.freeze().expect("freeze")
    }

    fn var_pos(name: &str) -> TermPattern {
        TermPattern::Variable(Variable::new(name))
    }

    fn iri_pos(iri: &str) -> TermPattern {
        TermPattern::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn pred(iri: &str) -> NamedNodePattern {
        NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn triple(s: TermPattern, p: NamedNodePattern, o: TermPattern) -> TriplePattern {
        TriplePattern {
            subject: s,
            predicate: p,
            object: o,
        }
    }

    /// Run a BGP over `ds` and materialize each row's bindings for the given
    /// variables as `TermValue`s, sorted for order-insensitive comparison.
    fn run(
        ds: &RdfDataset,
        patterns: &[TriplePattern],
        vars: &[&str],
    ) -> Vec<Vec<Option<TermValue>>> {
        let ctx = EvalCtx::new(ds);
        let seq = eval_bgp(patterns, &ctx).expect("bgp");
        let cols: Vec<usize> = vars
            .iter()
            .map(|v| {
                seq.schema
                    .index_of(&Variable::new(*v))
                    .expect("var present")
            })
            .collect();
        let scratch = ScratchInterner::new();
        let mut out: Vec<Vec<Option<TermValue>>> = seq
            .rows
            .iter()
            .map(|row| {
                cols.iter()
                    .map(|&c| row[c].map(|t| scratch.value_of(ds, t)))
                    .collect()
            })
            .collect();
        // TermValue is not Ord; sort by a stable Debug key for order-insensitive
        // comparison of the (unordered) solution multiset.
        out.sort_by_key(|row| format!("{row:?}"));
        out
    }

    fn iri_val(iri: &str) -> Option<TermValue> {
        Some(TermValue::Iri(iri.to_owned()))
    }

    #[test]
    fn single_pattern_one_variable() {
        let ds = social_graph();
        // SELECT ?o WHERE { :alice :knows ?o }
        let patterns = [triple(
            iri_pos("http://ex/alice"),
            pred("http://ex/knows"),
            var_pos("o"),
        )];
        let rows = run(&ds, &patterns, &["o"]);
        assert_eq!(rows, vec![vec![iri_val("http://ex/bob")]]);
    }

    #[test]
    fn single_pattern_two_variables_enumerates_all_quads() {
        let ds = social_graph();
        // { ?s :knows ?o }  → all three knows-edges.
        let patterns = [triple(var_pos("s"), pred("http://ex/knows"), var_pos("o"))];
        let rows = run(&ds, &patterns, &["s", "o"]);
        assert_eq!(
            rows,
            vec![
                vec![iri_val("http://ex/alice"), iri_val("http://ex/bob")],
                vec![iri_val("http://ex/bob"), iri_val("http://ex/carol")],
                vec![iri_val("http://ex/carol"), iri_val("http://ex/alice")],
            ]
        );
    }

    #[test]
    fn two_pattern_join_on_shared_variable() {
        let ds = social_graph();
        // { ?a :knows ?b . ?b :knows ?c }  — friends-of-friends.
        let patterns = [
            triple(var_pos("a"), pred("http://ex/knows"), var_pos("b")),
            triple(var_pos("b"), pred("http://ex/knows"), var_pos("c")),
        ];
        let rows = run(&ds, &patterns, &["a", "b", "c"]);
        assert_eq!(
            rows,
            vec![
                vec![
                    iri_val("http://ex/alice"),
                    iri_val("http://ex/bob"),
                    iri_val("http://ex/carol")
                ],
                vec![
                    iri_val("http://ex/bob"),
                    iri_val("http://ex/carol"),
                    iri_val("http://ex/alice")
                ],
                vec![
                    iri_val("http://ex/carol"),
                    iri_val("http://ex/alice"),
                    iri_val("http://ex/bob")
                ],
            ]
        );
    }

    #[test]
    fn absent_constant_yields_empty() {
        let ds = social_graph();
        // :nobody is not in the graph → the constant resolves to absent → empty.
        let patterns = [triple(
            iri_pos("http://ex/nobody"),
            pred("http://ex/knows"),
            var_pos("o"),
        )];
        let rows = run(&ds, &patterns, &["o"]);
        assert!(rows.is_empty());
    }

    #[test]
    fn repeated_variable_requires_self_loop() {
        // A graph with one genuine self-loop and one non-loop edge.
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/p");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        b.push_quad(x, p, x, None); // self-loop
        b.push_quad(x, p, y, None); // not a loop
        let ds = b.freeze().expect("freeze");

        // { ?v :p ?v } matches only the self-loop.
        let patterns = [triple(var_pos("v"), pred("http://ex/p"), var_pos("v"))];
        let rows = run(&ds, &patterns, &["v"]);
        assert_eq!(rows, vec![vec![iri_val("http://ex/x")]]);
    }

    #[test]
    fn literal_object_constant_matches() {
        let ds = social_graph();
        // { ?s :name "Alice" } → alice.
        let lit = TermPattern::Literal(Literal::new_simple("Alice"));
        let patterns = [triple(var_pos("s"), pred("http://ex/name"), lit)];
        let rows = run(&ds, &patterns, &["s"]);
        assert_eq!(rows, vec![vec![iri_val("http://ex/alice")]]);
    }

    #[test]
    fn blank_node_acts_as_a_variable_and_is_projected_out() {
        let ds = social_graph();
        // { _:b :knows ?o } — the blank is an anonymous variable; it matches every
        // knows-subject, and is NOT exposed as a column.
        let patterns = [triple(
            TermPattern::BlankNode(purrdf_sparql_algebra::BlankNode::new("b")),
            pred("http://ex/knows"),
            var_pos("o"),
        )];
        let ctx = EvalCtx::new(&ds);
        let seq = eval_bgp(&patterns, &ctx).expect("bgp");
        // Only ?o is a real column; the blank slot was projected away.
        assert_eq!(seq.schema.vars(), &[Variable::new("o")]);
        assert_eq!(seq.len(), 3); // three knows-edges, one row each.
    }

    // ---- RDF 1.2 reification layer -----------------------------------------

    /// A dataset with one quoted statement `:alice :age 42` reified by `:r1`, which
    /// carries two annotations:
    ///   :r1 rdf:reifies <<( :alice :age 42 )>> .
    ///   :r1 :confidence "high" .
    ///   :r1 :source     :census .
    /// The reified statement itself is NOT asserted as a plain quad (the only quads
    /// table content is one unrelated `:bob :age 7` triple), proving the layer is read
    /// from the side-tables, not from `quads`.
    fn reified_graph() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let age = b.intern_iri("http://ex/age");
        let alice = b.intern_iri("http://ex/alice");
        let bob = b.intern_iri("http://ex/bob");
        let forty_two = b.intern_literal(RdfLiteral::typed(
            "42",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        let seven = b.intern_literal(RdfLiteral::typed(
            "7",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        let statement = b.intern_triple(alice, age, forty_two);
        let r1 = b.intern_iri("http://ex/r1");
        let reifies = b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies");
        let confidence = b.intern_iri("http://ex/confidence");
        let source = b.intern_iri("http://ex/source");
        let high = b.intern_literal(RdfLiteral::simple("high"));
        let census = b.intern_iri("http://ex/census");

        // The interned `reifies` id is the virtual predicate; keep it referenced.
        let _ = reifies;
        // One unrelated asserted quad to prove the reified statement is NOT in `quads`.
        b.push_quad(bob, age, seven, None);
        b.push_reifier(r1, statement);
        b.push_annotation(r1, confidence, high);
        b.push_annotation(r1, source, census);
        b.freeze().expect("freeze")
    }

    fn int_val(lex: &str) -> Option<TermValue> {
        Some(TermValue::Literal {
            lexical_form: lex.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        })
    }

    fn str_val(lex: &str) -> Option<TermValue> {
        Some(TermValue::Literal {
            lexical_form: lex.to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        })
    }

    /// A predicate-position variable.
    fn pred_var(name: &str) -> NamedNodePattern {
        NamedNodePattern::Variable(Variable::new(name))
    }

    /// A nested quoted-triple object pattern `<<( s p o )>>`.
    fn triple_obj(s: TermPattern, p: NamedNodePattern, o: TermPattern) -> TermPattern {
        TermPattern::Triple(Box::new(TriplePattern {
            subject: s,
            predicate: p,
            object: o,
        }))
    }

    /// `?r rdf:reifies <<( ?s ?p ?o )>>` binds the reifier and the inner s/p/o from the
    /// reifier side-table — the reified statement is not in `quads`.
    #[test]
    fn reifies_pattern_binds_reifier_and_inner_variables() {
        let ds = reified_graph();
        let patterns = [triple(
            var_pos("r"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
            triple_obj(var_pos("s"), pred_var("p"), var_pos("o")),
        )];
        let rows = run(&ds, &patterns, &["r", "s", "p", "o"]);
        assert_eq!(
            rows,
            vec![vec![
                iri_val("http://ex/r1"),
                iri_val("http://ex/alice"),
                iri_val("http://ex/age"),
                int_val("42"),
            ]]
        );
    }

    /// `?r rdf:reifies <<( :alice :age ?o )>>` — partially-ground inner pattern still
    /// binds `?r` and the free inner `?o`, and the ground inner positions filter.
    #[test]
    fn reifies_pattern_with_partly_ground_inner() {
        let ds = reified_graph();
        let patterns = [triple(
            var_pos("r"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
            triple_obj(
                iri_pos("http://ex/alice"),
                pred("http://ex/age"),
                var_pos("o"),
            ),
        )];
        let rows = run(&ds, &patterns, &["r", "o"]);
        assert_eq!(rows, vec![vec![iri_val("http://ex/r1"), int_val("42")]]);
    }

    /// A non-matching ground inner position yields no rows (the statement is
    /// `:alice :age 42`, not `:alice :age 99`).
    #[test]
    fn reifies_pattern_inner_mismatch_is_empty() {
        let ds = reified_graph();
        let patterns = [triple(
            var_pos("r"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
            triple_obj(
                iri_pos("http://ex/alice"),
                pred("http://ex/age"),
                TermPattern::Literal(Literal::new_typed(
                    "99",
                    NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
                )),
            ),
        )];
        let rows = run(&ds, &patterns, &["r"]);
        assert!(rows.is_empty());
    }

    /// A fully-open pattern `?r ?ap ?av` enumerates EVERY triple visible to the BGP:
    /// the one asserted quad, the virtual `rdf:reifies` edge, and both annotation rows.
    /// The reification layer is fully folded into ordinary BGP matching.
    #[test]
    fn open_pattern_enumerates_assertions_reifies_edge_and_annotations() {
        let ds = reified_graph();
        let patterns = [triple(var_pos("r"), pred_var("ap"), var_pos("av"))];
        let rows = run(&ds, &patterns, &["r", "ap", "av"]);
        let reifies = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
        // Rows are sorted by their Debug string for order-insensitive comparison, so
        // the expected order follows that key: bob < r1/confidence < r1/reifies <
        // r1/source.
        assert_eq!(
            rows,
            vec![
                // The asserted plain quad.
                vec![
                    iri_val("http://ex/bob"),
                    iri_val("http://ex/age"),
                    int_val("7"),
                ],
                // Annotation rows of :r1 (confidence, source sort before reifies under
                // the Debug-string key: "http://ex/…" < "http://www.…").
                vec![
                    iri_val("http://ex/r1"),
                    iri_val("http://ex/confidence"),
                    str_val("high"),
                ],
                vec![
                    iri_val("http://ex/r1"),
                    iri_val("http://ex/source"),
                    iri_val("http://ex/census"),
                ],
                // The virtual rdf:reifies edge (object is the quoted statement).
                vec![
                    iri_val("http://ex/r1"),
                    iri_val(reifies),
                    Some(TermValue::Triple {
                        s: Box::new(TermValue::Iri("http://ex/alice".to_owned())),
                        p: Box::new(TermValue::Iri("http://ex/age".to_owned())),
                        o: Box::new(TermValue::Literal {
                            lexical_form: "42".to_owned(),
                            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                            language: None,
                            direction: None,
                        }),
                    }),
                ],
            ]
        );
    }

    /// An annotation pattern with a bound annotation predicate `?r :confidence ?v`
    /// binds only the annotation rows of that predicate (here, one).
    #[test]
    fn annotation_pattern_bound_predicate() {
        let ds = reified_graph();
        let patterns = [triple(
            var_pos("r"),
            pred("http://ex/confidence"),
            var_pos("v"),
        )];
        let rows = run(&ds, &patterns, &["r", "v"]);
        assert_eq!(rows, vec![vec![iri_val("http://ex/r1"), str_val("high")]]);
    }

    /// A bound-subject annotation pattern `:r1 :confidence ?v` indexes straight to the
    /// reifier's annotation run via `annotations_of`.
    #[test]
    fn annotation_pattern_bound_subject_indexes() {
        let ds = reified_graph();
        let patterns = [triple(
            iri_pos("http://ex/r1"),
            pred("http://ex/confidence"),
            var_pos("v"),
        )];
        let rows = run(&ds, &patterns, &["v"]);
        assert_eq!(rows, vec![vec![str_val("high")]]);
    }

    /// Joining the two layers: find the confidence of every age-statement reifier.
    /// `?r rdf:reifies <<( ?s :age ?age )>> . ?r :confidence ?c`
    #[test]
    fn join_reifier_to_its_annotation() {
        let ds = reified_graph();
        let patterns = [
            triple(
                var_pos("r"),
                pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
                triple_obj(var_pos("s"), pred("http://ex/age"), var_pos("age")),
            ),
            triple(var_pos("r"), pred("http://ex/confidence"), var_pos("c")),
        ];
        let rows = run(&ds, &patterns, &["s", "age", "c"]);
        assert_eq!(
            rows,
            vec![vec![
                iri_val("http://ex/alice"),
                int_val("42"),
                str_val("high"),
            ]]
        );
    }

    /// A repeated inner variable `<<( ?x :age ?x )>>` enforces consistency: the only
    /// reified statement is `:alice :age 42`, where subject ≠ object, so it is rejected.
    #[test]
    fn reifies_pattern_repeated_inner_variable_enforced() {
        let ds = reified_graph();
        let patterns = [triple(
            var_pos("r"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
            triple_obj(var_pos("x"), pred("http://ex/age"), var_pos("x")),
        )];
        let rows = run(&ds, &patterns, &["r", "x"]);
        assert!(rows.is_empty());
    }

    /// A dataset with no reifiers never interns `rdf:reifies`, so a reifies-pattern
    /// query returns empty without panicking (the `None` reifies-id branch).
    #[test]
    fn reifies_pattern_on_plain_graph_is_empty() {
        let ds = social_graph();
        let patterns = [triple(
            var_pos("r"),
            pred("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
            triple_obj(var_pos("s"), pred_var("p"), var_pos("o")),
        )];
        let rows = run(&ds, &patterns, &["r"]);
        assert!(rows.is_empty());
    }

    // ---- cost_based_order --------------------------------------------------

    fn cp(s: Pos, p: Pos, o: Pos) -> CompiledPattern {
        CompiledPattern { s, p, o }
    }

    /// Plan a hand-built BGP over `ds` (default-graph scope `Any`, so estimates equal
    /// the full per-pattern counts of these single-graph fixtures).
    fn plan(ds: &RdfDataset, compiled: &[CompiledPattern]) -> Vec<usize> {
        cost_based_order(compiled, ds, &GraphScope::One(GraphMatch::Any))
    }

    /// Replay an order through the cost model and return its total estimated cost (the
    /// sum of intermediate sizes) — the objective the planner minimises.
    fn order_cost(
        compiled: &[CompiledPattern],
        order: &[usize],
        base: &[f64],
        t: f64,
        n_cols: usize,
    ) -> f64 {
        let mut bound = vec![false; n_cols];
        let mut running = 1.0f64;
        let mut total = 0.0f64;
        for (k, &i) in order.iter().enumerate() {
            let joins = if k == 0 {
                0
            } else {
                join_positions(&compiled[i], &bound)
            };
            running = step_size(running, base[i], joins, t);
            total += running;
            mark_bound(&compiled[i], &mut bound);
        }
        total
    }

    /// Interned ids of a deliberately skewed graph: `:hot` links the hub to 20 leaves,
    /// `:mid` to 5, `:rare` to 1 — so the per-predicate cardinalities are 20 / 5 / 1.
    struct Skewed {
        ds: Arc<RdfDataset>,
        hot: TermId,
        mid: TermId,
        rare: TermId,
        hub: TermId,
    }

    fn skewed_graph() -> Skewed {
        let mut b = RdfDatasetBuilder::new();
        let hot = b.intern_iri("http://ex/hot");
        let mid = b.intern_iri("http://ex/mid");
        let rare = b.intern_iri("http://ex/rare");
        let hub = b.intern_iri("http://ex/hub");
        for i in 0..20 {
            let leaf = b.intern_iri(&format!("http://ex/hot{i}"));
            b.push_quad(hub, hot, leaf, None);
        }
        for i in 0..5 {
            let leaf = b.intern_iri(&format!("http://ex/mid{i}"));
            b.push_quad(hub, mid, leaf, None);
        }
        let only = b.intern_iri("http://ex/rare0");
        b.push_quad(hub, rare, only, None);
        Skewed {
            ds: b.freeze().expect("freeze"),
            hot,
            mid,
            rare,
            hub,
        }
    }

    /// Reordering the *source* order of a BGP never changes its result multiset:
    /// `Pos::Slot` is an absolute column index, so the join is commutative.
    #[test]
    fn reordering_source_patterns_preserves_results() {
        let ds = social_graph();
        // A 3-cycle: { ?a :knows ?b . ?b :knows ?c . ?c :knows ?a } → the 3 rotations.
        let p0 = triple(var_pos("a"), pred("http://ex/knows"), var_pos("b"));
        let p1 = triple(var_pos("b"), pred("http://ex/knows"), var_pos("c"));
        let p2 = triple(var_pos("c"), pred("http://ex/knows"), var_pos("a"));

        let forward = run(&ds, &[p0.clone(), p1.clone(), p2.clone()], &["a", "b", "c"]);
        let reversed = run(&ds, &[p2, p1, p0], &["a", "b", "c"]);

        assert_eq!(forward.len(), 3);
        assert_eq!(forward, reversed);
    }

    /// The core cost-based win: with all three patterns equally constrained
    /// *structurally* (one bound predicate each — the old heuristic would tie them and
    /// keep source order `[0, 1, 2]`), the planner orders by REAL cardinality, seeding
    /// with the lowest-cardinality predicate and ascending from there.
    #[test]
    fn cost_order_seeds_with_lowest_cardinality_pattern() {
        let g = skewed_graph();
        // All three share ?s (col 0). Cardinalities: hot 20, mid 5, rare 1.
        let p0 = cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1)); // 20
        let p1 = cp(Pos::Slot(0), Pos::Bound(g.mid), Pos::Slot(2)); // 5
        let p2 = cp(Pos::Slot(0), Pos::Bound(g.rare), Pos::Slot(3)); // 1
        assert_eq!(plan(&g.ds, &[p0, p1, p2]), vec![2, 1, 0]);
    }

    /// The no-cross-product invariant: once the seed binds a variable, a *connected*
    /// pattern is scheduled before a *disconnected* one even when the disconnected one
    /// has the lower base cardinality.
    #[test]
    fn connectivity_keeps_connected_before_disconnected() {
        let g = skewed_graph();
        // P0 seeds (rare, card 1) and binds ?b = col 0.
        let p0 = cp(Pos::Bound(g.hub), Pos::Bound(g.rare), Pos::Slot(0));
        // P1 (hot, card 20) is connected via ?b.
        let p1 = cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1));
        // P2 (mid, card 5) is disconnected — lower base than P1, but cut off.
        let p2 = cp(Pos::Slot(2), Pos::Bound(g.mid), Pos::Slot(3));

        let order = plan(&g.ds, &[p0, p1, p2]);
        assert_eq!(order, vec![0, 1, 2]);
        let pos_of = |i: usize| order.iter().position(|&x| x == i).unwrap();
        assert!(
            pos_of(1) < pos_of(2),
            "connected P1 must precede disconnected P2 (no Cartesian product)"
        );
    }

    /// A fully disconnected BGP still yields a complete, valid permutation
    /// (lowest-cardinality first), without panicking.
    #[test]
    fn disconnected_bgp_yields_a_valid_permutation() {
        let g = skewed_graph();
        let p0 = cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1)); // 20
        let p1 = cp(Pos::Slot(2), Pos::Bound(g.rare), Pos::Slot(3)); // 1, disconnected
        let order = plan(&g.ds, &[p0, p1]);
        assert_eq!(order, vec![1, 0]); // cheaper disconnected pattern first.
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1]); // a genuine permutation of 0..n.
    }

    /// The order is identical run to run (no hash-iteration nondeterminism): the cost
    /// arithmetic is order-stable and ties break on lowest index.
    #[test]
    fn order_is_deterministic() {
        let g = skewed_graph();
        let make = || {
            vec![
                cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1)),
                cp(Pos::Slot(0), Pos::Bound(g.mid), Pos::Slot(2)),
                cp(Pos::Slot(0), Pos::Bound(g.rare), Pos::Slot(3)),
            ]
        };
        assert_eq!(plan(&g.ds, &make()), plan(&g.ds, &make()));
    }

    /// Equal-cardinality patterns are broken by lowest original index (stable).
    #[test]
    fn ties_break_on_lowest_original_index() {
        let g = skewed_graph();
        // Two disconnected patterns, both `:mid` (card 5) → index 0 must lead.
        let p0 = cp(Pos::Slot(0), Pos::Bound(g.mid), Pos::Slot(1));
        let p1 = cp(Pos::Slot(2), Pos::Bound(g.mid), Pos::Slot(3));
        assert_eq!(plan(&g.ds, &[p0, p1]), vec![0, 1]);
    }

    /// An empty BGP plans to an empty order (the `n <= 1` fast path), and a single
    /// pattern plans to `[0]` — neither probes the dataset.
    #[test]
    fn trivial_bgps_plan_without_probing() {
        let g = skewed_graph();
        assert_eq!(plan(&g.ds, &[]), Vec::<usize>::new());
        let one = cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1));
        assert_eq!(plan(&g.ds, &[one]), vec![0]);
    }

    /// All-ground patterns contain no `Pos::Slot`, so `n_cols == 0` and the bound-mask
    /// is zero-length. `join_positions` and `mark_bound` must not index the empty mask.
    /// Both existing ground triples have cardinality 1, so the tie breaks on index.
    #[test]
    fn all_ground_bgp_orders_without_panicking() {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/p");
        let a = b.intern_iri("http://ex/a");
        let c = b.intern_iri("http://ex/c");
        b.push_quad(a, p, c, None);
        b.push_quad(c, p, a, None);
        let ds = b.freeze().expect("freeze");

        let p0 = cp(Pos::Bound(a), Pos::Bound(p), Pos::Bound(c)); // card 1
        let p1 = cp(Pos::Bound(c), Pos::Bound(p), Pos::Bound(a)); // card 1
        let order = plan(&ds, &[p0, p1]);
        assert_eq!(order, vec![0, 1]);
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1]);
    }

    /// A STAR shape: one shared hub variable (col 0) in every spoke. After the
    /// lowest-cardinality spoke seeds and binds the hub, every remaining spoke is
    /// connected (no Cartesian product) and they schedule in ascending cardinality.
    #[test]
    fn star_spokes_follow_hub_ordered_by_cardinality() {
        let g = skewed_graph();
        // P0 hot (20), P1 mid (5), P2 rare (1) — all share ?hub (col 0).
        let p0 = cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1));
        let p1 = cp(Pos::Slot(0), Pos::Bound(g.mid), Pos::Slot(2));
        let p2 = cp(Pos::Slot(0), Pos::Bound(g.rare), Pos::Slot(3));

        let order = plan(&g.ds, &[p0, p1, p2]);
        // Ascending cardinality: rare (idx 2), mid (idx 1), hot (idx 0).
        assert_eq!(order, vec![2, 1, 0]);
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2]); // valid permutation, every spoke present.
    }

    /// The left-deep DP is never worse than the greedy walk on the same BGP: greedy is
    /// itself a connected left-deep plan the DP also enumerates, so `dp_cost <=
    /// greedy_cost`. Exercised on a 4-pattern cyclic BGP (a–b–c–d–a) with skewed
    /// per-predicate cardinalities.
    #[test]
    fn dp_is_never_worse_than_greedy() {
        let g = skewed_graph();
        // cols: a=0, b=1, c=2, d=3.
        let compiled = [
            cp(Pos::Slot(0), Pos::Bound(g.hot), Pos::Slot(1)), // ?a :hot ?b  (20)
            cp(Pos::Slot(1), Pos::Bound(g.mid), Pos::Slot(2)), // ?b :mid ?c  (5)
            cp(Pos::Slot(2), Pos::Bound(g.rare), Pos::Slot(3)), // ?c :rare ?d (1)
            cp(Pos::Slot(0), Pos::Bound(g.mid), Pos::Slot(3)), // ?a :mid ?d  (5)
        ];
        let scope = GraphScope::One(GraphMatch::Any);
        let base: Vec<f64> = compiled
            .iter()
            .map(|c| base_cardinality(&g.ds, c, &scope) as f64)
            .collect();
        let t = g.ds.term_count().max(1) as f64;
        let mut n_cols = 0usize;
        for c in &compiled {
            for pos in [&c.s, &c.p, &c.o] {
                for_each_slot(pos, &mut |col| n_cols = n_cols.max(col + 1));
            }
        }

        let dp = cost_order_dp(&compiled, &base, t, n_cols);
        let greedy = cost_order_greedy(&compiled, &base, t, n_cols);

        // Both are valid permutations of 0..4.
        let mut dp_sorted = dp.clone();
        dp_sorted.sort_unstable();
        assert_eq!(dp_sorted, vec![0, 1, 2, 3]);

        let dp_cost = order_cost(&compiled, &dp, &base, t, n_cols);
        let greedy_cost = order_cost(&compiled, &greedy, &base, t, n_cols);
        assert!(
            dp_cost <= greedy_cost + 1e-9,
            "DP cost {dp_cost} must not exceed greedy cost {greedy_cost}"
        );
    }

    /// Above the DP ceiling the planner switches to the greedy walk and still returns a
    /// valid permutation. A connected chain one pattern longer than the DP ceiling
    /// (?v0 :hot ?v1 … so the greedy branch is taken).
    #[test]
    fn large_bgp_uses_greedy_and_returns_valid_permutation() {
        let g = skewed_graph();
        let n = COST_DP_MAX_PATTERNS + 1; // strictly above the ceiling ⇒ greedy branch.
        let compiled: Vec<CompiledPattern> = (0..n)
            .map(|i| cp(Pos::Slot(i), Pos::Bound(g.hot), Pos::Slot(i + 1)))
            .collect();
        let order = plan(&g.ds, &compiled);
        let mut sorted = order;
        sorted.sort_unstable();
        assert_eq!(sorted, (0..n).collect::<Vec<_>>());
    }

    // ---- measurable win vs the retired structural heuristic -------------------

    /// The retired most-constrained-first STRUCTURAL heuristic, reproduced here ONLY as
    /// the baseline the cost planner must beat: schedule greedily by the count of bound
    /// positions, keep the join connected, break ties on lowest index. (This is the S7
    /// behaviour `cost_based_order` replaced.)
    fn structural_order(compiled: &[CompiledPattern]) -> Vec<usize> {
        fn constrained(pos: &Pos, bound: &[bool]) -> bool {
            match pos {
                Pos::Bound(_) => true,
                Pos::Slot(c) => bound[*c],
                Pos::Triple(t) => {
                    constrained(&t.s, bound) && constrained(&t.p, bound) && constrained(&t.o, bound)
                }
            }
        }
        let n = compiled.len();
        let mut n_cols = 0usize;
        for cp in compiled {
            for pos in [&cp.s, &cp.p, &cp.o] {
                for_each_slot(pos, &mut |c| n_cols = n_cols.max(c + 1));
            }
        }
        let mut bound = vec![false; n_cols];
        let mut scheduled = vec![false; n];
        let mut order = Vec::with_capacity(n);
        for _ in 0..n {
            let any_connected =
                (0..n).any(|i| !scheduled[i] && pattern_connected(&compiled[i], &bound));
            let mut best: Option<usize> = None;
            let mut best_score = 0usize;
            for i in 0..n {
                if scheduled[i] || (any_connected && !pattern_connected(&compiled[i], &bound)) {
                    continue;
                }
                let cp = &compiled[i];
                let score = [&cp.s, &cp.p, &cp.o]
                    .into_iter()
                    .filter(|p| constrained(p, &bound))
                    .count();
                if best.is_none() || score > best_score {
                    best = Some(i);
                    best_score = score;
                }
            }
            let chosen = best.expect("an unscheduled pattern always remains");
            scheduled[chosen] = true;
            mark_bound(&compiled[chosen], &mut bound);
            order.push(chosen);
        }
        order
    }

    /// GROUND TRUTH: the total number of intermediate solution rows a left-deep
    /// execution in `order` materialises — the sum over each prefix of the REAL result
    /// count of that prefix BGP (evaluated through `eval_bgp`). Not the model's estimate:
    /// if the cost model were wrong, the cost order could lose here and the gate would
    /// fail.
    fn materialized_rows(ds: &RdfDataset, patterns: &[TriplePattern], order: &[usize]) -> usize {
        let mut total = 0usize;
        for k in 1..=order.len() {
            let prefix: Vec<TriplePattern> =
                order[..k].iter().map(|&i| patterns[i].clone()).collect();
            let ctx = EvalCtx::new(ds);
            total += eval_bgp(&prefix, &ctx).expect("bgp").rows.len();
        }
        total
    }

    /// On a skewed multi-join star (predicates of cardinality 20 / 10 / 5 / 1, all
    /// sharing the hub variable), the cost planner's order materialises STRICTLY fewer
    /// intermediate rows than the structural heuristic — measured by real result counts,
    /// not the model — and both orders yield the identical final result multiset.
    #[test]
    fn cost_order_materialises_fewer_rows_than_structural() {
        // Skewed fixture: hub --pred--> N leaves, for N ∈ {20, 10, 5, 1}.
        let mut b = RdfDatasetBuilder::new();
        let hub = b.intern_iri("http://ex/hub");
        for (name, count) in [("hot", 20), ("warm", 10), ("mid", 5), ("rare", 1)] {
            let pred_id = b.intern_iri(&format!("http://ex/{name}"));
            for i in 0..count {
                let leaf = b.intern_iri(&format!("http://ex/{name}{i}"));
                b.push_quad(hub, pred_id, leaf, None);
            }
        }
        let ds = b.freeze().expect("freeze");

        // Source order hot, warm, mid, rare — all share ?s, all bound on the predicate.
        let patterns = [
            triple(var_pos("s"), pred("http://ex/hot"), var_pos("a")),
            triple(var_pos("s"), pred("http://ex/warm"), var_pos("b")),
            triple(var_pos("s"), pred("http://ex/mid"), var_pos("c")),
            triple(var_pos("s"), pred("http://ex/rare"), var_pos("d")),
        ];

        // Compile the patterns and derive both orders.
        let mut working = VarSchema::new();
        for p in &patterns {
            for key in slot_keys(p) {
                working.push(key);
            }
        }
        let compiled: Vec<CompiledPattern> = patterns
            .iter()
            .map(|p| {
                compile_pattern(p, &working, &ds)
                    .expect("compile")
                    .expect("constant present")
            })
            .collect();
        let cost = cost_based_order(&compiled, &ds, &GraphScope::One(GraphMatch::Any));
        let structural = structural_order(&compiled);

        // The structural heuristic seeds with the (tied) lowest-index pattern → the hot
        // predicate; the cost planner seeds with the rare one and ascends.
        assert_eq!(structural, vec![0, 1, 2, 3]);
        assert_eq!(cost, vec![3, 2, 1, 0]);

        // GROUND-TRUTH WIN: strictly fewer materialised intermediate rows.
        let cost_rows = materialized_rows(&ds, &patterns, &cost);
        let structural_rows = materialized_rows(&ds, &patterns, &structural);
        assert!(
            cost_rows < structural_rows,
            "cost order must materialise strictly fewer rows: cost={cost_rows} structural={structural_rows}"
        );

        // SAFETY: the final result multiset is identical under both orders.
        let cost_arranged: Vec<TriplePattern> = cost.iter().map(|&i| patterns[i].clone()).collect();
        let structural_arranged: Vec<TriplePattern> =
            structural.iter().map(|&i| patterns[i].clone()).collect();
        let vars = ["s", "a", "b", "c", "d"];
        let r_cost = run(&ds, &cost_arranged, &vars);
        let r_structural = run(&ds, &structural_arranged, &vars);
        assert_eq!(r_cost, r_structural);
        // Full cross-on-hub join: 20 (hot) × 10 (warm) × 5 (mid) × 1 (rare).
        assert_eq!(r_cost.len(), 20 * 10 * 5, "full cross-on-hub join");
    }
}
