<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent final G3 performance review

VERDICT: PASS. Required performance finding G3-P1 is resolved on the frozen
source below. No further required performance finding was established. This
is local G3 performance qualification, not security certification, signed
transport, hosted acceptance, current-base integration or whole Stage 2 completion.
The initial BLOCKED report remains intact at tasks/G3-performance-review-initial.md,
SHA256 ae6e441e283205807213a84e193d0cdcc24936feed8af10faaa502f0b178807c.

## Reviewed identity and independence

Issue458/PR464, Blackcat-Informatics/purrdf; branch
paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
ROOT=/home/paudley/Active/purrdf;
W=ROOT/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519;
S=W/.stage/purrdf-gts-composite-ml-dsa-65-ed25519.
Committed HEAD ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6, tree
dacde4562e8f79d857ce3aa57dbe464ddaf651db, PLUS the exact frozen15-file delta.
Observed main090b14bb8c00e2504078cb694d278ee2d614443a supplies no integration
proof. Index is unchanged; this reviewed source is not yet a new signed commit.

| Exact artifact | Independently checked SHA256 |
|---|---|
| tasks/G3-implementation.md, read in full | de0be46cc01c9db33063c410984e60d6ccf6616c9b99cce4ddbef17e1289d596 |
| raw/G3-frozen-source.sha256, all15 current paths verified | a0541868761c184a776e4ffe65c40cf80df4a6253c9ef5a8081697fd0d65b50d |
| raw/G3-frozen-delta.patch, includes new secret.rs | c7778d471554c3a4544f74ae83fd62b34042ae1510f040432831b6f4056e5d55 |
| raw/G3-frozen-branch.sha256, all56 paths verified | 14fb06bbc20acfd865738f4402cfc1030b6815232603f6f49ae11e1635fb2696 |
| raw/G3-frozen-branch.patch | 663c3564974ef716448c56b6d8afe7b257ded1995b19141260f505e9f10b74e5 |

Immutable plan dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a
is preserved. Read actual AGENTS/root baseline/goals, Stage2/Stagectl quality/
validation, complete security report, remediation plan and addendum, coherent
current delta and final reports/commands/runtime/profile/artifact receipts.
This reviewer implemented no source. Independent work included source/lifetime
inspection, actual optimized-code inspection, SIX actual interleaved executions
of retained binaries, source checks before/after and final hash readbacks.
Other suites are explicitly attributable implementer executions independently
adjudicated, not relabeled this reviewer's execution. No source/index/forge,
private-memory/model/GPU/service action, children or duplicate expensive suite.
Only this review and its named selected Stage evidence are written. Gap analysis
is unchanged by this assignment.

## Resolution of G3-P1 and static cost judgment

The initial version constructed/destroyed three guarded scratch arrays in each
of24 Keccak rounds:72 wipe/observation/fence calls per permutation. Actual paired
Keccak/SHA3 measurements established meaningful avoidable work. That failure
and its source/measurements are preserved rather than renamed a passing phase.

Final public keccak_f1600 creates ONE allocation-free SecretArray<u64,35>, then
uses the same original round body and compile-time rho/pi lanes for all rounds.
The private keccak_permute splits the active workspace into c5/d5/b25. All five
c slots are assigned before any c read, all five d slots before any d read,
and all25 b slots before chi reads b. Previous round contents do not influence
the next calculation. Full scratch is explicitly cleared after the last round;
its owning guard clears on drop/unwind. No scratch escapes permutation return.
Security separately confirmed this active-workspace lifetime is compatible with
the required controlled-state cleanup. No guard/wipe/fence requirement was waived.

Production SecretArray owns fixed storage and has no allocator or observer field;
the observer Arc is test-only. Shared BlockBuffer also remains inline fixed storage.
Root stays zero-runtime-dependency, one wipe body and one Keccak implementation.
Sampler/output-lane cleanup is actual mandatory owned-state work, not an entropy
fallback or a new per-coefficient map. Current ownership tests inspect live
cleared storage; public fixture/caller behavior remains real output/verification.

Actual optimized native assembly disproves the suspected noninlined private
helper: only public keccak_f1600 is emitted, and its entire body contains no
call instruction. Head stack reservation is0x198 versus baseline0x58; actual
zero stores are retained at cleanup. The explicit clear and subsequent Drop
repeat final workspace writes, but no separately meaningful avoidable-cost
finding was established by this inspection/measurements. No dirty-flag framework,
speculative rewrite, arbitrary percentage gate or unlimited tuning is justified.
This verdict does not assert globally optimal assembly or zero cleanup cost.

## Actual independent paired executions and measured cost

