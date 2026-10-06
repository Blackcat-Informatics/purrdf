<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Task 2 security and consumer review

VERDICT: PASS

Required findings: none. This verdict qualifies the approved native ML-DSA-65
substrate task, not the composite, writer, certification, commit or whole issue.

## Reviewed identity and authority

Reviewed 2026-10-06 in
`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`,
branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
HEAD is `74bf968ce28da00f9ab6ad054b150c4f5d91da18`; captured base is
`ce3c07192aba1e36666062c00f958670a827cfb5`. Task 2 is uncommitted and the
index remains empty. The authoritative plan hash independently matches
`dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

All 17 current source/document/fixture identities pass
`sha256sum -c raw/T2-files.sha256`. The source-manifest identity is
`feb341c14e7e8887ad2497479d8c8a435c5e32ae608cdd15e74a6c85e7938ce4`.
The complete patch including new files is `raw/T2-source.diff`, independently
hashed as `3bfdf38748668d0941d28f60c4cc706a0f13f2e9073056c8c026b06e8a22178b`.
The observed tracked and untracked source paths match the implementation report;
selected Stage evidence remains separate.

Read the complete issue, its analysis and prior-art assessment, authoritative
plan and plan-review, implementation report, applicable AGENTS, parent
`.baseline`/`.goals`, clean deficiency ledger, helper/layer declarations and
Stage 1/Stagectl quality, validation, delegation and no-deferral instructions.
Inspected all four new implementation modules, public integration tests, fixture
provenance, shared Ed25519 changes, manifests and relevant shard/CI wiring.
Read the captured FIPS 204 parameter, key generation, signing, verification,
sampling, packing, rounding and NTT definitions and its security requirements.
No source/index edits, commits, pushes, forge mutation or delegation occurred.
Only review artifacts were written under the selected Stage directory.

## Functional and adversarial assessment

| Approved Task 2 requirement | Independent assessment |
|---|---|
| Original FIPS 204 ML-DSA-65 implementation | Parameters match Tables 1–2: q=8380417, k/l=6/5, eta=4, tau=49, beta=196, gamma1=2^19, gamma2=(q-1)/32, omega=55, challenge 48 bytes; exact public/private/signature lengths 1952/4032/3309. Roots are generated from 1753^BitRev8(i), not a pasted array. |
| Key expansion and validated expanded import | Seed expansion includes k/l domain bytes, splits rho/rho-prime/K correctly, computes A*s1+s2 and Power2Round, and stores the complete public hash. Import checks length before slices, rejects all eta-4 forbidden nibbles through the strict norm, derives and compares t0/public hash over full storage, and guards owned secrets on failure. Independent K cannot be validated from the public key and is correctly treated as independent material. |
| Arithmetic and samplers | Forward/inverse schedules follow FIPS Algorithms 41–42, including reverse-root sign and 256 inverse normalization. Matrix expansion uses column,row bytes; secret/mask nonces use little-endian 16-bit values. Rejection tests implement the specified 23-bit uniform, eta-4 nibble and weight-49 challenge sampling. Modular operands remain in the stated ranges; widened products fit u64, additions/subtractions fit i32, and bit accumulators fit u32. |
| Deterministic and hedged signing | Both public methods route through one body; deterministic uses exactly zero rnd and hedged consumes the provided 32 bytes. Pure context framing is 0x00, one-byte context length, context, message. The private seed hashes K/rnd/mu; masks use disjoint groups of five nonces. Response/low-part/product norms and hint-weight rejection are present, with no rejected signature released. The hint comparison is equivalent to HighBits(residual+ct0) versus the already qualified HighBits(residual). |
| Strict parsing and authentication | Public and signature lengths are exact. Ten-bit public coefficients cover legal t1 encodings, including q-1 after the 13-bit shift. Signature decoding enforces strict response norm, monotone cumulative hint offsets, strictly ordered unique positions and zero unused storage. The private Signature representation preserves those invariants: external bytes must pass decoding, while the only signing constructor follows rejection checks. Verification reconstructs A*z-c*t1*2^d, applies hints, hashes w1 and compares all challenge bytes. It does not equate successful parsing with authentication. |
| Typed exhaustion and refusal | Samplers have finite candidate budgets; signing reserves the entire next nonce group before generating masks. Exhaustion/overflow has a typed error and no output. Internal tests exercise actual starved sampler/nonce branches, including u32 overflow. Context >255 is checked before secret decoding; malformed imports and each response/hint/challenge/key section are covered. |
| Portability and real primitive callers | Tests call the public keygen/import/sign/decode/verify APIs; fixture generation is not an oracle. The same public suite actually executes in wasm/Node, including all 70 official cases and sealed entropy/clock behavior. No platform entropy lookup or new dependency/feature/unsafe implementation appears. |

The mathematical tests independently compare complete NTT multiplication with a
negacyclic integer convolution and exhaustively check rounding over all q
residues. These support the source trace; they are not substitutes for the
official complete key/signature bytes. Public tests additionally cover changed
messages/contexts, 255/256-byte contexts, empty input, distinct valid hedged
signatures, expanded-key invariant corruption, all forbidden eta nibbles,
response norm neighbors of both signs, canonical hints and section tampering.

## Independent fixture audit

Independently verified the six captured primary JSON hashes and the FIPS PDF
hash against the fixture provenance. The selected prompt metadata is genuinely
ML-DSA-65, pure external mode; sigGen groups 3/15 declare deterministic true/false.
No assumption that result groups repeat prompt metadata was made.

Wrote a separate jq inspection query, `raw/T2-review-fixture-join.jq`, which
joins expected groups/cases uniquely by tgId/tcId, checks parameter/interface
metadata, independently formats all required fields, and obtains deterministic
zero rnd from the prompt's deterministic flag. It does not use the importer's
code, checksums or PurRDF-generated outputs. Each query output was compared with
the complete corresponding frozen record body using `cmp`; all three commands
terminated with exit 0:

| Fixture | Independent complete comparison | Saved audit inputs/outputs |
|---|---|---|
| keyGen | All 25 seeds, IDs, complete public and expanded-secret encodings equal pinned JSON | raw/T2-review-keyGen-{primary,frozen}-records.txt |
| sigGen | All 30 IDs, secret/message/context/randomizer fields and complete signatures equal pinned JSON | raw/T2-review-sigGen-{primary,frozen}-records.txt |
| sigVer | All 15 IDs, public/message/context/signature bytes and Boolean verdicts equal pinned JSON | raw/T2-review-sigVer-{primary,frozen}-records.txt |

Compared the exact three-paragraph License section from pinned
`raw/ACVP-README.md` with shipped `NIST-NOTICE.txt`; `cmp` exits 0.
The extracted section is `raw/T2-review-NIST-license.txt`. Whole-README equality
is neither required nor claimed. Fixture provenance identifies the source,
date and JSON-to-record formatting. Excluding preHash/internal groups matches
the pure external API claim. Governed root GTS vectors are untouched.

## Security design and precise limits

Owned expanded bytes and polynomial/byte scratch use private RAII guards that
call the existing shared clearing home on normal return, rejection and error.
Clone explicitly creates another guarded owned secret; Debug exposes only the
public key. Export copies into caller-owned storage, with caller responsibility
documented. Secret import temporaries, transformed secrets, masks, rejected
commitments/challenges, products, responses, residuals and hints remain guarded.
Matrix/public coefficients need no secret guard.

The clearing helper overwrites defaults, observes the slice through black_box
and issues a compiler fence; its source is reused rather than duplicated.
This establishes the stated controlled-storage overwrite design. It does not
prove destruction of historical moved/compiler/register/stack copies, and SHAKE
internal states remain outside this clearing boundary. The shipped module and
implementation report explicitly disclose that limitation. This verdict does
not assert complete device-level FIPS 204 intermediate-data destruction or
FIPS certification.

Secret coefficient arithmetic, transforms, packing and norm scans follow fixed
source schedules without secret coefficient indexing or norm-scan early exits.
FIPS samplers/retries are variable-time; challenge indexing follows derived
challenge positions, with rejected candidates guarded. Constant-divisor
remainder and comparisons can lower differently across native/wasm targets.
No timing measurement, assembly audit, hardware/JIT constant-time proof or fault
resistance qualification was executed or claimed. Functional vectors cannot
establish those properties. The documented source-level boundary accurately
reflects this review. Caller seed quality and fresh signing randomness remain
caller obligations; actual production provider wiring is Task 4.

Generalizing Ed25519's comparison to slices preserves the existing equal-length
32-byte field caller and explicitly rejects unequal public lengths. Existing
Ed25519 suites and new wide/prefix tests pass in the attributable native log.
The generic wipe is the previous implementation made accessible, not a second
body. Helper registration and AGENTS inventory match these actual homes.

## Qualification adjudication

Reused attributable terminal implementation executions against the unchanged
reviewed 17-file identity. Read their relevant output and source tests; no
expensive suite was repeated solely to create a second invocation.

- `raw/T2-tests-complete.log`: all 28 result groups are passing; independent
  awk summation confirms 294 cases, including nine doctests. Public ML-DSA tests,
  arithmetic/exhaustion tests and Ed25519 regressions actually execute.
- `raw/T2-wasm-runtime-qualified.log`: six public tests pass with zero ignored,
  including the three official fixture suites and sealed-host test. This is
  actual wasm runtime evidence, beyond `raw/T2-wasm-build.log`'s successful
  affected-library compilation.
- `raw/T2-clippy-qualified.log`: affected packages/all targets compile through
  terminal optimized dev completion with -D warnings, no warning/error output.
- `raw/T2-helpers-qualified.log`: 80 enforced jobs, 91 distinct rows, 1813 files,
  no open copies. `raw/T2-shards.log`: six shards/42 members; the explicit
  harness=false mldsa65 target falls in the existing [e-o]* integration shard.
- `raw/T2-format.log`, `raw/T2-diff-check.log` and
  `raw/T2-manifest-check.log` are terminal passing evidence. Current file hashes
  were independently checked in this review.

These executions use the implementation report's recorded nightly
4b6d04e706108ccfeafe2547fbe857dfe8972bad/LLVM 23.1.1, repository optimized
profiles with assertions/overflow checks, and Node 26.10.0/wasm-bindgen 0.2.125.
No compiler/profile weakening or source change invalidates those results.
Reviewer-executed checks comprise current identities, branch/index inspection,
primary hashes, independent fixture comparisons, exact license-section comparison
and log count verification; no new cryptographic execution is relabeled as such.

## Disposition

Task 2's required implementation, independent primary evidence, refusal behavior,
native/wasm runtime qualification and security-design review are met against
the recorded source identity. There is no required fix before this task's normal
signed commit/hook/push boundary. No commit, hook, push or hosted CI has been
qualified here. Composite domain/encoding and strict COSE dispatch, actual Writer
entropy/resolver wiring, compaction/certification and final whole-issue acceptance
remain the distinct approved Tasks 3–6; this PASS does not replace them.

## Notice refinement addendum

After the original review, parent committed Task 2 locally as
`34e7cfdde620e78bcc336feec9b43ea712796920` (independently observed signature G),
without pushing, then corrected only NIST-NOTICE trailing whitespace and added
accurate provenance disclosure. Independent refinement review is
`tasks/T2-notice-review.md`, VERDICT: PASS, no required findings.

The original identity section above remains historical. Current HEAD plus
`raw/T2-notice-source.diff` identifies the refined source; patch SHA-256 is
`39119e36acc0f4e13ddbe4485c6cf77aaff83455b8929c15ad26b743812fdf65`.
Both refined document identities match `raw/T2-notice-files.sha256`; all
other 15 files still match the original manifest. Independently comparing
the complete pinned NIST notice with trailing blanks normalized proves all
wording and paragraph order unchanged. Runtime/clippy/wasm/official-answer
evidence remains applicable to the unchanged code/tests/fixture bytes.

The original working-tree diff check did not cover files that were then
untracked, and must not be treated as full-source whitespace qualification.
Parent's staged check discovered the imported notice's trailing blank.
The reviewer now independently ran
`git diff --check 74bf968ce28da00f9ab6ad054b150c4f5d91da18` against the
complete final Task 2 patch including all formerly new files: exit 0.
This closes the whitespace coverage gap. Corrective commit hooks/signing,
push and remote readback remain unqualified by this review.
