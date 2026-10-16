# Branch under review — issue #484

Branch `paudley/484-planner-statistics-per-predicate-per` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (11)

- `279a26a0e` Version snapshot-aware planning charges and document source format costs  _2026-10-15_
- `1a2e326a7` Expose sound statistics for chronological stack queries  _2026-10-15_
- `839018278` Persist certified source statistics for segmented queries  _2026-10-15_
- `7c5a41cc2` Estimate structural RDF patterns across every statement stream  _2026-10-15_
- `5bcff126e` Use snapshot statistics to break connected rule-order ties  _2026-10-15_
- `2bb0691b2` Seal paged predicate statistics in the shared dictionary  _2026-10-15_
- `076aac6d3` Demonstrate reifier join estimates on one million claims  _2026-10-15_
- `0f5137046` Record snapshot statistics and exactness in query explanations  _2026-10-15_
- `de36c4840` Use predicate distinct domains in join planning and forecasts  _2026-10-15_
- `527f70f64` Seal native predicate statistics through original admission  _2026-10-15_
- `a84f28cdd` Expose snapshot statistics to actual SPARQL planning  _2026-10-15_

## Files changed

```
crates/datalog/src/plan.rs                         |  157 ++-
 crates/rdf-core/src/dataset_view.rs                |   15 +
 crates/rdf-core/src/ir/builder.rs                  |   62 +-
 crates/rdf-core/src/ir/builder/triple.rs           |   27 +
 crates/rdf-core/src/ir/dataset.rs                  |   29 +-
 crates/rdf-core/src/ir/mod.rs                      |    1 +
 crates/rdf-core/src/ir/pack/bits.rs                |   56 +-
 crates/rdf-core/src/ir/paged/mod.rs                |   97 +-
 crates/rdf-core/src/ir/paged/query.rs              |    4 +
 crates/rdf-core/src/ir/paged/stack.rs              |   41 +-
 crates/rdf-core/src/ir/paged/summary.rs            |   84 +-
 crates/rdf-core/src/ir/paged/translation.rs        |   25 +-
 crates/rdf-core/src/ir/segmented/build.rs          |  239 +++-
 crates/rdf-core/src/ir/segmented/format.rs         |   48 +-
 crates/rdf-core/src/ir/segmented/mod.rs            |    6 +-
 crates/rdf-core/src/ir/segmented/query.rs          |    1 +
 .../rdf-core/src/ir/segmented/query/statistics.rs  |   88 ++
 crates/rdf-core/src/ir/segmented/query/view.rs     |    8 +-
 crates/rdf-core/src/ir/segmented/session.rs        |   22 +-
 .../src/ir/segmented/session/statistics.rs         |  362 +++++
 crates/rdf-core/src/ir/segmented/stack.rs          |   31 +
 crates/rdf-core/src/ir/statistics.rs               | 1426 ++++++++++++++++++++
 crates/rdf-core/src/ir/statistics/codec.rs         |  368 +++++
 crates/rdf-core/src/ir/triple.rs                   |    1 +
 crates/rdf-core/src/ir/triple/read.rs              |  110 ++
 crates/rdf-core/src/lib.rs                         |    5 +
 crates/rdf-core/src/statistics.rs                  |  233 ++++
 crates/rdf-core/src/statistics/layered.rs          |  221 +++
 crates/rdf-core/tests/ir_zero_alloc.rs             |   90 ++
 crates/rdf-core/tests/paged_stack.rs               |   51 +-
 crates/rdf-core/tests/segmented_stack.rs           |   32 +
 crates/rdf-core/tests/support/statistics.rs        |   77 ++
 crates/sparql-eval/src/bgp.rs                      |  857 ++++++++++--
 crates/sparql-eval/src/bgp/positive.rs             |   10 +-
 crates/sparql-eval/src/engine.rs                   |    2 +-
 crates/sparql-eval/src/governor/ledger.rs          |   35 +-
 crates/sparql-eval/src/governor/mod.rs             |   13 +-
 crates/sparql-eval/tests/governed_query.rs         |  201 +++
 crates/sparql-eval/tests/support/mod.rs            |   33 +-
 docs/SPARQL-GOVERNOR-PROFILE.md                    |   11 +-
 docs/design/purrdf-backend-contract.md             |   55 +
 docs/design/purrdf-segmented-query-storage.md      |   30 +
 helpers-ledger.toml                                |  268 ++--
 43 files changed, 5188 insertions(+), 344 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

