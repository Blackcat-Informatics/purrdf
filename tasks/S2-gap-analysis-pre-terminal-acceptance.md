<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent Stage 2 gap analysis — signed G3 checkpoint

VERDICT: FINDINGS-OPEN

G1 and G3 are FIXED. G3-P1 is FIXED. G2's three historical CodeQL witnesses are evidenced false positives with actual accepted forge dispositions. No remaining required source finding is established by the completed specialist reviews. W1 current-base integration qualification and W2 fresh hosted/full feedback/publication verification/distinct exit audit remain OPEN. This report does not grant Stage 2 completion or merge readiness.

## Current identity and preserved history

Issue 458 / PR 464, Blackcat-Informatics/purrdf. Worktree `/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519`; selected Stage is its `.stage/purrdf-gts-composite-ml-dsa-65-ed25519`. Branch `paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519`.

Independently observed current HEAD `241061e7cc08d76de97f8a597074ec5c5fcb2ec7`, tree `02cde50a6ed4c0c79ad97183a24267e82889fff3`, parent `ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6`. Git signature status G, fingerprint `AF5E0032F7494CEBCAA7BBBE9B87CFBBCFDBAF11`. Working source and index are clean; only selected Stage is untracked. Local origin/main is `090b14bb8c00e2504078cb694d278ee2d614443a`. Captured remote branch and after-transport PR metadata agree on current HEAD; PR464 is OPEN, non-draft, targets main, and uses the recorded issue branch.

Original implementation base remains `ce3c07192aba1e36666062c00f958670a827cfb5`. The after-transport PR metadata still records baseRefOid `0d6575a46088e9420d73654b7b5965b9580c5d36`; that capture is not silently substituted for the newer origin/main. Parent's current-base predicted merge tree is `16680b5080728165b7e5da8ce139a8bcba0ff10d`, from base090b14 plus head241061, normal exit0 as parent reported. `raw/G3-predicted-test-merge-tree.txt` records the tree. Neither this clean merge calculation nor branch-local tests proves execution on that combined tree.

Both source manifests independently pass against the committed files: all15 G3 paths and all56 branch paths. Thus the reviewed frozen uncommitted source is now exactly the signed shipping source, not a later algorithm revision. Immutable plan hash is unchanged.

| Current artifact | SHA256 |
|---|---|
| plan.md | dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a |
| raw/G3-frozen-source.sha256, 15 paths | a0541868761c184a776e4ffe65c40cf80df4a6253c9ef5a8081697fd0d65b50d |
| raw/G3-frozen-branch.sha256, 56 paths | 14fb06bbc20acfd865738f4402cfc1030b6815232603f6f49ae11e1635fb2696 |
| tasks/G3-implementation.md | de0be46cc01c9db33063c410984e60d6ccf6616c9b99cce4ddbef17e1289d596 |
| tasks/G1-review.md | ab8f3a721aa6aa8fd8e7d8cd596f2b532aafa135b721a418f3da16fa775f7771 |
| tasks/G2-review.md | 562c765413300db48a6d2b5dd5d18295c780819de25828bd3728380e53170542 |
| tasks/G3-review.md, security/structure | e300b1a069b25d769c6e8c995b613bca29bc83a40dcf8c0a08a5e6c564501649 |
| tasks/G3-performance-review.md | f18eea2a6c32a80f20b338b9ac9a22cc014209ce4fb5fbe3ac929cd7379d5d50 |
| raw/G3-command-status.json, final21 records | 3aff5a2cf26f71be793cf27175d45246e5ccb7a683b2bd85ee2ada339b9fe617 |
| raw/S2-gap-current-G3-identity.json | 88c226a2e97192d3a6612ef141930424ab59c1453e73f2f4d01f2a9e126e1bba |

