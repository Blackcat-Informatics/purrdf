<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 child-entry/resume ownership seam — 2026-10-13

This is concrete source guidance for an **executable** FILTER/EXISTS child driver, not a claim that one exists or that optimized floors pass. Current `FilterRowFrame::run` moves its row/link/cursor into an admitted pending frame but still invokes recursive `resolve(Suspend::Exists)`; `exists_prepared` then invokes recursive `eval_correlated`, which calls `eval_evaluated`. Merely moving those calls into another function does not remove the recurring Rust frames.

## One owned prepared site across all three entries

`expr.rs::PreparedSite<'s>` presently borrows both `PreparedExists` and its plan map. A suspended parent cannot retain that borrow when the site came from a local preparation. Use a by-value site owner in the existing `expr.rs` home: `key: usize`, `prepared: SharedWorkspace<PreparedExists>`, `plan_map: SharedWorkspace<SubstitutionSourceMap>`, plus an optional stable original source owner for the layered case. Pattern fields are borrowed only temporarily from this owner while making a child-entry decision; no frame stores references into another field of itself.

The three exact caller adaptations are:

1. `exists_recorded`: `ctx.prepared_exists(pattern)` already returns `SharedWorkspace<PreparedExists>`. Clone its `ledger_source` shared map from the Pattern variant before moving the preparation into the owned site; preserve `key = address` and FoldedFalse return.
2. `exists_deferred`, Empty: `slot.site.prepared` and `slot.site.plan_map` are already shared owners. Clone those controls, preserve the current `normalized` address key, and retain the enclosing deferred slot as needed until child preparation has taken its site/plan map. Merged and Layered remain distinct paths.
3. `exists_layered`: `copy: Option<CorrelatedPattern>` and `body` are local. Make the completed copy an admitted `PreparedPattern` **before** building `PreparedExists` from it; wrap the newly built preparation in `SharedWorkspace` and retain the source owner in the pending site/frame. Keep `key` tied to that stable body address. Dropping the local `copy` before a resumed child would otherwise invalidate the ledger-source address correspondence even if the normalized preparation were separately owned.

`PreparedExists::Pattern` itself already owns normalized/witness patterns, analysis and nested sites. An owned site needs no second preparation algorithm and no parallel `EXISTS` evaluator.

## The context and child interval

`SubstitutionRunner` currently contains `&mut EvalCtx` plus admitted `SubstitutedFrame`s. The driver must have **one** mutable context owner: either the runner itself owns the complete loop and splits its context/frame borrows within methods, or its admitted copied-scope frame stack moves into the driver and each push/pop accepts `&mut EvalCtx`. Do not embed a second `&mut EvalCtx` in a pending frame and do not retain a `&GraphPattern` into a movable vector; `PreparedPattern` keeps the tree at a stable shared allocation and paths identify child nodes.

At the definition path in `exists_prepared`, perform the current collision, strategy, memo-hit and first-witness decisions once. `begin_correlated_map_scope` now provides the exact admitted `ScopeRestore::CorrelatedMap` token; carry it on the parent continuation instead of a borrowed `EvalScopeGuard`. Produce a child-enter request (selected normalized/witness pattern, nested-site source, row and schema owner, optional memo key, ledger token) rather than invoking `eval_correlated`. The shared correlated substitution path prepares one `PreparedPattern` plus placeholders/map and pushes one `SubstitutedFrame` before the child enters the existing node dispatcher. `NodeBoundary::enter/start` runs once for that child. On its result, `NodeBoundary::finish` runs first, the copied substitution frame restores its scope/active owner next, the correlated map token restores next, and `ExistsDefinitionResume::finish` performs the original barrier, answered/consumed charges, evidence and memo writes last. That order matches the current nested lexical call chain.

On an error, run the same child boundary cleanup and LIFO scope restoration before returning the original error; preserve `NodeBoundary::finish`'s read-error precedence. On unwind, a driver-owned cleanup path must leave node cursors, pop search frames and clear local Slice caps, then restore substitution and correlated-map tokens before releasing copied AST owners. `NodeBoundary` currently has no `Drop` cleanup, so source movement alone is insufficient. For `RowDelivery`, keep the existing outer `RowConsumer`/`RowDelivery` owner with the driver and store only mode/state in pending node frames; call `eval_delivered_node` through the existing dispatch rather than copying its loop or retaining a borrowed consumer per frame. Native fork remains on its current distinct branch until the sequential driver is qualified.

The structural obstacle is therefore finite and concrete: local `PreparedSite`/layered source ownership plus one context owner and LIFO boundary cleanup. No evidence shows an unavoidable borrow or semantic impossibility. The actual child-enter/result-resume driver and wrapper recurrence are still unimplemented; any path that calls recursive `resolve`, `eval_correlated` or `eval_evaluated` for the same nested child remains unqualified for the #531 floors.
