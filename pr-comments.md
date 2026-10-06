# PR #464 — comments and review threads

10 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
> 
> Your organization has reached its usage spending cap. Adjust your spending cap in the [billing tab](https://app.coderabbit.ai/settings/billing?tab=usage&orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> **Next included review available in 3 minutes.**
> 
> [Check out review usage here](https://app.coderabbit.ai/dashboard/review-capacity?orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> <details>
> <summary>View limit details</summary>
> 
> **Limit details:** You’ve used the included review currently available. Your 109 included PR review attempts over the past 7 days set your current allowance at 1 review per hour.
> 
> [Learn how review limits work](https://docs.coderabbit.ai/management/plans#rate-limits).
> 
> **Review configuration:**
> 
> <details>
> <summary>⚙️ Run configuration</summary>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `280c73da-4702-4a78-b906-92754c56db81`
> 
> </details>
> 
> <details>
> <summary>📥 Commits</summary>
> 
> Reviewing files that changed from the base of the PR and between 0d6575a46088e9420d73654b7b5965b9580c5d36 and 241061e7cc08d76de97f8a597074ec5c5fcb2ec7.
> 
> </details>
> 
> <details>
> <summary>⛔ Files ignored due to path filters (1)</summary>
> 
> * `Cargo.lock` is excluded by `!**/*.lock`
> 
> </details>
> 
> <details>
> <summary>📒 Files selected for processing (55)</summary>
> 
> * `AGENTS.md`
> * `bindings/python/src/py_gts.rs`
> * `crates/ed25519/Cargo.toml`
> * `crates/ed25519/src/ct.rs`
> * `crates/ed25519/src/lib.rs`
> * `crates/ed25519/tests/rfc8032.rs`
> * `crates/gts/Cargo.toml`
> * `crates/gts/README.md`
> * `crates/gts/src/compact.rs`
> * `crates/gts/src/cose.rs`
> * `crates/gts/src/cose/composite.rs`
> * `crates/gts/src/cose/sign1.rs`
> * `crates/gts/src/cose/tests.rs`
> * `crates/gts/src/fixture.rs`
> * `crates/gts/src/lib.rs`
> * `crates/gts/src/mldsa65/codec.rs`
> * `crates/gts/src/mldsa65/math.rs`
> * `crates/gts/src/mldsa65/mod.rs`
> * `crates/gts/src/mldsa65/sampling.rs`
> * `crates/gts/src/model.rs`
> * `crates/gts/src/policy.rs`
> * `crates/gts/src/reader.rs`
> * `crates/gts/src/verify.rs`
> * `crates/gts/src/writer.rs`
> * `crates/gts/tests/compaction_signatures.rs`
> * `crates/gts/tests/composite/IETF-NOTICE.txt`
> * `crates/gts/tests/composite/PROVENANCE.md`
> * `crates/gts/tests/composite/ietf-cose-example.txt`
> * `crates/gts/tests/composite_support/mod.rs`
> * `crates/gts/tests/cose_composite.rs`
> * `crates/gts/tests/hedged_writer.rs`
> * `crates/gts/tests/mldsa65.rs`
> * `crates/gts/tests/mldsa65/NIST-NOTICE.txt`
> * `crates/gts/tests/mldsa65/PROVENANCE.md`
> * `crates/gts/tests/mldsa65/keyGen.txt`
> * `crates/gts/tests/mldsa65/sigGen.txt`
> * `crates/gts/tests/mldsa65/sigVer.txt`
> * `crates/hash-conformance/Cargo.toml`
> * `crates/hash-conformance/tests/shake.rs`
> * `crates/hash-conformance/tests/vectors/shake-PROVENANCE.md`
> * `crates/hash-conformance/tests/vectors/shake_boundary_vectors.txt`
> * `crates/hash-conformance/tests/vectors/shake_nist_vectors.txt`
> * `crates/hash/PROVENANCE.md`
> * `crates/hash/README.md`
> * `crates/hash/src/block.rs`
> * `crates/hash/src/lib.rs`
> * `crates/hash/src/secret.rs`
> * `crates/hash/src/sha3.rs`
> * `crates/rdf/Cargo.toml`
> * `crates/rdf/src/gts_certify.rs`
> * `crates/rdf/src/gts_write.rs`
> * `crates/rdf/tests/gts_certify.rs`
> * `crates/rdf/tests/gts_composite_compaction.rs`
> * `helpers-ledger.toml`
> * `scripts/check-issue-refs.py`
> 
> </details>
> 
> </details>

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autopilot</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

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

## 3. paudley — 

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

## 4. paudley — 

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

## 5. paudley — 

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Stage 2 remediation plan

Issue #458; PR #464. Captured head `52988974f2d11a40214281648983f9145af7a3fb`, tree `43b0817e30a3ad110cd013796a064c177a27e067`. Worktree and Stage directory are those recorded by stagectl in `open.json`. The immutable Stage 1 plan remains SHA256 `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

Independent requirement/history/gap/completion and specialist reports were read. Stage 1's functional qualification does not establish closure of the newly found defects. Stage 2 is open. No scope cuts or deficiency entries are authorized.

| Item | Finding and complete fix | Executable acceptance | Commit/publication |
|---|---|---|---|
| G1, MEDIUM | Nonstreamable eager/evented reads collect provenance they cannot consume. The same 50,000-subject input adds 2,228,396 requested allocation bytes and 15 allocations. Gate shared reader-local observation by actual streamable segment capability; preserve the single classifier and materialized RDF projection. | Replay unchanged `raw/S2-reader-performance.rs` with the same corpus/profiles in both modes. Remove subject-cardinality-dependent unused allocation; retain genuine signed packaging. Exercise segment reset, foreign predicates before/after type, ordinary reserved-class data, invalid/non-UTC source shapes, valid UTC and exact carried COSE through native and actual wasm consumers. No noisy timing percentage is an acceptance claim. | One coherent signed hook-verified reader fix, independent `tasks/G1-review.md`, push/readback and both issue/PR evidence updates. |
| G2, HIGH hosted acceptance | Three failed CodeQL hard-coded-value annotations are independently supported false positives: FIPS Algorithm 7 mandates internal mask counter κ=0 after deriving the private seed; fixed Encrypt0 key/IV are a registered single-encryption opaque-authentication test. Capture full native alert IDs/instances and make precise forge false-positive dispositions. | Read back exact alert scope/state and current required check. Preserve standard counter, test assertions and CodeQL rule/policy. Extraction success is distinct from alert acceptance. | No speculative source change. Forge disposition receipt plus both issue/PR updates and independent exit adjudication. |
| G3, HIGH security | Directly owned SHAKE buffer/lane state contains private K/keygen/mask input and lacks destruction. Complete controlled-state cleanup on finalize, drop, clone, success, error and unwind, including sensitive sampling scratch. Preserve one Keccak/XOF and one governed clearing home reachable from the zero-dependency hash root. A coherent relocation/re-export of the existing clearing primitive is permitted; no second wipe body or new runtime edge. Correct the current disclosure to the implemented boundary. | Safe live-storage tests in owning modules observe complete buffer/lane clearing and transfer/lifecycle, rather than reading freed storage or counting drops alone. Execute complete SHAKE/SHA3 answers, all official ML-DSA deterministic/hedged fixtures, composite primary/refusal cases and actual native/wasm Writer/verification/compaction/certifier callers. Check dependent Ed25519 clearing callers, warnings, helper/layer/root/dependency and affected metadata. Preserve exact output and Writer atomicity. | Separate coherent signed hook-verified security fix, independent `tasks/G3-review.md`, push/readback and both issue/PR evidence updates. |
| W1 integration | Current base advanced to `090b14bb8c00e2504078cb694d278ee2d614443a`; initial CI used `0d6575a46088e9420d73654b7b5965b9580c5d36`. Clean predicted tree alone does not qualify integration. | Refresh base/head/predicted tree; inspect merged lock/dependency/policy inputs and run newly affected closure checks. Record qualifying upstream/head reuse honestly. Stage 3 repeats assessment if base advances. | Integration evidence/publication; source commit only if a real conflict/behavior defect exists. |
| W2 feedback/completion | Fresh feedback, current-source qualification and independent exit audit remain required. CodeRabbit's capped green status supplies no substantive review. | Capture complete paginated comments/reviews/inline/threads and current checks after fixes and useful processing time. Run missing/invalidated checks, maintain validation source/artifact identities, and obtain a fresh independent Stage 2 completion verdict using actual production entry-point evidence. New required findings return to remediation. | Final Stage 2 summary on issue/PR only after actual closure; then continue Stage 3 through ghprsq and verified archive/integration/cleanup. |

Implementation is sequential, one source mutator, one coherent fix per commit. Parent performs normal signed commits/pushes and Stage-authorized publications after independent PASS. All hooks remain enabled. Source main, siblings, immutable governed GTS vectors, unrelated dirty work and original failed review/probe history are preserved. Existing executions are reused only for unchanged qualified closure; neither an old full gate nor another head's CI is relabeled current.

G1 executes first; G3 follows after its commit. G2 native alert inspection/disposition and read-only integration preparation can proceed while the sole implementation agent works. The thread allocation limit requires reusing an idle implementer for a fresh gap assignment; independent reviewers remain separate from source implementation.

Material specialist reports: `reviews/S2-performance.md` SHA256 `9b4ee49c863a103110b84cd9835fa8438d95a1a0ab338ddc44d7b953403401d2`; `reviews/S2-security.md` `d377e71d54a97d57a846b050df2f082a600d57d74441d283263ee4420deea068`; `reviews/S2-quality-structure.md` `cb7ac5d9f302eda0e0f4f542f0fe6f44c20369ae1a9a96dcd639e0f56ced3a01`. Required owned-state clause is [FIPS 204 §3.6.3](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf); this plan claims no hardware/compiler historical-copy eradication or formal certification.

## 6. paudley — 

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

G2 is closed as an evidenced false positive on head `52988974f2d11a40214281648983f9145af7a3fb`. GitHub alerts [235](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/235), [236](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/236) and [237](https://github.com/Blackcat-Informatics/purrdf/security/code-scanning/237) now read back as dismissed, reason false positive, including their exact-head instances.

Alert 235 identifies the internal ExpandMask counter κ. [FIPS 204 Algorithm 7](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf) derives the private per-signature seed from K, caller randomizer and message representative, then starts κ at zero and increments by five. The actual implementation reserves each five-counter group and refuses exhaustion before reuse. This is a specified sampler counter; production Writer still requires a fallible fresh-randomness provider.

Alerts 236/237 identify public fixed key/IV bytes in the registered single-encryption test `file_integrity_rejects_header_chain_and_torn_damage_but_allows_opaque_encryption`. The test signs controlled opaque ciphertext and verifies without decryption material, asserting valid author authentication plus MissingKey. These bytes are no production secret/default. Production Encrypt0Options takes caller key and IV. No source constant, rule, workflow or assertion changed.

Normal dismissal requests initially failed HTTP 422 because comments exceeded GitHub's 280-character limit. Specific bounded comments succeeded; fresh readbacks confirm the outcome. All failures and original annotations remain preserved in Stage. Independent `tasks/G2-review.md` is PASS, SHA256 `562c765413300db48a6d2b5dd5d18295c780819de25828bd3728380e53170542`.

The [CodeQL alert check](https://github.com/Blackcat-Informatics/purrdf/runs/112352384442) now reports completed SUCCESS on that exact head; readback SHA256 `a1ed978e564fb8e60e2729adc84d9f0cfb60c90936fa15e29dd392299973da3d`. Original annotation count remains three. Initial CI run 37487670367 also completed SUCCESS at head529 and captured base `0d6575a46088e9420d73654b7b5965b9580c5d36`; this qualifies that captured input, not the later source/base.

G1 reader allocation and G3 owned SHAKE secret cleanup remain required/open. G1 implementation is in progress. Neither this disposition nor initial green CI establishes Stage 2 completion.

## 7. paudley — 

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

G1 is fixed in signed/pushed commit [ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6](https://github.com/Blackcat-Informatics/purrdf/commit/ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6), tree `dacde4562e8f79d857ce3aa57dbe464ddaf651db`. Fresh remote branch readback matches.

The shared reader now observes provenance only when its actual segment header declares streamable layout. Both eager and evented readers use that capability. All streamable facts, including foreign predicates before/after type, still reach the same classifier. Materialized RDF projection retains its independent classification. New public controls exercise both segment orders, exact detached authorship, mandatory composite packaging, Ed repacking and actual certification.

The unchanged original 50,000-subject allocation probe removes exactly 15 allocations and 2,228,396 requested bytes in each ordinary reader mode, restoring original-base totals. Repeated subjects remove six allocations/4,380 bytes. Genuine-pack allocation totals and accepted row counts remain unchanged. The independent reviewer reproduced the result; no timing percentage is claimed.

Affected native qualification passed 1,362 cases plus 13 docs. After repairing a new assertion-style lint, the frozen public target passed all 12 groups natively and in actual Node/WebAssembly, with zero failures/ignores. Workspace all-target denied-warning clippy, frozen target clippy, strict affected docs, helper/layer/profile/generator/format checks and normal release GTS wasm build passed. The affected package run's unchanged groups are attributed to their earlier test identity; the entire changed public target was rerun on final bytes. Old full-gate/CI results retain their original identities.

Independent `tasks/G1-review.md` PASS SHA256 `ab8f3a721aa6aa8fd8e7d8cd596f2b532aafa135b721a418f3da16fa775f7771`; implementation report `0b3916c8b5047e8b4f52e397ed86592f7bc12b3b26c14a0993744fc45b01a91c`. Branch file manifest SHA256 `7b6b35e4674b44fdecbce3ead80825705b56766a9ed23fb36db4e4b10abe35d0` still matches after transport. Explicit two-file staging, separate cached whitespace check, normal signed commit/hooks and normal push all succeeded. Commit log SHA256 `ed3c5743aa488cecd653be1c6b77a274fa089489de74aeb5881ac9aae4172bdc`; push log `7c7773054867d724af7413d5486b0c56acbc0469c238873b3e41e533864c16e0`. Source/index are clean except selected Stage evidence.

Original failures and BLOCKED performance evidence remain preserved. Parent's initial evidence-manifest check used the Stage directory for worktree-relative paths and failed; the corrected worktree invocation verified all 58 entries. No missing file or failed check is represented as passing.

G3 owned SHAKE secret-state cleanup is next and remains required/open. Fresh hosted checks, current-base integration, independent Stage 2 exit and Stage 3 merge/archive/cleanup remain unverified.

## 8. paudley — 

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Stage 2 remediation plan

Issue #458; PR #464. Captured head `52988974f2d11a40214281648983f9145af7a3fb`, tree `43b0817e30a3ad110cd013796a064c177a27e067`. Worktree and Stage directory are those recorded by stagectl in `open.json`. The immutable Stage 1 plan remains SHA256 `dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a`.

Independent requirement/history/gap/completion and specialist reports were read. Stage 1's functional qualification does not establish closure of the newly found defects. Stage 2 is open. No scope cuts or deficiency entries are authorized.

| Item | Finding and complete fix | Executable acceptance | Commit/publication |
|---|---|---|---|
| G1, MEDIUM | Nonstreamable eager/evented reads collect provenance they cannot consume. The same 50,000-subject input adds 2,228,396 requested allocation bytes and 15 allocations. Gate shared reader-local observation by actual streamable segment capability; preserve the single classifier and materialized RDF projection. | Replay unchanged `raw/S2-reader-performance.rs` with the same corpus/profiles in both modes. Remove subject-cardinality-dependent unused allocation; retain genuine signed packaging. Exercise segment reset, foreign predicates before/after type, ordinary reserved-class data, invalid/non-UTC source shapes, valid UTC and exact carried COSE through native and actual wasm consumers. No noisy timing percentage is an acceptance claim. | One coherent signed hook-verified reader fix, independent `tasks/G1-review.md`, push/readback and both issue/PR evidence updates. |
| G2, HIGH hosted acceptance | Three failed CodeQL hard-coded-value annotations are independently supported false positives: FIPS Algorithm 7 mandates internal mask counter κ=0 after deriving the private seed; fixed Encrypt0 key/IV are a registered single-encryption opaque-authentication test. Capture full native alert IDs/instances and make precise forge false-positive dispositions. | Read back exact alert scope/state and current required check. Preserve standard counter, test assertions and CodeQL rule/policy. Extraction success is distinct from alert acceptance. | No speculative source change. Forge disposition receipt plus both issue/PR updates and independent exit adjudication. |
| G3, HIGH security | Directly owned SHAKE buffer/lane state contains private K/keygen/mask input and lacks destruction. Complete controlled-state cleanup on finalize, drop, clone, success, error and unwind, including sensitive sampling scratch. Preserve one Keccak/XOF and one governed clearing home reachable from the zero-dependency hash root. A coherent relocation/re-export of the existing clearing primitive is permitted; no second wipe body or new runtime edge. Correct the current disclosure to the implemented boundary. | Safe live-storage tests in owning modules observe complete buffer/lane clearing and transfer/lifecycle, rather than reading freed storage or counting drops alone. Execute complete SHAKE/SHA3 answers, all official ML-DSA deterministic/hedged fixtures, composite primary/refusal cases and actual native/wasm Writer/verification/compaction/certifier callers. Check dependent Ed25519 clearing callers, warnings, helper/layer/root/dependency and affected metadata. Preserve exact output and Writer atomicity. | Separate coherent signed hook-verified security fix, independent `tasks/G3-review.md`, push/readback and both issue/PR evidence updates. |
| W1 integration | Current base advanced to `090b14bb8c00e2504078cb694d278ee2d614443a`; initial CI used `0d6575a46088e9420d73654b7b5965b9580c5d36`. Clean predicted tree alone does not qualify integration. | Refresh base/head/predicted tree; inspect merged lock/dependency/policy inputs and run newly affected closure checks. Record qualifying upstream/head reuse honestly. Stage 3 repeats assessment if base advances. | Integration evidence/publication; source commit only if a real conflict/behavior defect exists. |
| W2 feedback/completion | Fresh feedback, current-source qualification and independent exit audit remain required. CodeRabbit's capped green status supplies no substantive review. | Capture complete paginated comments/reviews/inline/threads and current checks after fixes and useful processing time. Run missing/invalidated checks, maintain validation source/artifact identities, and obtain a fresh independent Stage 2 completion verdict using actual production entry-point evidence. New required findings return to remediation. | Final Stage 2 summary on issue/PR only after actual closure; then continue Stage 3 through ghprsq and verified archive/integration/cleanup. |

Implementation is sequential, one source mutator, one coherent fix per commit. Parent performs normal signed commits/pushes and Stage-authorized publications after independent PASS. All hooks remain enabled. Source main, siblings, immutable governed GTS vectors, unrelated dirty work and original failed review/probe history are preserved. Existing executions are reused only for unchanged qualified closure; neither an old full gate nor another head's CI is relabeled current.

G1 executes first; G3 follows after its commit. G2 native alert inspection/disposition and read-only integration preparation can proceed while the sole implementation agent works. The thread allocation limit requires reusing an idle implementer for a fresh gap assignment; independent reviewers remain separate from source implementation.

Material specialist reports: `reviews/S2-performance.md` SHA256 `9b4ee49c863a103110b84cd9835fa8438d95a1a0ab338ddc44d7b953403401d2`; `reviews/S2-security.md` `d377e71d54a97d57a846b050df2f082a600d57d74441d283263ee4420deea068`; `reviews/S2-quality-structure.md` `cb7ac5d9f302eda0e0f4f542f0fe6f44c20369ae1a9a96dcd639e0f56ced3a01`. Required owned-state clause is [FIPS 204 §3.6.3](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf); this plan claims no hardware/compiler historical-copy eradication or formal certification.

## G3 implementation-review addendum

G1 is signed/pushed as `ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6` and G2 has exact-head false-positive receipts. G3 remains active. Its independent provisional ownership review identified borrowed SHA3 finalization panic edges: clear the old live lanes before publishing the digest, and guard the internal digest temporary until copying to caller-owned output succeeds, including unwind. Public returned digest arrays retain their existing API. Safe live-storage checks and the actual caught-panic route must prove the corrected owned lifetime; historical compiler copies remain a separate limitation.

G3-P1 is REQUIRED/MEDIUM performance remediation within this same security commit. The initial independent report `tasks/G3-performance-review-initial.md` SHA256 `ae6e441e283205807213a84e193d0cdcc24936feed8af10faaa502f0b178807c` finds avoidable 72 scratch wipe/fence calls per Keccak permutation. Actual paired normal-profile direct Keccak and SHA3 ranges are disjoint; MD5/SHA1 controls and noisy whole-operation samples are preserved. Replace per-round owners with one guarded c/d/b workspace reused across all rounds, fully overwriting every slot before each read and clearing the complete workspace at permutation last use and unwind. Security review approves this active-storage lifetime; guards and cleanup remain mandatory. Let current required checks finish normally before editing. Replay the same paired workloads, exact signature identity and affected native/actual wasm storage/primary/caller checks on the corrected frozen source. No arbitrary timing percentage gate or repeated unrelated full suite is required.

Parent reads both final `tasks/G3-review.md` and `tasks/G3-performance-review.md`, then performs the same normal signed commit, push/readback and both issue/PR publication boundary already specified for G3. Required findings remain open until exact-source acceptance; original failures and provisional measurements stay preserved.

W1 evidence correction: earlier PR/run base fields record `0d6575a46088e9420d73654b7b5965b9580c5d36`; those metadata fields alone do not establish an executed base. Captured G1 workspace checkout actually uses generated merge `edd84fceebd90cbc330f2840af0fab2502445702`, parents current main `090b14bb8c00e2504078cb694d278ee2d614443a` and G1 head `ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6`, tree `9e3a5cce0482cec001f2f6fb23be17becaaa2e48` matching the clean local prediction. CI 37495496562 and all 40 jobs passed on these captured inputs. G3 changes invalidate their affected closure; final head/current-base qualification, full feedback and independent exit remain required.

## 9. paudley — 

G3 is fixed and committed as `241061e7cc08d76de97f8a597074ec5c5fcb2ec7` (`fix(crypto): clear owned sponge and sampling storage`), tree `02cde50a6ed4c0c79ad97183a24267e82889fff3`. Normal commit signing and every staged-index hook completed successfully; qualified source bytes are unchanged by transport.

The required finding was directly owned secret-fed SHAKE buffers and absorber/reader lanes lacking destruction, contrary to FIPS 204 section 3.6.3. One unchanged safe-Rust wipe body now lives in the zero-dependency hash root and is re-exported by Ed25519. Allocation-free guarded arrays clear complete buffers, lanes and operation scratch at last use or drop, including clone, error and unwind. Two finite SHA3 short-output panic findings also close: live lanes/full buffer clear before caller copy, and the private digest transfer is guarded until that copy succeeds. Actual tests inspect zeroed storage while its owner remains live.

Independent performance review found avoidable per-round scratch cleanup. The corrected Keccak uses one complete-overwrite guarded workspace across all24 rounds. Six independent interleaved measurements and optimized assembly qualify that correction without inventing a stable hardware timing percentage. All six ordinary/genuine-pack reader allocation cells still exactly match accepted G1 results.

Current affected native qualification passes255 cases, including56 ownership cases; actual Node/wasm passes55 public groups, plus10 repeated hash groups after a private helper spelling correction. The complete corpora include all70 official NIST ML-DSA records, the pinned complete IETF composite answer, unchanged SHAKE boundary answers, digest differential records and RFC8032 vectors. Real Writer/provider/keyring and RDF pack/certify/repack consumers pass, including both-component refusals, entropy-failure atomicity, exact authorship and timestamp/capability controls. Workspace all-target warnings, strict affected docs, helpers/layers/root/profile, frozen corpus/dependency/feature checks, generators, normal ratchet and four affected release-wasm libraries pass. Historical full-gate evidence is reused only for unaffected inputs; preserved failed legacy wasm selections and the obsolete negative probe are excluded from acceptance.

Independent security/structure PASS: `e300b1a069b25d769c6e8c995b613bca29bc83a40dcf8c0a08a5e6c564501649`. Independent performance PASS: `f18eea2a6c32a80f20b338b9ac9a22cc014209ce4fb5fbe3ac929cd7379d5d50`. Complete56-file branch manifest SHA256: `14fb06bbc20acfd865738f4402cfc1030b6815232603f6f49ae11e1635fb2696`. Parent verified every source/checksum and the lossless preserved baseline archive.

Owned SHAKE cleanup is now included in the explicit guarantee. Caller copies, historical compiler copies, registers/spills, external SHA-512 state and physical/timing/FIPS certification are outside that source-level ownership claim. Refreshed confidence answers and PR body record these limits accurately. Fresh hosted CI, complete feedback refresh, current-base integration and independent exit/Stage3 acceptance are still being performed; this checkpoint does not claim merge readiness.

## 10. paudley — 

What am I least confident about, and why?

Compiler, JIT and hardware side-channel behavior. Complete official NIST and pinned IETF answers, native and actual wasm execution, independent source review and fixed-schedule arithmetic establish functional behavior. They do not establish a complete-operation timing or fault-resistance proof. FIPS rejection sampling remains variable-time; no formal FIPS certification is claimed.

What should the user know?

The construction is pinned to JOSE/COSE draft-04 and LAMPS draft-19. The requested provisional COSE identifier -58 is within the official IANA unassigned range in the latest captured refresh; standalone ML-DSA-65's -49 is a different algorithm. The governed GTS upstream at 0d1c8299c9411ea4ead853e31721d42ea66f081e publishes its two EdDSA Sign1 vectors and no composite vector. Complete primary composite answers therefore do not establish interoperability with another GTS engine. These external facts must be refreshed before integration.

One detached Sign1 authenticates through BOTH components. Production callers supply dedicated independent component keys and a fresh full cryptographic randomizer for every signature. The Writer refuses provider failure atomically. The portable library cannot detect external key reuse or certify a provider's successful output as high-quality entropy.

Controlled owned SHAKE input buffers, absorber and reader lanes, permutation/sampling scratch and private polynomial/byte arrays now clear at last use or drop through one shared safe-Rust home. Clones, error exits and unwinding retain guarded ownership. Live-storage tests also cover borrowed SHA3 finalization's short-output panic, including its private digest transfer. Caller-owned output/export copies, historical compiler copies, registers, spills and external SHA-512 state remain outside that ownership guarantee; physical memory erasure is not claimed.

Actual native and wasm producer/consumer controls prove malformed or non-UTC provenance retains authored COSE/content, invalid producer parameters refuse before randomness, and valid explicit UTC spellings retain their bytes. Ordinary segments regain the exact baseline allocation counts in both readers; genuine signed packaging and independent materialized projection remain exercised.

The immutable approved plan retains its original planning snapshot. Signed checkpoints and source-bound qualification record implementation. Local qualification, independent review, current-head hosted CI and Stage 3 integration are distinct acceptance states; the latest public checkpoint records which have completed.

