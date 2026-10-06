<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Stage 2 exit completion audit

VERDICT: PASS

No required source or execution finding remains for the captured Stage 2
result. G1, G3 and G3-P1 are FIXED; G2 is an evidenced false positive with
successful final-head analysis. Current combined-tree qualification is terminal
successful. The conditional shared composite corpus has not been published at
the captured authority identity: shared-engine interoperability remains
UNVERIFIED, rather than being claimed from primary answers. This PASS permits
the parent to complete Stage 2 accounting/publication and continue Stage 3. It
does not perform or qualify a later merge, release, deployment or cleanup.

## Exact authority and identities

STAGE=2; issue458/PR464; Blackcat-Informatics/purrdf.
ROOT=/home/paudley/Active/purrdf.
W=ROOT/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
S=W/.stage/purrdf-gts-composite-ml-dsa-65-ed25519.
Branch=paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.

Independently observed HEAD241061e7cc08d76de97f8a597074ec5c5fcb2ec7,
tree02cde50a6ed4c0c79ad97183a24267e82889fff3 and actual
origin/main090b14bb8c00e2504078cb694d278ee2d614443a. Captured implementation
merge-base is ce3c07192aba1e36666062c00f958670a827cfb5. Source/index are clean;
only selected Stage evidence is untracked. Independently verified all56 branch
files against raw/G3-frozen-branch.sha256, whose SHA256 is
14fb06bbc20acfd865738f4402cfc1030b6815232603f6f49ae11e1635fb2696.
The immutable approved plan remains SHA256
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

The clean predicted integration tree is
16680b5080728165b7e5da8ce139a8bcba0ff10d. Native forge commit identity
raw/G3-generated-test-merge-commit.json binds generated merge
b7df3cd5ee216e9435e1f37863c80a62743e7054 to exactly base090 and head241,
with that same tree. Its SHA256 is
3270e3ef61ac9b1d6a8b59d8da62fcaa97f093f5ce2b7ce2014d971f755ea6bd.
PR/run metadata's older base0d657 is not executed-checkout evidence. Actual
workspace, wasm-exec and Docs job checkout logs independently name b7df3cd
and print its full commit ID. This audit did not fetch the generated commit
into the local object database or pretend a local cat-file lookup succeeded.

Applied the FULL completion-adversary, Stage2/Stagectl, validation, quality and
no-deferrals instructions. Read actual applicable AGENTS, ROOT baseline/goals,
both deficiency ledgers, issue title/body/all15 comments, immutable plan,
issue-analysis/historical-fit/prior-art, initial completion/specialist reports,
material T1–T6 reviews and preserved failures/corrections, current remediation
and gap dispositions, ENTIRE G3 implementation and both final specialist reports.
No governing ADR/constitution was identified by the captured brief/inventory.
There is no user-authorized cut or ledger entry below either marker.

This auditor is distinct from the initial completion auditor and implementation
and gap roles. Independently inspected production source and substantive public
assertions, actual terminal logs, manifests, artifact/profile attribution,
transport/publication readbacks, current integration delta and final hosted
receipts. No source/index/forge/private-memory mutation, child delegation,
model/GPU/NPU/server/deployment/lifecycle action or duplicate blanket suite
occurred. Only this audit and its selected raw scan/hash witnesses were written.

## Criterion-by-criterion production demonstration

E1–E9 below identify actual commands, source applicability and outcomes.
Negative tests are assessed beside successful production outputs; rejecting a
requested composite input is not the implementation's completion claim.

