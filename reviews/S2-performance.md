<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Stage 2 performance specialist review

VERDICT: BLOCKED

One MEDIUM required finding remains: S2-P1, avoid unconsumed provenance
collection on nonstreamable reader segments. This is measured allocation
traffic at actual public callers, not an inferred slowdown. No further
performance finding is asserted for native ML-DSA/SHAKE, strict Sign1 or
certification on available evidence.

## Identity and authority

Issue 458, PR 464; worktree
/home/paudley/Active/purrdf/.worktrees/458-purrdf-gts-composite-ml-dsa-65-ed25519.
Branch paudley/458-purrdf-gts-composite-ml-dsa-65-ed25519.
HEAD 52988974f2d11a40214281648983f9145af7a3fb,
tree 43b0817e30a3ad110cd013796a064c177a27e067.
Original implementation base ce3c07192aba1e36666062c00f958670a827cfb5.
Approved plan SHA256
dc98e6e58ed497a0d3f2117e7e60d0278062c292a2bcea16d3f9570255104a1a.
Full 52-source manifest raw/T6-R1-final-branch-files.sha256 SHA256
e899501200bd56af128b504c84c9ca1c8681f6c92fe3719815bb6fa635c6567d.

Independently verified those identities and all 52 current source entries;
raw/S2-performance-head-manifest-check.log. Source/index are clean, only selected
.stage is untracked. Read governing AGENTS performance rules, parent baseline/
goals, Stage 2/Stagectl quality/validation/no-deferrals, current issue requirements,
approved plan, T5 implementation and T6 qualification/final reviews, integration
observations and changed hot paths. No shipping/index/forge/memory mutation,
delegation, branch synchronization, main/sibling change or lifecycle action.
This is scoped performance judgment, separate from security/structure/completion.

## S2-P1: MEDIUM required unused ordinary-reader subject map

Owner: parent-assigned Stage 2 implementer.
Locations: reader.rs Folder::h_quads:1011, both Folder constructors and
ActiveStreamingSegment state; compact.rs ProvenanceSubjects::observe:548 and
packaging_role:652.

Every validated quad calls provenance.observe in eager AND evented readers.
It inserts an entry for every subject and tests three closed vocabularies,
including ordinary IRI subjects with foreign predicates. Its role consumer
short-circuits false unless the current header declares streamable and the frame
is an index. Consequently nonstreamable segments never consume this collected
state to determine packaging. Global RDF projection independently invokes
ProvenanceSubjects::from_graph; that does not justify reader-side unused state.

Bounded actual public read/read_to_sink calls on IDENTICAL byte inputs compare
captured base and exact head. Both return 50,000 content rows without diagnostics.

| Ordinary input / caller | Base allocations / requested bytes | Head allocations / requested bytes | Added traffic |
|---|---:|---:|---:|
| 50,000 unique subjects / eager | 700,313 / 115,249,595 | 700,328 / 117,477,991 | 15 allocations; 2,228,396 bytes |
| 50,000 unique subjects / evented | 850,393 / 116,200,709 | 850,408 / 118,429,105 | 15 allocations; 2,228,396 bytes |
| 100 subjects over 50,000 rows / eager | 101,468 / 40,977,643 | 101,474 / 40,982,023 | 6 allocations; 4,380 bytes |
| 100 subjects over 50,000 rows / evented | 101,848 / 35,751,957 | 101,854 / 35,756,337 | 6 allocations; 4,380 bytes |

Input sizes are 1,988,969 and 242,444 bytes. Unique-subject eager peak working
layout demand increases 47,701,413 -> 48,447,725 (+746,312 bytes).
Evented peak is unchanged because another phase determines its peak; extra
traffic is still measured. Retention after result drop is zero. These separate
metrics are layout traffic/working demand, not RSS or final Graph retention.

Genuine streamable compaction of the SAME source is a positive control:
both readers return 50,004 rows without diagnostics. Its required classification
and measured overhead are not this finding's scope. Earlier foreign predicates
must continue to disqualify candidate provenance subjects regardless of type order.

Required fix: bypass role observation/state collection for segments whose header
cannot produce a streamable packaging index, through the shared reader path.
Keep the sole classifier for genuine streamable reads/global content projection.
Do not skip foreign-before-type facts, weaken UTC/mandatory fields, introduce
another parser or change crypto fallback/entropy contracts.

Acceptance: rerun the SAME ordinary/genuine-pack probe on corrected source;
ordinary reads must no longer pay subject-cardinality-dependent provenance
allocation. Preserve actual native/wasm eager/evented packaging/authorship/refold
tests: foreign-before-type, valid/invalid UTC, reserved-class ordinary content,
genuine pack plus fresh history and genuine packaging. No arbitrary percentage
speed gate is imposed; removal of demonstrated unconsumed work is required.

## Reproducible evidence and limits

