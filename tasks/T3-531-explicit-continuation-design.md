<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531: explicit admitted evaluation continuation (design for material review)

Status: **design only; no implementation or depth claim**. Source base is the clean
`330c55f8c` N checkout. The compact-ownership experiment was rejected and
reverted (`T3-531-compact-continuation-disposition.md`): native semantics passed,
but its exact optimized module answered only 233/281 right-nested LATERAL and
145/180 FILTER NOT EXISTS levels. The original synchronous SELECT floor also
remains required. Moving retained row payloads to boxes did not remove the
recursive evaluation call chain. Further isolated no-inline helpers are not a
credible complete correction.

Independent narrow source review is recorded in
`../reviews/T3-531-explicit-continuation-source-review.md`. It confirms that
the VM linker and suspended EXISTS site must be converted together, and that
scope restoration needs one shared LIFO body on ordinary return, typed error
and unwind. It did not review all delivery/operator cases or qualify depth.

## Existing computation that must become an effect

`eval_evaluated_with` owns per-node stack check, Slice-cap installation, source
read checks and optional search-evidence push; `eval_evaluated_inner_with`
selects the no-governor fast path or the existing entry/output ledger charges.
`eval_node_with` dispatches every algebra arm. A LATERAL row reaches
`eval_application → eval_correlated → eval_substituted_delivered → eval_evaluated`.
A FILTER predicate reaches `eval_filter_sequence → Linked::filter →
Linked::term → VmStep::Suspend(Exists) → exists_recorded`, whose merged deferred
definition reaches the same substituted evaluation. Each call is still live
when the next algebra level begins. An owned row box alone therefore cannot
remove the repeating machine frames.

The proposed engine is **one loop over admitted frames**, not a second query
evaluator or a special recognizer of the three test query strings. A step may
return `Enter(child)`, `SuspendExpression(site)`, `Yield(result)`, or a typed
`EvalError`. The loop pops/resumes the parent frame after a child finishes.
Frames cover the actual operator families on the repeating paths: node
admission/exit, LATERAL left then each right row then ordered merge, FILTER
child then each sequential predicate row, VM suspension/resume, deferred or
prepared EXISTS probe/definition, and correlated substitution. Other arms
retain their existing production bodies initially; entering one from a frame
still uses the original stack check and typed refusal. This is **not** a claim
that arbitrary deep mixtures of all algebra variants become iterative: the
accepted depth routes and every production caller of the converted arms must
be checked before limiting the migration to these variants.
The copied trees on those routes may add identity `Join`, `Values` or `Slice`
wrappers. If an actual repeated wrapper recurses before the next LATERAL or
FILTER, its enter/finish effect must also be represented by the same loop;
leaving one ordinary recursive frame per level would defeat this design.

Use `AdmittedVec<Frame>` (the existing workspace-accounted vector home) for
the logical stack. A large variant may retain an existing `WorkspaceBox` or
`SharedWorkspace` payload, but every added control/slot and every growth is
admitted before physical allocation. `Frame` contains no `&mut EvalCtx`, no
pointer into its own payload, and no unadmitted closure. The runner borrows
`EvalCtx` only for one step; it neither retains that borrow nor clones a row
on suspension. A returned `Evaluated` moves once into the parent slot.
The existing plan-height admission (`stack/height.rs`) remains in force, so
the 20,000-level refusal is still typed even if the evaluation loop itself
uses constant machine stack. The original per-node `stack::check` remains at
entry for non-trampolined leaf/operator calls and nested user functions.
Success on an ordinary default row must not gain an unmeasured hidden
allocation: count per-node frame/vector growth and compare the existing
allocation/fixed-work controls. If a new ordinary allocation is needed, it is
an explicit regression to evaluate against #531 rather than a free stack win.

## Safe ownership and exact ordering

* `CorrelatedPattern` already owns its boxed copied tree and `LexicalFrame`;
  `PreparedPattern` already shares that owner with admitted control storage.
  A frame for a copied tree keeps `PreparedPattern` plus a *safe structural
  node path*, never a reference into its own frame or a raw address. Resolve
  the path for the duration of one step. Original prepared-query nodes remain
  borrowed from the runner's external query lifetime. Resolution must reach
  the **same node of the same copied tree**, preserving address-keyed memo and
  ledger maps; rebuilding/cloning a subtree at resume would break identity.
  The path's bytes and traversal work need admission/work accounting.
* `Linked` currently retains `Vec<&GraphPattern>` for `EXISTS`, and
  `VmStep::Suspend(Exists(&GraphPattern))` borrows that link. A suspended link
  cannot be moved into a frame beside a reference into itself. Keep its owned
  program, slots, constants, value stack and evidence in the frame; represent
  the pending EXISTS as its existing compiled site ordinal until one step
  resolves the pattern through the external or `PreparedPattern` owner. The
  original UDF suspension keeps its evaluated arguments and registry identity;
  resolve through the existing VM home, retaining program counter and evidence
  record across child evaluation. No expression replay is permitted because
  stateful functions and first errors depend on one execution.
  The ordinary linker is used by FILTER, OPTIONAL predicates, ORDER keys and
  aggregates; changing only the suspend payload leaves its `Vec<&GraphPattern>`
  self-borrow. A single site representation must cover those externally
  borrowed expressions and copied-tree ownership. The existing algebra
  `NodeRef::for_each_child` is the shared child-order home for an admitted
  structural path into a copied tree; do not duplicate a pattern/expr walker
  or use a raw pointer. A copied-root owner must be an actual caller when this
  site representation lands, since an unused owned-site compatibility variant
  would not fix the recursion. Empty/no-EXISTS links must retain their current
  zero-allocation footprint.
