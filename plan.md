# Fallible canonicalization through actual callers

Behavior: return original native canonicalization refusals through every affected
public identity/comparison caller, using one fallible implementation.
Dependencies: existing canonicalizer, DatasetView, digest/error protocols and
actual Rust/host callers; no external repository changes or new external package.
Exclusions: configurable search limits, UDF frames, Datalog defaults and recursive
SHACL; their parent requirements remain open and ordered in the campaign.
Acceptance: compile affected production callers and changed test/bench callers;
execute the named native refusal/healthy/conformance paths and exact JS host
exception case; normal hooks, required hosted checks/review, ghprsq and cleanup.
Risk: shared public signatures, content identities and host errors; preserve
successful bytes, original refusal ordering/causes and final host presentation.

This complete v4 behavior removes the parallel trusted/panicking public family
and CanonState::run. Keep the existing try_canonicalize homes as the sole
fallible path. Update actual production signatures and native error chains,
re-exports, tests, examples and documentation. No compatibility shims or staged
deprecation. Do not alter successful bytes, labels, hash choice or refusal order.

## Task 1: Remove input-driven panic routing

Replace the existing trusted-panic unit control with typed refusal coverage for
all applicable remaining dataset/view/graph/digest entry points. Reuse original
poison and healthy-neighbor fixtures. Remove infallible exports and the run
method; production callers propagate original CanonError values, including
content/proof identity consumers. Result unwrapping belongs only in controlled
test/example fixtures, never as replacement production error handling.

## Task 2: Qualify actual consumers

Compile real callers early after a coherent API change. Exercise native RDFC,
profile, source/view and certificate/proof/host paths affected by the migration.
Run strict affected caller checks and required gates; release WASM builds are
separate from ordinary native semantics. Reuse valid unchanged qualification
with its exact scope. Capture complete typed input failure and healthy-neighbor
parity before publication. Independent hosted review, all required CI, normal
signed hooks and ghprsq integration/archival/cleanup remain mandatory.

Parent 489 retains configurable/removable RDFC and dataset-state search budgets,
exact configuration/work identity, heap-framed UDF execution and explicit Datalog
logical defaults. No original requirement is dropped or represented as completed.

Whole-workspace runs are not scheduled for completion or publication. Use the
actual affected crates, concrete tests/binaries/consumers and required profiles;
mandatory hooks run fully and are not manually duplicated. Scope is frozen at
the behaviors above; stop discretionary test growth once these are covered.

Active work begins 2026-10-24 16:56 UTC, four-hour budget with a two-hour scope
assessment. Hosted waits excluded. One owned Cargo at a time. Preserve sibling
work and no external implementation/agents. Execution state stays in the queue.
