# Explicit evaluation coordinates

Deliver the entire coordinate contract: caller-selected query time and entropy,
owned historical PagedStack snapshots, and summary-pruned annotation value range
scans. Keep defaults unchanged and preserve exact native values, RDF 1.2 layers,
source generation checks, sticky faults and resource evidence. No prefix-only
or clock-only PR closes the issue. Use existing homes and no new dependency.

## Task 1: Historical sealed prefixes

Extend the current snapshot construction home with an explicit sealed-layer
count. Public snapshot_at(count) retains the initial bases plus the first count
sealed delta generations, excluding the mutable head and later layers. Zero is
the base snapshot; sealed_delta_count() reports the latest position. Refuse a
count beyond available delta depth with its exact
operands. Pin/check only selected source descriptors. Preserve removals, empty
graph declarations, physical RDF layer typing and original-page routing through
the existing provider. Test five generations against eager prefix oracles,
including removal-only layers, reinsertion, unsealed head exclusion, old snapshot
ownership, selected-source drift, excluded-source drift and G8 page evidence.

## Task 2: Query and rule time/entropy inputs

Read the actual QueryOptions, EvalCtx and engine invocation homes before editing.
Thread explicit time and seed through root, nested, prepared, governed, worker
and rule callers; record selected coordinates in evaluation identity. Preserve
normal live-clock behavior without options. Handle pre-epoch host time at the
existing temporal home rather than silently substituting 1970. Tests execute
NOW/RAND/UUID through actual native entry points, compare repeated same-coordinate
answers and changed-coordinate answers, and cover applicable shipped hosts.

## Task 3: Annotation range execution

Reuse the landed predicate/object indexes and summary interfaces. Admit typed
ordered bounds, compile compatible annotation-object FILTER ranges into the real
scan, and retain a sound post-filter for residual expressions. Reject blocks by
sealed summaries before page admission. Compare results with post-filter plans
and execute the required ten-million-claim fixed-work/page-evidence control,
including boundaries, outliers, unbound cases and historical snapshots. No timing
or throughput measurement.

## Task 4: Integrate and deliver

Focused checks accompany coherent API slices. One owned Cargo command runs at
once, using standard host tools/directories and existing resource caps. Final
whole-contract qualification, hosted material review and CI precede ghprsq merge,
closure verification, mirror readback and branch cleanup. No subagents or invented
independent reviews. Keep REWORK_LIST.md current. Preserve the active text gate;
prioritize actionable text-candidate feedback and delivery.

## Prior art and current observations

The Stage brief finds no earlier delivery branch, commit or PR for this issue.
Linked storage/index/statistics foundations are closed and provide actual homes.
PagedStack builds all sealed sources in snapshot_with_head; its ordered
visibility machinery already accepts an introduction prefix internally. Extend
that construction path instead of adding another stack or query implementation.
The referenced Katamari ADR is external and not a local governing PurRDF ADR;
its relevant coordinate requirements are quoted in the issue. Owner assessment
only; independent implementation judgment will come from hosted material review.
