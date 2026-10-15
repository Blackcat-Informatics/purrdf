# Preserve authored ordinary rows across sealed layers

The complete contract is layer invariance for explicitly ordinary persisted
rows, including rows whose subject is a reifier in another layer. Default and
named graphs, keyed ordinary reads, reifier rows and annotations must agree with
one combined image. Keep removals, deduplication, source failures, snapshot
ownership and explicit annotation behavior intact. No subagents, timing gates,
new dependencies, duplicate implementations or toolchain changes.

Read the full issue and discussion (none), current source, root .baseline/.goals
and repository AGENTS.md. The existing shared classifier promotes ordinary
rows based on external reifier presence. Both segmented and paged stack adapters
use it; the mutable value writer also uses it for its distinct inferred-input
operation. Native combined-image ordinary typing is the authoritative control.

## Task 1: Reproduce through public persisted reads

Reuse the existing typed fixture builders. Check all three-layer permutations,
the two ordinary layers without declarations, and one combined image in default
and named graphs. Compare ordinary count, keyed subject/predicate/object values,
reifier count and annotation count with the authored rows. Establish an actual
pre-fix failure, not just code-reading evidence.

## Task 2: Correct the existing classification home and its callers

Separate explicit sealed typing from mutable input inference at the existing
classifier; do not add a competing classification algorithm. Both stack adapters
must preserve authored ordinary rows and stop probing irrelevant declarations
for them. Remove obsolete promotion-only paths. Retain annotation demotion,
chronological occurrence/removal semantics and original readiness/fault owners.
Update the obsolete promotion expectation and technical contract accordingly.

## Task 3: Qualify and deliver the complete correction

Run only directly affected stack/mutable and public query controls during work,
plus the new real persisted regression, strict affected compilation and helper
checks. Execute applicable portable behavior through the existing runner. Check
original lazy/bounded/first-fault controls, actual removals/reinsertion and
retained snapshots. Use existing qualifying evidence for unchanged paths.
Publish after normal hooks, resolve material review and hosted final checks,
merge with ghprsq, verify closure/mirror and remove this delivery checkout.
Owner review remains explicitly distinct from independent hosted review.