The complete preceding gap analysis and specialist addendum are preserved byte-for-byte as `tasks/S2-gap-analysis-pre-G3-closure.md`, SHA256 `b14ee6ed17ea0cd6482ced1d95af1d2b7e75298dd9ac5eb8c821aa1edcc69af6`. Original intake analysis remains `tasks/S2-gap-analysis-initial.md`, SHA256 `4cdfb78180fb9dc785a9e516c58f1fc1a681497fd91f31286ad35252ace9df00`. Original specialist BLOCKED reports and initial G3 security/performance BLOCKED reports remain intact. This checkpoint changes dispositions through concrete correction evidence; it does not relabel original failures as passes.

Repository/user authority, actual root/worktree AGENTS, root .baseline/.goals, Stage2/stagectl and shared quality/validation/no-deferrals/delegation instructions apply. Historical fit and the full requirement intake remain as preserved in the preceding report. Read all current G1/G2/G3 independent reviews, full G3 implementation and actual attributable evidence. No source/index/forge/private-memory/child/service/model mutation occurred. No repeated expensive suite was warranted. A lightweight memory search provided no applicable factual premise. Only selected Stage evidence/report files changed.

## Disposition of the complete material gap list

| Finding | Classification and current disposition | Evidence / remaining requirement |
|---|---|---|
| G1 / S2-P1 unused ordinary-segment classifier map | MEDIUM, FIXED | Actual segment streamable capability gates the shared reader-local observer. Original public allocation probe, meaningful native/actual wasm consumer controls and independent G1 review pass. Signed commit ad0d8e3, both publications captured. |
| G2 hosted CodeQL alert acceptance | HIGH original acceptance gap, CLOSED for the exact historical alerts through evidenced FALSE POSITIVES | Alerts235/236/237 dismissed at their exact PR-head identities, accepted readbacks and completed check112352384442 success. No source suppression or rule weakening. Fresh head241061 CodeQL acceptance belongs to W2. |
| G3 / S2-SEC-1 controlled sensitive sponge/storage destruction | HIGH, FIXED | Complete controlled ownership and last-use correction, actual live-storage tests/native/wasm production callers, independent security/structure PASS; signed current15-file transport and full56-byte match. |
| G3-P1 repeated per-round scratch guard cleanup | MEDIUM, FIXED | Single guarded35-lane workspace across24 rounds, complete overwrite before use and last-use/unwind cleanup. Independent interleaved measurements/assembly PASS. |
| W1 current-base combined-tree qualification | MEDIUM, OPEN workflow obligation | Clean candidate tree16680b… exists but executed combined-tree evidence and affected base-policy assessment are not supplied here. No actual combined-tree defect asserted. |
| W2 fresh hosted/full feedback/publication readback and distinct exit completion | MEDIUM, OPEN workflow obligation | Latest supplied initial head241061 runs queued/in-progress with null conclusions. Complete final feedback, exact publication/body readbacks and fresh independent exit audit remain required. |

### G1 — exact allocation and behavior closure retained

All three Folder constructions derive the private observer capability from their own segment header's exact streamable layout. Shared h_quads observes every validated streamable quad, including foreign predicates before reserved type/timestamp facts, while ordinary segments avoid that unused map. No second classifier, type-only prefilter, weakened provenance/UTC rule or changed materialized projection exists. Per-segment state reset prevents capability leakage across concatenation.

The independently reviewed G1 source files remain exact current bytes: reader SHA256 `14878fe996f2fa28ced78a6a723291bd8208084eadcbfa06ec2daf58fe9be7a1`, public RDF target `1a5280a077cbf45c0e1fca9e789d9755137abc1dc240b0eec8e81d04329841c7`. G3 changes neither. G3's unchanged public allocation replay preserves all six cells:

| Input / actual caller | Allocations / requested bytes |
|---|---:|
| ordinary unique / eager | 700313 / 115249595 |
| ordinary unique / evented | 850393 / 116200709 |
| ordinary repeated / eager | 101468 / 40977643 |
| ordinary repeated / evented | 101848 / 35751957 |
| genuine pack / eager | 700732 / 117515312 |
| genuine pack / evented | 850928 / 118471657 |

