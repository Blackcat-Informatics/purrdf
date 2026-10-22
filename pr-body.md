Maintain a materialized Datalog model across committed insertions and retractions, preserving its original dictionary, row identities and host watermark. Signed multiplicities, native reverse supports and over-deletion/re-grounding remove unsupported cycles, find alternative finite witnesses and propagate stratified negation. Completed transactions expose exact net membership deltas; pending work withholds uncertified rows. The entailment refutation lane uses this model.

Initialization and updates share the native rule, guard and publication kernels. Retained metadata, directory, permutation, seal, sort and merge frames resume under cumulative work and original storage admission. Supplied-admission constructors account supplemental buffers while existing sealed payloads keep their owners. Compiled analysis is borrowed, and event queues use geometric growth. The independent proof checker re-decides grouped negation with native filters; caller callback meaning remains an explicit refusal by that checker.

Validation:

- Full `make check` passed, including release WASM compilation. Its native tests preceded the final bootstrap scan corrections; affected native/runtime, portable, strict and helper checks were rerun afterward.
- Native coverage: 466 Datalog library tests, 48 public runtime cases, five allocation cases, 818 entailment library tests and 16 public refutation cases; one existing entailment test remains ignored. Preparation and transaction checks cover 26,945 and 86,246 work cuts.
- Actual WASM: all 48 Datalog runtime and 16 public refutation cases through the shipped runner. The new bootstrap case checks vacant-prefix refusal, original coordinates, resumed work and absence of fabricated watermarks.
- All three explicit scale cases passed. At 1,000 and 1,000,000 edges insertion takes three joins and deletion one, with equal allocation quantities. Four churn blocks retain zero bytes. Forced 1,133,504-row compaction has equal declared work and physical factories across uninterrupted execution and 55 pauses. These measured update paths are unchanged by the bootstrap correction.
- Current all-target Datalog/entailment/SHACL/validation Clippy passes with warnings denied; formatting, helper census and hash-domain checks pass. Normal commit hooks pass.

Closes #493.
