# Existing arithmetic assembly qualification

The preserved donor's October 15 validation says transform registration exists but the seven-configuration projection is not qualified. That execution-state claim is stale against current main.

Current scripts/simd-asm-manifest.toml xsd.exact-arith names mul_fast, div_rem, ntt::transform and append_large_decimal_groups. Current docs/design/purrdf-simd.md projects all seven configurations for all four sites. Its transform has integer-vector counts 25 on x86-64-v3, 16 on x86-64-v4 and 10 on wasm simd128; the other measured transform configurations show zero vectors, and every listed site has zero floating or reciprocal operations. Product/division/decimal-tree sites show zero vector operations in that projection. Do not turn those zeros into a claim of a new vector kernel or optimized architecture throughput.

PR 674's final hosted evidence (refs/ghprsq/pr/674/stage:raw/final-hosted-checks.json) has successful terminal jobs for x86_64, v3, v4, aarch64, aarch64-neoverse-v1, wasm32 and wasm32-simd128 plus the successful aggregate. Main's ntt.rs is unchanged from donor 1e216aaec. These results qualify existing emitted-assembly/catalogue coverage and supersede that narrow stale donor statement; no need to replay the unchanged seven builds merely for its old wording.

This is not a native/WASM instruction threshold sweep, a query-performance result, a new correctness execution, or completion of the whole architecture/SIMD contract. Scalar native and packaged-WASM exact arithmetic oracles are executed separately in the current render qualification. Current canonical output reservation changes no registered limb/transform/decimal-tree body or backend dispatch.
