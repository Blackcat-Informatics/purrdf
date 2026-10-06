<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Task 1 implementation review

VERDICT: PASS

Reviewed on 2026-10-06. No required Task 1 behavior is missing, and no actionable
correctness, wiring, provenance or repository-policy finding was established.
This verdict qualifies the SHAKE substrate task before commit; it is not an
overall composite-signature implementation, hook, hosted-CI or merge verdict.

## Identity and review authority

Worktree: `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
HEAD and captured base: `ce3c07192aba1e36666062c00f958670a827cfb5`.
Approved plan SHA-256:
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.
Complete source-diff artifact SHA-256:
`a6efa1d948137049c1b47e2504f45a5009d919aaae21112641b4de6e97a828d6`.

I independently checked both hashes and all ten current files against
`raw/T1-files.sha256`; every file matched. Read the actual six tracked-file
changes and four new test/fixture/provenance files, `plan.md`, the independent
plan review and implementation report, repository AGENTS.md and parent `.baseline`
and `.goals`, and the Stage 1/Stagectl quality, validation, delegation and
no-deferrals requirements. The emergency ledger has no entries. A focused
memory-registry search returned no relevant evidence.

Also inspected the unchanged shared BlockBuffer, existing SHA3 implementation,
digest-differential consumer, VectorFile validation implementation, crate
manifests, build profiles and integration-shard selection. The BlockBuffer,
SHA3 differential consumer and four SHA3 fixture files, root Cargo manifest
and Cargo.lock are unchanged against HEAD. There are no unexpected source edits.
No source, index, commit, remote or forge mutation was performed by this reviewer.
Only this review and the independent external-fixture verification artifacts
under the selected Stage directory were written.

## Task requirements and actual behavior

| Task 1 requirement | Evidence and assessment |
|---|---|
| Streamed SHAKE128/256 input using FIPS 202 rates and domain separation | `Shake<SECURITY>::RATE` is `200 - SECURITY / 4`, yielding 168 and 136 bytes. Both public aliases construct through the inline compile-time assertion allowing only 128/256. `update` calls shared `absorb`, which feeds the unchanged BlockBuffer and `absorb_blocks`; SHAKE uses suffix `0x1f`. |
| Separate absorbing and squeezing phases | Consuming `finalize(self)` returns `ShakeReader<SECURITY>`. Private state/position fields and no reader input/constructor API prevent safe clients from creating an invalid reader or absorbing into one. The public compile-fail doctest proves `reader.update(...)` fails. Clone preserves the position or partial input intentionally. |
| Correct incremental output and empty calls | Squeeze reads little-endian bytes from consecutive lanes and permutes only when position equals the rate and more output is requested. Rates are whole-lane multiples, so lane-sized copies cannot cross the capacity boundary. Empty output returns without permutation or position change, including at an exact rate boundary. There is no aggregate output-length counter to overflow. |
| Reuse the existing Keccak and buffer home | The permutation and all its generated constants are unchanged. Shared `absorb` and `finish_absorbing` replace the SHA3 inline bodies and serve both modes. There is one Keccak/block-absorption body, no copied constants, no extra runtime dependency and no semantic Cargo feature. The existing `sha3` helper-ledger row covers both public SHAKE types and both fixtures. |
| Meaningful public API known answers and streaming boundaries | The new external test calls `Shake::digest`, `new`/`Default`, `update`, consuming `finalize` and `squeeze` through the public crate API. Every split of each independent input, byte-at-a-time absorption, cloned absorber/reader snapshots, lane/rate output splits, eight output chunk sizes and empty calls are compared with full external expected bytes. All four NIST 512-byte answers and all 16 independent boundary/long-output records are replayed. |
| Unchanged SHA3 behavior | The complete package run executes the unchanged four differential tests, each enforcing exactly 14,097 independent records and checking one-shot plus streamed Digest paths. This establishes 56,388 unchanged SHA3 record answers through the refactored shared absorption/padding paths. Fixtures were not altered. |
| Documentation, registration and target checks | Module/root/README documentation accurately names caller-selected byte output and distinct phases; fixed-output Digest claims explicitly exclude SHAKE. Provenance gives primary identities and recipes. The new harness-false target uses the existing testkit and maps to integration-4's `[!a-r]*` partition. Affected native tests, clippy and wasm-library build have qualifying logs; formatter and diff checks passed in this review. |

I checked the byte-aligned padding against the primary
[FIPS 202](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf), §§4–6.2
and Appendix B. The standard's XOF capacity is 256/512 bits; Appendix B's
padding table requires `0x9f` for one remaining rate byte and `0x1f`, zero
bytes, `0x80` otherwise. The shared helper preserves buffered input, writes
the suffix at `filled`, clears subsequent bytes and ORs the final bit. For
rate-minus-one input this becomes `0x9f` in the same byte; exact-rate input
has already been absorbed and receives a separate full padding block. SHA3
still passes `0x06`, preserving its corresponding `0x86` edge. The reader's
first block is the post-final-absorption state, and later blocks apply the
same permutation, matching the sponge construction. Data does not choose
branches or memory indexes in the permutation; only public lengths/stream
position govern buffering and output copies.

## Independent fixture provenance verification

I recalculated SHA-256 for all four captured NIST PDFs. Each equals the
published fixture-provenance identity. The PDFs' input displays identify empty
messages and 200 `A3` bytes as required. The independent Rust verifier
`raw/T1-review-fixtures.rs` invokes `pdftotext` afresh on each captured PDF,
extracts its final `Output val is` block, requires exactly 512 bytes, and compares
every byte with the corresponding committed fixture record. All four matched.
This did not rely on the importer's captured text or extraction result.

The same verifier separately invokes the live OpenSSL 3.6.4 public `dgst`
interface for all 16 fixture inputs and output lengths. Every byte matched,
including both 4097-byte outputs and each rate-1/rate/rate+1,
2*rate-1/2*rate/2*rate+1 and 3*rate input. Verification succeeded with exit 0;
complete results are in `raw/T1-review-fixtures.log`. It never imports, links
or calls the implementation under review. The captured importer likewise
obtains independent answers, not implementation source or constants. OpenSSL
does not become a committed test or runtime dependency.

The shared Rust VectorFile reader independently checks header count and SHA-256
body identity before the public consumer tests replay any records. The test
then checks the exact NIST field order/512-byte lengths and boundary recipes,
plus required totals of four and sixteen. Their public API comparisons are
against actual frozen expected bytes, not against another local SHAKE invocation.

## Qualification evidence and reuse assessment

I read the final logs, their package/worktree names and terminal successful
results rather than accepting only the implementation summary:

- `raw/T1-test-final.log`: 129 native cases passed across the selected hash and
  hash-conformance packages, with zero failures/ignored/filtered cases; SHAKE's
  two external tests and all four SHA3 differential tests executed. The same
  run has 28 successful runtime doctests and one successful compile-fail doctest.
- `raw/T1-docs-final.log`: all 29 current hash doctests passed after the final
  root documentation qualification. This covers the consuming SHAKE reader
  example and forbidden reader update example.
- `raw/T1-clippy-final.log`: successful warning-free affected-package all-targets
  completion. The command/report specifies `--locked` and `-D warnings`.
- `raw/T1-wasm.log`: successful optimized dev-profile hash-library compilation
  for `wasm32-unknown-unknown`, attributable to this worktree. This is a build
  result, not a wasm-runtime replay claim.
- `raw/T1-helpers.log`: 77 enforced jobs, 91 distinct rows and no external
  path-includes; `raw/T1-shards.log`: all 42 members covered by six shards.
- This review reran `git diff --check` and
  `cargo fmt --check -p purrdf-hash -p purrdf-hash-conformance`; both exit 0.

The implementation report records the compiler identity as nightly
`4b6d04e706108ccfeafe2547fbe857dfe8972bad` and limits Cargo parallelism to four,
without disabling warnings, assertions or overflow checks. Root profiles are
unchanged and explicitly retain optimized development/test code with runtime
checks. Successful package logs qualify the current algorithm/test bodies.
The final changes after the earlier clippy/wasm runs only qualify root/README
prose; they do not alter implementation, tests, manifests, ledger or target
behavior. The final full package run and final doctest log cover the affected
documentation. Repeating unchanged package/clippy/wasm work is not warranted.
The earlier failed development logs are superseded and are not counted as passes.

## Disposition and boundaries

Required findings: none. No task requirement is satisfied by a stub, fallback,
unrun example or scope cut. The public SHAKE consumer and independent fixture
checks establish the Task 1 substrate contract. The original ML-DSA/composite,
writer, compaction/certification, wasm runtime and final issue requirements
belong to subsequent approved tasks; this review does not claim they exist.

Signed commit and all actual commit/push hooks, remote-OID verification and
task publication remain the parent's next Stage actions. Full-workspace final
qualification and hosted CI were not run by this reviewer and are not proved
by this Task 1 PASS.
