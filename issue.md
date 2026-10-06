# Issue #458: purrdf-gts: composite ML-DSA-65 + Ed25519 signatures under COSE_Sign1

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

`purrdf-gts` signs and verifies GTS frame ids with `COSE_Sign1` over EdDSA/Ed25519 only (`crates/gts/src/cose.rs` declares the one algorithm constant). This asks for a **composite ML-DSA-65 + Ed25519** algorithm in the same `COSE_Sign1` path, conforming to the GTS specification's vectors once they exist.

## Why

A signed append-only segment cannot be re-signed without breaking what its signature attests, because the signature over the head id anchors all prior history. Signatures that must stay verifiable for decades need a post-quantum component from the first write. The cost is about 3.3 KB per signature.

## Requested

- `sign` and `verify_sig` accept the composite algorithm alongside EdDSA, dispatching on the declared COSE `alg`.
- Verification requires **both** components; either one stripped, zeroed, replaced or swapped is refused. One composite algorithm under `COSE_Sign1`, not two `COSE_Sign` signers, so the post-quantum half cannot be stripped and the remainder accepted.
- Construction per the IETF composite ML-DSA work's ML-DSA-65 + Ed25519 pairing; FIPS 204's deterministic variant for vectors, the hedged variant for production signing.
- Conformance against the shared cross-engine vectors when the specification publishes them, plus the stripping and swapping refusals above as local tests.

## Out of scope

Key discovery and trust anchoring (deployment policy, as today), and encryption.


## Comments (9)

### paudley — 2026-10-06T10:01:49Z

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Composite GTS signatures implementation plan

Issue: 458. Repository: Blackcat-Informatics/purrdf. Worktree:
`/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Branch: `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.
Base: `origin/main`, captured `ce3c07192aba1e36666062c00f958670a827cfb5`.
The user authorized the complete Stage 1 → Stage 2 → Stage 3 workflow and merge,
processing one issue at a time in descending issue-number order.

## Requirements, authority and prior art

Implement ML-DSA-65 + Ed25519 as one composite algorithm in detached
COSE_Sign1, retaining EdDSA. Honor the protected algorithm, require both
components, support deterministic known answers and hedged production signing,
and exercise published shared GTS vectors when they exist. Key discovery,
deployment trust policy and encryption are excluded by the issue.

Read `issue.md`, `issue-analysis.md`, `prior-art.md`, `prior-art-assessment.md`,
`brief.json` and worktree `AGENTS.md`. The standing `.baseline` and `.goals`
are untracked files in `/home/paudley/Active/purrdf`, absent in the worktree;
their parent copies govern this work. No applicable ADR or constitution was
found. Preserve protected main, all sibling work and existing evidence. The
emergency deficiency ledger has no entries. No authorized scope cuts exist.

Baseline requires adversarial analysis, complete requirements and no development
issue references in shipping documentation. Goals require original Rust,
maximal utility/performance/portability and hard failures. No semantic Cargo
features, banned dependency edges, copied implementations or coefficient arrays
are permitted. Reuse Ed25519, Keccak, SHA-512, CBOR and shared test infrastructure
in their existing homes. Tooling and new tests are Rust. All hooks and signing
run normally; no bypasses. Generated artifacts are regenerated, never hand-edited.

GTS §9.2 is algorithm-agile and requires honoring the declared algorithm, both
locally and at upstream gmeow-gts `0d1c8299c9411ea4ead853e31721d42ea66f081e`.
Pin composite construction to `draft-ietf-lamps-pq-composite-sigs-19` and the
published `draft-ietf-jose-pq-composite-sigs-04`, not mutable draft HTML.
Use their requested composite COSE value `-58`, identified explicitly as
provisional draft support. IANA currently leaves that value unassigned.
This follows the issue's IETF construction request and existing algorithm
agility; it makes no final registration claim. No invented private/text algorithm
or substitution of standalone ML-DSA-65's `-49` is permitted.

