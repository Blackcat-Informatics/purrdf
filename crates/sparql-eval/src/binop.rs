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

use purrdf_core::{DatasetView, TermId, ViewTermId};
use purrdf_sparql_algebra::{Expression, GraphPattern};

use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::lift::{Evaluated, Lift, Truncation};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema, compatible};
use crate::{DetHashMap, DetHasher};

/// A hash-join key over the shared columns of two solutions.
///
/// The overwhelmingly common case is a **single** shared variable (a star join on
/// `?p`), so that case is specialized to a `Copy` join-key atom ([`SolutionTerm::join_key`])
/// — no per-row heap allocation on either the build or probe side. Joins on zero or
/// ≥2 shared columns keep the general owned-vector key. Within one join the shared
/// column count is fixed, so every key is the same variant and the two never compare
/// across variants.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum JoinKey<I: ViewTermId = TermId> {
    /// Exactly one shared column: the term encoded as a collision-free join-key atom
    /// ([`SolutionTerm::join_key`]). For `I = TermId` this is the historical `u64`.
    Single(I::JoinKeyAtom),
    /// Zero or ≥2 shared columns: the bound terms in shared-column order.
    Multi(Vec<SolutionTerm<I>>),
}

/// Evaluate `left . right` (algebra `Join`) as a hash join on shared variables.
///
/// # Under a truncated child
///
/// A join's output is a function of **both** inputs, so a truncated left arm leaves
/// nothing computable: the right arm is deliberately not evaluated (that would be an
/// unbounded scan after the budget is spent — see [`Lift`]) and the node yields the empty
/// bag, which is a sound lower bound. A truncated right arm joins normally against the
/// rows in hand and yields a sub-bag of the true output — sound as a multiset, and
/// classified [`crate::governor::soundness::PrefixFidelity::BagOnly`] because the missing
/// rows come from the middle of each left row's block rather than from the end.
pub(crate) fn eval_join<D: DatasetView + Sync>(
    node: &GraphPattern,
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(l) = lift.absorb(0, eval_evaluated(left, ctx)?) else {
        return Ok(lift.withheld());
    };
    if lift.is_truncated() {
        return Ok(lift.finish(SolutionSeq::empty(l.schema)));
    }
    let Some(r) = lift.absorb(1, eval_evaluated(right, ctx)?) else {
        return Ok(lift.withheld());
    };
    Ok(lift.finish(hash_join(&l, &r, ctx)))
}

/// Evaluate a graph pattern for ONE outer row μ, substituted in — the single seam
/// shared by `LATERAL`'s per-left-row evaluation ([`eval_lateral`], below) and an
/// expression-correlated `EXISTS`'s per-outer-row evaluation
/// (`crate::expr::exists`'s correlated branch): both need "evaluate `pattern` as
/// if μ's bindings had been joined in below every solution modifier, once, for
/// THIS row alone, uncached" and nothing else. Replaces what were two textually
/// identical blocks (a `bindings`/`substitute_pattern` call pair, a hand-rolled
/// `ctx.in_substituted_exists` save/set/restore, and the guarded `eval_evaluated`
/// call) with one function and one RAII guard
/// ([`crate::eval::EvalCtx::enter_substituted_exists`]).
///
/// # The theorem
///
/// This seam and the parser's `LATERAL` scope-conflict check
/// (`purrdf_sparql_algebra::parser`) satisfy one theorem together: the parser
/// rejects exactly those programs in which injecting bindings across the RHS's
/// top scope level would be observable as a rebinding; [`crate::expr::substitute_pattern`]'s
/// injection never crosses a `Project` boundary the projection does not carry.
/// Together: a parsed `LATERAL` evaluates per SEP-0006's
/// `Lateral(Ω,P) = ⋃ eval(inject(P, μ))`, with `inject` this seam's substitution
/// walk (SEP-0007's `Replace` mechanism — "Values Insertion" — for triple/leaf
/// positions; ordinary constant substitution, unchanged, for expression
/// positions).
///
/// # Errors
///
/// Propagates whatever `pattern`'s evaluation returns — including a truncation,
/// which the caller decides how to fold into its own result (a `LATERAL` row is
/// discarded whole; a correlated `EXISTS` records the truncation on the
/// expression barrier and answers `false`). Neither caller may memoize this
/// result: it is specific to `mu`.
pub(crate) fn eval_correlated<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    mu: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let row = crate::expr::outer_bindings_for_substitution(mu, schema, ctx);
    // `pattern` is a real PLAN node exactly when it (or, for a `LATERAL` nested inside
    // another `LATERAL`'s substituted RHS, its already-installed enclosing map) resolves
    // to a ledger ordinal — a `LATERAL` right operand, never a correlated `EXISTS` inner
    // (which is not walked by `walk_spine` even unsubstituted; see the ledger module
    // doc's "Attribution of work that is not a plan node"). Only then is the substitution
    // walk worth tracking: every node it copies 1:1 keeps its own ledger identity across
    // the per-row substituted copy instead of folding into the enclosing node.
    let source_addr = std::ptr::from_ref(pattern) as usize;
    let (substituted, ledger_map) = if ctx.resolve_ledger_ordinal(source_addr).is_some() {
        // The immediately enclosing window's map, when THIS window is itself substituting a
        // subtree an enclosing `LATERAL` window already substituted (`pattern` is then one
        // of that window's own synthetic copies, not a real plan address) — read before
        // this call's own map is pushed below, so it names exactly the one window `pattern`
        // came from. `substitute_pattern_tracked` uses it to resolve past that window's
        // scaffolding straight to the real plan address, and to arbitrate `counts_rows` so
        // nesting never mints more than one counting node per real ordinal — see
        // `crate::expr::SubstitutionSource`'s doc.
        let enclosing = ctx.correlated_node_maps.last().map(Arc::as_ref);
        let mut map = crate::expr::SubstitutionSourceMap::default();
        let substituted =
            crate::expr::substitute_pattern_tracked(pattern, &row, &mut map, enclosing);
        (substituted, Some(map))
    } else {
        (crate::expr::substitute_pattern(pattern, &row), None)
    };
    // `substituted` is a per-row heap temporary whose node addresses do not
    // outlive this call; the guard flags the window so address-keyed
    // memoization is bypassed while it is evaluated, and restores the prior
    // flag (and pops `ledger_map`, when one was pushed) on drop — even on the
    // `?` this function's caller applies to its result — so nested correlated
    // evaluations compose correctly.
    let mut guard = ctx.enter_substituted_exists(ledger_map);
    eval_evaluated(&substituted, &mut guard)
}

