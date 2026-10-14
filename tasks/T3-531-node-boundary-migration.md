<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 node-boundary state for the evaluator worklist

Status: the shared `NodeBoundary` entry/completion state is implemented but
uncommitted atop `330c55f8c`. It is still used through recursive
`eval_evaluated_with`; no explicit loop or stack-floor result exists.

The existing before/after code now lives in one `NodeBoundary` state. The
state retains booleans for local Slice cap and search evidence and optional
`NodeCursors` from `enter_node`. It retains neither `&mut EvalCtx` nor a
reference into an owned copied tree. The future runner frame must retain its
own pattern owner/path and consume this boundary when its child returns.

Entry order from current `eval.rs` lines 3619–3778 is:

1. Raw bounded-dataset admission and stack-floor check.
2. Local Slice-cap installation, source-read check, evidence-search push.
3. If governed: pre-operator stop, node-cursor entry, `AlgebraNodeEntry`
   charge. A stop or charge failure returns a truncated empty syntactic bag;
   a charge failure leaves the node cursor before creating that bag.
4. Dispatch exactly once, with existing active `RowDelivery` caller.

On normal child completion, re-enter the parent node cursor, commit its output,
apply the post-operator stop, then leave the cursor. On evaluator error, leave
the cursor without committing. In either case, pop the search frame, clear the
local Slice cap, check the source read error (which can replace the returned
error), then combine outcome search evidence into the rows and parent frame
only when the result is `Ok`. This order is observable: moving read-error
before the pop, or evidence combination before cap clearing, changes the
original first-error/owner behavior.

An entry failure before the search frame is pushed needs cleanup of a locally
installed Slice cap while preserving its original error. A stop/schema failure
after search push must run the same outer completion path as an operator error.
On Rust unwind the worklist runner must discard pending node cursors/search
frames and LIFO `ScopeRestore` tokens before copied AST owners; the current
recursive closure has no such movable state, so a new boundary cannot be
considered complete until this runner Drop path exists.

The boundary refactor alone is not an optimized-stack correction. It adds no
default-row allocation or new dispatch body. Current affected evidence:

- `cargo check --locked -j 4 -p purrdf-sparql-eval --lib`: PASS, session 3950.
- `cargo clippy --locked -j 4 -p purrdf-sparql-eval --lib -- -D warnings`:
  PASS after removing one redundant conversion, session 37999. The first
  strict run's one Clippy diagnostic is retained in conversation, not a pass.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib eval::tests:: --
  --test-threads=4`: 69/69 PASS, session 5850. This includes evidence,
  correlated-copy and local-ceiling controls.
- The existing `expr.rs` `FaultView` now latches a source refusal before a
  semantic Slice entry. The production `eval_evaluated` call returns an error
  and releases its newly installed local cap: directed test 1/1 PASS, session
  38746. The first lib-test compile exposed one unnecessary qualification;
  corrected before this terminal result.
- After extracting the VM's existing start/advance/resume phases, the same
  evaluator lib strict check passed and `vm::tests::` passed 14/14 on the
  final compiled test binary. These phases still run synchronously inside
  `Linked::term`; they establish no suspended worklist execution.
- The saved VM program counter is now `TermCursor`: warning-denied evaluator
  library Clippy and the focused VM 14/14 tests passed on that exact source
  in session 54893. This verifies the primitive only.
- The next source slice connects the actual copied substitution
  runner to `PreparedPattern`, retains that owner during FILTER linking,
  records an admitted root-to-expression path with the existing algebra walk
  and one child ordinal update per entered node,
  and resolves the existing VM EXISTS ordinal through that owner. The ordinary
  borrowed/no-EXISTS link remains its existing path. The sequential FILTER
  caller now retains a `FilterRowState` cursor while the same VM interpreter
  resolves suspensions. Directed copied-owner and runner tests are written.
  Warning-denied library Clippy passed and the two directed
  `copied_filter_exists` tests passed 2/2 on this source in session 77397.
  The first compiler retry required an explicit `Vec<usize>` type and the
  next strict retry required a short path type alias and immutable `resume`
  receiver; both failed results preceded the terminal pass. Broader runtime,
  cost and stack-floor qualification remains pending; the current child
  evaluation is still synchronous and does not remove recursion.
- Independent source review:
  `reviews/T3-531-vm-boundary-current-source-review.md`, scoped PASS for
  corrected VM site ownership and the shared boundary order; and
  `reviews/T3-531-copied-filter-owner-source-review.md`, scoped PASS for this
  copied-owner/FilterRowState source. Neither review qualifies depth.

The future runner must consume pending cursor/search/cap state and restore
`ScopeRestore` tokens LIFO before copied AST owners on error or unwind. The
three original optimized floor assertions remain wholly open.

Current uncompiled continuation source (after session 77397): the sequential
FILTER now retains an actual `FilterRowFrame` in an admitted pending list, but
its child still calls recursive `resolve`. The EXISTS definition completion
body has moved into `ExistsDefinitionResume`, and the copied runner can push
another admitted scope frame. These are preparatory, not a nonrecursive
execution path. `PreparedSite` now owns its admitted shared preparation and
source map; layered deferral also retains its copied body so the memo key stays
valid across suspension. The definition ledger layer uses a detachable
`ScopeRestore::CorrelatedMap` token on the existing recursive path. Finally,
`DetachedLift` in the original lift home retains pre-child admission without
borrowing a movable copied node; actual FILTER, Slice and application child
intervals use detach/reattach. These changes have only formatting/diff checks
so far. The settled shared Rust gate owns the compilation lane; no depth,
allocation or current-source runtime result is claimed from these edits.

Later source-only work makes the VM row yield its actual `Exists(frame, site)`
step and resume through the same interpreter; the recursive adapter still
consumes that step synchronously. The uncached probe's post-child indexing,
cache insert and answer now share `ExistsProbeResume::finish`, with its ledger
scope restored before completion/error. H's scoped source read found no
ordering or allocation change in this extraction; see
`reviews/T3-531-first-witness-streaming-driver.md` for the necessary streaming
parent-frame flow. There is still no executable nonrecursive child dispatcher,
so none of these changes qualifies a floor. Compiler feedback follows below.

Focused current-source compiler checkpoint, 2026-10-13: direct standard-dir
`CARGO_BUILD_JOBS=1 cargo check --locked -p purrdf-sparql-eval --lib` returned
exit 0 (`Finished dev profile`, 8.99s). The initial retry exposed three
borrowed `ledger_source` callers and a stale native `Linked::filter` adapter;
the next retry exposed two unused prospective worklist accessors under
warning denial. Those are corrected/removed. This proves production library
typing only; no current-source evaluator runtime, streaming delivery, or
optimized depth floor was run. The nonrecursive child dispatcher is still
unimplemented.
