<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 3 R1 implementation: content-type header grammar

Status: SUCCESS

T3-R1 is implemented and the affected qualification is terminal. Source is
frozen for independent re-review. This status covers the required implementation
and checks, not independent review, commit, publication or whole-issue completion.
The original review remains a historical BLOCKED verdict until the independent
reviewer assesses this new identity. No source delegation, commit, push, forge
post, hook invocation, main/sibling mutation or real-index mutation occurred.

## Exact identity and preserved history

Worktree `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`;
branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`;
unchanged HEAD `bf7d5d14ff72f4e6a21ea6efc2b58680f4fdf33f`;
captured base `ce3c07192aba1e36666062c00f958670a827cfb5`;
plan SHA-256 `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
All checks had terminal results by 2026-10-06 11:35 UTC.

Final complete 13-file manifest `raw/T3-R1-files.sha256`, SHA-256:
`54d8e8204b39b971126d9398f0a3d216175edc1b9ee0ca309356af01ce67c0a6`.
Final complete HEAD-to-source patch `raw/T3-R1-source.diff`, SHA-256:
`cccbf183ac6de2283855cf38065511e904743fdbec92ea42bda836e5f6f75022`.
Separate initial-to-R1 delta `raw/T3-R1-delta.diff`, SHA-256:
`64675af4e09ccfc3a00d0988f2d3a2d2cbe5addb2abe8779c278b6aae791d5f2`.
`raw/T3-R1-manifest-check.log` reads back and verifies all 13 current files.

Only `crates/gts/src/cose/sign1.rs` and
`crates/gts/tests/cose_composite.rs` changed from the independently reviewed
Task 3 identity. No source file or dependency was added for R1. The remaining
11 entries are byte-identical to the initial manifest. The complete manifest
still covers all seven tracked and six new Task 3 source/doc/fixture paths;
selected Stage evidence is excluded from source.

Initial `raw/T3-files.sha256` retains SHA-256
`b4d3c20a6a4529ba876fcdc2fb1c2623a826afaf7a35ca6ec6a688f16e4e680b`;
initial `raw/T3-source.diff` retains SHA-256
`62741e8be9088b900538201b17815e33ee058cb61db7c151271052fc4343b70d`.
Both were preserved, as were the original report and independent review.
For the delta, the two initial new-file bodies were reconstructed from that
complete initial patch into `raw/T3-R1-initial/`; their hashes were verified
against the initial manifest before no-index comparison. No index was used.

## Finding and correction

Read the full `tasks/T3-review.md` finding and captured primary RFC 9052 §3.1
(`raw/rfc9052.txt`, lines 699–708) and RFC 6838 §4.2
(`raw/rfc6838.txt`, lines 419–431). RFC 6838 capture SHA-256:
`b08ccba7e5116e61085f2e1fe447d90eee785fb0efaa448a4b4ef6ea48b03b80`.
The COSE text syntax is exactly a type-name/subtype-name pair. Each name is
1–127 ASCII bytes, starts with ALPHA/DIGIT, and continues with ALPHA/DIGIT or
`! # $ & - ^ _ . +`. RFC 9052 separately forbids edge whitespace. Its registry
reference to parameters/subparameters does not extend the specified pair
grammar, so parameter-bearing strings are refused.

Searched existing lexical/IRI/GTS and workspace grammar homes before adding
the small original private validator. No RFC 6838 restricted-name validator
exists. The helper-ledger media-type-by-extension home maps extensions rather
than parsing syntax. `sparql-eval::protocol::parse_media_range` is an HTTP Accept
negotiator with trimming, wildcards and parameters; JSON Schema's
`is_json_media_type` identifies a JSON essence with parameters. Neither provides
this grammar or an allowed lower-layer home. No body was copied or dependency
added.

The existing `header_labels` path now checks label 3 through one validator for
both protected and unprotected maps. Nonnegative integer content formats remain
accepted. Text is admitted only when both slash-separated names satisfy the
exact restricted-name grammar. Additional slashes, whitespace, parameters,
non-ASCII and invalid first/continuation bytes fail. The parser neither trims
nor changes case and preserves the exact received protected bytes. Invalid
content type yields `Sign1Error::Malformed` before key lookup and becomes
`SigStatus::Invalid` in the real legacy resolver path.

## Public behavior and terminal qualification

Added the registered harness group
`content_type_headers_obey_restricted_names_before_lookup` to the existing
native/wasm public test target. It exercises both header buckets:

