<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 5 correction handoff

Status: SUCCESS

Both required findings T5-R1 and T5-R2 are corrected and their local acceptance
checks pass. Source mutation has stopped. No index mutation, commit, push,
forge post, child delegation, dependency/feature/exemption change, private
memory write, or model/service/lifecycle action occurred. This report is the
implementation handoff for independent review; it makes no PR, hosted CI,
whole-issue gate, release or merge claim.

## Identity and complete artifacts

Worktree: `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Unchanged HEAD: `39f0d4dce74c7635a8d4675406cc37eb763cb590`.
Captured base: `ce3c07192aba1e36666062c00f958670a827cfb5`.
Approved plan SHA-256:
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
Stage: `.stage/purrdf-gts-composite-ml-dsa-65-ed25519` within this worktree.

| Artifact | SHA-256 |
|---|---|
| `raw/T5-correction-files.sha256` — all 13 tracked/new changed source files | `38580bdbbf4209b77c08a608574a4fcb288c50c09c0f545d6a59d837dd430037` |
| `raw/T5-correction-source.diff` — complete tracked/new patch against HEAD | `cb4aa57335b216f673870c0cde85974749e95043b9dedd98a5b38a957586c8a9` |
| `raw/T5-correction-delta.diff` — complete four-file correction against initial Task 5 source | `96674603f12903c59b9d51ef68b7a366409c69a98662ba9ee98ac5577ecfe832` |
| preserved initial `raw/T5-files.sha256` | `4b6369ae05670afeb0d11f0a566e75229075831308c2c7253ea760cd72b6d1ff` |
| preserved initial `raw/T5-source.diff` | `b74ce609f9ef484c131ff0f9e2cb572275a1677446ed3d5660dbaccae3c983b6` |
| unchanged independent `raw/T5-review-shape-probe.rs` | `77d97a275ea5eacf6785da4b91493d44373c190430b25c1a423cef8cc4a322b8` |

Initial source bytes are preserved at `raw/T5-initial-source/` for the delta.
`raw/T5-correction-artifacts.sha256` records the artifact hashes;
`raw/T5-correction-logs.sha256` records terminal log hashes.
`raw/T5-correction-manifest-check.log` verifies all 13 current files.
Read-only `git apply --reverse --check` passes for both complete patches,
including the new test file. The index remains empty.

The correction changes only:

- `crates/gts/src/compact.rs`
- `crates/gts/src/reader.rs`
- `crates/rdf/src/gts_certify.rs`
- `crates/rdf/tests/gts_composite_compaction.rs`

The complete manifest also retains the initial Task 5 modifications to
`crates/gts/src/{cose/tests.rs,fixture.rs,model.rs,policy.rs}`,
`crates/gts/tests/{compaction_signatures.rs,composite_support/mod.rs,cose_composite.rs}`,
`crates/rdf/Cargo.toml`, and `crates/rdf/tests/gts_certify.rs`.

## R1: semantic provenance and verbatim authored index preservation

Moved the existing closed class/predicate vocabulary from RDF projection to
the single GTS `compact::ProvenanceSubjects` implementation. RDF projection,
the eager reader and the evented reader now use that shared rule. Evented
folding retains per-subject classification facts, rather than materializing
the content quads. Those facts remain local to the segment that owns a frame.

A Compaction positive anchor requires a blank-node subject, a single reserved
class, a closed predicate vocabulary, singleton string agent, singleton
`xsd:dateTime` timestamp, and literal `blake3:<32-byte hex>` source heads.
Ordinary content with foreign predicates, a literal class name, or any
incomplete combination of mandatory fields remains content and its authored
index signature remains in the detached authorship union.

The additional type-only neighbor was executed against the first correction
and failed: `raw/T5-correction-type-only-before.log`, exit 101, one actual
failure (eight other groups filtered). This observation is owned by this task,
preserved, and fixed in the shared classifier. The final public regression
checks every incomplete mandatory-field combination and complete fields with
a foreign predicate or literal class. It checks duplicate vocabulary IDs,
both reader paths, exact original `(frame_id, COSE)` bytes through actual
composite packaging, Ed repack, and successful independent certification.

Genuine pack/repack and pack followed by fresh composite-authored streamable
history remain covered. The latter checks evented/eager signature agreement,
preserves all five original authorship pairs, and distinguishes the old root
from the new root after the new tail is incorporated.

## R2: strict current-subject root decoding

Removed the global lexical first-match helpers. One GTS decoder validates the
root records on genuine Compaction subjects using exact predicate IRI
identity, literal kind, at-most-one root per node and 32-byte hex encoding.
Source heads are literal `blake3:<hex>` values. The current rewrite is selected
by its complete actual pre-compaction segment-head list, preserving repeated
heads per source segment. An absent or ambiguous current event refuses
binding. Legitimate historical events may retain different earlier roots.

The same decoder serves `signatures_bound_ok`, certificate root extraction,
and the actual compactor's incoming-evidence gate. Malformed incoming roots
return an actionable refusal before any artifact is returned. Carried
signature crypto verification remains separate: malformed root structure
fails binding while an otherwise valid carried signature still verifies.
The actual final packaging index authentication remains a separate check.

Registered public regressions cover contradictory roots at equivalent
predicate IDs, blank-node root objects, malformed hex, an untyped unrelated
root subject, an unrelated matching root masking a wrong actual root, wrong
source-head selection, and an unused literal spelling the predicate IRI.
The reconstructed valid pack remains valid. Incoming malformed roots refuse
real `compact_and_certify`; differing legitimate historical roots survive.

## Executed final-source checks

All commands ran in the selected worktree with `CARGO_BUILD_JOBS=4`, unchanged
repository profiles and warnings denied. `raw/T5-correction-environment.log`
captures current toolchain, HEAD, branch, plan identity and profiles:
rustc `1.100.0-nightly`, commit
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1;
Cargo `1.100.0-nightly (7941be6fb)`; Node `v26.10.0`;
wasm-bindgen `0.2.125`. Native target is `x86_64-unknown-linux-gnu`;
wasm target is `wasm32-unknown-unknown`. Dev/test codegen is opt-level 3 with
debug assertions and overflow checks enabled. No profile weakening occurred.

| Command | Result | Terminal log under `raw/` |
|---|---|---|
| `cargo test -p purrdf-rdf --test gts_composite_compaction --test gts_certify --test streamable_vectors --test dict_vectors --test pinned_dict_compaction` | exit 0; 44 cases (9+17+4+11+3) | `T5-correction-terminal-rdf.log` |
| `cargo test -p purrdf-gts --test compaction_signatures --test pinned_dict_compaction --test cose_composite --test hedged_writer` | exit 0; 40 cases (6+10+11+13) | `T5-correction-terminal-gts.log` |
| `cargo test -p purrdf-gts --lib compact::` | exit 0; 12 cases, 108 filtered | `T5-correction-terminal-unit.log` |
| `cargo test -p purrdf-rdf --target wasm32-unknown-unknown --test gts_composite_compaction` with the existing `scripts/wasm-test-runner.sh` runner | exit 0; all 9 groups execute in actual Node/wasm | `T5-correction-terminal-wasm.log` |
| `cargo run --locked --offline --manifest-path .stage/purrdf-gts-composite-ml-dsa-65-ed25519/raw/T5-review-probe/Cargo.toml` | exit 0; original independent probe UNALTERED | `T5-correction-terminal-original-probe.log` |
| `cargo clippy -p purrdf-gts -p purrdf-rdf --all-targets -- -D warnings` | exit 0; affected generators/benches included | `T5-correction-terminal-clippy.log` |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p purrdf-gts -p purrdf-rdf --no-deps` | exit 0 | `T5-correction-terminal-docs.log` |
| `python3 scripts/check-shared-helpers.py` (existing Rust-backed gate) | exit 0; 81 jobs, 91 distinct groups, 1819 files, no copies/exemptions | `T5-correction-terminal-helpers.log` |
| `cargo fmt -p purrdf-gts -p purrdf-rdf -- --check` | exit 0 | `T5-correction-terminal-format.log` |
| `git diff --check HEAD` | exit 0 | `T5-correction-terminal-whitespace.log` |
| new-file `git diff --no-index --check /dev/null crates/rdf/tests/gts_composite_compaction.rs` | expected exit 1 for file difference; empty whitespace diagnostics | `T5-correction-new-whitespace.log` |
| current complete manifest check | exit 0, all 13 match | `T5-correction-manifest-check.log` |
| full patch and correction delta reverse applicability checks | exit 0 each, read-only | `T5-correction-full-patch-check.log`, `T5-correction-delta-check.log` |

