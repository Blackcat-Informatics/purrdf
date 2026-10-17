# Current contract assessment

Owner assessment, not an independent completion PASS. Source is published in
PR 568 at f3c5b1677. The complete issue and zero initial comments are captured
by the brief; the current PR comments and review availability are in
tasks/pr-current.json. No required scope is moved to another issue.

| Requirement | Evidence / remaining work |
| --- | --- |
| Keyed hash and streaming | Existing key/flags owner reaches all chunks/parents; official full-length answers and both stream sizes pass on available native and scalar/SIMD WASM backends. |
| Context derivation and streaming | Both domain flags and little-endian derived key are retained. Final default context path calls hash_mode; forced streams call hash_on with their named backend. Long Unicode contexts and all official material lengths pass after the dispatch correction. |
| Extended output | All 131 official bytes, digest prefixes, megabyte output, unrelated read partitions, cloned positions, high output-counter carry and EOF pass. Native overflow refusal is atomic. |
| No dependency / same dispatch | Manifests and lockfile unchanged; root remains dependency-free. Existing round bodies specialize eight/sixteen output words. Explicit streams do not silently substitute a scalar kernel. |
| Existing behavior | Retained native unkeyed frozen/tree/counter controls, plus final official prefix controls, cover the preserved computation. Short record finalization keeps the original direct hash_on path. |

Acceptance debt, not completed work: full assembly/document projection, required
hosted matrix including actual Arm execution, and real independent source review.
CodeRabbit currently reports quota exhaustion despite a successful check status;
retry once at the recorded time. The initial assembly attempt was interrupted
after a real dispatch finding and is not a pass. The corrected run continues.
No new implementation gap is established by current focused results; final
completion remains unverified while these acceptance obligations are pending.

The first hosted assembly failure is now diagnosed from the completed job log:
wasm32-simd128 reports only one stale document cell, 2141 recorded packed
operations versus 2147 measured for blake3.hash-many-dispatch. It reports no
kernel threshold/mnemonic failure. Retain unchanged thresholds and let the
existing full projection generator update the document; no Rust fix or new
benchmark is indicated by this result. See tasks/wasm-simd-asm-job-raw.log.
