# Branch under review — issue #470

Branch `paudley/470-v4-0-exact-tower-only-and-an-exact-or` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (12)

- `0e9a3d9c4` Preserve exact calendar casts and temporal schema bounds  _2026-10-14_
- `50ae94847` Remove obsolete bounded decimal comparison exemption  _2026-10-13_
- `330c55f8c` geo: remove obsolete scan argument lint expectation  _2026-10-12_
- `3b05f4343` tests: align exact numeric transcript and i686 environment evidence  _2026-10-12_
- `538eb2f71` Adapt fixtures to admitted cursors and exact integers  _2026-10-12_
- `838667cab` Refresh measured Python conformance count  _2026-10-12_
- `7ee086ffd` Keep exact-value consumers strict and admitted  _2026-10-12_
- `36b31dfb1` Cover bare-scale policy in C client  _2026-10-12_
- `fff66a72e` Make exact XSD values and temporal seconds the public default  _2026-10-12_
- `2a1a15dce` Charge exact fallback for bounded numeric overflow  _2026-10-11_
- `d69c201eb` Expose exact decimal reader across XSD and geo  _2026-10-11_
- `fb40c6e3b` Advance exact numeric defaults and integer ingress  _2026-10-11_

## Files changed

```
CHANGELOG.md                                       |   12 +
 README.md                                          |    4 +-
 bindings/python/README.md                          |    8 +-
 bindings/python/python/src/purrdf/__init__.pyi     |    7 +-
 bindings/python/src/py_store/env.rs                |    7 +-
 bindings/python/tests/test_division_policy.py      |   12 +-
 crates/cdt/src/functions.rs                        |    2 +-
 crates/cli/src/cli.rs                              |    4 +-
 crates/cli/src/query.rs                            |    6 +-
 crates/cli/tests/exact_numerics_cli.rs             |   39 +-
 crates/entail/src/owl_dl/data.rs                   |    5 +-
 crates/geo-kernel/src/metric.rs                    |    8 +-
 crates/rdf-capi/README.md                          |    3 +-
 crates/rdf-capi/include/purrdf.h                   |    9 +-
 crates/rdf-capi/src/handles.rs                     |    3 +-
 crates/rdf-capi/src/query.rs                       |    6 +-
 crates/rdf-capi/src/query/division_tests.rs        |    8 +-
 crates/rdf-capi/tests/smoke.c                      |   55 +-
 crates/rdf-core/src/xsd_regex/ecma_emit.rs         |   12 +-
 crates/rdf-wasm/js/README.md                       |    3 +-
 crates/rdf-wasm/js/index.d.ts                      |    8 +-
 crates/rdf-wasm/js/tests/division-policy.test.mjs  |   36 +-
 crates/rdf-wasm/src/query.rs                       |   14 +-
 crates/rdf/src/projections/okf/mapping.rs          |    4 +-
 crates/shapes/src/json_schema/numeric_order.rs     |   28 +-
 crates/shapes/src/json_schema/temporal_order.rs    |   65 +-
 crates/shapes/src/shapes/parser/node_expr.rs       |   15 +-
 crates/shapes/tests/custom_node_expressions.rs     |   18 +
 crates/shapes/tests/emitter_projections.rs         |    9 +-
 crates/shex/src/exact.rs                           |    2 +-
 crates/shex/src/validate/node.rs                   |    6 +-
 .../tests/goldens/ast_snapshot.golden              |    4 +
 crates/sparql-conformance/src/rs_resultset.rs      |    6 +-
 .../tests/cdt_lexical_divergence.rs                |    2 +-
 .../tests/goldens/evaluator-trace/governors.trace  |   40 +
 .../tests/goldens/evaluator-trace/suite.trace      |  535 +++--
 crates/sparql-eval/src/engine.rs                   |    6 +-
 crates/sparql-eval/src/eval.rs                     |    3 +-
 crates/sparql-eval/src/expr.rs                     |  240 +-
 crates/sparql-eval/src/expr/cast_rounding_tests.rs |   21 +-
 crates/sparql-eval/src/governor/mod.rs             |   31 +-
 crates/sparql-eval/src/interned.rs                 |    3 +-
 crates/sparql-eval/src/knn/invocation.rs           |   24 +-
 crates/sparql-eval/src/list_fn.rs                  |    4 +-
 crates/sparql-eval/src/modifier.rs                 |  136 +-
 crates/sparql-eval/src/native_numeric.rs           |   83 +-
 crates/sparql-eval/src/shapes/dictionary.rs        |    2 +-
 crates/sparql-eval/src/shapes/window.rs            |    2 +-
 crates/sparql-eval/src/stat_agg.rs                 |   77 +-
 crates/sparql-eval/tests/exact_numerics.rs         |   15 +-
 crates/sparql-eval/tests/numeric_casts.rs          |   11 +-
 crates/sparql-eval/tests/numeric_governance.rs     |   86 +
 .../sparql-eval/tests/numeric_wasm_determinism.rs  |    2 +-
 crates/xsd/benches/exact.rs                        |   20 +-
 crates/xsd/examples/gen_exact_numeric_vectors.rs   |   14 +-
 crates/xsd/src/exact/cost.rs                       |  120 +-
 crates/xsd/src/exact/decimal.rs                    |   92 +-
 crates/xsd/src/exact/error.rs                      |   17 +-
 crates/xsd/src/exact/integer.rs                    |   27 +
 crates/xsd/src/exact/mod.rs                        |   22 +-
 crates/xsd/src/exact/rational.rs                   |   38 +-
 crates/xsd/src/exact/rounding.rs                   |   80 +-
 crates/xsd/src/lib.rs                              |   27 +-
 crates/xsd/src/numeric.rs                          | 1558 +++++++------
 crates/xsd/src/numeric/exact_path.rs               |   57 +-
 crates/xsd/src/ops.rs                              |  240 +-
 crates/xsd/src/range.rs                            |  114 +-
 crates/xsd/src/range/storage.rs                    |   14 +-
 crates/xsd/src/rational.rs                         |  352 ---
 crates/xsd/src/temporal.rs                         | 2322 +++++++++++++++-----
 crates/xsd/src/value.rs                            |  246 +--
 crates/xsd/tests/exact_governance.rs               |  961 +++++++-
 crates/xsd/tests/exact_tower.rs                    |  215 +-
 crates/xsd/tests/exact_values.rs                   |   34 +-
 crates/xsd/tests/numeric_error_codes.rs            |   39 +-
 crates/xsd/tests/range_storage.rs                  |   72 +
 crates/xsd/tests/value_space.rs                    |   11 +-
 crates/xsd/tests/vectors/exact_numeric_vectors.txt |  480 ++--
 docs/CONFORMANCE.md                                |    6 +-
 docs/CUTOVER.md                                    |    6 +-
 docs/SPARQL-GOVERNOR-PROFILE.md                    |   20 +-
 docs/book/po/zh-Hans.po                            |  962 ++++----
 docs/book/src/sparql/querying.md                   |   22 +-
 helpers-ledger.toml                                |   88 +-
 .../vectors-sparql-governors.sha256                |   17 +-
 vectors/sparql-governors/README.md                 |   14 +
 vectors/sparql-governors/cases/numeric-default.rq  |    8 +
 vectors/sparql-governors/cases/numeric-default.ttl |    4 +
 vectors/sparql-governors/cases/temporal-exact.rq   |   12 +
 .../expected/numeric-default-fuel-boundary.answer  |    3 +
 .../expected/numeric-default-fuel-boundary.metered |    5 +
 .../expected/numeric-default-fuel-boundary.spend   |    5 +
 .../numeric-default-fuel-over-bound.answer         |    5 +
 .../numeric-default-fuel-over-bound.metered        |    5 +
 .../expected/numeric-default-fuel-over-bound.spend |    5 +
 .../expected/temporal-exact-fuel-boundary.answer   |    3 +
 .../expected/temporal-exact-fuel-boundary.metered  |    5 +
 .../expected/temporal-exact-fuel-boundary.spend    |    5 +
 .../expected/temporal-exact-fuel-over-bound.answer |    5 +
 .../temporal-exact-fuel-over-bound.metered         |    5 +
 .../expected/temporal-exact-fuel-over-bound.spend  |    5 +
 vectors/sparql-governors/manifest.tsv              |    6 +-
 102 files changed, 6397 insertions(+), 3747 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