* `EvalScopeGuard` presently holds `&mut EvalCtx` and restores substitution
  flag, deferred map and correlated ledger map in `Drop`. It cannot be stored
  beside the runner's `&mut EvalCtx`. Factor its existing `ScopeRestore` state
  into a detached token used by both the recursive guard and the runner. The
  runner owns `&mut EvalCtx`; normal completion and error unwind restore tokens
  in LIFO order **before** releasing their copied AST owners. A runner `Drop`
  must restore outstanding tokens if a Rust unwind occurs. Do not hand-write a
  second independent restoration algorithm.
* A node-entry frame records whether it pushed search evidence and a Slice
  cap, its node cursors and ledger charge state. After child completion it
  performs the original **search pop → cap clear → source read error check →
  evidence combine/parent propagation** order. Entry stop checks and node
  charge precede node work; output charge and post-operator stop precede
  cursor restoration. An inner operational failure cannot be replaced by a
  generic expression false, except where the original expression semantics
  deliberately map a type error to unbound.
* A FILTER frame retains the one `Linked` VM, row, `RowCheckpoint`, output
  builder and `Lift`. `checkpoint.pass` precedes the predicate; only a true
  result attaches computed evidence and calls `keep`; `settle` follows the
  predicate. The unconsumed row iterator drops at the old loop-exit point,
  before `RowsBuilder::finish` or barrier processing. The native worker/fork
  path retains its original single implementation and witness harvest/ordered
  commit. It must not accidentally enter the sequential continuation path.
* A LATERAL frame retains the left bag/schema, current row, per-row results,
  deferred site and `Lift`. Property-function interception, SERVICE endpoint
  preflight and placeholder refusal remain before ordinary right-row work.
  For each row, substitute/evaluate right once, keep a complete result with
  its original owner, or absorb a truncated block and stop. Schema union and
  merge run only after right children return, in original row order, with
  the existing cell-ceiling admission. Declared application optional retry
  and `FIRST` cap/cursor restoration remain separate states, not a replay.
* The VM's `RowDelivery`/`RowConsumer` active path cannot be held as a borrow
  into a movable frame. The runner receives the original caller-owned
  consumer and frames encode delivery effects/indices; callbacks fire at the
  same completed-block points as `yield_transform` and may stop before another
  child. An implementation that only trampolines unit `M=()` but silently
  replays or drops contextual/async delivery is incomplete.
  In current source `eval_evaluated_with` takes a `FnOnce` around node dispatch,
  and `yield_transform` creates a stack-local callback that enters a child and
  clones its emitted block before transformation. Neither closure can be
  retained across a suspended child. The converted arms must call the same
  node-entry/exit and completed-block transformation homes as the recursive
  boundary, with the block clone occurring at the original callback point;
  copying the dispatch match or postponing delivery until all child rows exist
  would change behavior.

## Implementation dependency and checkpoints

This is a **structural migration**, not a one-attribute patch. Two existing
interfaces block a safe immediate frame type: borrowed `Linked.exists` sites
and `EvalScopeGuard`'s exclusive context borrow. The first implementation
slice must establish safe owned pattern handles/site ordinals and detached
scope restoration while preserving the current recursive callers. Only then
can the one loop move actual LATERAL and FILTER/EXISTS recurrence off the
machine stack. Any design using a raw self-pointer, unsafe lifetime erasure,
copy of the whole evaluator match, or a second VM interpreter is rejected.
The `FnOnce` node boundary and stack-local contextual delivery callback are a
third interface seam: factor their existing entry/exit and delivered-block
operations for use by both recursive and suspended execution before replacing
the converted arms. This must keep one dispatch body and the original early
`RowConsumer::stopped` observation.

Executable qualification sequence, once the mechanism passes material review:

1. Strict affected evaluator/core check and native identity, exact first
   error, lexical/snapshot, nested LATERAL, FILTER/EXISTS, forced
   probe/definition/layered, memo, ledger and evidence controls. Run resident
   and bounded/refusal controls, including a physical allocator failure while
   growing the continuation and release of all admitted bytes on every exit.
2. Existing native async/consumer and native fork controls: delivery stops at
   the original block; no duplicate row, stateful function call, search frame
   or worker witness; unordered fork computation still commits in source
   order. Compare ordinary fixed-work and allocation counts against clean
   source. Preserve original error and truncation precedence.
3. One standard default-directory optimized WASM package from exact source;
   provided bindgen/opt/link, actual SIMD and package checks. Run the **three
   unchanged** original depth assertions (the async-concurrency test's
   synchronous NOT EXISTS lane ≥181, right-nested LATERAL ≥281, and the
   query test's FILTER NOT EXISTS ≥180) and same-package
   shadow-stack byte controls. A typed deeper refusal must leave a healthy
   sibling instance usable. No threshold, stack floor or budget edit.
4. Only after focused qualification: normal hooks, full required `make check`,
   complete Node/package and portable/native acceptance, independent source
   review, current-main extraction and hosted PR/merge. A pass in one of the
   three depth routes is not a #531 pass while another fails.

Material review must decide whether the safe pattern-handle and delivery
states can be implemented without duplicating the evaluator or adding a
default-row allocation that violates the issue's cost contract. This document
does not authorize a build or claim that the loop is implemented.