/// Evaluate `LATERAL` (a correlated join): for each left solution μ, evaluate
/// `right` with μ's bindings substituted in as ground constants, then merge each
/// right solution ν with μ over the ordered-union schema.
///
/// Unlike `Join` (which evaluates `right` once, unconstrained), LATERAL evaluates
/// `right` **once per left row** — required when `right`'s *evaluation* depends on
/// μ, the sole case being a variable-endpoint `SERVICE ?g` whose endpoint IRI is
/// bound by μ. Reuses the correlated-EXISTS substitution machinery via
/// [`eval_correlated`], including the address-keyed-cache ABA guard
/// (`in_substituted_exists`) around the inner eval.
///
/// # Under a truncated child
///
/// The right side is evaluated once per left row, so a trip inside it is a trip **in
/// flight**: the left row being processed has not finished producing its output and is
/// discarded whole, while every left row already processed keeps its complete block. That
/// is the commit-per-input-row rule, and it is what makes the surviving rows a sound
/// sub-bag rather than a mixture of complete and half-complete blocks.
pub(crate) fn eval_lateral<D: DatasetView + Sync>(
    node: &GraphPattern,
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(l) = lift.absorb(0, eval_evaluated(left, ctx)?) else {
        return Ok(lift.withheld());
    };
    if lift.is_truncated() {
        return Ok(lift.finish(SolutionSeq::empty(l.schema)));
    }
    // A property-function call is driven PER LEFT ROW with the row in hand, rather than
    // substituted into and re-evaluated like an ordinary right operand. The generic path
    // below would work — the lateral join's compatibility test reconciles everything the
    // IRI-only substitution could not carry — but it would hand the relation the wrong
    // access pattern: a literal, blank-node or quoted-triple binding would arrive as a
    // FREE position, so a relation that can only be invoked with that position bound
    // would be refused an invocation the engine can make. See `crate::property_fn_eval`.
    if let GraphPattern::PropertyFunction(call) = right {
        // The answer-cap / `LIMIT` ceiling the plan licensed for THIS node, read while
        // the cursor is still on it. The interception FUSES `Lateral(left, call)` into
        // one driven operator: the dispatch reads each left row itself and emits rows
        // that are already joined, so what it produces is this node's OUTPUT bag — one
        // row for one row, in order — and the ceiling that bounds it is the one recorded
        // here rather than one pushed to the call node, which carries none (see
        // `crate::governor::soundness::child_row_ceiling`). No new licence is minted:
        // the operator consumes its own node's ceiling, which is what every operator
        // that stops early does.
        let ceiling = ctx.row_ceiling();
        let restore = ctx.enter_node(right);
        // The node-entry charge the generic path pays on each of this node's
        // evaluations, kept here so the call is metered exactly as an ordinary right
        // operand is.
        let evaluated = match ctx.charge(crate::governor::ChargePoint::AlgebraNodeEntry) {
            Err(tripped) => Ok(Evaluated::Truncated(Truncation::origin(
                SolutionSeq::empty(crate::eval::syntactic_schema(right)),
                tripped,
            ))),
            Ok(()) => {
                crate::property_fn_eval::eval_lateral_property_function(call, &l, ceiling, ctx)
            }
        };
        ctx.leave_node(restore);
        let absorbed = lift.absorb(1, evaluated?);
        return Ok(match absorbed {
            Some(seq) => lift.finish(seq),
            None => lift.withheld(),
        });
    }

    let left_schema = Arc::clone(&l.schema);
    let left_len = left_schema.len();

    // Evaluate `right` once per left row with μ substituted in; accumulate the
    // per-row results and the union of their schemas (stable across rows for the
    // SERVICE ?var use, but computed generally).
    let mut right_schema = VarSchema::new();
    // Each left row μ paired with the per-row `right` result it drives.
    type LateralPerRow<I> = Vec<(Solution<I>, SolutionSeq<I>)>;
    let mut per_row: LateralPerRow<D::Id> = Vec::with_capacity(l.rows.len());
    for mu in &l.rows {
        let evaluated = eval_correlated(right, mu, &left_schema, ctx)?;
        let r = match evaluated {
            Evaluated::Complete(seq) => seq,
            truncated @ Evaluated::Truncated(_) => {
                // Commit granularity: this left row's block is incomplete, so it is
                // discarded entirely rather than emitted short. Absorbing here (rather
                // than merging the partial block) is what keeps the surviving rows a
                // sound bound instead of a bag with one truncated block inside it.
                drop(lift.absorb(1, truncated));
                break;
            }
        };
        for v in r.schema.vars() {
            right_schema.push(v.clone());
        }
        per_row.push((mu.clone(), r));
    }

    let out = Arc::new(left_schema.union(&right_schema));
    let out_len = out.len();
    let cell_ceiling = ctx.cell_row_ceiling(out_len);
    let mut rows: Vec<Solution<D::Id>> =
        Vec::with_capacity(cell_ceiling.map_or(per_row.len(), |cap| cap.min(per_row.len())));
    'left: for (mu, r) in &per_row {
        let right_to_out = right_to_out_map(&r.schema, &out);
        for nu in &r.rows {
            // Test compatibility before allocating the joined row. The cell ceiling is
            // an admission boundary, so the first over-limit emit-worthy candidate must
            // be refused before its output storage is constructed.
            let compatible_row = nu.iter().enumerate().all(|(j, cell)| {
                let oi = right_to_out[j];
                !matches!((cell, mu.get(oi).copied().flatten()), (Some(t), Some(existing)) if *t != existing)
            });
            if !compatible_row {
                continue;
            }
            if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                break 'left;
            }

            // Start from μ: left columns are out[0..left_len] in the same order, then
            // overlay the now-known-compatible ν cells.
            let mut row = smallvec::smallvec![None; out_len];
            row[..left_len].copy_from_slice(mu);
            for (j, cell) in nu.iter().enumerate() {
                if let Some(term) = cell {
                    row[right_to_out[j]] = Some(*term);
                }
            }
            rows.push(row);
        }
    }
    Ok(lift.finish(SolutionSeq { schema: out, rows }))
}

/// Evaluate `left UNION right` as a multiset concatenation over the union schema.
///
/// Gated on [`crate::parallel::is_parallel_safe_pattern`] over **both** branches:
/// if either reaches an unsafe (counter/RNG-mutating) builtin, the sequential
/// body below runs, evaluating both branches directly against the real `ctx`
/// exactly as before this task. Otherwise both branches mint new `Computed`
/// terms that must escape into the union's output rows, so they are evaluated
/// concurrently (`rayon::join`) against their own forked child context, and each
/// branch's escaping rows are captured via [`crate::parallel::portable_row`]
/// **while its child is still alive** (the child is dropped the instant its
/// closure returns). Only once both branches are done does the MAIN thread
/// re-intern them back into `ctx.scratch`, left branch first then right, via
/// [`crate::parallel::reintern_portable_row`] — reproducing the sequential
/// concat's exact row order (left rows, then right rows) and column layout.
///
/// # Under a truncated child: ONE rule, both paths
///
/// `UNION` is a concatenation, and the rule is stated so that the result depends on the
/// query, the data, and the budget — and on nothing else:
///
/// > A truncated `UNION` yields the rows of the branches that COMPLETED, in branch order.
/// > If the LEFT branch truncates, the union carries only the left branch's rows and the
/// > right branch contributes nothing **even if it was already computed**. If the left
/// > completes and the right truncates, the union carries the left rows followed by the
/// > right branch's partial rows.
///
/// The clause about already-computed rows is the whole point. `rayon::join` starts both
/// branches, so on the parallel path the right branch's rows may well exist by the time
/// the left branch's truncation is known — and admitting them would make a governed
/// result larger *because a second thread got there*. Same query, same data, same budget
/// would then give different partial answers on different machines and under different
/// scheduling, which is precisely the property the order-stable reduction exists to
/// guarantee. Being "more informative" is not a licence a governor has; the result is
/// truncated to the branch-ordered prefix on both paths and the extra rows are dropped.
///
/// The soundness half is the `MONOTONE_BAG` / `MONOTONE` split the one visitor already
/// records for this node, which the lift reads rather than restating: truncating the LEFT
/// branch removes rows from the middle of the concatenation (a sub-bag, not a prefix),
/// while truncating the RIGHT branch removes them from the end (a genuine prefix).
pub(crate) fn eval_union<D: DatasetView + Sync>(
    node: &GraphPattern,
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    if crate::parallel::sequential_operation_required()
        // A governed `UNION` evaluates its arms in source order on one thread: both arms
        // charge the same shared `GovernorState`, so forking them makes the budget — and
        // with it the trip point and the certified rows — a lottery. See
        // `EvalCtx::may_fork_sibling_patterns`.
        || !ctx.may_fork_sibling_patterns()
        || !crate::parallel::is_parallel_safe_pattern(left, ctx.safety_registries())
        || !crate::parallel::is_parallel_safe_pattern(right, ctx.safety_registries())
    {
        let Some(l) = lift.absorb(0, eval_evaluated(left, ctx)?) else {
            return Ok(lift.withheld());
        };
        if lift.is_truncated() {
            // The right arm is never started: the rows in hand ARE the union's output
            // so far, and evaluating a fresh subtree after the budget is spent is the
            // unbounded work the lift's own budget rule forbids.
            return Ok(lift.finish(l));
        }
        let Some(r) = lift.absorb(1, eval_evaluated(right, ctx)?) else {
            return Ok(lift.withheld());
        };
        return Ok(lift.finish(concat_union(&l, &r, ctx)));
    }

    let base = ctx.scratch.computed_count();
    // A shared (immutable) borrow of `ctx`: both closures below only need
    // `fork_for_worker`'s `&self` access, so they run concurrently over the same
    // `&EvalCtx` — `EvalCtx: Sync` (see its definition) makes this sound.
    let ctx_ref: &EvalCtx<'_, D> = ctx;

    // Each closure forks its own child, evaluates its branch on it, and
    // classifies every result row (see [`crate::parallel::minted_row`]): a row
    // the child minted nothing new into (the common case for a UNION-over-BGP
    // branch) is kept as-is with zero extra allocation, and only a row
    // carrying a genuinely fresh cell pays for the portable-materialize round
    // trip. The child (and its scratch) does not survive past this closure.
    let eval_branch = |pattern: &GraphPattern| -> Result<UnionBranch<crate::parallel::MintedRow<D::Id>>, EvalError> {
        let mut child = ctx_ref.fork_for_worker();
        let evaluated = eval_evaluated(pattern, &mut child)?;
        let truncated = evaluated.is_truncated();
        // The branch's rows are materialized against the child's scratch either way; a
        // truncated branch's certificate rides back separately because the portable-row
        // round trip below has to happen while the child is still alive.
        let (schema, rows, certificate) = match evaluated {
            Evaluated::Complete(seq) => (seq.schema, seq.rows, None),
            Evaluated::Truncated(truncation) => {
                let (seq, certificate) = truncation.split();
                (seq.schema, seq.rows, Some(certificate))
            }
        };
        debug_assert_eq!(truncated, certificate.is_some());
        let minted: Vec<_> = rows
            .into_iter()
            .map(|row| crate::parallel::minted_row(&child.scratch, base, row))
            .collect();
        Ok(UnionBranch {
            schema,
            rows: minted,
            certificate,
        })
    };

    let (left_result, right_result) = rayon::join(|| eval_branch(left), || eval_branch(right));
    // The branch-order rule, applied before a single row is concatenated: see
    // `union_branch_order` for why a computed-but-discarded right branch is the point.
    let (left_branch, right_branch, governing) = union_branch_order(left_result?, right_result?);
    let UnionBranch {
        schema: l_schema,
        rows: l_minted,
        certificate: _,
    } = left_branch;
    let UnionBranch {
        schema: r_schema,
        rows: r_minted,
        certificate: _,
    } = right_branch;

    let out = l_schema.union(&r_schema);
    let out_len = out.len();
    let left_len = l_schema.len();
    let right_to_out = right_to_out_map(&r_schema, &out);

    let mut rows = Vec::with_capacity(l_minted.len() + r_minted.len());
    for minted in l_minted {
        let mut row = smallvec::smallvec![None; out_len];
        let reinterned =
            crate::parallel::reintern_minted_row(&mut ctx.scratch, ctx.dataset, minted);
        row[..left_len].copy_from_slice(&reinterned);
        rows.push(row);
    }
    for minted in r_minted {
        let reinterned =
            crate::parallel::reintern_minted_row(&mut ctx.scratch, ctx.dataset, minted);
        let mut row = smallvec::smallvec![None; out_len];
        for (j, &cell) in reinterned.iter().enumerate() {
            row[right_to_out[j]] = cell;
        }
        rows.push(row);
    }

    let united = SolutionSeq {
        schema: Arc::new(out),
        rows,
    };
    let Some((ordinal, certificate)) = governing else {
        return Ok(lift.finish(united));
    };
    if lift.absorb_certificate(ordinal, certificate) {
        return Ok(lift.finish(united));
    }
    Ok(lift.finish(SolutionSeq::empty(united.schema)))
}