| Requirement / accepted contract | Actual demonstration and observed versus required behavior | Judgment |
|---|---|---|
| EdDSA and composite share one detached Sign1 and dispatch on protected alg | E2 calls public typed sign/parse/verify; asserts tag18, four fields, null detached payload, exact protected -58 and3373 signature bytes. Both frozen Ed envelopes retain exact -8 bytes. Real Writer authors Ed/composite/Ed/composite history, reader observes four rows and supplied-key file verifier returns `(signed,valid,invalid,unverified)=(4,4,0,0)`. Legacy Ed convenience APIs call the same typed core; suffixes preserve existing key-typed contracts, not a second signer. | DEMONSTRATED |
| BOTH components mandatory; stripping, zeroing, replacing, swapping and downgrade refused | E2 public verifier attacks each3309/64-byte half, splices other-message/key halves, mixes public halves, reverses order, truncates/appends and changes protected alg. Valid controls precede Invalid assertions. Real Writer files and E3 carried/package signatures mutate each half and fail the corresponding cryptographic facet. Composite source evaluates both checks and combines their results. | DEMONSTRATED |
| Exact pinned IETF pairing and one authenticated preimage | E2 computes actual Sig_structure and compares the complete primary representative, public key and deterministic signature. Source uses Prefix32 || Label30 ||00||SHA512(Sig_structure), the same127-byte representative for pure ML-DSA under fixed label context and plain Ed25519. Public/key encoding is1952+32; signature3309+64. Attached primary example is refused at the detached GTS boundary; its unchanged detached preimage/signature genuinely verifies. | DEMONSTRATED |
| FIPS204 deterministic answers plus hedged variant | E2 executes all70 complete NIST records:25 keygen,15 deterministic,15 hedged,15 verification (3 valid/12 invalid), asserting full byte equality and exact counts. E1 runs four complete512-byte NIST SHAKE answers,16 boundary/long-output answers, digest/RFC corpora. The primary fixture audits are independent prior evidence, not PurRDF-generated expected bytes. | DEMONSTRATED |
| Explicit fresh-randomness production contract, actionable atomic failure | Actual sealed Writer requires a caller provider for composite installation. E2 signs two authentic outputs with distinct supplied randomizers and unchanged content IDs; provider failure/partial fill returns typed errors and preserves bytes/head/index state across13 append conveniences. Explicit recovery signs/verifies. Source publishes buffer/offset/type/id/head only after successful signing. E3 proves failed packaging returns no pack and fresh retry certifies. Successful provider quality/freshness is the caller's stated responsibility. | DEMONSTRATED |
| Actual typed key resolution and consumers; no trust/discovery scope expansion | E2 Writer→reader→file keyring uses exact optional opaque IDs and declared algorithm through the sole strict parser/verifier. Malformed/unsupported refuses before lookup, supported unresolved is Unverified, wrong type/key is Invalid, absent/empty/binary remain distinct. Existing OpenPGP Ed-only assembly routes through the same integrity core. Key discovery, trust anchoring and encryption remain the literal issue's exclusions. | DEMONSTRATED |
| Original signed history survives real pack/repack/certification | E3 authors mixed source via Writer, verifies it, calls compact_and_certify with Ed and composite mandatory signers, compares exact original frame-id/COSE pairs and refold digest, verifies MMR proofs and all certificate facets, then repacks and verifies again. Direct compact_streamable positive control also certifies. Final ordering index is authenticated, not replaced by an unrelated valid content signature. | DEMONSTRATED |
| Strict evidence/UTC/segment capability boundaries retain legitimate neighbors | E3 actual eager/evented, content projection, pack/repack and certifier controls preserve ordinary/incomplete/foreign/invalid/non-UTC authored COSE/content; strict root identity/cardinality and ordering attacks refuse. Invalid producer time refuses before provider invocation; valid explicit UTC/XSD spellings preserve bytes. Segment capability resets in both orders. Original unchanged T6 probes were independently rerun in their correction review; current registered caller groups execute again. | DEMONSTRATED |
| Controlled secret-fed owned storage destroyed at last use/drop | E4 safe owning-module tests inspect actual live zero lanes/full capacity and transfer, clones, error/unwind and caught short-output panic. Source guards SHAKE absorbers/readers, BlockBuffer, shared permutation workspace, private digest transfer and sampler/polynomial/byte scratch through one root wipe home. These unit tests qualify ownership; E1–E3 separately prove real output/caller behavior remains correct. External SHA512/compiler history is not substituted for owned cleanup. | DEMONSTRATED; G3 FIXED |
| Reader hot-path cost and mandatory cleanup efficiency | E5 unchanged50000-subject probe replays all6 allocation cells: ordinary unique removes15 allocations/2228396 bytes, repeated removes6/4380; genuine-pack counts/rows remain accepted. Independent final performance report executes6 interleaved optimized measurements and inspects assembly. One completely overwritten guarded workspace replaces72 per-round wipe/fence calls, while last-use/unwind clearing remains. No fabricated timing percentage threshold. | DEMONSTRATED; G1/G3-P1 FIXED |
| One native portable home, dependency/root/features/corpora policy | E1–E3 actually execute native and Node/wasm public callers, E6 qualifies current affected release wasm and declaration/generator/lint gates. Wipe relocates one unchanged body into zero-dependency hash root; Ed re-exports it. Existing SHA512/Ed/CBOR/XSD homes are reused, no new external package/semantic feature or governed-vector edit. E8 checks combined975-unit profile,207 layers/root and changed-base policy consumers. | DEMONSTRATED |
| Shared cross-engine vectors when specification publishes them | Full nontruncated upstream tree at0d1c8299/treebbac332e contains only sign1-basic and sign1-empty-id. E2 replays those governed bytes unchanged. No shared composite fixture exists in that captured corpus. Primary composite answers are accurately separated from shared-engine interoperability. | CONDITIONAL publication absent; shared composite interoperability UNVERIFIED, no current missing implementation requirement |
| CodeQL substantive alerts and final exact-head acceptance | G2 source trace identifies standard private-seed-derived κ counter and registered single-encryption fixed public fixture. E9 final native alert readback shows235/236/237 dismissed false positive with actual241 instances; all5 analyzers succeed and final CodeQL succeeds with zero annotations/no new alerts. Prior missing-Rust-configuration NEUTRAL is preserved but superseded, not accepted as security evidence. | DEMONSTRATED; G2 FALSE POSITIVE |
| Current combined inputs, full feedback, signed source and concrete publication | E7 independently verifies signature/head/manifests and exact5 body comparisons. Full paginated feedback has15 issue/10 PR comments, one old empty COMMENTED review and3 resolved current-mapped CodeQL threads with terminal outer/nested pages. E8/E9 prove current candidate qualification, all40 CI jobs success, Docs build success and final alerts. CodeRabbit's usage cap is not substantive approval. | DEMONSTRATED Stage2 exit |

