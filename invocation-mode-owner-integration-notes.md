# Invocation-mode physical walk admission

Status: source proposal, unqualified.

Stage/invocation-mode-owner-draft.patch replaces only the existing invocation_mode and term_is_bound region in property_fn_plan.rs. Ordinary git apply --check and Stage rustfmt syntax passed. No source build or test was performed by the author.

Native API: invocation_mode_with_memory(call, &dyn Fn(&Variable)->bool, &mut Memory) -> Result<BindingPattern,StorageError>.

The same native boundness body handles constants, non-distinguished blank variables, variables, variable predicates and quoted subject/object triples in original order. It borrows certainty membership instead of creating a raw copied HashSet. The existing common WorkList keeps eight pending terms inline without allocation; try_push_admitted prices actual spill and release_admitted destroys spill before reducing the original grant, including early false answers. A physical failure propagates instead of returning false. The original BindingPattern MAX_ARITY=64 is its existing value-space contract, not a new supported subset; a fixed stack array collects those Boolean positions without heap storage.

Resident invocation and test-only term wrapper use this same body with common allocation::Resident. Existing generated terms vs recursive oracle and 100k-depth cases remain meaningful semantic tests but are UNRUN on this change. Forecast caller must pass its original LexicalFrame and immutable VarSchema membership and map StorageError through that original frame's stored typed failure. New/updated public forecast tests must prove actual allocator peak, healthy parity and deep spill refusal. No source/public qualification follows from this proposal's syntax/apply checks.
