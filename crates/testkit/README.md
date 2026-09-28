<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# purrdf-testkit

The workspace's shared test support, written once instead of once per crate.

Never published, never a runtime dependency: it appears only in
`[dev-dependencies]`, so no release crate and no release wasm build ever sees
it. It builds and runs on `wasm32-unknown-unknown` too, where the workspace's
cross-target test targets run on its harness. It
depends on no `purrdf-*` crate — every member's tests may use it, and a
first-party edge from here would close a cycle through that member.

## What it provides

* **Golden files** — `purrdf_testkit::assert_golden!("dir/name.txt", &text)`
  compares `text` with the calling crate's `tests/golden/dir/name.txt` byte for
  byte, CRLF and trailing whitespace included. `PURRDF_REGENERATE_GOLDEN=1`
  rewrites the file from the produced text instead; the diff is the review.
  `assert_golden_bytes!("dir/name.bin", &bytes)` is the same for output that
  is bytes rather than text, and a difference is reported at its first offset.
  The switch is read by `purrdf_testkit::env_flag`, the one rule for a switch
  in the environment: set to exactly `1`, and nothing else counts.
* **Workspace paths** — `purrdf_testkit::workspace_root!()` is the nearest
  ancestor of the calling crate's manifest directory whose `Cargo.toml`
  declares a workspace, as spelled and never canonicalized;
  `paths::collect_files(&root, "rs")` and `paths::collect_rs(&root)` sweep a
  tree in sorted order, skipping `target`, `.git`, `node_modules` and
  `.worktrees` at every depth, and refusing an unreadable root rather than
  answering with an empty sweep.
* **Temporary paths under `target/`** — `temp_dir!()` and `temp_file!()` in
  integration tests and benches (they read `CARGO_TARGET_TMPDIR` at the call
  site), `TempDir::for_unit_test()` and `NamedTempFile::for_unit_test()` in a
  crate's `src/` unit tests. Names are unique per process, thread and instant;
  creation is exclusive and a collision is an error; the path is removed on
  drop. A caller may place the target directory under the system temporary
  directory; these helpers still keep every path beneath that target.
* **Frozen differential vectors** — `purrdf_testkit::vectors` records an
  implementation's answers to a line-oriented file whose header carries a
  SHA-256 of its own body, and replays the file against a replacement,
  reporting the first record it disagrees with. An edited record fails the
  digest check before any replay runs.
* **A `harness = false` runner** — `purrdf_testkit::harness::main(trials)`
  prints libtest's console output (including the exact
  `test result: … passed; … failed; … ignored; 0 measured; … filtered out; finished in …s`
  tally line), accepts libtest's flags, runs cases on `--test-threads` workers
  with per-case panic isolation, and refuses any flag it does not implement.
  `purrdf_testkit::harness_main!(case_a, case_b)` writes the `main` that runs
  plain functions as cases named after them. The same target runs on
  `wasm32-unknown-unknown` in Node under `scripts/wasm-test-runner.sh`, the
  cargo runner `make wasm-test` sets: the command line, environment, console
  and clock are Node's, cases run serially, and since a panic aborts a wasm32
  module, a panicking case is reported `FAILED` with its message and tally
  before the module traps and the runner exits non-zero.
  `harness::without_host_clock_or_entropy` runs a computation with every host
  clock and entropy source throwing on wasm32, for answers that must be a
  function of their inputs alone; `harness::print_line` prints a line that
  reaches the console on both targets.
* **Deterministic streams** — `purrdf_testkit::rng`: the SplitMix64 step
  every fixed-seed test draws from (with `SplitMix64::below`, the modulo draw
  the per-crate wrappers made), the xoshiro256** generator behind `prop`, the
  signed-unit coordinate draws with their zero substitutes, the Fisher–Yates
  `permute` the determinism tests shuffle with, the xorshift64 byte stream the
  frozen hash vectors were recorded from, and Knuth's MMIX `Lcg` with both of
  the high-bit selections fixtures use — each pinned by known answers so a
  frozen expectation cannot move.
* **Property-based testing** — `purrdf_testkit::prop` and `prop_test!`.
  Strategies (ranges, `any::<T>()`, `prop::collection::{vec, btree_set,
  btree_map}`, `prop::option::of`, `prop::sample::select`, `prop_oneof!`,
  `Just`, `prop_map`/`prop_filter`/`prop_flat_map`/`prop_recursive`, and
  `prop::string::regex`, which walks the `regex-syntax` IR so a pattern means
  what it means to `regex`) draw bounded integers from a recorded choice
  sequence. A failing input is shrunk by editing that sequence and replaying
  it, so a shrunk value always satisfies its generator; the failure prints the
  value and the sequence in hex, and `prop::replay(&strategy, hex)` turns it
  into an ordinary regression test. `prop::state_machine` checks an
  implementation against a reference model over generated, shrinkable runs of
  operations. Every property's seed is derived from its module path and name —
  never OS entropy — and `PURRDF_PROP_SEED` replaces it for one run;
  `prop::cases_from_env(n)` reads `PURRDF_PROP_CASES` for properties that offer
  a deeper search on demand.