Same Stage Rust probe raw/G3-throughput.rs SHA256
a4649ff6b1be6f81b13de339b8e5e363a3976bc39dc4db4c4adecca4391fcdfc.
It uses three warmups and nine timed samples per workload, black_box inputs and
outputs, fixed equal seed/message, plus real public sign/verify. It measures
SHA3_25664KiB, SHAKE256128B-in/640B-out, direct Keccak, MD5/SHA1 controls and
MLDSA keygen/sign/verify. Signature identity is identical in EVERY run:
d221e89921473da22be53c3707a081bb770265866dbe2ba64b25d36e27c2aa0c.

Executed existing binaries directly, with no rebuild, sequential order
base/head/head/base/base/head. Combined process terminated0. Before/after
held14-path source readbacks both matched exactly. These are independent actual
executions, not an inference from the implementer's provisional timing report.

| Workload | Three baseline run medians | Three guarded-workspace run medians |
|---|---|---|
| Direct Keccak, ns | 196.6 / 215.1 / 217.0 | 215.2 / 251.7 / 251.6 |
| SHA3_25664KiB, us | 105.2 / 98.5 / 107.1 | 104.4 / 112.3 / 114.0 |
| SHAKE256128in/640out, us | 1.084 / 1.221 / 1.184 | 1.236 / 1.507 / 1.204 |
| MLDSA keygen, us | 136.2 / 141.7 / 143.5 | 136.1 / 143.5 / 142.9 |
| MLDSA sign, us | 316.6 / 311.2 / 344.0 | 310.4 / 338.4 / 322.7 |
| MLDSA verify, us | 143.1 / 151.8 / 146.1 | 134.9 / 136.4 / 150.6 |

MD5/SHA1 controls also vary with run phase. Earlier sequential logs had much
larger absolute Keccak/SHAKE times; the interleaved actual measurements show
those differences cannot support a blanket50–80% regression claim. There is
some measured cleanup overhead and host variation. Neither a constant timing
ratio, cross-machine throughput equivalence, hardware side-channel guarantee
nor a percentage acceptance threshold follows. The concrete72-round-cleanup
finding is closed by the coherent workspace repair and qualified output; no
further required cost defect is demonstrated in the remaining guarded design.

Profile: rustc nightly4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM23.1.1,
native x86_64; normal dev opt3/debug1/assertions/overflowON, package debugfalse,
buildoverrideopt3. Both standalone manifests have those exact settings; logs
print assertions=true. Existing binaries, independently hashed before and after:
head95d25dbef07fb616a98d4ca969ea963a23aee48862e243f0da30a2c2e6cd9278;
base332cc332bcfde4e76d85d106ba38ffeffb364d0d1dae4c5ecd526382fa87e4eb.
They are the binaries named in the final throughput logs, not the shipping
release profile. No optimized observer seam is used in these production crates.

## G1 allocation retention and baseline reproducibility

Actual G3 reader replay is attributable terminal0, raw/G3-reader-performance.log,
SHA256 ab0d10b5bd743dbaf967b678a12e1026e5bc16865d30e423c33ba76add4146a4.
Read every measured row; its unchanged common Rust probe, current normal-profile
manifest/lock and all three corpus identities independently match. The reader
and public G1 test source hashes remain14878fe996f2fa28ced78a6a723291bd8208084eadcbfa06ec2daf58fe9be7a1
and1a5280a077cbf45c0e1fca9e789d9755137abc1dc240b0eec8e81d04329841c7.

| Corpus/caller | Allocations / requested bytes after G3 |
|---|---:|
| Ordinary unique eager | 700313 / 115249595 |
| Ordinary unique evented | 850393 / 116200709 |
| Ordinary repeated eager | 101468 / 40977643 |
| Ordinary repeated evented | 101848 / 35751957 |
| Genuine pack eager | 700732 / 117515312 |
| Genuine pack evented | 850928 / 118471657 |

All six cells equal accepted G1 results. Ordinary50000/genuine50004 rows,
no diagnostics and retained0; genuine classification remains positive.
No renewed subject-cardinality allocation or extra guard heap cost was observed.
These are allocation traffic/working-demand metrics, not RSS or a timing pass.

Independent hash readback verified authoritative compressed archive
fb5469ca2bc07dee7e01c426e48e590fa50e7b541e1edc4ab6ea852ce054620d and preserved
original tar16fd9702b39196081d9b83527d34c04a8a917d4eda8995f0c9915bb6d2c76b02.
Read complete packaging receipt: lossless decompression and16,045 file/symlink
payload comparisons are attributable implementer verification. Original source
read paths remain symlinks into governed build scratch; authoritative archive
stays in Stage. Normal working-tree ratchet passed without an ignore/exemption.
This preserves baseline measurement reproducibility while keeping extracted
historical non-Rust code outside source inventory. No baseline body was edited.

## Terminal qualification and precise reuse