The original probe's positive reconstructed pack has all six fields true.
Each malformed-root neighbor has binding false and all_ok false. Its ordinary
Compaction content has packaging=false, one original author pair, exact COSE
preserved in the resulting pack, and all six legitimate compaction fields true.
The expanded registered suite additionally exercises the missing positive
anchor and runs the full typed composite provider/signing/certifying path.

Routine compile/lint failures are preserved as
`T5-correction-original-probe.log` (missing Debug),
`T5-correction-probe-import-failure.log`,
`T5-correction-native-rdf-constructor-failure.log`,
`T5-correction-clippy-bit-test-failure.log`, and
`T5-correction-clippy-test-borrows-failure.log`. They were diagnosed from the
actual compiler output and fixed. Earlier intermediate successful runs are
preserved; only the `terminal` runs above qualify final changed behavior.
No newly added TODO/FIXME exists (no-match scan exit 1).

## Reused unaffected evidence and limits

Initial Task 5's layer and shard registration evidence remains applicable:
correction changes no manifest, dependency edge, target or shard registration.
Initial public documentation examples and missing-provider compile-fail
boundary are unchanged; current strict rustdoc recompiles the affected public
surface. Initial native WASM/C/Python consumer compile evidence is prior
source-compatibility evidence for their unchanged constructor/call surfaces,
not a new consumer runtime claim. The Signature metadata and its constructor
consumers remain unchanged by this correction. The earlier actual wasm
hedged-writer policy checks are prior evidence for unchanged policy/Term IRI
behavior; the current nine wasm public groups directly qualify the changed
reader/provenance/root behavior. Primitive crypto and frozen corpus bytes are
unchanged; the current frozen streamable/dictionary cases pass without edits.

No broad `make check`, whole-workspace wasm gate, hook/commit/push, hosted CI,
forge refresh or independent review was represented as executed here. Those
remain parent workflow boundaries. No Task 5 correction acceptance criterion
is deferred. Dedicated-key and caller entropy-quality policies are unchanged;
fixture draws are explicit test inputs, not an entropy-quality claim.