This restores exact baseline ordinary counts, removing2,228,396 requested bytes/15allocations for50,000unique subjects and4,380/6 for100subject control, in both readers. Genuine packaging retains its original metrics. Rows50,000/50,004, diagnostics absent, retained bytes0. These are requested allocation volumes, not RSS or a stable percentage latency claim. Original G1 independent12-group native and actual Node/wasm execution, plus current G3 real certifier controls, preserve foreign-order/reset/UTC/mandatory packaging/exact author COSE semantics. Publication receipts name issue6020684202 and PR6020684998. This closes G1 only.

### G2 — actual disposition, preserved failure and current-head boundary

Original check112352384442 failed on HEAD52988974 with three retained annotations. Independently traced counterκ=0 is mandatory FIPS204 ExpandMask initialization after private per-signature seed H(K||rnd||mu), with L=5 bounded distinct u16 counter reservations. The fixed key/IV pair occurs only in the registered signed-opaque MissingKey regression. These are not production default randomizers, secrets or reused AEAD nonces.

Read the exact G2 reviewer adjudication and accepted alert readbacks:235/236/237 are dismissed as false positives by paudley at2026-10-06T15:58:50Z/53Z/55Z. Initial overlong dismissal HTTP422 attempts remain recorded failures. The accepted bounded rationale and fresh readback close those exact alerts without changing protocol constants, fixtures, rules or suppression policy. `raw/S2-codeql-check-after-disposition.json` SHA256 `a1ed978e564fb8e60e2729adc84d9f0cfb60c90936fa15e29dd392299973da3d` identifies completed success on HEAD52988974, title No new alerts in code changed by this pull request, still3historical annotations. G2 publications: issue6020254848 / PR6020256425. Fresh head241061 CodeQL execution is not silently qualified by that historical success.

### G3 — concrete owned-state closure and required finite edges

FIPS204§3.6.3 required destruction of potentially sensitive intermediate data at last use. The defect was directly owned128-byte secret K||rnd||mu buffered beneath SHAKE256rate136, private absorber/reader lanes and operation scratch lacking clearing. Existing guarded keys/polynomials did not reach those owners. Compiler/register-history limitations never excused that owned-state omission.

Current allocation-free root SecretArray clears complete storage through the relocated unchanged `purrdf_hash::wipe_secret`; Ed25519 re-exports its public API. The root remains zero-runtime-dependency, with one wipe and one Keccak/XOF body, no new package/semantic feature/copied coefficients. Complete BlockBuffer capacity, absorber/reader lanes/clones, serialization bytes, sampler/codec/scalar/polynomial scratch are guarded at actual last use, errors and unwind. Two finite borrowed-SHA3 edges were found and fixed: live lanes/full buffer clear before output copying; finalize_reset's private digest is guarded while a short output can panic. Meaningful tests inspect zeroed storage while the actual owner is live; counters or freed-memory reads do not substitute for that evidence.

Security/structure review e300b1… closes the original HIGH finding and those finite edges. Performance review f18eea… independently qualifies actual workspace lifetimes/computation, and unchanged byte/consumer evidence. Tests and source establish this owned-storage claim, not physical memory erasure, external SHA512 state destruction, whole-machine timing safety or formal certification. The final module/PR/confidence prose names those limitations without concealing owned SHAKE storage.

### G3-P1 — measured correction with no arbitrary performance gate

Initial per-round c[5]/d[5]/b[25] guards caused72clear/fence operations per permutation. Preserved `tasks/G3-performance-review-initial.md` required a scoped correction. Current public Keccak owns one guarded35-lane workspace; each slot is fully assigned before reads in each of24 rounds, then the complete workspace clears at final use with Drop protection on unwind. Mandatory cleanup is preserved.