The composite representative is
`CompositeAlgorithmSignatures2025 || COMPSIG-MLDSA65-Ed25519-SHA512 || 0x00 || SHA512(Sig_structure)`.
The ML-DSA component also receives that fixed label as its FIPS 204 context;
Ed25519 signs the representative using the existing native implementation.
Signature encoding is 3309 ML-DSA bytes then 64 Ed25519 bytes. Public-key encoding
is 1952 ML-DSA bytes then 32 Ed25519 bytes. Both component keys must be freshly
generated for the composite and dedicated to it. A composite seed constructor
accepts independent caller-owned seeds; document key dedication without claiming
the library can detect reuse outside its boundary.

Shared GTS composite vectors are absent in that captured upstream tree; only
the two existing EdDSA Sign1 fixtures are present. The issue makes new shared
vectors conditional on publication. Refresh upstream before PR creation and
merge; if published, consume them under corpus policy. If still absent, state
that shared GTS composite interoperability is unverified, preserving independent
FIPS/IETF fixture and local adversarial coverage. Never fabricate shared vectors.

Prior title-search found no linked work or previous composite implementation.
Composite-defect recurrence is UNKNOWN; three `none` trailers in 200 commits
do not measure this defect class. Current `verify_sig` ignores protected alg,
and `parse` accepts wrong tags/payloads/trailing input. Real authoring, keyring
verification and RDF detached certification currently constrain keys to Ed25519.
RustCrypto ml-dsa has an unconditional banned `signature` dependency, so adopt
an original first-party FIPS 204 implementation rather than weakening the ban.

## Design and completeness contract

Keep SHAKE in `purrdf_hash::sha3` over the existing permutation and block buffer.
Keep the original ML-DSA-65 implementation in one public `purrdf_gts::mldsa65`
module, with private arithmetic/encoding submodules as warranted; register its
single home in the helper ledger. It is a GTS signing primitive with no new
external runtime dependency. Use generated-from-definition NTT constants,
bounded canonical decoding and secret-independent arithmetic. Existing `sha2`
supplies SHA-512 through a workspace-inherited dependency where needed.

Expose typed composite signing/verifying keys in the COSE path, and a strict
typed Sign1 parser shared by key lookup and verification. Retain existing
Ed25519 convenience APIs through that implementation, with no second parser or
verification body. Algorithm/key mismatch is invalid. Parse a single complete
tagged-or-untagged Sign1 item, detached null payload, exact supported protected
algorithm and exact signature length. Reject ambiguous algorithm declarations,
wrong tags, unprotected conflicting alg, trailing input and malformed fields.
An unresolved key on a well-formed supported envelope remains unverified;
malformed/unsupported envelopes are invalid even without a resolved key.

Production signing requires caller-supplied fresh cryptographic hedging input
for every composite signature through an explicit fallible provider. Deterministic
fixture signing is explicitly named and distinct. No silent deterministic fallback
on a missing/failing provider. Propagate signing failures through the existing
fallible writer entry point before publishing bytes or advancing head/index
state. Evaluate the infallible writer conveniences and concrete callers so that
new signing failures cannot become partial output or a hidden panic contract.
Caller-provided entropy keeps the library portable without OS RNG syscalls.

Generalize caller-supplied keyring and affected compaction/certification paths to
algorithm-aware keys/signers through the same core. Retain OpenPGP Ed25519 key
discovery, as the issue excludes new key discovery. Preserve carried authorship
signatures byte-for-byte and mandatory packaging signatures.

