# #531 deferred LATERAL layered-fallback candidate — rejected

The source-only candidate moved the disagreeing `EnvState::Layered` fallback out
of `eval_deferred_lateral` while keeping the merged-row success body unchanged.
Candidate `binop.rs` SHA-256 was
`a3dcbf6fa0a4d4bc486b0a542ade32df1002aa23701e3042d530f416ca38a9d4`.
Affected strict Clippy passed; the original native batch passed 58/58
(`T3-531-layered-strict-corrected.log`, `T3-531-layered-native.log`).

Unchanged `scripts/check-shared-helpers.py` stopped at the previously obsolete
`purrdf_xsd::numeric::Decimal::cmp_exact` ledger symbol. Its terminal 1 does
not certify duplicate coverage (`T3-531-layered-helper-census.log`). No ledger
or checker change was made.

Standard-directory optimized Cargo release, selected compiler artifact,
provided wasm-bindgen, wasm-opt, wasm-link and SIMD count all passed. The raw
WASM SHA-256 was
`8d83e4c5264c8beae3429bc86b5080480a2509f06d9e1d54ecc5b3ef495e94a7`;
linked WASM SHA-256 was
`d1bfc28895a3d7e81542b783ce8e846e74454c684c3731926a270548fb81ce1c`;
SIMD count was 423,693. See `T3-531-layered-default-dir-*` receipts.

On that exact package, the unchanged original LATERAL depth test failed at
**267 < required 281**, and the unchanged FILTER NOT EXISTS test failed at
**133 < required 180** (`T3-531-layered-original-floors.log`). The production
`queryGovernedAsync` high-water control for the original LATERAL query shape
reported depths 2/4/8/16/32 as 33,804/40,972/55,308/83,980/141,324 bytes:
**3,584 bytes per added level**, versus the earlier 4,080. The original
nine-edge NOT EXISTS walk reported 47,772/61,980/90,396/147,228/260,892
bytes: still **7,104 bytes per level**. All these short controls completed
with exact row answers (`T3-531-layered-{stack-bytes,walk-stack-bytes}.log`).
The alternative simple repeated-NOT-EXISTS diagnostic in the first log has a
different syntax and must not be compared with the prior repeated control.

The candidate improved the LATERAL slope but did not meet the original
accepted-depth floor and did nothing for NOT EXISTS. The exact patch is
`T3-531-layered-fallback.patch`, SHA-256
`1ca583b4cb192e46c919dc73109e48851b5d575cc9e550dea87fa7256f52295c`.
`git apply -p0 --reverse --check` passed; the patch was reverse-applied and
the source worktree is clean. The installed package is now a **rejected,
stale-source diagnostic artifact**, not final #470/#531 qualification.

The remaining 3,584-byte LATERAL slope includes the caller's retained left
solution, per-row result collection and the successful substituted-right
evaluation; this source observation alone does not assign individual linked
frame sizes. A further boundary change would need direct retained-frame
evidence before implementation. #531 and the original full Node package gate
remain open; no stack budget, floor, or assertion was changed.

Separately, current-main-based #539 hosted package log
`.worktrees/539-purrdf-text-make-the-surfaceindex/.stage/purrdf-text-make-the-surfaceindex/tasks/third-wasm-package.log`
reports the same three floor failure modes under hosted Rust 1.101: synchronous
job 135 versus 182, LATERAL 236 versus 282, and NOT EXISTS 134 versus 181.
Those are a distinct source/toolchain artifact, not a direct before/after
comparison with this local Rust 1.100 candidate. They confirm that the open
#531 stack-floor debt is not resolved by #539's unrelated surface-index work.
