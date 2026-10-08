<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Targeted RowDelivery native-cost recheck

Read-only source/previous-artifact review while the sole implementer is active.
This is not Task 1 acceptance or a native-performance PASS. No builds, tests,
timings, environment changes, commits, forge activity or shipping edits were
performed. Only this report was created; no temporary probes/debris were made.
The required final proof remains work within this issue, not deferred work.

## Identity

The detached baseline was independently read back as clean at HEAD
`18489a91cb452b1773c60b469d4c30dc33a9a4a4`, tree
`5d150fa4076cc75e293b517f79dd080b6ea807ed`. Candidate is the live issue worktree
with that HEAD and ongoing uncommitted implementation. Its tracked binary-diff
SHA-256 was `9ffb319246dcadef8caae23d2597bf585f615d58f420bf51c49533a410ebe357`
at both read-time samplings. Plan SHA-256 remains
`8627bc25be581a26269cf8ad5cbe9f826d2917f6168d8cb5e35c71e132ca01f6`.
These are sampled identities, not an atomic snapshot or qualification binding.

Read-time file SHA-256 values:

| File under crates/sparql-eval/src | SHA-256 |
|---|---|
| eval.rs | 211f0a8eb7560ad2a15dcb257106a2c7085c0407f3b97ac486b6cf5a67ea2480 |
| modifier.rs | 27677c50b8a15d54ac0c52984fdf859b74179b56030716af1f4d9340be436d6a |
| expr.rs | 8cc4ea27d9b19db10af59451bfaf45b235a626f3ddebf8417fa39f8c00adedef |
| binop.rs | a01cf48064aadcf0133a31db4c56b5d1f86e9fd55f5298f02546adf53176ac15 |
| engine.rs | f2322feae4b27406fb886715ac31a8c7a6fd13714c4c31e02902ee2d3255a580 |

## Source conclusions

No unit-mode row delivery, additional native row clone, block collection,
dynamic callback or extra whole-plan admission traversal was found in the
inspected changes. These are source-level conclusions about specialization,
not claims of equal compiled instructions or frame sizes.

- Ordinary `eval_node` calls `eval_node_with(pattern, (), ctx)`. Unit has
  `ACTIVE = false`, returns unit from reborrow, never stops and has a no-op
  delivery method. The streamed classifier short-circuits false and the
  function returns `evaluate(ctx)` before delivery's row clone/one-row Vec loop.
  No delivery reference or dynamic function is carried by the unit value.
- Filter/Extend/Project/Slice/Union unit wrappers route to their existing
  native operator. Their streaming transforms, block collections and child
  yielding calls are const-unreachable. Filter/Extend/Project native sequence
  operations were moved into inline-always helpers without introducing a
  source-level allocation or second traversal; their native operator functions
  retain inline-never boundaries.
- Dedup's ordinary dispatch selects unit delivery and `ADJACENT = false`.
  Its original hash-map, ordinal ordering and shared-blank masking remain;
  streamed `seen`, previous-row clones and adjacent dedup are unreachable.
  The renamed inline-never false kernel replaces the original inline-never
  native body behind an inline-always wrapper.
- `eval_graph_with<D, ()>` remains inline-never. Fixed GRAPH retains the same
  graph lookup, active-graph save/restore, single child evaluation and lift.
  Variable GRAPH's stop checks and consumer closures are const-unreachable;
  old graph census/empty-graph narrowing, child evaluation, errors/restoration,
  output buffer and empty/no-graph schema cases remain. `append_graph_rows`
  reproduces the old schema clone, existing-column compatibility branch or
  append/resize branch, using the SAME native output Vec across graphs. It
  creates no second native buffer or extra schema allocation. It is marked
  inline-always, but native helper return/copy elimination still needs evidence.
- `GroupDomain = ()` calls the original shared-blank `visible_columns` home.
  `CONTEXTUAL = false` removes the contextual aggregate scan and yielding group
  path. Native grouping, aggregate evaluation and DISTINCT-star identity
  retain the old algorithms. Its domain parameter has zero size, and the
  native Group body retains an inline-never boundary behind its wrapper.
- Ordinary application now uses `ApplicationMode = ()`, replacing the prior
  optional policy ABI argument; the earlier correlation `Some(sites)` wrapper
  has been removed in favor of definite initialization on the native branch.
  Native correlation selects DECLARED=false, FIRST=false, unit delivery: no
  witness Slice construction/cap plan, current_row reset or callback enters
  that path. Wrapper and callee codegen still require comparison.
- Ordinary owned-algebra preparation selects `from_algebra::<false>` and the
  existing admission walk's false specialization. Height admission and one
  structural traversal remain. The Apply rejection is an arm of that existing
  traversal, not a new preflight whole-query scan. Existing prepared-plan
  soundness rechecks still select the shared admitted-algebra validation.