| Requirement | Tasks | Executable acceptance |
|---|---|---|
| Same Sign1 path supports EdDSA and composite, dispatching protected alg | 3, 4 | Public COSE signing/verification and Writer → reader → `verify_file_with_keyring` accept both algorithms; inspect emitted tag, detached payload, protected alg and lengths. Existing frozen EdDSA bytes match. |
| Both halves required; no stripping/zeroing/replacement/swapping | 3, 4, 5 | Mutate each component independently; cross-message/cross-key splice, reorder, truncate, append, downgrade header and use wrong key type. All refuse. Rewrap as COSE_Sign and refuse. Writer-produced file tampering yields invalid verification. |
| Exact IETF pairing construction | 2, 3 | Replay primary-source FIPS 204 known answers and pinned IETF composite example through public primitive/COSE APIs. Verify representative, context, key/signature encoding and component order against external bytes. |
| Deterministic vector variant | 1, 2, 3 | SHAKE known answers and absorption/squeeze boundaries; deterministic key/signature outputs equal independent FIPS/IETF fixtures byte-for-byte. |
| Hedged production variant | 2, 4 | Distinct supplied randomizers produce distinct valid signatures; provider error/missing entropy returns typed error and leaves Writer state/output unchanged. Real production Writer calls this path. |
| Published shared GTS vectors, conditional on publication | 6, Stage 2/3 | Refresh upstream corpus, run all existing COSE vectors, and replay composite shared vectors if published. Record absent conditional evidence honestly when still unpublished. |
| Portability, caller wiring, preservation of signed history | 1–6 | Native affected-package tests, affected wasm build and Node runtime replay of public composite behavior; actual compaction and RDF certification accept valid composite evidence, reject tampered evidence, retain exact source signature bytes and verify new packaging. |
| Final review, qualification and merge | 6, Stage 2/3 | Full captured review surface, independent completion audits on shipped entry points, required hooks/checks, ghprsq audit refs/signature/tree, issue closure and safe cleanup. |

## Enhancements considered and declined

Do not add ML-DSA-44/87, new encryption, composite OpenPGP discovery, trust policy
or a new standalone published crypto crate: the concrete requirement is the
65/Ed25519 pairing in GTS; an additional release crate would enlarge publication
and packaging obligations without an existing second consumer. Preserve one
ML-DSA home in GTS. No separate private protocol, two-signer envelope or
per-consumer cryptographic implementation. SIMD tuning requires measured evidence
and is not a substitute for a performant NTT-based portable implementation.

## Task 1: Extend the existing Keccak home with SHAKE128 and SHAKE256

Implement streamed absorption and incremental squeezing with type separation
between absorbing and squeezing, using FIPS 202 suffix and rates. Reuse Keccak
and the shared block buffer. Add meaningful Rust tests of public SHAKE against
primary known answers, long output and rate boundaries, empty calls, split
absorption/squeezing and unchanged SHA3 differential vectors. Update relevant
API documentation. Run affected hash/hash-conformance tests, warning-free clippy
and affected wasm build. Independent task review writes `tasks/T1-review.md`;
then signed commit with complete hooks, push, verify remote OID and issue update.

## Task 2: Implement and independently qualify native ML-DSA-65

Implement FIPS 204 key expansion, canonical public/secret/signature encodings,
NTT polynomial arithmetic, rejection sampling, rounding/decomposition/hints,
deterministic signing, explicit hedged signing and strict verification. Use fixed
65 parameters, owned redacted secrets with clearing on drop and no copied code
or constants. Register its single home. Import independent primary frozen
fixtures with provenance outside the governed GTS corpus; acquire them with
Rust tooling if a retained generator is necessary. Known-answer keygen/sign/verify
must pass, including canonicality/length/hint failures and changed message/context.
Check native and wasm public primitive behavior and constant-time design in
independent security review. Independent review writes `tasks/T2-review.md`;
signed hook-verified commit, push, verify remote and publish task evidence.

## Task 3: Implement the composite and strict algorithm-aware Sign1 core

