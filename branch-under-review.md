# Branch under review — issue #365

Branch `paudley/365-post-v3-0-asynchronous-wasm-lane-and` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (8)

- `1057a7846` Retain the original synchronous nested-query acceptance floor  _2026-10-14_
- `be0ab6cfd` docs(sparql)!: document compact projection migration  _2026-10-13_
- `a73408ddf` Exercise both projection forms in evaluator integration tests  _2026-10-12_
- `ae502bfc1` Keep nested SELECT star projections persistent across consumers  _2026-10-12_
- `0cbe9b4f4` Share canonical GTS term identity across nested comparisons  _2026-10-12_
- `ce2be1058` Classify typed evaluator stack refusal as client error  _2026-10-11_
- `f376bf5f9` fix(wasm): align nested stack refusals across query lanes  _2026-10-11_
- `2d1ee69cb` Give poisoned WASM package errors a stable code  _2026-10-11_

## Files changed

```
CHANGELOG.md                                       |   8 +
 crates/gts/src/writer.rs                           | 688 ++++++++++++++++++++-
 crates/purrdf/src/reasoning.rs                     |  21 +-
 crates/rdf-wasm/js/README.md                       |  16 +-
 crates/rdf-wasm/js/cloudflare.d.ts                 |   6 +-
 crates/rdf-wasm/js/src/purrdf_jspi.mjs             |   4 +-
 .../rdf-wasm/js/tests/async-concurrency.test.mjs   |  41 +-
 crates/rdf-wasm/js/tests/cloudflare-stack.test.mjs |  85 +++
 crates/rdf-wasm/js/tests/cloudflare.test.mjs       |   9 +-
 crates/rdf-wasm/js/tests/fixtures/nesting.mjs      |   2 +-
 .../js/tests/fixtures/poisoned-surface.mjs         |   4 +-
 crates/rdf-wasm/js/tests/panic-poison.test.mjs     |   2 +-
 .../rdf-wasm/js/tests/stack-lane-parity.test.mjs   |  64 ++
 crates/rdf-wasm/js/tests/trap-poison.test.mjs      |  10 +-
 crates/rdf-wasm/src/interleaving.rs                |   7 +
 crates/rdf-wasm/src/operation.rs                   |  16 +
 crates/rdf-wasm/src/protocol.rs                    |  11 +
 crates/rdf/src/projections/dataset_description.rs  |   1 +
 crates/shapes/src/prebinding.rs                    |   9 +-
 crates/shapes/src/shapes/parser/functions.rs       |   1 +
 crates/shapes/src/shapes/parser/node_expr.rs       |   5 +-
 crates/shapes/src/srl/reads.rs                     |   1 +
 crates/slice/src/ownership.rs                      |   1 +
 crates/sparql-algebra/benches/algebra_walks.rs     |   2 +-
 crates/sparql-algebra/src/algebra.rs               |  18 +
 crates/sparql-algebra/src/lib.rs                   |   2 +
 crates/sparql-algebra/src/owned.rs                 |   5 +
 crates/sparql-algebra/src/parser.rs                | 323 +++++++++-
 crates/sparql-algebra/src/parser/machine.rs        | 162 +++--
 crates/sparql-algebra/src/projection.rs            | 560 +++++++++++++++++
 crates/sparql-algebra/src/retained_size.rs         |  51 +-
 crates/sparql-algebra/src/serialize.rs             | 378 +++++++++--
 crates/sparql-algebra/src/substitute.rs            |  21 +
 crates/sparql-algebra/src/traits.rs                |  12 +-
 crates/sparql-algebra/src/validate.rs              |   6 +
 crates/sparql-algebra/src/walk.rs                  |   4 +
 .../tests/alternative_property_paths.rs            |  41 +-
 crates/sparql-algebra/tests/cdt_fold_unfold.rs     |   4 +-
 .../sparql-algebra/tests/chain_parse_is_linear.rs  | 454 +++++++++++++-
 crates/sparql-algebra/tests/iterative_traits.rs    |   4 +
 .../sparql-algebra/tests/linear_property_paths.rs  |  16 +-
 .../tests/nesting_bounded_by_memory.rs             |  31 +-
 .../tests/serializer_nesting_roundtrip.rs          |   4 +-
 .../tests/serializer_roundtrip_sweep.rs            |  23 +-
 crates/sparql-algebra/tests/support/patterns.rs    |   4 +-
 .../tests/support/scope_candidates.rs              |  14 +-
 crates/sparql-conformance/src/run.rs               |   1 +
 crates/sparql-eval/src/basic_profile.rs            |   2 +
 crates/sparql-eval/src/bgp.rs                      |   4 +
 crates/sparql-eval/src/bgp/positive.rs             |   6 +-
 crates/sparql-eval/src/blank_scope.rs              |   4 +
 crates/sparql-eval/src/construct.rs                |   2 +
 crates/sparql-eval/src/deferred_exists.rs          |  13 +-
 crates/sparql-eval/src/enf.rs                      |  42 +-
 crates/sparql-eval/src/eval.rs                     |  30 +-
 crates/sparql-eval/src/expr.rs                     |  33 +
 crates/sparql-eval/src/governor/soundness.rs       |  49 +-
 crates/sparql-eval/src/modifier.rs                 |  24 +-
 crates/sparql-eval/src/prebind_memo.rs             |   3 +
 crates/sparql-eval/src/property_fn_eval.rs         |  15 +-
 crates/sparql-eval/src/property_fn_plan.rs         |  55 +-
 crates/sparql-eval/src/protocol.rs                 |   7 +-
 crates/sparql-eval/src/rdflib.rs                   |   6 +-
 crates/sparql-eval/src/remote.rs                   |  11 +-
 crates/sparql-eval/src/service_endpoints.rs        |  38 +-
 crates/sparql-eval/src/substitute.rs               |  70 ++-
 .../tests/alternative_property_paths.rs            |   9 +-
 crates/sparql-eval/tests/nested_sub_select.rs      |  29 +
 crates/sparql-eval/tests/property_function_e2e.rs  |  20 +-
 crates/sparql-eval/tests/protocol.rs               |   8 +
 crates/sparql-eval/tests/source_header.rs          |   8 +-
 crates/sparql-eval/tests/union_join_positions.rs   |   3 +-
 docs/MIGRATION-4.0.md                              |  44 ++
 73 files changed, 3362 insertions(+), 325 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

