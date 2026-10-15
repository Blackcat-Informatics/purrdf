# Branch under review — issue #492

Branch `paudley/492-triple-only-carrier-for-rdf-1-2` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (10)

- `364fc3090` Allow complete hosted assembly projection and retain interrupted evidence  _2026-10-14_
- `463d0bf16` Measure frozen IRI hashing at its shared lexical owner  _2026-10-14_
- `78b07bb6d` Exclude metadata-only dependencies from consumer assembly evidence  _2026-10-14_
- `295ea4a8b` Admit scheduled facts before materializing retained-base overflow  _2026-10-14_
- `da57b2500` Emit cross-target consumer assembly without linking  _2026-10-14_
- `1885414d8` Place assembly consumer notes in the correct table columns  _2026-10-14_
- `2425e5b8d` Measure carrier consumers through standard Cargo builds  _2026-10-14_
- `a9e6ae626` Link ranked retrieval documentation to public argument constants  _2026-10-14_
- `29bba23ed` Clarify graph-free text ranking and identity documentation  _2026-10-14_
- `525b3cf1b` Add a graph-free RDF carrier and rule execution  _2026-10-14_

## Files changed

```
.github/workflows/ci.yaml                          |   13 +-
 Makefile                                           |   22 +
 crates/datalog/README.md                           |   16 +
 crates/datalog/src/admission.rs                    |   19 +-
 crates/datalog/src/cache.rs                        |   99 +-
 crates/datalog/src/chase.rs                        |  268 +++--
 crates/datalog/src/clause.rs                       |  267 +++--
 crates/datalog/src/dictionary.rs                   |   21 +-
 crates/datalog/src/guard.rs                        |   42 +-
 crates/datalog/src/plan.rs                         |  178 ++--
 crates/datalog/src/proof.rs                        |  411 +++++---
 crates/datalog/src/schedule.rs                     |  168 +--
 crates/datalog/src/seminaive.rs                    | 1109 ++++++++++++--------
 crates/datalog/src/seminaive/evidence.rs           |   81 +-
 crates/datalog/src/seminaive/factors.rs            |  261 +++--
 crates/datalog/src/store.rs                        |  619 +++++++----
 crates/datalog/src/test_support/host_dictionary.rs |    6 +-
 crates/datalog/tests/rules_runtime.rs              |  478 +++++++++
 crates/hash/README.md                              |    5 +
 crates/helper-census/src/capacity.rs               |   84 +-
 crates/helper-census/src/source.rs                 |   13 +-
 crates/lex/src/allocation.rs                       |    5 +-
 crates/lex/src/constructors.rs                     |   33 +-
 crates/purrdf/tests/base_surface.rs                |   35 +
 crates/rdf-core/Cargo.toml                         |    4 +
 crates/rdf-core/benches/ir_layout.rs               |  120 ++-
 crates/rdf-core/src/dataset_view.rs                |   55 +-
 crates/rdf-core/src/ir/builder.rs                  |   90 +-
 crates/rdf-core/src/ir/builder/triple.rs           |  400 +++++++
 crates/rdf-core/src/ir/canon.rs                    |    1 +
 crates/rdf-core/src/ir/dataset.rs                  |  373 +++----
 crates/rdf-core/src/ir/frozen_terms.rs             |  191 ++++
 crates/rdf-core/src/ir/mod.rs                      |   10 +-
 crates/rdf-core/src/ir/pipeline_bundle.rs          |    8 +
 crates/rdf-core/src/ir/triple.rs                   |  402 +++++++
 crates/rdf-core/src/ir/triple/read.rs              |  343 ++++++
 crates/rdf-core/src/ir/validate.rs                 |   52 +-
 crates/rdf-core/src/lib.rs                         |   19 +-
 crates/rdf-core/tests/triple_carrier.rs            |  379 +++++++
 crates/rdf/src/lib.rs                              |   20 +-
 crates/shapes/Cargo.toml                           |    4 +
 crates/shapes/README.md                            |   11 +
 crates/shapes/src/rules.rs                         |  157 ++-
 crates/shapes/src/srl/document.rs                  |   48 +-
 crates/shapes/src/srl/eval.rs                      |  331 ++++--
 crates/shapes/src/srl/lower.rs                     |  211 +++-
 crates/shapes/src/srl/mod.rs                       |    2 +-
 crates/shapes/src/term.rs                          |   44 +-
 crates/shapes/tests/rules_engine.rs                |   46 +-
 crates/shapes/tests/srl_language.rs                |  246 ++++-
 crates/sparql-conformance/src/compare.rs           |   78 +-
 crates/sparql-conformance/src/lib.rs               |   79 +-
 crates/sparql-conformance/src/run.rs               |    7 +
 .../tests/golden/triple-query-inventory.tsv        |  Bin 0 -> 189434 bytes
 crates/sparql-conformance/tests/triple_carrier.rs  |  137 +++
 crates/sparql-eval/src/dataset_spec.rs             |   61 ++
 crates/sparql-eval/src/eval.rs                     |    9 +
 crates/sparql-eval/tests/host_identity.rs          |  140 ++-
 crates/text/README.md                              |   30 +-
 crates/text/src/corpus.rs                          |  105 ++
 crates/text/src/dictionary.rs                      |   43 +-
 crates/text/src/dictionary_relation.rs             |   50 +-
 crates/text/src/index.rs                           |  272 +++--
 crates/text/src/lib.rs                             |   37 +
 crates/text/src/occurrence_dictionary.rs           |   23 +-
 crates/text/src/relation.rs                        |  109 +-
 crates/text/src/score.rs                           |  135 +--
 crates/text/tests/determinism.rs                   |    2 +-
 crates/text/tests/physical_shapes.rs               |    3 +
 crates/text/tests/support/shape_cases.rs           |  218 ++++
 docs/WASM_TESTING.md                               |   14 +-
 docs/book/po/zh-Hans.po                            |   87 +-
 docs/book/src/concepts/interned-dataset.md         |   36 +-
 docs/design/purrdf-backend-contract.md             |   41 +-
 docs/design/purrdf-simd.md                         |    4 +-
 helpers-ledger.toml                                |  360 ++++---
 scripts/check-simd-asm.py                          |   44 +-
 scripts/simd-asm-manifest.toml                     |   13 +-
 scripts/simd_asm_runtime.py                        |   36 +-
 scripts/test_simd_asm_runtime.py                   |    6 +-
 80 files changed, 7577 insertions(+), 2422 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

