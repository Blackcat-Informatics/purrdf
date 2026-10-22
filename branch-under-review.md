# Branch under review — issue #493

Branch `paudley/493-datalog-incremental-maintenance-of-a` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (10)

- `55e3b0527` fix(datalog): invalidate prepared seals when removing rows  _2026-10-22_
- `578bb65da` feat(datalog): govern native maintenance initialization and sealing  _2026-10-22_
- `4e942a3fe` Govern maintenance allocation work before native growth  _2026-10-22_
- `4349803d9` Admit native batch sealing before row publication  _2026-10-22_
- `bf6fab38b` Admit maintained partition arrays and mutable tail columns  _2026-10-22_
- `36063bee0` Admit native maintenance indexes through their original owner  _2026-10-22_
- `d2e19593a` Admit maintenance transaction frames before native mutation  _2026-10-22_
- `5d6542565` Admit retained native maintenance proof storage  _2026-10-22_
- `b5075ebd7` Retain native model maintenance across signed transactions  _2026-10-22_
- `de6098321` Admit native signed Datalog change batches with host watermarks  _2026-10-21_

## Files changed

```
crates/datalog/Cargo.toml                          |    4 +
 crates/datalog/README.md                           |   17 +
 crates/datalog/src/allocation.rs                   |  131 ++
 crates/datalog/src/clause.rs                       |   40 +-
 crates/datalog/src/dictionary.rs                   |   23 +
 crates/datalog/src/dictionary/integer.rs           |   20 +
 crates/datalog/src/id.rs                           |    2 +-
 crates/datalog/src/lib.rs                          |    2 +
 crates/datalog/src/maintenance.rs                  |  406 ++++
 crates/datalog/src/plan.rs                         |  293 ++-
 crates/datalog/src/proof.rs                        |  179 +-
 crates/datalog/src/schedule.rs                     |   29 +-
 crates/datalog/src/seminaive.rs                    |  329 ++-
 crates/datalog/src/seminaive/atom.rs               |    6 +-
 crates/datalog/src/seminaive/commit.rs             |  372 +++-
 crates/datalog/src/seminaive/cycle.rs              |    2 +-
 crates/datalog/src/seminaive/evidence.rs           |    2 +-
 crates/datalog/src/seminaive/factors.rs            |   92 +-
 crates/datalog/src/seminaive/factors/execution.rs  |   15 +-
 crates/datalog/src/seminaive/factors/filters.rs    |   10 +-
 crates/datalog/src/seminaive/factors/matches.rs    |   36 +-
 crates/datalog/src/seminaive/guards.rs             |   35 +-
 crates/datalog/src/seminaive/join.rs               |  106 +-
 crates/datalog/src/seminaive/maintained.rs         | 2142 ++++++++++++++++++
 crates/datalog/src/seminaive/maintained/event.rs   |  284 +++
 crates/datalog/src/seminaive/maintained/initial.rs |  294 +++
 crates/datalog/src/seminaive/maintained/prepare.rs |  585 +++++
 crates/datalog/src/seminaive/maintained/reclaim.rs |  156 ++
 crates/datalog/src/seminaive/maintained/storage.rs | 1375 ++++++++++++
 crates/datalog/src/seminaive/maintained/work.rs    |  302 +++
 crates/datalog/src/seminaive/negative.rs           |   12 +-
 crates/datalog/src/seminaive/ordered.rs            |   35 +-
 crates/datalog/src/seminaive/probe.rs              |  565 +++++
 crates/datalog/src/seminaive/rule.rs               |   10 +
 crates/datalog/src/seminaive/session.rs            |  100 +-
 crates/datalog/src/seminaive/stratum.rs            |   33 +-
 crates/datalog/src/store.rs                        | 1688 ++++++++++----
 crates/datalog/src/store/columns.rs                |   71 +-
 crates/datalog/src/store/directory.rs              |  645 ++++++
 crates/datalog/src/store/index.rs                  |   21 +-
 crates/datalog/src/store/maintenance_indexes.rs    |  352 +++
 crates/datalog/src/store/metadata.rs               |  300 +++
 crates/datalog/src/store/rows.rs                   |  873 ++++++++
 crates/datalog/src/store/seal.rs                   |  465 ++++
 crates/datalog/src/store/terms.rs                  |  447 ++++
 crates/datalog/src/test_support/host_dictionary.rs |   11 +-
 crates/datalog/tests/maintained_scale.rs           |  327 +++
 crates/datalog/tests/rules_runtime.rs              | 2309 +++++++++++++++++++-
 crates/entail/src/engine.rs                        |  514 +++--
 crates/entail/src/entails/freeze.rs                |    6 +-
 crates/entail/src/entails/mod.rs                   |    1 +
 crates/entail/src/entails/refutation.rs            |    8 +-
 crates/entail/src/lib.rs                           |   11 +
 crates/entail/tests/current_clash_support.rs       |   47 +
 crates/hash/README.md                              |    4 +
 crates/lex/src/allocation.rs                       |    2 +-
 crates/lex/src/allocation/ordered.rs               |  517 ++++-
 crates/lex/src/walk.rs                             |    4 +-
 crates/lex/src/walk/sort.rs                        |  198 +-
 crates/shapes/src/srl/eval.rs                      |    8 +-
 docs/MIGRATION-4.0.md                              |   41 +
 docs/design/purrdf-simd.md                         |    2 +-
 helpers-ledger.toml                                |   95 +-
 63 files changed, 15943 insertions(+), 1068 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

