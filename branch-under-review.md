# Branch under review — issue #596

Branch `paudley/596-owl-dl-expose-checked-source-bound` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (26)

- `86d651ace` test: align proof consumers and measured assembly projection  _2026-10-18_
- `1f6617be2` docs(entail): state the merge proof-path requirement  _2026-10-18_
- `00dc4bae2` test(python): include the recursive nominal proof vector  _2026-10-18_
- `62b0e4f11` test(entail): qualify public causal proof consumers  _2026-10-18_
- `0a6f8eea5` fix(entail): saturate and verify nominal introduction  _2026-10-18_
- `6825a7248` feat(entail): check reachable native merge closures  _2026-10-18_
- `13a4a6885` feat(entail): retain native selected counting dependencies  _2026-10-18_
- `7669ba33b` feat(entail): expose question-bound checked causal proof views  _2026-10-18_
- `94698a9a0` fix(entail): retain causal edges and universal role licenses  _2026-10-18_
- `719883987` fix(entail): retain selected native effects and generated births  _2026-10-18_
- `87fd296ab` fix(entail): link key equalities to checked membership support  _2026-10-18_
- `e0f5e5160` Retain checked key membership proofs in their original source phase  _2026-10-17_
- `66e664a5f` Retain role constraints and indexed native key value evidence  _2026-10-17_
- `60799582b` Retain native role closure and generated clause derivations  _2026-10-17_
- `2ee4fed4c` Retain original role inputs through native compiler transitions  _2026-10-17_
- `97147753f` Retain checked schema dependencies in native causal replay  _2026-10-17_
- `9c695a88f` Retain original native syntax reads for checked causal support  _2026-10-17_
- `85f99a088` Bind native replay inputs to their original source owners  _2026-10-17_
- `651887268` Retain native transitive cache witnesses through updates and merges  _2026-10-17_
- `afd966203` Retain accepted native role-language paths during replay  _2026-10-17_
- `19cb3ef2e` Trace selected replay reads through immutable fact dependencies  _2026-10-17_
- `4575d4cac` Record native replay mutations with their original owners  _2026-10-17_
- `bb867363b` Check recorded DL bindings without enumerating alternative frames  _2026-10-17_
- `2cd5574fe` Preserve original DL inclusion sources through normalization  _2026-10-17_
- `af1b1e9ab` Retain original source anchors for normalized DL inclusions  _2026-10-17_
- `d00d9bd8c` Expose borrowed views of checked DL clash evidence  _2026-10-17_

## Files changed

```
Makefile                                           |    7 +
 bindings/python/tests/test_entail_reasoning.py     |    2 +-
 crates/entail/README.md                            |    6 +-
 crates/entail/src/lib.rs                           |    1 +
 crates/entail/src/owl_dl/absorb.rs                 |   89 +-
 crates/entail/src/owl_dl/axiom_source.rs           |   33 +
 crates/entail/src/owl_dl/axiom_source/tests.rs     |  150 ++
 crates/entail/src/owl_dl/bounds.rs                 |   23 +-
 crates/entail/src/owl_dl/clause.rs                 |  108 +-
 crates/entail/src/owl_dl/concept.rs                |    8 +
 crates/entail/src/owl_dl/data.rs                   |    5 +
 crates/entail/src/owl_dl/graph.rs                  |  847 +++++++++-
 crates/entail/src/owl_dl/graph/changes.rs          |  498 ++++++
 crates/entail/src/owl_dl/graph/changes/reach.rs    |  109 ++
 .../entail/src/owl_dl/graph/changes/reach/tests.rs |  235 +++
 crates/entail/src/owl_dl/graph/changes/slice.rs    | 1207 +++++++++++++
 .../entail/src/owl_dl/graph/changes/slice/key.rs   |  139 ++
 .../src/owl_dl/graph/changes/slice/schema.rs       |   95 ++
 .../entail/src/owl_dl/graph/changes/slice/tests.rs |  339 ++++
 crates/entail/src/owl_dl/hyper.rs                  | 1781 +++++++++++++++++++-
 crates/entail/src/owl_dl/input_source.rs           |  268 +++
 crates/entail/src/owl_dl/input_source/tests.rs     |  255 +++
 crates/entail/src/owl_dl/key_proofs.rs             |  211 +++
 crates/entail/src/owl_dl/key_proofs/tests.rs       |  148 ++
 crates/entail/src/owl_dl/key_values.rs             |  169 ++
 crates/entail/src/owl_dl/key_values/tests.rs       |  163 ++
 crates/entail/src/owl_dl/mod.rs                    |  176 +-
 crates/entail/src/owl_dl/parser.rs                 |  320 +++-
 crates/entail/src/owl_dl/proof.rs                  |  581 ++++++-
 crates/entail/src/owl_dl/proof/capture.rs          |  104 ++
 crates/entail/src/owl_dl/proof/source.rs           |  432 +++++
 crates/entail/src/owl_dl/proof/source/causal.rs    | 1345 +++++++++++++++
 .../src/owl_dl/proof/source/causal/branch.rs       |  167 ++
 .../src/owl_dl/proof/source/causal/schema.rs       |   61 +
 .../src/owl_dl/proof/source/causal/syntax.rs       |  211 +++
 .../entail/src/owl_dl/proof/source/causal/tests.rs | 1095 ++++++++++++
 .../src/owl_dl/proof/source/normalization_tests.rs |  172 ++
 crates/entail/src/owl_dl/proof/source/origin.rs    |   59 +
 crates/entail/src/owl_dl/proof/source/schema.rs    |  166 ++
 crates/entail/src/owl_dl/proof/source/tests.rs     | 1666 ++++++++++++++++++
 crates/entail/src/owl_dl/query.rs                  |   20 +-
 crates/entail/src/owl_dl/roles.rs                  |  410 ++++-
 crates/entail/src/owl_dl/roles/licence.rs          |  244 +++
 crates/entail/src/owl_dl/roles/licence/tests.rs    |   69 +
 crates/entail/src/owl_dl/roles/order.rs            |  154 ++
 crates/entail/src/owl_dl/roles/source.rs           |   96 ++
 crates/entail/src/owl_dl/roles/source/tests.rs     |   70 +
 crates/entail/src/owl_dl/syntax.rs                 |  377 +++++
 crates/entail/src/owl_dl/syntax/tests.rs           |  245 +++
 crates/entail/src/owl_dl/tableau.rs                |    2 +-
 crates/entail/src/reasoner/mod.rs                  |    2 +-
 crates/entail/src/reasoner/proof.rs                |   59 +-
 crates/entail/src/reasoner/proof/causal.rs         |  176 ++
 crates/entail/tests/current_clash_support.rs       |  619 +++++++
 crates/hash/README.md                              |    2 +
 crates/rdf-capi/src/entail.rs                      |   11 +-
 crates/validate/src/regime.rs                      |   58 +
 crates/validate/tests/dl_proof_vectors.rs          |    4 +-
 crates/validate/tests/fixtures/dl-proof.vectors    |  140 ++
 docs/WASM_TESTING.md                               |    8 +-
 docs/design/purrdf-simd.md                         |    2 +-
 61 files changed, 15776 insertions(+), 443 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

