<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# purrdf-testkit

The workspace's shared test support, written once instead of once per crate.

Never published, never a runtime dependency: it appears only in
`[dev-dependencies]`, so no release crate and no wasm build ever sees it. It
depends on no `purrdf-*` crate — every member's tests may use it, and a
first-party edge from here would close a cycle through that member.

## What it provides

* **Golden files** — `purrdf_testkit::assert_golden!("dir/name.txt", &text)`
  compares `text` with the calling crate's `tests/golden/dir/name.txt` byte for
  byte, CRLF and trailing whitespace included. `PURRDF_REGENERATE_GOLDEN=1`
  rewrites the file from the produced text instead; the diff is the review.
* **Temporary paths under `target/`** — `temp_dir!()` and `temp_file!()` in
  integration tests and benches (they read `CARGO_TARGET_TMPDIR` at the call
  site), `TempDir::for_unit_test()` and `NamedTempFile::for_unit_test()` in a
  crate's `src/` unit tests. Names are unique per process, thread and instant;
  creation is exclusive and a collision is an error; the path is removed on
  drop. The system temporary directory is never used.
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