Introduce composite key encoding, pinned domain/context construction and signature
encoding. Unify strict parsing and algorithm/key dispatch with existing EdDSA.
Use the primary IETF example and existing frozen Sign1 fixtures, plus every
component/header/envelope attack listed above. Preserve Encrypt0 behavior with
affected regressions. Public signing and verifying APIs provide execution evidence.
Independent security/consumer review writes `tasks/T3-review.md`; commit/push
with hooks and publish task evidence.

## Task 4: Wire hedged signing and algorithm-aware resolved verification

Integrate typed signer and fallible randomness provider into the actual Writer
and signed-frame emission, preserving atomic writer state on failure. Route file
keyrings and folded signature resolution through the shared typed parser/verifier.
Resolve infallible convenience call contracts coherently rather than introducing
a parallel writer or silently accepting incomplete output. Run real Writer →
reader → keyring verification with EdDSA/composite mixed history, changed frame
bytes, malformed signatures, unresolved keys and failed entropy. Demonstrate
hedged output variation and typed failure. Independent review writes
`tasks/T4-review.md`; signed commit/push and task publication.

## Task 5: Carry composite evidence through compaction and certification

Update affected GTS compaction and RDF detached signature certification key/signing
contracts. Execute real compact/certify paths over a composite-signed source,
verifying exact carried authorship COSE bytes and valid mandatory packaging
signatures. Corrupt each half and require refusal. Preserve existing EdDSA
compaction/certification corpus. Document portable caller-provided composite key
and randomness usage without issue references. Independent review writes
`tasks/T5-review.md`; signed hook-verified commit/push and task publication.

## Task 6: Final Stage 1 qualification, PR creation and publication

Refresh external conditional vector state. Consolidate exact source, compiler,
profile, target, command/status and artifact identities in `validation.md`.
Run final affected package/consumer tests and native/wasm entry-point demonstrations;
run required local qualification from repository policy and accepted coverage,
including helper/layer/banned-dependency checks and generated metadata drift where
affected. Inspect actual gate/CI coverage; no unrun check is reported as passed.
Scan source diff, plan, commits and PR text for incomplete work and adjudicate
every hit. Independent task review writes `tasks/T6-review.md`.

Use stagectl to create a PR with `Closes #458`, concrete behavior, provisional
draft status and accurate checks. Publish this plan on issue and PR, plus the
required confidence/risk answers. Verify PR URL, remote head and evidence.
Continue Stage 2 independently reconstructing requirements and reviewing actual
production demonstrations, fixing and publishing every actionable finding.
Stage 3 captures full check/review/thread debt, predicts and assesses integration
tree, independently audits completion, writes and publishes concrete squash notes,
and merges exclusively through `/home/paudley/stage/root/bin/ghprsq` with the exact
stage directory. Verify signed result, parents/tree, notes and archived stage refs,
remote branch deletion, issue closure, then dry-run and perform safe owned cleanup.

## Progress and review state

Intake reports are written and read. No implementation, tests, source commits,
pushes or PR yet. Plan review has not run. Required independent review must
establish completeness and repository compliance before executing Task 1.


### paudley — 2026-10-06T10:20:37Z

Task 1 implemented streaming SHAKE128 and SHAKE256 in the existing native
Keccak home. The public API consumes the absorber into a distinct incremental
output reader and uses caller-owned output buffers. Existing SHA3 modes use
the same absorption and padding implementation.

Independent task review returned PASS against the approved plan and exact
source identities. It independently replayed all four complete NIST answers
from the captured primary PDFs and all 16 OpenSSL boundary/long-output answers.

Validation passed: 129 native affected-package tests, 29 doctests, all-target
clippy with warnings denied, wasm library build, helper census, shard coverage,
formatting and whitespace checks. The unchanged four SHA3 suites replayed
56,388 frozen records. The required staged-index commit hooks ran successfully.

Signed commit: `74bf968ce28da00f9ab6ad054b150c4f5d91da18`
(`feat(hash): add streaming SHAKE128 and SHAKE256`).
Push succeeded and remote branch readback matches that exact commit.