Read full implementation report, command/status phases and actual named logs;
all21 log hashes independently passed readback. The obsolete needs_drop=false
security assertion exits101 as explicitly expected changed observation; it is
not counted as passing destruction acceptance. Legacy libtest wasm selection
failures remain preserved and disqualified; accepted portable runtime logs are
G3-gts-wasm-portable and G3-rdf-wasm-portable, not the failed *-wasm-final names.

Accepted actual runtime closure: root56 owning tests; native GTS157 over5 groups,
RDF29 over2 groups, primitive13 over3 groups; actual Node/wasm primitive13,
GTS30 and RDF12, plus name-corrected hash10. Includes complete SHAKE answers,
four complete SHA3 corpora, MD5/SHA1 split/reset/native paths, five RFC8032
vectors, all70 official MLDSA records, complete IETF answer and both-component
refusals, Writer provider failure/recovery, real mixed-source pack/certify/
repack/refold, exact author COSE, mandatory packaging and timestamp/capability
controls. Current final root/wasm digest artifact hashes independently match
raw/G3-final-runtime-artifacts.json (71797f7e4ca60eccd222a98bedd87bef2f31b846688e8f6f8f6e9cd0969e5527).

Workspace clippy then affected post-rename clippy deny warnings; strict affected
docs and final composite documentation pass. Current helpers/layers/root ring
fence and974 optimized/assertion/overflow profile units pass. Generated metadata,
normal affected-four-library release wasm and policy/ratchet/feature checks are
terminal0. These are attributable checks, not extra reviewer execution.

Measured/functional source phase was held14paths a29a07fd1bc729b2134cf2db6ea0d5d4ac45def9887dd9bdb260128e51dd6780.
Final differences are precisely the private helper rename and composite module
documentation. Independently inverse-substituting keccak_permute→permute in
the COMPLETE current source yields original held source hash698721d0e725d5678229fd7641e294634363b44a4a12d468d34010f2bdf73d87.
Removing //! documentation lines from old/current composite gives identical
complete remaining source hashes. Thus measured runtime computation/lifetimes,
APIs/assertions/profile/caller closure are unchanged. Final root/wasm ownership,
warning/docs/hygiene checks qualify the private spelling; reuse of existing
throughput/consumer/release receipts is justified by that exact delta.

Provisional broad1583/109-group package execution is not a final-source full
rerun; only its unchanged closure is reused. G1's CI and Stage1's broad make
check are not G3 qualification. No whole-workspace runtime or all-release-crate
wasm rerun is invented or performed by this reviewer. No new hosted/head/base
or predicted-merge-tree acceptance is claimed.

## Durable reviewer receipts and remaining boundary

| Own evidence | SHA256 |
|---|---|
| raw/G3-performance-review-interleaved.log | c4af2387d8543279f8ea9a8b85fc63e93e2fd59f5f589e69490233b260c829d5 |
| raw/G3-performance-review-interleaved-summary.json | 1f9638e89b0df6c5804673d08baa93964db86ae0c512f082979298869c1c2281 |
| raw/G3-performance-review-keccak-head.asm | 22f2d0e1ccc832b44b6c15fb1b2cf5996f223a5d7947a91d3c68f343ea46b6e1 |
| raw/G3-performance-review-keccak-base.asm | 87caa4a33966185f08be01dbe3e79a1cc5c59ceb049078d8dce248f2114dc651 |
| raw/G3-performance-review-final-source-check.log | 75ee40acf067441f718e7445e0713ce594a6044fcc1e012703a66701c82bb7c6 |
| raw/G3-performance-review-final-branch-check.log | 45ad9330973b1acf1349b74c975de2ab880740d413ff345e08b649949c653bb0 |
| raw/G3-performance-review-command-logs-check.log | 1ebbaebbd526d36447977d5d365c94d6704279e0ae1b53488bed65713eac2561 |
| raw/G3-performance-review-final-inputs-check.log | 63f6b2a0d9c82c56627ec17f1557f4ef48158b6332216b09992861b292446965 |
| raw/G3-performance-review-final-runtime-check.log | d1e657ceb563e09b99e164c35d7ae170fef1f28fdfba42df0f4e8642a293ab37 |

One reviewer hash-check invocation used the wrong cwd for Stage-relative log
paths and failed to read21 files; raw/G3-performance-review-command-logs-wrong-cwd.log
preserves that miss. Corrected invocation from S verifies all21 hashes; none
of that initial miss is qualification. The old initial corpus manifest's two
preliminary profile hashes also correctly fail against later profile refinement;
the complete accepted final profile manifest and current bound inputs pass.

Required performance findings remaining: NONE. G3-P1 is FIXED; inlining concern
is an evidenced false positive; G1 allocation closure is retained. Required
security judgment remains the separate security reviewer; parent must still
perform normal signed transport/publication, current hosted/feedback and
integration assessment and independent Stage2 exit/Stage3 gates. This PASS
waives none of those obligations or mandatory controlled-state cleanup.
