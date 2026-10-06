# Stage 2 assembly failure diagnosis

Read-only specialist diagnosis, 2026-10-06. This is not a gate pass or a merge recommendation. No Cargo invocation, generator, source edit, forge mutation, or retry was performed. The concurrent remediation implementer retains exclusive source and Cargo ownership.

## Finding

The two failed jobs have one problem each: committed document parity for `core.paged-map-quad`. The v3 document cell is `v14·f0·r0` while the measurement is `v0·f0·r0`; v4 is `v10·f0·r0` versus `v0·f0·r0`. These are observed document failures, not failures of an explicit vectorization guarantee. The manifest sets `min_vector_ops = 0`, `max_fma = 0`, and `forbid_relaxed = true`, with no required mnemonic or single-copy guarantee for this site. Both configurations completed assembly measurement without a missing-function, vector-floor, FMA, relaxed-instruction, or provenance refusal.

There is independently verified stale descriptive prose in the manifest and document: `paged::map_quad_to_global` is no longer a source function. Both integration-base and reviewed PR source call the shared `QuadIds::map_ids` home, with `PageTranslation::to_global` as the closure. The actual manifest selector is the existing, non-generic `<purrdf_core::ir::paged::PagedDataset>::from_provider` method. Its selected body is the *whole eager seal*, not a separately isolated four-column gather. The old positive counts therefore did not establish how many instructions the gather itself used.

The exact instruction-level reason for 14/10 becoming zero is **not established** by the available CI receipts. New stream iterators and changed checked dictionary reinterning can alter codegen/inlining, but that remains an inference. The failed jobs expose aggregate selected-function counts only; they do not upload raw `.s`, parsed functions, compiler-command receipts, or even a passed JSON report. Do not claim a proved vector regression, proved generic outlining, or proved harmless count relocation from these logs alone.

## Exact identities and receipts

CI run: https://github.com/Blackcat-Informatics/purrdf/actions/runs/37525131954

