Adds native BLAKE3 keyed hashing, context-separated key derivation and extendable
output through the existing portable and SIMD kernels, without adding a dependency.
Both streaming buffer sizes support keyed and derived input. An owned output reader
fills caller storage and seeks without rehashing input; ordinary digests remain
the first 32 output bytes. Reset preserves the selected mode.

Closes #562.

Validation completed: all 35 official full-length mode vectors, all available
native backends and both stream buffers; real scalar/SIMD WASM replay; long Unicode
contexts, megabyte output, counter/seek boundaries and atomic range refusal;
existing frozen unkeyed/tree controls; strict affected-package compilation and
helper hygiene and normal metadata regeneration. Default context hashing uses
the ordinary dispatcher; explicitly forced streams retain their named backend.

Before merge: finish the seven-configuration assembly projection, commit its
generated count changes, and satisfy hosted qualification (including real
Arm/NEON execution) and source review. No timing measurements were run.
