<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Stage 2 independent historical-fit assessment

VERDICT: FIT. No new required historical-fit or doctrine finding established on
the captured issue head. This permits gap analysis; it is not completion,
security certification, current-base integration, hosted CI or merge acceptance.

## Identities, authority and coverage

Repository /home/paudley/Active/purrdf; isolated worktree
/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Issue 458; PR 464; branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Independent read-only Git inspection confirms HEAD
52988974f2d11a40214281648983f9145af7a3fb and tree
43b0817e30a3ad110cd013796a064c177a27e067. Source/index are clean; selected .stage
evidence is untracked. All 52 source paths independently passed the final
manifest readback. Manifest raw/T6-R1-final-branch-files.sha256 SHA256:
e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d.
Immutable plan.md SHA256 independently remains
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

Implementation captured base is ce3c07192aba1e36666062c00f958670a827cfb5.
Captured PR publication/review-thread base is
0d6575a46088e9420d73654b7b5965b9580c5d36. During this assessment the local
origin/main ref already resolves to 090b14bb8c00e2504078cb694d278ee2d614443a.
No ref was fetched or mutated by this assessor. Parent was notified of the
newer observed ref. No branch-local proof is relabeled integrated-tree proof.

Read Stage 2/Stagectl skills and shared quality, validation, delegation and
no-deferrals doctrine; worktree AGENTS.md, user-supplied standing instructions
and root .baseline/.goals (absent inside the worktree); GTS-SPEC §§9.2, 10.1
and 13.3; helper registrations; issue.md including ALL NINE comments; fresh
prior-art.md/brief.json and branch-under-review.md/review-brief.json; immutable
plan and prior-art-assessment.md; original tasks/S1-* intake; all T1–T6 current
reviews and material corrections, including original T3/T4/T5 BLOCKED reports
and T6 timestamp findings. No applicable ADR is identified in captured briefs
or affected-path inventory.

Read publication metadata and exact retained PR body/confidence/plan readbacks.
The PR is OPEN, unmerged, with the captured exact issue branch/head. Full initial
review API artifacts raw/S2-{review-submissions,inline-comments,review-threads}.json
contain complete empty review surfaces; reviewThreads has hasNextPage=false and
exact head/base. Empty surfaces are absence of submitted review in this snapshot,
not approval. CodeRabbit's success check accompanies an explicit usage-cap notice;
it supplied no completed review. Hosted CI is pending in supplied capture.

A quick memory registry lookup supplied the evidence-preservation lesson:
historical receipts qualify their captured inputs, and staged/unstaged/source
evidence must remain distinct. No historical checkout result supplies a current
implementation premise. That lesson is also independently present in current
Stage validation doctrine and the actual preserved task artifacts.

No source/index/forge/private-memory mutation, model/lifecycle action, child
delegation or test execution occurred. Only this selected Stage report is written.
Fresh forge retrieval would duplicate parent-owned evidence; missing tests are
for named gap/completion assessment, not an automatic historical-fit rerun.

## Precedent and recurrence: actual evidence, not title inference

Original intake had zero issue comments, no linked ADR/issues, no related title
hits and three "none" defect trailers in the last 200 commits. Fresh intake has
nine comments, a self-link to this issue, no related title hits and five "none"
trailers. Neither count measures composite-signature defects. Composite-defect
recurrence is UNKNOWN, not zero or five. This bounded search also cannot prove
the absence of every cryptographic precedent across other repositories.

Technical precedent is concrete: GTS §9.2 permits algorithm-agile Sign1 and
requires honoring its declared algorithm; §10.1 preserves original frame-id/COSE
authorship through ordering rewrites. Native Ed25519, SHA-512, Keccak, CBOR,
XSD temporal parsing and fixed hashers already have governed homes. The branch
extends those homes rather than introducing independent competing primitives.

Original actual caller inspection found an Ed-only writer/keyring/certifier and
a parser ignoring protected alg, full consumption, outer tag and detached payload.
These were behavioral gaps, not speculation inferred from the issue title.
The chosen implementation addresses that initial architecture instead of leaving
an isolated composite helper unconsumed.