## Executed checks, exact attribution and bounded reuse

All executions below are attributable existing demonstrations, independently
adjudicated here. They are not relabeled as this auditor's test execution.
The canonical21-record raw/G3-command-status.json SHA256 is
3aff5a2cf26f71be793cf27175d45246e5ccb7a683b2bd85ee2ada339b9fe617;
all21 log hashes independently read back. It records command, phase and exit.
Present126 compiled artifacts independently match raw/G3-runtime-artifacts.json
SHA25636ee70dd712b7ff6cc5ffb5691bf92f940e71f1d647c0490c3b6220f38dfd378;
own raw/S2-exit-completion-runtime-hash-readback.log records those readbacks.
Artifact inventory agreement is not alone execution proof: actual command logs,
source phases and substantive assertions establish each demonstration.

Local native target x86_64-unknown-linux-gnu, wasm target
wasm32-unknown-unknown; rustc1.100.0-nightly
4b6d04e706108ccfeafe2547fbe857dfe8972bad/LLVM23.1.1,
Node26.10.0/wasm-bindgen0.2.125. Cargo is locked/offline with jobs2, opt3,
assertions and overflow checks. Actual wasm runner invokes Node and requires
explicit completion; no build-only or zero-case output substitutes for runtime.

| Evidence | Actual command / terminal observation / log SHA256 |
|---|---|
| E1 primitives | `cargo test --locked --offline -p purrdf-hash-conformance --test shake --test digest_differential -p purrdf-ed25519 --test rfc8032`;13 registered cases pass, full corpora inside them, exit0. Native15241730459af10c5df96d5adc963d1d3f79c96cc8bd8bb76b161be9587da2b2; same target/runner wasm13 pass b6dca13628c0e88d10e95eb4b868b2162f58805c4e377eb6e7cc76cff0c55ef3. Post-name-correction hash wasm10 passes cbaa0f6aadb829f043b8f08937fc3505ad3042fe661cb76c2b00a92f1353927e. |
| E2 GTS producer/consumer | `cargo test --locked --offline -p purrdf-gts --lib --test mldsa65 --test cose_composite --test hedged_writer --test compaction_signatures`;157 cases pass, exit0, c67874b57e4a5a7c38043072dfffa22e68859cb47964e03d52b4922819d4742f. Actual wasm portable mldsa65/cose_composite/hedged_writer30 groups pass, exit0,3625778b3175bf9a23aa817216e5f62331008c8c52dcdeac3350e9a12efb4379. |
| E3 RDF real consumers | `cargo test --locked --offline -p purrdf-rdf --test gts_composite_compaction --test gts_certify`;29 cases pass, exit0, d961a9a30e838ba9167c4ea614b5af81fe481d4f6cc257032a5ffaeca105450f. Actual wasm public gts_composite_compaction12 groups pass, exit0,fff50d42bc6b0b0b3d1c94f2c242a1059fd236f6e28134800f5ec393714d8865. |
| E4 owners | Final name-corrected `cargo test --locked --offline -p purrdf-hash --lib`;56 ownership/root cases pass, exit0,cfc1806c0b22d04d777e3b2ebebf1e2723126eae6df62eff1a9434e4c77c68c4. Together E1–E4 total255 current affected native cases. E1–E3 total55 actual wasm groups, plus10 repeated hash groups. |
| E5 measurements | Existing Rust performance manifest `cargo test --locked --offline --manifest-path S/raw/S2-performance-head-probe/Cargo.toml --test performance -- head S/raw/S2-performance-corpus`;exit0,6 exact allocation cells,ab0d10b5bd743dbaf967b678a12e1026e5bc16865d30e423c33ba76add4146a4. Independent paired throughput/assembly execution and exact source/profile/artifact identities are in ENTIRE tasks/G3-performance-review.md SHA256f18eea2a6c32a80f20b338b9ac9a22cc014209ce4fb5fbe3ac929cd7379d5d50. |
| E6 current local declarations/consumers | Workspace all-target clippy -D warnings, affected post-name clippy, strict affected/post-name/post-prose rustdoc, helpers/layers/root/profiles, generated projection,4 affected release wasm libraries, normal frozen-tree ratchet and no-features all exit0 in21-record index. Normal profiles show974 units locally, not the combined975. Original primitive/source/fixture audits remain attributable; final security/structure PASS e300b1a069b25d769c6e8c995b613bca29bc83a40dcf8c0a08a5e6c564501649. |
| E7 transport/publication | Normal signed commit/hooks and push exit0, exact remote241 readback in tasks/G3-transport.md. Independently `git verify-commit HEAD` exits0 with AF5E0032F7494CEBCAA7BBBE9B87CFBBCFDBAF11. `cmp` current PR body, both confidence bodies and both G3-public-update bodies against native API exact readbacks each exits0. Wrong guessed update filename initially exited2; real inventoried G3-public-update.md comparisons corrected it, not a hidden successful check. |
| E8 combined CI/Docs | CI37506083735 attempt1 completedSUCCESS; all40 paginated jobs completedSUCCESS, none unfinished/skipped. Run SHA3e6b6b5fbd81a41855c7a0fcd1f4415d26b146a3c1b7e0bb8ea15d091de4d606; jobs9198b4c592090cbfbb24c5f97d28843e77b0fc0478de9c0c09e15145838546bf. Docs37506083612 completedSUCCESS: buildbookSUCCESS, normal PR deploymentSKIPPED; run2ecb3ad76cfb72439f1ed9f0f669cb5f47d7ad0fc7fe079e2e270a4bad33796e; jobs6a34a238c93267ab277d5ac7c739b97b2f7410f47bc9049f4078b97e86f53d15. Actual workspace logbf3a8006c6aa7c6a941a5a3e7f4e3cb995d743de76c9792b49a298cd1f151c94, wasm-execc2106e2cf54f0cc25c370e048f952f8ae38b5b7523eef7816218f8ab16e6a36f and Docs1fe41ee1d0a6d5b6073da7c094fd72a6c86bf31980f31fbad0b070e4f29a846e prove generated checkoutb7df3cd. |
| E9 final security acceptance | CodeQL dynamic37506077821 exact241 completesSUCCESS; all5 analyzer jobsSUCCESS (Rust112414960503 included). Run6900e50ad762b3c35d87b7086317b3f12cf9d89ad83c8d2ec6cc58095e37ad3c; jobsd6231d819477e34267e834a7564b7f346293d79c782e72ea1827c6a95dff235e. Final CodeQL112415270018 SUCCESS, no new alerts/zero annotations, d9bc039da11637c2f4aefb06a8492aa956bca8ff95cfff2dd03c7d7a5509ef6d. Paginated exact-head dismissed alerts a7173713864055140f4449b9cfc69202b71779ad311ab60bffb3e41067c921e6. |

