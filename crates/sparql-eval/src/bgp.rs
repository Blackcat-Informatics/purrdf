// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Basic-graph-pattern (BGP) evaluation — the TermId hot path.
//!
//! A BGP is a conjunction of triple patterns. Evaluation stays entirely in
//! interned [`TermId`](purrdf_core::TermId) space:
//!
//! 1. **Compile** each triple pattern's three positions to either a [`Pos::Slot`]
//!    (a variable column) or a [`Pos::Bound`] (a ground constant resolved once via
//!    `term_id_by_value`, the reverse index). If a ground constant is absent from
//!    the dataset the whole BGP is empty — that constant cannot match.
//! 2. **Order** the patterns cheapest-first with a cost-based join planner
//!    ([`cost_based_order`]): probe each pattern's real cardinality through the
//!    lazy permutation index and search join orders (exhaustive left-deep DP for a
//!    small BGP, greedy beyond) to minimise the estimated total intermediate
//!    cardinality, keeping the join connected.
//! 3. **Index-nested-loop join** in that order; for each partial solution, substitute
//!    its already-bound variables into the next pattern's positions and call the
//!    indexed `quads_for_pattern`, then extend. Repeated variables (`?x p ?x`) and
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

use purrdf_core::{DatasetView, GraphMatch, QuadIds, TermId, TermRef, ViewTermId};
use purrdf_lex::term_syntax::{
    TRIPLE_TERM_CLOSE, TRIPLE_TERM_OPEN, write_blank, write_iri, write_literal,
};
use purrdf_sparql_algebra::{
    GraphPattern, NamedNodePattern, PropertyFunctionCall, TermPattern, TriplePattern, Variable,
};
use purrdf_xsd::ieee::{Binary64, Binary64Scope};

use crate::DetHashSet;
use crate::convert::{ground_term_pattern_to_value, named_node_to_value};
use crate::dataset_spec::{ActiveDataset, GraphScope};
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::governor::ledger::PlanEstimate;
use crate::plan::PlanShape;
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};
use crate::statement_layer::{self, StatementProbe};
use std::sync::Arc;

/// The `rdf:reifies` predicate IRI — the indirection edge of the RDF 1.2 reification
/// layer. A triple pattern whose predicate is bound to this IRI (and whose object is a
/// quoted-triple pattern) draws candidates from the dataset's reifier side-table via
/// [`RdfDataset::reifier_quads`], which is invisible to the `quads` table.
use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;

/// The `NUL`-prefixed marker that distinguishes a synthetic blank-node slot
/// variable from a real, projectable SPARQL variable.
///
/// Spelled out in full rather than as the bare `NUL`: a blank label SHARED between
/// the pieces of one basic graph pattern is renamed to a different `NUL`-prefixed
/// variable (see `crate::blank_scope`), and that one must keep its column here so
/// the pieces can join on it.
const BLANK_VAR_PREFIX: &str = "\u{0}bnode:";

/// A compiled triple-pattern position.
enum Pos<I: ViewTermId = TermId> {
    /// A variable (or blank-node) column index into the working schema.
    Slot(usize),
    /// A ground constant resolved to its dataset id.
    Bound(I),
    /// A nested RDF 1.2 quoted-triple pattern `<<( s p o )>>` that contains at least
    /// one variable (a fully-ground quoted triple resolves to a single [`Pos::Bound`]
    /// id instead). Binding descends into the candidate row's triple-term value,
    /// unifying the inner positions and enforcing repeated-variable consistency.
    Triple(Box<TriplePos<I>>),
}

/// Release a position without descending into it: a nested quoted triple's components
/// are moved onto a work list and released from there, so a position nested to any
/// depth is dropped on constant machine stack.
impl<I: ViewTermId> Drop for Pos<I> {
    fn drop(&mut self) {
        let Self::Triple(triple) = self else {
            return;
        };
        if !matches!(triple.s, Self::Triple(_))
            && !matches!(triple.p, Self::Triple(_))
            && !matches!(triple.o, Self::Triple(_))
        {
            return;
        }
        let mut pending: Vec<Self> = vec![
            std::mem::replace(&mut triple.s, Self::Slot(0)),
            std::mem::replace(&mut triple.p, Self::Slot(0)),
            std::mem::replace(&mut triple.o, Self::Slot(0)),
        ];
        while let Some(mut position) = pending.pop() {
            if let Self::Triple(inner) = &mut position {
                pending.extend([
                    std::mem::replace(&mut inner.s, Self::Slot(0)),
                    std::mem::replace(&mut inner.p, Self::Slot(0)),
                    std::mem::replace(&mut inner.o, Self::Slot(0)),
                ]);
            }
        }
    }
}

/// A compiled nested quoted-triple position: its three component positions, each
/// itself a [`Pos`] (so quoted triples may nest, and any component may be a variable,
/// a ground constant, or a further nested triple).
struct TriplePos<I: ViewTermId = TermId> {
    s: Pos<I>,
    p: Pos<I>,
    o: Pos<I>,
}

/// One compiled triple pattern: its three positions in `(s, p, o)` order.
struct CompiledPattern<I: ViewTermId = TermId> {
    s: Pos<I>,
    p: Pos<I>,
    o: Pos<I>,
}

/// Evaluate a basic graph pattern to a multiset of solutions over its real
/// (non-blank) variables.
/// # A leaf of the partial-lift channel
///
/// A basic graph pattern has no sub-pattern, so there is no child truncation to compose:
/// it is where a truncation ORIGINATES rather than somewhere one passes through. The
/// dispatch in [`crate::eval::eval`] therefore wraps this result directly, and this
/// function keeps its `&EvalCtx` (shared, not exclusive) borrow — the property that lets
/// a parallel worker call it from a shared context.
// Out of line by design: see the thin-dispatcher invariant on `eval::eval_node`.
#[inline(never)]
pub(crate) fn eval_bgp<D: DatasetView + Sync>(
    patterns: &[TriplePattern],
    ctx: &EvalCtx<'_, D>,
) -> Result<SolutionSeq<D::Id>, EvalError> {
    // The empty BGP is the identity table Z: one solution binding nothing.
    if patterns.is_empty() {
        return Ok(SolutionSeq::unit());
    }

    // Pass 1: collect every slot variable (real + synthetic blank) in first-seen
    // (subject, predicate, object) order — the working column layout.
    let mut working = VarSchema::default();
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
    //
    // The differential planner-correctness test can force the retired structural
    // heuristic instead; because the reorder is still just a permutation, the result
    // multiset must stay identical.
    let order = if ctx.options.force_structural_bgp_order {
        Arc::from(structural_order(&compiled))
    } else {
        plan_or_cached_order(&compiled, ctx.dataset, &scope, ctx.bgp_order_cache)
    };

    // The interned id of `rdf:reifies`, resolved once. `None` ⇒ the dataset has no
    // reifier layer at all (the predicate was never interned), so no virtual reifier
    // candidates exist for any pattern.
    //
    // The lookup value is a process-wide `static`, not a fresh `TermValue::Iri` per
    // BGP: `DatasetView::term_id_by_value` takes a borrowed value, the IRI is a
    // constant, and minting it here charged one heap allocation to every BGP
    // evaluation — a per-focus-node cost on the SHACL path, for a string that is the
    // same bytes every time. `TermValue::Iri` owns a `String`, so it cannot be a
    // `const`; a write-once `OnceLock` is the allocation-free-after-first-use form
    // and needs no dependency (same shape as `SINGLETON` in `plan_or_cached_order`).
    static REIFIES: std::sync::OnceLock<purrdf_core::TermValue> = std::sync::OnceLock::new();
    let reifies_value = REIFIES.get_or_init(|| purrdf_core::TermValue::Iri(RDF_REIFIES.to_owned()));
    let reifies_id = ctx
        .dataset
        .term_id_by_value(reifies_value)
        .map_err(EvalError::source_read)?;

    // Whether this execution charges fuel at all. Read once, outside the pattern loop:
    // an ungoverned run (and a run whose caller set only a deadline or only an answer
    // cap) answers `false` here and allocates no charge ledger, so the index-nested-loop
    // below is byte-for-byte the loop it was before governors existed.
    let metered = ctx
        .governor_state()
        .is_some_and(|state| state.is_engaged_in(purrdf_core::ResourceDimension::Fuel));
    // A latching stop signal is inherently a sequential observation boundary. Keeping
    // this scan on one worker makes the rows accepted before the firing poll a genuine
    // source-order prefix instead of a scheduling-dependent mixture of later chunks.
    let stop_observable = ctx.stop_signal().is_some();

    // The answer-cap / `LIMIT` pushdown's verdict for this BGP: the number of output rows
    // past which nothing this node produces can reach the query's answer. `None` is every
    // BGP the plan did not license a ceiling for, and costs one hash probe.
    //
    // It is applied to the **last** pattern of the join order and to no other, because
    // only there are the loop's accumulated rows this node's OUTPUT rows. Cutting an
    // earlier stage would cut *intermediate* rows, and an intermediate row that a later
    // pattern fails to extend contributes no output row at all — so `k` intermediates can
    // yield fewer than `k` answers, and the query would report a short answer as a
    // complete one. The last stage has no such gap: its rows are the answer rows, in
    // order, so stopping at `k` of them yields exactly the first `k`.
    let ceiling = ctx.row_ceiling();
    let last_stage = order.len().saturating_sub(1);

    // Index-nested-loop evaluation. Rows start as a single all-unbound solution. That
    // seed is itself a working-width intermediate row, so a ceiling narrower than one row
    // refuses it before the `SmallVec` is allocated.
    let cell_ceiling = ctx.cell_row_ceiling(working.len());
    if cell_ceiling == Some(0) {
        let _ = ctx.observe_cells(1, working.len());
        return Ok(empty_over_real_vars(&working));
    }
    let mut rows: Vec<Solution<D::Id>> = vec![purrdf_core::smallvec![None; working.len()]];
    for (stage, &i) in order.iter().enumerate() {
        let cp = &compiled[i];
        // The probe's bound-axis shape is fixed across this slot's rows (a variable is
        // bound by an earlier pattern for every row or for none), so the permutation
        // choice is loop-invariant: for a single-graph scope, compute the
        // `QuadProbePlan` once from the first row (non-empty — the loop `break`s above
        // the moment `rows` empties) and capture the `Copy` plan in every worker,
        // instead of re-selecting it per row.
        let plan: Option<D::ProbePlan> = match &scope {
            GraphScope::One(gm) => Some(ctx.dataset.probe_plan(
                query_id(&cp.s, &rows[0]).is_some(),
                query_id(&cp.p, &rows[0]).is_some(),
                query_id(&cp.o, &rows[0]).is_some(),
                *gm,
            )),
            GraphScope::Merge(_) => None,
        };
        let expand = |acc: &mut crate::parallel::RowSink<'_, Solution<D::Id>>,
                      fuel: &mut u64,
                      row: &Solution<D::Id>| {
            let s = query_id(&cp.s, row);
            let p = query_id(&cp.p, row);
            let o = query_id(&cp.o, row);
            match &scope {
                // Single-graph scope (store default / a named graph): the indexed
                // partition_point read, unchanged — no de-dup overhead.
                GraphScope::One(gm) => {
                    let plan = plan.expect("plan computed above for GraphScope::One");
                    for quad in ctx.dataset.quads_for_pattern_with_plan(&plan, s, p, o, *gm) {
                        if ctx.stop_check().is_some() {
                            return;
                        }
                        // The `bgp-candidate-quad` charge point: one unit per candidate
                        // EXAMINED, not per candidate that binds. Charging only the ones
                        // that bind would let the single most expensive shape in the
                        // evaluator — a highly selective pattern scanned over a large
                        // index — cost nothing at all.
                        *fuel = fuel.saturating_add(1);
                        if let Some(extended) = bind_row(row, cp, &quad, ctx.dataset) {
                            acc.push(extended);
                        }
                        // The pushed row ceiling, applied INSIDE the scan. This is the
                        // whole point of the pushdown: a `LIMIT 10` over a million-quad
                        // store stops here, having examined eleven candidates, instead
                        // of materialising a million rows for the root to discard.
                        if acc.is_full() {
                            return;
                        }
                    }
                    // The RDF 1.2 reification layer is a side-table outside `quads`, so
                    // fold its virtual triples in here — additively (no double
                    // counting). Each reifier/annotation row carries its own graph, so
                    // the probe binds `?g` under `GRAPH ?g`: a default-graph reifier
                    // shows only when the scope admits the default graph (unchanged),
                    // and a `GRAPH :g`-scoped reifier shows only under that named graph.
                    emit_virtual_candidates(ctx.dataset, cp, s, p, o, reifies_id, *gm, |quad| {
                        if acc.is_full() || ctx.stop_check().is_some() {
                            return;
                        }
                        *fuel = fuel.saturating_add(1);
                        if let Some(extended) = bind_row(row, cp, &quad, ctx.dataset) {
                            acc.push(extended);
                        }
                    });
                }
                // A FROM/USING-merged default graph: union the per-graph reads, but
                // RDF-merge unions *triples*, so a triple present in two merged graphs
                // must bind once — de-dupe by (s, p, o) for this pattern+row. The
                // reification layer is store-default content (not part of an explicitly
                // FROM-named merge), so it is not folded into a merged scope. `seen` is
                // local to this row's worker (each row gets its own), identical to the
                // sequential path.
                GraphScope::Merge(gs) => {
                    let mut seen: DetHashSet<(D::Id, D::Id, D::Id)> = DetHashSet::default();
                    for &g in gs {
                        for quad in ctx.dataset.quads_for_pattern(s, p, o, GraphMatch::Named(g)) {
                            if ctx.stop_check().is_some() {
                                return;
                            }
                            *fuel = fuel.saturating_add(1);
                            if !seen.insert((quad.s, quad.p, quad.o)) {
                                continue;
                            }
                            if let Some(extended) = bind_row(row, cp, &quad, ctx.dataset) {
                                acc.push(extended);
                            }
                            if acc.is_full() {
                                return;
                            }
                        }
                    }
                }
            }
        };
        // The bounded driver is sequential; see `parallel::bounded_chunk_map_metered` for
        // why a per-chunk ceiling would stop nothing. Every other stage — and every BGP
        // with no ceiling at all — takes the parallel driver unchanged.
        let semantic_ceiling = ceiling.filter(|_| stage == last_stage);
        // A semantic ceiling (LIMIT or the answer-cap + 1 pushdown) may stop exactly at its
        // bound: rows beyond it provably cannot affect the query. The allocation ceiling is
        // different and inclusive, so it must probe for one more *qualifying* row before it
        // can call an exactly-full bag truncated. When the semantic ceiling is tighter, it
        // wins and no cell overflow exists to discover.
        let cell_governs =
            cell_ceiling.filter(|cell| semantic_ceiling.is_none_or(|semantic| *cell < semantic));
        let (next, ledger, cell_overflowed) = if let Some(cell) = cell_governs {
            crate::parallel::cell_bounded_chunk_map_metered(&rows, metered, cell, expand)
        } else {
            let effective_ceiling =
                semantic_ceiling.or_else(|| stop_observable.then_some(usize::MAX));
            let (next, ledger) = match effective_ceiling {
                Some(ceiling) => {
                    crate::parallel::bounded_chunk_map_metered(&rows, metered, ceiling, expand)
                }
                None => crate::parallel::par_chunk_map_metered(
                    ctx.sequential_operation_required(),
                    &rows,
                    metered,
                    expand,
                ),
            };
            (next, ledger, false)
        };
        rows = next;
        if cell_overflowed {
            // The sink refused (and did not store) row `cell + 1`; record precisely that
            // attempted peak. `commit_node_output` will wrap the retained prefix in the
            // ordinary leaf-origin certificate.
            let _ = ctx.observe_cells(rows.len().saturating_add(1), working.len());
            if stage != last_stage {
                // These are working rows waiting for a later triple pattern, not answers.
                // Publishing them would leave that later pattern's variables unbound and
                // certify rows the complete BGP never contains. The empty bag is the only
                // sound lower bound when an internal stage is what crossed the ceiling;
                // a final-stage cut, by contrast, is already a genuine answer prefix.
                rows.clear();
            }
        }
        // The ordered fold. Chunk workers accumulated their per-item charges with no
        // atomics and nothing shared; this walks the concatenated ledger in source-item
        // order on one thread, so the item at which the budget runs out is the same one
        // whatever the worker count, chunk geometry, or scheduling was. The certified
        // partial is the extension of the input rows strictly before that item — a
        // positional prefix of what this pattern would have produced, which is what
        // `eval` then wraps as the truncation's certificate.
        //
        // `GovernorState::should_abandon` is deliberately NOT consulted here, and not by
        // the workers either. It is a `Relaxed` read, so what it reports depends on which
        // thread asked and when — and a worker that stopped early on it would hand this
        // fold a ledger missing the very entries the crossing sits among, making the
        // reported trip point a function of scheduling. The honest consequence is stated
        // rather than papered over: the actual work of one pattern expansion can overshoot
        // the reported trip point by the remainder of that expansion. Reported work is
        // exact; the overshoot is bounded by one pattern, and the next pattern never
        // starts.
        if let Some(state) = ctx.governor_state() {
            if state.tripped().is_some() {
                break;
            } else if metered {
                let crossing = state.commit_ordered_items(&ledger);
                // The ledger line for this BGP node. The fold charged every item strictly
                // before the crossing plus the crossing item itself (the charge is applied
                // and then compared), so the same prefix is what is reported — the ledger
                // records what the schedule spent, never what it would have spent.
                let charged = crossing.map_or(ledger.len(), |(index, _, _)| index + 1);
                ctx.note_fuel(
                    crate::governor::ChargePoint::BgpCandidateQuad,
                    ledger[..charged.min(ledger.len())]
                        .iter()
                        .fold(0_u64, |sum, item| sum.saturating_add(item.fuel)),
                );
                if let Some((_, committed, _)) = crossing {
                    rows.truncate(usize::try_from(committed).unwrap_or(usize::MAX));
                    break;
                }
            }
        }
        if rows.is_empty() {
            break;
        }
    }

    Ok(project_out_blanks(&working, rows))
}