This checkpoint establishes the hash substrate. The remaining approved tasks
are still required for ML-DSA, composite Sign1, writer/resolver integration,
compaction/certification and final qualification. No PR or merge is claimed.


### paudley — 2026-10-06T10:53:18Z

Task 2 implements original native pure ML-DSA-65, including key expansion,
validated expanded-secret import, canonical encodings, NTT arithmetic,
deterministic and caller-randomized hedged signing, strict verification and
typed context/sampling/nonce errors. Shared secret clearing and byte comparison
reuse the existing Ed25519 home. No runtime dependency or semantic feature was added.

Independent security/consumer review returned PASS. It separately joined all
70 frozen records to pinned NIST primary JSON: 25 complete public/expanded-secret
keys, 15 deterministic signatures, 15 hedged signatures and 15 verification
decisions (three valid, 12 refused). All records match byte-for-byte.

Validation passed: 294 affected native cases including nine doctests; six real
wasm/Node public tests covering every official case and sealed clock/entropy
behavior; affected all-target clippy with warnings denied; affected wasm library
build; helper census, shard coverage and formatting. Source security review
assesses fixed arithmetic/norm schedules and variable-time FIPS rejection;
it does not claim measured compiler/JIT/hardware timing or FIPS certification.
Owned secret storage is overwritten on drop/error; SHAKE state and historical
compiler/register/stack copies are outside that documented clearing boundary.

Signed implementation commit: `34e7cfdde620e78bcc336feec9b43ea712796920`.
A staged check caught trailing whitespace in the copied NIST notice that the
earlier unstaged check had not covered. The complete notice wording is retained
with whitespace normalized and accurately disclosed. Independent refinement
review passed; full Task 2 source whitespace check now passes. The correction
is recorded in a separate signed commit without rewriting the implementation.
Signed correction: `bf7d5d14ff72f4e6a21ea6efc2b58680f4fdf33f`.
Both commits ran the required staged-index hooks successfully.
Push succeeded; remote branch readback matches
`bf7d5d14ff72f4e6a21ea6efc2b58680f4fdf33f`. Final branch diff whitespace
check passes, including all imported files.

This checkpoint establishes the primitive. Composite Sign1 and algorithm dispatch,
actual writer/resolver integration, compaction/certification and final whole-issue
qualification remain required. No PR or merge is claimed.


### paudley — 2026-10-06T11:46:14Z

Task 3 is independently reviewed, committed and pushed as signed `31498279c83c7fb9c3c6d97faffc1628a82452e1`; normal hooks passed and remote branch readback matches.

The native combiner implements the pinned ML-DSA-65/Ed25519 construction, with both components mandatory. One strict detached COSE Sign1 parser now honors the protected algorithm, preserves exact protected bytes, refuses malformed/ambiguous envelopes before key lookup, and supports optional opaque key IDs. Existing EdDSA convenience APIs use that same core and retain the frozen output bytes. Composite `-58` remains the pinned draft's provisional request, with no final registration or shared GTS interoperability claim.

Independent extraction matched all eight published IETF fixture fields, including all 3373 signature bytes. Review found malformed textual content-type acceptance; the shared validator was corrected to RFC 9052/RFC 6838 grammar. The independent original public reproducer now rejects every malformed neighbor before lookup. The initial failed review and its correction evidence are preserved.

Qualification: 263 affected GTS cases including nine doctests; eleven public groups execute successfully in native and wasm/Node, covering external answers, both-component attacks, optional/binary IDs, and 20 valid plus 162 malformed content-type cases. GTS all-target clippy with warnings denied, formatting, helper census, declaration gates, all new-file whitespace and exact-source readback pass. RDF compilation and separate wasm library evidence remain applicable across the parser-only correction; the report distinguishes those earlier checks from current execution.

