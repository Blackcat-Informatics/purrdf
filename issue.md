# Issue #536: Restore MSRV shapes build and Arm64 numerical replay gates

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

PR #535's hosted run on main-derived source exposes two additional compiler/runtime gate failures outside the packaging patch. Both require production or test corrections and rerun; neither is a passing check. Related evaluator consumer/doc failures are recorded in #531, and the geo Clippy failure is recorded in #533.

## MSRV shapes build

The `msrv` job fails `E0275`, overflow evaluating `&_: IntoIterator`, at `crates/shapes/src/schema_surface.rs:3462` in `scc_component_index`; `purrdf-shapes` does not compile under the job's pinned compiler. Diagnose the exact inference chain and make the implementation compile at the declared MSRV without raising the MSRV or suppressing a check. Run the affected compiler/package check and the hosted job.

## Arm64 numerical replay

The `aarch64` job runs `cargo test --locked -p purrdf-xsd --test certified_numerical` with `PURRDF_REQUIRE_SIMD_PATHS=numeric:portable,numeric:neon`. `directed_endpoint_product_replay` fails: `numeric` is not a registered `PURRDF_REQUIRE_SIMD_PATHS` family (registered families reported: blake3, crc32, csv, deflate, distance, hex, sha1). Reconcile the test's path requirement with the actual numeric dispatch contract, preserve the real replay, and rerun it on Arm64. Do not treat target compilation as runtime acceptance.

These failures were observed on PR #535 head `96f8c5f63`, whose source delta is confined to packaging scripts and Makefile comments; both named Rust files are unchanged from its main base. PR #535 was rejected and closed without merging. Its discarded tooling changes must not be revived. The hosted job logs are the source of these observations, not evidence that their fixes have been implemented.


## Comments (1)

### paudley — 2026-10-12T05:01:27Z

The same unregistered `numeric` SIMD-family failure also stops the hosted `test (lib)` shard, not just the Arm64 replay: `purrdf-xsd::math::floating::packed::tests::forced_available_paths_preserve_every_product_and_outward_endpoint_bit` panics at `crates/hash/src/dispatch.rs:285` under `PURRDF_REQUIRE_SIMD_PATHS`. That shard reports 413 passes and 1 failure before stopping. The `test (integration-1)` shard repeats the `directed_endpoint_product_replay` failure. These are additional affected checks for this issue's numeric dispatch correction; neither shard completed as a passing gate.


