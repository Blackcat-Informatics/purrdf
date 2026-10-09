# PR507 ordering remediation

The sole current review finding is valid: classification overwrites the insertion ordinal of an independently authored annotation when its source is a delta ordinary record. Undo then leaves duplicate ordinals. The base-origin regression did not exercise this branch.

Use the lightweight Stage 2 path for this bounded three-line correction within the independently reviewed classification design. Retain the complete T1/T2/T3 contract assessment and actual execution evidence; recheck affected ordering, mutable ownership and production UPDATE paths, with one independent reviewer covering the changed behavior and evidence reuse. Escalate if those checks expose wider effects.

1. Add a delta-origin regression with an interleaved annotation, preserving target order before/during/after classification and checking ordinal uniqueness after undo. The failing-first run is recorded in tasks/T4-ordinal-before.log and .exit (101, expected ordinal 2, actual 0).
2. Remove the unconditional ordinal minimum rewrite. insert_record_rows already assigns the supplied ordinal when it creates the target; an independent existing target retains its own ordinal.
3. Run core library and immutable shared-view controls, public native UPDATE typed-record cases, their WASM runtime counterpart, strict affected lint and formatting. Reuse the earlier full qualification for unchanged contracts, explicitly retaining its original tested tree and setup-failure/retry history. Fresh hosted CI must qualify the updated PR head.
4. Independent reviewer adjudicates the delta and current index, then normal hooks/commit/push publish this one coherent fix. Post actual evidence to issue and PR and resolve the underlying review thread after publication.
5. Refresh current CI, complete feedback and merge-tree assessment. Finalize and integrate only through ghprsq, then clean up this delivery and proceed with remembered-graph defaults.

No contract is cut or deferred. Local regression repair is not hosted CI or completed integration.
