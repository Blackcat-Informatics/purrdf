<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 4: Hedged Writer and algorithm-aware file resolution

Status: SUCCESS

The approved Task 4 implementation and focused qualification are complete.
Source mutation has stopped for independent review. This report does not claim
independent review, commit, push, forge publication, Task 5/6 or issue completion.
No child delegation, index mutation, commit/hook invocation, forge mutation,
protected-main edit, sibling mutation or model/service action occurred.

## Exact source and authority

Worktree: `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Unchanged HEAD: `31498279c83c7fb9c3c6d97faffc1628a82452e1`.
Captured base: `ce3c07192aba1e36666062c00f958670a827cfb5`.
Approved plan SHA-256:
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
Parent `.baseline`/`.goals` govern; selected Stage evidence stays separate from
source. The deficiency ledger still has only its notice and marker.

Final complete ten-file manifest: `raw/T4-files.sha256`, SHA-256
`5bf4fe0e02b17147d476fb1122ec22b911ff08b17551e651a2a2b0438103c5a9`.
Complete HEAD-to-source patch, including both new files:
`raw/T4-source.diff`, SHA-256
`4c52edc05e7448779175aedc181c8b04b11880a8370eed0e02077ea0695a1931`.
`raw/T4-manifest-check.log` verifies every final file.

Changed tracked files:

- `crates/gts/Cargo.toml`
- `crates/gts/src/cose.rs`
- `crates/gts/src/cose/sign1.rs`
- `crates/gts/src/verify.rs`
- `crates/gts/src/writer.rs`
- `crates/gts/tests/cose_composite.rs`
- `bindings/python/src/py_gts.rs` (module documentation only)
- `crates/rdf/src/gts_write.rs` (module documentation only)

New files:

- `crates/gts/tests/hedged_writer.rs`
- `crates/gts/tests/composite_support/mod.rs`

No dependency, lockfile, semantic feature, primitive, frozen root vector, pinned
IETF/NIST fixture, helper exemption or generated artifact changed.

## One Writer, explicit fallibility

`Writer<M = Infallible>` has one frame-construction/append body. Its sealed mode
controls only the simple conveniences' return contract. Existing unsigned/Ed25519
callers keep `Vec<u8>` conveniences and the existing advanced fallible entry point.
`with_composite_signer(key, opaque_kid, provider)` consumes that same Writer into
`Writer<Hedged<P>>`, preserving buffer, head, offsets, types, frame IDs, catalogs
and dictionaries. Every simple `add_*` method then returns `Result<Vec<u8>,
WriterError>` through the shared body. Rotation back to Ed25519 retains this
fallible mode. Composite installation is unavailable in the default mode, so
new provider/native failures cannot reach its existing `expect` boundary.

This small type boundary avoids changing the many real existing GTS/RDF/binding
convenience consumers, while making failure visible for actual composite callers.
It is not a second writer or parser. Constructors remain the existing default
mode; actual production appending calls the hedged COSE API.

`RandomnessProvider` is caller-owned and fallible, also implemented for `FnMut`
providers. Conversion requires a provider at compile time; a compile-fail doctest
actually establishes omission is rejected. There is no optional provider,
system RNG, clock, automatic generation or deterministic composite fallback.
Runtime missing entropy is the provider's actionable `RandomnessError`, retained
as `WriterError::Randomness` with its error source. No runtime absent-provider
configuration is claimed to have been exercised: it is unrepresentable through
public configuration. `MissingRandomnessProvider` is a defensive internal error
for the sealed default mode, whose public key-installation invariant excludes
composite keys.

Each newly authored composite signature calls the provider once for a fresh
32-byte randomizer. Its owned RAII guard calls the existing native `wipe_secret`
home on normal, failed-provider/partial-fill and native-signing error exits.
Native Sign1 refusal propagates as `WriterError::Signing`, preserving its source;
no native-budget exhaustion was artificially forced through Writer. Existing
primitive budget tests run in the affected package. Signing completes before
any output/head/index mutation. Provider quality, full successful fill, freshness
and component-key dedication remain the caller's stated cryptographic contract;
fixed draws in tests are explicitly fixture inputs.

Ed25519 uses the same typed COSE signing body and does not request randomness.
Explicit carried `FrameOptions::signature` retains its existing meaning. OpenPGP
authoring/discovery remains real Ed25519, with no composite armor invention.

`SnapshotSigner` now has public-only Debug and clears its owned seed on Drop.
Its manual Clone completes allocating public strings before initializing the
new Drop-protected seed owner. Snapshot metadata fields move with `mem::take`
so this ownership cleanup compiles through the actual snapshot authoring path.
These guarantees concern controlled owned storage, not registers, compiler
copies, historical stack copies or aborting processes. Partial-fill refusal is
executed; post-drop memory inspection is not claimed.

## Shared algorithm-aware verification and file integrity

`verify_signatures_with_resolver` supplies exact `Option<&[u8]>` identifiers and
the declared algorithm to borrowed typed key resolution. The legacy String/Ed
resolver and this new API share one strict parse/status/verification loop.
Malformed and unsupported signatures are Invalid before lookup; supported
unresolved signatures are Unverified; wrong key types are Invalid. Binary IDs
do not become invented text, and absent IDs do not become empty IDs.

`SignatureKeyring` makes `verify_file_with_keyring` generic over the same resolved
path. Existing `HashMap<String, EdVerifyingKey, S>` consumers retain compatibility.
The new fixed-hasher `Keyring` stores exact byte IDs and a separately configured
absent-id key. Its `VerificationKey` enum borrows typed COSE keys; composite keys
are boxed to keep entries small. Owned enum conversions use `variant_from!`.
Embedded/out-of-band OpenPGP resolution still folds into this same core.

Actual reader inspection revealed that damaged frames can disappear from folded
signature rows. Valid survivors could consequently conceal corrupt content.
The shared verifier now converts actual `DamagedFrame`, `BrokenChain`,
`TruncatedLog`, `TornAppendError` and `ResourceLimit` diagnostics into file errors.
Cryptographic counts and original diagnostics remain available. Reader diagnostics
have no severity field: policy Severity is a separate type. `MissingKey` and
`UnknownCodec` remain compatible with authenticating opaque bytes. The unsigned
opt-in path reuses the same integrity predicate rather than accepting damage.

## Executed public acceptance

The new ten-group native/wasm harness demonstrates:

- Complete frozen `vectors/signed/basic.json` file reproduced byte-for-byte by
  the actual Ed25519 Writer, not merely the primitive helper.
- One actual file with Ed/composite/Ed/composite rotation, including opaque
  binary ID and a signed MMR/index footer; reader and typed file keyring accept
  all four signatures. Composite envelope/protected -58/3373-byte signature
  shape is checked through the strict parser.
- Same frame identity under distinct explicit randomizers produces different
  valid composite signatures. This whole authoring/verification test runs under
  the wasm host clock/entropy seal.
- All thirteen actual append entry points refuse unavailable entropy with
  unchanged bytes/head. Partial-fill provider failure also refuses atomically;
  recovery writes a usable four-signature file with correct index count and MMR,
  demonstrating failed calls did not pollute offsets/types/frame IDs.
- Corrupting either component independently in Writer-produced frames yields
  one valid survivor and one invalid signature. Changed content is refused even
  when its damaged frame is withheld and another signature remains valid.
- Damaged header, validly re-signed broken chain and torn trailing CBOR all
  refuse with two valid cryptographic signatures still reported. Signed opaque
  Encrypt0 remains accepted with a genuine MissingKey diagnostic.
- Malformed/unsupported envelopes never call a panic-on-lookup keyring;
  supported unknown keys remain Unverified and wrong key type becomes Invalid.
- Empty/absent IDs resolve separately; binary IDs reach the borrowed resolver
  exactly, without fabricated text.
- Snapshot secret redaction/owned Clone behavior, and real embedded/out-of-band
  OpenPGP verification, including integrity refusal and clean unsigned opt-in.

The existing eleven-group public composite suite also executes against the new
single shared fixture-key factory. Initial census correctly refused duplicate
factory bodies; they were consolidated, not exempted. No expected crypto answer
was generated from PurRDF.

## Terminal checks and evidence

All commands ran in the worktree. Compile concurrency was capped with
`CARGO_BUILD_JOBS=4`; repository compiler, profiles, assertions, overflow checks
and warning policies were preserved. Final commands returned exit 0:

| Command | Artifact and result |
|---|---|
| `cargo test --locked -p purrdf-gts` | `raw/T4-native-complete.log`: 274 passing cases across 25 runner groups, including ten doctests (one compile-fail), old frozen/Encrypt0/compaction/authoring regressions and both public suites |
| `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$WORKTREE/scripts/wasm-test-runner.sh" cargo test --locked --target wasm32-unknown-unknown -p purrdf-gts --test hedged_writer --test cose_composite` | `raw/T4-wasm-complete.log`: both suites actually execute in wasm/Node, ten Writer/file groups and eleven composite groups pass |
| `cargo clippy --locked -p purrdf-gts --all-targets -- -D warnings` | `raw/T4-clippy-final-qualified.log`: warning-free final source |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps -p purrdf-gts` | `raw/T4-docs.log`: strict public API documentation/linkage passes |
| `cargo check --locked -p purrdf-rdf -p purrdf-wasm -p purrdf-capi -p purrdf-python` | `raw/T4-consumers-complete.log`: actual existing shipping library consumers compile; no binding runtime assertion |
| `python3 scripts/check-shared-helpers.py` | `raw/T4-helpers-complete.log`: existing wrapper's Rust census passes, 81 enforced jobs, 91 distinct rows, 1818 source files |
| `python3 scripts/check-test-shards.py` | `raw/T4-shards-complete.log`: six feature-unified shards cover all 42 workspace members, including the new harness |
| `cargo fmt --check -p purrdf-gts` | `raw/T4-format.log`: final GTS source passes |
| `cargo fmt --check -p purrdf-gts -p purrdf-rdf -p purrdf-python` | `raw/T4-consumer-docs-format.log`: final narrow consumer documentation cleanup passes |
| `git diff --check HEAD` and separate new-file `git diff --no-index --check /dev/null NEWFILE` | `raw/T4-whitespace.log`: tracked and both new files covered; no-index 0/1 accepted only with empty diagnostics |
| Complete file-hash readback and added-line marker scan | `raw/T4-manifest-check.log`, `raw/T4-deferral-scan.log`: all ten identities match; no added TODO/FIXME/XXX markers |

`raw/T4-identity.log` records rustc nightly
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1;
Cargo `1.100.0-nightly (7941be6fb 2026-09-11)`; native
`x86_64-unknown-linux-gnu`, wasm `wasm32-unknown-unknown`; Node `v26.10.0`;
wasm-bindgen CLI `0.2.125`. Final source capture followed terminal qualification
and the final prose-only cleanup on 2026-10-06 UTC.

Parent inspection required removing development identifiers `(Task 8 / C7)`
and `C0.6` from the two actual inspected consumer module documents, under the
standing baseline. Useful explanation and links are retained. Only those doc
comments changed after the compiler/runtime checks; final `raw/T4-consumer-docs.diff`
SHA-256 is `5b73eebc814147e143fc6c083719cc86be3cca20e65ebc46c3a338bb1028cc43`.
The eight GTS hashes are byte-identical to their qualified identities, verified
with `cmp`. Earlier complete compiler closure is reused for these unchanged
consumer implementations/APIs; it is not relabeled as a post-doc compile run.
The prior eight-file manifest/patch remain `raw/T4-pre-doc-files.sha256`
(`fdd4db269810b9b7b71051676c50892191e27db5be2f78f9106b584cc6f31293`)
and `raw/T4-pre-doc-source.diff`
(`b3de05d005b09b54befbd88b1c6dc849c38c874e63f226469e4d4f7d49ec33c2`).

Development failures remain preserved in `raw/T4-*development*.log`, the initial
failed census, and failed macro/refinement lint logs. They include oversized
signer enum variants, moving a Drop-managed CBOR Value in a test, test semicolons,
redundant Clone, duplicate fixture factory, incorrect macro conversion attempts,
an unused import and excessive test-support visibility. All were corrected
without bypass, suppression or gate alteration. Earlier passing executions
are historical; final complete native/wasm/lint/census runs qualify the frozen
implementation after the ownership and shared-factory refinements.

Unchanged primary fixture provenance and SHAKE/ML-DSA cryptographic observations
remain their prior task evidence; current public suites replay the fixture
answers. No full-workspace gate, hosted CI, commit hooks, signing, push,
publication, Task 5 composite compaction/certification, external vector refresh,
hardware timing certification or FIPS certification is claimed. Independent
review and parent normal signed transport are the next required boundary.
