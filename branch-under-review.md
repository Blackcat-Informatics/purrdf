# Branch under review — issue #562

Branch `paudley/562-purrdf-hash-blake3-keyed-hash-derive` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `f3c5b1677` Add BLAKE3 keyed modes and seekable extended output  _2026-10-16_

## Files changed

```
CHANGELOG.md                                       |   6 +
 Makefile                                           |   6 +-
 crates/hash-conformance/tests/blake3.rs            | 175 +++++++++
 .../vectors/blake3_official_test_vectors.json      | 217 +++++++++++
 .../tests/vectors/blake3_official_vectors.txt      |  41 ++
 crates/hash/PROVENANCE.md                          |  15 +-
 crates/hash/README.md                              |  36 +-
 crates/hash/src/arch/blake3_lanes.rs               |  54 ++-
 crates/hash/src/arch/blake3_parents2.rs            |  29 +-
 crates/hash/src/arch/blake3_rows.rs                |  20 +-
 crates/hash/src/arch/blake3_x86.rs                 | 149 +++++---
 crates/hash/src/arch/mod.rs                        |  90 +++--
 crates/hash/src/blake3.rs                          | 424 +++++++++++++++++----
 crates/hash/src/lib.rs                             |   2 +-
 helpers-ledger.toml                                |   7 +-
 15 files changed, 1069 insertions(+), 202 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