/// The retired most-constrained-first STRUCTURAL heuristic, reproduced here as
/// the baseline the cost planner must beat and as the forced order used by the
/// differential planner-correctness corpus test. Schedule greedily by the count
/// of bound positions, keep the join connected, break ties on lowest index.
///
/// This is the structural order `cost_based_order` replaced; it is intentionally
/// deterministic and never materialises a plan as triples.
fn structural_order<I: ViewTermId>(compiled: &[CompiledPattern<I>]) -> Vec<usize> {
    /// Whether every slot reachable from `pos` is already bound, over a work list of
    /// the nested positions in `(s, p, o)` order, stopping at the first unbound slot.
    fn constrained<I: ViewTermId>(pos: &Pos<I>, bound: &[bool]) -> bool {
        let mut pending: purrdf_core::SmallVec<[&Pos<I>; 8]> = purrdf_core::smallvec![pos];
        while let Some(pos) = pending.pop() {
            match pos {
                Pos::Bound(_) => {}
                Pos::Slot(c) => {
                    if !bound[*c] {
                        return false;
                    }
                }
                Pos::Triple(t) => pending.extend([&t.o, &t.p, &t.s]),
            }
        }
        true
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
fn plan_or_cached_order<D: DatasetView>(
    compiled: &[CompiledPattern<D::Id>],
    dataset: &D,
    scope: &GraphScope<D::Id>,
    cache: Option<crate::plan_cache::OrderCacheRef<'_>>,
) -> Arc<[usize]> {
    // A one-pattern BGP has exactly one join order, so there is nothing to plan and
    // nothing worth remembering. Short-circuiting it is not a micro-optimisation of
    // a cheap case: it is what keeps the cache USEFUL once a pre-bound variable has
    // been pushed into the pattern.
    //
    // `bgp_shape_key` hashes a `Pos::Bound`'s interned ID, so `<c1> <p> ?o` and
    // `<c2> <p> ?o` are different keys. That is right for a multi-pattern BGP, where
    // which constant is bound really can change the best order. For the single
    // pattern a SHACL-SPARQL constraint body usually is, it meant one fresh key per
    // FOCUS NODE — a cache that could never hit, charged the full `cost_based_order`
    // walk anyway, and then evicted a live entry to store an answer nobody would ask
    // for again. The order-cache traffic was the largest term left in the per-focus
    // cost after the pre-binding pushdown, and it was the only one that was not
    // exactly linear in the focus count, because what a bounded cache evicts depends
    // on how the focus nodes were chunked across workers.
    if compiled.len() == 1 {
        static SINGLETON: std::sync::OnceLock<Arc<[usize]>> = std::sync::OnceLock::new();
        return Arc::clone(SINGLETON.get_or_init(|| Arc::from(vec![0_usize])));
    }
    let Some(cache) = cache else {
        return Arc::from(cost_based_order(compiled, dataset, scope));
    };
    let key = (dataset.stats_fingerprint(), bgp_shape_key(compiled, scope));
    use crate::plan_cache::OrderCacheRef;
    let cached = match cache {
        OrderCacheRef::Legacy(cache) => cache
            .read()
            .expect("order cache lock poisoned")
            .get(&key)
            .cloned(),
        OrderCacheRef::Bounded(cache) => cache.lock().expect("order cache lock poisoned").get(&key),
    };
    if let Some(order) = cached {
        // A shape-key collision is NOT licensed by the stats-fingerprint safety
        // argument: a wrong-length order would index out of bounds in the join loop.
        // Guard it — on a length mismatch fall through and re-plan.
        if order.len() == compiled.len() {
            return order;
        }
    }
    let order: Arc<[usize]> = Arc::from(cost_based_order(compiled, dataset, scope));
    match cache {
        OrderCacheRef::Legacy(cache) => {
            cache
                .write()
                .expect("order cache lock poisoned")
                .insert(key, Arc::clone(&order));
        }
        OrderCacheRef::Bounded(cache) => {
            let bytes = size_of_val(order.as_ref())
                .saturating_add(2 * size_of::<usize>())
                .saturating_add(2 * size_of_val(&key));
            cache
                .lock()
                .expect("order cache lock poisoned")
                .insert(key, Arc::clone(&order), bytes);
        }
    }
    order
}

/// A deterministic hash of a BGP's *shape* — its pattern count, every position's
/// structure (slot column / bound id / nested quoted-triple), and the graph scope — for
/// the order cache key. Encodes `compiled.len()` and the full positional structure so two
/// structurally distinct BGPs cannot collide to one cached order, and folds in the scope
/// because a pattern's cardinality (hence its best order) is scope-dependent.
fn bgp_shape_key<I: ViewTermId>(compiled: &[CompiledPattern<I>], scope: &GraphScope<I>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = purrdf_hash::fixed::FixedHasher::default();
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
                    id.encode().hash(&mut h);
                }
            }
        }
        GraphScope::Merge(gs) => {
            1u8.hash(&mut h);
            gs.len().hash(&mut h);
            for g in gs {
                g.encode().hash(&mut h);
            }
        }
    }
    h.finish()
}

/// Hash one compiled position structurally (tag + payload), descending into nested
/// quoted triples — a helper for [`bgp_shape_key`].
///
/// The hasher is fed in pre-order over a work list: a nested triple's tag, then the
/// whole of its subject, then its predicate, then its object — the sequence a
/// recursive descent writes — so a position nested to any depth hashes on constant
/// machine stack to the same value.
fn hash_pos<I: ViewTermId, H: std::hash::Hasher>(pos: &Pos<I>, h: &mut H) {
    use std::hash::Hash;
    let mut pending: purrdf_core::SmallVec<[&Pos<I>; 8]> = purrdf_core::smallvec![pos];
    while let Some(pos) = pending.pop() {
        match pos {
            Pos::Slot(c) => {
                0u8.hash(h);
                c.hash(h);
            }
            Pos::Bound(id) => {
                1u8.hash(h);
                // Hash the id via its join-key encoding: for `TermId` this is the 0-based
                // index as `u64` (byte-identical to the historical `id.index().hash(h)` on
                // a 64-bit target), and it is available for any `ViewTermId` id width.
                // Only a join-order-cache discriminator (a collision is at worst a
                // suboptimal order, never a wrong result), so the exact bytes need only be
                // deterministic.
                id.encode().hash(h);
            }
            Pos::Triple(t) => {
                2u8.hash(h);
                pending.extend([&t.o, &t.p, &t.s]);
            }
        }
    }
}

