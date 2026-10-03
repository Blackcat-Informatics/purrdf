# PR 391 final-head bounded review

Verdict: **No new defect or failed check is present in this snapshot. Merge qualification remains pending.** This is a single fresh snapshot, not a final all-green assertion.

## Exact identity and capture

- PR: https://github.com/Blackcat-Informatics/purrdf/pull/391
- Live head: `28744b104af37faeb43a25bf865c25fa7094bc4d`; base `main`; PR OPEN, non-draft; GitHub merge state `UNSTABLE`; no submitted review decision.
- REST records captured 2026-10-03 11:31:42.390685 UTC; complete GraphQL threads captured 11:31:42.940455 UTC. All list endpoints were paginated; thread comments were fully retrieved.
- Raw records, full completed-job logs, retrieval commands/errors, machine-readable summary and SHA-256 manifest: `/tmp/purrdf-391-final-head-live/`.
- One initial log request per completed job was refused by gh's terminal-escape output policy. The recorded retry uses `--allow-escape-sequences`, succeeds, and preserves each full raw log. No job status was refreshed for that retry.

## Current-head hosted checks

46 check runs: **12 success, 1 neutral CodeQL, 33 in progress, zero failed/cancelled/timed-out/action-required**. Three current-head workflow runs (CI 37119631972, Docs 37119631979, PR security 37119630127) are all in progress. Downstream aggregate jobs not yet created cannot be credited.

Completed successes: conformance (sparql), conformance (shapes), windows-mtime, python-publisher, avx512-sde, doc, wasm, msrv, Analyze (actions), Analyze (python), Analyze (javascript-typescript), Analyze (c-cpp).

All seven assembly shards are pending in the captured record; none has a finished outcome or final log available in this snapshot:

| Shard | Status | Conclusion | Job ID |
| --- | --- | --- | --- |
| simd-asm-config (x86_64) | in_progress | pending | 111193073548 |
| simd-asm-config (wasm32) | in_progress | pending | 111193073495 |
| simd-asm-config (wasm32-simd128) | in_progress | pending | 111193073475 |
| simd-asm-config (x86_64-v4) | in_progress | pending | 111193073450 |
| simd-asm-config (aarch64-neoverse-v1) | in_progress | pending | 111193073444 |
| simd-asm-config (x86_64-v3) | in_progress | pending | 111193073431 |
| simd-asm-config (aarch64) | in_progress | pending | 111193073392 |

The earlier missing-benchmark-row failures belong to older heads and are not failures of this head. The parent's stable local writer and separate final-source report passed at 103 sites / seven configurations; this bounded review does not substitute that local result for pending hosted shard results. Root's complete current-head local gate runner also remains pending here.

## Actual current-head compiler identities

Full logs of already-completed current-head jobs—not workflow labels—establish:

| Surface | rustc | Full compiler commit | LLVM | Evidence |
| --- | --- | --- | --- | --- |
| Nightly SPARQL conformance + wasm | 1.101.0-nightly, 2026-10-02 | `0abfedbc7cd4e725f126913880c95800394f7c37` | 23.1.1 | jobs 111193073489 and 111193073331 |
| MSRV | 1.98.1, 2026-09-01 | `48a229ceaefd4985c50990b14116b6d856af0985` | 22.1.8 | job 111193073305; explicit `RUSTUP_TOOLCHAIN=1.98` and version assertion |

The captured SPARQL log reports its seven-suite shard GREEN; wasm and MSRV check conclusions are success. The local historical measurement compiler was 1.100.0-nightly (`4b6d04e70`, 2026-09-13), so equal LLVM 23.1.1 alone cannot establish hosted assembly count parity. Actual hosted ASM compiler/count qualification remains pending.

## Complete review/comment/thread state

11 issue comments, three formal reviews, three inline comments, one thread. Every formal review is historical (928426 or aa529); zero formal reviews are submitted against current head. The sole thread `PRRT_kwDOTKq-Ms6omkGE` is resolved and outdated; it records the Markdown pipe fix in aa529 and CodeRabbit's confirmation. **Zero unresolved threads**.

Current-head CodeRabbit commit status is pending, “Review in progress.” Run `aae12513-b068-40ef-96ef-93a4b37f6a9a` explicitly reviews 7f137→28744 and selects only the SIMD document and manifest. Its top-level comment still contains the completed previous run `8ef933a2-92ab-44b4-811c-bd71ae0682cd` and its “no actionable comments” result for aa529→7f137; that result is not current-head approval.

The comment retains the older aggregate Docstring Coverage advisory (69.74% versus 80%, 304 functions / 42 files / two unsupported), without a missing-function inventory. It is not a failed current commit status. The independent public-contract remediation and immutable measurement provenance are recorded in `/tmp/purrdf-391-documentation-provenance-review.md`; the current-head author's assembly repair comment accurately states that complete hosted/local qualification is still required. No new actionable review comment is present in the complete snapshot.

## Standing completion constraints

The current `.deficiencies` is notice/marker only, SHA-256 `091e58efd8bd3d2ca55dc1a6211527f37a67d3870c0c53c8e82b021b14b6fdf5`. No emergency entry authorizes an exception or incomplete work. No build, tracked-file edit, Git fetch/mutation, metadata rewrite or GitHub post was performed for this review. Earlier measurement/capture provenance and next-issue review artifacts were preserved.

Acceptance remains contingent on the parent's final current-source local gates, all required hosted jobs/aggregates, and completed current-head CodeRabbit review with any real findings resolved. This artifact deliberately does not infer those outcomes from elapsed time, older heads, or pending states. No further polling was done.
