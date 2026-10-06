<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 4 correction: unsigned verification and governed Debug

Status: SUCCESS

Required findings T4-R1 and T4-R2 are implemented and their focused checks are
terminal. Source mutation has stopped for the independent recheck. The original
review remains BLOCKED until its reviewer adjudicates this new identity; this
report claims neither review acceptance nor commit/push/publication. No child,
index mutation, hook bypass, Task 5 work, sibling/main edit or memory mutation
occurred.

## Identity and preserved evidence

Worktree `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`;
unchanged HEAD `31498279c83c7fb9c3c6d97faffc1628a82452e1`;
branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`;
base `ce3c07192aba1e36666062c00f958670a827cfb5`;
plan SHA-256 `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

Final complete ten-file manifest: `raw/T4-R1-files.sha256`, SHA-256
`9d4e72ee4bd4401648035678f33dbe5be90a7a7f93189cb78d028bd70bc95029`.
Complete HEAD-to-final patch, including both new Task 4 files:
`raw/T4-R1-source.diff`, SHA-256
`980c0c1af287f5b4290ad416242d96259cd5d68134257723419d068213ae4143`.
Initial-to-correction source delta: `raw/T4-R1-delta.diff`, SHA-256
`22e38764164c55f391fbd7f8d4521b0a410dd439ca2f2a4704c1ab8b6cb9b95b`.
Final hash readback passes for all ten files in `raw/T4-R1-manifest-check.log`.

The original review was copied verbatim BEFORE source changes to
`tasks/T4-review-initial.md`, SHA-256
`6985401566fa7f3861786371a91f80d3466ffd542efe12a4f8e2171a09760a1e`.
Original `raw/T4-files.sha256` remains
`5bf4fe0e02b17147d476fb1122ec22b911ff08b17551e651a2a2b0438103c5a9`;
original `raw/T4-source.diff` remains
`4c52edc05e7448779175aedc181c8b04b11880a8370eed0e02077ea0695a1931`.
All original reports/logs/probes remain unchanged. The initial ten-file source
was reconstructed from HEAD blobs plus that exact original patch under
`raw/T4-R1-initial/`, using working-tree-only `git apply` inside this artifact
copy, with no index change. All reconstructed hashes match the original
manifest (`raw/T4-R1-initial-manifest-check.log`) before generating the delta.

Only these three original Task 4 files changed for the correction:

- `crates/gts/src/verify.rs`
- `crates/gts/src/writer.rs`
- `crates/gts/tests/hedged_writer.rs`

The other seven final manifest entries match their original Task 4 identities.
No new correction source file, dependency, feature, policy implementation,
primitive, fixture, exemption, corpus or generated artifact was added.

## Corrections

T4-R1: `EmptyFile` is now a shared file-integrity error, preserving its actual
reader detail. Empty bytes, a torn first byte string and invalid first CBOR item
cannot succeed under unsigned opt-in. The unsigned/no-transport/no-signature
branch now calls `verify_graph_with_keyring` with an explicitly empty keyring.
That helper is the same result assembly used by ordinary keyring and resolved
OpenPGP verification, and invokes the existing signature/count/integrity/trust/
profile pipeline. Single-key metadata is added to that assembled result. There
is no unsigned policy copy or early success based solely on a diagnostic list.

`require_signatures(false)` disables the ordinary file-signature requirement,
while declared evidence/opaque profile rules remain the existing policy's
responsibility. Actual `ProfileSignatureRequired` and
`EvidenceHeadCommitmentRequired` findings, severity, profile and actionable
details reach the result. The existing sealed-source evidence exception remains
in the sole policy implementation, which was not changed. Clean generic unsigned
files and headers still succeed. Signed opaque MissingKey and UnknownCodec
payloads remain cryptographically verifiable without classifying unavailable
decoding capability as damaged bytes.

T4-R2: the manual nongeneric SnapshotSigner field-eliding Debug was replaced by
the exact existing `purrdf_hash::debug_non_exhaustive!(SnapshotSigner { kid,
public_key_armor });` invocation. Owned secret clearing and allocation-before-
secret-copy Clone are unchanged. The public redaction/Clone test executes again.

## Actual public counterexamples and neighbors

The final thirteen-group Writer/file suite executes natively and in wasm/Node.
The original ten groups rerun, covering mixed algorithms, provider atomicity,
both components, tampered file integrity, raw IDs, SnapshotSigner and actual
embedded/out-of-band OpenPGP behavior. Three new groups establish:

- Empty input, torn first item `[0x5f]`, invalid first item `[0xff]`, and a
  complete non-header item `[0]` refuse with nonempty errors and original
  EmptyFile/DamagedFrame details. Clean generic header and unsigned blob pass.
