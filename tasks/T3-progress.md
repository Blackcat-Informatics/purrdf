Task 3: canonical confirmations and guarded semantics

Commit 3912955a5 shares the exact candidate witness law with assumed confirmations
across firings, rounds and schedule groups. Owned fact surfaces and retained
height sums survive retraction; ordinary earlier admission proofs stay intact.
Actual SRL/SHACL regressions preserve disconnected solution multiplicity,
FreshBlank/BNODE mint state, multirow producers and computed negation. Scheduled
Datalog regressions cover crossed operational errors and malformed callback rows.

Focused validation passed: 328 Datalog unit tests and one doc test; 23 SRL,
26 rules-engine and six capacity tests; affected clippy, formatting and whitespace.
Independent Task 3 review PASS. Normal commit hooks passed.

Task 4 now owns shared round credits and admission before touched expansion.
CLI scaling, broader consumers, wasm and full qualification remain Tasks 5–6.
These checks do not bound eager callback Vec/Producer allocations or close #364.