Final reviewed manifest SHA256: `54d8e8204b39b971126d9398f0a3d216175edc1b9ee0ca309356af01ce67c0a6`. Complete Task 3 patch SHA256: `cccbf183ac6de2283855cf38065511e904743fdbec92ea42bda836e5f6f75022`.

The next approved Task 4 integrates the actual Writer's fallible hedging provider and algorithm-aware key resolution. Task 5 covers compaction/RDF certification; Task 6 and Stages 2–3 cover final qualification, PR and merge. Whole-issue completion, hosted CI, hardware timing and formal certification are not claimed by this checkpoint.


### paudley — 2026-10-06T12:34:22Z

Task 4 is implemented, independently reviewed, signed and pushed in commit
`39f0d4dce74c7635a8d4675406cc37eb763cb590`; remote branch readback matches.

Composite Writer authoring now requires an explicit fallible randomness
provider. Its sealed signing mode makes every append convenience return a
Result, preserves existing output and chain/index state on provider failure,
and supports Ed25519/composite rotation through the actual shared append path.
Typed keyrings resolve exact opaque optional identifiers and protected algorithms
through the strict shared parser/verifier; existing Ed/OpenPGP callers compile
and use the same result pipeline. SnapshotSigner redacts and clears its owned
secret storage using existing shared implementations.

The independent review found unsigned acceptance of empty/malformed input and
skipped evidence/opaque profile policy. Both are fixed through the existing
shared verification pipeline. Its original unchanged probe now refuses all
five counterexamples while valid generic unsigned input passes. The governed
Debug macro finding is also fixed. Original failed evidence remains preserved.

Qualification: thirteen current public Writer/file groups pass natively and in
actual wasm/Node, including entropy failure/recovery, mixed algorithms, both
signature components, framing/content tampering, profile findings, sealed-source
exception and signed opaque MissingKey/UnknownCodec. Clippy, strict rustdoc,
formatting, helper census and separate staged whitespace check pass. Normal
commit hooks passed; commit signature verifies. The original complete GTS run
covered 274 cases; its unchanged crypto/codec/consumer evidence is reused with
the source delta assessed, not presented as a new whole-package execution.

Task 5 compaction/RDF certification and Task 6 final qualification/PR publication
are the next planned units. No PR, hosted CI qualification or merge yet. The
composite construction remains pinned provisional draft support; this checkpoint
makes no formal certification, hardware-timing or shared-composite-corpus claim.


### paudley — 2026-10-06T13:49:21Z

Task 5 is committed and pushed as `b2560386263ff53578b3a06e40f5739f577829ac`.
The commit is signed, normal hooks passed, and the remote branch readback matches.

The existing compactor and RDF certifier now support mandatory composite or
Ed25519 packaging through one sealed signing contract. Composite packaging
requires the caller's key and randomness provider; signing errors return an
actionable refusal without a returned artifact. Carried authorship keeps its
exact original frame IDs and COSE bytes, including binary identifiers resolved
through the typed keyring. Certificate packaging identifiers retain their
existing deliberate textual schema.

Certification authenticates the actual final ordering index, preserves the
exact authorship set and checks its root/proofs. One shared strict decoder
handles carried fields and genuine Compaction root records. Current root
selection uses the complete actual source-head list; legitimate earlier roots
remain available through repacks and newly authored tails.

Independent review initially blocked the task on two reproduced defects:
ordinary content was misclassified as packaging and lost its author signature;
conflicting or nonliteral roots certified successfully. Both are fixed and
independently discharged. A further type-only counterexample was reproduced
and fixed before acceptance. One shared incremental provenance classifier now
requires the Compaction node's mandatory positive fields and closed predicates;
ordinary or incomplete content retains its original authorship. Eager and
evented readers use the same segment-local computation.

Actual corrected-source qualification passed:

- 44 RDF caller/certificate/frozen streamable/dictionary cases.
- 40 GTS compaction, pinned-byte, COSE and Writer/profile cases.
- 12 compaction unit cases.
- All nine public compaction/certification groups executed in actual wasm/Node.
- The unchanged original independent public probe, separately replayed by the
  reviewer, now passes both malformed-root refusals and exact authorship preservation.
- Affected all-target clippy with denied warnings, strict rustdoc, helper census,
  formatting, complete whitespace checks and source identity verification.

The independent Task 5 verdict is PASS on the exact thirteen-file source
manifest `38580bdbbf4209b77c08a608574a4fcb288c50c09c0f545d6a59d837dd430037`.
Original failed reports, probes and logs are retained as historical evidence.
Frozen governed vector bytes are unchanged.

Five of six Stage 1 tasks are complete. Final whole-issue qualification,
conditional external-vector/allocation refresh and PR publication remain Task 6.
No full final local gate, PR-head hosted CI, Stage 2/3 readiness or merge is
claimed by this checkpoint.


### paudley — 2026-10-06T14:49:17Z

# Final qualification checkpoint: required repair remains open

Stage 1 Task 6 is incomplete; no PR has been created.

The required full `CARGO_BUILD_JOBS=4 make check` completed normally with exit 0 on its captured source. It executed 21,045 passing native cases, 460 passing doctests, the preserve-order consumer, core hygiene and actual release WebAssembly builds. Existing ignored cases are recorded separately. The captured gate manifest is `77082e9b925a52e168b3523d872174f96c39de49840fb11127a1ed6e04651eda`; its log is `7f7aea50e039f0d0643fd1ddb0170089290c8cf8f691385463e4e3c4d8562c99`.

Independent public producer/consumer probes then established required finding T6-R1. An authored streamable index with invalid or non-UTC `xsd:dateTime` compaction facts is misclassified as packaging. Actual compaction/certification drops its original COSE signature and reports all six checks true. Direct compaction also accepts invalid rewrite timestamp parameters. The probe's process exit 0 records these defective observations; it is not passing acceptance. GTS specification section 13.3 requires rewrite time as `xsd:dateTime` in UTC.

Parent accepted the independent BLOCKED report `tasks/T6-timestamp-review.md`, SHA256 `a5d69a181c58a373dc592df6aaa279be3a3b59de14ca68ba38c80b3b94cf462e`, after reading the complete report, both unchanged probe bodies and terminal outputs, the governing specification and actual classifier/emitter. Reviewed source is signed/pushed HEAD `b2560386263ff53578b3a06e40f5739f577829ac` plus four qualification prose/registry changes, complete manifest `1bf6eded811940dba80f8b34156e2102b4a16ebb32a3b2b8ede0f60edf5fa989`, complete patch `1d9dacca903f3649ee8de5e823e4a51e4bf359dd7fe5b61af744aaec00ae6741`.

Remediation owner is the current Task 6 implementer, under parent responsibility, working only in the issue worktree. The assigned repair uses the existing XSD parser for one shared classifier/producer rule, preserves legitimate UTC offsets and XSD lexical forms, retains exact authored evidence for malformed source shapes, and refuses invalid production timestamp parameters. Actual native/wasm pack/repack and producer checks, affected qualification and independent closure are required before signed transport and PR publication. Original failed reports, probes, logs and qualification snapshots remain preserved. No scope cut, ledger entry, accepted limitation or future ticket discharges this finding.

The approved plan remains unchanged at SHA256 `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`. Stage 2, hosted PR checks and Stage 3 merge have not run.


### paudley — 2026-10-06T15:28:42Z

What am I least confident about, and why?

Compiler, JIT and hardware side-channel behavior. The original native
implementation has independently reviewed fixed-schedule arithmetic and
full-scan comparisons, and it reproduces complete official NIST and IETF
answers natively and in wasm. Those checks establish functional behavior,
not a timing or fault-resistance proof. FIPS rejection sampling is
variable-time; no complete-operation constant-time or formal FIPS
certification claim is made.