/// Order compiled BGP patterns cheapest-first with a cost-based join planner. Unlike a structural heuristic, this probes the dataset's
/// real per-pattern cardinalities (the lazy permutation index, via
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
/// pure, the cost arithmetic is order-stable `f64`, each operation correctly rounded on
/// every target (`purrdf_xsd::ieee`), compared via `total_cmp`, and
/// ties break on the lexicographically smallest order (lowest original index first) —
/// identical run to run, no hash-iteration leak.
///
/// Returns a permutation of `0..compiled.len()`.
fn cost_based_order<D: DatasetView>(
    compiled: &[CompiledPattern<D::Id>],
    dataset: &D,
    scope: &GraphScope<D::Id>,
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
fn base_cardinality<D: DatasetView>(
    dataset: &D,
    cp: &CompiledPattern<D::Id>,
    scope: &GraphScope<D::Id>,
) -> u64 {
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
fn constant_of<I: ViewTermId>(pos: &Pos<I>) -> Option<I> {
    match pos {
        Pos::Bound(id) => Some(*id),
        Pos::Slot(_) | Pos::Triple(_) => None,
    }
}

/// How many of a pattern's three top-level positions are already bound by an
/// earlier-scheduled pattern — each is an equality-join axis (selectivity ~`1/T`).
/// Ground constants are excluded: their selectivity is already folded into the
/// pattern's base cardinality.
fn join_positions<I: ViewTermId>(cp: &CompiledPattern<I>, bound: &[bool]) -> usize {
    [&cp.s, &cp.p, &cp.o]
        .into_iter()
        .filter(|pos| pos_has_bound_slot(pos, bound))
        .count()
}

/// The running intermediate-size estimate after appending a pattern: scale by its
/// base size, divide by `T` for each already-bound join axis.
///
/// Every operation is correctly rounded binary64 through `ops`, so the estimate -- and
/// with it the join order a query's unordered rows come out in, and the peak a governed
/// query is refused at -- is the same on every target, the x87 included.
fn step_size(ops: Binary64<'_>, running: f64, base_p: f64, joins: usize, t: f64) -> f64 {
    ops.div(ops.mul(running, base_p), power(ops, t, joins))
}

/// `base^exponent` by square-and-multiply, low bit first -- the order `f64::powi`'s
/// runtime (`__powidf2`) multiplies in -- each product correctly rounded, so the power
/// is one defined value on every target rather than whatever a target's `powi` computes.
fn power(ops: Binary64<'_>, base: f64, exponent: usize) -> f64 {
    let mut result = 1.0_f64;
    let mut base = base;
    let mut exponent = exponent;
    loop {
        if exponent & 1 != 0 {
            result = ops.mul(result, base);
        }
        exponent >>= 1;
        if exponent == 0 {
            return result;
        }
        base = ops.mul(base, base);
    }
}

/// Greedy minimum-cardinality join order for a large BGP (`n > COST_DP_MAX_PATTERNS`):
/// repeatedly schedule the connected pattern whose appended intermediate-size estimate
/// is smallest, lowest-index on ties. Connectivity is enforced exactly as in the DP.
fn cost_order_greedy<I: ViewTermId>(
    compiled: &[CompiledPattern<I>],
    base: &[f64],
    t: f64,
    n_cols: usize,
) -> Vec<usize> {
    let n = compiled.len();
    let precision = Binary64Scope::enter();
    let ops = precision.ops();
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
            let size = step_size(ops, running, base[i], joins, t);
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
fn cost_order_dp<I: ViewTermId>(
    compiled: &[CompiledPattern<I>],
    base: &[f64],
    t: f64,
    n_cols: usize,
) -> Vec<usize> {
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
    let precision = Binary64Scope::enter();
    let ops = precision.ops();
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
            let size = step_size(ops, plan.size, base[i], joins, t);
            let cost = ops.add(plan.cost, size);
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
fn pattern_connected<I: ViewTermId>(cp: &CompiledPattern<I>, bound: &[bool]) -> bool {
    [&cp.s, &cp.p, &cp.o]
        .into_iter()
        .any(|pos| pos_has_bound_slot(pos, bound))
}

/// Whether a position contains an already-bound slot anywhere, over a work list of
/// the nested positions in `(s, p, o)` order, stopping at the first bound slot.
fn pos_has_bound_slot<I: ViewTermId>(pos: &Pos<I>, bound: &[bool]) -> bool {
    let mut pending: purrdf_core::SmallVec<[&Pos<I>; 8]> = purrdf_core::smallvec![pos];
    while let Some(pos) = pending.pop() {
        match pos {
            Pos::Bound(_) => {}
            Pos::Slot(c) => {
                if bound[*c] {
                    return true;
                }
            }
            Pos::Triple(t) => pending.extend([&t.o, &t.p, &t.s]),
        }
    }
    false
}

/// Record a scheduled pattern's slot columns as now-bound (descending into nested
/// quoted triples).
fn mark_bound<I: ViewTermId>(cp: &CompiledPattern<I>, bound: &mut [bool]) {
    for pos in [&cp.s, &cp.p, &cp.o] {
        for_each_slot(pos, &mut |c| bound[c] = true);
    }
}

/// Visit every slot column reachable from a position (itself, or the inner positions
/// of a nested quoted triple), in `(s, p, o)` pre-order over a work list.
fn for_each_slot<I: ViewTermId>(pos: &Pos<I>, f: &mut impl FnMut(usize)) {
    let mut pending: purrdf_core::SmallVec<[&Pos<I>; 8]> = purrdf_core::smallvec![pos];
    while let Some(pos) = pending.pop() {
        match pos {
            Pos::Bound(_) => {}
            Pos::Slot(c) => f(*c),
            Pos::Triple(t) => pending.extend([&t.o, &t.p, &t.s]),
        }
    }
}

/// The slot variables a triple pattern introduces, in `(s, p, o)` order — descending
/// into any nested quoted-triple position so its inner variables become columns too. A
/// ground position yields nothing; a blank node yields a synthetic slot variable.
///
/// Inline for a pattern of up to four slots, which every pattern without a quoted
/// triple is.
fn slot_keys(pattern: &TriplePattern) -> purrdf_core::SmallVec<[Variable; 4]> {
    let mut keys = purrdf_core::SmallVec::new();
    collect_triple_slot_keys(pattern, &mut keys);
    keys
}

/// Append a triple pattern's slot variables, through nested quoted triples, in
/// `(s, p, o)` order.
fn collect_triple_slot_keys(pattern: &TriplePattern, keys: &mut impl Extend<Variable>) {
    collect_slot_keys(
        purrdf_core::smallvec![
            SlotPosition::Term(&pattern.object),
            SlotPosition::Predicate(&pattern.predicate),
            SlotPosition::Term(&pattern.subject),
        ],
        keys,
    );
}

/// One position of a triple pattern still to be read for its slot variables.
enum SlotPosition<'a> {
    Term(&'a TermPattern),
    Predicate(&'a NamedNodePattern),
}

/// Append the slot variables of the positions on `pending` — a work list whose top is
/// the next position in `(s, p, o)` order — to `keys`: a real variable, a synthetic
/// blank-node variable, or, for a quoted triple, its inner positions in the same
/// order. Ground terms yield nothing.
fn collect_slot_keys(
    mut pending: purrdf_core::SmallVec<[SlotPosition<'_>; 8]>,
    keys: &mut impl Extend<Variable>,
) {
    while let Some(position) = pending.pop() {
        match position {
            SlotPosition::Predicate(NamedNodePattern::Variable(v))
            | SlotPosition::Term(TermPattern::Variable(v)) => keys.extend([v.clone()]),
            SlotPosition::Term(TermPattern::BlankNode(b)) => {
                keys.extend([blank_var(b.as_str())]);
            }
            SlotPosition::Term(TermPattern::Triple(t)) => pending.extend([
                SlotPosition::Term(&t.object),
                SlotPosition::Predicate(&t.predicate),
                SlotPosition::Term(&t.subject),
            ]),
            SlotPosition::Predicate(NamedNodePattern::NamedNode(_))
            | SlotPosition::Term(TermPattern::NamedNode(_) | TermPattern::Literal(_)) => {}
        }
    }
}

/// The synthetic slot variable for a blank-node label (NUL-prefixed; cannot collide
/// with a parser-produced `?var`).
///
/// Shared with the property-function dispatch, which gives a blank-node ARGUMENT the
/// same non-distinguished-variable treatment: one slot per label, consistency enforced
/// across its occurrences, and the column projected away before the node's rows leave.
/// One spelling of the synthetic name, so the two cannot disagree about what a blank
/// slot is called.
pub(crate) fn blank_var(label: &str) -> Variable {
    Variable::new(format!("{BLANK_VAR_PREFIX}{label}"))
}

/// Whether a schema variable is a synthetic blank-node slot (vs. a real variable).
fn is_blank_var(var: &Variable) -> bool {
    var.as_str().starts_with(BLANK_VAR_PREFIX)
}

/// Compile a triple pattern's positions. Returns `Ok(None)` if a ground constant is
/// absent from the dataset (the pattern — and hence the BGP — cannot match).
///
/// So does a pattern whose subject or object holds triple terms nested deeper than any
/// the dataset holds ([`DatasetView::triple_term_nesting_bound`]): no term it could
/// match exists, and it is answered as matching nothing before any of it is converted,
/// looked up or matched level by level. The nesting is counted without recursion. A
/// dataset that vouches for no bound is matched the long way: its positions are
/// compiled and bound over work lists, so a term of any depth costs no more machine
/// stack.
fn compile_pattern<D: DatasetView>(
    pattern: &TriplePattern,
    schema: &VarSchema,
    dataset: &D,
) -> Result<Option<CompiledPattern<D::Id>>, EvalError> {
    if let Some(bound) = dataset.triple_term_nesting_bound()
        && (pattern.subject.triple_term_nesting() > bound
            || pattern.object.triple_term_nesting() > bound)
    {
        return Ok(None);
    }
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
fn compile_term<D: DatasetView>(
    term: &TermPattern,
    schema: &VarSchema,
    dataset: &D,
) -> Result<Option<Pos<D::Id>>, EvalError> {
    match term {
        TermPattern::Variable(v) => Ok(Some(Pos::Slot(slot_col(schema, v)))),
        TermPattern::BlankNode(b) => Ok(Some(Pos::Slot(slot_col(schema, &blank_var(b.as_str()))))),
        // A quoted-triple position: if it contains a variable it is a STRUCTURAL match
        // that binds inner columns (`Pos::Triple`); a fully-ground quoted triple
        // resolves to a single interned id (`Pos::Bound`) exactly like any constant.
        TermPattern::Triple(t) => {
            let nested = index_nested_triples(t);
            if nested[0].has_variable {
                match compile_triple_pos(&nested, schema, dataset)? {
                    Some(tp) => Ok(Some(Pos::Triple(Box::new(tp)))),
                    None => Ok(None),
                }
            } else {
                let value = ground_term_pattern_to_value(term, "a BGP")?;
                Ok(dataset
                    .term_id_by_value(&value)
                    .map_err(EvalError::source_read)?
                    .map(Pos::Bound))
            }
        }
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => {
            let value = ground_term_pattern_to_value(term, "a BGP")?;
            Ok(dataset
                .term_id_by_value(&value)
                .map_err(EvalError::source_read)?
                .map(Pos::Bound))
        }
    }
}

/// One quoted-triple pattern of a nested position, indexed by [`index_nested_triples`]:
/// where its subject and object quoted triples sit in the same index, and whether a
/// variable (or blank node) occurs anywhere inside it.
struct NestedTriple<'a> {
    triple: &'a TriplePattern,
    subject: Option<usize>,
    object: Option<usize>,
    has_variable: bool,
}

/// Index every quoted-triple pattern reachable from `root` — `root` itself at index 0,
/// each triple before the triples nested inside it — and mark which of them contain a
/// variable, in two passes over the index rather than a descent per triple.
fn index_nested_triples(root: &TriplePattern) -> Vec<NestedTriple<'_>> {
    // The position a pending triple fills in its parent, so the parent's index entry
    // can name it once the pending triple's own index is known.
    enum Under {
        Root,
        Subject(usize),
        Object(usize),
    }
    let mut nested: Vec<NestedTriple<'_>> = Vec::new();
    let mut pending = vec![(root, Under::Root)];
    while let Some((triple, under)) = pending.pop() {
        let index = nested.len();
        match under {
            Under::Root => {}
            Under::Subject(parent) => nested[parent].subject = Some(index),
            Under::Object(parent) => nested[parent].object = Some(index),
        }
        nested.push(NestedTriple {
            triple,
            subject: None,
            object: None,
            has_variable: false,
        });
        if let TermPattern::Triple(object) = &triple.object {
            pending.push((object, Under::Object(index)));
        }
        if let TermPattern::Triple(subject) = &triple.subject {
            pending.push((subject, Under::Subject(index)));
        }
    }
    // Every nested triple is indexed after the triple holding it, so reading the index
    // backwards meets each triple's own nested triples before the triple itself.
    for index in (0..nested.len()).rev() {
        let entry = &nested[index];
        let position_has_variable = |term: &TermPattern, nested_at: Option<usize>| match term {
            TermPattern::Variable(_) | TermPattern::BlankNode(_) => true,
            TermPattern::Triple(_) => {
                nested[nested_at.expect("a nested triple is indexed")].has_variable
            }
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => false,
        };
        let has_variable = position_has_variable(&entry.triple.subject, entry.subject)
            || matches!(entry.triple.predicate, NamedNodePattern::Variable(_))
            || position_has_variable(&entry.triple.object, entry.object);
        nested[index].has_variable = has_variable;
    }
    nested
}

/// Compile the variable-carrying quoted-triple pattern at index 0 of `nested` — the
/// three positions of it and of every variable-carrying triple nested inside it,
/// subject, predicate then object, each ground position looked up in the dataset as it
/// is reached and each ground quoted triple looked up whole. `Ok(None)` if any ground
/// position is absent from the dataset (so the whole pattern cannot match).
///
/// One frame per quoted triple still being compiled lives on an explicit stack, so a
/// position nested to any depth compiles on constant machine stack; the dataset is
/// consulted in exactly the order a descent would consult it.
fn compile_triple_pos<D: DatasetView>(
    nested: &[NestedTriple<'_>],
    schema: &VarSchema,
    dataset: &D,
) -> Result<Option<TriplePos<D::Id>>, EvalError> {
    /// A quoted triple being compiled: the positions compiled so far.
    struct Frame<I: ViewTermId> {
        at: usize,
        s: Option<Pos<I>>,
        p: Option<Pos<I>>,
    }
    let mut frames: Vec<Frame<D::Id>> = vec![Frame {
        at: 0,
        s: None,
        p: None,
    }];
    // A quoted triple compiled in full, on its way to the position that holds it.
    let mut completed: Option<TriplePos<D::Id>> = None;
    loop {
        let frame = frames
            .last_mut()
            .expect("a frame is open until the root completes");
        let entry = &nested[frame.at];
        let position = if let Some(completed) = completed.take() {
            Pos::Triple(Box::new(completed))
        } else if frame.s.is_none() || frame.p.is_some() {
            // The next term position: the subject first, the object once the predicate
            // is compiled.
            let (term, nested_at) = if frame.s.is_none() {
                (&entry.triple.subject, entry.subject)
            } else {
                (&entry.triple.object, entry.object)
            };
            match term {
                TermPattern::Variable(v) => Pos::Slot(slot_col(schema, v)),
                TermPattern::BlankNode(b) => Pos::Slot(slot_col(schema, &blank_var(b.as_str()))),
                TermPattern::Triple(_) => {
                    let at = nested_at.expect("a nested triple is indexed");
                    if nested[at].has_variable {
                        frames.push(Frame {
                            at,
                            s: None,
                            p: None,
                        });
                        continue;
                    }
                    let value = ground_term_pattern_to_value(term, "a BGP")?;
                    match dataset
                        .term_id_by_value(&value)
                        .map_err(EvalError::source_read)?
                    {
                        Some(id) => Pos::Bound(id),
                        None => return Ok(None),
                    }
                }
                TermPattern::NamedNode(_) | TermPattern::Literal(_) => {
                    let value = ground_term_pattern_to_value(term, "a BGP")?;
                    match dataset
                        .term_id_by_value(&value)
                        .map_err(EvalError::source_read)?
                    {
                        Some(id) => Pos::Bound(id),
                        None => return Ok(None),
                    }
                }
            }
        } else {
            let Some(p) = compile_predicate(&entry.triple.predicate, schema, dataset) else {
                return Ok(None);
            };
            frame.p = Some(p);
            continue;
        };
        if frame.s.is_none() {
            frame.s = Some(position);
            continue;
        }
        let Frame { s, p, .. } = frames.pop().expect("the frame is open");
        let triple = TriplePos {
            s: s.expect("the subject is compiled before the object"),
            p: p.expect("the predicate is compiled before the object"),
            o: position,
        };
        if frames.is_empty() {
            return Ok(Some(triple));
        }
        completed = Some(triple);
    }
}

/// The working-schema column of a slot variable (registered in pass 1).
fn slot_col(schema: &VarSchema, var: &Variable) -> usize {
    schema
        .index_of(var)
        .expect("every slot key was registered in pass 1")
}

/// Compile a predicate position (IRI or variable). `None` = an absent ground IRI.
fn compile_predicate<D: DatasetView>(
    predicate: &NamedNodePattern,
    schema: &VarSchema,
    dataset: &D,
) -> Option<Pos<D::Id>> {
    match predicate {
        NamedNodePattern::Variable(v) => Some(Pos::Slot(
            schema
                .index_of(v)
                .expect("every slot key was registered in pass 1"),
        )),
        NamedNodePattern::NamedNode(n) => dataset
            .term_id_if_ready(&named_node_to_value(n))
            .map(Pos::Bound),
    }
}

/// The id to query a position with, given the current partial solution: a bound
/// constant, an already-bound variable's id, or `None` (a wildcard / a variable not
/// yet bound). A `Computed` binding (never produced inside a BGP) degrades to a
/// wildcard and is rejected by [`bind_row`].
fn query_id<I: ViewTermId>(pos: &Pos<I>, row: &Solution<I>) -> Option<I> {
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
fn bind_row<D: DatasetView>(
    row: &Solution<D::Id>,
    cp: &CompiledPattern<D::Id>,
    quad: &QuadIds<D::Id>,
    dataset: &D,
) -> Option<Solution<D::Id>> {
    // Reject what can be rejected WITHOUT the row copy first. A row wider than the
    // inline capacity spills to the heap, so a copy made before the test is an
    // allocation spent on a candidate that was never going to survive.
    //
    // This filter is deliberately a SUBSET of what the binding loop below rejects,
    // never a second opinion about it: both arms read only state the loop cannot have
    // changed yet, so a candidate this rejects the loop would reject too. Everything
    // else — a repeated variable within one pattern, a nested quoted-triple
    // unification, a `Computed` binding — still falls to the loop, which remains the
    // single authority on whether a row binds.
    //
    // It earns its place on the VIRTUAL REIFICATION path. On the ordinary path
    // `query_id` has already handed both of these to `quads_for_pattern`, so the index
    // filtered them and almost nothing arrives here to reject; the reification
    // candidates are not pre-filtered at all, which is why the doc above says a
    // `Pos::Bound` mismatch must be caught in this function.
    for (pos, id) in [(&cp.s, quad.s), (&cp.p, quad.p), (&cp.o, quad.o)] {
        match pos {
            Pos::Bound(want) if *want != id => return None,
            Pos::Slot(col) => match row[*col] {
                Some(existing) if existing != SolutionTerm::Existing(id) => return None,
                _ => {}
            },
            _ => {}
        }
    }

    let mut out = Solution::from_slice(row);
    for (pos, id) in [(&cp.s, quad.s), (&cp.p, quad.p), (&cp.o, quad.o)] {
        if !bind_pos(&mut out, pos, id, dataset) {
            return None;
        }
    }
    Some(out)
}

/// [`bind_pos`]'s work list: positions still to unify, each with its candidate id.
type PosWork<'p, I> = purrdf_core::SmallVec<[(&'p Pos<I>, I); 4]>;

/// Unify one compiled position against a candidate term id, mutating `out` with any
/// newly bound slots. Returns `false` (caller rejects the row) on any disagreement:
/// - a `Pos::Bound` constant that does not equal the candidate id;
/// - a `Pos::Slot` repeated/previously-bound variable that disagrees;
/// - a `Pos::Triple` whose candidate id is not a triple term, or whose components fail
///   to unify.
///
/// A nested position's components are unified subject, predicate then object, each
/// resolved against the dataset as it is reached, over a work list rather than the
/// call stack: the first disagreement ends the unification with `out` as written so
/// far, which the caller discards.
fn bind_pos<D: DatasetView>(
    out: &mut Solution<D::Id>,
    pos: &Pos<D::Id>,
    id: D::Id,
    dataset: &D,
) -> bool {
    // Inline for a plain position and for a quoted triple of plain positions, so
    // binding a row allocates nothing beyond the row it writes.
    let mut pending: PosWork<'_, D::Id> = purrdf_core::smallvec![(pos, id)];
    while let Some((pos, id)) = pending.pop() {
        match pos {
            Pos::Bound(want) => {
                if *want != id {
                    return false;
                }
            }
            Pos::Slot(col) => {
                let value = SolutionTerm::Existing(id);
                match out[*col] {
                    Some(existing) => {
                        if existing != value {
                            return false;
                        }
                    }
                    None => out[*col] = Some(value),
                }
            }
            Pos::Triple(t) => match dataset
                .with_term(id, |term| match term {
                    TermRef::Triple { s, p, o } => Some((s, p, o)),
                    _ => None,
                })
                .ok()
                .flatten()
            {
                Some((s, p, o)) => pending.extend([(&t.o, o), (&t.p, p), (&t.s, s)]),
                // The candidate term is not a quoted triple, so a structural triple
                // pattern cannot match it.
                _ => return false,
            },
        }
    }
    true
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
///   layer is skipped entirely. When the pattern's subject is bound,
///   [`DatasetView::reifier_quads_of`] indexes straight to that reifier's run;
///   otherwise the whole reifier table is scanned.
/// - **Annotation rows** `(reifier, annPred, annObj)` — a reifier's statement
///   annotations look like ordinary triples whose subject is a reifier. When the
///   pattern's subject is bound, [`RdfDataset::annotations_of`] indexes straight to
///   that reifier's run; otherwise the whole annotation table is scanned.
///
/// Every candidate is residually filtered by the same id-equality the default scan
/// applies (`quads_for_pattern`), because — unlike `quads_for_pattern` — the virtual
/// side-table walks are not pre-narrowed by the probe. A callback keeps this hot path
/// lazy without boxing or allocating an intermediate candidate buffer.
///
/// The walk itself lives in [`crate::statement_layer`], shared with the path-witness
/// snapshot; all this adds is the pattern-shaped decision about whether the reifier table
/// can contribute at all.
#[allow(clippy::too_many_arguments)]
fn emit_virtual_candidates<D: DatasetView>(
    dataset: &D,
    cp: &CompiledPattern<D::Id>,
    s: Option<D::Id>,
    p: Option<D::Id>,
    o: Option<D::Id>,
    reifies_id: Option<D::Id>,
    gm: GraphMatch<D::Id>,
    emit: impl FnMut(QuadIds<D::Id>),
) {
    // Reifier layer: only when the predicate can be `rdf:reifies`. The object must also
    // be triple-term-shaped to be worth scanning — a quoted-triple pattern position
    // (`Pos::Triple`), a quoted-triple constant (`Pos::Bound` of a triple id), or a
    // free variable (`Pos::Slot`). A literal/IRI object constant can never be a triple
    // term, so the reifier scan is skipped. The residual `bind_row` enforces the exact
    // object match.
    let predicate_can_reify = reifies_id.is_some_and(|reifies| match &cp.p {
        Pos::Slot(_) => true,
        Pos::Bound(id) => *id == reifies,
        // A quoted triple is never a predicate position.
        Pos::Triple(_) => false,
    });
    statement_layer::visit_quads(
        dataset,
        StatementProbe {
            s,
            p,
            o,
            graph: gm,
            scan_reifier_rows: predicate_can_reify && object_can_be_triple_term(&cp.o, dataset),
        },
        emit,
    );
}

/// Whether an object position could resolve to a quoted-triple term (so the reifier
/// layer — whose object is always a triple term — is worth scanning for it). An IRI or
/// literal constant never is.
fn object_can_be_triple_term<D: DatasetView>(pos: &Pos<D::Id>, dataset: &D) -> bool {
    match pos {
        // A free variable or a structural quoted-triple pattern can match a triple term.
        Pos::Slot(_) | Pos::Triple(_) => true,
        // A bound constant is worth scanning only if the constant is itself a triple
        // term; an IRI/literal/blank object can never match a reifier row.
        Pos::Bound(id) => dataset
            .with_term(*id, |term| matches!(term, TermRef::Triple { .. }))
            .unwrap_or(false),
    }
}

// ---------------------------------------------------------------------------
// EXPLAIN / plan introspection helpers
// ---------------------------------------------------------------------------

/// Format a triple pattern as a compact SPARQL-like string for plan-introspection
/// output. The format is human-readable and stable; it is not a round-trippable
/// serializer.
fn triple_pattern_to_string(tp: &TriplePattern) -> String {
    format!(
        "{} {} {} .",
        term_pattern_to_string(&tp.subject),
        named_node_pattern_to_string(&tp.predicate),
        term_pattern_to_string(&tp.object)
    )
}

/// Format a term pattern as a compact SPARQL-like string: every constant in the
/// RDF 1.2 term syntax ([`purrdf_lex::term_syntax`]), a quoted triple as
/// `<<( s p o )>>`, written left to right over a work list of the pieces still to
/// emit, so a term nested to any depth is formatted on constant machine stack.
fn term_pattern_to_string(term: &TermPattern) -> String {
    /// One piece of the text still to emit.
    enum Piece<'a> {
        Term(&'a TermPattern),
        Predicate(&'a NamedNodePattern),
        Text(&'static str),
    }
    let mut out = String::new();
    let mut pending = vec![Piece::Term(term)];
    while let Some(piece) = pending.pop() {
        match piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Predicate(nn) => write_named_node_pattern(nn, &mut out),
            Piece::Term(TermPattern::Variable(v)) => {
                out.push('?');
                out.push_str(v.as_str());
            }
            Piece::Term(TermPattern::BlankNode(b)) => write_blank(b.as_str(), &mut out),
            Piece::Term(TermPattern::NamedNode(n)) => write_iri(n.as_str(), &mut out),
            Piece::Term(TermPattern::Literal(l)) => write_literal(
                l.value(),
                l.datatype().as_str(),
                l.language(),
                l.direction()
                    .map(purrdf_sparql_algebra::BaseDirection::as_str),
                &mut out,
            ),
            Piece::Term(TermPattern::Triple(t)) => {
                out.push_str(TRIPLE_TERM_OPEN);
                out.push(' ');
                pending.extend([
                    Piece::Text(TRIPLE_TERM_CLOSE),
                    Piece::Text(" "),
                    Piece::Term(&t.object),
                    Piece::Text(" "),
                    Piece::Predicate(&t.predicate),
                    Piece::Text(" "),
                    Piece::Term(&t.subject),
                ]);
            }
        }
    }
    out
}

/// Append a named-node pattern (IRI or variable).
fn write_named_node_pattern(nn: &NamedNodePattern, out: &mut String) {
    match nn {
        NamedNodePattern::Variable(v) => {
            out.push('?');
            out.push_str(v.as_str());
        }
        NamedNodePattern::NamedNode(n) => write_iri(n.as_str(), out),
    }
}

/// Format a named-node pattern (IRI or variable) as a compact string.
fn named_node_pattern_to_string(nn: &NamedNodePattern) -> String {
    let mut out = String::new();
    write_named_node_pattern(nn, &mut out);
    out
}

/// What one planner-side walk of a query learns about it, without evaluating it: the
/// join order the cost model chose for every BGP, and the cardinality that model
/// predicted.
///
/// Both halves come from one walk because both are read from the same probe of the
/// dataset's statistics, and doing it twice would let the explained order and the refused
/// estimate describe two different plans.
pub(crate) struct PlanSurvey {
    /// Human-readable triple-pattern strings, in the order the planner chose, for every
    /// BGP with at least two patterns. The historical `explain_query` output.
    pub(crate) orders: Vec<String>,
    /// The planner's prediction per node, indexed by [`NodeId`] in the tree the survey
    /// walks; `None` where it predicts nothing.
    pub(crate) estimates: Vec<Option<PlanEstimate>>,
    /// The shape of that tree, which is where a surveyed node finds its id.
    shape: Arc<PlanShape>,
}

purrdf_hash::debug_non_exhaustive!(PlanSurvey { orders, estimates });

impl PlanSurvey {
    /// An empty survey of the tree `shape` numbers.
    pub(crate) fn for_shape(shape: &Arc<PlanShape>) -> Self {
        Self {
            orders: Vec::new(),
            estimates: vec![None; shape.len()],
            shape: Arc::clone(shape),
        }
    }

    /// The shape of the surveyed tree.
    pub(crate) const fn shape(&self) -> &Arc<PlanShape> {
        &self.shape
    }

    /// Record `estimate` as the prediction for `node`, a node of the surveyed tree.
    fn record(&mut self, node: &GraphPattern, estimate: PlanEstimate) {
        let id = self
            .shape
            .node_of(node)
            .expect("the survey walks only nodes of the tree it was sized for");
        self.estimates[id.index()] = Some(estimate);
    }

    /// The prediction for `node`, when it is a node of the surveyed tree and the survey
    /// made one.
    fn estimate_of(&self, node: &GraphPattern) -> Option<&PlanEstimate> {
        self.shape
            .node_of(node)
            .and_then(|id| self.estimates[id.index()].as_ref())
    }

    /// The largest predicted intermediate bag anywhere in the plan, in cells, together
    /// with nothing else — this is the single number admission control compares against
    /// the caller's intermediate-cardinality ceiling.
    ///
    /// A maximum rather than a sum, because that is what the ceiling itself is: it bounds
    /// how large one operator's materialized bag may get, never how many bags a query may
    /// build. Comparing a sum against a peak ceiling would refuse long, cheap queries and
    /// admit the single catastrophic one.
    pub(crate) fn peak_cells(&self) -> u64 {
        self.estimates
            .iter()
            .flatten()
            .map(PlanEstimate::peak_cells)
            .max()
            .unwrap_or(0)
    }
}

/// Walk `pattern`, compute the cost-based order and cardinality estimate for every BGP,
/// and record both into `survey`. Scope changes from `GRAPH` blocks are tracked so
/// cardinality estimates use the right graph filter.
///
/// This is a pure-introspection path: it does not evaluate the query, and it falls back to
/// source order for BGPs whose constants are absent from the dataset (those BGPs are empty
/// regardless of order — and their estimate is zero, which is the truth).
///
/// The join-order **strings** are still only emitted for BGPs with at least two patterns,
/// because a one-pattern BGP has no order to choose. The **estimate** is recorded for every
/// BGP, one-pattern ones included: a single unconstrained triple pattern over a large store
/// is exactly the shape a cardinality ceiling exists to refuse.
///
/// The walk runs over an explicit work list in pre-order, each node's children left to
/// right, so a plan of any height costs no machine stack; the join-order strings are
/// appended, the estimates recorded and the first error raised in exactly that order.
///
/// # How a property-function call is priced, and what that composition rule is
///
/// A call's output bag is not predicted from the dataset — no index sized it — so its
/// prediction is the relation's own declaration,
/// [`PropertyFunction::rows_per_invocation`](crate::property_fn::PropertyFunction::rows_per_invocation),
/// read for the access pattern the call is *actually* invoked in (the plan has already been
/// feasibility-ordered, so that pattern is fixed by the time this walk runs).
///
/// The composition rule is stated exactly, because the survey has **no cross-node
/// arithmetic at all**: [`PlanSurvey::peak_cells`] is a maximum over per-node peaks, and a
/// `Join` of two basic graph patterns composes to neither node's product. So:
///
/// - A call contributes `rows_per_invocation(mode) × driving_rows` as its own peak, where
///   `driving_rows` is the row count predicted for its **immediate left arm** — the bag it
///   is invoked once per row of — when that arm is a node this walk predicts at all (a
///   basic graph pattern, or the chain spine ending in another call). It is `1` otherwise,
///   which is the truth for a call with nothing written before it and an honest
///   under-count for a driving arm nothing predicts. An under-estimate costs a caller
///   nothing: the live cell ceiling is still in force and still trips.
/// - Its `columns` is the relation's flattened arity — the number of positions it fills on
///   every row it emits. That is a declared quantity rather than a schema this walk would
///   have to derive, and it is never zero, so a call can never be denominated out of a
///   cell-denominated ceiling entirely.
///
/// A call whose IRI resolves against no supplied registry contributes nothing, exactly as
/// every other unpredictable leaf does; evaluation refuses that query on its own terms.
pub(crate) fn survey_pattern_plans<D: DatasetView>(
    dataset: &D,
    active_dataset: &ActiveDataset<D::Id>,
    active_graph: GraphMatch<D::Id>,
    pattern: &GraphPattern,
    relations: &crate::property_fn::PropertyFunctionRegistry,
    survey: &mut PlanSurvey,
) -> Result<(), EvalError> {
    /// One pending step of the walk.
    enum Step<'a, I> {
        /// Survey the subtree rooted at the node, evaluated against the graph.
        Visit(&'a GraphPattern, GraphMatch<I>),
        /// Price the call `node` is, driven once per row of `left`'s bag: taken once
        /// `left`'s whole subtree has been surveyed, so its prediction is on record.
        Call {
            node: &'a GraphPattern,
            call: &'a PropertyFunctionCall,
            left: &'a GraphPattern,
        },
    }

    let mut steps: Vec<Step<'_, D::Id>> = vec![Step::Visit(pattern, active_graph)];
    while let Some(step) = steps.pop() {
        let (pattern, active_graph) = match step {
            Step::Call { node, call, left } => {
                let mut bound = DetHashSet::default();
                crate::property_fn_plan::collect_certainly_bound(left, &mut bound);
                record_call_estimate(
                    node,
                    call,
                    &bound,
                    predicted_rows(left, survey).unwrap_or(1),
                    relations,
                    survey,
                )?;
                continue;
            }
            Step::Visit(pattern, active_graph) => (pattern, active_graph),
        };
        match pattern {
            GraphPattern::Bgp { patterns } => {
                survey_bgp(dataset, active_dataset, active_graph, pattern, patterns, survey)?;
            }
            // The two shapes a property-function call is attached through, and therefore the
            // only place a call's driving side is in scope. The right side is pushed first so
            // the left's whole subtree is surveyed before it.
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                if let GraphPattern::PropertyFunction(call) = &**right {
                    steps.push(Step::Call {
                        node: right,
                        call,
                        left,
                    });
                } else {
                    steps.push(Step::Visit(right, active_graph));
                }
                steps.push(Step::Visit(left, active_graph));
            }
            GraphPattern::Union { arms } => {
                steps.extend(arms.iter().rev().map(|arm| Step::Visit(arm, active_graph)));
            }
            GraphPattern::LeftJoin { left, right, .. } | GraphPattern::Minus { left, right } => {
                steps.push(Step::Visit(right, active_graph));
                steps.push(Step::Visit(left, active_graph));
            }
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Extend { inner, .. }
            // `UNFOLD` predicts nothing of its own: how many rows one input row
            // expands to is a property of a composite value this walk never sees, so
            // the survey reports its inner pattern and stops — an honest under-count,
            // which is the direction this survey is documented to fail in.
            | GraphPattern::Unfold { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Group { inner, .. } => {
                steps.push(Step::Visit(inner, active_graph));
            }
            GraphPattern::Graph { name, inner } => {
                let inner_graph = match name {
                    NamedNodePattern::NamedNode(n) => dataset
                        .term_id_if_ready(&named_node_to_value(n))
                        .map_or(GraphMatch::Default, GraphMatch::Named),
                    NamedNodePattern::Variable(_) => GraphMatch::Any,
                };
                steps.push(Step::Visit(inner, inner_graph));
            }
            // A call with nothing written before it: its driving bag is the identity table,
            // which is one row, so its whole prediction is the relation's declared bound.
            GraphPattern::PropertyFunction(call) => {
                record_call_estimate(pattern, call, &DetHashSet::default(), 1, relations, survey)?;
            }
            // Leaves that hold no BGP: there is no triple-pattern join order to choose and
            // no base cardinality to probe.
            GraphPattern::Path { .. } | GraphPattern::Values { .. } | GraphPattern::Service { .. } => {}
        }
    }
    Ok(())
}

/// Survey one basic graph pattern — `node`, holding `patterns` — for
/// [`survey_pattern_plans`]: its join order, when it has one to choose, and its estimate.
///
/// # Errors
///
/// [`EvalError`] from compiling a pattern against the dataset.
fn survey_bgp<D: DatasetView>(
    dataset: &D,
    active_dataset: &ActiveDataset<D::Id>,
    active_graph: GraphMatch<D::Id>,
    node: &GraphPattern,
    patterns: &[TriplePattern],
    survey: &mut PlanSurvey,
) -> Result<(), EvalError> {
    if patterns.is_empty() {
        return Ok(());
    }
    let scope = active_dataset.scope_for(active_graph);
    let mut working = VarSchema::default();
    for pattern in patterns {
        for key in slot_keys(pattern) {
            working.push(key);
        }
    }
    let mut compiled = Vec::with_capacity(patterns.len());
    let mut any_absent = false;
    for pattern in patterns {
        match compile_pattern(pattern, &working, dataset)? {
            Some(cp) => compiled.push(cp),
            None => {
                any_absent = true;
                break;
            }
        }
    }
    let order: Vec<usize> = if any_absent {
        (0..patterns.len()).collect()
    } else {
        cost_based_order(&compiled, dataset, &scope)
    };
    if patterns.len() >= 2 {
        for &i in &order {
            survey.orders.push(triple_pattern_to_string(&patterns[i]));
        }
    }
    let columns = real_var_schema(&working).len() as u64;
    let estimate = if any_absent {
        // A ground constant absent from the dataset makes the whole BGP empty, so
        // the honest prediction is zero rows — not the product the cost model
        // would compute from cardinalities it never probed.
        PlanEstimate {
            rows: 0,
            peak_rows: 0,
            columns,
        }
    } else {
        let (rows, peak_rows) = replay_cost_estimate(&compiled, dataset, &scope, &order);
        PlanEstimate {
            rows,
            peak_rows,
            columns,
        }
    };
    survey.record(node, estimate);
    Ok(())
}

/// Record the survey's prediction for one property-function node, keyed by that node's
/// address exactly as a basic graph pattern's is.
///
/// See [`survey_pattern_plans`] for the composition rule this implements.
///
/// # Errors
///
/// [`EvalError::Function`] if the resolved relation's declaration methods (`arity`,
/// `rows_per_invocation`) panic — read through
/// [`crate::property_fn::declaration_contained`] exactly as every other reader of a
/// `dyn PropertyFunction` declaration is, since this walk runs at prepare/explain time
/// over caller-injected host code just as the feasibility pass does.
fn record_call_estimate(
    node: &GraphPattern,
    call: &PropertyFunctionCall,
    bound: &DetHashSet<Variable>,
    driving_rows: u64,
    relations: &crate::property_fn::PropertyFunctionRegistry,
    survey: &mut PlanSurvey,
) -> Result<(), EvalError> {
    let Some(relation) = relations.resolve(&call.iri) else {
        return Ok(());
    };
    let mode = crate::property_fn_plan::invocation_mode(call, bound);
    let rows_per_invocation =
        crate::property_fn::declaration_contained(&call.iri, "row bound", || {
            relation.rows_per_invocation(mode)
        })?;
    let rows = rows_per_invocation.saturating_mul(driving_rows);
    let arity = crate::property_fn::declaration_contained(&call.iri, "arity", || relation.arity())?;
    // The positions the relation fills on every row it emits. Never zero, so a declared
    // bound cannot be denominated out of a cell ceiling; saturating, because a `usize`
    // arity beyond `u64` is already past any ceiling a caller can express.
    let columns = u64::try_from(arity.total()).unwrap_or(u64::MAX).max(1);
    survey.record(
        node,
        PlanEstimate {
            rows,
            peak_rows: rows,
            columns,
        },
    );
    Ok(())
}

/// The row count this walk predicts for `pattern`, when it predicts one at all.
///
/// A direct estimate is the answer. Failing that, a chain spine's row count is its last
/// member's, because each member's own estimate already carries the product of everything
/// to its left — which is what lets a chain of calls compose without this walk inventing
/// join arithmetic the cost model does not have. The spine is followed in a loop, so a
/// chain of any length costs no machine stack.
fn predicted_rows(pattern: &GraphPattern, survey: &PlanSurvey) -> Option<u64> {
    let mut pattern = pattern;
    loop {
        if let Some(estimate) = survey.estimate_of(pattern) {
            return Some(estimate.rows);
        }
        match pattern {
            GraphPattern::Lateral { right, .. } => pattern = right,
            _ => return None,
        }
    }
}

/// Replay `order` through the same cost model [`cost_based_order`] minimised, returning the
/// running estimate at the last stage and the largest running estimate at any stage — the
/// BGP's predicted output size and its predicted peak.
///
/// Replayed rather than returned from the search because the search's two strategies (the
/// subset DP and the greedy walk) carry their costs differently, and a second, shared
/// evaluation of the *chosen* order is one place where "what the planner predicted" is
/// defined, instead of two that can disagree.
///
/// Saturating `f64`-to-`u64` conversion: Rust's `as` cast saturates rather than wrapping,
/// so an estimate beyond `u64::MAX` becomes `u64::MAX` and a negative one — which the
/// model cannot produce, every factor being non-negative — would become zero.
fn replay_cost_estimate<D: DatasetView>(
    compiled: &[CompiledPattern<D::Id>],
    dataset: &D,
    scope: &GraphScope<D::Id>,
    order: &[usize],
) -> (u64, u64) {
    let base: Vec<f64> = compiled
        .iter()
        .map(|cp| base_cardinality(dataset, cp, scope) as f64)
        .collect();
    let t = dataset.term_count().max(1) as f64;
    let mut n_cols = 0usize;
    for cp in compiled {
        for pos in [&cp.s, &cp.p, &cp.o] {
            for_each_slot(pos, &mut |c| n_cols = n_cols.max(c + 1));
        }
    }

    let precision = Binary64Scope::enter();
    let ops = precision.ops();
    let mut bound = vec![false; n_cols];
    let mut running = 1.0f64;
    let mut peak = 0.0f64;
    for &i in order {
        let joins = join_positions(&compiled[i], &bound);
        running = step_size(ops, running, base[i], joins, t);
        peak = peak.max(running);
        mark_bound(&compiled[i], &mut bound);
    }
    (running as u64, peak as u64)
}

/// An empty solution sequence over only the real (non-blank) variables of `working`.
fn empty_over_real_vars<I: ViewTermId>(working: &VarSchema) -> SolutionSeq<I> {
    let real = real_var_schema(working);
    SolutionSeq::empty(Arc::new(real))
}

/// The schema of `working` restricted to its real variables, in order.
fn real_var_schema(working: &VarSchema) -> VarSchema {
    VarSchema::from_vars(working.vars().iter().filter(|v| !is_blank_var(v)).cloned())
}

/// Project the working rows onto only the real variables, dropping the synthetic
/// blank-node columns (which are scoped to this BGP and must not leak into joins).
/// Multiset cardinality is preserved (no dedup).
fn project_out_blanks<I: ViewTermId>(
    working: &VarSchema,
    rows: Vec<Solution<I>>,
) -> SolutionSeq<I> {
    // The working columns that survive, in order.
    let keep: purrdf_core::SmallVec<[usize; 8]> = working
        .vars()
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (!is_blank_var(v)).then_some(i))
        .collect();

    // Fast path: no blank columns — reuse rows as-is.
    if keep.len() == working.len() {
        return SolutionSeq {
            schema: Arc::new(real_var_schema(working)),
            rows,
        };
    }

    let schema = Arc::new(real_var_schema(working));
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
    use purrdf_core::TermBox;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
    use purrdf_sparql_algebra::Child;
    use purrdf_sparql_algebra::{Literal, NamedNode};

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
        let scratch = ScratchInterner::default();
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
        assert_eq!(rows, [] as [Vec<Option<TermValue>>; 0]);
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
        TermPattern::Triple(Child::new(TriplePattern {
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
        assert_eq!(rows, [] as [Vec<Option<TermValue>>; 0]);
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
                        s: TermBox::new(TermValue::Iri("http://ex/alice".to_owned())),
                        p: TermBox::new(TermValue::Iri("http://ex/age".to_owned())),
                        o: TermBox::new(TermValue::Literal {
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
        assert_eq!(rows, [] as [Vec<Option<TermValue>>; 0]);
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
        assert_eq!(rows, [] as [Vec<Option<TermValue>>; 0]);
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
        let precision = Binary64Scope::enter();
        let ops = precision.ops();
        let mut bound = vec![false; n_cols];
        let mut running = 1.0f64;
        let mut total = 0.0f64;
        for (k, &i) in order.iter().enumerate() {
            let joins = if k == 0 {
                0
            } else {
                join_positions(&compiled[i], &bound)
            };
            running = step_size(ops, running, base[i], joins, t);
            total = ops.add(total, running);
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
        let mut working = VarSchema::default();
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

    /// The cost model's power and step are correctly rounded binary64 in a defined
    /// order: the software reference's square-and-multiply, bit for bit on every target,
    /// and -- where the unit is IEEE -- exactly what `f64::powi` gave before.
    #[test]
    fn the_cost_model_is_correctly_rounded_binary64_in_a_defined_order() {
        use purrdf_xsd::ieee::reference as soft;

        let precision = Binary64Scope::enter();
        let ops = precision.ops();
        let mut state = 0xc057_u64;
        for _ in 0..20_000 {
            state = purrdf_testkit::rng::splitmix64_step(state);
            // A term count and cardinalities anywhere a dataset can have them.
            let t = (state >> 11) as f64 / 4096.0 + 1.0;
            let running = (state & 0xffff_ffff) as f64 * 1.0e-3;
            let base_p = ((state >> 20) & 0xf_ffff) as f64;
            for joins in 0..=3_usize {
                let mut reference_power = 1.0_f64;
                for _ in 0..joins {
                    reference_power = soft::mul(reference_power, t);
                }
                // `t·t` then `t·t²`: the same products as the multiply-first chain.
                if joins == 3 {
                    reference_power = soft::mul(t, soft::mul(t, t));
                }
                assert_eq!(power(ops, t, joins).to_bits(), reference_power.to_bits());
                let step = soft::div(soft::mul(running, base_p), reference_power);
                assert_eq!(
                    step_size(ops, running, base_p, joins, t).to_bits(),
                    step.to_bits()
                );
                if !purrdf_xsd::ieee::X87 {
                    assert_eq!(
                        power(ops, t, joins).to_bits(),
                        t.powi(joins as i32).to_bits()
                    );
                }
            }
        }
    }
}

/// The work-list walks over compiled positions and term patterns — the bound-slot and
/// constrained checks, slot visiting, the shape hash, slot-key collection,
/// compilation, binding and the plan-introspection rendering — checked against
/// recursive references over generated shapes (the same answers, and the same dataset
/// lookups in the same order), and at a hundred thousand levels on a thread with a
/// 128 KiB stack.
#[cfg(test)]
mod term_walk_tests {
    use purrdf_core::dataset_view::TermGuard as _;
    use std::cell::RefCell;
    use std::hash::{Hash, Hasher};
    use std::sync::Arc;

    use purrdf_core::{
        DatasetView, GraphMatch, QuadIds, RdfDataset, RdfDatasetBuilder, RdfLiteral,
        RdfStoreCapabilities, TermId, TermRef, TermValue, ViewTermId,
    };
    use purrdf_sparql_algebra::{
        BaseDirection, BlankNode, Child, Literal, NamedNode, NamedNodePattern, TermPattern,
        TriplePattern, Variable,
    };

    use super::{
        CompiledPattern, Pos, TriplePos, bind_pos, blank_var, collect_triple_slot_keys,
        compile_predicate, compile_term, for_each_slot, hash_pos, named_node_pattern_to_string,
        pos_has_bound_slot, slot_col, slot_keys, structural_order, term_pattern_to_string,
        triple_pattern_to_string,
    };
    use crate::convert::ground_term_pattern_to_value;
    use crate::error::EvalError;
    use crate::scratch::SolutionTerm;
    use crate::solution::{Solution, VarSchema};

    const EX: &str = "http://example.org/";
    const DEPTH: usize = 100_000;
    const SMALL_STACK: usize = 128 * 1024;
    const COLUMNS: usize = 4;

    /// A deterministic choice sequence.
    struct Choices {
        state: purrdf_testkit::rng::SplitMix64,
        budget: usize,
    }

    impl Choices {
        const fn new(seed: u64) -> Self {
            Self {
                state: purrdf_testkit::rng::SplitMix64::new(seed),
                budget: 24,
            }
        }

        /// One choice below `n`.
        fn choose(&mut self, n: usize) -> usize {
            self.state.below_usize(n)
        }

        /// Whether another nesting level may be spent.
        fn spend(&mut self) -> bool {
            if self.budget == 0 {
                return false;
            }
            self.budget -= 1;
            true
        }
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{EX}{local}"))
    }

    // ── The recursive references ───────────────────────────────────────────────────

    fn reference_constrained(pos: &Pos, bound: &[bool]) -> bool {
        match pos {
            Pos::Bound(_) => true,
            Pos::Slot(c) => bound[*c],
            Pos::Triple(t) => {
                reference_constrained(&t.s, bound)
                    && reference_constrained(&t.p, bound)
                    && reference_constrained(&t.o, bound)
            }
        }
    }

    fn reference_hash_pos<H: Hasher>(pos: &Pos, h: &mut H) {
        match pos {
            Pos::Slot(c) => {
                0u8.hash(h);
                c.hash(h);
            }
            Pos::Bound(id) => {
                1u8.hash(h);
                id.encode().hash(h);
            }
            Pos::Triple(t) => {
                2u8.hash(h);
                reference_hash_pos(&t.s, h);
                reference_hash_pos(&t.p, h);
                reference_hash_pos(&t.o, h);
            }
        }
    }

    fn reference_pos_has_bound_slot(pos: &Pos, bound: &[bool]) -> bool {
        match pos {
            Pos::Bound(_) => false,
            Pos::Slot(c) => bound[*c],
            Pos::Triple(t) => [&t.s, &t.p, &t.o]
                .into_iter()
                .any(|p| reference_pos_has_bound_slot(p, bound)),
        }
    }

    fn reference_for_each_slot(pos: &Pos, f: &mut impl FnMut(usize)) {
        match pos {
            Pos::Bound(_) => {}
            Pos::Slot(c) => f(*c),
            Pos::Triple(t) => {
                for inner in [&t.s, &t.p, &t.o] {
                    reference_for_each_slot(inner, f);
                }
            }
        }
    }

    fn reference_collect_triple_slot_keys(pattern: &TriplePattern, keys: &mut Vec<Variable>) {
        reference_collect_term_slot_keys(&pattern.subject, keys);
        if let NamedNodePattern::Variable(v) = &pattern.predicate {
            keys.push(v.clone());
        }
        reference_collect_term_slot_keys(&pattern.object, keys);
    }

    fn reference_collect_term_slot_keys(term: &TermPattern, keys: &mut Vec<Variable>) {
        match term {
            TermPattern::Variable(v) => keys.push(v.clone()),
            TermPattern::BlankNode(b) => keys.push(blank_var(b.as_str())),
            TermPattern::Triple(t) => reference_collect_triple_slot_keys(t, keys),
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => {}
        }
    }

    fn reference_triple_has_variable(triple: &TriplePattern) -> bool {
        reference_term_has_variable(&triple.subject)
            || matches!(triple.predicate, NamedNodePattern::Variable(_))
            || reference_term_has_variable(&triple.object)
    }

    fn reference_term_has_variable(term: &TermPattern) -> bool {
        match term {
            TermPattern::Variable(_) | TermPattern::BlankNode(_) => true,
            TermPattern::Triple(t) => reference_triple_has_variable(t),
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => false,
        }
    }

    fn reference_compile_term<D: DatasetView>(
        term: &TermPattern,
        schema: &VarSchema,
        dataset: &D,
    ) -> Result<Option<Pos<D::Id>>, EvalError> {
        match term {
            TermPattern::Variable(v) => Ok(Some(Pos::Slot(slot_col(schema, v)))),
            TermPattern::BlankNode(b) => {
                Ok(Some(Pos::Slot(slot_col(schema, &blank_var(b.as_str())))))
            }
            TermPattern::Triple(t) => {
                if reference_triple_has_variable(t) {
                    match reference_compile_triple_pos(t, schema, dataset)? {
                        Some(tp) => Ok(Some(Pos::Triple(Box::new(tp)))),
                        None => Ok(None),
                    }
                } else {
                    let value = ground_term_pattern_to_value(term, "a BGP")?;
                    Ok(dataset
                        .term_id_by_value(&value)
                        .map_err(EvalError::source_read)?
                        .map(Pos::Bound))
                }
            }
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => {
                let value = ground_term_pattern_to_value(term, "a BGP")?;
                Ok(dataset
                    .term_id_by_value(&value)
                    .map_err(EvalError::source_read)?
                    .map(Pos::Bound))
            }
        }
    }

    fn reference_compile_triple_pos<D: DatasetView>(
        triple: &TriplePattern,
        schema: &VarSchema,
        dataset: &D,
    ) -> Result<Option<TriplePos<D::Id>>, EvalError> {
        let Some(s) = reference_compile_term(&triple.subject, schema, dataset)? else {
            return Ok(None);
        };
        let Some(p) = compile_predicate(&triple.predicate, schema, dataset) else {
            return Ok(None);
        };
        let Some(o) = reference_compile_term(&triple.object, schema, dataset)? else {
            return Ok(None);
        };
        Ok(Some(TriplePos { s, p, o }))
    }

    fn reference_bind_pos<D: DatasetView>(
        out: &mut Solution<D::Id>,
        pos: &Pos<D::Id>,
        id: D::Id,
        dataset: &D,
    ) -> bool {
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
            Pos::Triple(t) => {
                let guard = dataset.resolve(id).unwrap();
                match guard.term() {
                    TermRef::Triple { s, p, o } => {
                        reference_bind_pos(out, &t.s, s, dataset)
                            && reference_bind_pos(out, &t.p, p, dataset)
                            && reference_bind_pos(out, &t.o, o, dataset)
                    }
                    _ => false,
                }
            }
        }
    }

    fn reference_term_pattern_to_string(term: &TermPattern) -> String {
        let mut out = String::new();
        match term {
            TermPattern::Variable(v) => format!("?{}", v.as_str()),
            TermPattern::BlankNode(b) => format!("_:{}", b.as_str()),
            TermPattern::NamedNode(n) => {
                purrdf_lex::term_syntax::write_iri(n.as_str(), &mut out);
                out
            }
            TermPattern::Literal(l) => {
                purrdf_lex::term_syntax::write_literal(
                    l.value(),
                    l.datatype().as_str(),
                    l.language(),
                    l.direction().map(BaseDirection::as_str),
                    &mut out,
                );
                out
            }
            TermPattern::Triple(t) => format!(
                "<<( {} {} {} )>>",
                reference_term_pattern_to_string(&t.subject),
                named_node_pattern_to_string(&t.predicate),
                reference_term_pattern_to_string(&t.object)
            ),
        }
    }

    // ── A recording view ───────────────────────────────────────────────────────────

    /// A view over a dataset that records every lookup made through it, so two walks
    /// can be compared on WHICH terms they resolved and in WHAT order, not only on
    /// their answers.
    struct Recording<'a> {
        inner: &'a RdfDataset,
        log: RefCell<Vec<String>>,
    }

    impl<'a> Recording<'a> {
        fn over(inner: &'a RdfDataset) -> Self {
            Self {
                inner,
                log: RefCell::new(Vec::new()),
            }
        }

        fn take_log(&self) -> Vec<String> {
            std::mem::take(&mut *self.log.borrow_mut())
        }
    }

    impl DatasetView for Recording<'_> {
        type Id = TermId;
        type ReadError = std::convert::Infallible;
        type TermGuard<'a>
            = TermRef<'a, Self::Id>
        where
            Self: 'a;
        type ProbePlan = ();

        fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.quads()
        }

        fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
            Ok({
                self.log
                    .borrow_mut()
                    .push(format!("resolve {}", id.index()));
                self.inner.resolve(id)
            })
        }

        fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
            Ok({
                self.log.borrow_mut().push(format!("lookup {value:?}"));
                self.inner.term_id_by_value(value)
            })
        }

        fn capabilities(&self) -> RdfStoreCapabilities {
            self.inner.capabilities()
        }

        fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}

        fn quads_for_pattern_with_plan(
            &self,
            _plan: &(),
            s: Option<TermId>,
            p: Option<TermId>,
            o: Option<TermId>,
            g: GraphMatch,
        ) -> impl Iterator<Item = QuadIds> + '_ {
            self.inner.quads_for_pattern(s, p, o, g)
        }

        fn term_count(&self) -> u64 {
            u64::try_from({ self.inner.term_count() }).expect("bounded local count fits u64")
        }
    }

    /// A view whose terms are one chain of quoted triples `depth` levels deep: index 0
    /// is the IRI every subject holds, index 1 the predicate, and index `k ≥ 2` the
    /// triple whose object is index `k + 1` — the last one's object is index 0. No
    /// frozen dataset holds a term this deep, so the chain is served by the view.
    struct Chain {
        depth: u32,
        leaf: String,
        predicate: String,
    }

    impl Chain {
        fn new(depth: usize) -> Self {
            Self {
                depth: u32::try_from(depth).expect("the depth fits"),
                leaf: format!("{EX}leaf"),
                predicate: format!("{EX}p"),
            }
        }

        /// The id of the outermost triple.
        fn top(&self) -> TermId {
            TermId::from_index(2)
        }
    }

    impl DatasetView for Chain {
        type Id = TermId;
        type ReadError = std::convert::Infallible;
        type TermGuard<'a>
            = TermRef<'a, Self::Id>
        where
            Self: 'a;
        type ProbePlan = ();

        fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            std::iter::empty()
        }

        fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
            Ok({
                let index = u32::try_from(id.index()).expect("the index fits");
                match index {
                    0 => TermRef::Iri(&self.leaf),
                    1 => TermRef::Iri(&self.predicate),
                    _ => TermRef::Triple {
                        s: TermId::from_index(0),
                        p: TermId::from_index(1),
                        o: if index == self.depth + 1 {
                            TermId::from_index(0)
                        } else {
                            TermId::from_index(index + 1)
                        },
                    },
                }
            })
        }

        fn term_id_by_value(
            &self,
            _value: &TermValue,
        ) -> Result<Option<Self::Id>, Self::ReadError> {
            Ok(None)
        }

        fn capabilities(&self) -> RdfStoreCapabilities {
            RdfStoreCapabilities::default()
        }

        fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}

        fn quads_for_pattern_with_plan(
            &self,
            _plan: &(),
            _s: Option<TermId>,
            _p: Option<TermId>,
            _o: Option<TermId>,
            _g: GraphMatch,
        ) -> impl Iterator<Item = QuadIds> + '_ {
            std::iter::empty()
        }

        fn term_count(&self) -> u64 {
            u64::try_from({ usize::try_from(self.depth).expect("the depth fits") + 2 })
                .expect("bounded local count fits u64")
        }
    }

    // ── Fixtures and generators ────────────────────────────────────────────────────

    /// A dataset holding IRIs `a`..`d`, predicates `p`/`q`, a literal of each shape and
    /// quoted triples nested three deep, so a generated pattern's ground positions
    /// sometimes name a term the dataset holds and sometimes one it does not. Every
    /// triple term nests only in the OBJECT of another triple term and every asserted
    /// subject is an IRI, the RDF 1.2 term model the freeze gate enforces.
    fn dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(&format!("{EX}a"));
        let bb = b.intern_iri(&format!("{EX}b"));
        let c = b.intern_iri(&format!("{EX}c"));
        let p = b.intern_iri(&format!("{EX}p"));
        let q = b.intern_iri(&format!("{EX}q"));
        let plain = b.intern_literal(RdfLiteral::simple("plain"));
        let one = b.intern_literal(RdfLiteral::typed(
            "1",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        let t1 = b.intern_triple(a, p, bb);
        let t2 = b.intern_triple(c, q, t1);
        let t3 = b.intern_triple(a, p, t2);
        let t4 = b.intern_triple(bb, p, t1);
        b.push_quad(a, p, bb, None);
        b.push_quad(bb, q, plain, None);
        b.push_quad(c, p, one, None);
        b.push_quad(a, q, t1, None);
        b.push_quad(c, p, t2, None);
        b.push_quad(a, q, t3, None);
        b.push_quad(bb, q, t4, None);
        b.freeze().expect("freeze")
    }

    /// Every term id `dataset` holds.
    fn ids(dataset: &RdfDataset) -> Vec<TermId> {
        (0..dataset.term_count())
            .map(|index| TermId::from_index(u32::try_from(index).expect("the index fits")))
            .collect()
    }

    /// A compiled position over `COLUMNS` slot columns and the ids of `ids`.
    fn position(choices: &mut Choices, ids: &[TermId]) -> Pos {
        if !choices.spend() {
            return Pos::Slot(choices.choose(COLUMNS));
        }
        match choices.choose(5) {
            0 | 1 => Pos::Slot(choices.choose(COLUMNS)),
            2 => Pos::Bound(ids[choices.choose(ids.len())]),
            _ => Pos::Triple(Box::new(TriplePos {
                s: position(choices, ids),
                p: if choices.choose(2) == 0 {
                    Pos::Slot(choices.choose(COLUMNS))
                } else {
                    Pos::Bound(ids[choices.choose(ids.len())])
                },
                o: position(choices, ids),
            })),
        }
    }

    /// A compiled position that follows the structure of the term `id` names in
    /// `dataset`, with a slot or a ground constant in place of some components, so a
    /// binding against `id` sometimes succeeds and sometimes fails partway.
    fn mirroring(choices: &mut Choices, dataset: &RdfDataset, id: TermId) -> Pos {
        match choices.choose(6) {
            0 => Pos::Slot(choices.choose(COLUMNS)),
            1 => Pos::Bound(ids(dataset)[choices.choose(dataset.term_count())]),
            _ => match dataset.resolve(id) {
                TermRef::Triple { s, p, o } => Pos::Triple(Box::new(TriplePos {
                    s: mirroring(choices, dataset, s),
                    p: if choices.choose(3) == 0 {
                        Pos::Slot(choices.choose(COLUMNS))
                    } else {
                        Pos::Bound(p)
                    },
                    o: mirroring(choices, dataset, o),
                })),
                _ => Pos::Bound(id),
            },
        }
    }

    /// A ground quoted triple spelling one of the four triple terms `dataset` interns,
    /// so a generated ground quoted triple is sometimes one the dataset holds whole —
    /// an arbitrary ground triple over the vocabulary almost never is.
    fn held_triple(which: usize) -> TermPattern {
        let triple = |s: TermPattern, p: &str, o: TermPattern| {
            TermPattern::Triple(Child::new(TriplePattern {
                subject: s,
                predicate: NamedNodePattern::NamedNode(iri(p)),
                object: o,
            }))
        };
        let node = |local: &str| TermPattern::NamedNode(iri(local));
        let t1 = || triple(node("a"), "p", node("b"));
        let t2 = || triple(node("c"), "q", t1());
        match which {
            0 => t1(),
            1 => t2(),
            2 => triple(node("a"), "p", t2()),
            _ => triple(node("b"), "p", t1()),
        }
    }

    /// A term pattern over the fixture's vocabulary: variables, blank nodes, IRIs the
    /// dataset holds or lacks, literals of every shape, nested quoted triples, and the
    /// quoted triples the dataset holds.
    fn term_pattern(choices: &mut Choices) -> TermPattern {
        if !choices.spend() {
            return TermPattern::Variable(Variable::new(format!("v{}", choices.choose(3))));
        }
        match choices.choose(10) {
            9 => held_triple(choices.choose(4)),
            0 => TermPattern::Variable(Variable::new(format!("v{}", choices.choose(3)))),
            1 => TermPattern::BlankNode(BlankNode::new(format!("b{}", choices.choose(2)))),
            2 | 3 => TermPattern::NamedNode(iri(["a", "b", "c", "d"][choices.choose(4)])),
            4 => TermPattern::Literal(match choices.choose(4) {
                0 => Literal::new_simple("plain"),
                1 => Literal::new_typed(
                    "1",
                    NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
                ),
                2 => Literal::new_lang("say \"hi\"", "en", None),
                _ => Literal::new_lang("שלום", "he", Some(BaseDirection::Rtl)),
            }),
            _ => TermPattern::Triple(Child::new(triple_pattern(choices))),
        }
    }

    fn predicate(choices: &mut Choices) -> NamedNodePattern {
        match choices.choose(4) {
            0 => NamedNodePattern::Variable(Variable::new(format!("v{}", choices.choose(3)))),
            1 => NamedNodePattern::NamedNode(iri("q")),
            2 => NamedNodePattern::NamedNode(iri("missing")),
            _ => NamedNodePattern::NamedNode(iri("p")),
        }
    }

    fn triple_pattern(choices: &mut Choices) -> TriplePattern {
        TriplePattern {
            subject: term_pattern(choices),
            predicate: predicate(choices),
            object: term_pattern(choices),
        }
    }

    /// A position `depth` quoted triples deep: each level's subject is `leaf`, its
    /// predicate slot 0 and its object the next level, the deepest object slot 1.
    fn deep_position(depth: usize, leaf: TermId) -> Pos {
        let mut pos = Pos::Slot(1);
        for _ in 0..depth {
            pos = Pos::Triple(Box::new(TriplePos {
                s: Pos::Bound(leaf),
                p: Pos::Slot(0),
                o: pos,
            }));
        }
        pos
    }

    /// A term pattern `depth` quoted triples deep: `<<(<a> <p> <<(… ?v …)>>)>>`.
    fn deep_term_pattern(depth: usize) -> TermPattern {
        let mut term = TermPattern::Variable(Variable::new("v"));
        for _ in 0..depth {
            term = TermPattern::Triple(Child::new(TriplePattern {
                subject: TermPattern::NamedNode(iri("a")),
                predicate: NamedNodePattern::NamedNode(iri("p")),
                object: term,
            }));
        }
        term
    }

    /// A rendering of a compiled position over a work list — `Pos` has no `Debug` —
    /// so two compilations can be compared structurally.
    fn render(pos: &Pos) -> String {
        use std::fmt::Write as _;
        enum Piece<'a> {
            Pos(&'a Pos),
            Text(&'static str),
        }
        let mut out = String::new();
        let mut pending = vec![Piece::Pos(pos)];
        while let Some(piece) = pending.pop() {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Pos(Pos::Slot(c)) => write!(out, "?{c}").expect("a String write"),
                Piece::Pos(Pos::Bound(id)) => {
                    write!(out, "#{}", id.index()).expect("a String write");
                }
                Piece::Pos(Pos::Triple(t)) => {
                    out.push('(');
                    pending.extend([
                        Piece::Text(")"),
                        Piece::Pos(&t.o),
                        Piece::Text(" "),
                        Piece::Pos(&t.p),
                        Piece::Text(" "),
                        Piece::Pos(&t.s),
                    ]);
                }
            }
        }
        out
    }

    fn render_compiled(compiled: &Result<Option<Pos>, EvalError>) -> String {
        match compiled {
            Ok(Some(pos)) => render(pos),
            Ok(None) => "none".to_owned(),
            Err(error) => format!("error: {error}"),
        }
    }

    /// How many quoted-triple levels `pos` nests, counted over a work list.
    fn nesting(pos: &Pos) -> usize {
        let mut deepest = 0;
        let mut pending = vec![(pos, 0)];
        while let Some((pos, depth)) = pending.pop() {
            if let Pos::Triple(t) = pos {
                deepest = deepest.max(depth + 1);
                pending.extend([(&t.s, depth + 1), (&t.p, depth + 1), (&t.o, depth + 1)]);
            }
        }
        deepest
    }

    // ── The tests ──────────────────────────────────────────────────────────────────

    /// The bound-slot test, the slot visit, the shape hash and — through the
    /// structural order it decides — the constrained test answer as their recursive
    /// references do, for every generated position.
    #[test]
    fn position_walks_agree_with_their_references_on_generated_positions() {
        let dataset = dataset();
        let ids = ids(&dataset);
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let pos = position(&mut choices, &ids);
            nested += usize::from(matches!(pos, Pos::Triple(_)));
            let bound: Vec<bool> = (0..COLUMNS).map(|_| choices.choose(2) == 0).collect();
            let context = format!("seed {seed}: {}", render(&pos));
            assert_eq!(
                pos_has_bound_slot(&pos, &bound),
                reference_pos_has_bound_slot(&pos, &bound),
                "{context}"
            );
            let mut visited = Vec::new();
            for_each_slot(&pos, &mut |c| visited.push(c));
            let mut expected = Vec::new();
            reference_for_each_slot(&pos, &mut |c| expected.push(c));
            assert_eq!(visited, expected, "{context}");
            let mut ours = purrdf_hash::fixed::FixedHasher::default();
            hash_pos(&pos, &mut ours);
            let mut theirs = purrdf_hash::fixed::FixedHasher::default();
            reference_hash_pos(&pos, &mut theirs);
            assert_eq!(ours.finish(), theirs.finish(), "{context}");
            // `constrained` is private to `structural_order`: with nothing bound yet
            // and neither pattern connected, the order puts first whichever of the two
            // patterns has more constrained positions, the lower index on a tie — so
            // the generated position's constrained count decides it.
            let all_free = vec![false; COLUMNS];
            let generated_constrained = usize::from(reference_constrained(&pos, &all_free));
            let compiled = [
                CompiledPattern {
                    s: Pos::Slot(0),
                    p: Pos::Slot(1),
                    o: Pos::Slot(2),
                },
                CompiledPattern {
                    s: Pos::Slot(3),
                    p: Pos::Slot(3),
                    o: pos,
                },
            ];
            let expected_order = if generated_constrained > 0 {
                vec![1, 0]
            } else {
                vec![0, 1]
            };
            assert_eq!(structural_order(&compiled), expected_order, "{context}");
        }
        assert!(
            nested > 100,
            "the generator nests positions ({nested} of 400 are quoted triples)"
        );
    }

    /// Slot-key collection and the plan-introspection rendering answer as their
    /// recursive references do, for every generated triple pattern.
    #[test]
    fn explain_writes_literals_in_term_syntax_with_the_direction_token() {
        let rtl =
            TermPattern::Literal(Literal::new_lang("a\"b\\c", "ar", Some(BaseDirection::Rtl)));
        assert_eq!(term_pattern_to_string(&rtl), "\"a\\\"b\\\\c\"@ar--rtl");
        let ltr = TermPattern::Literal(Literal::new_lang("x", "en", Some(BaseDirection::Ltr)));
        assert_eq!(term_pattern_to_string(&ltr), "\"x\"@en--ltr");
        // A non-directional literal keeps its tag or datatype.
        let tagged = TermPattern::Literal(Literal::new_lang("x", "en", None));
        assert_eq!(term_pattern_to_string(&tagged), "\"x\"@en");
        let typed = TermPattern::Literal(Literal::new_typed(
            "1",
            NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
        ));
        assert_eq!(
            term_pattern_to_string(&typed),
            "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
        // A backslash in the lexical form is escaped, not left to swallow the quote.
        let slash = TermPattern::Literal(Literal::new_simple("a\\"));
        assert_eq!(term_pattern_to_string(&slash), "\"a\\\\\"");
        let triple = TermPattern::Triple(Child::new(TriplePattern {
            subject: TermPattern::NamedNode(NamedNode::new_unchecked("http://example.org/s")),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                "http://example.org/p",
            )),
            object: TermPattern::Variable(Variable::new("o")),
        }));
        assert_eq!(
            term_pattern_to_string(&triple),
            "<<( <http://example.org/s> <http://example.org/p> ?o )>>"
        );
    }

    #[test]
    fn slot_keys_and_rendering_agree_with_their_references_on_generated_patterns() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let pattern = triple_pattern(&mut choices);
            nested += usize::from(
                matches!(pattern.subject, TermPattern::Triple(_))
                    || matches!(pattern.object, TermPattern::Triple(_)),
            );
            let mut keys = Vec::new();
            collect_triple_slot_keys(&pattern, &mut keys);
            let mut expected = Vec::new();
            reference_collect_triple_slot_keys(&pattern, &mut expected);
            assert_eq!(keys, expected, "seed {seed}: {pattern:?}");
            assert_eq!(slot_keys(&pattern).to_vec(), expected, "seed {seed}");
            for term in [&pattern.subject, &pattern.object] {
                assert_eq!(
                    term_pattern_to_string(term),
                    reference_term_pattern_to_string(term),
                    "seed {seed}: {term:?}"
                );
            }
            assert_eq!(
                triple_pattern_to_string(&pattern),
                format!(
                    "{} {} {} .",
                    reference_term_pattern_to_string(&pattern.subject),
                    named_node_pattern_to_string(&pattern.predicate),
                    reference_term_pattern_to_string(&pattern.object)
                ),
                "seed {seed}"
            );
        }
        assert!(
            nested > 100,
            "the generator nests patterns ({nested} of 400)"
        );
    }

    /// Compiling a term position gives the reference's answer AND makes the
    /// reference's dataset lookups, in the reference's order, for every generated
    /// pattern — nested structural triples, ground quoted triples looked up whole,
    /// and constants the dataset lacks alike.
    #[test]
    fn compilation_agrees_with_its_reference_on_generated_patterns() {
        let dataset = dataset();
        let recording = Recording::over(&dataset);
        let (mut structural, mut ground, mut absent) = (0, 0, 0);
        for seed in 0..600_u64 {
            let mut choices = Choices::new(seed);
            let pattern = triple_pattern(&mut choices);
            let schema = Arc::new(VarSchema::from_vars(slot_keys(&pattern)));
            for term in [&pattern.subject, &pattern.object] {
                let ours = compile_term(term, &schema, &recording);
                let our_log = recording.take_log();
                let theirs = reference_compile_term(term, &schema, &recording);
                let their_log = recording.take_log();
                assert_eq!(
                    render_compiled(&ours),
                    render_compiled(&theirs),
                    "seed {seed}: {term:?}"
                );
                assert_eq!(our_log, their_log, "seed {seed}: {term:?}");
                match &ours {
                    Ok(Some(Pos::Triple(_))) => structural += 1,
                    Ok(Some(Pos::Bound(_))) if matches!(term, TermPattern::Triple(_)) => {
                        ground += 1;
                    }
                    Ok(None) => absent += 1,
                    _ => {}
                }
            }
        }
        assert!(
            structural > 50 && ground > 10 && absent > 50,
            "the generator reaches every compilation outcome ({structural} structural, \
             {ground} ground quoted triples, {absent} absent)"
        );
    }

    /// Binding a position against a candidate gives the reference's verdict, leaves the
    /// row as the reference leaves it, and resolves the same terms in the same order,
    /// for every generated position and candidate.
    #[test]
    fn binding_agrees_with_its_reference_on_generated_positions() {
        let dataset = dataset();
        let all = ids(&dataset);
        let recording = Recording::over(&dataset);
        let (mut bound, mut refused) = (0, 0);
        for seed in 0..600_u64 {
            let mut choices = Choices::new(seed);
            let candidate = all[choices.choose(all.len())];
            let pos = if choices.choose(2) == 0 {
                mirroring(&mut choices, &dataset, candidate)
            } else {
                position(&mut choices, &all)
            };
            let row: Solution = (0..COLUMNS)
                .map(|_| {
                    (choices.choose(3) == 0)
                        .then(|| SolutionTerm::Existing(all[choices.choose(all.len())]))
                })
                .collect();
            let mut ours = row.clone();
            let our_verdict = bind_pos(&mut ours, &pos, candidate, &recording);
            let our_log = recording.take_log();
            let mut theirs = row;
            let their_verdict = reference_bind_pos(&mut theirs, &pos, candidate, &recording);
            let their_log = recording.take_log();
            let context = format!(
                "seed {seed}: {} against {}",
                render(&pos),
                candidate.index()
            );
            assert_eq!(our_verdict, their_verdict, "{context}");
            assert_eq!(ours, theirs, "{context}");
            assert_eq!(our_log, their_log, "{context}");
            if our_verdict {
                bound += 1;
            } else {
                refused += 1;
            }
        }
        assert!(
            bound > 50 && refused > 50,
            "the generator binds and refuses ({bound} bound, {refused} refused)"
        );
    }

    /// A position a hundred thousand quoted triples deep is tested for bound slots,
    /// visited, hashed and released; a term pattern as deep has its slot keys
    /// collected, is rendered and is compiled; and the compiled position is bound
    /// against a term chain as deep — all on a thread with a 128 KiB stack.
    #[test]
    fn a_hundred_thousand_level_position_is_walked_on_a_128_kib_stack() {
        purrdf_stack::on_stack(SMALL_STACK, || {
            let chain = Chain::new(DEPTH);
            let leaf = TermId::from_index(0);
            let deep = deep_position(DEPTH, leaf);
            let bound = [false, true];
            assert!(pos_has_bound_slot(&deep, &bound));
            assert!(!pos_has_bound_slot(&deep, &[false, false]));
            let mut slots = 0;
            for_each_slot(&deep, &mut |_| slots += 1);
            assert_eq!(slots, DEPTH + 1);
            let mut hasher = purrdf_hash::fixed::FixedHasher::default();
            hash_pos(&deep, &mut hasher);
            let hash = hasher.finish();
            let mut again = purrdf_hash::fixed::FixedHasher::default();
            hash_pos(&deep_position(DEPTH, leaf), &mut again);
            assert_eq!(hash, again.finish());
            assert_eq!(
                structural_order(&[CompiledPattern {
                    s: Pos::Slot(0),
                    p: Pos::Slot(1),
                    o: deep_position(DEPTH, leaf),
                }]),
                vec![0]
            );

            // Binding: every level's subject is the leaf, every predicate binds slot 0
            // to the chain's predicate, and the deepest object binds slot 1.
            let mut row: Solution = [None, None].into_iter().collect();
            assert!(bind_pos(&mut row, &deep, chain.top(), &chain));
            assert_eq!(
                row.as_slice(),
                &[
                    Some(SolutionTerm::Existing(TermId::from_index(1))),
                    Some(SolutionTerm::Existing(leaf)),
                ]
            );
            // A row whose slot 0 already disagrees is refused at the first level.
            let mut disagreeing: Solution = [Some(SolutionTerm::Existing(leaf)), None]
                .into_iter()
                .collect();
            assert!(!bind_pos(&mut disagreeing, &deep, chain.top(), &chain));
            drop(deep);

            let term = deep_term_pattern(DEPTH);
            let mut keys = Vec::new();
            collect_triple_slot_keys(
                &TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(iri("p")),
                    object: term.clone(),
                },
                &mut keys,
            );
            assert_eq!(keys, vec![Variable::new("s"), Variable::new("v")]);
            let rendered = term_pattern_to_string(&term);
            assert!(rendered.starts_with(&format!("<<( <{EX}a> <{EX}p> <<( <{EX}a> <{EX}p> ")));
            assert!(rendered.ends_with(&format!("?v{}", " )>>".repeat(DEPTH))));
            assert_eq!(
                rendered.len(),
                DEPTH * (format!("<<( <{EX}a> <{EX}p>  )>>").len()) + 2
            );

            let dataset = dataset();
            let schema = Arc::new(VarSchema::from_vars([Variable::new("v")]));
            let compiled = compile_term(&term, &schema, dataset.as_ref())
                .expect("compiles")
                .expect("every constant is held");
            assert_eq!(nesting(&compiled), DEPTH);
            drop(compiled);
        })
        .expect("spawn");
    }
}