This reviewer actually executed six paired existing binaries in base/head/head/base/base/head order, without rebuilding, and verified source stability around them. Direct Keccak base run medians196.6/215.1/217.0ns, head215.2/251.7/251.6ns; SHA3_25664KiB base105.2/98.5/107.1us, head104.4/112.3/114.0us; SHAKE base1.084/1.221/1.184us, head1.236/1.507/1.204us. All signature identities match. Other controls and earlier sequential samples show host variation; no stable aggregate percentage is claimed. Optimized assembly has no private-helper call, so non-inlining concern is a false positive. Explicit clear plus Drop leaves duplicate final clearing visible, but no separately meaningful required defect is established. No unsupported optimality, unbounded tuning or weakening of cleanup is prescribed.

Own interleaved log SHA256 `c4af2387d8543279f8ea9a8b85fc63e93e2fd59f5f589e69490233b260c829d5`; summary `1f9638e89b0df6c5804673d08baa93964db86ae0c512f082979298869c1c2281`; head assembly `22f2d0e1ccc832b44b6c15fb1b2cf5996f223a5d7947a91d3c68f343ea46b6e1`. Full exact probe/profile/source/corpus attribution is in the performance review. G3-P1 has no remaining required finding.

## Qualified local evidence, normal transport and publication boundary

Current accepted native closure255cases includes56ownership cases and actual GTS/RDF public callers. Portable Node/wasm55groups includes primitive13, GTS30 and RDF12, plus10name-corrected hash groups separately. Corpora remain all70official NIST records, complete pinned IETF answer, unchanged SHAKE boundaries,14,097digest differentials and fiveRFC8032 answers. Writer/provider atomicity, typed keyring, both-component refusals and real pack/certify/repack exact authorship execute. Workspace all-target clippy denies warnings; strict docs, helper/layer/root/profile, generator, feature/dependency/frozen-corpus/ratchet and four affected release-wasm libraries terminate0. Full source/runtime/status attribution is in current implementation and independent reviews.

Private helper rename and module prose changes preserve exact measured computation; final owner/native/hash-wasm/warnings/docs/hygiene checks qualify the final spelling. Five-package provisional1583/109group closure is reused only where unchanged; it is not a final-source full rerun. Stage1's21045native/460docs full make check and all-release wasm belong to their captured source, not current G3 or combined-tree full execution. Preserved failed legacy wasm selections and obsolete negative security probe are excluded from acceptance. The final21-record command manifest supersedes an earlier20-record evidence assembly phase without a source delta.

Normal transport is now concrete: current commit includes exactly15G3 files, follows signed G1 commitad0d8e3, has valid G signature; `raw/G3-commit.log` records accepted index ratchet and commit; `raw/G3-push.log` records ad0→241061 remote update. Configured `.githooks/pre-commit` runs staged-index ratchet, formatting and fast policy checks with hard failure propagation, and source hook is unchanged. Parent observed normal hook/commit/push exit0; this reviewer reads those receipts and independently verifies current signature, complete bytes, clean index, remote capture and PR metadata. No claim that hooks themselves ran the full test suite is made. A mistaken `.git/hooks` lookup was corrected to actual configured `.githooks`; that failed lookup is not evidence of a missing hook.

G3 publication URLs are captured in exact receipt files:

- Issue finding/fix checkpoint: https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6022070454
- PR checkpoint: https://github.com/Blackcat-Informatics/purrdf/pull/464#issuecomment-6022071221
- Refreshed confidence issue: https://github.com/Blackcat-Informatics/purrdf/issues/458#issuecomment-6022071952
- Refreshed confidence PR: https://github.com/Blackcat-Informatics/purrdf/pull/464#issuecomment-6022072738

Exact submitted checkpoint/confidence bodies and `tasks/G3-pr-body.md` were read. Parent reports the latter is now actual PR body, with complete readback being captured. This analysis verifies supplied URL/transport receipts and prepared content; it does not invent independent live exact-body readback. That final receipt comparison remains part of W2 and does not reopen a closed source finding.

## W1 and W2 — exact still-open obligations