/// The sequential `UNION` body: concatenate `l` then `r` over the ordered
/// union schema (left columns first). Shared by both the sequential fallback
/// and (conceptually) documents the exact row shape the parallel path in
/// [`eval_union`] must reproduce.
fn concat_union<D: DatasetView + Sync>(
    l: &SolutionSeq<D::Id>,
    r: &SolutionSeq<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> SolutionSeq<D::Id> {
    let out = l.schema.union(&r.schema);
    let out_len = out.len();
    let left_len = l.schema.len();
    let right_to_out = right_to_out_map(&r.schema, &out);

    let expected = l.rows.len().saturating_add(r.rows.len());
    let cell_ceiling = ctx.cell_row_ceiling(out_len);
    let mut rows = Vec::with_capacity(cell_ceiling.map_or(expected, |cap| cap.min(expected)));
    for lrow in &l.rows {
        if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
            let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
            break;
        }
        // Left columns are out[0..left_len] in order; pad the rest with None.
        let mut row = smallvec::smallvec![None; out_len];
        row[..left_len].copy_from_slice(lrow);
        rows.push(row);
    }
    for rrow in &r.rows {
        if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
            let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
            break;
        }
        let mut row = smallvec::smallvec![None; out_len];
        for (j, &cell) in rrow.iter().enumerate() {
            row[right_to_out[j]] = cell;
        }
        rows.push(row);
    }

    SolutionSeq {
        schema: Arc::new(out),
        rows,
    }
}

/// One `UNION` branch's forked-child result: its output schema plus every result row
/// classified via [`crate::parallel::minted_row`] — `Direct` (no remap needed) or
/// `Portable` (materialized while the branch's forked child is still alive) — so it
/// survives that child being dropped, together with the certificate when the branch
/// truncated.
///
/// Generic over the row payload so the branch-order rule ([`union_branch_order`]) can be
/// exercised without building a forked worker's interned rows: the rule is a function of
/// the certificates alone, and it must be checkable for the branch combination only true
/// concurrency can produce.
#[derive(Debug)]
struct UnionBranch<T> {
    /// The branch's output schema.
    schema: Arc<VarSchema>,
    /// The branch's rows.
    rows: Vec<T>,
    /// The certificate, when a governor stopped the arm short.
    certificate: Option<crate::governor::lift::Certificate>,
}

/// The `UNION` branch-order truncation rule, applied.
///
/// > A truncated `UNION` yields the rows of the branches that COMPLETED, in branch order.
/// > If the LEFT branch truncates, only its rows survive and the right branch contributes
/// > nothing **even if it was already computed**; if the left completes and the right
/// > truncates, both contribute and the right's partial rows are a genuine suffix.
///
/// The "even if it was already computed" clause is the reason this is a function rather
/// than an `if` inside the sequential body. The sequential body never starts the right
/// branch after the left truncates, but `rayon::join` starts both, so on the parallel
/// path the right branch's rows can exist by the time the left branch's trip is known.
/// Admitting them would make a governed result larger *because a second thread got
/// there* — the same query, data, and budget would then produce different partial answers
/// under different scheduling. Emptying the right branch here is what makes the two paths
/// one observable rule.
///
/// Returns the surviving branches plus the ordinal of the branch whose certificate
/// governs (left before right, so a simultaneous trip reports the branch a sequential
/// evaluation would have reached first — the same source-order reduction
/// [`crate::parallel`] applies to errors).
fn union_branch_order<T>(
    left: UnionBranch<T>,
    mut right: UnionBranch<T>,
) -> (
    UnionBranch<T>,
    UnionBranch<T>,
    Option<(usize, ChildCertificate)>,
) {
    if let Some(certificate) = left.certificate.clone() {
        // Discarded, not concatenated. An empty schema unions to the left schema, so the
        // concatenation downstream reproduces the sequential body's output exactly.
        right.rows = Vec::new();
        right.schema = Arc::new(VarSchema::new());
        right.certificate = None;
        return (left, right, Some((0, certificate)));
    }
    let governing = right
        .certificate
        .clone()
        .map(|certificate| (1, certificate));
    (left, right, governing)
}

