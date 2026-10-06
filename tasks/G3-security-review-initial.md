<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Provisional G3 security and structure observations

VERDICT: BLOCKED pending frozen source and terminal qualification. This is an
initial review record, not the required final tasks/G3-review.md verdict.

Issue 458 / PR 464; branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Captured committed HEAD ad0d8e3ebcbe8410e91c41f5d7a80316ffa73ef6, tree
dacde4562e8f79d857ce3aa57dbe464ddaf651db, plus provisional G3 source edits
by the sole implementation agent. Current origin/main is
090b14bb8c00e2504078cb694d278ee2d614443a. Immutable Stage 1 plan SHA256 is
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
The reader G1 CI and earlier unchanged-head output answers do not qualify G3.

Read actual governing AGENTS/baseline/goals, Stage2/Stagectl quality/validation
and no-deferrals doctrine, original full security and quality/structure reports,
remediation plan, literal issue/comments/plan/initial task reports and actual
provisional cleanup source/callers. No reviewer source/index/forge/private-memory
mutation, child delegation, model/GPU/service actions, or duplicate tests.
Only this selected Stage review record is written.

The root SecretArray covers the complete directly owned array with one original
wipe body (default stores, black_box observation and compiler fence). Clones
produce independently guarded owners. Safe test-only callbacks inspect actual
live storage after cleanup before the owner is destroyed, rather than reading
freed storage. Root remains zero dependencies; Ed25519 re-exports the same API
and all existing Ed callers still route through that one body. BlockBuffer
clears full capacity after buffered block consumption and final absorption,
including SHAKE256's actual private K||rnd||mu partial-block case. SHAKE
finalization creates a guarded reader and immediately clears old lanes/buffer;
reader/drop/clone/error/unwind paths have guarded owners. Sampler bytes,
candidate nibbles, signs, codec accumulators, decoded hints and MLDSA polynomial/
byte arrays have controlled cleanup. Final source still requires inspection.

Two finite additional last-use edges were identified and conveyed to parent
and sole implementer before freezing, as required G3 closure details:

1. Sha3::finish left finalized owned lanes populated before finalize_reset's
   caller copy. A too-short output slice could panic while the borrowed Sha3
   owner remained live after catch_unwind. The provisional correction clears
   lanes after producing the digest; its actual owning-module test observes
   zero lanes/full buffer while the same hash is still live after the caught
   panic and preserves subsequent empty/abc digest behavior. Executed terminal
   final-source qualification is still pending.
2. finalize_reset's private digest array remains library-owned until the caller
   copy succeeds. A short-output panic delivers no public output, so this
   finite internal array is controlled scratch rather than caller-owned public
   return storage. Required minimal correction: guard that internal array,
   copy through its borrowed slice and destroy promptly. Public digest/finalize
   [u8; OUT] return APIs remain unchanged. Actual final source/tests are pending.

Measured repeated-round scratch cleanup cost is independently under performance
review. A single guarded c/d/b workspace active for the whole permutation is
compatible with the controlled lifetime obligation if every slot is completely
overwritten before read each round, no stale arrays escape, and complete cleanup
occurs immediately at final permutation use and on unwind. This is scoped
design adjudication, not permission to remove guards or an unreviewed PASS.

The actual Ed25519 RFC target conversion was read in full: all five vectors,
exact public-key/signature comparisons, strict/ordinary verification and
changed-message refusal assertions remain. The first-party portable harness
registers both original functions and an added shared-wipe/clone live-array
test. Its corrected actual wasm run has terminal success; the original legacy
runner failure is preserved. No new runtime edge or vector regeneration occurs.

The implementation boundary remains controlled owned storage, not physical
memory erasure, formal certification, external sha2 state destruction or
historical compiler/register/spill copies. Those limitations cannot substitute
for missing owned cleanup. Final closure requires exact source/manifest identity
and full final implementation report, substantive safe live-storage checks,
primary native/actual wasm answers and affected production consumer/gate logs.
