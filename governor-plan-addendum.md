## Governor profile correctness and qualification addendum

Full qualification of commit `d6f94f62a80c641f3b51734c73f19c64c0140ebc` exposed a required profile transition. The per-arena scratch watermark correctly counts computed values independently of separately charged aggregate survivor buffers and custom accumulator state. The preceding global-consumption subtraction hid those allocations: the twelve-input SUM corpus retains 879 bytes of input values and a separate 74-byte result, so the correct total is 953 rather than 879.

The published governor profile requires a version increment whenever a caller's budget trip can move, including changed event counts under an unchanged fuel schedule. Its first-party corpus explicitly documents measured regeneration, freeze-manifest refresh and corpus-digest re-pin as one reviewable profile transition. Independent plan and compliance reviews accepted that procedure and rejected restoring the undercount merely to preserve the old expectations.

This amends Task 4's blanket corpus-preservation wording: all official/upstream and GTS corpus bytes remain unchanged. The specifically versioned first-party `purrdf-sparql-governors` evidence moves from profile 9 to 10. Existing input cases, independent answers and semantic requirements remain authoritative; only measured cost/boundary/trace records and their explanations may change as the corrected accounting requires.

Completion contract:

1. Add an independently calculated aggregate retention test covering the result after separate buffer charges, inclusive budget boundaries, built-in/custom parity and typed exhaustion.
2. Publish profile 10 with the unchanged fuel schedule, derived new profile digest, explicit scratch ownership/reservation semantics and consumer budget remeasurement requirements.
3. Regenerate the complete first-party governor corpus and fuel sweep through their existing generators, inspect every changed record against the accounting law, update only its freeze manifest, and re-pin its public corpus digest and prose. Verify all other frozen manifests and corpus bytes unchanged.
4. Re-run full repository, wasm, complete conformance and generated-artifact gates on the resulting committed source. Preserve failed and passing qualification logs. Commit normally with hooks, push and report the final profile identities before PR creation.

All six original investigation acceptance criteria and the issue-to-PR, CodeRabbit and structured-merge workflow remain required.