The strongest recurring evidence is from this very branch's preserved
independent counterexamples: strict parsing initially missed malformed textual
content types; unsigned verification bypassed integrity/profile policy;
provenance classification confused authored content with packaging; root
commitments accepted ambiguous/nonliteral values; and timestamp classification
still lost authenticated history after a green broad gate. This is a recurring
boundary/claim problem in the inspected work, not a measured historical defect
class count. Each actual failure remains visible alongside its later closure.

## Fit of the chosen design to standing doctrine and requested behavior

The original first-party ML-DSA home is justified by the concrete dependency
constraint: the inspected external candidate requires banned signature, and
weakening that ban would contradict current repository law. The implementation
lives once at purrdf_gts::mldsa65; its polynomial roots are const-generated from
1753 raised to bit-reversed exponents rather than an imported coefficient array.
Its module openly distinguishes pure-message ML-DSA from unrequested HashML-DSA.
SHAKE remains in purrdf_hash::sha3 over the existing Keccak/buffer; native
Ed25519, clearing/comparison, sha2 SHA-512 and lexical CBOR remain shared homes.
GTS adds an existing sha2 workspace runtime edge, not a new external package.
Public test targets use existing Rust testkit. This fits one-home, first-party,
no-semantic-feature and wasm constraints without speculative new release crates.

The provisional wire contract follows the authorized plan and issue's IETF
construction request: JOSE/COSE draft-04 plus LAMPS draft-19, requested -58,
explicitly unassigned rather than a final registry claim. Actual composite code
uses Prefix || Label || 00 || SHA512(Sig_structure), gives the same representative
to both components and the fixed Label context to pure ML-DSA. Public/key/signature
encodings put ML-DSA before Ed25519; acceptance executes BOTH checks and requires
their conjunction. Algorithm/key mismatch cannot fall back to Ed25519.

The one strict Sign1 parser consumes the full CBOR item, validates supported
protected alg, tag/shape/detached null, exact component length/canonical encoding,
duplicate/cross-bucket headers and understood critical instructions. Exact
received protected bytes survive into the preimage. Legacy Ed conveniences call
that same parser/verifier; the new typed public APIs accept composite material.
Keeping the legacy Ed key types is coherent because they cannot represent a
composite key; it does not create a second implementation. An invalid/unsupported
envelope fails before lookup, while a supported unresolved envelope stays
Unverified. Optional, empty and binary IDs stay distinct. OpenPGP remains the
existing Ed discovery surface; inventing composite discovery/trust policy would
exceed the issue.

Actual Writer::add_frame_with_options forms the ID and signs before modifying
offset/type/frame-id/output/head state. Composite installation consumes Writer
into sealed Hedged<P>, whose conveniences return Result and whose provider is
mandatory. The provider fills one fresh 32-byte caller randomizer; errors reach
WriterError. There is no production deterministic fallback, OS entropy lookup
or clock policy. Deterministic primary-fixture methods are explicitly named.
Caller entropy quality, full successful fill, fresh inputs and dedicated
independent seeds are boundaries the library cannot infer from supplied bytes,
not unauthorized missing implementation. Guarded owned storage clearing is
accurately limited; neither functional vectors nor this design establish
compiler/JIT/hardware timing or destruction of historical copies.

The actual compactor has one mandatory sealed PackagingSigner contract. Ed and
composite implementations both sign the final ordering index through Writer;
composite finish propagates provider/signing failure before returning bytes.
Certification retains its deliberate textual packaging-kid schema while carried
authorship resolves opaque IDs. This preserves mandatory signing and existing
certificate bytes instead of silently making packaging optional.

Actual signatures_bound_ok compares exact pre/post author pairs, validates the
current source-head-selected root and proofs; signatures_verify_ok uses the
same typed parser/resolver/verification core; packaging_signature_ok authenticates
the actual final valid streamable index and requires no invalid/unverified pack
signatures. A valid unrelated survivor cannot replace the ordering commitment.
The compactor's structural refusal gate lacks supplied key material and must not
be called cryptographic author authentication; the real certifier supplies that
distinct verification. Current publication prose makes that distinction.

## Failure history and corrected production boundaries

Preserved blocker hashes independently match their current artifacts:

