<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Governor foundation requirements

Scope is the combined accepted plan for issues 478 and 469, tasks 1 and 2. The initial source is main dfc0c21ad in the assigned isolated issue worktree. The complete captured bodies have no comments. Publication, commits and final independent review belong to the parent workflow; this document is an implementation analysis, not an independent verdict.

## Requirements and acceptance

| Required behavior | Production surface | Qualification |
|---|---|---|
| Ordered blocks only with fuel/scratch headroom | parallel::par_loop_try_map_init and row/item checkpoint callers | Stop-only and ungoverned controls; ordered limited-work controls |
| Useful minimum/worker-derived block length | parallel::par_blocks_try_map_init | Source-order, error/harvest, small/large/thread-count neighbours; 16,384-row A/B at 4 and 32 threads |
| Exact trips, consumption, deterministic answers and bounded in-flight work | RowCheckpoint, ItemLedger and GovernorState ordered commit | Existing/new focused governor, scratch, cancellation and numeric determinism tests; unchanged profile vectors |
| Honest OPTIONAL production claim | CLI metered governor decoder and actual OPTIONAL filter caller | Real CLI fuel OPTIONAL witness, cell-accounting coverage, corrected documentation or executed fork |
| Metered overhead at most 1.2 times ungoverned | numeric_eval_group through native query execution | Testkit bench fuel/scratch/metered/ungoverned lanes at 4 and 32 threads, measured against unchanged main |
| Performance account includes actual conditions | docs/BENCHMARKS.md and Stage validation | Baseline/candidate results, compiler/profile, thread and lane conditions; no unsupported general speed claim |

## Constraints and initial observations

Read the assigned plan, validation index, issue captures, Stage 1/stagectl instructions and required quality/validation/delegation/no-deferrals references, tracked AGENTS, root-only .baseline and .goals, and the empty deficiency ledger. No child AGENTS or governing ADR/constitution was found in the affected crate/docs search; intake names no ADR. No semantic features, new dependencies, alternate metering engine, frozen vendor-vector edits, broad make check/wasm, hook bypass, sibling mutation, Java/Gitee or model actions are permitted.

Initial inspection confirms every governed parallel loop selects blocks, including stop-signal-only execution. The block length is row_count/(workers*64), with minimum 1: 16,384 rows at 32 threads produce 2,048 blocks of 8. Each block initializes and harvests a context. The row_checkpoint module retains the retired prefix-stop account. Metered charging still requires direct profiling; the baseline claims alone do not establish its current bottleneck or the candidate outcome.

The detached qual-454-task3-main-baseline is clean at the approved main base. Baseline builds and bench outputs use task-owned target/log paths so no sibling source or artifacts are modified. All source work remains in the assigned issue worktree.
