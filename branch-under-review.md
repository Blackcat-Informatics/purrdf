# Branch under review — issue #475

Branch `paudley/475-benchmarks-replace-the-java-lubm` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (4)

- `a82d2426e` Apply an explicit entailment join budget throughout the LUBM lane  _2026-10-08_
- `d526fc530` Supply the native graph acceptance certificate description  _2026-10-08_
- `b994b61d0` bench: run and verify LUBM through the native corpus lane  _2026-10-07_
- `5f7c8b6fb` Add deterministic native university corpus generator  _2026-10-07_

## Files changed

```
AGENTS.md                               |   6 +-
 Cargo.lock                              |   1 +
 Makefile                                |  16 +-
 README.md                               |   2 +-
 README_zh.md                            |   2 +-
 crates/bench/Cargo.toml                 |  23 +-
 crates/bench/LUBM_PROFILE.md            |  12 +
 crates/bench/README.md                  |  21 +-
 crates/bench/examples/lubm_fault_cli.rs | 115 +++++
 crates/bench/src/lib.rs                 |   3 +
 crates/bench/src/lubm/check.rs          | 489 +++++++++++++++++++++
 crates/bench/src/lubm/generate.rs       | 282 ++++++++++++
 crates/bench/src/lubm/mod.rs            | 177 ++++++++
 crates/bench/src/lubm/output.rs         | 157 +++++++
 crates/bench/src/lubm_check_main.rs     | 125 ++++++
 crates/bench/src/lubm_main.rs           |  66 +++
 crates/bench/tests/lane_common_laws.rs  |  11 +-
 crates/bench/tests/lubm_check_cli.rs    | 398 +++++++++++++++++
 crates/bench/tests/lubm_cli.rs          | 676 +++++++++++++++++++++++++++++
 crates/bench/tests/make_bench_lanes.rs  | 138 +++---
 docs/BENCHMARKS.md                      | 163 ++-----
 docs/design/purrdf-bench-lane-laws.md   |  59 +--
 layers.toml                             |   2 +-
 scripts/benchmark-acquire.py            | 134 +-----
 scripts/lane-common.sh                  |  12 +-
 scripts/lubm-lane.sh                    | 738 +++++++++-----------------------
 scripts/lubm-queries.py                 |   7 +-
 27 files changed, 2901 insertions(+), 934 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

