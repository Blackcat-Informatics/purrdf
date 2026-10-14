# Resident property cursor bridge: independent source review

Verdict: PASS for the bounded three-file source delta. Runtime, strict checks and the unchanged census remain the executing owner's responsibility; this is not full S1 acceptance.

Read the actual diff in sparql-eval/src/property_fn.rs, hnsw/src/relation.rs and retrieval/tests/real_producers.rs, and compared the production macro and HNSW declarations against origin/main. The shared bridge and HNSW conversion match the already-landed production implementation. S1's fallible builder fixture adaptations remain intact.

Both converted implementations override open_admitted, so the documented default-method recursion hazard is absent. Expansion passes the original arguments and ceiling, constructs the same resident capability, and uses the same into_resident conversion. It does not alter the admitted cursor's ownership, error propagation, lazy search or work accounting. CountingTextRelation still increments its original invocation counter and constructs the same optional refusal/evidence wrapper in its admitted method. The macro contains one bridge body; the two handwritten copies are removed rather than retained as alternatives.

The exported declaration uses crate-qualified public types and core/std-qualified container types. HNSW's cursor trait import moves to its existing test module; production documentation links are explicitly qualified. No runtime dependency, tooling, census ledger, fixture workload or public trait signature changes are introduced. No material source finding was identified. This reviewer ran no build and changed no production source.
