# PR 391 final compliance and hosted-state review

**ACCEPT the reviewed compliance and documentation provenance. Final assembly/current-head qualification remains pending; this is not merge approval.**

Read-only audit on 2026-10-03 of https://github.com/Blackcat-Informatics/purrdf/pull/391 . Pushed and local HEAD both identify `7f137d3e9c3b02961ab2ee368a7ea171c7a282a0`; base is `97769c0d95026d95211768f0c30ce0c70c7309b3`. The PR is open, non-draft, and reports `UNSTABLE`. No builds, fetches, tracked edits, local Git mutation or GitHub mutation were performed. Complete REST pages, GraphQL threads and full failed-job logs are retained in `/tmp/purrdf-391-hosted-audit-7f137`.

## Current-head checks

The latest check-run snapshot at **2026-10-03 11:12:16 UTC** has 49 created check runs: 28 success, 11 in progress, eight failures, one skipped and one neutral. The neutral result is the CodeQL app check; its individual analysis jobs are separate. The CodeRabbit commit-status context has since completed successfully; its older pending event remains historical status data.

All eight failures belong to the known missing benchmark-table row on current CI run **37118188444**:

| Check | Job |
|---|---:|
| simd-asm-config (x86_64) | 111188972863 |
| simd-asm-config (x86_64-v3) | 111188972869 |
| simd-asm-config (x86_64-v4) | 111188972883 |
| simd-asm-config (aarch64) | 111188972881 |
| simd-asm-config (aarch64-neoverse-v1) | 111188972906 |
| simd-asm-config (wasm32) | 111188972858 |
| simd-asm-config (wasm32-simd128) | 111188972862 |
| simd-asm aggregator | 111189113325 |

Every configuration's complete fetched log contains:

```text
FAIL: bench `crates/sparql-algebra/benches/scope_checks.rs` has no row in the bench table
make: *** [Makefile:678: simd-asm] Error 1
```

Each configuration exits 2 before compilation. The aggregator correctly exits 1 at `test "$MATRIX_RESULT" = success`. No failing check beyond this common coverage gap was found. Per-job raw log hashes and exact diagnostics are in `failure-log-summary.json`.

Pending checks are Rust CodeQL analysis, C ABI, two integration shards, library tests, pytest, wasm packaging, cross-architecture aggregation and three RISC-V suites/determinism jobs. They have not been credited as passes. Other completed outcomes include core/shapes/SPARQL/Python conformance, release-crate wasm, wasm execution/oracle jobs, MSRV, docs, Miri and relevant architecture checks as recorded by the snapshot.

Older-head results are separated explicitly:

| Head | CI run | Snapshot check outcomes |
|---|---:|---|
| `92842627b840a755a3fca31a78d8aa274a34fa4b` | 37115388645 | 41 success, one skipped, eight known SIMD failures |
| `aa529ee697baf39aaea2148db25bc03dfff66e5e` | 37116724641 | 41 success, one skipped, eight known SIMD failures |
| `7f137d3e9c3b02961ab2ee368a7ea171c7a282a0` | 37118188444 | 28 success, one skipped, one neutral, eight known SIMD failures, 11 in progress |

Older successful checks and the four full local gates on `92842627b` do not establish final current-head hosted qualification.

## Complete review/feedback state

All three submitted reviews, all three inline comments, all ten issue comments and the single complete review thread were retrieved, with pagination. The GraphQL thread inventory was refreshed after current-head review completion: **one resolved/outdated thread, three comments, zero unresolved threads**. The original CodeRabbit pipe-formatting finding on `92842627b` has the author's fix reply and CodeRabbit's confirmation that commit `aa529ee` addresses it.

CodeRabbit completed the incremental `aa529ee` to `7f137d3e` review at 11:11:43 UTC, run `8ef933a2-92ab-44b4-811c-bd71ae0682cd`. The current bot comment explicitly reports **no actionable comments**, minimal merge risk through `7f137`, and no established merge-blocking implementation risk. The successful current commit status agrees. The three formal review records remain `COMMENTED` records on earlier heads; no current approval record is fabricated from the bot's summary/status.

