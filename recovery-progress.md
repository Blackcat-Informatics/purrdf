<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Recovery progress

## Current acceptance

The user has resumed the approved recovery and selected instruction and allocation counting for performance qualification. Elapsed-time receipts are historical measurements. Current reports use retired instructions, allocation calls, requested bytes, retained bytes and peak working bytes. There is no machine reservation.

The authoritative plan is plan.md. Its current measurement contract selects ten fixed-work samples, existing testkit median/MAD/95-percent bootstrap statistics and one-percent comparison rule, complete production-result guards, caller/worker coverage and separate instruction/allocation passes. The numeric instruction-overhead criterion is metered/plain at most 1.20 at four and 32 workers. Row controls compare main, the normal-chunk control and the candidate at one, four and 32 workers. Bounded work, exact accounting, functional behavior, host execution, normal hooks and protected integration remain required.

Performance counts, final qualification, commit, PR and merge are pending. Current CLI/Wasm refresh passes. No governor completion is claimed.

## Active source

Worktree: /home/paudley/Active/purrdf/.worktrees/478-sparql-governor-governed-parallel-row.
Branch: paudley/478-sparql-governor-governed-parallel-row.
Integration baseline: dfc0c21adabe557e5d11f027aaf2576bddbe1dd6.
Signed parser prerequisite f35709b0e is committed and pushed with normal hooks.
Governor implementation remains uncommitted. Repaired source diff fe1c8329f01cd4624d7c17b8d7a978dfee3ab7f9 is independently reviewed.

The implementation uses bounded ordered blocks only for caller fuel/scratch ceilings, preserves aggregate reduction geometry and exact source-order replay, combines worker-local numeric charge recording, and omits empty inherited scratch lookup layers. The common cell-row ceiling checks allocation layout representability, preserving reachable finite cell bounds, observations and forecast refusals. Parallel output reservation and concatenation return typed allocation failures and preserve first-source-error ordering.

FILTER and filtered OPTIONAL use typed identity admission methods. Their no-replay paths retain the existing scheduler result buffer. Governed replay delegates to the existing implementation; BIND and group minted-value conversions retain checked allocation. The direct ownership witness checks pointer, spare capacity, all ordered bindings, resume and every governor counter.

## Verified functional checks

- Seven repaired row-checkpoint cases pass, including identity ownership, charge/refusal boundaries, saturation, once-only stop work and the derived fork/continuation witness.
- All 57 repaired production cases pass: one isolated allocation case, 26 governed queries, nine correctness cases, eleven numeric-governance cases and ten numeric-determinism cases.
- Shipping and intended-profile lint, formatting/diff checks and helper hygiene pass. Helper census reports 81 enforced jobs, 23 variants, 91 distinct rows and no open copies or cross-crate includes across 1,867 files.
- The physical witness preserves the independently derived allocation ceiling and separates fork work from retained-prefix continuation. The forced case records fork 9/9, prefix one, retained 65,607 bytes = charged zero + pending 65,607, and continuation 4/5, with exact sequential outcome/accounting and unique source visits.
- Earlier common-layout corpus checks pass fifteen cases and one byte-identity corpus case; the existing generator is ignored. Earlier CLI tests pass 25 cases. The earlier release Wasm library and two Node runtime cases pass with digest da3900a93723c4ac and corpus length 2,727. These retain their captured historical identity; current host checks are below.

Current host refresh passes the release Wasm library, both numeric Node cases with the same frozen digest, and all 25 CLI governor cases. Normal release CLI artifact 4b7dd5184f24dd371b1f37e933b436c9a4394b81 returns identical verified 200,000-row plain/fuel/inclusive-cell answers and the exact 199,999-row refusal prefix. Explain counters remain unchanged. host-refresh.md records source/artifact identity, the corrected explicit-format invocation and all receipts.

Shared count reader/protocol tests pass five cases. Both existing schema-1 documented-text golden and bit-for-bit round-trip cases pass. The focused count targets compile and lint, and both normal-allocator instruction and counting-allocator allocation binaries build under the benchmark profile. reviews/count-code-review.md records scoped PASS for the protocol/record/store chunk; executable qualification is detailed below.

The initial instruction witness executes all 37 precreated worker indices across one, four and 32 workers, with exact enabled/running equality. A later review found its aggregate instruction floor can also be satisfied by non-target scheduler work. Coverage acceptance is therefore held open while the witness parks non-target callbacks and measures enable/work/disable on the selected worker. Raw matrices remain preserved. Allocation worker coverage passes at those three pool sizes.

