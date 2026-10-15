# Branch under review — issue #551

Branch `paudley/551-segmented-stack-queries-are-quadratic` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (3)

- `47725646e` Stop bounded statement scans and index segmented source probes  _2026-10-14_
- `b5db98eb7` Admit segmented request and cache buffers as they grow  _2026-10-14_
- `dc9741fcc` Use admitted amortized growth for segmented term representatives  _2026-10-14_

## Files changed

```
crates/rdf-core/examples/segmented_fixture.rs  |  45 +-----
 crates/rdf-core/src/ir/mod.rs                  |   2 +
 crates/rdf-core/src/ir/segmented/file.rs       |  75 ++++++++++
 crates/rdf-core/src/ir/segmented/mod.rs        |   7 +-
 crates/rdf-core/src/ir/segmented/query.rs      |  53 ++++++-
 crates/rdf-core/src/ir/segmented/query/view.rs | 189 ++++++++++++++++++-------
 crates/rdf-core/src/ir/segmented/session.rs    |  62 +++++---
 crates/rdf-core/src/lib.rs                     |   2 +
 crates/rdf-core/tests/segmented_stack.rs       |  54 +++++++
 crates/sparql-eval/src/bgp.rs                  |  11 +-
 crates/sparql-eval/src/bgp/window.rs           |   6 +-
 crates/sparql-eval/src/path_relation.rs        |  13 +-
 crates/sparql-eval/src/statement_layer.rs      |  22 ++-
 crates/sparql-eval/tests/segmented_query.rs    | 178 +++++++++++++++++++++++
 docs/design/purrdf-segmented-query-storage.md  |  56 ++++++++
 helpers-ledger.toml                            |  88 ++++++------
 16 files changed, 686 insertions(+), 177 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

