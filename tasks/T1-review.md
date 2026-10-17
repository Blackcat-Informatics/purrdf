# Task1 shared support ownership

Review type: self-review only; no independent reviewer was available.

Implementation activates the existing Journal in recorded searches, retains its links once in DlProof, and stores only current cursor/frame per clash. Prepared bounds are copied separately; the obsolete per-clash application vectors are removed. Unrecorded decisions retain disabled cursors. Proofs without current clash evidence discard unused applications.

Evidence: arena-tests.log has3 passing cases, including actual pointer identity of a common application after two clash extractions (3 stored applications for4 total path entries), original allocation admission/refusal, and branch predecessors. Full entail-unit-tests-2.log has722 passes and1 existing ignored, including recording/search parity. Native/WASM public controls preserve certificate identity. First refusal remains typed and truncated without altering the decision.

VERDICT: PASS for implementation and self-review. This is not an independent-review verdict. Commit/push are part of the coherent tasks1–3 unit after the final helper gate.
