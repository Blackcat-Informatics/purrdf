Add BLAKE3 keyed modes and seekable extended output

Closes #562.

Native consumers can compute keyed hashes, context-separated derived keys and
arbitrary-length output without an external BLAKE3 dependency. Both supported
stream buffers accept keyed/derived construction and preserve mode on reset.
The owned output reader fills caller storage and seeks without rehashing input.

One key/domain value flows through the existing chunk, parent and SIMD kernels.
Existing compression bodies specialize eight chaining words or sixteen root
output words. Default context derivation reuses ordinary dispatch; explicitly
forced streams retain their kernel. Short record finalization keeps its direct
compression path. No dependency, feature, toolchain/cache or wire-format change.

Focused evidence: all 35 official full-length mode answers, both stream sizes,
native and real scalar/SIMD WASM; long Unicode contexts, split/clone/reset/seek
controls, megabyte output, output-counter carry and atomic overflow refusal.
Strict affected compilation, current helper registrations, normal metadata
regeneration and ordinary commit/push hooks pass. Previous unkeyed controls
retain their recorded scope; no timing measurement was used.

The actual hosted source review covers the complete implementation at Low risk.
Its minor provenance contradiction is corrected in the working tree; publication
and thread resolution remain pending. The docstring heuristic disposition is
posted with the public Rustdoc and strict missing_docs evidence.

Pending before final notes: complete seven-configuration projection and generated
counts, publication of the documentation corrections, final hosted qualification
including Arm execution, refreshed feedback/base and integration assessment.
This remains a draft; none of those pending items is claimed as passed.

Defect-Class: none

Plan: .stage/purrdf-hash-blake3-keyed-hash-derive/plan.md
Selected evidence: .stage/purrdf-hash-blake3-keyed-hash-derive