E7's original five-body comparison is retained as its pre-terminal publication
snapshot. The parent then updated only the PR body's hosted-acceptance paragraph.
Independently compared the current tasks/G3-pr-body.md to the new native exact
readback raw/G3-pr-body-terminal-exact.md: cmp exit0, SHA256
1c92c2d304da88877badb5a51f3a287fd6705d09f5ff16218934154cbcb10bae.
Its exact current paragraph correctly names all40 CI jobs, generated merge,
base/head/tree, distinct compiler environments and final Docs/CodeQL/thread
acceptance. Original raw/G3-pr-body-exact.md remains the historical submitted
snapshot; its pending-CI text is not represented as the current PR body.

Report-only factual correction before finalization: the exact ASCII prefix
CompositeAlgorithmSignatures2025 is32 bytes, label30, separator1 and digest64,
giving127. The initial report's Prefix31 label was incorrect; actual source,
complete primary assertion and executed signatures were already correct. No
source or execution changed to correct this audit text.

Current CI floating nightly is DISTINCT: rustc1.101.0
ea137335b78829b4514bf1b4c16302f74fab8581/LLVM23.1.3. Actual native workspace
logs show combined denied-warning clippy,975 opt3/assertion/overflow units,
207 layer edges, zero-runtime-dependency root and normal changed-base
generator/translation/license gates. CI includes native six-shard/test aggregate,
bindings, conformance, actual portable wasm, package/oracle, architecture/SIMD,
MSRV and Miri gates. Its normal wasm-test selection has explicit filtered cases;
it is not falsely called execution of all55 local crypto/certifier wasm groups.
Local55 groups retain their actual compiler/artifact attribution; newer CI
combined native and all-crate wasm qualification complements them.