The bot still reports a **Docstring Coverage advisory: 69.74% versus 80%, 304 functions across 42 files, two unsupported**. This is current advisory text, not a stale prior percentage or a failed GitHub commit status. It supplies no missing-function inventory. Root's public-API audit found and repaired concrete prefix, carrier and deep-work-list contracts without suppressing any gate; the pushed comment records meaningful documentation for fourteen new callable APIs, six types and the scope module, 59 focused controls and warning-free Rust documentation. No unresolved inline/API defect was established by this audit. An advisory metric is not represented as an 80% pass.

## Compliance and preserved provenance

The governing `.goals` and `.baseline` were read from the top-level checkout because they are untracked and absent in the issue worktree. The worktree has no `CONSTITUTION.md`. Standing requirements remain Rust-first, deterministic, portable, single-home, no semantic optionality, full required scope, no process references in repository documentation and uncompromised hooks.

The local and committed `.deficiencies` files are identical and contain only the notice and marker, with no entries below it. SHA-256: `091e58efd8bd3d2ca55dc1a6211527f37a67d3870c0c53c8e82b021b14b6fdf5`.

The committed delta from `92842627b` to `7f137d3e` has four Rust comment edits and three documentation/evidence files. Removing only `///` and `//!` lines leaves all four Rust line streams exactly identical to the qualified snapshot. Current Rust bytes equal committed HEAD; there is no pending executable drift. This correction changes no dependency, feature, public signature, corpus input, generated artifact or executable body.

Both evidence JSON objects become exactly equal to their originals after removing only the new `environment.post_capture_documentation` object. Every original sample, environment, compiler, source hash, capture head, working-diff record and measurement remains intact. Exactly one of ten prototype source hashes drifts (`scope.rs`); exactly two of nineteen current runtime hashes drift (`eval.rs`, `remote.rs`). Every annotation matches both its preserved captured hash and actual current documented hash; all other records still match. Thirty 100-sample prototype payloads and the three paired ten-sample runtime payloads are preserved. The report explicitly avoids a replacement timing or isolated causal/cross-hardware/wasm speed claim. The eight-pending-node inline/heap-spill boundary accurately describes `WorkList<_, 8>`.

All seven changed repository documentation files were scanned for development issue/PR references and incomplete-work markers, with no matches. No required implementation is deferred under the typed unavailable finite prototype boundary. Official/upstream and GTS preservation and the explicitly versioned first-party governor-profile transition remain as previously independently accepted; the later documentation commits introduce no further corpus changes. `git diff --check` passes.

## Required finalization still in progress

Only `docs/design/purrdf-simd.md` and `scripts/simd-asm-manifest.toml` remain dirty, belonging to the authorized measured coverage repair. Root's stable writer is underway. The earlier writer's source-identity-change refusal and individual symbol/floor observations are not credited as a passed combined assembly gate. This review makes no instruction-count/compiler-parity claim and does not alter those files.

The honest coverage repair must finish generation and final-source verification, pass normal hooks, commit/push, and receive its own complete current-head hosted checks and review audit. All still-running checks must reach their required outcomes; new review findings must be assessed. Stage 3 must retain the latest-base synchronization, deficiency/deferral checks and required `ghprsq` merge path. These are pending workflow gates, not authorized omissions or completed work.

Supporting read-only artifacts remain unchanged:

* `/tmp/purrdf-391-documentation-provenance-review.md`: `fa08ec088f190675e76dee665f53d7badbfe7e22eafe421c51df877600505247`.
* `/tmp/purrdf-387-simd-asm-ci-review.md`: `1d7a5fa633579a828ac016ac35d325d09afa365509552d66bd491411853f15c7`.
* `/tmp/purrdf-384-compliance-review.md`: `fb8385d3c5578d9555f30f88b45441c4c9dd096dcbe82e22fd33d000cd133278`.
