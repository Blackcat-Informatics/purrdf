# Branch under review — issue #542

Branch `paudley/542-rdf-core-mutabledataset-freeze-uses` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (6)

- `20ede055f` docs: align the handwritten Python conformance count  _2026-10-13_
- `ba624d211` Refresh conformance scoreboard from hosted shard measurements  _2026-10-13_
- `8360fff43` Fix retrieval rustdoc links and cross-architecture HNSW fixtures  _2026-10-13_
- `2916fbce4` Align shared fixtures and documentation with admitted behavior  _2026-10-13_
- `5170165b8` Correct the deliberately faulty x87 precision fixture  _2026-10-13_
- `3d9571582` Allow caller-selected limits for mutable compaction  _2026-10-13_

## Files changed

```
bindings/python/tests/test_retrieval.py        |   2 +-
 crates/geo-kernel/src/measure.rs               |   2 +-
 crates/hnsw/tests/float_environment.rs         |  38 +---
 crates/hnsw/tests/float_environment_rayon.rs   |  29 +--
 crates/hnsw/tests/support/fixture.rs           |  20 +-
 crates/purrdf/tests/base_surface.rs            |  21 +-
 crates/rdf-core/src/distance/binary64_tests.rs |  25 ++-
 crates/rdf-core/src/distance/tests.rs          |   7 +-
 crates/rdf-core/src/ir/mutable.rs              |  31 ++-
 crates/rdf-core/src/query_scope/counters.rs    | 276 ++++++++++++-------------
 crates/rdf-core/tests/shared_views.rs          | 190 ++++++++++++++++-
 crates/retrieval/src/search.rs                 |  12 +-
 crates/sparql-eval/src/engine.rs               |  30 +--
 crates/sparql-eval/src/geo/relation.rs         |   2 +-
 crates/sparql-eval/src/property_fn_eval.rs     |   2 +-
 docs/CONFORMANCE.md                            |   4 +-
 docs/design/purrdf-simd.md                     |  16 +-
 17 files changed, 447 insertions(+), 260 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