Independently read the supplemental actual current native shard execution
records, not just their job conclusions. The actual commands are
`make test-shard SHARD=lib`, `SHARD=integration-1` through `integration-4`,
and `SHARD=doc`, under the prescribed SIMD admission environment and optimized
assertion/overflow profiles; all six terminate successfully. CI selects Node24
separately from local Node26.10. All six shard checkout logs name
b7df3cd and the newer ea137 nightly. Integration1 executes complete IETF/frozen
COSE, both-component refusals and all11 public COSE groups. Integration2 executes
all13 real Writer/provider/file groups, all6 ML-DSA groups with complete NIST
answers, and all12 RDF pack/certify/repack/UTC/capability groups. Lib executes
all56 hash ownership/root and121 GTS cases, including actual guarded live-state
and caught-panic assertions. Integration3 executes all3 RFC8032 groups. These
are direct current combined-tree public-entry-point demonstrations in addition
to the source-applicable local native/wasm evidence.

Raw G3-ci-test-{lib,integration-1,integration-2,integration-3,integration-4,doc}
native-job log SHA256 identities, independently recalculated, respectively:
759c3df130315c4db3f81343f35e823fd36ee5201ad956e1217e6b38c8b299c1;
1c51134474b1998509f9574bfd621aae62839c66cf3ecd2c968e53a513459c78;
eb65f8e8c942d1a38fe6ff5aabcc751b2f40efeea7c73667462bc875a497ac8e;
c2062476877bdc98a1ab35ad77920f5a1925395d63c8e68b313c576d35b46a51;
266b40f65766238860ea3c77631f035f38bbe56c68fb7d264e5711ed88d117d6;
3df756b7edb5b760a29dc5e67cc1ae222fa60ce8b5af821bf1ff74071c4ed105.
raw/G3-ci-six-shard-accounting.json records647 terminal groups,21526 passed,
zero failed,37 default ignores and1 CLI argument-filtered case. Those ignored
or filtered cases are accounted separately and are not described as passing;
the affected composite/owner public groups shown above actually execute fully.

Optimized G3 executions precede only the spelling-only private keccak helper
rename and one documentation correction. The full implementation/reviews bind
the exact inverse delta and preserved phases; root/native hash and actual wasm
hash, affected lint/docs/hygiene were rerun afterward. No production behavior,
fixture or consumer assertion changed in that final delta. Current committed
56-file manifest matches the frozen qualified bytes. Reuse of unchanged E2/E3
outputs across that bounded delta is therefore valid, not a claim every old
binary was rebuilt on the final tree.

