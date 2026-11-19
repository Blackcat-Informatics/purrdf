# Issue #1360: The SIMD assembly aggregation fails when the floating nightly changes during the matrix

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

The `simd-asm` aggregation fails with "assembly report producer compiler receipt mismatch" whenever the floating nightly changes while the seven `simd-asm-config` matrix jobs are running. The matrix straddles the publish, the jobs install different compilers, and the aggregator compares every report against a single expected compiler.

Rerunning one failed config job does not repair it: the rerun picks up the newest compiler, so the mismatch moves to a different report. Only a rerun of the whole matrix, after the nightly has settled, passes.

## Evidence

- `.github/workflows/ci.yaml`: the `simd-asm-config` job exposes `compiler-version` as a job output, and the aggregator writes it to `compiler-receipt.txt` as the expected compiler.
- `crates/helper-census/src/asm_reports.rs`: each report's recorded compiler must equal the receipt, or the merge fails with "producer compiler receipt mismatch".
- Observed on one pull request: attempt 1 failed on the `aarch64-neoverse-v1` report; a rerun of that job alone passed and the same aggregation then failed on the `aarch64` report.

## Proposed acceptance

- The matrix uses one compiler for all jobs of a run. For example, one job resolves the nightly once and the config jobs install that exact dated toolchain, or the aggregator accepts the set of compilers the jobs actually used when each report is internally consistent.
- Either way, the floating nightly of `rust-toolchain.toml` stays floating for the other gates, and a run that spans a publish passes without a manual rerun.
- A failing case: a run whose jobs install two different nightlies must still be refused unless the design explicitly allows mixed compilers; the neighbouring case: a run on one compiler passes.


## Comments (0)