| Original independent report | SHA256 | Current disposition/evidence |
|---|---|---|
| T3-review-initial.md | 606cae7c672d1895466043e612a0d5b84801f6b49a6e5965741e630b5da42009 | T3-review.md closes textual content-type grammar through the sole header validator, original unchanged consumer replay and native/wasm accepted/refused neighbors. |
| T4-review-initial.md | 6985401566fa7f3861786371a91f80d3466ffd542efe12a4f8e2171a09760a1e | T4-review.md closes EmptyFile/profile shortcut and governed Debug duplication; unsigned verification now calls the same result assembly, with the original five counterexamples and actual wasm controls. |
| T5-review-initial.md | c394393dfaebc0bf068f627d24663fb440010aad73d949e6f3b57d7928812db7 | T5-review.md closes lost authored index/root-shape findings through positive shared provenance and source-head-selected strict roots; unaltered public probe preserves exact author pair and refuses both malformed roots. |
| T6-timestamp-review.md | a5d69a181c58a373dc592df6aaa279be3a3b59de14ca68ba38c80b3b94cf462e | T6 qualification/final reviews close invalid/non-UTC source classification and invalid producer parameters using the existing shared XSD parser, with both original unmodified probes plus actual native/wasm pack/repack controls. |

Code inspection confirms the corrected shape requires positive blank-node
Compaction, closed predicates, singleton agent and valid explicit-UTC XSD
dateTime, and well-formed source heads. Both eager and evented readers use
ProvenanceSubjects and reset its state per segment; RDF content projection
uses that same home. Malformed/non-UTC/incomplete/foreign-predicate shapes retain
authored evidence rather than being discarded as packaging. Current producer
preflight uses the SAME is_utc_timestamp predicate and preserves legitimate XSD
lexical forms instead of a new RFC3339-only or Z-only parser.

These corrections fulfill the issue's decades-long history-preservation motivation:
new signatures must have both components from the first write, and rewriting
distribution order must not discard original authored signatures. They are
necessary consumer work, not gratuitous scope growth. Passing broad tests alone
did not discharge those findings; visible reproduced behavior and source-bound
closure did. The immutable plan/footer remains historical and is described as
such in current publication.

## Consumers, blast radius and remaining acceptance boundary

Consumers directly changed or traced: public COSE/ML-DSA/SHAKE APIs, actual Writer,
eager/evented GTS readers, file typed/legacy/OpenPGP verification, shared policy,
GTS streamable compaction/proofs, RDF content projection/certification and
transitive RDF/Python/C/wasm bridges. The new primitive is consumed by actual
signing and packaging. C/Python source adjustments and compilation do not
establish installed-language runtime acceptance; there is no separate new
composite key-discovery interface to claim. Frozen governed vectors have no
branch delta. Independent NIST/IETF answers remain distinct from shared GTS
composite interoperability.

The shared classifier observes ordinary quads and stores subject facts even
outside genuine provenance. This broadens the reader hot-path/memory closure;
the parent already identified it for scoped Stage 2 performance assessment.
I establish neither a slowdown nor optimal throughput from source reading.
The cryptographic implementation and substantial modules similarly warrant
the planned specialist judgments; this historical report does not replace them.

Captured external refresh at 2026-10-06T14:57:27Z identifies upstream gmeow-gts
0d1c8299c9411ea4ead853e31721d42ea66f081e, complete canonical tree
bbac332e919339ab0b1a9b99dcba841f0b434464 with only two EdDSA COSE fixtures,
and official IANA -58 unassigned. This meets the captured conditional-state
assessment; it is not a later merge refresh or shared-composite pass.

Local qualification is attributable: required broad make check passed on its
older captured snapshot; current GTS/RDF closure, docs, actual wasm runtime,
workspace clippy/generator/hygiene and release wasm were requalified after
T6-R1. Old full-gate output is not a new full-gate run on current source.
Signed source/publication receipts are separate from pending hosted CI.

The captured base-to-PR-base delta is 19 translation/prose/book-build/glossary/
license paths. The newly observed origin/main adds a further 15-path OWL DL/
validation/benchmark/changelog/Cargo.lock change. Thus any integration judgment
restricted to the earlier 19-path delta is now insufficient. The source head is
unchanged, but current-base candidate tree, Cargo.lock combination, applicable
build/policy checks and hosted merge-tree identities must be assessed freshly
by parent/Stage 3. No integration operation or claim occurs here.

No historical-fit finding remains open on the assigned head. Gap analysis and
independent real-entry-point completion auditing must adjudicate any remaining
behavior, specialists, feedback and exact current integration evidence before
Stage 3 acceptance and the exclusively authorized ghprsq merge.