/// The certificate half of a [`UnionBranch`], named for readability at the boundary.
type ChildCertificate = crate::governor::lift::Certificate;

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
pub(crate) fn build_index<I: ViewTermId>(
    r: &SolutionSeq<I>,
    shared: &[(usize, usize)],
) -> (DetHashMap<JoinKey<I>, Vec<usize>>, Vec<usize>) {
    // Pre-size to the build-row count: the exact upper bound on distinct keys, so a
    // large build side is filled without incremental rehash-and-reallocate churn.
    let mut keyed: DetHashMap<JoinKey<I>, Vec<usize>> =
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
pub(crate) fn probe_has_match<I: ViewTermId>(
    probe: &[Option<SolutionTerm<I>>],
    shared: &[(usize, usize)],
    keyed: &DetHashMap<JoinKey<I>, Vec<usize>>,
    wild: &[usize],
    r_rows: &[Solution<I>],
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
fn hash_join<D: DatasetView + Sync>(
    l: &SolutionSeq<D::Id>,
    r: &SolutionSeq<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> SolutionSeq<D::Id> {
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
    let rows = if let Some(cell_ceiling) = ctx.cell_row_ceiling(out_len) {
        // A global allocation bound cannot be divided among parallel chunks without each
        // chunk receiving (and allocating) the whole allowance. Keep this governed lane in
        // source order and test the ceiling before `merge` constructs the next row.
        let worst_case = l.rows.len().saturating_mul(r.rows.len());
        let mut rows = Vec::with_capacity(cell_ceiling.min(worst_case));
        'left: for lrow in &l.rows {
            match bound_key(lrow, &shared, KeySide::Left) {
                Some(key) => {
                    if let Some(idxs) = keyed.get(&key) {
                        for &idx in idxs {
                            if rows.len() >= cell_ceiling {
                                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                                break 'left;
                            }
                            rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                    for &idx in &wild {
                        if compatible(lrow, &r.rows[idx], &shared) {
                            if rows.len() >= cell_ceiling {
                                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                                break 'left;
                            }
                            rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                }
                None => {
                    for rrow in &r.rows {
                        if compatible(lrow, rrow, &shared) {
                            if rows.len() >= cell_ceiling {
                                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                                break 'left;
                            }
                            rows.push(merge(lrow, rrow, left_len, &right_to_out, out_len));
                        }
                    }
                }
            }
        }
        rows
    } else {
        crate::parallel::par_chunk_map(&l.rows, |acc, lrow| {
            match bound_key(lrow, &shared, KeySide::Left) {
                // Probe is fully bound on shared columns: hit the matching bucket
                // (exact key ⇒ compatible) plus any wild build rows it is compatible
                // with (a wild row's None shared column matches anything).
                Some(key) => {
                    if let Some(idxs) = keyed.get(&key) {
                        for &idx in idxs {
                            acc.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                    for &idx in &wild {
                        if compatible(lrow, &r.rows[idx], &shared) {
                            acc.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                }
                // Probe has an unbound shared column: it can match any build row, so
                // fall back to a compatibility scan over all of them.
                None => {
                    for rrow in &r.rows {
                        if compatible(lrow, rrow, &shared) {
                            acc.push(merge(lrow, rrow, left_len, &right_to_out, out_len));
                        }
                    }
                }
            }
        })
    };

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
fn bound_key<I: ViewTermId>(
    row: &[Option<SolutionTerm<I>>],
    shared: &[(usize, usize)],
    side: KeySide,
) -> Option<JoinKey<I>> {
    let col_of = |ia: usize, ib: usize| match side {
        KeySide::Left => ia,
        KeySide::Right => ib,
    };
    if let [(ia, ib)] = *shared {
        // Single shared column: no heap allocation for the key.
        return Some(JoinKey::Single(row[col_of(ia, ib)]?.join_key()));
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
fn merge<I: ViewTermId>(
    left_row: &Solution<I>,
    right_row: &Solution<I>,
    left_len: usize,
    right_to_out: &[usize],
    out_len: usize,
) -> Solution<I> {
    #[cfg(test)]
    MERGE_COUNT.with(|count| {
        if let Some(current) = count.get() {
            count.set(Some(current.saturating_add(1)));
        }
    });
    debug_assert_eq!(left_row.len(), left_len);
    // One exact-size allocation, initialized from the left row directly (no
    // write-None-then-overwrite pass over the left prefix).
    let mut merged = Solution::with_capacity(out_len);
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

#[cfg(test)]
std::thread_local! {
    /// Per-test proof that a cell-bounded cross product checks the sink before constructing
    /// row `limit + 1`. `None` keeps ordinary tests and production builds free.
    static MERGE_COUNT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
fn counting_merges<T>(run: impl FnOnce() -> T) -> (T, usize) {
    let previous = MERGE_COUNT.with(|count| count.replace(Some(0)));
    let output = run();
    let observed = MERGE_COUNT.with(|count| count.replace(previous).unwrap_or(0));
    (output, observed)
}

/// Evaluate `left OPTIONAL { right }` (algebra `LeftJoin`) as a left outer join,
/// with an optional inline `FILTER` condition evaluated on the merged solution.
///
/// # Under a truncated child: the fabrication hazard
///
/// A left outer join emits a left row **padded with unbound** exactly when no compatible
/// right row exists — a claim about the *whole* right bag. So:
///
/// - **Truncated left arm.** The right arm is not evaluated at all, and the left rows in
///   hand are NOT emitted padded. Padding them would assert "the OPTIONAL matched
///   nothing" for rows whose right bag was never looked at: a fabricated answer, not a
///   missing one. The node yields the empty bag, a sound lower bound.
/// - **Truncated right arm.** Padding is **suppressed** and the operator degenerates to
///   an inner join over the partial right bag. This is the same fabrication hazard, one
///   step down: a left row whose only match lies past the cut would come out padded, and
///   that padded row asserts "the OPTIONAL matched nothing" about a right bag the
///   evaluator only holds a prefix of.
///
///   Padding it is not merely imprecise, it is **unsound in both directions**. It is not
///   a lower bound — the padded row is not an answer. And it is not an upper bound
///   either, which is the subtler half: emitting `l` padded *in place of* the true rows
///   `l ⋈ m` for every `m` past the cut means those true answers are absent from the
///   result, so the one licence an upper bound grants — "a row absent from this result is
///   definitively not an answer" — is false of exactly the rows the cut hid.
///
///   Suppressing the padding restores a bound: what is emitted is `{l ⋈ m : m ∈ R'}` for
///   the partial right bag `R' ⊆ R`, and every such row is in the true output, so the
///   result is a certified sub-bag. That is why `LeftJoin`'s right edge is classified
///   [`ChildEdge::MONOTONE_BAG`] — identical to `Join`'s right edge, which is exactly what
///   this operator becomes once its right bag is known to be incomplete.
///
/// [`ChildEdge::MONOTONE_BAG`]: crate::governor::soundness::ChildEdge::MONOTONE_BAG
pub(crate) fn eval_left_join<D: DatasetView + Sync>(
    node: &GraphPattern,
    left: &GraphPattern,
    right: &GraphPattern,
    expression: Option<&Expression>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    left_join_lift(
        node,
        eval_evaluated(left, ctx)?,
        |ctx| eval_evaluated(right, ctx),
        expression,
        ctx,
    )
}

/// The `LeftJoin` lift, given the left arm's result and a **lazy** right arm.
///
/// The right arm arrives as a closure rather than as a value for a reason that is part of
/// the contract: when the left arm has already truncated, the right arm is *never
/// evaluated*. Handing this function a value would make that impossible to express, and
/// would license a full scan of the right arm after the budget was spent.
///
/// Split out from [`eval_left_join`] so the fabrication hazard can be tested by feeding
/// the arms directly, rather than only through a whole query whose trip point depends on
/// a charge schedule.
fn left_join_lift<D: DatasetView + Sync>(
    node: &GraphPattern,
    left: Evaluated<D::Id>,
    right: impl FnOnce(&mut EvalCtx<'_, D>) -> Result<Evaluated<D::Id>, EvalError>,
    expression: Option<&Expression>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(l) = lift.absorb(0, left) else {
        return Ok(lift.withheld());
    };
    if lift.is_truncated() {
        // The right arm is NOT evaluated, and the left rows in hand are NOT emitted
        // padded with unbound: padding asserts "the OPTIONAL matched nothing", which is a
        // claim about a right bag that was never looked at. The empty bag is a sound
        // lower bound; a padded row would be a fabricated answer.
        return Ok(lift.finish(SolutionSeq::empty(l.schema)));
    }
    let Some(r) = lift.absorb(1, right(ctx)?) else {
        return Ok(lift.withheld());
    };
    // The left arm did not truncate (that returned above), so the lift is truncated here
    // exactly when the RIGHT arm did — i.e. exactly when `r` is a prefix of the right bag
    // rather than the whole of it, and "no compatible right row exists" is therefore not
    // a fact this operator holds about any left row. See this operator's doc comment for
    // why padding then bounds the answer on neither side.
    let pad_unmatched = !lift.is_truncated();
    let joined = match expression {
        None => left_outer_join(&l, &r, ctx, pad_unmatched),
        Some(expr) => left_outer_join_filtered(&l, &r, expr, ctx, pad_unmatched)?,
    };
    // An `EXISTS` inside the inline condition is an opaque edge: a trip inside it makes
    // every emitted row's condition a boolean computed over a truncated bag, so the whole
    // output is withheld rather than certified.
    if let Some(tripped) = ctx.expression_barrier.observed() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            node,
            tripped,
            Arc::clone(&joined.schema),
        )));
    }
    Ok(lift.finish(joined))
}

/// A left outer join whose right-side pairings must additionally satisfy `expr`
/// (the inline `OPTIONAL { ... FILTER expr }` condition, §18.6). A left solution
/// with no pairing that is both compatible and passes the filter is emitted alone.
///
/// Gated on [`crate::parallel::is_parallel_safe`] exactly like `eval_filter`: an
/// unsafe `expr` (reaches `RAND`/`UUID`/`STRUUID`/`BNODE`/the PurRDF list
/// constructors) MUST run on the real `ctx` sequentially, since a forked child
/// would advance a throwaway copy of the per-query counter/RNG state instead of
/// the real one. A safe `expr` only decides which merged rows pass; every
/// surviving cell is copied from `lrow`/`rrow` by [`merge`] (which interns
/// nothing — see its doc comment), so nothing new escapes a forked child's
/// scratch and it is discarded after use — no re-interning via
/// [`crate::parallel::reintern_minted_row`] is needed.
///
/// `pad_unmatched` is `false` exactly when `r` is a *prefix* of the right bag rather than
/// the whole of it, in which case "no pairing passes" is not a fact this operator holds
/// and the left-alone row must not be emitted; see [`eval_left_join`].
fn left_outer_join_filtered<D: DatasetView + Sync>(
    l: &SolutionSeq<D::Id>,
    r: &SolutionSeq<D::Id>,
    expr: &Expression,
    ctx: &mut EvalCtx<'_, D>,
    pad_unmatched: bool,
) -> Result<SolutionSeq<D::Id>, EvalError> {
    let out = Arc::new(l.schema.union(&r.schema));
    let out_len = out.len();
    let left_len = l.schema.len();
    let right_to_out = right_to_out_map(&r.schema, &out);
    let shared = l.schema.shared_columns(&r.schema);

    // A left outer join emits at least one row per left row.
    let cell_ceiling = ctx.cell_row_ceiling(out_len);
    let rows = if cell_ceiling.is_none() && ctx.may_fork_row_loop(expr) {
        crate::parallel::par_chunk_try_map_init(
            &l.rows,
            || ctx.fork_for_worker(),
            |child, acc, lrow| {
                let before = acc.len();
                for rrow in &r.rows {
                    if !compatible(lrow, rrow, &shared) {
                        continue;
                    }
                    let merged = merge(lrow, rrow, left_len, &right_to_out, out_len);
                    if crate::expr::eval_ebv(expr, &merged, &out, child)? == Some(true) {
                        acc.push(merged);
                    }
                }
                if pad_unmatched && acc.len() == before {
                    let mut row = smallvec::smallvec![None; out_len];
                    row[..left_len].copy_from_slice(lrow);
                    acc.push(row);
                }
                Ok(())
            },
        )?
    } else {
        let mut rows =
            Vec::with_capacity(cell_ceiling.map_or(l.rows.len(), |cap| cap.min(l.rows.len())));
        'left: for lrow in &l.rows {
            let mut matched = false;
            for rrow in &r.rows {
                if !compatible(lrow, rrow, &shared) {
                    continue;
                }
                let merged = merge(lrow, rrow, left_len, &right_to_out, out_len);
                if crate::expr::eval_ebv(expr, &merged, &out, ctx)? == Some(true) {
                    if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
                        let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                        break 'left;
                    }
                    rows.push(merged);
                    matched = true;
                }
            }
            if pad_unmatched && !matched {
                if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
                    let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                    break;
                }
                let mut row = smallvec::smallvec![None; out_len];
                row[..left_len].copy_from_slice(lrow);
                rows.push(row);
            }
        }
        rows
    };
    Ok(SolutionSeq { schema: out, rows })
}

/// Left outer join: every left solution merged with each compatible right
/// solution, or emitted alone (right columns unbound) when none is compatible.
///
/// `pad_unmatched` is `false` exactly when `r` is a *prefix* of the right bag rather than
/// the whole of it, in which case "no compatible right row exists" is not a fact this
/// operator holds and the left-alone row must not be emitted; see [`eval_left_join`]. The
/// operator then computes an inner join, which is what a `LeftJoin` over an incomplete
/// right bag soundly is.
fn left_outer_join<D: DatasetView + Sync>(
    l: &SolutionSeq<D::Id>,
    r: &SolutionSeq<D::Id>,
    ctx: &EvalCtx<'_, D>,
    pad_unmatched: bool,
) -> SolutionSeq<D::Id> {
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
    let cell_ceiling = ctx.cell_row_ceiling(out_len);
    let rows = if let Some(cell_ceiling) = cell_ceiling {
        let worst_case = l.rows.len().saturating_mul(r.rows.len().max(1));
        let mut rows = Vec::with_capacity(cell_ceiling.min(worst_case));
        'left: for lrow in &l.rows {
            let before = rows.len();
            match bound_key(lrow, &shared, KeySide::Left) {
                Some(key) => {
                    if let Some(idxs) = keyed.get(&key) {
                        for &idx in idxs {
                            if rows.len() >= cell_ceiling {
                                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                                break 'left;
                            }
                            rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                    for &idx in &wild {
                        if compatible(lrow, &r.rows[idx], &shared) {
                            if rows.len() >= cell_ceiling {
                                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                                break 'left;
                            }
                            rows.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                }
                None => {
                    for rrow in &r.rows {
                        if compatible(lrow, rrow, &shared) {
                            if rows.len() >= cell_ceiling {
                                let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                                break 'left;
                            }
                            rows.push(merge(lrow, rrow, left_len, &right_to_out, out_len));
                        }
                    }
                }
            }
            if pad_unmatched && rows.len() == before {
                if rows.len() >= cell_ceiling {
                    let _ = ctx.observe_cells(rows.len().saturating_add(1), out_len);
                    break;
                }
                let mut row = smallvec::smallvec![None; out_len];
                row[..left_len].copy_from_slice(lrow);
                rows.push(row);
            }
        }
        rows
    } else {
        crate::parallel::par_chunk_map(&l.rows, |acc, lrow| {
            let before = acc.len();
            match bound_key(lrow, &shared, KeySide::Left) {
                Some(key) => {
                    if let Some(idxs) = keyed.get(&key) {
                        for &idx in idxs {
                            acc.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                    for &idx in &wild {
                        if compatible(lrow, &r.rows[idx], &shared) {
                            acc.push(merge(lrow, &r.rows[idx], left_len, &right_to_out, out_len));
                        }
                    }
                }
                None => {
                    for rrow in &r.rows {
                        if compatible(lrow, rrow, &shared) {
                            acc.push(merge(lrow, rrow, left_len, &right_to_out, out_len));
                        }
                    }
                }
            }
            // No compatible right solution → keep the left solution alone (the OPTIONAL
            // contributed nothing, its variables stay unbound).
            if pad_unmatched && acc.len() == before {
                let mut row = smallvec::smallvec![None; out_len];
                row[..left_len].copy_from_slice(lrow);
                acc.push(row);
            }
        })
    };

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
///
/// # Under a truncated child
///
/// Removal is a claim about the *whole* right bag, so a truncated LEFT arm yields the
/// empty bag rather than left rows that were never checked for removal — emitting them
/// would fabricate rows `MINUS` would have deleted. A truncated RIGHT arm subtracts less
/// than the true query would, so the output contains the true answer: an upper bound, not
/// a black hole.
pub(crate) fn eval_minus<D: DatasetView + Sync>(
    node: &GraphPattern,
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(l) = lift.absorb(0, eval_evaluated(left, ctx)?) else {
        return Ok(lift.withheld());
    };
    if lift.is_truncated() {
        return Ok(lift.finish(SolutionSeq::empty(l.schema)));
    }
    let Some(r) = lift.absorb(1, eval_evaluated(right, ctx)?) else {
        return Ok(lift.withheld());
    };
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

    Ok(lift.finish(SolutionSeq {
        schema: l.schema,
        rows,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EvalCtx;
    use crate::eval::eval;
    use crate::governor::{
        GovernorState as TestGovernorState, QueryGovernors as TestQueryGovernors,
    };

    // The operators take the algebra node itself (it names the barrier and supplies the
    // child edge classification), so these tests build the node and drive the ordinary
    // dispatch — which is also what keeps them testing the wiring rather than a private
    // entry point no query ever reaches.
    fn eval_join<D: DatasetView<Id = TermId> + Sync>(
        left: &GraphPattern,
        right: &GraphPattern,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::Join {
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },
            ctx,
        )
    }

    fn eval_union<D: DatasetView<Id = TermId> + Sync>(
        left: &GraphPattern,
        right: &GraphPattern,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::Union {
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },
            ctx,
        )
    }

    fn eval_left_join<D: DatasetView<Id = TermId> + Sync>(
        left: &GraphPattern,
        right: &GraphPattern,
        expression: Option<&Expression>,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::LeftJoin {
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
                expression: expression.cloned(),
            },
            ctx,
        )
    }

    fn eval_minus<D: DatasetView<Id = TermId> + Sync>(
        left: &GraphPattern,
        right: &GraphPattern,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<SolutionSeq, EvalError> {
        eval(
            &GraphPattern::Minus {
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
            },
            ctx,
        )
    }
    use pretty_assertions::assert_eq;
    use purrdf_core::{
        RdfDataset, RdfDatasetBuilder, ResourceDimension as TestResourceDimension, TermValue,
        TrippedGovernor as TestTrippedGovernor,
    };
    use purrdf_sparql_algebra::{
        Literal, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
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
                            TermValue::Iri(s) => s,
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
    fn cell_bounded_cross_product_refuses_before_constructing_limit_plus_one() {
        let ds = graph();
        let left_id = ds
            .term_id_by_value(&TermValue::Iri("http://ex/a".to_owned()))
            .expect("left term");
        let right_id = ds
            .term_id_by_value(&TermValue::Iri("http://ex/b".to_owned()))
            .expect("right term");
        let left = SolutionSeq {
            schema: Arc::new(VarSchema::from_vars([Variable::new("left")])),
            rows: (0..100)
                .map(|_| smallvec::smallvec![Some(SolutionTerm::Existing(left_id))])
                .collect(),
        };
        let right = SolutionSeq {
            schema: Arc::new(VarSchema::from_vars([Variable::new("right")])),
            rows: (0..100)
                .map(|_| smallvec::smallvec![Some(SolutionTerm::Existing(right_id))])
                .collect(),
        };
        // Two columns, four cells: exactly two rows fit. The 10,000-row unbounded cross
        // product must construct only those two; candidate three records six attempted
        // cells without calling `merge`.
        let state = Arc::new(TestGovernorState::new(
            &TestQueryGovernors::UNBOUNDED.with_max_intermediate_cells(4),
        ));
        let ctx = EvalCtx::new(&ds).with_governors(Arc::clone(&state));
        let (joined, merges) = counting_merges(|| hash_join(&left, &right, &ctx));

        assert_eq!(joined.rows.len(), 2);
        assert_eq!(
            merges, 2,
            "row three must be refused before merge allocates it"
        );
        assert_eq!(
            state.tripped(),
            Some(TestTrippedGovernor::Budget {
                dimension: TestResourceDimension::IntermediateCells,
                limit: 4,
                consumed: 6,
            })
        );
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
            rows: vec![
                smallvec::smallvec![t(1)],
                smallvec::smallvec![t(2)],
                smallvec::smallvec![None],
            ],
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
            rows: vec![smallvec::smallvec![t(1)], smallvec::smallvec![t(2)]],
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

    // ── UNION branch that is only a FILTER (no BGP) ────────────────────────────
    //
    // A branch `{ FILTER(expr) }` is `Filter { expr, inner: <empty BGP> }` over the
    // unit solution (one row, empty schema). Per SPARQL 1.1:
    //   * `{ FILTER(true) }` keeps that unit solution — one empty binding — which,
    //     after the enclosing join, joins with EVERY outer row (its unbound shared
    //     column matches anything).
    //   * `{ FILTER(?a > 0) }` sees ?a UNBOUND inside that group (it has no BGP that
    //     binds ?a), so `?a > 0` is a type error ⇒ EBV false ⇒ the branch yields ZERO
    //     rows. The whole query must NOT collapse to empty — only the OTHER UNION
    //     branch contributes.
    // These two shapes have DIFFERENT correct results; the tests encode the split.

    const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const XBOOL: &str = "http://www.w3.org/2001/XMLSchema#boolean";

    /// `ex:x :v 5`, `ex:y :v 7`, `ex:x :flag true` — only x carries the flag.
    fn union_filter_branch_ds() -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        let int = |lex: &str| RdfLiteral {
            lexical_form: lex.to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        };
        let mut b = RdfDatasetBuilder::new();
        let v = b.intern_iri("http://example.org/v");
        let flag = b.intern_iri("http://example.org/flag");
        let x = b.intern_iri("http://example.org/x");
        let y = b.intern_iri("http://example.org/y");
        let i5 = b.intern_literal(int("5"));
        let i7 = b.intern_literal(int("7"));
        let tru = b.intern_literal(RdfLiteral {
            lexical_form: "true".to_owned(),
            datatype: Some(XBOOL.to_owned()),
            language: None,
            direction: None,
        });
        b.push_quad(x, v, i5, None);
        b.push_quad(y, v, i7, None);
        b.push_quad(x, flag, tru, None);
        b.freeze().expect("freeze")
    }

    /// `{ ?s :v ?a . { FILTER(cond) } UNION { ?s :flag true } }` — the left UNION
    /// branch is a lone FILTER over the empty BGP (the unit solution).
    fn union_filter_branch_pattern(cond: Expression) -> GraphPattern {
        let scan = bgp(vp("s"), pred("http://example.org/v"), vp("a"));
        let filter_branch = GraphPattern::Filter {
            expr: cond,
            inner: Box::new(GraphPattern::Bgp { patterns: vec![] }),
        };
        let flag_branch = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://example.org/flag"),
                object: TermPattern::Literal(Literal::new_typed(
                    "true",
                    NamedNode::new_unchecked(XBOOL),
                )),
            }],
        };
        GraphPattern::Join {
            left: Box::new(scan),
            right: Box::new(GraphPattern::Union {
                left: Box::new(filter_branch),
                right: Box::new(flag_branch),
            }),
        }
    }

    /// Render `(?s, ?a)` rows as `(iri, lexical)` string pairs, sorted for a
    /// multiset comparison.
    fn s_a_rows(ds: &RdfDataset, seq: &SolutionSeq) -> Vec<(String, String)> {
        let scratch = crate::scratch::ScratchInterner::new();
        let s_col = seq.schema.index_of(&Variable::new("s")).expect("s");
        let a_col = seq.schema.index_of(&Variable::new("a")).expect("a");
        let render_cell = |t: SolutionTerm| match scratch.value_of(ds, t) {
            TermValue::Iri(s) => s,
            TermValue::Literal { lexical_form, .. } => lexical_form,
            other => format!("{other:?}"),
        };
        let mut out: Vec<(String, String)> = seq
            .rows
            .iter()
            .map(|row| {
                (
                    render_cell(row[s_col].expect("s bound")),
                    render_cell(row[a_col].expect("a bound")),
                )
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn union_filter_true_branch_contributes_unit_solution() {
        // { FILTER(true) } keeps the unit solution, which joins with EVERY ?s :v ?a
        // row; the { ?s :flag true } branch additionally re-contributes x. So x
        // appears twice (once per branch) and y once — three rows total.
        let ds = union_filter_branch_ds();
        let mut ctx = EvalCtx::new(&ds);
        let pattern = union_filter_branch_pattern(Expression::Literal(Literal::new_typed(
            "true",
            NamedNode::new_unchecked(XBOOL),
        )));
        let seq = eval(&pattern, &mut ctx).expect("eval");
        assert_eq!(
            s_a_rows(&ds, &seq),
            vec![
                ("http://example.org/x".to_owned(), "5".to_owned()),
                ("http://example.org/x".to_owned(), "5".to_owned()),
                ("http://example.org/y".to_owned(), "7".to_owned()),
            ],
            "FILTER(true) branch contributes the unit solution joined with every row"
        );
    }

    #[test]
    fn union_filter_unbound_branch_yields_zero_but_query_not_empty() {
        // { FILTER(?a > 0) } sees ?a UNBOUND inside the group ⇒ type error ⇒ EBV
        // false ⇒ zero rows from the left branch. ONLY the { ?s :flag true } branch
        // contributes: exactly x. The query must NOT be empty (the 0.2.0 defect).
        let ds = union_filter_branch_ds();
        let mut ctx = EvalCtx::new(&ds);
        let pattern = union_filter_branch_pattern(Expression::Greater(
            Box::new(Expression::Variable(Variable::new("a"))),
            Box::new(Expression::Literal(Literal::new_typed(
                "0",
                NamedNode::new_unchecked(XINT),
            ))),
        ));
        let seq = eval(&pattern, &mut ctx).expect("eval");
        assert_eq!(
            s_a_rows(&ds, &seq),
            vec![("http://example.org/x".to_owned(), "5".to_owned())],
            "only the ?s :flag true branch contributes; the query is not empty"
        );
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

    /// Determinism smoke test: a 3-branch `UNION` (the `d_union_4` bench
    /// shape) where each branch `BIND`s a freshly-computed value — two branches
    /// minting the SAME value (`11`), one minting a DISJOINT value (`12`) — so the
    /// escaping-computed-term merge in [`eval_union`] is exercised both ways.
    /// Evaluated once with the parallel `UNION` path FORCED and once with the
    /// sequential path FORCED, the two must produce byte-identical rows (schema
    /// and row order, left-branch rows then right-branch rows, nested left-to-right).
    #[test]
    fn union_three_branches_with_overlapping_and_disjoint_binds_agree() {
        use purrdf_sparql_algebra::Variable;

        let mut b = RdfDatasetBuilder::new();
        let p1 = b.intern_iri("http://ex/p1");
        let p2 = b.intern_iri("http://ex/p2");
        let p3 = b.intern_iri("http://ex/p3");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";
        let ten = b.intern_literal(purrdf_core::RdfLiteral {
            lexical_form: "10".to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        });
        let twenty = b.intern_literal(purrdf_core::RdfLiteral {
            lexical_form: "20".to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        });
        b.push_quad(a, p1, ten, None);
        b.push_quad(bb, p2, twenty, None);
        b.push_quad(c, p3, ten, None);
        let ds = b.freeze().expect("freeze");

        let one = || Expression::Literal(Literal::new_typed("1", NamedNode::new_unchecked(XINT)));
        let nine = || Expression::Literal(Literal::new_typed("9", NamedNode::new_unchecked(XINT)));
        let two = || Expression::Literal(Literal::new_typed("2", NamedNode::new_unchecked(XINT)));

        // branch1: {?s :p1 ?o} BIND(?o + 1 AS ?sum)  -> s=a, o=10, sum=11
        let branch1 = GraphPattern::Extend {
            inner: Box::new(bgp(vp("s"), pred("http://ex/p1"), vp("o"))),
            variable: Variable::new("sum"),
            expression: Expression::Add(
                Box::new(Expression::Variable(Variable::new("o"))),
                Box::new(one()),
            ),
        };
        // branch2: {?s :p2 ?o} BIND(?o - 9 AS ?sum)  -> s=b, o=20, sum=11 (SAME as branch1)
        let branch2 = GraphPattern::Extend {
            inner: Box::new(bgp(vp("s"), pred("http://ex/p2"), vp("o"))),
            variable: Variable::new("sum"),
            expression: Expression::Subtract(
                Box::new(Expression::Variable(Variable::new("o"))),
                Box::new(nine()),
            ),
        };
        // branch3: {?s :p3 ?o} BIND(?o + 2 AS ?sum)  -> s=c, o=10, sum=12 (DISJOINT)
        let branch3 = GraphPattern::Extend {
            inner: Box::new(bgp(vp("s"), pred("http://ex/p3"), vp("o"))),
            variable: Variable::new("sum"),
            expression: Expression::Add(
                Box::new(Expression::Variable(Variable::new("o"))),
                Box::new(two()),
            ),
        };

        let pattern = GraphPattern::Union {
            left: Box::new(GraphPattern::Union {
                left: Box::new(branch1),
                right: Box::new(branch2),
            }),
            right: Box::new(branch3),
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&pattern, &mut ctx).expect("eval");
            let schema = seq.schema.vars().to_vec();
            let s_col = seq.schema.index_of(&Variable::new("s")).unwrap();
            let sum_col = seq.schema.index_of(&Variable::new("sum")).unwrap();
            let resolved: Vec<(TermValue, TermValue)> = seq
                .rows
                .iter()
                .map(|row| {
                    (
                        ctx.scratch.value_of(&ds, row[s_col].unwrap()),
                        ctx.scratch.value_of(&ds, row[sum_col].unwrap()),
                    )
                })
                .collect();
            (schema, seq.rows, resolved)
        };

        let (schema_par, rows_par, resolved_par) = run(true);
        let (schema_seq, rows_seq, resolved_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential UNION paths must produce byte-identical row order"
        );
        assert_eq!(
            resolved_par, resolved_seq,
            "resolved (s, sum) values must match regardless of path"
        );
        // Row order: branch1 then branch2 then branch3 (left-to-right nesting).
        let expected_sums = vec!["11".to_owned(), "11".to_owned(), "12".to_owned()];
        let got_sums: Vec<String> = resolved_seq
            .iter()
            .map(|(_, v)| match v {
                TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(got_sums, expected_sums);
    }

    // -----------------------------------------------------------------------
    // The partial-lift channel
    // -----------------------------------------------------------------------

    /// A fuel trip, so the certificate under test names the governor the invariant is
    /// stated about rather than whichever one happens to be easiest to fire.
    const FUEL: purrdf_core::TrippedGovernor = purrdf_core::TrippedGovernor::Budget {
        dimension: purrdf_core::ResourceDimension::Fuel,
        limit: 4,
        consumed: 5,
    };

    #[test]
    fn fuel_trip_mid_optional_probe_never_emits_an_unbound_extension() {
        // `{ ?x :knows ?y } OPTIONAL { ?y :likes ?z }`. Ungoverned, `x=a, y=b` finds
        // `z=tea`, so the true answer binds `?z`. The hazard is a partial evaluation that
        // emits `x=a, y=b, z=UNBOUND` — a row the true query never returns, and one a
        // caller reading a lower bound would admit as an answer.
        let ds = graph();
        let left = bgp(vp("x"), pred("http://ex/knows"), vp("y"));
        let right_arm = bgp(vp("y"), pred("http://ex/likes"), vp("z"));
        let node = GraphPattern::LeftJoin {
            left: Box::new(left.clone()),
            right: Box::new(right_arm.clone()),
            expression: None,
        };

        let mut ctx = EvalCtx::new(&ds);
        let full = eval(&node, &mut ctx).expect("ungoverned optional");
        assert_eq!(full.len(), 1);
        assert!(
            full.rows[0].iter().all(Option::is_some),
            "ungoverned, the OPTIONAL binds ?z"
        );

        // Case 1: the LEFT arm truncates. The right arm has not been looked at, so the
        // left rows in hand must NOT be padded — no row may be emitted at all.
        let left_rows = eval(&left, &mut ctx).expect("left arm");
        let truncated_left = Evaluated::Truncated(Truncation::origin(left_rows.clone(), FUEL));
        let lifted = left_join_lift(
            &node,
            truncated_left,
            |_ctx| -> Result<Evaluated<TermId>, EvalError> {
                panic!("the right arm must not be evaluated once the left arm truncated")
            },
            None,
            &mut ctx,
        )
        .expect("lift over a truncated left arm");
        let Evaluated::Truncated(certificate) = lifted else {
            panic!("a truncated child must stay truncated");
        };
        assert_eq!(
            certificate.bound(),
            crate::governor::soundness::SpineClass::Certain,
            "the empty bag is a sound lower bound"
        );
        assert!(
            certificate.rows().is_empty(),
            "a left row padded with unbound here would assert that the OPTIONAL matched \
             nothing, about a right bag that was never evaluated: a fabricated answer"
        );

        // Case 2: the RIGHT arm truncates to nothing. Padding is the SAME fabrication
        // hazard one step down: a left row emitted alone asserts that the whole right bag
        // held no match, about a bag the evaluator holds only a prefix of. Worse, it is
        // emitted INSTEAD of the true rows the cut hid, so it is not an upper bound
        // either — the answers `l ⋈ m` past the cut would be missing from a result whose
        // only licence is "absent means definitively not an answer".
        let empty_right = SolutionSeq::empty(Arc::new(VarSchema::from_vars([
            Variable::new("y"),
            Variable::new("z"),
        ])));
        let lifted = left_join_lift(
            &node,
            Evaluated::Complete(left_rows.clone()),
            move |_ctx| Ok(Evaluated::Truncated(Truncation::origin(empty_right, FUEL))),
            None,
            &mut ctx,
        )
        .expect("lift over a truncated right arm");
        let Evaluated::Truncated(certificate) = lifted else {
            panic!("a truncated child must stay truncated");
        };
        assert_eq!(
            certificate.bound(),
            crate::governor::soundness::SpineClass::Certain,
            "with the padding suppressed the operator is an inner join over a prefix of \
             the right bag, so every row it does emit is an answer"
        );
        assert!(
            certificate.rows().is_empty(),
            "the empty right prefix pairs with nothing, and the left rows must NOT be \
             padded out in its place"
        );
        assert_eq!(certificate.tripped(), FUEL);

        // And a NON-empty right prefix still emits the pairings it actually found: the
        // suppression removes fabricated rows, not real ones.
        let right_rows = eval(&right_arm, &mut ctx).expect("right arm");
        assert!(!right_rows.is_empty(), "the fixture's OPTIONAL does match");
        let lifted = left_join_lift(
            &node,
            Evaluated::Complete(left_rows),
            move |_ctx| Ok(Evaluated::Truncated(Truncation::origin(right_rows, FUEL))),
            None,
            &mut ctx,
        )
        .expect("lift over a truncated right arm");
        let Evaluated::Truncated(certificate) = lifted else {
            panic!("a truncated child must stay truncated");
        };
        assert_eq!(
            certificate.bound(),
            crate::governor::soundness::SpineClass::Certain
        );
        assert_eq!(certificate.rows().len(), 1);
        assert!(
            certificate.rows().rows[0].iter().all(Option::is_some),
            "the surviving row is a genuine pairing, not a padded one"
        );
        assert!(
            certificate.certain_rows().is_some(),
            "and a genuine pairing IS reachable through the certified-answer channel"
        );
    }

    #[test]
    fn ungoverned_evaluation_is_unchanged_and_always_complete() {
        // Every binary operator over the same fixture, driven through the ordinary
        // dispatch with no governors attached: each must answer `Complete`, and the rows
        // must be exactly what the completion-required entry point returns.
        let ds = graph();
        let knows = bgp(vp("x"), pred("http://ex/knows"), vp("y"));
        let likes = bgp(vp("y"), pred("http://ex/likes"), vp("z"));
        let plans = [
            GraphPattern::Join {
                left: Box::new(knows.clone()),
                right: Box::new(likes.clone()),
            },
            GraphPattern::Union {
                left: Box::new(knows.clone()),
                right: Box::new(likes.clone()),
            },
            GraphPattern::LeftJoin {
                left: Box::new(knows.clone()),
                right: Box::new(likes.clone()),
                expression: None,
            },
            GraphPattern::Minus {
                left: Box::new(knows.clone()),
                right: Box::new(likes.clone()),
            },
            GraphPattern::Lateral {
                left: Box::new(knows),
                right: Box::new(likes),
            },
        ];

        for plan in plans {
            let mut ctx = EvalCtx::new(&ds);
            let evaluated = eval_evaluated(&plan, &mut ctx).expect("ungoverned eval");
            assert!(
                !evaluated.is_truncated(),
                "an ungoverned execution has no ceiling to exceed and no signal to \
                 observe, so it can only be complete"
            );
            let via_channel = evaluated.rows().clone();

            let mut ctx = EvalCtx::new(&ds);
            let via_entry = eval(&plan, &mut ctx).expect("completion entry point");
            assert_eq!(via_channel.schema, via_entry.schema);
            assert_eq!(
                via_channel.rows, via_entry.rows,
                "the channel must not change a single row of an ungoverned result"
            );
        }
    }

    /// A [`crate::StopSignal`] that fires on its `n`-th poll and latches thereafter.
    #[derive(Debug)]
    struct StopOnPoll {
        /// Polls remaining before the signal fires.
        remaining: std::sync::atomic::AtomicU64,
    }

    impl crate::StopSignal for StopOnPoll {
        fn poll(&self) -> Option<purrdf_core::StopCause> {
            // `try_update` (the non-deprecated name for this atomic) postdates the
            // workspace MSRV floor, so the original name is kept here.
            #[allow(deprecated)]
            let previous = self
                .remaining
                .fetch_update(
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                    |left| Some(left.saturating_sub(1)),
                )
                .unwrap_or(0);
            (previous == 0).then_some(purrdf_core::StopCause::Cancelled)
        }
    }

    /// A three-edge `knows` chain with `likes` edges, so the `UNION`'s left branch has
    /// several per-input-row commit boundaries to be stopped between.
    fn wide_graph() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("https://example.org/knows");
        let likes = b.intern_iri("https://example.org/likes");
        let a = b.intern_iri("https://example.org/a");
        let bb = b.intern_iri("https://example.org/b");
        let c = b.intern_iri("https://example.org/c");
        let d = b.intern_iri("https://example.org/d");
        let tea = b.intern_iri("https://example.org/tea");
        let cake = b.intern_iri("https://example.org/cake");
        b.push_quad(a, knows, bb, None);
        b.push_quad(bb, knows, c, None);
        b.push_quad(c, knows, d, None);
        b.push_quad(bb, likes, tea, None);
        b.push_quad(c, likes, cake, None);
        b.push_quad(d, likes, tea, None);
        b.freeze().expect("freeze")
    }

    /// `{ LATERAL { ?x :knows ?y } { ?y :likes ?z } } UNION { ?p :likes ?q }`.
    ///
    /// The left branch commits one block per left row, so a stop lands *between* blocks
    /// with rows already in hand; the right branch is non-empty, so a result that
    /// wrongly admitted it would be visibly larger.
    fn union_over_lateral() -> GraphPattern {
        GraphPattern::Union {
            left: Box::new(GraphPattern::Lateral {
                left: Box::new(bgp(vp("x"), pred("https://example.org/knows"), vp("y"))),
                right: Box::new(bgp(vp("y"), pred("https://example.org/likes"), vp("z"))),
            }),
            right: Box::new(bgp(vp("p"), pred("https://example.org/likes"), vp("q"))),
        }
    }

    /// Everything about one evaluation that a caller can observe, rendered so two runs
    /// over the same dataset compare structurally rather than by interner identity.
    type UnionObservation = (bool, Option<String>, Vec<String>, Vec<Vec<Option<String>>>);

    fn observe(ds: &RdfDataset, evaluated: &Evaluated<TermId>) -> UnionObservation {
        let scratch = crate::scratch::ScratchInterner::new();
        let seq = evaluated.rows();
        let truncated = match evaluated {
            Evaluated::Complete(_) => None,
            Evaluated::Truncated(truncation) => Some(truncation.describe()),
        };
        let vars = seq
            .schema
            .vars()
            .iter()
            .map(|v| v.as_str().to_owned())
            .collect();
        let rows = seq
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| {
                        cell.map(|t| match scratch.value_of(ds, t) {
                            TermValue::Iri(iri) => iri,
                            other => format!("{other:?}"),
                        })
                    })
                    .collect()
            })
            .collect();
        (evaluated.is_truncated(), truncated, vars, rows)
    }

    /// Evaluate `plan` under a stop signal that fires on its `budget`-th poll.
    fn run_under_budget(ds: &RdfDataset, plan: &GraphPattern, budget: u64) -> Evaluated<TermId> {
        let signal = Arc::new(StopOnPoll {
            remaining: std::sync::atomic::AtomicU64::new(budget),
        });
        let governors = crate::QueryGovernors::UNBOUNDED.with_stop_signal(signal);
        let state = Arc::new(crate::GovernorState::new(&governors));
        let mut ctx = EvalCtx::new(ds).with_governors(state);
        eval_evaluated(plan, &mut ctx).expect("governed eval")
    }

    #[test]
    fn union_truncation_is_identical_under_forced_parallel_and_forced_sequential() {
        // A governor may not report more rows because a second thread got there, and
        // under engaged governors it now cannot: `EvalCtx::may_fork_sibling_patterns` is
        // false, so a governed `UNION` evaluates its arms in source order on one thread
        // whatever the scheduler wants. (It did not always. Forking them let the two arms
        // race the one shared fuel counter, and the same query, data and budget produced
        // seven different answers across sixty runs — see
        // `tests/governor_correctness.rs`'s
        // `a_governed_union_reports_one_outcome_however_it_is_scheduled`.)
        //
        // What this test pins is therefore the *parallel gate's* irrelevance to a governed
        // result: driving `force_parallel_for_test` and `force_sequential_operation`
        // against each other, over a whole ladder of budgets, must not move a single row.
        // The one-worker pool keeps the poll-counting signal comparable across the two
        // runs; the DISCARD rule that the fork used to need is pinned directly on
        // `union_branch_order` at the bottom of this test, where it does not depend on
        // winning a race to be exercised.
        //
        // `force_parallel_for_test` governs the chunked row loops rather than the UNION
        // fork — the fork's own gate is `force_sequential_operation` — so both are driven
        // here, each on the thread that will read it.
        {
            let ds = wide_graph();
            let governors = crate::QueryGovernors::UNBOUNDED.with_fuel(64);
            let state = Arc::new(crate::GovernorState::new(&governors));
            let governed = EvalCtx::new(&ds).with_governors(state);
            assert!(
                !governed.may_fork_sibling_patterns(),
                "an engaged governor must refuse the UNION fork outright"
            );
            assert!(
                EvalCtx::new(&ds).may_fork_sibling_patterns(),
                "an ungoverned UNION has no counter to race, so it keeps the fork"
            );
            let unbounded = Arc::new(crate::GovernorState::new(&crate::QueryGovernors::UNBOUNDED));
            assert!(
                EvalCtx::new(&ds)
                    .with_governors(unbounded)
                    .may_fork_sibling_patterns(),
                "UNBOUNDED declines the accounting as well as the ceilings, so it has no \
                 counter to race either"
            );
        }
        let ds = wide_graph();
        let plan = union_over_lateral();

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("one-worker pool");

        let mut saw_truncated_with_rows = false;
        let mut saw_complete = false;
        // Upper end raised from the pre-Values-Insertion 40: `union_over_lateral`'s
        // `LATERAL` arm now evaluates `Join(Bgp, Values)` per left row on its right
        // side (Values Insertion's leaf join, `crate::expr::substitute_pattern`)
        // instead of a single term-rewritten `Bgp`, so completing it costs a few
        // more node-entry polls than before. The exact count is an implementation
        // detail; the margin is generous rather than tight.
        //
        // NOT tightened back by the shared-str term-text change (`SubstitutionRow`'s
        // term text now lives behind `Arc<str>` — see `purrdf_sparql_algebra::
        // NamedNode`'s doc): that removed per-leaf TEXT allocation, but the governor
        // polls PLAN NODES, and the extra `Join`/`Values` node this bound accounts
        // for is still built once per left row regardless — the node COUNT this
        // bound tracks is unchanged by making that node's content cheap to
        // construct.
        for budget in 1..=60_u64 {
            let sequential = {
                let _sequential = crate::parallel::force_sequential_operation();
                let evaluated = run_under_budget(&ds, &plan, budget);
                observe(&ds, &evaluated)
            };
            let parallel = pool.install(|| {
                let _parallel = crate::parallel::force_parallel_for_test(true);
                let evaluated = run_under_budget(&ds, &plan, budget);
                observe(&ds, &evaluated)
            });

            assert_eq!(
                sequential, parallel,
                "budget {budget}: the same query, data and budget must yield the same \
                 partial result whichever path evaluated it"
            );
            saw_complete |= !sequential.0;
            saw_truncated_with_rows |= sequential.0 && !sequential.3.is_empty();
        }

        assert!(
            saw_complete,
            "the largest budgets must let the query finish, or the complete path is untested"
        );
        assert!(
            saw_truncated_with_rows,
            "some budget must stop the UNION with left-branch rows already committed, or \
             the rule under test is never reached"
        );

        // The combination above cannot reach: a LEFT branch that truncated beside a RIGHT
        // branch that COMPLETED with rows. A latched signal stops everything evaluated
        // after it, so only genuine concurrency — the right branch finishing before the
        // left branch's trip becomes known — produces it, and racing for it would be a
        // test that passes by luck. The rule is therefore pinned directly on the function
        // the parallel path calls: the computed right rows are discarded, so the result
        // is the same one the sequential body (which never starts the right branch)
        // produces.
        let truncated_left = UnionBranch {
            schema: Arc::new(VarSchema::from_vars([Variable::new("x")])),
            rows: vec!["left-1", "left-2"],
            certificate: Some(crate::governor::lift::Certificate::origin(
                purrdf_core::TrippedGovernor::Stopped {
                    cause: purrdf_core::StopCause::Cancelled,
                },
            )),
        };
        let complete_right = UnionBranch {
            schema: Arc::new(VarSchema::from_vars([Variable::new("p")])),
            rows: vec!["right-1", "right-2", "right-3"],
            certificate: None,
        };
        let (left_branch, right_branch, governing) =
            union_branch_order(truncated_left, complete_right);
        assert_eq!(left_branch.rows, vec!["left-1", "left-2"]);
        assert!(
            right_branch.rows.is_empty(),
            "a right branch computed only because rayon started it must not enlarge a \
             governed result"
        );
        assert!(right_branch.schema.is_empty());
        assert_eq!(
            governing.map(|(ordinal, _)| ordinal),
            Some(0),
            "the left branch's truncation governs"
        );

        // And the mirror case: a completed left beside a truncated right keeps both,
        // because the right's partial rows are a genuine suffix of the concatenation.
        let complete_left = UnionBranch {
            schema: Arc::new(VarSchema::from_vars([Variable::new("x")])),
            rows: vec!["left-1"],
            certificate: None,
        };
        let truncated_right = UnionBranch {
            schema: Arc::new(VarSchema::from_vars([Variable::new("p")])),
            rows: vec!["right-1"],
            certificate: Some(crate::governor::lift::Certificate::origin(
                purrdf_core::TrippedGovernor::Stopped {
                    cause: purrdf_core::StopCause::Cancelled,
                },
            )),
        };
        let (left_branch, right_branch, governing) =
            union_branch_order(complete_left, truncated_right);
        assert_eq!(left_branch.rows, vec!["left-1"]);
        assert_eq!(right_branch.rows, vec!["right-1"]);
        assert_eq!(governing.map(|(ordinal, _)| ordinal), Some(1));
    }

    #[test]
    fn a_truncated_left_arm_reports_the_evaluated_arms_schema() {
        // The stated contract, pinned: see this crate's `governor` module documentation.
        // When the LEFT arm of a JOIN / OPTIONAL / LATERAL / UNION truncates, the right
        // arm is never evaluated, and this engine chooses column ORDER during evaluation
        // (a BGP's columns appear in cost-based join order), so the right arm's columns
        // are not derivable without the work that was just refused. The partial result
        // therefore reports the LEFT arm's columns. No row is affected — none cross —
        // but a caller diffing column lists must expect the narrower list.
        let ds = wide_graph();
        let left = bgp(vp("x"), pred("https://example.org/knows"), vp("y"));
        let right = bgp(vp("p"), pred("https://example.org/likes"), vp("q"));
        let node = GraphPattern::Join {
            left: Box::new(left.clone()),
            right: Box::new(right),
        };

        let mut ctx = EvalCtx::new(&ds);
        let complete = eval(&node, &mut ctx).expect("ungoverned join");
        assert_eq!(
            complete
                .schema
                .vars()
                .iter()
                .map(|v| v.as_str().to_owned())
                .collect::<Vec<_>>(),
            vec!["x", "y", "p", "q"],
            "a complete join reports both arms' columns"
        );

        let left_rows = {
            let mut ctx = EvalCtx::new(&ds);
            eval(&left, &mut ctx).expect("left arm")
        };
        let mut lift = Lift::at(&node);
        let absorbed = lift
            .absorb(
                0,
                Evaluated::Truncated(Truncation::origin(
                    left_rows,
                    purrdf_core::TrippedGovernor::Stopped {
                        cause: purrdf_core::StopCause::Cancelled,
                    },
                )),
            )
            .expect("a monotone edge keeps its rows");
        assert!(lift.is_truncated());
        let Evaluated::Truncated(truncation) =
            lift.finish(SolutionSeq::<TermId>::empty(absorbed.schema))
        else {
            panic!("a truncated child must stay truncated");
        };
        assert_eq!(
            truncation
                .rows()
                .schema
                .vars()
                .iter()
                .map(|v| v.as_str().to_owned())
                .collect::<Vec<_>>(),
            vec!["x", "y"],
            "the partial result reports the evaluated arm's columns"
        );
        assert!(
            truncation.rows().is_empty(),
            "and no row crosses, so the narrower column list describes an empty bag"
        );
    }
}
