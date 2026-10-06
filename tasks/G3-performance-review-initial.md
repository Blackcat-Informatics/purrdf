<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Initial G3 performance finding

VERDICT: BLOCKED. Required MEDIUM original-home permutation optimization,
G3-P1, remains before final performance qualification. This records provisional
source/measurement observations; all-source qualification is still running and
no final G3 verdict is supplied. Mandatory controlled-secret cleanup remains
required, not waived to obtain faster output.

Issue458/PR464, branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519;
W=/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519;
S=W/.stage/purrdf-gts-composite-ml-dsa-65-ed25519.
HEAD ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6, tree
dacde4562e8f79d857ce3aa57dbe464ddaf651db, plus held G3 source delta.
Origin/main090b14bb8c00e2504078cb694d278ee2d614443a is not integration proof.
Read actual governing AGENTS/baseline/goals, Stage2/Stagectl quality/validation,
complete security report and remediation plan, G1 review and held G3 delta.
No source/index/forge/private-memory/model/GPU/service mutation or children.
Only this Stage report and scoped initial source manifest were written.

G3-P1: round! currently constructs c[5], d[5], b[25] guarded owners and destroys
each in every round. Keccak24 rounds therefore perform72 clearing calls and
compiler fences for scratch. These wipes cover real owned sensitive data, but
a guarded active workspace can retain the same cleanup obligation while
avoiding repeated owner initialization/destruction per algebraic round.
The source uses fixed loops and compile-time rho/pi assignments that overwrite
every slot before the next round reads it. There is one public Keccak body.

The existing Stage Rust probe executes real SHA3/SHAKE/Keccak plus MD5/SHA1
controls and MLDSA keygen/sign/verify. Nine timed samples each, three warmups,
black_box input/results; signature identity is unchanged
d221e89921473da22be53c3707a081bb770265866dbe2ba64b25d36e27c2aa0c.
Paired normal dev manifests have opt3, debug1, assertions/overflow on, package
debugfalse and buildoverrideopt3. Logs show assertions=true and terminal
program output after optimized builds. Their sole implementer execution is
adjudicated; this reviewer did not rename it as independent execution.

Observed sample ranges: direct Keccak base312.6–330.8ns/head447.7–454.9ns;
SHA3_25664KiB base153.3–166.4us/head216.4–239.2us. SHAKE also increases.
MD5/SHA1 controls move faster athead, so an indiscriminate host slowdown does
not explain the Keccak result. MLDSA samples are mixed/noisy. This is scoped
evidence of meaningful avoidable per-permutation work, not a universal stable
percentage, cross-machine throughput claim or arbitrary percentage gate.

Required repair, conveyed directly to sole implementer: one guarded c/d/b
workspace reused across the permutation, with COMPLETE overwrite before any
read in each round and complete wipe immediately after final permutation use
and on unwind. Security reviewer independently confirms this active-workspace
lifetime is compatible with FIPS204§3.6.3; it is not permission to remove the
guards, expose stale arrays or extend their lifetime beyond permutation return.
Exact architecture/tests remain subject to review. Implement only after
current required checks terminate normally, never interrupt them.

Closure: retain this initial source/log history; bind revised source/patch and
compiled probe identities; repeat the same meaningful paired workloads and
functional owner/native/actual wasm checks affected by workspace mutation;
preserve complete official/pinned answers and G1 reader-allocation closure.
No redundant blanket gate is demanded. Final performance report waits for
frozen revised source and terminal receipts. Security cleanup review is separate.

Evidence SHA256:

| Artifact | SHA256 |
|---|---|
| raw/G3-performance-initial-source.sha256, four exact relevant source paths | 6c1e671430027d48044b6815e6e1ea58e08f13cf0c42debf33cec5ae69dd77c7 |
| raw/G3-throughput.rs | a4649ff6b1be6f81b13de339b8e5e363a3976bc39dc4db4c4adecca4391fcdfc |
| raw/G3-throughput-base-corrected.log | 10ab618ecc236854f9c7cb48fa9d5a80e540abb3649d9bada0f4a420faae2665 |
| raw/G3-throughput-head-corrected.log | 887675ec9acfee1b986cdedaa7d5163d996ef9524b857a4303f5bc6aa5420c50 |

Other static performance observations: SecretArray has no production allocator
or observer field (test-only Arc seam); wipe is the original single shared fill/
black_box/compiler-fence home moved to the zero-dependency root, re-exported
through Ed25519. Shared BlockBuffer change also affects MD5/SHA1 and requires
appropriate closure; those controls now exist in this same probe. No new
dependency/semantic feature or copied implementation is needed. Per-squeeze
lane and sampler cleanup remain mandatory; no separate unsupported defect is
claimed merely because a cleanup has cost.
