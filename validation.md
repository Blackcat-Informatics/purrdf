# Qualification index

| Requirement | Actual evidence | Result |
| --- | --- | --- |
| Actual prepared allocation, no Kb rebuild/clone | prepared-context-tests.log | Four tests passed: actual pointer identity, unchanged preparation observations, later owner reuse, both owning/borrowed outcomes, prepared query, wrong source/question, unrecorded refusal and retained stop |
| Same complete native checker and hostile proof controls | prepared-proof-tests.log | All 174 selected proof regressions passed, including existing tampering, byte identity, prepared support, equality, source and receipt controls |
| Public portable behavior | prepared-public-tests.log | All six cases passed, including repeated prepared replay and owning-context report parity on named and scoped-blank source |
| Actual validation production caller | prepared-validation-tests.log | Both cases passed: sixteen shared service/schema vectors and imported-source checking with altered input/answer/proof rejection |
| Strict affected all-target Clippy | clippy.log | Passed for entail and validate with warnings denied |
| Scalar WASM public behavior | wasm-clash-tests.log | All six exact registered cases passed |
| Scalar WASM actual validation | wasm-validation-tests.log | Both exact cases passed, including the sixteen-vector sweep and imported-source controls |
| Lifetime mutation exclusion | prepared-lifetime-tests.log | The final compile-fail mutation-exclusion example passed |
| Normal signed commit hooks | commit.log | Passed normally, with automatic signing and no bypass or disabled verification |
| Publication, external review and ghprsq | Pending | Required before integration |

Initial compiler failures and the initial Clippy pointer-qualification findings are retained separately. They were repaired without suppressing those lints or weakening tests. The final runtime evidence above covers the prepared-verification changes; borrowing-only baseline logs remain historical and are not substituted for these results.

Prepared verification reuses the identity computed from the consumer source during recorded construction. The actual validation path therefore removes its second complete-source identity computation. No timing speedup is inferred. Proof truth, schema/source/world selection, certificate/stop semantics and wire bytes remain under the same native owner. Precise local source support, monotone ABox updates and downstream class delegation remain unfinished in the original plan.

Packaged qualification is separate from the passing scalar WASM targets. The actual optimized npm/WASM CI job is live, recorded in packaged-wasm-job.json. Its npm proof golden entry invokes check_dl_proof_golden_vectors, which includes the shared schema vectors, through the exported binding. This packaged acceptance has not passed yet. The issue remains open until that actual job qualifies. The user explicitly permits integration with hosted checks pending; no source or verification gate is weakened.

Final external documentation disposition: the latest CodeRabbit reply confirms all 28 targets, 165 named cases, 175 executions and 33 invocations; exact URI, scalar/SIMD statement and shared BLAKE3 selections; corrected six/five/three counts; contiguous Markdown; and all three function descriptions. It states the documentation mismatch is addressed and the documentation-only changes do not invalidate the reported native/scalar results. It did not rerun tests or independently verify hooks. Packaged qualification remains pending. No updated numeric documentation-coverage percentage is claimed.
