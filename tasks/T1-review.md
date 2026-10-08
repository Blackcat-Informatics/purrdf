<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Task 1 review

VERDICT: PASS

Reviewed the settled uncommitted Task 1 diff against `6273b6173`, the authoritative
plan and its PASS review, the issue analysis, applicable AGENTS.md, and main's
untracked `.baseline` and `.goals`. Reviewed `T1-implementation.md` and the updated
validation index. Scope is the four Datalog source files: plan, cache, seminaive
and schedule. No source edits, builds, forge actions or commits were performed by
this reviewer. An independent `git diff --check` passed.

## Criteria and findings

| Task 1 criterion | Review result |
| --- | --- |
| Ordinary connectivity before static selectivity | PASS. The first priority is a bound **variable** in any of the four carrier positions. Constants contribute only to later selectivity fields. Same-class and different-class type regressions force the connecting edge before the second type scan. Predicate and graph tests explicitly defeat a disconnected atom with more constants. |
| Deterministic ties and authored coordinates | PASS. Known, constant, repeated-variable and unique reverse authored-position fields retain deterministic ties. Binding slots still come from authored traversal. Existing constant/ground lowering and new repeated-variable coverage remain present. |
| Hybrid connectivity and cyclic certification | PASS. Groups use the same binding-aware loop. Taking the maximum member key implements ANY-member connectivity, including a later member when the first is disconnected. Cycle certification and its internal variable descent are unchanged. The cycle regression verifies authored atom coordinates and slots. |
| Actual lowering and authored premise restoration | PASS. Operators and both restoration maps are constructed from the reordered group's complete authored-coordinate sequence. Production binary and hybrid joins apply their respective restoration maps. The analytic runtime oracle checks exact six-premise proofs and facts for planned and forced-binary kernels across eight insertion permutations, rather than only comparing two implementations. |
| Guard stages and caller context | PASS. Production guard scheduling is unchanged. The new runtime regression exercises ordinary and certified-cyclic plans through both public stratified and scheduled entry points, verifies two reached rows, stage-major callbacks, identical row order between stages, and the original caller thread. |
| Versioned changed observations, stable authored hashes | PASS. Both planner and calculus versions change to v2. Documentation correctly states the facts/derivations/budget-report condition for a physical-only exemption. Exactly five contract/plan pins change to observed v2 outputs; both guard-free and guarded authored-clause pins remain unchanged. |
| Constraints and implementation scope | PASS. No defaults, ceilings, dependencies, features, certificate rules, shared helper homes, or unrelated files change. The generic ordering loop shares the concrete ordinary/group selection job without introducing a separate production path. |

No required findings are open. In particular, the hybrid lowering now follows its
physical group order consistently; it does not preserve the old operator order
while moving only the group executor. Shared default-graph or predicate constants
cannot accidentally connect otherwise independent variable components.

## Validation basis and limits

The implementation report and parent-provided check results record the settled
library suite as **314 passed, 0 failed, 0 ignored**, including planner, frozen
identity, exact proof and guard regressions. Package all-target clippy with
`-D warnings`, workspace formatting, and whitespace checks passed. Earlier
compilation/formatting failures and deliberately obsolete identity pins were
resolved; they are not counted as passing runs. The final pin diff matches the
five documented values and leaves both authored pins intact.

No redundant concurrent build was run. This verdict accepts Task 1's assigned
focused verification and source behavior; it does not claim a completed delivery,
performance qualification, full gate, wasm runtime, or hosted CI. Independent
factorization, global witness frontiers, canonical confirmations, shared admission,
real default-limit CLI scaling and consumer qualification remain Tasks 2–6 in the
approved plan. They are not prerequisite implementations for the Task 1 commit.
Normal hooks, signing, push and progress publication remain parent-owned steps.
