# Prior-art assessment

Read the generated brief and its coverage: no comments, linked issues, recorded
attempt or matching PR; title-term search is narrow and broader recurrence is
unknown. Source already provides bounded streaming trees, portable rounds,
SSE2/SSSE3/AVX2/AVX512, NEON and SIMD128. Reuse these homes rather than copying an
external implementation or adding a dependency. Existing differential vectors
cover unkeyed output and irregular trees but cannot certify keyed/derive or the
second half of root output. Add official frozen mode vectors and explicit
stream/reset/seek coverage. No independent agent review has occurred.
