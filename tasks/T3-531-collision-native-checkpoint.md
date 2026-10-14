# Collision-admission frame extraction: native checkpoint

Source: `expr.rs` SHA-256 `ac913d91f50b3ca06cb6b2b280e07db3588d2394c3eb4df2145e02e32af4ae55`; concurrent LATERAL `binop.rs` SHA-256 `2ef2cd3060508ed50e84f1ec991bd6935ab7b4dc89ce9756d2c161a79956f2ac`. Both changes are uncommitted. No package was built from this source yet.

- `cargo clippy --locked -j 8 -p purrdf-sparql-eval --all-targets -- -D warnings`: terminal 0, 51.27 s.
- `cargo test --locked -j 8 -p purrdf-sparql-eval --lib exists_ -- --test-threads=4`: terminal 0, 84 passed.
- `cargo test --locked -j 8 -p purrdf-sparql-eval --test evidence_ingress exists_transports_original_witnesses_through_vm_memos_and_deferred_sites -- --test-threads=4`: terminal 0, 1 passed.
- `cargo test --locked -j 8 -p purrdf-sparql-eval --test exists_sep0007 -- --test-threads=4`: terminal 0, 25 passed.
- `git diff --check`: terminal 0.

These checks cover collision refusal, definition and memo paths, and evidence/SEP semantics. They do not prove optimized WASM stack savings, the historical accepted depth floor, or whole-issue acceptance. Fresh default-directory optimized package and the unchanged NOT EXISTS slope/floor controls remain required.

## Optimized artifact and disposition

Default-directory `cargo rustc -j 8 -p purrdf-wasm --lib --crate-type cdylib --target wasm32-unknown-unknown --release --locked --message-format=json` passed. Cargo selected exactly one WASM artifact, SHA-256 `b6f07099a6a8e31b2536f80116fe01120757b4ac9856f56d98f652e578b53c43` (`T3-531-collision-default-dir-{artifact.jsonl,build.log}`). Provided wasm-bindgen, wasm-opt and wasm-link passed; optimized linked package SHA-256 `f4141cf4b0cf7917bf77dac248129aa423351cb59ec7e7cd2a0d89e6236916d8` (`T3-531-collision-default-dir-{wasm-opt,wasm-link}.log`).

The unchanged nested NOT EXISTS governed-query shape at depths 2/4/8/16/32 reached 45,276/59,484/87,900/144,732/258,396 shadow-stack bytes (`T3-531-collision-stack-bytes.log`), still exactly **7,104 bytes per level**. The unchanged original synchronous floor test **FAILED**: 133 accepted levels versus the required 180 (`T3-531-collision-original-floor.log`). The collision extraction did not improve the recursive stack slope or accepted endpoint.

The exact experiment is preserved as `T3-531-exists-collision-extraction.patch`, SHA-256 `b08f850f45a617f2ae58e9c08c8eecc58499fc1cd1952c74fd6c2fd63e239c21`. `git apply -p0 --reverse --check` passed and the patch was reverse-applied. `expr.rs` returned to SHA-256 `7602a00b68dfd89b05e81074d3183ac4ae816bc4a6ec21bf7b7ddb09985e324a`; only the separate LATERAL `binop.rs` experiment remains dirty. The installed package is now a rejected, stale-source diagnostic artifact, not #470 final qualification.
