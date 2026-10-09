# Owning host and performance continuation

Prepared commands; execution remains pending the corrected numeric continuation and parent lane admission. Use the current managed nightly first on PATH, private target/build/TMPDIR, eight Cargo jobs and an attributable 64GiB/Swap0 scope. Preserve actual command exits, complete outputs and source readbacks.

The registered harness=false targets run their actual wasm transcripts through the existing version-checking Node runner:

```sh
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$PWD/scripts/wasm-test-runner.sh"
cargo test -p purrdf-xsd --test exact_wasm_determinism --target wasm32-unknown-unknown --locked --jobs 8
cargo test -p purrdf-sparql-eval --test numeric_wasm_determinism --target wasm32-unknown-unknown --locked --jobs 8
cargo test -p purrdf-cli --test exact_numerics_cli --locked --jobs 8
```

C smoke and Python are the unchanged host routes qualified by fresh hosted CI on the published source. No separate Python wheel campaign is claimed or proposed.

For the accepted matched native performance comparison, the immutable main341 archive is `/opt/purrdf-477-main-341ad`. Use its private baseline target/build and the candidate's existing private target/build, the same SDK, production release profile and shared `PURRDF_BENCH_HOME=/opt/purrdf-477-qualification/bench-records`:

```sh
cargo bench -p purrdf-xsd --bench exact --locked --jobs 8 -- --quick --bench xsd_exact_small/ xsd_exact_growth/mul/ --save-baseline main341
cargo bench -p purrdf-xsd --bench exact --locked --jobs 8 -- --quick --bench xsd_exact_small/ xsd_exact_growth/mul/ --baseline main341
```

The first command runs from the immutable baseline; the second from the candidate. The existing harness supports both filters and limits quick sampling to ten samples, 0.5-second warm-up and one-second measurement. Retain every selected estimate and actual comparison, including regressions. No measured performance verdict exists yet.
