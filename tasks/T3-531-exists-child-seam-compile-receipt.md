# EXISTS child seam compiler feedback

- Checkout HEAD: `330c55f8c`. Affected `expr.rs`/`eval.rs`/`modifier.rs` diff SHA-256: `9d78e45b0b0e43bdc113c3892580910eb7e4fac4a46f9aa2c61645a8844eda6c`.
- Standard direct `CARGO_BUILD_JOBS=1 cargo check --locked -p purrdf-sparql-eval --lib`, session `28845`, terminal exit `0`; original stdout `tasks/T3-531-exists-child-seam-retry.log`.
- The first compiler attempt, `tasks/T3-531-exists-child-seam-check.log`, failed E0308 because the probe cache tuple shadowed the original site address. The names are now distinct and the retry passed.
- Scope: `ExistsChild` owns the prepared site through the existing uncached probe or per-row definition graph child and both recursive branches use its one completion body. This is still synchronous execution, not the worklist's resumed child. No current-source runtime or optimized WASM depth qualification has run.
