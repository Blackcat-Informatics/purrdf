# Caller-selected compaction source review

Current disposition: PASS for the inspected source and scoped native boundary,
scale, original core/graph, facade and executable-documentation runtime results
recorded below, plus the original core/umbrella portable compilation. Whole-delivery acceptance remains pending. Earlier
pending statements below describe their review point before those receipts.

The current two-file diff moves the existing compaction body into `freeze_with_limits` and makes `freeze` delegate with the original default. The only changed compaction operation is selecting `snapshot_view_with_limits(limits)`. Import, ordinary-row location remapping, builder freeze and successful-work accounting remain single-copy and unchanged. The parameter reaches the existing snapshot retention admission directly, without a new policy owner or weakened builder checks. The new executable documentation accurately distinguishes view retention ceilings from RDF validity/capacity.

The existing `complete_source` and `surface` helpers supply scoped blanks, nested quoted triples, directional text and all three physical row roles. The new control alters an annotation, adds an ordinary row, retains a snapshot, and checks one-short refusal for all five coordinates against measured exact limits. Each refusal checks the original diagnostic code and dimension plus unchanged successful-work counters, then retries through the same owner at the inclusive boundary. Default delegation is compared with explicit defaults and the retained snapshot; later mutation does not alter the snapshot's original row count. Existing graph/location controls remain applicable because their production body is unchanged and the default wrapper now traverses the new method.

No source defect found in this bounded diff. The named-graph assertion in the new control is a count, so exact empty-graph identity continues to depend on the existing graph regression controls; it is not independently proven by that count. Retain those planned controls. The approximately 950k-row run must report actual auxiliary bytes and default/high-limit outcomes rather than presume row count alone reproduces the original 512 MiB refusal. No runtime result was inferred from the active root check.

## Explicit scale acceptance update

Reviewed the later `large_mutable_claims_freeze_with_the_snapshot_limits` test and revised plan. Its shape is explicitly 950,000 claims, four RDF rows per claim: an ordinary row, one reifier and two annotations. It uses existing constructors and actual MutableDataset insertion, snapshot and freeze APIs. The ignored designation keeps this large native acceptance an explicitly scheduled run rather than repeating it on every ordinary test invocation; it does not excuse running it before completion.

The test prints measured rows/auxiliary bytes, asserts the actual auxiliary charge exceeds the default ceiling, then requires the original default method to refuse with the expected retention cause and the same owner to compact at the selected larger ceiling. Complete physical table counts and first/interior/last identities qualify the large shape without two additional whole-corpus BTreeSet copies. Full value parity is separately exercised by the small fixture. This is a bounded resource choice, not a claim that sample identities alone prove full value parity. Scope source PASS; actual large-run admission, default refusal and successful compaction remain unrun at this review. If the asserted footprint is not reached or exceeds the selected larger ceiling, diagnose the measured shape; no result may be inferred from claim count alone.

## Actual native runtime readback

Read the final `large-freeze.log`, `core-regressions.log`, and current two-file source. The explicit ignored-target invocation actually ran and passed one test in 58.66 seconds: 950,000 claims, 3,800,000 retained rows, 577,600,360 auxiliary bytes. This is above the default 536,870,912-byte ceiling and below the selected 1,073,741,824-byte ceiling. The executed source asserts the default typed retention refusal and successful same-owner explicit-limit compaction, all three table populations and original boundary/interior identities. The previously pending scale criterion is now PASS, with no inferred footprint or waived assertion.

The original core batch also actually passed 1,303 unit controls, 60 shared-view controls, 11 graph-existence controls and six declaration-drain controls. The scale test was ignored only in that routine batch and then explicitly executed as recorded above. These results qualify the scoped native boundary, compaction and retention behavior, including existing location/graph regressions. This is independent receipt/source review, not another test run. Strict Clippy, rustdoc, portable/public compilation, the complete required gate and hosted/integration acceptance remain pending; this review does not declare the entire issue complete.

## Umbrella consumer source

The later `base_surface.rs` addition imports MutableDataset and ViewLimits only
through the public umbrella, reuses parsed_under_base/text unchanged, and invokes
the new method before comparing serialized N-Triples with its parsed source.
It proves the intended public API is available to an ordinary facade consumer
and preserves resolved RDF on compaction when executed. It does not introduce a
new constructor/serializer body or claim to prove high-limit refusal itself;
the core boundary and actual scale controls own that proof. Scoped source PASS,
with the actual facade runtime still pending at review time.

Subsequent actual receipt review: facade-freeze.log ends with the named public
consumer test1/1 PASS on the settled three-file source. freeze-doctest.log also
records the executable freeze_with_limits documentation1/1 PASS. The facade
availability/resolved-RDF and documented-call examples are therefore now
runtime-qualified. These focused results do not replace the still-running
portable check or mandatory full gate.

Portable receipt now read: portable-freeze.log completes the original core and
umbrella target check successfully in4m21. This qualifies compilation of the
actual additive API on wasm32; it is not a portable runtime claim. The mandatory
full gate is still running and whole completion remains unqualified.