The candidate numeric METERED/plain median instruction ratios are 0.9370318917708115 at four workers and 0.5053358621663481 at 32. Independent inspection verifies all 80 production/empty records, exact raw totals, full typed guards and identical metered evidence across pool sizes. Baseline numeric plain changes are -0.0438% and -0.1867%, both WithinNoise. At 32 workers, row stop/fuel lanes improve 11–21% versus main and meet the normal-chunk rule. Plain BIND changes +1.124959% with 95-percent interval +0.665749% to +1.548266%, WithinNoise under the existing one-percent rule. These are measured instruction observations; final qualification also requires the strengthened witness and allocation repairs.

The full allocation matrix finds owned regressions: plain BIND requests 655,360 extra bytes at one/four/32 workers; some numeric scratch/metered cases increase requested bytes, and numeric fuel at 32 increases peak bytes by 2.928 MB (39.06%). The source writer owns diagnosis and repair without changing typed allocation failures or physical-work bounds. raw/count-allocation-comparisons.tsv preserves all 240 comparisons. No combined completion is claimed.

Baseline and normal-chunk control source projections are archived directly from dfc0c21ad. Their parser source remains the original blob; candidate separately includes committed parser prerequisite f35709b0e. Dev tooling overlays and the control's selector delta are audited independently from shipping source. Baseline/control benchmark builds pass. These distinctions must remain in final source provenance.

Current receipt paths: raw/identity-row-checkpoint-tests.log, raw/identity-production-governor-tests.log, raw/identity-evaluator-shipping-clippy.log, raw/identity-evaluator-test-clippy.log and raw/identity-shared-helper-hygiene.log. validation.md is the acceptance index; task reports and reviews retain the detailed checks.

## Earlier CLI proof

The earlier normal release CLI returns identical verified 200,000-row plain/fuel/inclusive-600,000-cell answers. The 599,999-cell neighbor exits 3 with the exact 199,999-row certain prefix. Explain remains fuel 1,400,005 / rows 200,000 / cells 600,000 / scratch 76. Actual OPTIONAL predicate execution is present on non-caller worker threads.

CLI artifact reconciliation is recorded in raw/cli-artifact-reconciliation.md. Functional calls used pristine normal blob e3178ec8c6f027736ebad7b5e34d508bedb5c8b9. Profiling used the preserved rewritten symbolized blob 9ba23f4c73742d53e59e6a338f9a93bca2276dff; independently verified code/data sections match the pristine executable. The repaired source needs current host evidence before completion.

## Interrupted measurement record

The task-owned benchmark and monitoring processes were stopped at the user's instruction. Cleanup confirmed no task benchmark or monitor remained; sibling processes, hooks and Brainstem were unchanged. Raw timing samples and exit status 143 remain recorded. Those samples are not current count-based acceptance. The user's continuation authorizes the instruction/allocation work above.

## Preservation and remaining recovery

Original source/index/worktree layers, untracked files, merge metadata, ref/worktree/stash inventories and a verified complete repository bundle are preserved under /home/paudley/Active/purrdf-recovery-artifacts/20261006-2253. Historical Stage data and task-owned experimental source projections are separately preserved there. Original protected main and sibling worktrees remain intact.

Stopped RDFLib work is now clean at pushed merge cee1c41b2faa370b7daa60cc7536e784a4756d0e, with parents 56e636497 and dfc0c21ad. rdflib-current.bundle verifies its prerequisites against the preserved complete-history bundle. This preserves the branch without qualifying its acceptance.

The approved order continues after the governor foundation: retained XPath/large-count forms; shared conformance kit/vocabulary; one BigInt engine and exact-only v4/default/temporal-seconds migration; SPARQL modifiers; stopped RDFLib and Python iteration; SHACL-only work; certified math, computational geodesy/grid and evaluator/host wiring; transfers and remembered graphs; datatype/OWL verdict repairs; Full schema and input-derived caps; deep Turtle/GTS and async Wasm; residual portable corpus, selected measurement and Rust comparison/replay; Rust LUBM. Unique advanced arithmetic, Datalog capacity and regular-role-chain work retain owners. Independent SHA-2, LargeRDFBench and translation work remain separate except necessary shared tooling boundaries. Release publication is separate.

xpath-preparation.md, numeric-preparation.md and recovery-coverage.md record extraction boundaries and all thirty issue mappings. Each activation requires current complete forge intake. Normal hooks cannot be bypassed; every final PR/branch merge uses /home/paudley/stage/root/bin/ghprsq. No full discretionary workspace qualification has run yet.