W1: parent must assess base090b14 plus head241061 candidate16680b… using actual base delta, merged declarations/lock and affected policy inputs. Prior captured base delta34paths includes translation/glossary/license/Makefile policy and OWL-DL/entailment/validation/bench/Cargo.lock changes. Select newly affected checks, or attributable generated-test-merge CI, by that closure; unchanged branch evidence may be reused explicitly. Record actual executed tree/base/head and required outputs in validation/integration evidence. Clean merge-tree computation alone is insufficient. No forced synchronization, conflict or mandatory third blanket suite is invented. If base advances, refresh candidate and applicability before final acceptance.

W2: obtain terminal current-head CI/Docs/CodeQL results, distinguish head from generated test-merge commit and bind actual base/tree. Supplied `raw/G3-initial-head-runs.json` records Docs37506083612 in_progress, CI37506083735 queued, dynamicCodeQL37506077821 in_progress, all head241061/null conclusion. These captured states are not final pass/failure and may have advanced; parent owns refresh. Do not substitute G1/HEAD529 green results or CodeRabbit usage-cap green status for current acceptance.

Refresh complete issue/PR body/comments/reviews/inline/thread/check surfaces after review processing. Compare exact current checkpoint/confidence/PR-body readbacks with submitted content and current identity; material new feedback re-enters remediation. Dispatch/read the distinct fresh Stage2 exit completion auditor using actual public shipped entrypoint evidence and source applicability. The initial completion report is not the final exit audit. Update validation with reuse/actual checks, then publish final source/check/audit dispositions to issue and PR. Stagectl presence gates are not correctness or acceptance.

Conditional external acceptance remains governed by the original ask: captured IANA -58unassigned and GTS upstream0d1c8299… has only two EdDSA COSE vectors; no published shared composite corpus was found. Preserve governed vectors and refresh authoritative publication facts before merge. If a composite corpus has actually appeared, consume it under the accepted contract. Primary known answers do not imply shared-engine GTS composite interoperability. Stage3 integration/ghprsq authority remains separate from this report.

## Requirement, false-positive and deferral adjudication

Original complete requirement/consumer map remains in the preserved gap analysis. Current correction strengthens owned storage without altering protected -8/-58 dispatch, both mandatory halves, exact draft04/draft19 representative/context/order/sizes, explicit hedged production Writer, deterministic primary answers, strict malformed/downgrade/refusal behavior, typed key resolution, mandatory packaging or exact authorship COSE. Current native/actual wasm consumer evidence covers the altered shared root and callers; no newly missing functional path is established. Evented observation preserves signatures; supplied-key file/certificate verification authenticates them. Structural compaction is not incorrectly called authentication.

Fresh independent deferral scan against base090b14…HEAD241061 has one added witness: mldsa65 module says distinct HashML-DSA is not implemented by pure-message APIs. It is an evidenced false positive: requested composite prehash representative and pure ML-DSA fixed context are implemented and externally tested; standalone HashML-DSA is a different unrequested algorithm. Current plan/PR/checkpoint/confidence bodies and full branch commit messages have zero deferral-keyword matches. `raw/S2-gap-current-G3-identity.json` binds exact paths/hashes/results.

Read complete final broadened implementation scan `raw/G3-deferral-scan-final.json`: nine witnesses are explicit caller key/randomizer ownership, compiler/register/spill/externalSHA512 limits, standalone algorithm boundary, frozen independent fixture location and restricted-name negative-test bytes. Actual corrected guards cover controlled buffers/lanes/scratch; those ownership disclosures do not excuse G3. Original laundering witnesses were adjudicated against source in the preserved report, and G3 review inspects actual new owners/callers. No TODO/stub/silent fallback/feature/ignored-test/corpus weakening or disguised issue deferral is established. `.deficiencies` contains only notice/marker; there is no authorized scope cut.

No remaining source finding is manufactured from still-authorized workflow steps. Equally, signed local closure does not discharge W1/W2. The required next evidence is current combined-tree qualification, final hosted/full feedback/publication readback and the independent exit audit. Until those receipts are adjudicated, Stage2 remains FINDINGS-OPEN.
