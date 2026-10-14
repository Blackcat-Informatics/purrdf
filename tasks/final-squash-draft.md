Use exact numeric values and half-even division defaults

Public integer and decimal values use the exact tower at every magnitude.
Division preserves terminating quotients exactly and otherwise rounds to 18
fractional places, half to even, with explicit policies retained. Temporal
seconds preserve their full precision across Rust, CLI, Python, C and WASM.
Remove bounded public numeric and obsolete AVG APIs, preserve constructor
datatypes, and update governor profile v18, frozen answers and bilingual
migration guidance. Exact temporal schema projections match the value space.

The repaired full local gate passes, including release WASM. The optimized
installed WASM package passes 458 tests plus types, installation and identity
controls; installed Python passes 42 selected controls. Affected native
arithmetic, calendar, grammar, schema, AST/CDT and query/update trace controls
pass. The complete assembly producer and aggregate pass 112 sites on all seven
configurations with unchanged floors. The generated patch changes one measured
document cell. Normal signing and hooks pass throughout.

Independent numeric, temporal and admission reviews retain their source scope.
Current CodeRabbit review covers the final published repair; its source findings
and the CodeQL synthetic-budget alert are addressed. The latter's thread is
resolved. The actual main integration is clean and introduces no base delta.
Selected Stage records preserve the original failed attempts and current
correctness, host, generation, review and integration evidence separately from
shipping source.

The final source run reached its test-shard job limit while tests were running.
The published workflow repair raises that budget to 60 minutes without changing
compiler, profile, selection or assertions. Standard workflow lint and the
existing shard-selection controls pass. The replacement integration shard passes.

Current CI and Docs pass, including every test shard and all seven assembly
configurations. The final review thread is resolved. Fresh remote integration
assessment remains clean and matches the qualified main interaction. The
emergency ledger has no entries. No issue requirement is moved to follow-up.

The authoritative plan is `plan.md` in the selected Stage directory
`.stage/v4-0-exact-tower-only-and-an-exact-or`. Its current acceptance index and
review dispositions accompany the archived raw receipts.

Closes #470.
Defect-Class: bug
