<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 2 implementation: native pure ML-DSA-65

Status: SUCCESS

Task scope is implemented and focused checks are terminal/pass. Independent
task/security review, signed commit with normal hooks, push and publication
belong to the parent workflow and have not been performed by this implementer.
This report does not declare the composite issue or Stage 1 complete.

## Identity and authority

Repository root `/home/paudley/Active/purrdf`; worktree
`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`;
branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
HEAD remains signed/pushed Task 1 commit
`74bf968ce28da00f9ab6ad054b150c4f5d91da18`. Task 2 source is uncommitted;
the index was not modified. Captured base is
`ce3c07192aba1e36666062c00f958670a827cfb5`.
Authoritative plan SHA-256 is
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

Read worktree AGENTS, parent `.baseline` and `.goals`, clean deficiency ledger,
the authoritative plan/completeness contract and its independent PASS review,
issue analysis/prior-art assessment, helper/layer declarations, Stage 1 and
Stagectl quality/validation/delegation/no-deferral instructions. Implementation
was the sole source mutation. No delegation, commits, pushes, forge posting,
main/sibling changes, model actions or hook invocation/bypass occurred.

Source identity is captured in `raw/T2-files.sha256` (17 source/document/fixture
files), manifest SHA-256
`feb341c14e7e8887ad2497479d8c8a435c5e32ae608cdd15e74a6c85e7938ce4`.
The complete HEAD-to-working-source patch, including all ten new files via
`git diff --no-index /dev/null`, is `raw/T2-source.diff`, SHA-256
`3bfdf38748668d0941d28f60c4cc706a0f13f2e9073056c8c026b06e8a22178b`.
`raw/T2-manifest-check.log` verifies every file against that manifest. Stage
evidence is separate and was not swept into the patch or index.

## Implemented behavior

`purrdf_gts::mldsa65` is one original first-party implementation of FIPS 204
pure-message ML-DSA-65. Public APIs provide `SigningKey::from_seed`, validated
`from_expanded_bytes`, caller-storage `export_expanded`, a borrowed derived
`verifying_key`, explicitly named `sign_deterministic`, caller-randomizer
`sign_hedged`, exact-length `VerifyingKey::from_bytes`/`verify`, and canonical
`Signature::from_bytes`/`as_bytes`. Errors distinguish invalid keys/signatures,
context lengths, rejection sampling exhaustion and signing nonce exhaustion.
Secret Debug exposes only the public key; Clone explicitly creates another
owned secret with the same drop discipline.

The implementation covers FIPS key expansion, A/s/mask/challenge sampling,
NTT/inverse/point products, rounding/decomposition/hints, canonical bit packing,
pure formatting `0x00 || len(context) || context || message`, all signing
rejection checks and strict verification. Parameters are exactly ML-DSA-65;
NTT roots derive in const Rust from `1753^BitRev8(i) mod 8380417`, without a
copied table. SHAKE uses the already qualified `purrdf_hash::sha3` home.
No runtime dependency, lockfile, layer graph, semantic feature or compiler/profile
override was added. Unsafe code remains forbidden.

Expanded-secret import rejects every eta-4 forbidden encoding, then derives
`A*s1+s2` to validate the stored t0 and derived public-key hash. Independent
private K is arbitrary key material, as specified; the module documents that it
cannot be derived from the public key. Public keys have exactly 1952 bytes,
and every ten-bit coefficient encoding of that length is legal under FIPS.
Signatures have exactly 3309 bytes, strictly bounded responses, ordered unique
hint indices, nondecreasing bounded cumulative offsets and zero unused hint
storage. Decoding is distinct from successful authentication.

The production primitive receives 32 fresh cryptographic bytes from the caller;
it performs no host entropy/clock lookup. Deterministic signing explicitly
supplies the standard zero randomizer through the same signing body. Contexts
over 255 bytes fail before secret decoding. Sampling is bounded at 2^20
candidates per polynomial/challenge. Signing reserves disjoint five-polynomial
nonce groups and returns a typed error before using an incomplete or overflowing
namespace. No partial signature is returned after failure.

Secret clearing and full-scan byte comparison are reused from the existing
Ed25519 ct module, now publicly re-exported as `wipe_secret` and
`constant_time_eq`. Byte comparison explicitly rejects unequal public lengths
before scanning, preserving Ed25519's existing equal-length semantics. The
helper ledger registers ML-DSA and those two shared homes; AGENTS lists them.

## Independent fixture evidence

Official NIST ACVP bytes are selected from commit
`975de31eb83d87039ec88934fdc47d8c312b892d`, joining prompt/results by group/case
identity. The one-time Rust importer is retained as
`raw/import-mldsa-fixtures.rs`; its actual run is `raw/T2-fixture-import.log`.
It uses the existing lexical JSON, base16 and testkit Recorder homes. It was
removed from shipped examples after capture. No PurRDF output generated an
expected answer, and no external implementation source was imported.

The fixture provenance records all six source hashes, pinned primary URLs,
record fields/lengths, exact selected groups and complete results. The entire
upstream NIST notice is retained in `NIST-NOTICE.txt` (read from the pinned
README, captured as `raw/ACVP-README.md`, SHA-256
`d5a569884ee83bd1c4737042d0a2cc7d68c6950690f75a73ef14f505a9aa3555`).

| Frozen fixture | Complete official cases executed natively and in wasm |
|---|---|
| `keyGen.txt` | Group 2, 25 cases: complete 1952-byte public AND 4032-byte expanded-secret outputs |
| `sigGen.txt` | Pure external groups 3/15, 15 deterministic + 15 hedged complete 3309-byte signatures |
| `sigVer.txt` | Pure external group 3, 15 verdicts: 3 valid and 12 invalid |

