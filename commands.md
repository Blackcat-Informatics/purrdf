# Native acceptance commands

All invocations use the normal Cargo wrapper with CARGO_BUILD_JOBS=8,
RAYON_NUM_THREADS=4 and at most four test threads.

- native-complete: cargo test --locked -p purrdf-cdt, RUST_TEST_THREADS=1; exit 0, 166 cases and 55 doctests.
- strict-workspace-final: cargo clippy --locked --workspace --all-targets -- -D warnings; exit 0.
- strict-cdt-final: cargo clippy --locked -p purrdf-cdt --all-targets -- -D warnings; exit 0 after final native-test additions.
- commit and push: normal signed commit and ordinary push, required hooks pass.
- strict-workspace preserves the earlier corrected test lint; native-errors is the initial focused pass.

GhpRsq excludes live .log files. Settled outputs below are preserved byte-for-byte
as acceptance-evidence/<same-stem>.txt and compared directly before capture.

- commit.log
- native-complete.log
- native-errors.log
- push.log
- strict-cdt-final.log
- strict-workspace-final.log
- strict-workspace.log

- integration-native: exact combined tree dbcbb7ee6653ef5aa47971e13dfcbe7dfffc085b; cargo test --locked -p purrdf-cdt -p purrdf-shapes --test structured_errors --test catalog_references --test conformance_memo --test shape_catalog --test change_path_alloc; exit0, all41 cases pass.
