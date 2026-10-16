# Native predicate summaries: owner review

This is owner review, not an independent approval or whole-issue completion.
The unit builds sealed native and compact-carrier metadata through the existing
freeze Memory. It retains predicate row counts, exact subject/object domains,
class frequencies, selected object frequencies, object kind/datatype counts and
reifier target-predicate statistics. Unknown high-cardinality selected frequencies
remain absent. Class counts are retained independently of that selection bound.

Distinct domains call the existing pack varint/delta reader and writer. Temporary
ordinal arrays and retained tables use original admission, with scope rollback
before refund. A single-graph relation shares domain ranges between the any-graph
and specific-graph summaries. Native-to-compact transfer preserves the metadata
and original local IDs. Compact metadata reads check the original graph/error
owner before returning counts. Query-output retention includes the new sidecars.
Bound structural components retain their existing range estimates rather than
being widened to a marginal predicate count.

Executed evidence:

- Native and compact graph/stream/domain parity: two tests pass.
- Original-admission growth sweep: partial refusals refund to the retained
  dictionary baseline; successful retained bytes equal the actual sidecar layouts;
  last-owner destruction and refund return the account to zero. Pass.
- Existing zero-allocation binary measures all metadata and distinct-domain reads
  with zero allocations, requested bytes or retained growth. Pass.
- Original RDF 1.2 freeze/index owner test passes.
- Actual supplied-statistics BGP order/100-answer control passes.
- Both actual native/compact statement query/refusal controls pass after wiring.
- Strict affected core/evaluator libraries and tests pass with warnings denied.

The capacity census retains exactly forty existing proof contracts. Only the
builder/dataset authority identities, constructor closure and moved sink
coordinates need refreshing. The constructor source set grows from 158 to 160
because the new metadata modules are in the audited closure. Constructor inputs
remain validated original IDs. Domain reconstruction propagates typed malformed
metadata errors; no narrowing panic, unchecked constructor, widened visibility,
new proof, exemption or changed checker is introduced. A registration edit was
prepared with unchanged classes, sinks, reasons, symbols and authority paths.
The first edit command failed before writing; the resulting stale-registration
check failed as expected. The corrected patch is applied and the actual helper
recheck passes all 82 enforced jobs, 118 exact distinct groups and all 2275
source paths. No new duplicate body, source exemption or checker change is added.
No failed check is described as passing.

Remaining whole-issue work includes paged/segment metadata and exact translated
domain unions, full distinct-aware join selection/replay, rule ordering, EXPLAIN
evidence and all five complete acceptance demonstrations. No partial PR is opened.
