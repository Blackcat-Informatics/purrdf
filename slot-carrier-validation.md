# Final LinkML caller repair

Published in ea133b28828cbae033249eeae0c40dfe4d1c7c84. Normal hook and push pass;
source is clean. The final review request is final-slot-review-request.md/log.

The original hosted failure is ci-pytest-2443.log. The first local repair restored
member alternatives but the unchanged oracle still rejected the conforming
fixture: the official generator closes a named choice class over its empty local
attributes. compound-slot-official-oracle.log, compound-slot-diagnosis.log and its
generated/source JSON preserve that diagnosis. Mixed enum alternatives also need
the existing anonymous slot renderer. No oracle assertion was changed.

The final repair borrows original catalog bodies at reference boundaries, keeps
sibling assertions conjunctive, and reuses slot rendering. A four-entry inline
reference stack detects cycles without allocating in shallow cases; stack margin
exhaustion hard-errors. Expression count is bounded by source bytes to refuse
reference-diamond output growth. This is a count bound, not a universal linear
byte-cost claim. Ordinary class/scalar and enum carriers remain named where they
do not require this expansion. There is no schema-body clone or new public mode.

| Check | Result / exact log |
|---|---|
| Official locked LinkML 1.11.1 | slot-carrier-official-oracle.log: original 16 exact instance probes, 18 located losses, seven renames/reverse imports, eight list, 12 temporal and nine value-shape probes all pass; the one known generator has_member drop remains exactly located. |
| Native LinkML library | slot-carrier-library.log: 41 pass, including mixed member alternatives, cycle refusal and a 16-level repeated-reference diamond refusing at the expression budget. |
| Native projections | slot-carrier-projections-final.log: all 139 pass. The one corrected exact pin states both original anonymous alternatives and keeps original class bodies, cardinality and loss checks. |
| Strict compiler/lints | slot-carrier-strict-final.log: Shapes all-target Clippy passes with warnings denied. A redundant test-fixture String clone was removed after the behavior checks; the generated JSON is unchanged, and final strict checking compiles the exact fixture source. |
| Shared helper/domain hygiene | slot-carrier-helpers.log: 87 enforced jobs, 21 reasoned variants, zero open copies, all 121 distinct rows and 150 unique prefix-free hash domains pass. |
| Commit/push | slot-carrier-commit.log and slot-carrier-push.log: normal hook and publication pass; no bypass. |

Commands inherited CARGO_REAL pinned to nightly-2026-10-01 and PATH with the Cargo
wrapper first, including make linkml-oracle's Rust subprocess and commit hooks.
CARGO_BUILD_JOBS=8, RUST_TEST_THREADS=4 and RAYON_NUM_THREADS=4 bound local work.
Only one owned Cargo process ran at a time. Initial failed attempts are retained;
only the final logs above establish passing acceptance.

Production changes after 0b115e5bc are confined to LinkML projection and its exact
native pin. Core identity, SHACL parse/preparation/evaluation/rules, JSON Schema,
other projection producers, products and bindings are unchanged. Their qualifying
results and allocation ceilings in validation.md remain applicable. Fresh hosted
CI and independent review must still cover the final pushed head before merge.
