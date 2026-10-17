# Current acceptance

PR #568 is published at 415c8b9a4 with normal serial hooks and push passing.
Rust remains unchanged from the complete real Low-risk review at 2c9c08dc5.
The requested provenance correction is published, replied to and resolved;
review-threads-current.json and provenance-review-reply.json record the result.
The separate docstring heuristic disposition is posted at comment 6230246206.

The full prior same-code CI matrix completes with 45 successful checks, four
configuration shards failing only five stale document cells, and their failed
aggregate. All seven configurations measured their kernels. The existing writer
regenerates those five cells from the recorded CI results; existing document
coverage/parity checks pass. See tasks/ci-projection-reuse.md and its raw inputs.
The duplicate local run exits 130 and is NOT a pass; its documentation-sensitive
identity mistake is recorded there. No tool, threshold or report verdict changes.

Final candidate CI passes: 50 successful checks and five expected skips,
including all seven assembly configurations. The refreshed merge brief is pass.
Base ff302c091 combines cleanly with this head at tree
66b39d32db07e0029c1992f3263472ee73a12bc6; source is clean/pushed. See
completion-assessment.md. No merge, closure or mirror success is claimed yet.
The authoritative plan is plan.md; no subagents or independent local reviewers
were used. The actual hosted source review is retained without inventing one.

The evidence below retains the chronological qualification history. Later exact
rechecks supersede earlier pending statements; interrupted projections and failed
compilations remain failures, never passes. The authoritative plan is plan.md.
No subagents or independent local reviewers were used.

| Criterion | Result and evidence |
| --- | --- |
| Official full-length hash/keyed/derive output | PASS: official-native.log, 35 cases, 131 bytes each, default dispatch, all available native backends, both buffer sizes, whole-input and unrelated 63/2053-byte updates, mode-preserving reset, cloned stream/readers, prefix/digest parity and seek boundaries. |
| Large output and counter carry | PASS: official-native.log, 1,048,579 output bytes in all modes, independent portable/backend comparison, reads split at 4093 bytes, seek across the 32-bit block-counter carry and end of stream. |
| Existing unkeyed behavior | PASS: blake3-native.log, all nine integration cases including 8,214 frozen answers, irregular trees/alignment, streaming and caller scheduling. hash-native.log: 61 library tests. |
| Strict affected compilation | strict-2.log PASS. Initial strict.log found a test-only explicit drop of a non-Drop stream; removed. Final short-dispatch consolidation/portable-unused-argument correction still needs current strict compilation. |
| Portable/SIMD execution | Scalar run active; SIMD not yet run. Native official results remain applicable to the mode/compression algorithms; short-policy consolidation uses the pre-existing policy and needs final rerun. |
| No dependency / same dispatch | Cargo manifests and lockfile unchanged. All existing single/chunk/parent kernels accept mode; eight/sixteen-word specialization shares each compression body. No scalar fallback added. |
| Final gates and review | Pending affected hygiene, assembly/source projections, final full gate, hosted CI/review and integration. |

Historical failed compile logs are not passes. api-check.log was a const-generic
test type inference error, corrected to explicitly exercise sixteen output words.
modes-check logs record bounded intermediate compile corrections. No timing,
throughput or exclusive-host measurement was run. WASM uses the installed target,
compiler, wasm-bindgen and repository runner with standard target directories.

Current focused completion: strict-final.log passes both affected packages and
all targets. overflow-native.log passes atomic refusal before output/position
mutation. modes-final-native.log passes official modes and long Unicode-context
multi-chunk derivation after short-policy consolidation. Scalar official output
and megabyte/seek checks pass in official-wasm-scalar.log; final scalar mode and
context checks pass in modes-final-wasm-scalar.log. modes-final-wasm-simd.log
passes all four exact admission/mode/context/large-output cases on real SIMD128.
The existing unkeyed suite remains applicable: consolidation preserved its
previous exact short policy, and no unkeyed computation changed afterward.

helpers.log passes: all 82 enforced jobs, 118 distinct groups and 133 registered
hash domains; no new copy or exemption was added. metadata.log passes the normal
metadata/generated/source consumers and 37 license profiles with no resulting
projection drift. The first asm.log invocation rejected combining --write-doc
with --report before any build; asm-write.log is the corrected normal seven-
configuration document-generation run. Its result is pending, not a pass.

Whole-repository qualification is reserved for the existing hosted CI matrix,
which also executes real Arm/NEON acceptance unavailable locally. Do not duplicate
that full suite locally solely because the workflow advances. Required normal
commit hooks still run; a pending or failed hosted result blocks integration.

Dispatch source finding: passing the default short backend into context hashing
forced long x86 contexts entirely through portable compression. Corrected default
derivation to call the ordinary hash dispatcher; explicitly forced streams still
call their named backend. Restored the existing direct short-record finalization
path through hash_on. dispatch-* logs cover the affected rerun. The initial
assembly launcher was interrupted after this finding, with exit 130; it is not
qualification and no hook/signing/commit process was running. A complete assembly
run on corrected source remains required. Existing byte-vector results remain
historical; their algorithms are unchanged but final dispatch coverage is pending.

Corrected dispatch qualification passes: dispatch-strict.log, dispatch-native.log,
dispatch-wasm-scalar.log and dispatch-wasm-simd.log. Both exact cases cover all
official mode outputs and long Unicode contexts through the public default and
forced entry points. helpers-dispatch.log also passes all current registrations.
asm-corrected.log is the new complete projection run, still pending. Source inputs
remain frozen. Publication uses normal warmed hook validation while that one
assembly compiler job continues; no second test/build campaign was requested.
The normal commit and all final CI/assembly obligations remain binding.

Hosted SIMD WASM assembly job 117313416164 failed only on its generated document
cell: simd128 chunks v2141 versus measured v2147. The actual job log is retained
at tasks/wasm-simd-asm-job-raw.log. gh run view could not expose a job log while
the overall run continued; the normal read-only jobs/logs API supplied it.
Raw ANSI log bytes were saved to file and escaped before display. No tool,
threshold, compiler flag or acceptance rule was changed. The existing corrected
projection run must produce the new cells before this CI debt can clear.

Second hosted finding: x86-64-v3 lost the SSSE3 and AVX2 single-block symbols to
inlining after output-width specialization. The corrected projection attempt
was stopped (exit 130) after this demonstrated failure; it is not a pass.
Commit 2c9c08dc5 retains the three existing x86 single-block kernel boundaries
with inline(never), following the established single4/Arm audit pattern. No
wrapper, manifest rename, threshold or compiler/cache change was introduced.
Crate-only standard cargo rustc release emissions at the audit CGU/LTO settings
and system nm show all six named symbols at both v3 and v4 (each kernel at eight
and sixteen output words). See kernel-symbol-v3/v4 logs, artifact JSON and nm
captures. Current strict checks, official native modes/long contexts and helper
registrations pass. Normal serial hooks and push passed. WASM byte evidence
remains applicable because only x86 kernel attributes and module docs changed.
The new hosted matrix and complete document projection remain pending.