Final evidence manifest raw/S2-performance-final-artifacts.sha256 SHA256
9913312c85c1d203a39f22f213b8348fe9193e4dfbabba5345424d744d634054
binds common probe, both manifests/locks, final logs/unit graphs and corpus.
Common first-party Rust probe raw/S2-reader-performance.rs SHA256
f9646a8549b8ed5f27e5bdbe822c5885771fd1555f87447cf03c4424a6089f87.

Baseline raw/S2-performance-base is selective git-archive original source,
NOT another branch/worktree. Manifests/toolchain/lock/source/Rust targets come
from captured base blobs; no baseline body was edited. Reader was compared to
its original Git blob. Missing explicit bench targets in initial extraction were
supplied from the SAME base; no stubs or weakened manifests. Initial setup and
Cargo path-collision failures remain in *-build.log / *-build-qualified.log.
Final variants use their own identical original alloc-probe source, SHA256
606ab5130ee465d20272cf35e156a0006c8ab492c57b9e8c3b98ffa59b2e6a8a.

Corpus generated ONCE by actual head Writer and compact_streamable outside
measurement, mandatory Ed packaging with explicit fixed key and valid UTC.
Both variants read that exact corpus; logs record matching BLAKE3/byte lengths.
SHA256 inputs:

- ordinary_unique.gts: 04dadb1222c4e8d6ca815448f982322e7e6a91287012ad7cec00f4da18a51789
- ordinary_repeated.gts: 304769b8d924deeba126f62d51388b865c6ab70ddfd96bc0dfa6d6e365aaed8b
- genuine_pack.gts: 8070e9d66478af989c3cabf7cb23bd4cccb6b5cb77f3025ba6a8b9889f5c03e7

Probe installs existing purrdf-alloc-probe CountingAllocator/WholeProcessWindow,
covering possible Rayon work. No homemade allocator/runtime shipping dependency.
Two warmups, one allocation window and eleven elapsed samples per workload;
asserted counts/diagnostics, median/min/max. Variants ran sequentially.

Final commands from the worktree use CARGO_BUILD_JOBS=2 and absolute corpus:
cargo test --locked --offline --manifest-path FULL_STAGE/raw/S2-performance-{base,head}-probe/Cargo.toml
--test performance -- {base,head} ABS_CORPUS.
BOTH terminated exit 0, evidence raw/S2-performance-{base,head}-normal-profile.log.

Final artifact manifests mirror repository dev/test profiles: opt3, dev debug1,
assertions/overflow on, test debug0, package debugfalse, build-override opt3.
Compiler rustc nightly 4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM23.1.1,
native x86_64. Actual Cargo unit graphs retained as
raw/S2-performance-{base,head}-unit-graph.json; runtime-unit opt3/assertion/overflow
predicate passes both (raw/S2-performance-runtime-profile-check.log).
Cargo run-custom-build metadata defaults differ from runtime units, matching
unmodified repository build override; not mislabeled runtime qualification.
Standalone locks have identical package/version/source/checksum inventory after
normalizing ONLY own probe name; raw/S2-performance-lock-identity-diff.log empty.
These are paired public measurement locks, not workspace release qualification.

Initial preliminary profiles omitted repository build-override/debug details;
artifact manifests corrected and final executions supersede those runs.
Initial/reverse logs remain *-measured.log / *-reverse.log. Failed relative corpus
argument was corrected to absolute paths (Cargo executes tests in package cwd).
No failed observation is called a pass.

Shared-host timing is noisy: final unique-subject medians base/head 40.10/45.15 ms
eager and 46.65/47.12 ms evented; reverse runs vary and repeated/pack times can
move the other way. No stable percentage slowdown, confidence interval,
cross-machine latency or publication-ready throughput claim follows.
Allocation counts/traffic repeat exactly in ALL successful runs and establish
this finding without a timing claim.

## Other changed hot paths

ML-DSA uses fixed 256-coefficient polynomials, O(N log N) NTT/inverse and
transform-domain matrix products. A 6x5 public matrix and transformed secrets
are reused across bounded per-operation rejection attempts; guarded vector scratch
does not add a per-coefficient map. No equivalent base composite operation exists.
No unmeasured caching/SIMD/target timing claim or mandatory redesign is made.

SHAKE reuses Keccak/block absorption and fixed-size output state, writes caller
slices, allocates no XOF-output Vec. Strict Sign1 moves protected/signature
buffers, allocates exact optional kid, validates unique labels with ordered sets,
and performs bounded linear canonical decoding before authentication.
Those checks establish malformed-before-lookup behavior; no required parser
performance defect was demonstrated.

Certifier/compactor preserve mandatory verification/carried evidence.
Per-node graph scans already existed in base carried-signature extraction;
current full-field scans establish duplicate refusal. No invented newly introduced
quadratic regression or demand to remove integrity checks. S2-P1 is the sole
required performance finding.

No full gate, hosted CI, current-main integration, merge/signing/push,
native-vs-wasm throughput equivalence, formal constant time or certification
was performed/claimed. Existing runtime behavior evidence does not discharge the
measured unused-state concern. Exact-source remediation and narrow independent
recheck are required before Stage 2 performance acceptance.

