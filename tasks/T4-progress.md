Task 4 is committed and pushed as `3ec2dbba8` with normal verification hooks.

The evaluator now shares one successful-credit pool and one refusal latch across
round workers, admits touched factor/head/negative/guard expansion before owned
work, and refuses without committing an incomplete closure. Worker counts are
exact and deterministic, including zero, exact, one-below and full-width limits.

Focused validation passed: 333 Datalog library tests, one actual allocation
integration test, one doc test, all-target clippy and formatting. The affected
23 SRL, 26 rules and six capacity tests also passed. Independent review found
post-refusal negative-local cloning; the fix passed re-review and the regression
test. Its narrow defective control requested 8,393,905 bytes, while the restored
implementation passed the less-than-262,144-byte assertion.

Real CLI 1k/10k/100k measurements, further consumer/wasm qualification and the
settled full gate remain Tasks 5–6. Caller-produced eager Vec/string allocations
and full physical budget/continuation contracts remain later portfolio work;
issue 364 is not complete.