## Concrete source-operation drift to resolve or prove

Native LATERAL previously read `left_len = left_schema.len()` once before the
driver loop and copied each accepted row's prefix using that value. The new
`merge_application_row` reads `left.len()` inside each accepted `(mu, nu)`
merge. Equal row width is an invariant, but the compiler may not infer it from
the schema. This is a concrete change from one explicit width read to repeated
source reads, with potentially different bounds-check work. It was sent to the
implementer and parent. Supply a precomputed native prefix width or show the
actual compiled load/check hoisting; do not call it equivalent just because the
helper is inline-always. This is a risk requiring resolution, not an observed
machine-code regression.

## Compiled proof required at final identity

The base now has attributable `.ll` and `.s` files under
`raw/native-cost-base-ir`. This supersedes the preliminary review's lack of
named base sections. The eval crate file with stem
`qualification_454_native_cost.purrdf_sparql_eval-0ecc65cc8c2d1ea3.purrdf_sparql_eval.85e4f8f062a88157-cgu.0.rcgu.o.rcgu`
contains production `RdfDataset` monomorphizations. Examples: base IR eval_node
at line 240841, Filter at 248371, Graph at 343618, Group at 347146. Base assembly
eval_node begins at 248823 with two saved-register pushes and `subq $312, %rsp`
(CFA offset 336). These are specific base observations, not candidate comparisons.
The IR dispatcher already has three 96-byte allocas: source thinness does not
mean a zero-byte base frame, and raw alloca sums do not establish machine frames.

Final paired evidence must cover:

1. **Recursive dispatcher:** map base eval_node and its caller to candidate
   eval_node_with<D, ()>. Show the closure/reborrow/no-op delivery scaffolding
   has erased: no callback/vtable, delivery storage/test, row clone, materialized
   block, added wrapper call or residual producer classifier. Compare actual
   frame/spills and per-level call depth, including eval_evaluated's caller.
   Confirm native operator bodies remain outside this recursive frame. This
   is required because a larger frame repeats at every nested evaluation,
   including ordinary FILTER/EXISTS recursion and wasm shadow-stack use.
2. **Native kernels/call boundaries:** map native Filter, Extend, Project,
   Slice, Union, Dedup, fixed/variable Graph, Group and LATERAL to their unit/
   false candidate bodies. Verify unit ABI arguments erase; wrappers do not add
   calls; extracted sequence/graph/merge helpers do not add calls, aggregate
   return copies, extra cleanup, allocations or changed loop/bounds work.
   Specifically inspect per-graph output buffer reuse and prefix-width drift
   above. Source relocation alone cannot prove codegen.
3. **Group and correlation:** compare native GroupDomain visibility calls and
   aggregate DISTINCT-star paths, plus DECLARED=false/FIRST=false/unit
   substitution. Exclude contextual group scan, current_row reset, witness
   Slice/cap planning, delivery closure and policy/tag setup. Preserve charging,
   errors, endpoint/property-function intercept and deferred-site operation
   counts; fix any surviving extra native operation.
4. **Shared dispatch and admission:** retain the preliminary review's enum
   layout/offset/alignment and old-variant visitor/hash/clone/drop argument.
   Pair the changed existing admission walk and false-specialization routing;
   no extra tree scan should appear. Equal GraphPattern size alone does not
   exclude added dispatch compares or payload movement.
5. **Attribution/coverage:** pin final source/tree, probe/compiler/profile/
   target/flags for both sides. Update allocator/layout/retained receipts after
   final implementation. The previous seven samples predate RowDelivery and
   cannot qualify this change. They also omit GRAPH fixed/variable, COUNT
   DISTINCT *, parameterized/SHACL paths and property-function interception;
   cover touched uncovered paths with bounded untimed structural evidence or
   attributable native specialization arguments. No timing runs on this host.
   For wasm thin-frame claims, supply target evidence rather than treating
   x86-64 frame equality as a wasm result.

Normalize symbol names/addresses when comparing; retain loads, branches, calls,
spills, exceptional cleanup and allocation sites. Exact whole-binary equality
is unnecessary. Source-sufficient operation equivalence is acceptable where
there is no codegen-sensitive change, but ZST/const facts and inline annotations
cannot alone certify ABI/call-frame/instruction equality at changed boundaries.

The source design gives a coherent cost-free unit route at the algorithm level.
There is no demonstrated allocation regression in this read. The width-read
drift and final paired compiled/structural evidence remain unresolved acceptance
work; no performance or Task 1 PASS is granted.