#[cfg(test)]
mod survey_tests {
    //! The plan survey against its recursive reference over generated plans, and at a
    //! hundred thousand levels on a 128 KiB thread.

    use std::sync::Arc;

    use purrdf_core::{DatasetView, GraphMatch, RdfDataset, RdfDatasetBuilder, RdfLiteral};
    use purrdf_sparql_algebra::{
        Chain, Child, Expression, GraphPattern, GroundTerm, Literal, NamedNode, NamedNodePattern,
        NodeRef, OrderExpression, PropertyFunctionCall, PropertyPathExpression, TermPattern,
        TriplePattern, Variable,
    };

    use super::{PlanSurvey, record_call_estimate, survey_bgp, survey_pattern_plans};
    use crate::DetHashSet;
    use crate::dataset_spec::ActiveDataset;
    use crate::error::EvalError;
    use crate::governor::ledger::PlanEstimate;
    use crate::property_fn::{
        PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    };
    use crate::user_fn::Volatility;
    use purrdf_core::binding_pattern::BindingPattern;

    const EX: &str = "http://example.org/";
    const DEPTH: usize = 100_000;
    const SMALL_STACK: usize = 128 * 1024;
    /// The IRI of the one relation the registry holds.
    const RELATION: &str = "http://example.org/rel";
    /// An IRI no registry resolves.
    const UNKNOWN: &str = "http://example.org/unknown";
    /// What [`Declared`] declares it emits per invocation.
    const ROWS_PER_INVOCATION: u64 = 7;

