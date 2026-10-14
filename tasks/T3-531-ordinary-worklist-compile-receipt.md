# Ordinary FILTER/Slice worklist compiler checkpoint

- Checkout HEAD: `330c55f8c`.
- Affected source diff (`eval.rs`, `expr.rs`, `modifier.rs`) SHA-256: `d39514ac9d5930fbb64755837a92dfd8a8bfd66f477fa1105e0238e1a68371b2`.
- Command: `CARGO_BUILD_JOBS=1 cargo check --locked -p purrdf-sparql-eval --lib` from the numeric worktree, with standard Cargo directories.
- Original process/session: `41147`; terminal exit `0`. Raw stdout: `tasks/T3-531-ordinary-worklist-check.log`.
- Result: production library compiles. The ordinary FILTER/Slice graph-child worklist is only a partial driver. FILTER VM EXISTS, copied first-witness Slice, correlated LATERAL right children, and the three unchanged optimized WASM depth floors remain unqualified.

## Corrected source after prelude and unwind review

- Affected source diff SHA-256: `462d71709bba339137f4d2a827071574c5f6c805e8d2436657b137e1d51d4252`.
- `precomputed_evaluated` is the one existing dictionary/Shapes prelude for both the ordinary worklist and original dispatch. `OrdinaryRunner` retains active and suspended `NodeBoundary` owners and aborts them in LIFO order on unwind. One FILTER/Slice parent remains inline; nested frames use admitted storage.
- Direct standard j1 library check: `CARGO_BUILD_JOBS=1 cargo check --locked -p purrdf-sparql-eval --lib`, session `90717`, terminal exit `0`, raw `tasks/T3-531-ordinary-worklist-corrected-retry2.log`.
- Two failed compiler attempts remain visible: `tasks/T3-531-ordinary-worklist-corrected-check.log` (frame return lifetime E0506), then `tasks/T3-531-ordinary-worklist-corrected-retry.log` (obsolete `eval_node` wrapper dead code). Both were corrected in the final source. This remains compiler-only qualification; no current-source runtime or optimized depth claim.
