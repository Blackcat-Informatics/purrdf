# Branch under review — issue #563

Branch `paudley/563-segmented-scans-fetch-their-block` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `0735ac6d4` Retain segmented scan blocks across rows without pinning cache slots  _2026-10-16_

## Files changed

```
crates/rdf-core/src/ir/segmented/session.rs        |  58 +++----
 crates/rdf-core/src/ir/segmented/session/cache.rs  |   7 +-
 .../src/ir/segmented/session/cache_control.rs      | 167 +++++++++++++++++++++
 helpers-ledger.toml                                |  80 +++++-----
 4 files changed, 243 insertions(+), 69 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

