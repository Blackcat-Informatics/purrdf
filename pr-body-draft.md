RDFLib reassignment queries now compile to typed Rust algebra and run through the shared evaluator. Graph, Dataset, ConjunctiveGraph, processor and shadow callers preserve context and initial bindings through nested queries, projection, OPTIONAL, MINUS and grouping. Native SPARQL keeps strict admission, including the typed dataset-required refusal for single-graph GRAPH use and dated SHACL policy checks.

Python query rows add checked length, signed positional indexing and a lazy iterator that retains its result owner. Positional iteration preserves duplicate-labelled cells and unbound values; compatibility name lookup keeps its final-label behavior. Documentation and Chinese translations describe the interfaces. No new runtime dependency, semantic feature or second evaluator is introduced.

Validation: focused Rust behavior, strict affected lint/format/shared-home gates;28 irreducible Python cases against a fresh installed production wheel; English/Chinese documentation and glossary checks; an independent complete twelve-row analytical native/host cost audit. The cost proof uses its recorded analysis profiles and makes no timing or shipping fat-LTO claim. Generated conformance/SIMD tables and restated prose match their owning captured results.

Full current-head CI passed on08e7:40successful jobs and the declared optional SIMD-generation skip. Earlier failed cohorts and source-bound qualification receipts are retained. Final PR review and governed integration remain pending.

Closes #454. Closes #473.

Plan: https://github.com/Blackcat-Informatics/purrdf/issues/454#issuecomment-6027060153
