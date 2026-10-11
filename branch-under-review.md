# Branch under review — issue #515

Branch `paudley/515-sparql-eval-datalog-per-row-evidence` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (4)

- `ccfac0a5e` Normalize generated license notice endings  _2026-10-11_
- `f49c2f1a5` Carry query evidence and scoped ranked retrieval through physical shapes  _2026-10-11_
- `8229d73f9` small doc fix for gitee  _2026-10-03_
- `40a2cbf88` Buffer Gitee uploads and bound idle connections  _2026-10-02_

## Files changed

```
Cargo.lock                                         |    1 +
 Makefile                                           |   58 +
 bindings/python-rdflib-shadow/licenses/NOTICE.txt  |    1 -
 .../python-rdflib-shadow/licenses/inventory.json   |    2 +-
 bindings/python/licenses/NOTICE.txt                |    1 -
 bindings/python/licenses/inventory.json            |    2 +-
 bindings/python/src/py_retrieval.rs                |   37 +-
 crates/cdt/licenses/NOTICE.txt                     |    1 -
 crates/cdt/licenses/inventory.json                 |    2 +-
 crates/columnar/licenses/NOTICE.txt                |    1 -
 crates/columnar/licenses/inventory.json            |    2 +-
 crates/datalog/licenses/NOTICE.txt                 |    1 -
 crates/datalog/licenses/inventory.json             |    2 +-
 crates/datalog/src/evidence.rs                     |  302 ++
 crates/datalog/src/lib.rs                          |    1 +
 crates/datalog/src/seminaive.rs                    |  886 +++++-
 crates/datalog/src/seminaive/evidence.rs           |  330 +++
 crates/datalog/src/seminaive/factors.rs            |  167 +-
 crates/deflate/licenses/NOTICE.txt                 |    1 -
 crates/deflate/licenses/inventory.json             |    2 +-
 crates/ed25519/licenses/NOTICE.txt                 |    1 -
 crates/ed25519/licenses/inventory.json             |    2 +-
 crates/entail/licenses/NOTICE.txt                  |    1 -
 crates/entail/licenses/inventory.json              |    2 +-
 crates/geo/licenses/NOTICE.txt                     |    1 -
 crates/geo/licenses/inventory.json                 |    2 +-
 crates/gts/licenses/NOTICE.txt                     |    1 -
 crates/gts/licenses/inventory.json                 |    2 +-
 crates/hash/licenses/NOTICE.txt                    |    1 -
 crates/hash/licenses/inventory.json                |    2 +-
 crates/hash/src/frame.rs                           |   24 +-
 crates/hnsw/licenses/NOTICE.txt                    |    1 -
 crates/hnsw/licenses/inventory.json                |    2 +-
 crates/hnsw/src/relation.rs                        |   36 +-
 crates/iri/licenses/NOTICE.txt                     |    1 -
 crates/iri/licenses/inventory.json                 |    2 +-
 crates/iri/src/parse.rs                            |   71 +
 crates/json/licenses/NOTICE.txt                    |    1 -
 crates/json/licenses/inventory.json                |    2 +-
 crates/jsonschema/licenses/NOTICE.txt              |    1 -
 crates/jsonschema/licenses/inventory.json          |    2 +-
 crates/lex/licenses/NOTICE.txt                     |    1 -
 crates/lex/licenses/inventory.json                 |    2 +-
 crates/lex/src/allocation.rs                       |   50 +-
 .../table.rs => lex/src/allocation/ordered.rs}     |  386 ++-
 crates/lex/src/allocation/shared.rs                |   16 +
 crates/lex/src/allocation/text.rs                  |   16 +
 crates/lex/src/constructors.rs                     |    7 +
 crates/lex/src/lib.rs                              |    1 +
 crates/markdown/licenses/NOTICE.txt                |    1 -
 crates/markdown/licenses/inventory.json            |    2 +-
 crates/mime/licenses/NOTICE.txt                    |    1 -
 crates/mime/licenses/inventory.json                |    2 +-
 crates/purrdf/licenses/NOTICE.txt                  |    1 -
 crates/purrdf/licenses/inventory.json              |    2 +-
 crates/rdf-capi/licenses/NOTICE.txt                |    1 -
 crates/rdf-capi/licenses/inventory.json            |    2 +-
 crates/rdf-core/licenses/NOTICE.txt                |    1 -
 crates/rdf-core/licenses/inventory.json            |    2 +-
 crates/rdf-core/src/dataset_view.rs                |   58 +
 crates/rdf-core/src/evidence.rs                    |  425 +++
 crates/rdf-core/src/evidence/accepted.rs           |  417 +++
 crates/rdf-core/src/evidence/answer.rs             |  592 ++++
 crates/rdf-core/src/evidence/column.rs             |  313 ++
 crates/rdf-core/src/evidence/conversion.rs         |  250 ++
 crates/rdf-core/src/evidence/dictionary.rs         |  250 ++
 crates/rdf-core/src/evidence/frame.rs              |  334 +++
 crates/rdf-core/src/evidence/information.rs        |  410 +++
 crates/rdf-core/src/evidence/ledger.rs             |  353 +++
 crates/rdf-core/src/evidence/ledger/frame.rs       |  323 +++
 crates/rdf-core/src/evidence/ledger/row.rs         |  612 ++++
 crates/rdf-core/src/evidence/ledger/source.rs      |  521 ++++
 crates/rdf-core/src/evidence/ledger/terms.rs       |  660 +++++
 crates/rdf-core/src/evidence/ledger/view.rs        |  274 ++
 crates/rdf-core/src/evidence/reason.rs             |  507 ++++
 crates/rdf-core/src/evidence/record.rs             |  711 +++++
 crates/rdf-core/src/evidence/standpoint.rs         |  324 +++
 crates/rdf-core/src/evidence/workspace.rs          |  440 +++
 crates/rdf-core/src/ir/dataset.rs                  |   64 +
 crates/rdf-core/src/ir/term_walk.rs                |   99 +-
 crates/rdf-core/src/lib.rs                         |    2 +
 crates/rdf-core/src/query_scope.rs                 |  153 +
 crates/rdf-core/src/query_scope/counters.rs        |  315 ++
 crates/rdf-events/licenses/NOTICE.txt              |    1 -
 crates/rdf-events/licenses/inventory.json          |    2 +-
 crates/rdf-wasm/js/licenses/NOTICE.txt             |    1 -
 crates/rdf-wasm/js/licenses/inventory.json         |    2 +-
 crates/rdf-wasm/licenses/NOTICE.txt                |    1 -
 crates/rdf-wasm/licenses/inventory.json            |    2 +-
 crates/rdf/licenses/NOTICE.txt                     |    1 -
 crates/rdf/licenses/inventory.json                 |    2 +-
 crates/retrieval/Cargo.toml                        |    4 +
 crates/retrieval/benches/fusion_frontier_alloc.rs  |    8 +-
 crates/retrieval/benches/on_demand_read.rs         |    5 +-
 crates/retrieval/licenses/NOTICE.txt               |    1 -
 crates/retrieval/licenses/inventory.json           |    2 +-
 crates/retrieval/src/admission.rs                  |   40 +
 crates/retrieval/src/candidate.rs                  |  120 +-
 crates/retrieval/src/canonical.rs                  |  109 +-
 crates/retrieval/src/compile.rs                    |  151 +-
 crates/retrieval/src/document.rs                   |   37 +-
 crates/retrieval/src/error.rs                      |   24 +-
 crates/retrieval/src/execute.rs                    | 1754 ++++++++---
 crates/retrieval/src/fixture.rs                    |   14 +
 crates/retrieval/src/fuse.rs                       |   92 +-
 crates/retrieval/src/fusion_stream.rs              | 1227 +++++---
 crates/retrieval/src/id.rs                         |    9 +-
 crates/retrieval/src/iri.rs                        |   57 +-
 crates/retrieval/src/lib.rs                        |   13 +-
 crates/retrieval/src/matching.rs                   |   83 +-
 crates/retrieval/src/plan.rs                       |  119 +-
 crates/retrieval/src/planner.rs                    |   16 +-
 crates/retrieval/src/ranked_stream.rs              |   37 +-
 crates/retrieval/src/render.rs                     |  375 ++-
 crates/retrieval/src/request.rs                    |   19 +-
 crates/retrieval/src/scope.rs                      |  453 +++
 crates/retrieval/src/search.rs                     |  246 +-
 crates/retrieval/src/union.rs                      |  486 +++-
 crates/retrieval/tests/admission_tests.rs          |    4 +-
 crates/retrieval/tests/boundary.rs                 |   10 +-
 crates/retrieval/tests/candidate_union.rs          |  119 +-
 crates/retrieval/tests/compile_request.rs          |    4 +-
 crates/retrieval/tests/depth_free_membership.rs    |    2 +-
 crates/retrieval/tests/exclusion_lookup.rs         |    2 +-
 crates/retrieval/tests/execute_dataset.rs          |   10 +-
 .../tests/fixtures/planner/lexical_request.json    |    2 +-
 .../tests/fixtures/planner/mixed_request.json      |    2 +-
 crates/retrieval/tests/fusion.rs                   |  177 +-
 crates/retrieval/tests/multimodal_read_bound.rs    |   35 +-
 crates/retrieval/tests/on_demand_receipt.rs        |   15 +-
 crates/retrieval/tests/per_mode_row_bound.rs       |    4 +-
 crates/retrieval/tests/plan.rs                     |   41 +
 crates/retrieval/tests/planner_golden.rs           |   12 +-
 crates/retrieval/tests/real_producers.rs           | 1137 +++++++-
 crates/retrieval/tests/rendered_lookup_mode.rs     |    2 +-
 crates/retrieval/tests/search.rs                   |   89 +-
 .../retrieval/tests/supplied_exclusion_lookup.rs   |   22 +-
 crates/retrieval/tests/support/registry.rs         |    4 +-
 crates/retrieval/tests/text_exclusion_lookup.rs    |   14 +-
 crates/retrieval/tests/wasm_determinism.rs         |   90 +-
 crates/shapes/licenses/NOTICE.txt                  |    1 -
 crates/shapes/licenses/inventory.json              |    2 +-
 crates/shex/licenses/NOTICE.txt                    |    1 -
 crates/shex/licenses/inventory.json                |    2 +-
 crates/slice/licenses/NOTICE.txt                   |    1 -
 crates/slice/licenses/inventory.json               |    2 +-
 crates/sparql-algebra/licenses/NOTICE.txt          |    1 -
 crates/sparql-algebra/licenses/inventory.json      |    2 +-
 crates/sparql-algebra/src/parser/table.rs          |  550 +---
 .../tests/cost_planner_corpus.rs                   |    1 +
 .../tests/governor_correctness_corpus.rs           |    1 +
 crates/sparql-eval/Cargo.toml                      |    5 +
 crates/sparql-eval/licenses/NOTICE.txt             |    1 -
 crates/sparql-eval/licenses/inventory.json         |    2 +-
 crates/sparql-eval/src/bgp.rs                      |  186 +-
 crates/sparql-eval/src/bgp/window.rs               |  585 ++++
 crates/sparql-eval/src/binop.rs                    |  298 +-
 crates/sparql-eval/src/blank_scope.rs              |    7 +-
 crates/sparql-eval/src/candidate_scope.rs          |  328 +++
 crates/sparql-eval/src/construct.rs                |   22 +-
 crates/sparql-eval/src/deferred_exists.rs          |    6 +-
 crates/sparql-eval/src/describe_query.rs           |   71 +-
 crates/sparql-eval/src/engine.rs                   |  781 ++++-
 crates/sparql-eval/src/error.rs                    |   16 +
 crates/sparql-eval/src/eval.rs                     |  334 ++-
 crates/sparql-eval/src/evidence_exec.rs            | 2010 +++++++++++++
 crates/sparql-eval/src/execution.rs                |   91 +-
 crates/sparql-eval/src/expr.rs                     |  342 ++-
 crates/sparql-eval/src/extension_env.rs            |   20 +-
 crates/sparql-eval/src/governor/ledger.rs          |   37 +
 crates/sparql-eval/src/governor/lift.rs            |   19 +-
 crates/sparql-eval/src/interned.rs                 |  221 ++
 crates/sparql-eval/src/knn/invocation.rs           |  197 +-
 crates/sparql-eval/src/knn/mod.rs                  |   35 +-
 crates/sparql-eval/src/lib.rs                      |   28 +-
 crates/sparql-eval/src/modifier.rs                 |  635 +++-
 crates/sparql-eval/src/modifier/owners.rs          |   92 +-
 crates/sparql-eval/src/parallel.rs                 |   35 +-
 crates/sparql-eval/src/path.rs                     |  448 ++-
 crates/sparql-eval/src/property_fn.rs              |  454 ++-
 crates/sparql-eval/src/property_fn_eval.rs         |  889 ++++--
 crates/sparql-eval/src/property_fn_plan.rs         |    1 +
 crates/sparql-eval/src/remote.rs                   |   79 +-
 crates/sparql-eval/src/retained.rs                 |    2 +-
 crates/sparql-eval/src/row_checkpoint.rs           |   22 +-
 crates/sparql-eval/src/scratch.rs                  |    7 +-
 crates/sparql-eval/src/shapes/barrier.rs           |  576 ++++
 crates/sparql-eval/src/shapes/bucket.rs            |  128 +
 crates/sparql-eval/src/shapes/dictionary.rs        |  516 ++++
 crates/sparql-eval/src/shapes/mod.rs               |  308 ++
 crates/sparql-eval/src/shapes/runtime.rs           |  572 ++++
 crates/sparql-eval/src/shapes/spans.rs             |  164 ++
 crates/sparql-eval/src/shapes/window.rs            |  379 +++
 crates/sparql-eval/src/solution.rs                 |  265 +-
 crates/sparql-eval/src/substitute.rs               |    6 +
 crates/sparql-eval/src/update.rs                   |    3 +-
 crates/sparql-eval/src/user_fn.rs                  |   95 +
 crates/sparql-eval/src/vm/compile.rs               |    9 +-
 crates/sparql-eval/src/vm/evidence.rs              |  184 ++
 crates/sparql-eval/src/vm/mod.rs                   |  147 +-
 crates/sparql-eval/src/witness.rs                  |    6 +-
 crates/sparql-eval/src/workspace.rs                |  651 ++++-
 crates/sparql-eval/src/workspace/array.rs          |   50 +
 crates/sparql-eval/src/workspace/boxed.rs          |   53 +
 crates/sparql-eval/src/workspace/ordered.rs        |  340 +++
 crates/sparql-eval/tests/evidence_ingress.rs       | 3048 ++++++++++++++++++++
 crates/sparql-eval/tests/evidence_ledger_query.rs  |  410 +++
 crates/sparql-eval/tests/ranked_declaration.rs     |    4 +-
 crates/sparql-results/licenses/NOTICE.txt          |    1 -
 crates/sparql-results/licenses/inventory.json      |    2 +-
 crates/stack/licenses/NOTICE.txt                   |    1 -
 crates/stack/licenses/inventory.json               |    2 +-
 crates/text/Cargo.toml                             |   11 +
 crates/text/benches/physical_shapes.rs             |  257 ++
 crates/text/licenses/NOTICE.txt                    |    1 -
 crates/text/licenses/inventory.json                |    2 +-
 crates/text/src/dictionary.rs                      |  677 +++++
 crates/text/src/dictionary_relation.rs             |  470 +++
 crates/text/src/index.rs                           |  150 +-
 crates/text/src/lib.rs                             |   16 +-
 crates/text/src/occurrence_dictionary.rs           |  219 ++
 crates/text/src/relation.rs                        |  458 ++-
 crates/text/src/score.rs                           |  203 +-
 crates/text/tests/determinism.rs                   |    2 +-
 crates/text/tests/physical_shapes.rs               |   41 +
 crates/text/tests/scoring.rs                       |  126 +-
 crates/text/tests/support/scoped_candidates.rs     |  228 ++
 crates/text/tests/support/shape_cases.rs           | 2405 +++++++++++++++
 crates/text/tests/unrestricted_ranking.rs          |   79 +-
 crates/text/tests/wasm_determinism.rs              |   10 +
 crates/validate/licenses/NOTICE.txt                |    1 -
 crates/validate/licenses/inventory.json            |    2 +-
 crates/xsd/licenses/NOTICE.txt                     |    1 -
 crates/xsd/licenses/inventory.json                 |    2 +-
 docs/WASM_TESTING.md                               |   21 +-
 docs/design/purrdf-physical-shapes.md              |  193 ++
 docs/design/purrdf-simd.md                         |    1 +
 helpers-ledger.toml                                |    4 +-
 scripts/package-licenses.py                        |    2 +-
 239 files changed, 37409 insertions(+), 3132 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

