# Complete maintenance contract assessment

Owner's implementation assessment. Independent 62-file PR review is captured in
review-feedback.md, including its confirmed seal finding and correction.
This assessment is not independent judgment or a merge verdict.

| Requirement | Production implementation and acceptance |
|---|---|
| Signed native transactions and exact net changes | ChangeBatch retains the original admission/watermark and exact consolidation. Maintained retains asserted multiplicities and publishes original native rows. Public controls cover duplicate/cancelled/empty batches, source errors, withholding and acknowledgement boundaries. |
| Recursive deletion without rebuilding | Reverse chosen supports over-delete affected rows; original native head/body probes find finite alternative grounding. Original withdrawal tokens restore their row coordinates. Final-ground cycles disappear. Current public histories compare every completed batch with cold evaluation; original unrelated row IDs survive. No transaction clones or replaces the whole source store. |
| Stratified negation and unchanged-membership proof repair | Authored dependency impacts schedule absence-dependent work. Native groups/filters use the same kernels. Triple/quad grouped controls and recursive signed histories compare complete facts and canonical chosen derivations against cold evaluation. Independent proof checking re-decides native grouped negation and rejects a complete blocking extension. Caller callback meaning remains an explicit checker refusal. |
| Resumable work, physical admission and partial certainty | Initial capture, supplemental metadata, directory/object indexes, preparation, events, publication, sealing and settlement retain their positions. Original factory refusals consume no accepted factory credit. Work/stop/source/storage tests preserve results, proofs, grants, IDs and watermarks; exact work cuts and each initial factory refusal pass. Source/guard failures poison the original owner. |
| Useful public entry points and actual refutation integration | Public supplied-admission constructors and initialization_work expose initialization controls. Actual current-clash/refutation public cases pass on native and packaged WASM, preserving original scoped source identity and independently verified warrants. |
| Affected-part large-source work | At 1,000 / 1,000,000 edges the same one-fact insertion takes 3 joins/49 allocations/11908 requested/332 retained/3394 peak bytes; deletion 1 join/33 allocations/4768 requested/-56 retained/1758 peak bytes. Explicit hot compaction remains governed: 1,133,504 rows, 99615793 declared work, 9 allocations/46665200 requested and peak/1932 retained bytes, identical across 55 pauses. Four churn blocks retain 0. No universal constant-cost compaction claim. |

Current commands, actual terminal results, reuse limits and failed-run diagnoses
are in validation.md. Full make check exited 0 in its documented pre/post-fix
scopes; changed-path qualification re-executes the corrected edges. The dense-row
and partition/header audit corrections now pass native controls and the actual
public WASM bootstrap control. Current review-correction 467-library/49-runtime
native and 49-runtime/16-refutation WASM runs retain that acceptance.

Normal closure commit 578bb65da, main merge 9a695a072 and correction 55e3b0527
were signed and pushed with ordinary hooks. Paginated feedback is exhausted;
the independent correction review confirms the fix and the sole thread is
resolved. All 51 current hosted checks pass with four configured skips and no
waiver. No in-scope implementation or acceptance finding remains open. ghprsq
integration, archival and cleanup remain required before delivery.
