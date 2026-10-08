# Branch under review — issue #402

Branch `paudley/402-shacl-profiles-reports` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (20)

- `4faeb36ce` Pre-bind no shape context under the dated draft  _2026-10-06_
- `737cc1358` Expect the ordinary lane to admit a bound request like the SHACL lane  _2026-10-06_
- `7a35da6e0` Leave an unvalued shape context undeclared under the dated draft  _2026-10-06_
- `865e9ba18` Admit a dated query by the algebra its text parsed to  _2026-10-06_
- `139582384` docs(simd): give the conformance kit its crate-level site row  _2026-10-06_
- `44f6edf57` docs(iri): link the EARL schema rather than leaving a bare URL  _2026-10-06_
- `af23b4637` docs(book): count twelve never-published members in both languages  _2026-10-06_
- `372f98baa` docs(changelog): record dated SHACL profiles, complete reports and evaluator error changes  _2026-10-06_
- `0c7b4246c` test(validate): read every validation-result property from one complete result  _2026-10-06_
- `dec49c3da` test(sparql-conformance): run the community SHACL corpus under every dated profile  _2026-10-06_
- `d56f3fd5b` feat(conformance-kit): add the portable conformance grading kit  _2026-10-06_
- `f351bde59` test(corpora): add the community SHACL corpus and its agent review  _2026-10-06_
- `785894bd1` feat(iri): name the W3C test manifest, result-set, SHACL test and EARL terms  _2026-10-06_
- `08c399c15` test(shacl): verify dated alternatives preserve resource refusals  _2026-10-05_
- `9b88ac778` Compose dated SHACL requests with complete native reports  _2026-10-05_
- `b5a4f7994` Preserve XPath allocation resource diagnostics  _2026-10-05_
- `adcdb47a5` Enforce dated SHACL admission at actual query occurrences  _2026-10-05_
- `0142b7a30` Preserve typed function admission through final query boundaries  _2026-10-05_
- `37dc716ab` Add complete contextual SHACL reports  _2026-10-05_
- `8520523dd` Add dated SHACL query admission profiles  _2026-10-05_

## Files changed

