# Branch under review — issue #538

Branch `paudley/538-purrdf-text-update-a-textindex-by-delta` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `5258479c1` feat(text): update indexes with source-bound literal deltas  _2026-10-17_

## Files changed

```
crates/rdf-wasm/src/interleaving.rs   |   6 +
 crates/text/README.md                 |  17 +
 crates/text/src/analysis.rs           |   8 +
 crates/text/src/error.rs              | 108 +++++++
 crates/text/src/index.rs              | 153 +++++----
 crates/text/src/index/delta.rs        | 580 ++++++++++++++++++++++++++++++++++
 crates/text/src/lib.rs                |   2 +-
 crates/text/src/surface.rs            | 346 ++++++++++++++++++--
 crates/text/src/surface/delta.rs      | 274 ++++++++++++++++
 crates/text/src/surface/postings.rs   | 170 ++++++++++
 crates/text/tests/determinism.rs      |   2 +-
 crates/text/tests/wasm_determinism.rs |  93 +++++-
 12 files changed, 1669 insertions(+), 90 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

