# Branch under review — issue #454

Branch `paudley/454-rdflib-shim-algebra-level-reassignment` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (5)

- `08e7cd585` Align Python scoreboard with qualified conformance results  _2026-10-08_
- `239c3c1aa` Refresh generated conformance and SIMD measurements  _2026-10-08_
- `6ce3cc656` Complete contextual application policy fixtures  _2026-10-08_
- `56e636497` fix(rdflib): route graph queries through contextual algebra  _2026-10-06_
- `e72660e87` fix(sparql): compile contextual mappings through native algebra  _2026-10-06_

## Files changed

```
CHANGELOG.md                                       |    8 +
 bindings/python/python/src/purrdf/__init__.pyi     |   31 +-
 .../python/src/purrdf/compat/rdflib/graph.py       |   26 +-
 .../python/src/purrdf/compat/rdflib/query.py       |   27 +-
 bindings/python/src/py_store/quad_store.rs         |  243 ++-
 bindings/python/src/py_store/query.rs              |  131 +-
 bindings/python/tests/README.md                    |    3 +-
 bindings/python/tests/test_contextual_mappings.py  |  227 +++
 .../python/tests/test_solution_row_protocol.py     |  130 ++
 crates/purrdf/src/reasoning.rs                     |   74 +
 crates/rdf-core/src/small.rs                       |    4 +
 crates/rdf/src/projections/dataset_description.rs  |   74 +
 crates/shapes/src/prebinding.rs                    |   69 +
 crates/shapes/src/profile.rs                       |    5 +
 crates/shapes/src/shapes/parser/admission.rs       |   20 +-
 crates/shapes/src/srl/reads.rs                     |   42 +
 crates/slice/src/ownership.rs                      |   48 +-
 crates/sparql-algebra/src/algebra.rs               |   40 +
 crates/sparql-algebra/src/owned.rs                 |   51 +-
 crates/sparql-algebra/src/parser.rs                |   60 +-
 crates/sparql-algebra/src/parser/machine.rs        |  147 +-
 crates/sparql-algebra/src/parser/triples.rs        |    2 +-
 crates/sparql-algebra/src/retained_size.rs         |   31 +
 crates/sparql-algebra/src/serialize.rs             |   20 +
 crates/sparql-algebra/src/traits.rs                |   45 +-
 crates/sparql-algebra/src/validate.rs              |  134 +-
 crates/sparql-algebra/src/walk.rs                  |   83 +
 crates/sparql-algebra/tests/iterative_traits.rs    |   71 +
 .../tests/serializer_roundtrip_sweep.rs            |  102 ++
 crates/sparql-eval/src/basic_profile.rs            |   16 +
 crates/sparql-eval/src/bgp.rs                      |    8 +-
 crates/sparql-eval/src/binop.rs                    |  648 +++++++-
 crates/sparql-eval/src/blank_scope.rs              |   40 +-
 crates/sparql-eval/src/cdt_agg.rs                  |    7 +
 crates/sparql-eval/src/construct.rs                |    2 +
 crates/sparql-eval/src/engine.rs                   |  213 ++-
 crates/sparql-eval/src/eval.rs                     |  277 +++-
 crates/sparql-eval/src/expr.rs                     |  302 +++-
 crates/sparql-eval/src/governor/soundness.rs       |  144 +-
 crates/sparql-eval/src/lib.rs                      |    3 +-
 crates/sparql-eval/src/modifier.rs                 |  923 +++++++++++-
 crates/sparql-eval/src/prebind_memo.rs             |    3 +
 crates/sparql-eval/src/property_fn_eval.rs         |   27 +
 crates/sparql-eval/src/property_fn_plan.rs         |  110 ++
 crates/sparql-eval/src/rdflib.rs                   | 1263 ++++++++++++++++
 crates/sparql-eval/src/remote.rs                   |   15 +
 crates/sparql-eval/src/row_checkpoint.rs           |   74 +-
 crates/sparql-eval/src/service_endpoints.rs        |   40 +-
 crates/sparql-eval/src/substitute.rs               |   65 +-
 .../tests/fixtures/contextual-mappings.json        |    1 +
 crates/sparql-eval/tests/native_xpath.rs           |   53 +
 crates/sparql-eval/tests/rdflib_contextual.rs      | 1585 ++++++++++++++++++++
 crates/xsd/src/exact/cost.rs                       |   66 +-
 docs/CONFORMANCE.md                                |   13 +-
 docs/book/po/zh-Hans.po                            | 1180 ++++++++-------
 docs/book/src/interop/rdflib.md                    |   15 +-
 docs/book/src/sparql/querying.md                   |   14 +-
 docs/design/purrdf-simd.md                         |    2 +-
 58 files changed, 8076 insertions(+), 981 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

