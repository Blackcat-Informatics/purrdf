# PR 391 final-head review: 11:38 bounded snapshot

**No new defects or failures. Hosted assembly evidence and current-head CodeRabbit review pass; remaining local/hosted gates are still required.**

Exact live head: `28744b104af37faeb43a25bf865c25fa7094bc4d`. PR OPEN / non-draft / base main / GitHub merge state `UNSTABLE`. REST snapshot began **2026-10-03 11:38:49.766481 UTC**; complete paginated thread capture and finished-job/report evidence followed once. Raw bodies, full logs, report ZIPs, extracted JSON, commands, timestamps and hash manifest are preserved in `/tmp/purrdf-391-final-head-live-1138`. There was no status refresh or polling after the initial snapshot.

## Checks at initial capture

**48 current-head checks: 33 success, one skipped (Pages deployment), one neutral (CodeQL), 13 in progress, zero failed/cancelled/timed-out/action-required.** Complete conformance aggregation is success. CI/security workflow runs remain in progress; Docs workflow is complete success. The full current-head local gate is parent-owned and still running; no older-head result is substituted.

Pending checks: cross-arch, cross-arch-riscv64 (determinism-one-worker), simd-asm-config (x86_64-v4), simd-asm-config (aarch64-neoverse-v1), cross-arch-riscv64 (determinism-serial), cross-arch-riscv64 (suites), pytest, simd-asm-config (aarch64), test (integration-2), wasm-package, cross-arch-riscv64 (determinism-more-workers), capi, Analyze (rust). The SIMD aggregate had not yet appeared and is not credited.

| ASM configuration | Captured check conclusion | Uploaded report |
| --- | --- | --- |
| x86_64 | success | passed / 103 |
| wasm32 | success | passed / 103 |
| wasm32-simd128 | success | passed / 103 |
| x86_64-v4 | pending | passed / 103 |
| aarch64-neoverse-v1 | pending | passed / 103 |
| x86_64-v3 | success | passed / 103 |
| aarch64 | pending | passed / 103 |

All seven current-head uploaded report artifacts were available during the single evidence retrieval, including the three whose check statuses were still pending at initial capture. All seven full job logs contain `OK: 103 site(s) measured on 1 configurations (partial configuration shard)`; all seven JSON reports have `status: passed`, 103 sites each and **721 total site cells**. The pending statuses are retained as pending here despite their later successful gate output. No subsequent check-state request was made.

## Compiler, source and exact count parity

Every ASM report and full job log identifies **rustc 1.101.0-nightly (2026-10-02)**, full compiler commit `0abfedbc7cd4e725f126913880c95800394f7c37`, host x86_64-unknown-linux-gnu, **LLVM 23.1.1**. This establishes actual hosted compilation and does not rely on the older local compiler label.

All seven reports share source identity `7c8e3f2c7050157e7c5adfa388c8a4ec88dcdddebbd853b3aaa61c997be519bc` and manifest identity `7561af7b6739d6cb9274ef66d49dfec48b16426774ceb1a42f264d2981d12ae0`. Each artifact's workflow metadata binds it to exact head `28744b104af37faeb43a25bf865c25fa7094bc4d`. Current local manifest bytes match that manifest identity.

The existing pure `doc_checks` was applied to the complete combined hosted payload (no compilation/cache writes): **zero parity or coverage problems across all 721 cells**. Thus current hosted counts match the committed document exactly, with no inferred compiler parity. `sparql.scope-admission` is `v0·f0·r0` on all seven configurations; no vector-speedup claim is implied. Current document SHA-256 `1a5553527e791e894653467d33cc80165e3b31844bee8c5a2bf617d7fe802383`. Report payload hashes:

| Report | SHA-256 |
| --- | --- |
| aarch64-neoverse-v1 | `ec79090953475230bfe596c0a3674072e199ad1995b24d2ba3a463250c812a12` |
| aarch64 | `c1dfebb3d984ad9bf0396b4cd7d10e72e94bb8279d47624bf92332ef1b55a0b1` |
| wasm32-simd128 | `2d86593f86177fd55af6423f927fd75144c2d860eaf4526baac556671540d517` |
| wasm32 | `53e769cc67def6975e575e37326efdc471d58b004335eb7aeebdce10634814db` |
| x86_64-v3 | `a6bce53487503f9278ca4e6a91a0af083e0cb416b4f96a27149e3cdbe979a821` |
| x86_64-v4 | `b46bf4d92e94d6125345b2c350efb274bb3696eda6409269c0e9cac4b092e755` |
| x86_64 | `c3c48418f8da2dbe4f0f46c448eb5bd3af848c334be0b2a05f78fc1905e02867` |

## Complete review/comment/thread state

All **11 general comments, 3 formal reviews, 3 inline comments and one resolved/outdated three-comment thread** were captured with full bodies. **Zero unresolved threads**, no additional review findings or new formal reviews. Historical formal reviews remain COMMENTED against 928426/aa529; there is no formal approval review on current head.

Current-head CodeRabbit commit status is **success / Review completed at 11:38:19 UTC**. Updated comment 5968108401 at 11:38:16 explicitly completes run `aae12513-b068-40ef-96ef-93a4b37f6a9a` over **7f137→28744**, selecting the SIMD document and manifest, with **no actionable comments**. This is current-head completion, unlike the historical result in the prior 11:31 snapshot. The sole earlier Markdown pipe finding is resolved with bot confirmation. The older aggregate docstring advisory remains visible; it has no new concrete missing-function inventory and is not a failed current-head commit status. Meaningful public contract fixes and immutable measurement provenance were independently audited previously.

## Qualification boundary

No source edits, builds, Git fetch/mutations, metadata edits or GitHub posts occurred. The prior snapshot and all other audit artifacts were preserved. This review accepts the completed CodeRabbit and seven-config report/count evidence, **contingent on final current-head local gates and every required hosted check/aggregate completing successfully**. No pending check is counted as passed; no further polling was performed.