- Real unsigned evidence/opaque Writer files refuse with exactly the findings
  from the existing public policy evaluator, plus explicit assertions for
  required Error findings and nonempty details. Generic has no profile error.
- Actual evidence terms/quads using the existing sealedSource predicate pass
  under unsigned opt-in with the existing policy exception; a nonsealed
  predicate refuses. Warning findings are retained, not promoted to errors.
- A valid authenticated external blob using a declared unknown codec produces
  an actual UnknownCodec diagnostic and remains valid under the file keyring.

The first additional UnknownCodec input used `add_blob`, whose public digest
lets this reader keep the encoded payload lazy. It consequently did not attempt
the unknown codec and the diagnostic assertion failed both natively and in
wasm. Inspected the real `h_blob_frame`/`payload`/codec path before correction.
The final external-encoder neighbor omits the declared blob digest, forcing
the reader's actual decode attempt while computing it. The original failed
`raw/T4-R1-public-native-qualified.log` and `raw/T4-R1-wasm-qualified.log` remain
failed evidence; they are not passes. No reader or codec behavior was changed
to accommodate the test.

## Terminal qualification

All commands ran in the worktree with `CARGO_BUILD_JOBS=4` for compilation;
repository compiler/profiles/assertions/overflow/warning policy were preserved.
The following commands returned terminal exit 0:

| Command | Evidence |
|---|---|
| `cargo test --locked -p purrdf-gts --test hedged_writer` | `raw/T4-R1-public-native-complete.log`: all thirteen current groups pass |
| `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$WORKTREE/scripts/wasm-test-runner.sh" cargo test --locked --target wasm32-unknown-unknown -p purrdf-gts --test hedged_writer` | `raw/T4-R1-wasm-complete.log`: all thirteen groups actually execute and pass in wasm/Node |
| Original `cargo run --locked --offline --manifest-path "$STAGE/raw/T4-review-probe/Cargo.toml"` | `raw/T4-R1-review-probe.log`: all five independent counterexamples now refuse; generic valid neighbors pass; original findings/details are printed |
| `cargo clippy --locked -p purrdf-gts --all-targets -- -D warnings` | `raw/T4-R1-clippy-complete.log`: final warning policy passes |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps -p purrdf-gts` | `raw/T4-R1-docs.log`: current production API/linkage passes; subsequent test-input-only edit does not change rustdoc inputs |
| `python3 scripts/check-shared-helpers.py` | `raw/T4-R1-helpers-complete.log`: existing wrapper's Rust census passes, 81 enforced jobs, 91 distinct rows, 1818 files |
| `cargo fmt --check -p purrdf-gts -p purrdf-rdf -p purrdf-python` | `raw/T4-R1-format-complete.log`: final formatting passes |
| Complete tracked diff and both new-file no-index whitespace checks | `raw/T4-R1-whitespace.log`: no diagnostics; no-index 0/1 accepted only with empty output |
| Ten-file identity readback and complete added-line marker scan | `raw/T4-R1-manifest-check.log`, `raw/T4-R1-deferral-scan.log`: current hashes match, no added TODO/FIXME/XXX |

The standalone probe was NOT altered. Its source SHA-256 remains
`97f6ed0e4b340d57645f0accec834b6b14bf86419346d936cb0c5f8d373ed4f7`;
manifest remains `59c2baa52bf73f7ac0e97278eebd3982e91c0e4af8267aea41b1e1c35fc88178`;
its lock remains `ba55edbf116199e6500be7d8ffcc87abc762111e83cd90f8859050569a2e3b85`.
Its separate lock/profile binds the same actual GTS path with opt-level 3,
assertions and overflow checks. It is not another workspace dependency gate.
Production verifier/Debug source was unchanged between the successful probe
and final test-input-only refinement, so that original probe result applies.

One correction-development clippy run failed on assertions lacking useful
failure diagnostics. They now print the actual result/diagnostics without any
lint suppression; failed `raw/T4-R1-clippy.log` is preserved. Current lint/census
and runtime logs qualify the corrected final source.

`raw/T4-R1-identity.log` records unchanged rustc nightly
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1;
Cargo `1.100.0-nightly (7941be6fb 2026-09-11)`; Node `v26.10.0`;
wasm-bindgen CLI `0.2.125`; native x86_64 and actual wasm32 targets.

The original complete GTS 274-case run remains explicitly pre-correction
regression evidence. Its unchanged crypto/codec/primary-fixture observations,
eleven-group composite suite, shard inventory and unchanged consumer API
compilation are reused; none is renamed as a new whole-package run. Current
thirteen-group native/wasm caller tests requalify the changed verifier/result
assembly and SnapshotSigner. No API signature, consumer body, dependency,
fixture or harness registration changed in this correction. Full-workspace,
hosted CI, hooks, signing, push/publication, Task 5 and certification remain
unrun here. Exact-source independent re-review is required next.
