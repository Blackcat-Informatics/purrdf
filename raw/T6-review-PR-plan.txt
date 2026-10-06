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
