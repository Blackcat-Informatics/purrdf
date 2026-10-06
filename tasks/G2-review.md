<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# G2 independent CodeQL disposition closure

VERDICT: PASS

Scope: G2 ONLY. The three exact PR-head CodeQL false-positive dispositions and
resulting hosted CodeQL check acceptance are verified from complete captured
native API readbacks. No code repair, suppression or weaker test was necessary.
This does not discharge G3/S2-SEC-1, change the independent security BLOCKED
verdict, or establish whole-PR hosted/integration/merge acceptance.

## Identity and authority

Issue 458; PR 464; branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519;
worktree /home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Independent read-only Git identity remains
HEAD 52988974f2d11a40214281648983f9145af7a3fb,
tree 43b0817e30a3ad110cd013796a064c177a27e067.
All 52 current source entries independently match final manifest SHA256
e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d.
Approved plan SHA256 remains
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.

The preceding independent security assessment is reviews/S2-security.md SHA256
d377e71d54a97d57a846b050df2f082a600d57d74441d283263ee4420deea068.
Its source-based false-positive reasoning remains applicable to these exact
unchanged code locations; its separate owned-sensitive-state finding remains
REQUIRED/HIGH and OPEN.

This focused reviewer implemented no source and made no source/index/forge/
private-memory/child mutation or duplicate retrieval. Only this selected Stage
report is written. Current source/index are clean; selected Stage is untracked.
Native API mutations were performed by parent and their actual outcome, not
their planned command, is independently adjudicated here.

## Complete exact-alert and instance adjudication

Read raw/S2-codeql-alert-{235,236,237}.json and all three -instances.json,
original long dismissal payloads and HTTP 422 responses, bounded payloads/
responses, and complete fresh raw/S2-codeql-dismiss-{235,236,237}-readback.json.
The native PR-specific records bind every alert to refs/pull/464/head and the
exact source head above, with rule rust/hard-coded-cryptographic-value,
CodeQL 2.27.1 and /language:rust. The instances agree with the original three
annotations, rather than merely matching a repository-wide alert title.

| Alert | Exact code and disposition basis | Verified actual final state |
|---|---|---|
| 235 | mldsa65/mod.rs:312, internal kappa=0. FIPS 204 Algorithm 7 steps 7/8/11/31 require private per-signature seed H(K||rnd||mu), zero initial counter, ExpandMask and L increment. Actual source reserves five distinct u16 counters and refuses exhaustion before mask reuse. It is not production fixed rnd/private seed or an AEAD nonce. | Alert and most recent exact-head instance dismissed, reason false positive, by paudley at 2026-10-06T15:58:50Z. |
| 236 | hedged_writer.rs:396, public fixed key in registered single-encryption opaque MissingKey/authentication regression. Actual test signs the encrypted blob then verifies without the decrypt key, requiring valid signature, MissingKey and accepted opaque authenticity. It is not a library secret/default. | Alert and exact-head instance dismissed, reason false positive, by paudley at 2026-10-06T15:58:53Z; instance classification test. |
| 237 | hedged_writer.rs:397, public fixed IV for that one controlled regression encryption. There is no repeated production IV or production default; Encrypt0Options receives caller key and IV. | Alert and exact-head instance dismissed, reason false positive, by paudley at 2026-10-06T15:58:55Z; instance classification test. |

Actual source was re-read at both affected locations and the private-seed/counter
flow. Original complete rationale in S2-security.md and parent payloads correctly
distinguishes public fixture bytes from production entropy, and the mandatory
internal counter from private per-message randomness. The bounded accepted
comments preserve that concrete rationale and link the already published
independent remediation discussion. They do not characterize a real defect as
accepted risk or claim a changed algorithm.

All first PATCH attempts failed normally with HTTP 422 because dismissal comments
exceeded 280 characters (833, 751 and 702 supplied). Those requests do not count
as successful dispositions. Their responses remain raw/S2-codeql-dismiss-N-response.json.
Parent then used specific bounded comments; all accepted response bodies show
the intended exact alert/ref/head and reason. Independent fresh readbacks,
not PATCH response alone, establish current dismissal. No rule, workflow,
protocol constant, fixture or test source was changed to obtain this state.
No unrelated default-branch alert is included in this G2 acceptance.

Fresh readback SHA256:

- 235: a8cf7be99ec4349022a15e48ec996b8a846e6685c82cc690ad263b1b09bc14e3.
- 236: 151e494a39092bf8a2350747012a79f94c9f45fa15d0fcc1b91965d55f997f37.
- 237: cc513960db77b2b1790b85f6b0e1272fa33ae589426511f988017954d419d52d.

## Actual check acceptance and limits

Read raw/S2-codeql-check-after-disposition.json in full, SHA256
a1ed978e564fb8e60e2729adc84d9f0cfb60c90936fa15e29dd392299973da3d.
It identifies check-run 112352384442, exact head
52988974f2d11a40214281648983f9145af7a3fb, PR 464 and the correct repository.
Status is completed, conclusion success, with title
"No new alerts in code changed by this pull request".
The annotations_count remains THREE: original annotations are preserved
historical analysis evidence, not erased or misrepresented as never existing.
The earlier failure snapshot and successful extraction jobs remain separate
from this accepted alert-check readback.

G2's required outcome is therefore MET: independently supported false positives
are durably disposed at their actual PR-head alert identities and the hosted
CodeQL result acknowledges acceptance. No new test execution is needed for
the unchanged, already independently traced computation and registered native/
wasm regression evidence. This reviewer did not execute another crypto suite
or claim a new timing/secret-erasure proof.

Parent still must publish the complete rationales, actual receipt identities and
this review to issue and PR as specified. G3 owned-sensitive-state remediation
and all other Stage 2 gaps remain open under their independent findings.
This focused PASS must not be quoted as whole security approval or whole-issue
completion.

