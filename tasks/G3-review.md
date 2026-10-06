<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# G3 independent security and structure review

VERDICT: PASS

The required G3 owned-storage security correction is complete on the exact frozen source below. No remaining required G3 security or structure finding was identified. This verdict qualifies the reviewed uncommitted source and local G3 evidence; it does not claim new signed transport, current hosted CI, Stage 2 completion, or integration acceptance. Those remain the parent's authorized workflow.

## Identity, authority and review provenance

Issue 458, PR 464, branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`. W is `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`; S is W's `.stage/purrdf-gts-composite-ml-dsa-65-ed25519`.

Immediately before writing this report, read-only git readback still showed HEAD `ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6`, tree `dacde4562e8f79d857ce3aa57dbe464ddaf651db`, origin/main `090b14bb8c00e2504078cb694d278ee2d614443a`. Current reviewed source is that HEAD plus exactly 15 G3 files, including the one new `crates/hash/src/secret.rs`; the index remains unchanged. The full branch manifest has 56 paths against original captured base `ce3c07192aba1e36666062c00f958670a827cfb5`.

| Artifact | SHA256 independently read back |
|---|---|
| raw/G3-frozen-source.sha256 | a0541868761c184a776e4ffe65c40cf80df4a6253c9ef5a8081697fd0d65b50d |
| raw/G3-frozen-delta.patch | c7778d471554c3a4544f74ae83fd62b34042ae1510f040432831b6f4056e5d55 |
| raw/G3-frozen-branch.sha256 | 14fb06bbc20acfd865738f4402cfc1030b6815232603f6f49ae11e1635fb2696 |
| raw/G3-frozen-branch.patch | 663c3564974ef716448c56b6d8afe7b257ded1995b19141260f505e9f10b74e5 |
| immutable plan.md | dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a |
| tasks/G3-implementation.md | de0be46cc01c9db33063c410984e60d6ccf6616c9b99cce4ddbef17e1289d596 |
| raw/G3-command-status.json | 3aff5a2cf26f71be793cf27175d45246e5ccb7a683b2bd85ee2ada339b9fe617 |
| raw/G3-runtime-accounting.json | 215395cab1dd6f6baa452a3de7ba473b5242822791bcd29f08a2e01b9b63fe74 |
| raw/G3-final-runtime-artifacts.json | 71797f7e4ca60eccd222a98bedd87bef2f31b846688e8f6f8f6e9cd0969e5527 |
| raw/G3-terminal-artifacts.json | 79534ed67efce691ac53b9527df96d26850ad87afe59b3a583bb62db6849562c |

Both complete source manifests were independently checked against current files with `sha256sum -c`: all 15 and 56 paths passed. All 21 log checksums in the final command-status receipt passed independent readback, including the explicitly non-passing original negative probe. All 93 regular-file checksums in the terminal artifact receipt also pass; its two additional records preserve the original baseline read-path symlinks. The three name-corrected native/wasm artifacts in the final artifact receipt independently hash correctly at their actual cache paths. An earlier 20-record command-status readback was captured while final evidence assembly was still adding the no-features record; the final 21-record receipt above reconciles exactly with the terminal inventory. No source or runtime-log delta accompanies that evidence-only completion.

I read actual governing AGENTS, root .baseline/.goals, Stage 2/stagectl instructions and shared quality/validation/no-deferrals doctrine; the full original issue and nine comments, immutable plan, prior-art/intake/history and task corrections; original S2 security and structure reports, remediation addendum and initial G3 review; the complete affected source and production caller closure; the entire final implementation report, terminal command/status/accounting receipts and qualifying logs. This review was independent of implementation. No source, index, forge, private store, child-agent or service mutation was performed. No additional blanket test suite was run: current meaningful receipts were reusable, and the two specific missing cleanup checks were requested and executed by the sole implementer.

Historical fit remains as recorded in historical-fit.md: a first-party primitive through established homes, shared zero-dependency hash root, explicit caller entropy and mandatory packaging fit repository doctrine and actual consumers. Composite recurrence is UNKNOWN; five "none" trailers do not establish five cryptographic failures. Original blocked reports remain historical evidence and are not relabeled.

## Required finding and finite last-use corrections

Original S2-SEC-1 was a concrete controlled-storage defect. The private 128-byte K || rnd || mu preimage fits beneath SHAKE256's 136-byte rate, leaving K in the owned block buffer, with private absorber/reader lanes and sampler scratch lacking cleanup. FIPS 204 section 3.6.3 requires destroying potentially sensitive intermediate data as soon as it is no longer needed. The preserved primary FIPS204.pdf SHA256 is `57239b9f84c03227eda3ca0991204dc7764c79af9ce2e6824eda774918d46b6b`. A disclosure about compiler copies could not clear this owned-storage finding.

The frozen implementation now clears the controlled storage inventory below, with actual ownership and last-use wiring rather than stored-key Drop alone. S2-SEC-1 is CLOSED.

This review also identified two finite omitted edges during remediation, both corrected before freeze:

1. SHA3 finish initially retained finalized lanes across the caller-output copy. A short output panic caught while borrowing the still-live hash could retain them. Current `sha3.rs:275` clears the lanes after assembling the output and before returning it. The actual `Digest::finalize_reset` short-output path is caught in `borrowed_finalize_panic_leaves_no_live_owned_sponge_storage`; the still-live lanes and full buffer are asserted zero, followed by successful empty-message and abc reset semantics.
2. The private local digest in trait finalize_reset is controlled storage until the caller copy succeeds. Current `sha3.rs:460` guards it with SecretArray and a test-only observer on that actual owner. A short-output panic clears the complete private digest on unwind; success drops it immediately after copying. Public returned digest arrays remain caller-owned outputs with unchanged signatures. This does not impose an unbounded compiler/register-history requirement.

The original G3-P1 repeated-round scratch cost is also corrected without weakening cleanup: one guarded 35-lane workspace is reused across the active permutation, instead of 72 per-round clear/fence operations. Each of c[5], d[5], b[25] is completely assigned before each round's reads; b destinations 0 through 24 are explicit. `keccak_permute` clears the entire workspace at the complete permutation's last use, and owner Drop protects unwind. That is a coherent active scratch lifetime under section 3.6.3. The live private seam asserts all 35 slots zero after permutation and reuse; synthetic unwind exercises the same guarded owner's Drop. Test observation does not dereference freed storage.

## Controlled-storage inventory and boundaries

| Controlled owner or scratch | Actual cleanup and evidence |
|---|---|
| SecretArray fixed arrays | One shared fill-default/black_box/compiler-fence body; explicit clear and Drop; clone creates its own guard; Debug redacts contents. Observers run after clearing while actual owner and array are live. Byte, lane and polynomial arrays exercise return, error, clone and unwind. |
| Shared BlockBuffer capacity | Guarded complete capacity, cleared after buffered block consumption, final padding, reset and Drop. Partial unused tails are covered, not just logical length. MD5/SHA1 retain the same bytes through this common buffer. |
| SHA3/SHAKE absorber lanes and buffers | Owned guarded lanes, complete buffers; clone is guarded. SHAKE transfers to a guarded reader then clears the old absorber and buffer immediately. SHA3 finish and reset clear live state; both borrowed panic edges above execute. |
| SHAKE reader and squeeze lane bytes | Reader clone/Drop guarded; eight-byte serialization guard clears the entire temporary, including unused tail, at the actual caller copy's last use. Long squeeze and boundary differential execution remains exact. |
| Keccak c/d/b workspace | One guarded 35-lane owner, borrowed partitions, complete overwrite before reads, complete last-use clear and unwind cleanup; one Keccak computation body and unchanged constants. Public caller state remains caller-owned. |
| ML-DSA byte and polynomial owners | Existing SecretBytes/SecretPolys use the relocated single wipe home and owning-module live checks. Key expansion, private signing seed, masks, transformed polynomials, w/low/residual/products/high scratch and error/rejection exits are guarded; explicit drops shorten actual last use. |
| ML-DSA scalar/sampler/codec scratch | Guarded representative, high polynomial, uniform three-byte candidates, bounded byte/nibbles, challenge sign bytes/scalar/position byte and pack/unpack accumulator. Budget-exhausted partial polynomial outputs are cleared. Hint decode constructs a guarded polynomial owner before possible malformed-input errors. |

The generic array's Default-valued cleanup is the original API behavior; all sensitive instantiations here are numeric/byte zero defaults. No unsafe code, new external runtime, allocation in the production fixed owner, second wipe implementation, semantic feature or changed protocol constant was introduced. Test observers alone use Arc.

The owned SHAKE/sampling paths now satisfy the reviewed source-level storage requirement. Claims remain bounded: compiler-created historical copies, registers/spills, physical erasure and external sha2 SHA-512 state are not guaranteed. FIPS functional conformance and source-level arithmetic review are not formal certification or a whole-machine side-channel proof. These boundaries no longer conceal unguarded owned ML-DSA SHAKE storage. Final composite module prose accurately names the cleared owned scope.

## Structure, public API and actual consumers

The wipe body moved unchanged from Ed25519 to `purrdf_hash::wipe_secret`, the existing zero-runtime-dependency root. Ed25519 re-exports its prior public API and ct callers continue through that one home; GTS's existing callers therefore share it. SecretArray lives at that root and callers import it, rather than duplicating a guard. Applicable helper ledger and AGENTS home descriptions were updated. Layers remain unchanged: no new edge is needed because callers already depend on the root.

One original FIPS 202 Keccak body still serves SHA3 and SHAKE. The private helper name repair removes a real census fingerprint collision without exemption, detector alteration or computation change. I compared the held and name-corrected patches: after the exact definition/three-call spelling replacement, only the git index blob hash differs. The final fifteenth file is composite module prose only; removing module-doc lines yields identical executable source. This supports explicit reuse of optimized runtime evidence, supplemented by name-corrected owner/native, actual hash wasm, clippy, docs and hygiene, then frozen-prose GTS strict rustdoc. It does not rename old execution phases as frozen-source runs.

Ed's existing RFC test now uses the first-party portable harness, retaining all five RFC vectors and exact public keys/signatures/tampered-message assertions, plus the shared wipe check. Cargo adds only its harness=false test target and uses the existing dev dependency. Public SHA3/SHAKE/Ed APIs and primitive bytes are preserved.

Inspection traced actual ML-DSA and composite signing through the Writer provider and fresh fallible caller Randomizer, strict Sign1/key decoder and both-component verifier, then mandatory pack/repack and RDF certifier. No test-only signing path substitutes for production. Strict key encodings, fixed pure-message composite context/domain, typed missing-key outcomes, both-component protection, nonce bounds and rejection semantics remain intact. Entropy is acquired before append mutation; actual failure/partial entropy/retry tests preserve atomicity. Deterministic fixture APIs remain explicit; production hedging has no silent zero-randomizer fallback.

## Accepted execution, checks and provenance

The environment receipt records nightly rustc 1.100.0-nightly commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM23.1.1, native x86_64, Node26.10.0 and wasm-bindgen0.2.125. Dev/test builds retain opt-level3 with debug assertions/overflow checks ON; actual wasm runs use the repository runner and Node. All accepted executions below terminated normally with exit0 and zero ignored/filtered cases.

| Runtime receipt under raw | Executed coverage | SHA256 |
|---|---|---|
| G3-owner-native-name-corrected.log | 56 root tests, including live storage, clones, caught-panic digest and permutation scratch | cfc1806c0b22d04d777e3b2ebebf1e2723126eae6df62eff1a9434e4c77c68c4 |
| G3-primitive-native-final.log | 13 functions: RFC8032, eight digest differential functions, two SHAKE functions | 15241730459af10c5df96d5adc963d1d3f79c96cc8bd8bb76b161be9587da2b2 |
| G3-gts-native-final.log | 157 cases: 121 unit, six legacy compaction, 30 portable/public crypto and Writer | c67874b57e4a5a7c38043072dfffa22e68859cb47964e03d52b4922819d4742f |
| G3-rdf-native-final.log | 29 cases: 17 certifier, 12 composite compaction | d961a9a30e838ba9167c4ea614b5af81fe481d4f6cc257032a5ffaeca105450f |
| G3-primitive-wasm-final.log | Actual Node13 primitive functions | b6dca13628c0e88d10e95eb4b868b2162f58805c4e377eb6e7cc76cff0c55ef3 |
| G3-gts-wasm-portable.log | Actual Node30 ML-DSA/COSE/Writer functions | 3625778b3175bf9a23aa817216e5f62331008c8c52dcdeac3350e9a12efb4379 |
| G3-rdf-wasm-portable.log | Actual Node12 public certifier/pack/repack functions | fff50d42bc6b0b0b3d1c94f2c242a1059fd236f6e28134800f5ec393714d8865 |
| G3-hash-wasm-name-corrected.log | Actual Node10 hash functions after helper name repair | cbaa0f6aadb829f043b8f08937fc3505ad3042fe661cb76c2b00a92f1353927e |

The updated native closure totals 255 cases; actual portable wasm totals 55, with ten repeated name-corrected hash functions separately recorded. Function counts do not hide the actual corpus: digest differential uses all 14,097 records, SHAKE all four NIST512-byte records plus16 independent boundaries, and ML-DSA all70 official records (25 keygen,15 deterministic sign,15 hedged sign,15 verify). COSE/Writer fixtures preserve complete IETF and frozen Ed bytes and adversarial both-component/strict-header/key/entropy coverage.

The RDF portable target really calls public `gts_certify::compact_and_certify` and `verify_compaction`: adapters at lines89–111, success/certifier assertions146–188, entropy atomicity426 onward and timestamp controls726 onward, all12 functions registered at footer954. It executes Ed/composite packaging, mixed authorship, pack-to-repack, keyed verification/refold/order and rejection. This is runtime proof of the required consumer seam, not compile-only evidence.

The full five-package provisional closure is preserved and fully read: 1,583 tests/docs,109 groups, exit0. It precedes scratch optimization and is explicitly provisional; unchanged Ed reference/strictness and other unaffected cases can be reused, while altered primitive/GTS/RDF paths have the current evidence above. It is not claimed as a fresh final full make-check or hosted matrix.

Terminal checks inspected: full workspace all-target clippy with -Dwarnings, affected name-corrected clippy, strict affected/hash-name/final-prose rustdoc, helpers/layers/root/profile hygiene, generated projection verification, affected four-library release wasm build, dependency/corpus/CI-sharding/toolchain/law/feature/fmt/diff checks and frozen working-tree non-Rust ratchet. The receipt binds their exact commands/phases/logs. Census resolves81 jobs/23 sanctioned variants/91 distinct, zero copies; 207 normal dependency edges/42 members; root zero dependencies;974 compiled profile units retain required checks. No gate exemption, dependency/feature/profile weakening or corpus/golden edit occurs.

Legacy wasm compaction_signatures and gts_certify runner failures are preserved and excluded from acceptance. The original obsolete negative security probe exits101 after printing absorber/reader needs_drop=true; that expected changed observation is not counted PASS and does not replace live-storage tests. Earlier RFC legacy harness failure and compile/lint corrections remain recorded.

The root name-corrected native binary SHA256 `344c988e4d1f4b61cb6851b192d5ae941d79c41c4a0a20faf4a514855956918c`, digest wasm `811b1ecbbe7ac655c96f5d8c043a6793148f35374c9914999d83342b7ea25d59` and SHAKE wasm `cc7edf68e844e0218768e2d1755d4b0d4cf985f2bb289a631a6c17a3003dcbfe` independently match actual final receipt paths. These are post-qualification cache readbacks, not invented original execution captures.

## Preservation, performance and remaining workflow

The complete historical performance baseline is preserved losslessly in authoritative S: compressed archive51,439,037 bytes, SHA256 `fb5469ca2bc07dee7e01c426e48e590fa50e7b541e1edc4ab6ea852ce054620d`. Independent gzip decompression hashes to original239,902,720-byte tar `16fd9702b39196081d9b83527d34c04a8a917d4eda8995f0c9915bb6d2c76b02`. The16,045-entry file/symlink manifest and original read-path symlinks preserve extraction/probe inputs in governed ignored build scratch; no source ratchet weakening or data omission was used. The normal frozen ratchet passes. Compression receipt SHA256 is `96d4101ab2e6ea8977082556a7bdd387b790a2e4f4734bd84d821608a0ea6512`.

The independent performance specialist's six interleaved measurements distinguish host variation from the former72-clear defect. Its scoped judgment is separate; no fabricated independent benchmark execution is claimed here. Guarded workspace reuse preserves normative cleanup. Existing reader allocation replay passes unchanged six cells and zero retention; signing identity remains exact. No additional security-related optimization waiver is granted.

The three prior CodeQL findings remain independently evidenced false positives: FIPS private-seed ExpandMask counter, and two fixed regression-only Encrypt0 constants. G2 exact historical dismissals/check success are preserved in tasks/G2-review.md. They are not authority to disable rules or an assertion about fresh uncommitted G3 hosted results. CodeRabbit usage cap is not substantive acceptance. Parent must bind signed/pushed G3 bytes and obtain fresh current-head public/CI/integration readbacks through the normal authorized Stage workflow.

Required G3 findings: NONE REMAINING. Original S2-SEC-1 and the two finite SHA3 last-use omissions are closed by source changes and meaningful executed checks. Source remains frozen for parent transport.
