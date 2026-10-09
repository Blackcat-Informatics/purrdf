# Issue #308: Reduce native CI test compilation and C smoke time

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

The native workspace test lane spends most of its elapsed time compiling and preparing test artifacts. Profile and reduce that work while preserving the full gate.

Observed on final head `839e968a509246d6fc00d17d5fd9496eac8fa3e5` in [the native CI job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/34911560124/job/104199963497):

- The job ran from 00:03:32 to 00:36:12 UTC on 2026-09-15, about 32 minutes 40 seconds.
- Cargo reported 26 minutes 2 seconds to finish the optimized test build.
- `crates/rdf-capi/tests/c_smoke.rs` then took 275.71 seconds. That test invokes a separate `cargo build -p purrdf-capi --profile dev` to produce a fresh shared library before compiling, linking, and running its C callers. Its measured duration includes all of those steps; their individual costs still need profiling.
- The complete native suite passed, as did the other CI lanes.

Start with Cargo timing data and separate compilation, linking, and test execution costs. Check whether repeated compilation or artifact preparation contributes materially before selecting a change.

Acceptance:

- Record comparable cold and warm phase timings before and after the change, using the configured runner concurrency.
- Keep the complete Rust suite, strict lint and hygiene gates, optimized development profile, debug assertions, and overflow checks.
- Keep the real C header, linkage, and execution checks. Any artifact reuse must prove that it matches the current sources and build configuration; file existence alone is insufficient.

This is authorized performance work to track separately from the 2.0 release. The current release continues through its existing gates without performance changes.


## Comments (0)

