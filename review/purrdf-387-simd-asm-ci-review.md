# SIMD assembly CI diagnosis and repair review

Reviewed 2026-10-03. Worktree: `/home/paudley/Active/purrdf/.worktrees/387-blank-scope-investigation`. Branch HEAD: `92842627b840a755a3fca31a78d8aa274a34fa4b`; PR #391. Hosted run: https://github.com/Blackcat-Informatics/purrdf/actions/runs/37115388645 . CI checkout was merge commit `3890cd6dcfacee59c99123dbe9e5cc7548adecf2`, combining this head with main `97769c0d95026d95211768f0c30ce0c70c7309b3`.

**ACCEPT the narrow repair design and static validation. Assembly counts and hosted acceptance remain subject to the root's actual seven-configuration generation and verification; no measurement pass is inferred here.**

## Exact failure and provenance

All seven configuration jobs stop before compilation with the same diagnostic:

```text
FAIL: bench `crates/sparql-algebra/benches/scope_checks.rs` has no row in the bench table
make: *** [Makefile:678: simd-asm] Error 1
```

Their step exits with code 2. Full x86_64 and aarch64 logs were read; the remaining five completed job logs were fetched and inspected for the common error, exit status and content identity. The completed-job logs were read through `gh api --allow-escape-sequences repos/Blackcat-Informatics/purrdf/actions/jobs/<id>/logs`; the run-level CLI refused job-log retrieval while other run jobs remained active.

| Configuration | Job | Failure time (UTC, 2026-10-03) | Raw fetched log SHA-256 |
|---|---:|---|---|
| x86_64 | 111181114306 | 10:08:35.7893712 | e59cfd8d0076eb8d3db1f4919360c441f0e02e0092b0ef7d713bdb0569b842cd |
| x86_64-v3 | 111181114268 | 10:08:37.6160928 | 3286d348605d7c17c4b1bda915ea3ab048e727d7545f791d11c0cde80c6602b4 |
| x86_64-v4 | 111181114243 | 10:08:39.2479528 | 0ca5df6b0f86ecac78cfb3409ad2d0fe963fcb39897a1480e55f24316d31c2df |
| aarch64 | 111181114247 | 10:08:47.1031531 | 4182b0f4eaed5487f719bfc6b4bf76e3786dbae95669b0b483a65abe6b04a30c |
| aarch64-neoverse-v1 | 111181114318 | 10:08:48.4038181 | 1f87184404bb67133cf9f2a1ca9f3ca408e66df6ecb802fc1ad636cdcfed05a6 |
| wasm32 | 111181114135 | 10:08:49.1643738 | 66542fb626d3fdaa32e41827a7082e3b5f79cd2a1b41ea57b43bbee2426a8ba4 |
| wasm32-simd128 | 111181114319 | 10:08:52.1293361 | 65b5e8fb54cba30c472a63a5e6c2f7cec7530d2aa791729bd53f21e9b7adf0d0 |

The aggregator job `111181236772` correctly refuses an unsuccessful configuration matrix. No assembly reports were produced by these failed configuration steps.

The actual workflow is `.github/workflows/ci.yaml`: the seven jobs invoke `make simd-asm SIMD_ASM_ARGS="--config <configuration> --jobs 1 --report target/asm-reports/<configuration>.json"`. Makefile line 678 invokes `scripts/check-simd-asm.py --doc`. The driver calls document preflight with `compare=False` before compilation/report generation; document coverage requires every workspace bench to map to at least one valid site. Empty or unrelated mappings cannot establish coverage. Cross-toolchain installation succeeded; the failures provide no evidence of compiler, linker, target or SIMD-instruction defects.

## Honest complete repair

Only `scripts/simd-asm-manifest.toml` and descriptive rows in `docs/design/purrdf-simd.md` were changed. New site `sparql.scope-admission` measures the shipping symbol `purrdf_sparql_algebra::scope::validate_pattern` for every configuration, with `min_vector_ops=0`, `max_fma=0` and `forbid_relaxed=true`. Its leave rationale is an irregular borrowed typed-node walk, without an independent fixed-width lane kernel.

This is the actual timed baseline: `scope_checks::existing_check` calls candidate support `existing`, then `Query::validate_hidden_variables`, `scope::validate_query`, and `scope::validate_pattern`. The benchmark constructs raw algebra inputs; mapping it to lexer parsing or fixed hashing would misrepresent its timed work. Dev-only declaration/contract candidates are separately timed experiments, not shipping assembly kernels. Whole-symbol instruction counts will describe compiler-generated incidental vector work, not a claimed SIMD speedup.

The document now maps the bench to this site, describes its timed call chain, and lists it in algebra member coverage. All 102 previous manifest site records compare identically, including thresholds, symbols and configurations; build graph data is unchanged. The current total is 103 sites. Existing production algorithms, CI/gate code, ten candidate evidence sources, nineteen runtime measured sources, corpora and dependencies were not edited by this repair.

Prepared source identities, before root regeneration:

```text
7561af7b6739d6cb9274ef66d49dfec48b16426774ceb1a42f264d2981d12ae0  scripts/simd-asm-manifest.toml
34b5150323a87e9a9e229b7c0f317794aafe99269ec955087272fc871e56e5ad  docs/design/purrdf-simd.md
```

The new row initially contains seven em dashes rather than invented counts. Root is generating actual counts with the existing full `--write-doc` matrix, then separately generating/verifying reports for the resulting identity. `--write-doc` and `--report` are mutually exclusive driver modes. The document's prepared hash will change on legitimate count regeneration. If the shipping symbol is not emitted, actual symbol discovery must resolve it; no force-export, fake symbol, compilation bypass or attribution to unrelated work is accepted.

## Validation completed by this reviewer

* `python3 -B scripts/check-simd-asm.py --self-test`: exit 0, ten runtime tests pass; every refusal and valid-neighbour fixture passes.
* Static imported `doc_checks(actual_doc, actual_manifest, {}, workspace_world(), compare=False)`: 103 sites, seven configurations, zero problems; typed manifest, bench/site/member coverage and document structure pass. This mode does not verify instruction counts.
* `git diff --check`: exit 0.
* Typed old/new manifest comparison: no changed or removed old sites, only `sparql.scope-admission` added; build graph unchanged.

No Rust build was run by this reviewer during root's matrix measurement.

## Compiler identity and final qualification boundary

Both fully read CI logs installed `rustc 1.101.0-nightly (0abfedbc7 2026-10-02)`, full commit `0abfedbc7cd4e725f126913880c95800394f7c37`, host `x86_64-unknown-linux-gnu`, LLVM `23.1.1`. The channel update was 2026-10-03. CI's compiler cache identity was `0f9065be5746602644a4805e3f5fddcbb6feea69d04eba1b95f981d8e48ae120`.

Root's current local compiler is `rustc 1.100.0-nightly (4b6d04e70 2026-09-13)`, also LLVM `23.1.1`. Matching LLVM versions do not imply identical emitted instruction counts. The seven original failures contain no measured-count parity evidence. Root must verify the regenerated document and seven successful matching assembly reports, and the final hosted matrix must pass under its actual compiler. This finding does not authorize changing the toolchain, weakening any threshold, removing document comparison or substituting cached verdicts.

The separate issue 384 compliance review remains byte-identical: `/tmp/purrdf-384-compliance-review.md`, SHA-256 `fb8385d3c5578d9555f30f88b45441c4c9dd096dcbe82e22fd33d000cd133278`.
