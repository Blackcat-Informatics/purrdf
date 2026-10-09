# PR507 focused gap and completion delta review

VERDICT: PASS

Current local changed-behavior/completion verdict is PASS. The valid ordinal finding is corrected by the three-line removal and demonstrated by the meaningful delta-origin regression, full affected core/view controls and actual native/WASM production Update consumers. Current final strict/fmt and owning regression exits are0. This does not claim normal publication, fresh hosted CI or final integration complete. No additional implementation omission or scope cut was found. This is the proportional Stage2 combined gap/completion review; prior complete T1/T2/T3 judgments are reused for unaffected criteria rather than repeated.

Inputs: PR507, branch paudley/401-update-transfers-preserve-rdf-1-2, pre-change base aab23cbf2, published20e2e637c plus current one-file mutable.rs repair; full issue/plan, remediation-plan.md, validation.md, tasks/T1-review.md/T2-review.md/T3-review.md and T4-ordinal-finding-review.md. Completion-adversary/no-deferrals requirements apply without cuts. Reviewer performed no builds or shipping/index/ref/forge writes.

## Changed mechanism and meaningful controls

The removed three lines unconditionally lowered a classification target's ordinal even when insert_record_rows found it independently present. The existing home already supplies the source ordinal when it genuinely creates a target. Removing the duplicate overwrite therefore preserves independently authored target provenance while leaving conversion-owned ordinals and undo metadata intact. Membership, suppression, generated ownership, logical metrics, graph live counts and LOAD minting are unchanged.

The new empty-base regression exercises a real delta ordinary origin, an interleaved independently inserted annotation and an independent annotation equal to the source. It verifies original independent target ordinal during classification, exact annotation stream before/during/after, complete insertion-order restoration and all original ordinals after declaration undo, plus uniqueness of live ordinals. It retains the original base-origin regression and frozen shared-view probe golden. The before-state annotation stream is explicitly r,z because the earlier ordinary r primes its native term identity; comparing this actual stream is the correct immutable ordering contract, not a weakening of authored physical ordinal preservation.

Historical T4-ordinal-before exits101 at ordinal0 versus expected2, genuinely proving the reported production defect. T4-core-after exits101 after the ordinal assertion passes, at the erroneous z,r public stream fixture. The corrected fixture now checks the actual sorted native stream consistently. Both failures remain retained and neither is relabelled PASS. Actual T4-core-settled1232, shared_views59, native Update10 and real wasm32 Update10 all PASS with exits0. Initial T4-focused-controller101 is retained: strict rejected a redundant clone in the new test. Removing only that clone changed no production code; T4-regression-final1 PASS, strict-final0, fmt-final0 and lint-retry-controller0 establish the final source.

## Complete contract applicability

| Whole401 criterion | Current applicability and missing evidence | Judgment |
|---|---|---|
| Typed role membership, reversible ordinary classification, counters/lifetime/retained views | T1 full core and lifetime evidence remains applicable except the repaired independent-target ordering transition. Fresh full core and unchanged shared-view golden controls must close that affected gap. | DEMONSTRATED: core1232 and shared_views59 PASS |
| ADD/COPY/MOVE exact roles both graph modes | No transfer caller/typed membership algorithm changed. T2 real public-engine exact role, dedup, self/missing/empty and governed controls remain applicable. Fresh portable native10 provides current changed-core consumer confirmation. | DEMONSTRATED: qualifying reuse plus current native10/wasm10 PASS |
| Per-document LOAD fresh bare/nested/CDT identity, counter/prefix/collision and error handling | No map/mint/CDT/selected-export/resolver/publication code changed. Complete T2 and T3 actual public engine evidence remains applicable; fresh native10/wasm10 exercises the same production path against repaired core. | DEMONSTRATED: qualifying reuse plus current native10/wasm10 PASS |
| Governor physical attempts, cancellation/SILENT and public Arc atomicity | Ordinal-only correction adds no mutation/charge or new publication branch. T2 affected663 and T3 settled workspace remain qualifying for unchanged behavior. | DEMONSTRATED by qualifying reuse |
| Additive public API | Only private mutation behavior and cfg-test regression change. T3 core/eval196 checks each PASS against original base remain applicable; no new export/signature or mode default change. | DEMONSTRATED by qualifying reuse |
| Required complete native/WASM local qualification | T3 actual full retry38424 and nested all-release make wasm passed on20e2e637c, original failed setup/correction retained. Affected one-file core/views/native and actual WASM consumer/strict/fmt follow-up is appropriate; no new blanket full suite is required solely for this bounded change. | DEMONSTRATED: earlier full qualification plus current affected closure |
| Actual PR feedback/remediation | Sole captured CodeRabbit finding is valid, now corrected in source. Retain OPEN until actual owning qualification, normal publication and factual thread disposition. | Locally DISCHARGED; publication/thread disposition pending |
| Current hosted CI, fresh candidate, archive/ghprsq and cleanup | Prior-head green checks cannot cure the finding or qualify a newly published repair. Fresh head/feedback/candidate evidence and normal protected integration remain required Task4 work. | PENDING delivery gates |

No dark/test-only replacement, refusal-as-capability, duplicate engine, vacuous output, weakened original ceiling/golden or follow-up laundering was found. The test's exact owned ordinal and undo observations are distinct from the public native sorted annotation stream. Separate471 default adaptation remains the authorized next delivery, not missing401 work.

## Actual closure and remaining delivery

Read actual selected tasks/T4-core-settled.log/.exit (1232 PASS), T4-shared-views.log/.exit (59 PASS, original golden unchanged), T4-native-update.log/.exit and T4-wasm-update.log/.exit (ten each, real existing WASM runner, zero failed/ignored/filtered), and final T4-regression-final/strict-final/fmt-final/lint-retry-controller outputs/exits (all0). The final regression executes one owning case with1231 library cases deliberately filtered; it is not claimed as a repeated full core run. The preceding whole1232 and all consumers remain applicable through this test-only clone removal.

The final diff is one mutable.rs path: production removal of the redundant ordinal overwrite and a63-line cfg-test regression. No other shipping behavior changes. All required whole401 implementation criteria retain the qualifying earlier T1/T2/T3 demonstrations and the changed ordering criteria now have actual affected closure. No new blanket suite or benchmark was executed by this reviewer.

Normal hooks/commit/push, refreshed complete review/check surface, fresh hosted CI and candidate/ghprsq/archive/cleanup remain Task4 delivery conditions. Local PASS authorizes neither a claim that whole401 is merged nor a claim that the entire portfolio is complete.