What should the user know?

The construction is deliberately pinned to JOSE/COSE draft-04 and LAMPS
draft-19. Its requested COSE identifier -58 remains unassigned in the
official IANA refresh. The captured authoritative GTS upstream still
publishes only two EdDSA Sign1 vectors, so independent primary composite
answers do not establish shared-engine GTS composite interoperability.
Both components authenticate one Sign1; neither may survive independently.

The timestamp boundary found during qualification is corrected through the
existing XSD parser. Actual native and wasm controls now prove malformed or
non-UTC source shapes retain authored COSE/content, invalid producer parameters
refuse before randomness, and valid XSD UTC spellings retain their exact bytes.
The unchanged independent review probes reproduce the corrected behavior.

Production callers must supply dedicated independent component keys and a
fresh, full cryptographic randomizer for each signature. Provider failure
refuses atomically; the portable library cannot detect external key reuse
or certify a successful provider's entropy quality. Controlled owned
secrets are overwritten on drop, but historical compiler copies, registers,
spills and hash states are outside that clearing guarantee.

The attached approved plan preserves its original planning snapshot,
including its historical progress footer. Current implementation and
qualification are recorded in the signed task checkpoints and final
qualification evidence. PR publication, hosted checks and Stage 2/3
acceptance are separately verified workflow states.


### paudley — 2026-10-06T15:33:28Z

Task 6 completed: final Stage 1 qualification, signed transport and verified publication.

The timestamp finding recorded at https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6018859211 is DISCHARGED. The shared existing XSD dateTime parser now requires an explicit UTC timezone for positive rewrite provenance and producer parameters. Malformed/non-UTC source facts retain authored content and exact COSE; invalid producer parameters refuse before signing. Valid UTC spellings, including extended/negative years and legal 24:00:00, preserve caller bytes. The original independent reproducer bodies were unchanged and both were independently rerun successfully.

Current affected qualification passed 1,361 native GTS/RDF cases, 13 docs and all 11 actual Node/wasm compaction/certification groups. Workspace all-target clippy with warnings denied, strict affected docs, generators, hygiene/dependency checks and current release `make wasm` passed. The prior required full `make check` pass remains attributed to its captured older source; only unaffected closures are reused. No old gate is relabeled as execution on the corrected tree, and no existing default ignore is counted as a pass.

Signed commit and verified remote head: `52988974f2d11a40214281648983f9145af7a3fb`; tree `43b0817e30a3ad110cd013796a064c177a27e067`. All normal commit hooks completed; signature independently verified. Final 52-path source manifest SHA256: `e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d`. Independent final Task 6 review passed, SHA256 `ca9e397366d1c37f7ce3b2200f50b4e2a0f8fcb08d4a742972e90e91a32be35b`.

PR: https://github.com/Blackcat-Informatics/purrdf/pull/464. Its head, branch, main base and exact body were independently verified. The Stage tool created the PR but its wrapper returned a post-create JSON parsing error; lookup/readback confirmed the existing PR, so no duplicate or alternate creation was attempted.

The exact approved plan is published on this issue and PR:
https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6013901476
https://github.com/Blackcat-Informatics/purrdf/pull/464#issuecomment-6019626579

Confidence/limits answers are published on both:
https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6019631272
https://github.com/Blackcat-Informatics/purrdf/pull/464#issuecomment-6019634912

All six approved Stage 1 tasks now pass independent review and their source is signed/pushed. Hosted CI is pending; CodeRabbit's green check accompanies a usage-cap notice and supplies no completed review. The base advanced to `0d6575a46088e9420d73654b7b5965b9580c5d36`; current-base integration, Stage 2 gap/completion audits and Stage 3 acceptance/ghprsq merge are still required. Provisional draft allocation, absent shared composite vectors and entropy/timing/clearing limits remain explicit. Stage 1 publication does not establish merge or release readiness.


