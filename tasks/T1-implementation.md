<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 1 implementation evidence

Status: SUCCESS

Task 1 is implemented and its required focused checks passed. This report does
not claim completion of the remaining plan tasks, independent review, commit
hooks, the full workspace gate, hosted CI or merge. No commits, pushes or forge
mutations were performed by this implementation agent.

## Source identity

Worktree: `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
HEAD/base: `ce3c07192aba1e36666062c00f958670a827cfb5`.
Approved plan SHA-256: `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

`raw/T1-source.diff` records the six tracked-file changes and all four new
source/fixture files; SHA-256
`a6efa1d948137049c1b47e2504f45a5009d919aaae21112641b4de6e97a828d6`.
`raw/T1-files.sha256` records the individual current file identities. The
artifact was assembled with `git diff --binary HEAD` for existing files and
`git diff --no-index /dev/null` for each new file without changing the index.

## Files and behavior

- `crates/hash/src/sha3.rs`: public `Shake<128>`/`Shake<256>` with named aliases
  `Shake128`/`Shake256`, `const new`, `Default`, streaming `update`, consuming
  `finalize`, caller-buffer one-shot `digest`, and a distinct `ShakeReader`
  with incremental `squeeze`. Supported strengths are constrained at compile
  time. Readers cannot absorb input. Absorber/reader clones preserve stream
  position; Debug output elides buffered input and permutation state.
- The same file shares absorption and suffix/padding helpers with SHA3.
  SHAKE uses rates 168/136 and delimited suffix `0x1f`; SHA3 retains suffix
  `0x06`. Both use the existing BlockBuffer, `absorb_blocks` and Keccak-f1600
  implementation. No second permutation or block buffer, dependencies,
  Cargo features or external implementation constants were introduced.
- `crates/hash/src/lib.rs`, `crates/hash/README.md` and
  `crates/hash/PROVENANCE.md`: SHAKE API usage, output-stream semantics and
  first-party provenance. All core SHAKE calls operate in constant space over
  caller-owned slices without allocation or OS access.
- `helpers-ledger.toml`: existing `sha3` job names SHAKE entry points and both
  new fixture files in the same home.
- `crates/hash-conformance/Cargo.toml`: `shake` target uses the existing
  `harness = false` testkit runner and existing integration-4 shard pattern.
- `crates/hash-conformance/tests/shake.rs`: public API replay of four complete
  NIST answers, 16 independent boundary/long-output answers, every absorption
  split, byte-at-a-time input, lane/rate output splits, eight output chunk
  sizes, empty calls before/inside/after both phases, Default and cloned
  absorbers/readers. The module doctest also proves a reader's `update` is a
  compile-time error.
- `crates/hash-conformance/tests/vectors/shake_nist_vectors.txt`: all 512
  bytes from each NIST SHAKE128/256 msg0/msg1600 answer, with record count and
  SHA-256 body checksum verified by the existing Rust vector reader.
- `crates/hash-conformance/tests/vectors/shake_boundary_vectors.txt`: OpenSSL
  3.6.4 public `dgst` answers for each mode's rate-1/rate/rate+1,
  2*rate-1/2*rate/2*rate+1 and 3*rate input lengths, with 3*rate+17 output
  bytes; additionally 17-byte input with 4097-byte output for each mode.
- `crates/hash-conformance/tests/vectors/shake-PROVENANCE.md`: primary NIST
  URLs/PDF hashes, input recipes, output counts, licensing and independent
  OpenSSL capture procedure. No governed root `vectors/` file was modified.

The one-time import was written in Rust and retained in
`raw/import-shake-fixtures.rs`. It extracted only each captured NIST text's
final `Output val is` block, checked each length was exactly 512 bytes, and
queried OpenSSL for independent additional output bytes. It never called the
implementation under test. OpenSSL is absent from committed test/runtime
dependencies. All retained tests are Rust. Fixture body checksums were
computed independently and checked by testkit during the tests.