- PR head: `b2edf7450cf654d20ae97bd856d67102d0916b7b`.
- Actual checkout: GitHub synthetic merge `44a368ac1503db6f10bda40c24fd90bca063dc34`.
- Merge parents: integration base `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`, then PR head above.
- Merge tree: `13496228db0e5ec79074acad9020cac5aab7556e`; the PR head has the identical tree (verified locally and against GitHub's commit API).
- Compiler: `rustc 1.101.0-nightly (ea137335b 2026-10-05)`, full commit `ea137335b78829b4514bf1b4c16302f74fab8581`, host `x86_64-unknown-linux-gnu`, LLVM `23.1.3`.
- Manifest SHA-256: `aa5731127a2849e34bbfc84c518bd43f5b219a7bef30b0452fac6315f5f1ee2a`.
- Passed old-head shard source identity: `deea3f60539edd66dce894c39b9a38ef515d62b9c5e6e04c2519b34cf74e9ec6`.
- v3 executed `make simd-asm SIMD_ASM_ARGS="--config x86_64-v3 --jobs 1 --report target/asm-reports/x86_64-v3.json"`; v4 used its own name. Codegen configuration is release, LTO disabled, one codegen unit, `--emit=asm`, warnings denied, and the respective `target-cpu=x86-64-v3`/`x86-64-v4`. The reader checks the actual target rustc command lines and current assembly hashes, including reused units. v3 shows 31 rustc invocations and one reparsed unit; cached output from base was not accepted merely on a cached pass verdict.

Five successful shard reports were downloaded from this exact run to `raw/S2-simd-oldhead-reports/`, without rebuilding. All have the same source, manifest, and compiler identities above. Their measured cells are:

| Configuration | `core.paged-map-quad` |
|---|---|
| x86_64 | v0·f0·r0 |
| aarch64 | v0·f0·r0 |
| aarch64-neoverse-v1 | v0·f0·r0 |
| wasm32 | v0·f0·r0 |
| wasm32-simd128 | v6·f0·r0 |

This does not form a complete successful matrix, and these reports cannot qualify the forthcoming G1/G2 source or a changed document/manifest. The source identity deliberately includes all tracked content, including document edits. `--merge-reports` requires exact current identities, successful status, exactly one column per configuration, every site, and all seven configurations.

Additional receipts under `raw/`:

- `S2-CI-x86-v3-job.log` and `S2-CI-x86-v4-job.log`: full failed job logs, including checkout, compiler, commands, counts and the sole parity refusal.
- `S2-simd-run-metadata.json`, `S2-simd-run-artifacts.json`, `S2-simd-ci-merge-identity.json`: attributable GitHub readbacks.
- `S2-simd-oldhead-report-identities.jsonl`: extracted identities and cells from the five downloaded reports.
- `S2-simd-oldhead-paged-mod.rs`, `S2-simd-oldhead-paged-query.rs`, `S2-simd-oldhead-paged-source.patch`: reviewed old-head source and the exact integration-base delta.
- `S2-simd-diagnosis-inputs.sha256`: hashes of unchanged gate, manifest, document and shared `QuadIds` source used in this diagnosis.
- `S2-simd-local-assembly-inventory.txt`: no local assembly context at this worktree; `S2-simd-local-asm-candidates.txt` inventories shared-host assembly candidates. These were not used as measured evidence because none was established to have the required old CI source/compiler/configuration identity.

## Selector analysis

The reader's `symbol_names` is anchored: an exact method path or that path followed by generic arguments or compiler-generated nested items can match. It cannot silently pick a similarly named unrelated function. `measured` excludes nested closure bodies when the actual parent exists. `evaluate` takes the **minimum vector count** over matched non-closure copies; the printed cell does not disclose each copy, their graph/unit identities, or their instructions. Thus a generic-match collision is not supported here, but aggregate output alone cannot show the individual selected bodies or prove the gather remains inline in the parent.

The PR's `paged/mod.rs` delta at the failed head consists only of the stack module and exports; the `from_provider` source body, `QuadIds::map_ids`, PageTranslation source, gate, manifest, and document match the integration base. The new physical stream mapping code is in `paged/query.rs`. This strengthens the need to inspect optimization effects rather than changing the production algorithm in response to an instruction-count cell.

## Required repair and qualification

1. Finish G1/G2 and bind the final production source tree. Keep floating nightly repository and workflow policy unchanged. For reproducing this CI failure explicitly select `RUSTUP_TOOLCHAIN=nightly-2026-10-06` only after `rustc -vV` proves the full `ea137...`/LLVM `23.1.3` identity; the root default `4b6d04e...`/LLVM `23.1.1` cannot settle these CI counts. If hosted CI moves to a newer compiler, bind and measure that final hosted compiler instead.
2. On the stable final source, run the existing gate's probe for v3 and v4, preserving compiler identity, source tree and input hashes, output, `.s` hashes, the context key (and `.stage-asm-context` marker on Stage), evidence compiler-command receipts and parsed bodies. The runtime derives that context key from compiler/configuration/build-command/environment identity; it does not write an `identity.json`. Example (diagnostic, **not** a gate pass):

   ```sh
   RUSTUP_TOOLCHAIN=nightly-2026-10-06 CARGO_BUILD_JOBS=2 python3 scripts/check-simd-asm.py --config x86_64-v3 --jobs 1 --crate purrdf_core --probe 'PagedDataset.*from_provider|QuadIds.*map_ids|PageTranslation.*to_global' --dump
   ```

   Repeat with v4. Read `from_provider`'s actual page mapping call sites and any outlined body reached from them. This determines whether the measurement still includes the intended gather and whether multiple matched copies affect the minimum. A zero count is valid for a scalar gather, but it must be attributed to the actual body.
3. Correct the manifest summary and document `fn`/source/reason to name the shared `QuadIds::map_ids` mapping, with the enclosing eager seal explicitly identified. Retain `from_provider` as selector only if the body inspection supports that description. If the mapping is outlined, select and describe its real production instantiation/enclosing owner, with all configurations covered; do not pick a convenient unrelated positive-vector body and do not add a duplicated production helper for measurement.
4. After this semantic selector decision, run the complete seven-configuration measurement with the exact qualification compiler on final source, using the existing `--write-doc` generator. Partial shards cannot write the document. Inspect the complete generated diff; do not hand-rewrite only the two red cells from the old logs.
5. Because document/manifest edits change report source identity, generate seven checked reports on the final edited source (one full-matrix `--report` or seven shards), then verify `--merge-reports` against that same source/manifest/compiler. Final hosted CI must also pass all seven configurations and the aggregate at the final PR head/candidate; preserve all report identities and counts. If final hosted compiler differs, repeat projection/qualification against its actual identity.

No production vectorization repair is justified by current evidence. The concrete authorized fix scope starts with the stale shared-home description and fresh, correctly attributed projection. Whether the selector must change remains an exact-body question for final source qualification, not an assumption resolved by this read-only diagnosis.

Memory lesson used only to choose the validation method: `MEMORY.md:144-149` (historical SIMD compiler/projection discipline), rollout `01a1054e-45e6-7953-88ec-167a62c01f89`. Every compiler, source, artifact and failure fact above was freshly verified.
