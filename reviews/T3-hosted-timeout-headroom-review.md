# Hosted timeout headroom: scoped review

Date: 2026-10-13 (America/Edmonton). Scope: unapplied `tasks/hosted-timeout-headroom.patch` and hosted run 38488835253. This is a workflow-budget review, not a #531 completion verdict.

**VERDICT: PASS for applying the bounded timeout correction; hosted acceptance remains open.** The patch changes only the existing `test-shard`, `wasm-package`, and `pytest` job limits from 30 to 45 minutes and explains the observed workload. It leaves the matrices, recipes, compiler and package configuration, assertions, cache steps, and the Worker step's independent 8-minute limit intact. It neither skips a failing test nor changes a numerical acceptance floor.

Evidence for each changed job:

| Job | Actual run 38488835253 | Assessment |
| --- | --- | --- |
| `wasm-package` | `tasks/T2-hosted-wasm-package-cancelled.log` records 449/449 Node tests passing at 00:36:11 UTC, packed tarball smoke passing at 00:36:14, and the identity ABI receipt at 00:42:49, followed immediately by cancellation at the 30-minute job boundary. The two high-ID ABI controls are in that completed identity step. | The separate workerd recipe step had not started. Fifteen additional minutes is a bounded allowance for its unchanged 8-minute step and cleanup; only a complete rerun can establish the job pass. The Node test named “Worker recipe” does not substitute for that separate step. |
| `test-shard` | `tasks/T2-hosted-lib-cancelled.log` shows first-party build consuming nearly the whole budget. Earlier binaries finished, but the evaluator binary began `running 1620 tests` at 00:42:20 and still printed successful cases when cancelled near 00:42:49. | The 30-minute limit cut off real test execution, so 45 minutes is supported. The same matrix-wide timeout applies to all shards, but does not omit any shard. No whole library-shard pass can be claimed. |
| `pytest` | A fresh read-only job-metadata query for job 115608943313 records Python binding tests (maturin + pytest) success at 00:36:36, Pydantic oracle success at 00:42:41, LinkML oracle success at 00:42:46, then cancellation of post-cache restore at 00:42:46; job concluded cancelled at 00:42:49. | All substantive steps completed successfully, yet the 30-minute job limit prevented normal completion. The added 15 minutes is justified for the unchanged post step and variance. The direct job log was unavailable while the overall run was still in progress; this row rests on GitHub step metadata, not an invented pytest log. |

The three cancellations are deadline-limited observations, not hosted passes. Apply the patch through the normal branch/hook flow, rerun the required hosted jobs on the resulting head, and judge their actual terminal outcomes. If 45 minutes still expires, diagnose that measured run; do not mark acceptance complete from this review or weaken the retained tests.
