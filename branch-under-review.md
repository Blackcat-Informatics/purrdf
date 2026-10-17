# Branch under review — issue #566

Branch `paudley/566-unify-native-graph-projection-across` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `03902c2f5` feat(core): project default and named RDF graphs through one operation  _2026-10-16_

## Files changed

```
crates/rdf-core/src/ir/canon.rs                 |  21 +++-
 crates/rdf-core/src/ir/dataset.rs               | 152 ++++++++++++++++++++----
 crates/rdf-core/src/ir/pipeline_bundle.rs       |   6 +-
 crates/rdf-core/tests/shared_views.rs           |  14 ++-
 crates/rdf/tests/content_id_byte_determinism.rs |   4 +-
 crates/shapes/benches/shared_views.rs           |   4 +-
 crates/shapes/tests/named_graph_view.rs         |  16 ++-
 7 files changed, 180 insertions(+), 37 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

