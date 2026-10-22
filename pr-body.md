Evidence query entry points and scoped retrieval required infallible reads, preventing their use with lazy segmented views. New fallible answer, status, interned and prepared witnessed entry points reuse the existing evaluator; scoped search retains the original source account through candidate binding, exclusion lookups and fusion. Read and admission failures return their original typed cause and operational receipt, with the final checkpoint preventing partial ranked answers from escaping.

Validation: 62 native composition cases, 66 ownership regressions, strict affected all-target checking, and 45 cases on each scalar and SIMD WASM pass. Controls cover actual segmented/paged sources, original non-latching read causes, deferred exclusion failures, final descriptor drift, prepared recovery and semantic unknown neighbors. Helper gates and the normal commit hook pass. Full hosted checks and independent review are required before merge.

Closes #682.
