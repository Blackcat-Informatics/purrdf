# Final attachment native collection failure

`ordinary-attachment-native-family.log` terminated 101. The retrieval unit suite
and every executed retrieval integration target passed. Its on_demand_read
benchmark had exactly twelve failed lexical-field cases: intersecting, disjoint
and disjoint_asking, on-demand and materialised, at both 400 and 4000 rows. All
six depth_taking controls passed. Cargo then stopped; compiled evaluator/text
runtime targets were not executed by this command. This is a failed collection,
not native qualification.

The benchmark registration declared only RequestFacet::Value but each request
also supplied its original title/body predicate. The actual planner's
carries_content check correctly refused to erase that field, producing
NoApplicableProducers before search or materialised execution. The deterministic
counterpart in tests/multimodal_read_bound.rs already declares
RequestFacet::ScopedLexicalValue, which transports the lexical text, language
and predicate through the producer's existing argument.

The benchmark now declares the same scoped lexical facet. Its twelve cases,
six depth-taking cases, producer rows, read depths, exclusion behavior and
original field requests are unchanged. No production admission rule was relaxed
and no fallback was introduced. Formatting and diff validation pass; runtime
qualification remains pending. The next complete family uses --no-fail-fast,
reuses compiled artifacts and preserves this terminal failure receipt.
