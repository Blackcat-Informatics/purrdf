<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 prepared EXISTS lifetime: bounded source diagnosis

VERDICT: SOURCE-VALID CANDIDATE, UNMEASURED. Read-only review of #470 HEAD `838667cab` and current N-owned source. No build, production edit, stack/floor change, or new profiling tool was used. The retained optimized package measured 7,104 WASM shadow-stack bytes per nested NOT EXISTS level from depth 2 and failed the original synchronous/job and NOT EXISTS floors; this review does not discharge them.

The repeated test builds `?s p ?o FILTER NOT EXISTS { ... }` around a deeper body. On WASM `EvalCtx::may_fork_row_loop` forces the sequential `eval_filter_sequence` arm. Its `linked.filter` suspends at the VM's `EXISTS`, where `resolve` calls `exists_recorded` and then `exists_prepared`. For a nonterminal level, the inner filter contains another `EXISTS` correlated through `?s`. `exists_use_probe` runs `probe_admissible_admitted`; `expr_probe_step` refuses a nested `EXISTS` whose free variable intersects the current-row variables. The descending call is therefore the per-row definition arm's `binop::eval_correlated`, then `eval_substituted_delivered`, substituted-window guard, and `eval_evaluated` of the next level. The terminal body can make a different strategy choice.

The normal JS `Dataset`/`QueryEngine` default does not disable physical shapes (`EvalOptions::default().disable_shapes == false`), so option state alone cannot exclude the Shapes barrier. Its admission does exclude this control: `shapes::barrier::Plan::prepare` requires `certificate(pattern)`, whose root must be `GraphPattern::Filter` with a BGP inner and an `Expression::And` containing lower and upper ordinal comparisons. Each repeated control inner is a BGP followed by a `FILTER NOT EXISTS`, not that conjunction. `Plan::prepare` returns `None`, cached under the site; `shapes::barrier::inner` returns `Ok(None)` and the original `eval_correlated` call runs. The asynchronous evidence object reported by `queryGovernedAsync` is job telemetry, not evidence that a Shapes barrier plan was admitted.

At the definition arm's suspension, `exists_prepared` must retain the prepared site/normalized and witness pattern, ledger source, row/schema, optional evidence destination, restriction memo key, and the chosen site map. Those are needed after the child returns to pop the ledger map, propagate a truncation to the expression barrier, charge the answer and consumed solutions, compute existential evidence, and publish a memoized answer. `eval_correlated` must retain its substituted row until `eval_substituted` finishes; `eval_substituted_delivered` must retain the substituted tree, placeholder/ledger guard and previous pushdown/cursor until the nested evaluation returns. Dropping these owners or moving answer charges across the child call would change semantics. The VM's `resolve`/`advance` boundary is already out of line; another generic wrapper there is not a demonstrated saving.

One bounded source change is to move **only the memoized-probe arm** of `exists_prepared` (current `if exists_use_probe(...) { ... }` body) into an out-of-line helper, leaving collision checks, native-positive decision, strategy admission, and the per-row definition arm in their current order and frame. The helper must own the current probe key/cache lookup, ledger-map push/evaluate/pop, truncation and probe-answer charges, admitted inner/index construction, cache insertion, witness probe, and final boolean. Pass borrowed site/normalized/ledger source, row/schema, optional evidence destination and `ctx`; do not introduce a new allocation or clone, alter the cache key, or move any charge. The repeated correlated definition path would not call the new helper, so this split adds no helper frame at each descending level. It removes the probe's `Evaluated`/index/cache-result and witness locals from the same source function that holds the definition path across recursive descent. This is a source-supported lifetime boundary, not proof of smaller optimized stack slots: LLVM may already reuse/disjoint those slots.

The deciding runtime evidence is a same-source optimized WASM build after that single split: compare per-level stack high-water slope and original three floor bisections against the retained baseline; also check probe-path answer/evidence, memo reuse, ledger attribution, first refusal and truncation, because the extracted helper *does* add a frame when the probe itself evaluates an inner pattern. If the 7,104-byte definition slope does not fall, this split is not the correction; inspect the optimized frame at the actual `exists_prepared` definition call through `eval_correlated` and the substituted-window guard. Do not infer a frame owner from the source function's size alone.

## Root review of the written correction

Root read the actual `expr.rs` delta after the fresh pre-split package completed.
The new out-of-line `exists_memoized_probe` receives six borrowed/current context
arguments. The moved body preserves its cache key and substituted-cache exclusion,
ledger-map push and pop before propagating the child Result, truncation barrier,
answer charge, admitted shared/index owners, cache insertion and evidence result.
The definition arm and strategy order are unchanged. Existing clones remain
existing clones; this extraction introduces no allocation or cloned owner.
Bounded source verdict PASS. Native semantic controls and decisive optimized
stack/floor evidence remain UNMET for this written split. The separate measured
LATERAL helper still fails its original floor and is not a completed #531 repair.