```
AGENTS.md                                          |    5 +-
 CHANGELOG.md                                       |   68 +
 Cargo.lock                                         |   17 +
 Cargo.toml                                         |    2 +
 corpora/community/LICENSING.md                     |   33 +
 corpora/community/README.md                        |   38 +
 corpora/community/SPEC.md                          |   86 +
 corpora/community/catalog.json                     |   51 +
 corpora/community/licenses/Apache-2.0.txt          |  202 ++
 corpora/community/licenses/CC-BY-4.0.txt           |  156 +
 corpora/community/licenses/MIT.txt                 |   21 +
 corpora/community/licenses/MulanPSL-2.0.txt        |  130 +
 corpora/community/manifest.ttl                     |    5 +
 corpora/community/reviews/index.json               | 1559 ++++++++++
 corpora/community/reviews/shacl.md                 |  197 ++
 .../manifest.ttl                                   |   21 +
 .../shapes.ttl                                     |   12 +
 .../manifest.ttl                                   |   14 +
 .../shapes.ttl                                     |   12 +
 .../admission-local-values-draft/manifest.ttl      |   21 +
 .../shacl/admission-local-values-draft/shapes.ttl  |   12 +
 .../shacl/admission-local-values-rec/manifest.ttl  |   14 +
 .../shacl/admission-local-values-rec/shapes.ttl    |   12 +
 .../shacl/admission-minus-draft/manifest.ttl       |   14 +
 .../shacl/admission-minus-draft/shapes.ttl         |   12 +
 .../shacl/admission-minus-rec/manifest.ttl         |   14 +
 .../community/shacl/admission-minus-rec/shapes.ttl |   12 +
 .../shacl/admission-prebound-as-draft/manifest.ttl |   14 +
 .../shacl/admission-prebound-as-draft/shapes.ttl   |   12 +
 .../shacl/admission-prebound-as-rec/manifest.ttl   |   14 +
 .../shacl/admission-prebound-as-rec/shapes.ttl     |   12 +
 .../admission-prebound-values-draft/manifest.ttl   |   14 +
 .../admission-prebound-values-draft/shapes.ttl     |   12 +
 .../admission-prebound-values-rec/manifest.ttl     |   14 +
 .../shacl/admission-prebound-values-rec/shapes.ttl |   12 +
 .../shacl/admission-service-draft/manifest.ttl     |   14 +
 .../shacl/admission-service-draft/shapes.ttl       |   12 +
 .../shacl/admission-service-rec/manifest.ttl       |   14 +
 .../shacl/admission-service-rec/shapes.ttl         |   12 +
 .../manifest.ttl                                   |   25 +
 .../aggregate-subquery-blank-conforming/shapes.ttl |   12 +
 .../manifest.ttl                                   |   31 +
 .../aggregate-subquery-blank-violating/shapes.ttl  |   12 +
 .../aggregate-subquery-iri-conforming/manifest.ttl |   25 +
 .../aggregate-subquery-iri-conforming/shapes.ttl   |   12 +
 .../aggregate-subquery-iri-violating/manifest.ttl  |   31 +
 .../aggregate-subquery-iri-violating/shapes.ttl    |   12 +
 .../manifest.ttl                                   |   25 +
 .../anchored-optional-blank-conforming/shapes.ttl  |   12 +
 .../anchored-optional-blank-violating/manifest.ttl |   31 +
 .../anchored-optional-blank-violating/shapes.ttl   |   12 +
 .../anchored-optional-iri-conforming/manifest.ttl  |   25 +
 .../anchored-optional-iri-conforming/shapes.ttl    |   12 +
 .../anchored-optional-iri-violating/manifest.ttl   |   31 +
 .../anchored-optional-iri-violating/shapes.ttl     |   12 +
 .../manifest.ttl                                   |   25 +
 .../ask-validator-parameter-conforming/shapes.ttl  |   14 +
 .../ask-validator-parameter-violating/manifest.ttl |   29 +
 .../ask-validator-parameter-violating/shapes.ttl   |   14 +
 .../complex-sequence-inverse-path/manifest.ttl     |   24 +
 .../shacl/complex-sequence-inverse-path/shapes.ttl |   12 +
 .../manifest.ttl                                   |   25 +
 .../shapes.ttl                                     |   12 +
 .../manifest.ttl                                   |   31 +
 .../empty-left-optional-blank-violating/shapes.ttl |   12 +
 .../manifest.ttl                                   |   25 +
 .../empty-left-optional-iri-conforming/shapes.ttl  |   12 +
 .../empty-left-optional-iri-violating/manifest.ttl |   31 +
 .../empty-left-optional-iri-violating/shapes.ttl   |   12 +
 .../exists-absence-blank-conforming/manifest.ttl   |   25 +
 .../exists-absence-blank-conforming/shapes.ttl     |   12 +
 .../exists-absence-blank-violating/manifest.ttl    |   31 +
 .../exists-absence-blank-violating/shapes.ttl      |   12 +
 .../exists-absence-iri-conforming/manifest.ttl     |   25 +
 .../shacl/exists-absence-iri-conforming/shapes.ttl |   12 +
 .../exists-absence-iri-violating/manifest.ttl      |   31 +
 .../shacl/exists-absence-iri-violating/shapes.ttl  |   12 +
 corpora/community/shacl/inventory.json             | 3128 ++++++++++++++++++++
 corpora/community/shacl/manifest.ttl               |   11 +
 .../manifest.ttl                                   |   25 +
 .../nested-not-exists-blank-conforming/shapes.ttl  |   12 +
 .../nested-not-exists-blank-violating/manifest.ttl |   31 +
 .../nested-not-exists-blank-violating/shapes.ttl   |   12 +
 .../nested-not-exists-iri-conforming/manifest.ttl  |   25 +
 .../nested-not-exists-iri-conforming/shapes.ttl    |   12 +
 .../nested-not-exists-iri-violating/manifest.ttl   |   31 +
 .../nested-not-exists-iri-violating/shapes.ttl     |   12 +
 .../nested-optional-blank-conforming/manifest.ttl  |   25 +
 .../nested-optional-blank-conforming/shapes.ttl    |   12 +
 .../nested-optional-blank-violating/manifest.ttl   |   31 +
 .../nested-optional-blank-violating/shapes.ttl     |   12 +
 .../nested-optional-iri-conforming/manifest.ttl    |   25 +
 .../nested-optional-iri-conforming/shapes.ttl      |   12 +
 .../nested-optional-iri-violating/manifest.ttl     |   31 +
 .../shacl/nested-optional-iri-violating/shapes.ttl |   12 +
 .../shacl/not-exists-blank-conforming/manifest.ttl |   25 +
 .../shacl/not-exists-blank-conforming/shapes.ttl   |   12 +
 .../shacl/not-exists-blank-violating/manifest.ttl  |   31 +
 .../shacl/not-exists-blank-violating/shapes.ttl    |   12 +
 .../shacl/not-exists-iri-conforming/manifest.ttl   |   25 +
 .../shacl/not-exists-iri-conforming/shapes.ttl     |   12 +
 .../shacl/not-exists-iri-violating/manifest.ttl    |   31 +
 .../shacl/not-exists-iri-violating/shapes.ttl      |   12 +
 .../path-absence-blank-conforming/manifest.ttl     |   25 +
 .../shacl/path-absence-blank-conforming/shapes.ttl |   12 +
 .../path-absence-blank-violating/manifest.ttl      |   31 +
 .../shacl/path-absence-blank-violating/shapes.ttl  |   12 +
 .../shacl/path-absence-iri-conforming/manifest.ttl |   25 +
 .../shacl/path-absence-iri-conforming/shapes.ttl   |   12 +
 .../shacl/path-absence-iri-violating/manifest.ttl  |   31 +
 .../shacl/path-absence-iri-violating/shapes.ttl    |   12 +
 .../shacl/predicate-focus-blank/manifest.ttl       |   22 +
 .../shacl/predicate-focus-blank/shapes.ttl         |   12 +
 .../shacl/predicate-focus-iri/manifest.ttl         |   21 +
 .../community/shacl/predicate-focus-iri/shapes.ttl |   12 +
 .../manifest.ttl                                   |   25 +
 .../projected-subquery-blank-conforming/shapes.ttl |   12 +
 .../manifest.ttl                                   |   31 +
 .../projected-subquery-blank-violating/shapes.ttl  |   12 +
 .../projected-subquery-iri-conforming/manifest.ttl |   25 +
 .../projected-subquery-iri-conforming/shapes.ttl   |   12 +
 .../projected-subquery-iri-violating/manifest.ttl  |   31 +
 .../projected-subquery-iri-violating/shapes.ttl    |   12 +
 .../shacl/quoted-focus-optional/manifest.ttl       |   22 +
 .../shacl/quoted-focus-optional/shapes.ttl         |   12 +
 corpora/community/shacl/rule-not-exists/data.ttl   |   17 +
 .../community/shacl/rule-not-exists/expected.ttl   |   12 +
 .../community/shacl/rule-not-exists/manifest.ttl   |   12 +
 corpora/community/shacl/rule-not-exists/shapes.ttl |   12 +
 corpora/community/shacl/rule-optional/data.ttl     |   17 +
 corpora/community/shacl/rule-optional/expected.ttl |   12 +
 corpora/community/shacl/rule-optional/manifest.ttl |   12 +
 corpora/community/shacl/rule-optional/shapes.ttl   |   12 +
 .../manifest.ttl                                   |   25 +
 .../shapes.ttl                                     |   14 +
 .../manifest.ttl                                   |   29 +
 .../shapes.ttl                                     |   14 +
 .../shacl/solution-declared-failure/manifest.ttl   |   14 +
 .../shacl/solution-declared-failure/shapes.ttl     |   12 +
 .../shacl/solution-message-priority/manifest.ttl   |   21 +
 .../shacl/solution-message-priority/shapes.ttl     |   12 +
 .../union-absence-blank-conforming/manifest.ttl    |   25 +
 .../union-absence-blank-conforming/shapes.ttl      |   12 +
 .../union-absence-blank-violating/manifest.ttl     |   31 +
 .../shacl/union-absence-blank-violating/shapes.ttl |   12 +
 .../union-absence-iri-conforming/manifest.ttl      |   25 +
 .../shacl/union-absence-iri-conforming/shapes.ttl  |   12 +
 .../shacl/union-absence-iri-violating/manifest.ttl |   31 +
 .../shacl/union-absence-iri-violating/shapes.ttl   |   12 +
 crates/conformance-kit/Cargo.toml                  |   32 +
 crates/conformance-kit/src/bag.rs                  |  118 +
 crates/conformance-kit/src/graph.rs                |  108 +
 crates/conformance-kit/src/identity.rs             |   84 +
 crates/conformance-kit/src/inventory.rs            |  397 +++
 crates/conformance-kit/src/lib.rs                  |   48 +
 crates/conformance-kit/src/manifest.rs             |   97 +
 crates/conformance-kit/src/outcome.rs              |  192 ++
 crates/conformance-kit/src/report.rs               |  375 +++
 crates/conformance-kit/src/report/projection.rs    |  549 ++++
 crates/conformance-kit/src/result_set.rs           |  108 +
 crates/conformance-kit/src/reviews.rs              |  270 ++
 crates/conformance-kit/tests/grading.rs            |  924 ++++++
 crates/iri/src/vocab.rs                            |  151 +
 crates/rdf-wasm/src/async_query.rs                 |    4 +-
 crates/rdf-wasm/src/interleaving.rs                |   14 +-
 crates/shapes/README.md                            |   72 +
 crates/shapes/benches/shared_views.rs              |   81 +-
 crates/shapes/src/components.rs                    |  146 +-
 crates/shapes/src/constraints.rs                   |  392 ++-
 crates/shapes/src/data.rs                          |   14 +
 crates/shapes/src/engine.rs                        |  979 +++++-
 crates/shapes/src/engine/complete_reports.rs       |  456 +++
 crates/shapes/src/expression.rs                    |   24 +
 crates/shapes/src/extension_usage.rs               |   36 +-
 crates/shapes/src/lib.rs                           |    6 +
 crates/shapes/src/prebinding.rs                    |  328 +-
 crates/shapes/src/product/certified.rs             |   21 +-
 crates/shapes/src/product/mod.rs                   |  137 +-
 crates/shapes/src/profile.rs                       |  516 ++++
 crates/shapes/src/query_law.rs                     |  303 ++
 crates/shapes/src/report.rs                        | 1235 +++++++-
 crates/shapes/src/rules.rs                         |  108 +-
 crates/shapes/src/shacl_corpora/mod.rs             |   22 +-
 crates/shapes/src/shacl_corpora/shacl12.rs         |   11 +-
 crates/shapes/src/shapes.rs                        |  510 +++-
 crates/shapes/src/shapes/parser/admission.rs       | 1162 ++++++++
 crates/shapes/src/shapes/parser/functions.rs       |   36 +-
 crates/shapes/src/shapes/parser/mod.rs             |    1 +
 crates/shapes/src/shapes/parser/node_expr.rs       |  166 +-
 crates/shapes/src/shapes/parser/rule_parse.rs      |   68 +-
 crates/shapes/src/shapes/parser/target_types.rs    |   18 +-
 crates/shapes/src/sparql.rs                        |  311 +-
 crates/shapes/src/xpath.rs                         |   68 +-
 crates/shapes/tests/complete_reports.rs            |  777 +++++
 crates/shapes/tests/dated_execution.rs             |  660 +++++
 crates/shapes/tests/product_model_census.rs        |   98 +-
 crates/shapes/tests/profile_admission.rs           |  427 +++
 crates/shapes/tests/xpath_bundle.rs                |  103 +
 crates/sparql-conformance/Cargo.toml               |   16 +-
 .../src/bin/community-conformance.rs               |   68 +
 crates/sparql-conformance/src/community.rs         | 1129 +++++++
 crates/sparql-conformance/src/lib.rs               |    1 +
 crates/sparql-conformance/src/manifest.rs          |    4 +-
 crates/sparql-conformance/src/rs_resultset.rs      |    2 +-
 .../tests/community_conformance.rs                 |  302 ++
 crates/sparql-eval/src/engine.rs                   |  202 +-
 crates/sparql-eval/src/engine/bounded_workspace.rs |    1 +
 crates/sparql-eval/src/engine/graph_build.rs       |   26 +-
 crates/sparql-eval/src/engine/prepared_fallible.rs |   15 +-
 crates/sparql-eval/src/error.rs                    |    7 +
 crates/sparql-eval/src/eval.rs                     |   17 +
 crates/sparql-eval/src/execution.rs                |   11 +
 crates/sparql-eval/src/lib.rs                      |    2 +-
 crates/sparql-eval/src/protocol.rs                 |    1 +
 crates/sparql-eval/src/user_fn.rs                  |  774 +++++
 crates/sparql-eval/tests/prepared_parameters.rs    |   99 +
 crates/validate/README.md                          |   29 +
 crates/validate/src/complete.rs                    |  273 ++
 crates/validate/src/lib.rs                         |   10 +-
 crates/validate/src/product.rs                     |   50 +-
 crates/validate/src/shacl.rs                       |   15 +-
 crates/validate/tests/complete_shacl.rs            |  581 ++++
 docs/BENCHMARKS.md                                 |   21 +-
 docs/CONFORMANCE.md                                |   14 +
 docs/book/po/zh-Hans.po                            |   82 +-
 docs/book/src/project/releases.md                  |    8 +-
 docs/design/purrdf-prepared-products.md            |   11 +
 docs/design/purrdf-simd.md                         |    2 +
 helpers-ledger.toml                                |   10 +
 layers.toml                                        |    8 +
 scripts/conformance-baseline.json                  |    4 +
 scripts/conformance-matrix.py                      |  128 +-
 232 files changed, 24014 insertions(+), 773 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