## Validation

Compiler: `rustc 1.100.0-nightly (4b6d04e70 2026-09-13)`;
commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1,
host `x86_64-unknown-linux-gnu`. Build parallelism was capped using
`CARGO_BUILD_JOBS=4`; no profile, warning, assertion or overflow-check override
was used. Cargo reports the native `test` profile as optimized and the wasm
`dev` profile as optimized with debuginfo.

| Command | Actual result | Evidence |
|---|---|---|
| `CARGO_BUILD_JOBS=4 cargo test --locked -p purrdf-hash -p purrdf-hash-conformance` | Exit 0; 129 native tests and 29 doctests passed, including the compile-fail type-separation example | `raw/T1-test-final.log` |
| `CARGO_BUILD_JOBS=4 cargo test --locked -p purrdf-hash-conformance --test shake` | Exit 0; both SHAKE tests passed | `raw/T1-shake-test.log` |
| `CARGO_BUILD_JOBS=4 cargo test --locked -p purrdf-hash --doc` | Exit 0; all 29 doctests passed after final documentation qualification | `raw/T1-docs-final.log` |
| `CARGO_BUILD_JOBS=4 cargo clippy --locked -p purrdf-hash -p purrdf-hash-conformance --all-targets -- -D warnings` | Exit 0, warning-free | `raw/T1-clippy-final.log` |
| `CARGO_BUILD_JOBS=4 cargo build --locked --target wasm32-unknown-unknown -p purrdf-hash --lib` | Exit 0 | `raw/T1-wasm.log` |
| `CARGO_BUILD_JOBS=4 python3 scripts/check-shared-helpers.py` | Exit 0; 77 enforced jobs, 91 distinct rows, no includes leaving their crates; indexed 1808 Rust files | `raw/T1-helpers.log` |
| `CARGO_BUILD_JOBS=4 python3 scripts/check-test-shards.py` | Exit 0; six feature-unified shards cover all 42 members | `raw/T1-shards.log` |
| `cargo fmt --check -p purrdf-hash -p purrdf-hash-conformance` | Exit 0 | Checked after final edits |
| `git diff --check` | Exit 0 | Checked after final edits |

The final complete package run replays all 14,097 unchanged differential
records for each SHA3-224/256/384/512 variant (56,388 SHA3 records total),
using the existing one-shot and streamed public API checks. Frozen outputs
were not adjusted to match this implementation.

After the package/clippy/wasm checks, the parent requested two documentation
qualifications: `Digest` applies to fixed-output hashers, and SHAKE has four
NIST plus 16 boundary records rather than the fixed-output suite's 14,097 per
algorithm. Only prose in lib.rs/README.md changed; implementation and test
code were unchanged. The final affected doctest check passed against those
current bytes, and the diff/file identities above include the qualifications.

Earlier captured `raw/T1-test.log` and `raw/T1-clippy.log` are failed development
runs, not qualification evidence: the first SHAKE replay incorrectly accessed
an oracle field through VectorFile's input-only replay closure, and clippy
identified tuple-to-array and fixed-size chunk idioms in new tests. The real
VectorFile implementation was inspected; corrected tests access full verified
records, preserving complete independent expected bytes. Both lints were fixed
without allowances. The final complete runs above supersede those failures.

## Completion and limits

Every Task 1 requirement has executable evidence: public SHAKE byte answers,
independent rate/padding/long-output boundaries, split and empty calls,
distinct consuming phases, unchanged SHA3 vectors, warning-free affected
clippy and wasm library build. Helper/source rules and shard registration
passed. No Task 1 implementation requirement is deferred or stubbed.

Full-workspace `make check`, commit hooks and hosted CI were not run here.
Independent task review and the parent's authorized commit/push steps remain
the Stage workflow's next actions. This implementation report is not a review
verdict or an overall issue completion claim.