- 20 correctly signed accepted cases: eight textual neighbors and two integer
  formats in each bucket. These include mixed case, digits, every allowed
  continuation punctuation, a 127-byte type, a 127-byte subtype and both at
  that boundary, plus zero and `u64::MAX`. Public typed and resolver verification
  pass, with exact protected bytes unchanged.
- 162 rejected cases: 78 malformed text neighbors plus negative integer,
  byte string and null per bucket. Text covers missing/empty names, forbidden
  initial punctuation, edge/internal whitespace, controls/DEL, bad ASCII
  punctuation, extra slash, non-ASCII, parameters and 128-byte names. Public
  parse and supplied-key verification reject; resolver closure panics if called,
  establishing malformed-before-lookup through the actual public caller.

All listed commands ran in the worktree with `CARGO_BUILD_JOBS=4` where compiling.
All final commands returned terminal exit 0:

| Command | Evidence and result |
|---|---|
| `cargo test --locked -p purrdf-gts` | `raw/T3-R1-native.log`: 263 passing cases across 23 runner groups, including 11 public composite groups and nine doctests; existing Encrypt0/frozen Ed/writer/compaction regressions pass |
| `cargo test --locked -p purrdf-gts --test cose_composite` | `raw/T3-R1-public-native-qualified.log`: all 11 groups pass after the final test-only lint/comment edits |
| `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$WORKTREE/scripts/wasm-test-runner.sh" cargo test --locked --target wasm32-unknown-unknown -p purrdf-gts --test cose_composite` | `raw/T3-R1-wasm-qualified.log`: all 11 groups actually execute and pass in wasm/Node; library/test compilation is part of this command, not the sole evidence |
| `cargo clippy --locked -p purrdf-gts --all-targets -- -D warnings` | `raw/T3-R1-clippy-qualified.log`: warning-free on the frozen source |
| `cargo fmt --check -p purrdf-gts` | `raw/T3-R1-format-qualified.log`: passes on frozen source |
| `python3 scripts/check-shared-helpers.py` | `raw/T3-R1-helpers-qualified.log`: existing wrapper executes Rust census; 81 enforced jobs, 91 distinct rows, 1816 source files, no open copies or escaping includes |
| `git diff --check HEAD` plus individual `git diff --no-index --check /dev/null NEWFILE` for all six new paths | `raw/T3-R1-whitespace.log`: tracked patch and every new source/fixture/provenance/notice file pass; no-index status 0/1 accepted only with empty diagnostic output |
| `sha256sum -c raw/T3-R1-files.sha256` | `raw/T3-R1-manifest-check.log`: all 13 exact source hashes pass |

`raw/T3-R1-identity.log` records actual tools/source/plan. Rustc
`1.100.0-nightly (4b6d04e70 2026-09-13)`, full commit
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1;
Cargo `1.100.0-nightly (7941be6fb 2026-09-11)`;
native `x86_64-unknown-linux-gnu`; wasm `wasm32-unknown-unknown`;
Node `v26.10.0`; wasm-bindgen CLI `0.2.125` verified by the actual runner.
Repository opt-level-3 dev/test profiles, debug assertions, overflow checks,
warning policy and semantic-feature prohibition were preserved. Jobs capped
parallelism only. Complete added-line scan has no TODO/FIXME markers
(`raw/T3-R1-deferral-scan.log`).

The first R1 clippy run failed on an empty string built with `to_string` in the
new test. It was corrected to `String::new`, without lint suppression. Failed
`raw/T3-R1-clippy.log` is retained and never qualifies a pass. A test comment
was clarified to describe representative forbidden bytes. Full-package coverage
preceded those two test-only edits; the affected public suite, clippy, formatting
and helper census were then rerun on final source. Runtime validator behavior
was unchanged after the full-package pass.

Unchanged declaration gates (domain registry, shards, layers, banned dependencies),
the initial independent IETF provenance audit, unaffected SHAKE/ML-DSA substrate
qualification, separate initial wasm-library build and initial RDF consumer
compile remain their explicitly identified earlier evidence. They are not
relabeled as new R1 runs. The current public suites also rerun the complete
published IETF fixture and frozen Ed Sign1 groups, but the primary fixture itself
is unchanged. R1 changes no public API, dependency edge, fixture or cryptographic
primitive; additional consumer compilation is not invalidated. No broad workspace
gate, hooks, commit/push, hosted CI or Task 4/5 caller work was performed here.
Required independent narrow re-review and parent normal signed hooks remain next.
