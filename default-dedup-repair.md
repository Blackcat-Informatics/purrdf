# Default duplicate owner correction

Source correction is written; compilation, runtime and throughput are NOT RUN
for this correction. It joins the pending complete fusion repair batch.

Observed current bulk DISTINCT unconditionally coalesced duplicate evidence.
The implementation writer guarded coalescing with the existing explicit evidence
input flag. Root then restored row-only ordinary keys: `HashTable<RetainedRow>`
and an adjacent previous row. Tracked keys retain `(RetainedRow, survivor ordinal)`
so alternative witnesses still combine into the original output row.

One generic `DedupState` kernel handles both key types through the private
`DedupKey` trait. Equality and hashing remain binding-only. Workspace admission
computes the real hash-table allocation bound for its selected key type, retains
the old table grant until replacement population completes, and preserves typed
allocation and overflow failures. The generic row loop statically combines
evidence only for tracked keys. Ordinary delivery retains the row-only state.

Formatting and `git diff --check` pass for the two affected source files.
The existing DISTINCT semantics, tracked alternative-witness controls, native
workspace refusals, original fifty-allocation control and final paired default
throughput remain required. This source correction is not a measured speed claim
or completion of the five-contract group.