    // ── The recursive reference ────────────────────────────────────────────────────

    /// The survey as a recursion over the algebra: each node's children in written order,
    /// a call priced after the whole subtree to its left.
    fn reference_survey<D: DatasetView>(
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        pattern: &GraphPattern,
        relations: &PropertyFunctionRegistry,
        survey: &mut PlanSurvey,
    ) -> Result<(), EvalError> {
        match pattern {
            GraphPattern::Bgp { patterns } => survey_bgp(
                dataset,
                active_dataset,
                active_graph,
                pattern,
                patterns,
                survey,
            ),
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                reference_survey(
                    dataset,
                    active_dataset,
                    active_graph,
                    left,
                    relations,
                    survey,
                )?;
                if let GraphPattern::PropertyFunction(call) = &**right {
                    let mut bound = DetHashSet::default();
                    crate::property_fn_plan::collect_certainly_bound(left, &mut bound);
                    record_call_estimate(
                        right,
                        call,
                        &bound,
                        reference_predicted_rows(left, survey).unwrap_or(1),
                        relations,
                        survey,
                    )
                } else {
                    reference_survey(
                        dataset,
                        active_dataset,
                        active_graph,
                        right,
                        relations,
                        survey,
                    )
                }
            }
            GraphPattern::Union { arms } => {
                for arm in arms {
                    reference_survey(
                        dataset,
                        active_dataset,
                        active_graph,
                        arm,
                        relations,
                        survey,
                    )?;
                }
                Ok(())
            }
            GraphPattern::LeftJoin { left, right, .. } | GraphPattern::Minus { left, right } => {
                reference_survey(
                    dataset,
                    active_dataset,
                    active_graph,
                    left,
                    relations,
                    survey,
                )?;
                reference_survey(
                    dataset,
                    active_dataset,
                    active_graph,
                    right,
                    relations,
                    survey,
                )
            }
            GraphPattern::Filter { inner, .. }
            | GraphPattern::Extend { inner, .. }
            | GraphPattern::Unfold { inner, .. }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::OrderBy { inner, .. }
            | GraphPattern::Group { inner, .. } => reference_survey(
                dataset,
                active_dataset,
                active_graph,
                inner,
                relations,
                survey,
            ),
            GraphPattern::Graph { name, inner } => {
                let inner_graph = match name {
                    NamedNodePattern::NamedNode(n) => dataset
                        .term_id_by_value(&crate::convert::named_node_to_value(n))
                        .unwrap()
                        .map_or(GraphMatch::Default, GraphMatch::Named),
                    NamedNodePattern::Variable(_) => GraphMatch::Any,
                };
                reference_survey(
                    dataset,
                    active_dataset,
                    inner_graph,
                    inner,
                    relations,
                    survey,
                )
            }
            GraphPattern::PropertyFunction(call) => {
                record_call_estimate(pattern, call, &DetHashSet::default(), 1, relations, survey)
            }
            GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::Service { .. } => Ok(()),
        }
    }

    /// [`super::predicted_rows`] as a recursion down a `LATERAL` chain's right spine.
    fn reference_predicted_rows(pattern: &GraphPattern, survey: &PlanSurvey) -> Option<u64> {
        if let Some(estimate) = survey.estimate_of(pattern) {
            return Some(estimate.rows);
        }
        match pattern {
            GraphPattern::Lateral { right, .. } => reference_predicted_rows(right, survey),
            _ => None,
        }
    }

    // ── Fixtures ───────────────────────────────────────────────────────────────────

    /// A relation declaring one subject and one object argument, every mode free, and
    /// [`ROWS_PER_INVOCATION`] rows per call; never dispatched.
    #[derive(Debug)]
    struct Declared {
        modes: [BindingPattern; 1],
    }

    impl Declared {
        fn new() -> Self {
            Self {
                modes: [PfArity::new(1, 1).all_free_mode()],
            }
        }
    }

    /// The empty cursor [`Declared::open`] hands out.
    struct EmptyCursor;

    impl PfCursor for EmptyCursor {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            Ok(None)
        }
    }

    impl PropertyFunction for Declared {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            ROWS_PER_INVOCATION
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Ok(Box::new(EmptyCursor))
        }
    }

    fn relations() -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(RELATION, Arc::new(Declared::new()));
        registry
    }

    /// `<a> <p> <b>` and `<a> <q> "1"` in the default graph, `<b> <p> <c>` in `<g>`.
    fn dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(&format!("{EX}a"));
        let bb = b.intern_iri(&format!("{EX}b"));
        let c = b.intern_iri(&format!("{EX}c"));
        let p = b.intern_iri(&format!("{EX}p"));
        let q = b.intern_iri(&format!("{EX}q"));
        let g = b.intern_iri(&format!("{EX}g"));
        let one = b.intern_literal(RdfLiteral::typed(
            "1",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        b.push_quad(a, p, bb, None);
        b.push_quad(a, q, one, None);
        b.push_quad(bb, p, c, Some(g));
        b.freeze().expect("the fixture dataset")
    }

    /// A deterministic choice sequence.
    struct Choices {
        state: purrdf_testkit::rng::SplitMix64,
        budget: usize,
    }

    impl Choices {
        const fn new(seed: u64) -> Self {
            Self {
                state: purrdf_testkit::rng::SplitMix64::new(seed),
                budget: 24,
            }
        }

        /// One choice below `n`.
        fn choose(&mut self, n: usize) -> usize {
            self.state.below_usize(n)
        }

        /// Whether another nesting level may be spent.
        fn spend(&mut self) -> bool {
            if self.budget == 0 {
                return false;
            }
            self.budget -= 1;
            true
        }
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{EX}{local}"))
    }

    fn var(name: &str) -> Variable {
        Variable::new(name)
    }

    fn call(iri_text: &str) -> GraphPattern {
        GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: iri_text.to_owned(),
            subject_args: vec![TermPattern::Variable(var("s"))],
            object_args: vec![TermPattern::Variable(var("o"))],
        })
    }

    /// The one-pattern `?s <p> ?o`.
    fn bgp() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(var("s")),
                predicate: NamedNodePattern::NamedNode(iri("p")),
                object: TermPattern::Variable(var("o")),
            }],
        }
    }

    /// One to three triple patterns: subjects and objects variable or ground, one
    /// predicate in five absent from the dataset, so both the ordered and the empty
    /// estimate arise.
    fn generated_bgp(choices: &mut Choices) -> GraphPattern {
        let count = 1 + choices.choose(3);
        let patterns = (0..count)
            .map(|k| TriplePattern {
                subject: if choices.choose(3) == 0 {
                    TermPattern::NamedNode(iri("a"))
                } else {
                    TermPattern::Variable(var(&format!("s{}", k % 2)))
                },
                predicate: NamedNodePattern::NamedNode(match choices.choose(5) {
                    0 => iri("missing"),
                    1 => iri("q"),
                    _ => iri("p"),
                }),
                object: match choices.choose(4) {
                    0 => TermPattern::NamedNode(iri("b")),
                    1 => TermPattern::Literal(Literal::new_simple("1")),
                    _ => TermPattern::Variable(var(&format!("o{k}"))),
                },
            })
            .collect();
        GraphPattern::Bgp { patterns }
    }

    fn leaf(choices: &mut Choices) -> GraphPattern {
        match choices.choose(8) {
            0..=3 => generated_bgp(choices),
            4 => call(RELATION),
            5 => call(UNKNOWN),
            6 => GraphPattern::Path {
                subject: TermPattern::Variable(var("s")),
                path: PropertyPathExpression::NamedNode(iri("p")),
                object: TermPattern::Variable(var("o")),
            },
            _ => GraphPattern::Values {
                variables: vec![var("s")],
                bindings: vec![vec![Some(GroundTerm::NamedNode(iri("a")))]],
            },
        }
    }

    fn pattern(choices: &mut Choices) -> GraphPattern {
        if !choices.spend() {
            return leaf(choices);
        }
        match choices.choose(18) {
            0 | 1 => leaf(choices),
            2 => GraphPattern::Join {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            3 => GraphPattern::Join {
                left: Child::new(pattern(choices)),
                right: Child::new(call(if choices.choose(3) == 0 {
                    UNKNOWN
                } else {
                    RELATION
                })),
            },
            4 => GraphPattern::Lateral {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            5 => GraphPattern::Lateral {
                left: Child::new(pattern(choices)),
                right: Child::new(call(RELATION)),
            },
            6 => {
                let first = pattern(choices);
                let second = pattern(choices);
                let rest: Vec<GraphPattern> =
                    (0..choices.choose(2)).map(|_| pattern(choices)).collect();
                GraphPattern::Union {
                    arms: Chain::new(first, second, rest),
                }
            }
            7 => GraphPattern::LeftJoin {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
                expression: None,
            },
            8 => GraphPattern::Minus {
                left: Child::new(pattern(choices)),
                right: Child::new(pattern(choices)),
            },
            kind => {
                let inner = pattern(choices);
                wrapped(choices, kind, inner)
            }
        }
    }

    /// `inner` under the unary wrapper `kind` (9 to 17) names.
    fn wrapped(choices: &mut Choices, kind: usize, inner: GraphPattern) -> GraphPattern {
        let inner = Child::new(inner);
        let variable = |name: &str| Expression::Variable(var(name));
        match kind {
            9 => GraphPattern::Filter {
                expr: variable("s"),
                inner,
            },
            10 => GraphPattern::Extend {
                inner,
                variable: var("x"),
                expression: variable("s"),
            },
            11 => GraphPattern::Unfold {
                inner,
                expression: variable("s"),
                element: var("e"),
                companion: None,
            },
            12 => GraphPattern::Project {
                inner,
                variables: vec![var("s")],
            },
            13 => GraphPattern::Distinct { inner },
            14 => GraphPattern::Slice {
                inner,
                start: 0,
                length: Some(1),
            },
            15 => GraphPattern::OrderBy {
                inner,
                expression: vec![OrderExpression::Asc(variable("s"))],
            },
            16 => GraphPattern::Group {
                inner,
                variables: vec![var("s")],
                aggregates: Vec::new(),
            },
            _ => GraphPattern::Graph {
                name: match choices.choose(3) {
                    0 => NamedNodePattern::Variable(var("g")),
                    1 => NamedNodePattern::NamedNode(iri("g")),
                    _ => NamedNodePattern::NamedNode(iri("nowhere")),
                },
                inner,
            },
        }
    }

    /// How many calls of the registered relation `plan` attaches to the right of a
    /// `JOIN` or `LATERAL` whose left arm `survey` predicted: the calls priced against a
    /// driving bag rather than the identity table.
    fn driven_calls(plan: &GraphPattern, survey: &PlanSurvey) -> usize {
        let mut driven = 0;
        let mut pending = vec![NodeRef::Pattern(plan)];
        while let Some(node) = pending.pop() {
            if let NodeRef::Pattern(
                GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right },
            ) = node
                && let GraphPattern::PropertyFunction(call) = &**right
                && call.iri == RELATION
                && survey.estimate_of(left).is_some()
            {
                driven += 1;
            }
            node.for_each_child(|child| pending.push(child));
        }
        driven
    }

    /// A survey's content in a comparable form: its join-order strings in order, and its
    /// estimates by node id.
    fn content(survey: &PlanSurvey) -> (Vec<String>, Vec<(usize, PlanEstimate)>) {
        let estimates: Vec<(usize, PlanEstimate)> = survey
            .estimates
            .iter()
            .enumerate()
            .filter_map(|(id, estimate)| estimate.clone().map(|estimate| (id, estimate)))
            .collect();
        (survey.orders.clone(), estimates)
    }

    // ── The tests ──────────────────────────────────────────────────────────────────

    /// The work-list survey records the same join orders in the same order, the same
    /// estimate at every node and the same outcome as the recursive reference, for
    /// every generated plan — and the generator reaches priced calls, calls driven by a
    /// surveyed left arm, `GRAPH`-scoped estimates and empty estimates.
    #[test]
    fn the_survey_agrees_with_its_recursive_reference_on_generated_plans() {
        let dataset = dataset();
        let active = ActiveDataset::store_default();
        let relations = relations();
        let mut priced_calls = 0;
        let mut driven = 0;
        let mut empty = 0;
        let mut ordered = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let plan = pattern(&mut choices);
            let tree = crate::plan::Tree::build(&plan);
            let mut ours = PlanSurvey::for_shape(tree.shape());
            let answered = survey_pattern_plans(
                &*dataset,
                &active,
                GraphMatch::Default,
                &plan,
                &relations,
                &mut ours,
            );
            let mut theirs = PlanSurvey::for_shape(tree.shape());
            let expected = reference_survey(
                &*dataset,
                &active,
                GraphMatch::Default,
                &plan,
                &relations,
                &mut theirs,
            );
            let context = format!("seed {seed}: {plan:?}");
            assert_eq!(
                answered.as_ref().map_err(ToString::to_string),
                expected.as_ref().map_err(ToString::to_string),
                "{context}"
            );
            assert_eq!(content(&ours), content(&theirs), "{context}");
            assert_eq!(ours.peak_cells(), theirs.peak_cells(), "{context}");
            empty += ours
                .estimates
                .iter()
                .flatten()
                .filter(|estimate| estimate.rows == 0)
                .count();
            priced_calls += ours
                .estimates
                .iter()
                .flatten()
                .filter(|estimate| estimate.rows == ROWS_PER_INVOCATION)
                .count();
            driven += driven_calls(&plan, &ours);
            ordered += usize::from(!ours.orders.is_empty());
        }
        assert!(priced_calls > 0, "some generated call is priced");
        assert!(
            driven > 0,
            "some priced call is driven by a surveyed left arm"
        );
        assert!(empty > 0, "some generated BGP is predicted empty");
        assert!(ordered > 0, "some generated BGP has an order to choose");
    }

    /// A plan a hundred thousand levels tall — wrappers around one BGP, a `LATERAL`
    /// chain ending in a call, and a `JOIN` spine of BGPs — is surveyed whole on a
    /// 128 KiB thread, with the estimate count each shape has by construction.
    #[test]
    fn a_hundred_thousand_levels_are_surveyed_on_a_128_kib_thread() {
        purrdf_stack::on_stack(SMALL_STACK, || {
            let dataset = dataset();
            let active = ActiveDataset::store_default();
            let relations = relations();
            let survey = |plan: &GraphPattern| {
                let tree = crate::plan::Tree::build(plan);
                let mut survey = PlanSurvey::for_shape(tree.shape());
                survey_pattern_plans(
                    &*dataset,
                    &active,
                    GraphMatch::Default,
                    plan,
                    &relations,
                    &mut survey,
                )
                .expect("the deep plan is surveyed");
                survey
            };

            let mut wrapped = bgp();
            for level in 0..DEPTH {
                wrapped = if level.is_multiple_of(2) {
                    GraphPattern::Project {
                        inner: Child::new(wrapped),
                        variables: vec![var("s")],
                    }
                } else {
                    GraphPattern::Filter {
                        expr: Expression::Variable(var("s")),
                        inner: Child::new(wrapped),
                    }
                };
            }
            assert_eq!(
                survey(&wrapped).estimates.iter().flatten().count(),
                1,
                "one BGP under the wrappers"
            );
            drop(wrapped);

            // Every level's right side is the level below; the innermost is the call,
            // driven by the BGP beside it.
            let mut chain = GraphPattern::Lateral {
                left: Child::new(bgp()),
                right: Child::new(call(RELATION)),
            };
            for _ in 1..DEPTH {
                chain = GraphPattern::Lateral {
                    left: Child::new(bgp()),
                    right: Child::new(chain),
                };
            }
            let surveyed = survey(&chain);
            assert_eq!(
                surveyed.estimates.iter().flatten().count(),
                DEPTH + 1,
                "a BGP per level and the call"
            );
            let mut innermost = &chain;
            while let GraphPattern::Lateral { right, .. } = innermost
                && matches!(**right, GraphPattern::Lateral { .. })
            {
                innermost = right;
            }
            let GraphPattern::Lateral { left, right } = innermost else {
                unreachable!("the chain's innermost node is a LATERAL");
            };
            let driving = surveyed
                .estimate_of(left)
                .expect("the left arm is predicted")
                .rows;
            assert_eq!(
                surveyed
                    .estimate_of(right)
                    .expect("the call is priced")
                    .rows,
                ROWS_PER_INVOCATION * driving,
                "the innermost call is priced against the BGP driving it"
            );
            drop(chain);

            let mut spine = bgp();
            for _ in 0..DEPTH {
                spine = GraphPattern::Join {
                    left: Child::new(spine),
                    right: Child::new(bgp()),
                };
            }
            assert_eq!(
                survey(&spine).estimates.iter().flatten().count(),
                DEPTH + 1,
                "a BGP per level"
            );
            drop(spine);
        })
        .expect("spawn");
    }
}
