# Current acceptance

PASS for integration; delivery actions remain. PR #555 is open at 29f3ae565;
no issue closure is claimed. Complete planned behavior has focused execution
evidence. Material source review completed without actionable correctness
findings. The initial hosted failures exposed three concrete repair classes;
all repairs now pass focused checks and final hosted CI. There are 50 successful
checks, five expected skips and no failures or pending checks. The valid full
local run passed native tests and doctests and reached nested make wasm, then
was stopped as redundant after confirming equivalent completed CI. Its terminal
status is 143, not a local full-suite pass. See tasks/ci-remediation.md and
review-debt.md for the actual historical failures and current dispositions.
The authoritative plan is plan.md. Plan/prior-art presence gates passed;
owner analysis and review are explicitly not independent approvals.

The following table retains the focused qualification history; its former
final-gate dependencies are discharged by the current CI adjudication below.

| Criterion | Focused evidence | Original final dependency |
|---|---|---|
| Resident triple-to-reifier range | Native library filter passed 11 tests including both new controls; original-admission allocation test passed; all 5 graph-free carrier tests passed; corrected strict library/allocation-test compilation passed; actual native helper check passed; evaluator, SHACL and ShEx production consumers compile. Logs are tasks/target-*.log. | Portable runtime and complete issue consumers/scale |
| Components, annotations and actual consumers | Component masks and annotation p/o ranges reach shared statement traversal, both resident forms and Arc. Planner summaries and EXPLAIN support descriptors are wired. Strict affected compilation, admission/partial-retry/release tests, six carrier controls and complete native queries pass. Two shared native/compact portable cases pass natively. | Portable runtime, final review and required gate |
| Backend access and honest EXPLAIN | Resident range descriptors pass. Pack, paged (both views), segmented and stacked scan-default controls PASS: tasks/component-scan-backends-final.log. Positive caller evidence/I/O allowances corrected after original typed fixture exhaustion; production defaults and memory/cache limits unchanged. | Final gate |
| Ten-million/six-annotation population | Actual release execution PASS: 10,000,000 reifiers, 60,000,000 annotations, 30,000,014 terms, 2,567,778,553 payload bytes. Component and annotation complete queries visit two/three candidates and fit fixed 128 fuel; seeded component query passes. See tasks/component-ten-million.log. | Final delivery gate |
| Sublinear fixed work | All 90 executions and 45 positive paired instruction deltas pass with full event coverage. Across 256x population, median instruction ratios are 1.5666 target, 1.0051 components, 1.0018 annotations, below preselected 3x ceiling. See tasks/instruction-method.md and instruction-results.md. Later test-only fixture relocation changes no production/input/query. | None for measured scope |
| Portable execution and strict compilation | Two actual native/compact query/refusal cases pass natively, scalar WASM and SIMD WASM; original host-clock assumption corrected only in public query case. All six portable carrier cases PASS, including every component mask/graph/order control: tasks/component-carrier-portable-scalar.log. Final strict affected library/test Clippy PASS: tasks/component-strict-final-fixed.log. Makefile registers both query cases in scalar/SIMD lanes and the new all-mask carrier case in its scalar lane. | Final gate |
| Projections and documentation | make metadata PASS: tasks/component-metadata.log, including 37 license profiles; generated projections have no diff. Governor profile and backend read-contract manual claims match new access availability, preserved order and non-certificate estimates. | Final source gate |
| RDF 1.2 conformance | Governor corpus 15 pass, one regenerator ignored, unchanged goldens and inputs; profile version 19 documented for narrowed candidate charging/planning. W3C conformance 74 manifest harness cases PASS including RDF 1.2: tasks/component-w3c-conformance.log. | Final single full gate |
| Publication and integration | Complete unit d0f544f49 is committed/pushed with normal hooks. PR #555 is open; plan posted to issue and PR. stagectl creation returned an output-parse error after successful forge creation; resolving the branch found PR #555, so creation was not retried. | Actual source review, hosted checks, ghprsq merge, closure, archive, mirror, cleanup |
| Required full qualification | The invalidated first run remains exit 143, not a pass. Repairs are committed/pushed at 2c13dbe4d with normal hooks. Overflow, all twelve affected EXPLAIN controls, both original trace checks, strict affected compilation and refreshed helper hygiene pass. Nine of 712 trace cases change only in candidate charges and their fuel progression; all answer bytes and other node charges are unchanged. The valid final make check is running with four build jobs, four test threads and a 16 GiB virtual cap; log tasks/final-repaired-check.log. | Terminal final gate, refreshed review and hosted checks, protected integration |

No timing evidence is used. Native jobs use four Cargo jobs and four test
threads with a 16 GiB virtual-memory cap; configured target directories and
installed tools are unchanged. Historical failures and superseded results
remain in their original logs with their scope stated.

Final integration assessment: tasks/final-integration-base-delta.txt is empty;
the fetched origin/main introduces no new commits outside this branch.
git merge-tree --write-tree --no-messages completed successfully; candidate tree
is recorded in tasks/final-integration-candidate.txt. The repaired source is
clean, pushed and materially reviewed. The final two-file test-only delta is
covered by all 30 targeted cases, normal hooks and green hosted checks. Earlier
large-population and fixed-work evidence remains applicable for the reasons in
gap-analysis.md. Zero-subagent owner adjudication is explicitly distinguished
from actual CodeRabbit material source review. No separate reviewer is invented.

## Complete qualification adjudication

PASS: the 50 successful final-head CI checks provide complete equivalent
qualification. ci.yaml explicitly checks Makefile/workflow hygiene parity,
partitions the workspace suite over all seven successful test shards (with the
C ABI smoke covered by its successful job), checks the preserve-order consumer
and core hygiene, and builds exactly the same locked release WASM crate list as
make wasm. The remaining five skips are expected non-required profiling/Pages
lanes. No behavioral coverage unique to the local run remains unverified.
The required final gate is supplied by these completed hosted checks under
Stage's validation evidence-reuse policy; the plan imposes no local-only gate.

The redundant owned local process group 2480202 was stopped after this command
comparison. Its original exec handle returned 143. Its log remains unchanged,
and it is not claimed as a local full-suite pass. No commit hook or unique
required verification was interrupted or bypassed. Source is clean and fully
pushed. No issue requirement is waived or moved. The current material review,
clean integration assessment and retained focused evidence cover the delivery.
Next actions are ghprsq, closure/archive readback, mirror and cleanup.
