# Native declarations: current bounded verification

The two handle-pinning bodies now have one private declaration in their original
pipeline home. Each still invokes its carrier's original graph digest, verifies
the supplied digest, and only then publishes the payload. Both public signatures
and documentation are retained. Core all-target strict7241 passed; independent
source review passes in ../reviews/pipeline-pin-sharing-independent.md.

Unchanged helpers-hygiene23454 finished with exit2. Its original self-tests pass
and the handle-pinning group is no longer reported. Three other groups still
failed: segmented reservation, native nesting certificate and quoted capabilities.
This is a failed whole census, not a gate pass.

The operation and session reservation bodies now share a private declaration
while calling their distinct original ready methods before the unchanged admission
home. Session snapshot/length validation and sticky first-error behavior remain.
Core all-target strict55554 passed in1m50; independent source review passes in
../reviews/segmented-reservation-sharing-independent.md.

The two native nesting-bound overrides now use one crate-private declaration in
dataset_view. The original bound, flat-carrier docs and inline attribute remain;
unbounded host defaults and other view certificates are untouched. Core all-target
strict57680 passed in52.79 seconds. Independent source review passes in
../reviews/native-nesting-certificate-sharing-independent.md.

Existing segmented runtime controls72208 finished with exit0:13/13 PASS on the
reservation and original native-declaration source. Census17501 then failed:
the nesting macro hid the direct item required by the unchanged frozen-nesting-bound
ledger home. That entire macro slice was reverted; its earlier strict/source
review is historical and does not qualify the revised implementation.

The original direct native method is intact. SegmentedStackQuery now combines
actual session/head/removal/declaration certificates through existing
widest_nesting_bound, with no I/O, allocation or term traversal. This is
O(descriptor/carrier count), not constant time. Both quoted-only profiles now use
one fixed_store_capabilities declaration at the existing store home; all dynamic
profiles remain unchanged. Revised independent source review passes in
../reviews/composed-nesting-fixed-capabilities-independent.md.

Initial revised strict96426 failed one redundant closure; it was replaced with
the existing trait method reference. Corrected core/evaluator all-target strict74916
finished with exit0 in2m51. Final unchanged helpers-hygiene67323 also finished with
exit0:81 enforced jobs,118 distinct rows and128 hash domains. All six original
duplicate groups are absent, and the required direct native home resolves. No
ledger or checker edit was needed. Final native runtime8841 returned terminal0:
segmented_stack14/14, shared_views59/59 and host_identity3/3. The actual controls
include header-only opens, snapshot/purge ownership, chronological graph state,
pins/refusal and unchanged host IDs through joins. Receipt: final-sharing-native.log.
Normal hook14246 and push95040 passed, publishing clean checkpoint5ec569c32.
Full S1 acceptance,
enforced capacity-panic census, portable qualification and publication remain open.
No checker, ledger, tooling, target directory, compiler, hook
invocation or acceptance rule was changed.