Historical full make-check remains21,045 native/460 docs on its own captured
source; historical default ignores and later failed runners are not passes.
Provisional G3 closure1583 cases is not substituted for changed optimized-owner
checks. Legacy wasm compaction_signatures/gts_certify runner failures remain
preserved and excluded; real portable RDF consumer executes the integrated
positive/refusal behavior. Original security negative probe now exits101 on
obsolete needs_drop=false assertion and is correctly EXCLUDED from acceptance.
Meaningful safe live-storage checks prove the repair instead. No stale full
suite is relabeled current, and no missing functional demonstration warrants
another unrelated full execution.

## Integration, feedback, scans and residual boundaries

Independently inspected actual candidate-versus-head34 paths and merged lock:
OWL-DL/entail/validation/bench changes, translated-book/README/glossary/license
prose and book-zh URL target. Owned hash/Ed/GTS/RDF source, root manifest,
target/profile/toolchain and CI inputs have zero candidate-versus-head scoped
delta (raw/G3-candidate-owned-closure-delta.patch is ZERO bytes). Lock adds only
existing entail alloc-probe dev edge; no new external version/feature or normal
edge. Combined CI/Docs execute these actual combined inputs. Clean merging or
old green branch inputs alone was not used to close integration.

Fresh >150-second full feedback capture G3-feedback-* was read, including native
review/inline bodies and terminal outer/nested thread pagination. Actual issue
body and all15 comments grant no cut; immutable planning footer and old failed
claims remain openly historical. All3 current-mapped threads are resolved.
CodeRabbit cap text is the sole external PR comment and supplies no completed
review. Native empty COMMENTED review is historical529, not human approval.
Final exact241 Rust/alert comparison closes the substantive security receipt.
The subsequent complete S3-feedback-{all-issue-comments,all-pr-comments,reviews,
inline}.json captures independently compare byte-identical to those G3 inputs
(four cmp exits0). Canonical old-array first object versus new single-object
GraphQL threads compare identically (cmp exit0); actual outer/nested pagination
remains terminal and all3 current-head threads resolved. The preserved initial
missing-variable GraphQL fetch failure is not the successful capture. There is
no new external substantive feedback in that terminal refresh. Any still-later
material concern returns to remediation rather than being waived by this verdict.

Own required mechanical scan uses verified base090...HEAD241 added lines and
the complete doctrine expression; raw/S2-exit-completion-required-deferral-scan.log
SHA25627874f2a2142a5c379e4c76095d0b2bd54a26df12ddedbbdf3b6c0a38e008afa
has one witness: distinct HashML-DSA is not implemented by pure-message APIs.
It is an evidenced false positive; this requested composite representative and
fixed-context pure ML-DSA are implemented and externally tested. Broader source
scan additionally finds future/later fixture strings and authored-history prose,
not requirements parked for later. Plan/remediation/current PR/confidence and
all branch commit scan logs are empty. Broader laundering witnesses are denied
fallbacks, existing optional read materialization, hard test preconditions,
guarded caught-panic ownership tests and malformed-before-lookup assertions.
No TODO/stub/semantic feature/optional edge/ignore/xfail/golden weakening or
silent fallback clears requested work. Own raw/S2-exit-completion-{deferral-source,
deferral-prose,deferral-commits,laundering}.log retain full witnesses.

Current IANA capture leaves-58 in unassigned-256..-54, while-49 is standalone
ML-DSA-65. Upstream identity0d1c8299c9411ea4ead853e31721d42ea66f081e,
treebbac332e919339ab0b1a9b99dcba841f0b434464, has only its two EdDSA COSE
fixtures, truncated=false. This is conditional draft support honestly pinned
to JOSE04/LAMPS19, not an invented private algorithm or final allocation claim.
Stage3 must refresh allocation/corpus facts and current base/head/candidate;
published composite fixtures, an allocation collision or material source/base
advance would invalidate the corresponding applicability decision.

No complete-operation hardware/JIT timing, physical erasure, external SHA512
state destruction, caller-copy cleansing or formal FIPS certification is
claimed or required by the literal ask. Those limits do not excuse controlled
owned state: the latter is now implemented and demonstrated. Original failed
reviews/probes and losslessly retained baseline evidence remain present; no
deficiency entry, user cut, new ticket or softened prose was used for closure.

Required Stage2 findings remaining: NONE on the captured identities. Parent
still completes final validation/integration accounting and issue/PR summary
publication, then Stage3 performs its independent current landing audit and
authorized ghprsq/audit-ref/cleanup workflow. This report performs none of those
future workflow actions and does not silently adopt them as completed.
