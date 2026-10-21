# Branch under review — issue #488

Branch `delivery/488-datalog-round-state` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (2)

- `f0c296384` Retain complete Datalog rule phases across budget pauses  _2026-10-20_
- `610f3f425` Prepare immutable Datalog rule metadata with compiled programs  _2026-10-20_

## Files changed

```
crates/datalog/src/cache.rs                       |  20 +-
 crates/datalog/src/plan.rs                        |  37 ++-
 crates/datalog/src/seminaive.rs                   |  37 ++-
 crates/datalog/src/seminaive/factors.rs           |  15 +-
 crates/datalog/src/seminaive/factors/execution.rs |  10 +-
 crates/datalog/src/seminaive/factors/filters.rs   |  30 +-
 crates/datalog/src/seminaive/factors/matches.rs   |   1 +
 crates/datalog/src/seminaive/ordered.rs           | 327 ++++++++++++++++++++++
 crates/datalog/src/seminaive/rule.rs              | 297 ++++++++++++++++++++
 9 files changed, 730 insertions(+), 44 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