All 70 official cases match. HashML-DSA/preHash groups, including verification
group 4, and internal-interface groups are not included in the pure external
claim. Fixture files are outside governed root `vectors/`; no GTS wire corpus
was regenerated or changed.

Public tests also check malformed lengths, mutated public/signature sections,
message/context changes, 255/256-byte context boundaries, valid varied hedged
outputs, empty-message signing, expanded-key invariant corruption, all forbidden
eta nibbles, signature norm neighbors on both signs and sparse hint canonicality.
An entropy/clock-sealed test successfully generates a key and signs/verifies
through the public APIs in the actual wasm runtime. Mathematical unit tests
compare full NTT products to an independent negacyclic integer oracle and
exhaustively check rounding/decomposition over all q residues. Starved samplers
and exhausted/overflowing nonce counters produce the actual typed error path.

## Actual final checks

Every command below was executed with explicit worktree cwd and terminal exit 0.
Rust compiler: `rustc 1.100.0-nightly (4b6d04e70 2026-09-13)`, full commit
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1, native host
`x86_64-unknown-linux-gnu`. The repository's optimized dev/test profiles,
debug assertions and overflow checks were preserved. `CARGO_BUILD_JOBS=4`
only capped parallelism. No warning/assertion/overflow/feature weakening occurred.
Wasm ran under Node `v26.10.0` and exactly pinned wasm-bindgen `0.2.125`.

| Command | Result / evidence |
|---|---|
| `cargo test --locked -p purrdf-gts -p purrdf-ed25519` | 294 passed cases across 28 runner groups, including 9 doctests; no ignored/failing cases; `raw/T2-tests-complete.log` |
| `cargo clippy --locked -p purrdf-gts -p purrdf-ed25519 --all-targets -- -D warnings` | Warning-free; `raw/T2-clippy-qualified.log` |
| `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$WORKTREE/scripts/wasm-test-runner.sh" cargo test --locked --target wasm32-unknown-unknown -p purrdf-gts --test mldsa65` | Six public tests really executed, all 70 NIST cases plus refusal/entropy-sealed behavior; 6 passed, 0 ignored; `raw/T2-wasm-runtime-qualified.log` |
| `cargo build --locked --target wasm32-unknown-unknown -p purrdf-gts -p purrdf-ed25519 --lib` | Both affected libraries build; `raw/T2-wasm-build.log` |
| `python3 scripts/check-shared-helpers.py` | Existing wrapper runs Rust census: 80 enforced jobs, 91 distinct rows, 1813 files, no open copies; `raw/T2-helpers-qualified.log` |
| `python3 scripts/check-test-shards.py` | Six feature-unified shards cover 42 members; new explicit `mldsa65` test is in integration-2 by existing partition; `raw/T2-shards.log` |
| `cargo fmt --check -p purrdf-gts -p purrdf-ed25519` | Pass; `raw/T2-format.log` |
| `git diff --check` | Pass; `raw/T2-diff-check.log` |
| `sha256sum -c raw/T2-files.sha256` | All 17 identities match; `raw/T2-manifest-check.log` |

Initial development errors are retained, never treated as qualification: wrong
Debug macro syntax; use of a private vector decoder instead of its public
`decode_str`; two clippy idioms; numeric rather than string ledger constant
syntax; and a misplaced assertion when adding an overflowing nonce test. Each
was corrected in code/tests/schema, with no disabled gate or softened official
answer. Final logs above supersede the development/failing logs. There was no
official cryptographic-answer mismatch requiring an algorithm correction.

No full-workspace `make check`, commit/push hook, hosted CI, composite/COSE,
writer, compaction or RDF-certification qualification was performed by this
task implementer. Those are distinct parent gates/other approved tasks. Hash
substrate source was unchanged; its Task 1 evidence remains attributable to
its original identity rather than claimed as a new Task 2 hash execution.

## Clearing and timing limits

`SecretBytes` owns the expanded key, seed expansion and signing private seed,
serialized masks/commitments. `SecretPolys` owns decoded/transformed secrets,
derived low bits, masks, products, responses, residuals, hints and partial
challenges. Normal return, rejection, import failure and sampler/nonce errors
drop these guards and default-overwrite their controlled storage through the
one existing helper, observed by black_box and a compiler fence. Caller-owned
seed/randomizer/export copies are explicitly the caller's responsibility.
SHAKE internal absorber/reader states are unchanged and NOT cleared here.
Compiler-created copies, registers, stack spills and historical moved storage
are not guaranteed cleared; no stronger safe-Rust elimination claim is made.

Arithmetic/norm/packing source uses fixed schedules and no private coefficient
index or scan early exit. FIPS samplers and signing retries are variable-time,
and challenge sampling follows derived challenge positions. The source-level
design and known answers are functional evidence, NOT measured proof against
compiler/JIT/hardware timing. Constant-divisor remainder lowering is target
dependent. Independent security review is required by the parent task gate;
this implementation report does not substitute for that review or cryptographic
certification.

## Exact changed files and disposition

Tracked changes: `AGENTS.md`, `helpers-ledger.toml`,
`crates/ed25519/src/{ct,lib}.rs`, `crates/gts/Cargo.toml`,
`crates/gts/README.md`, `crates/gts/src/lib.rs`.
New files: `crates/gts/src/mldsa65/{codec,math,mod,sampling}.rs`,
`crates/gts/tests/mldsa65.rs`,
`crates/gts/tests/mldsa65/{PROVENANCE.md,NIST-NOTICE.txt,keyGen.txt,sigGen.txt,sigVer.txt}`.
No staged files or unrelated source changes. Task 2 implementation blockers:
none. Required independent review is awaiting parent dispatch; no PASS verdict
or signed source commit is fabricated by this report.
