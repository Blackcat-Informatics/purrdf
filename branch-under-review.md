# Branch under review — issue #591

Branch `paudley/591-proof-replay-imported-inconsistent` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `c86272091` Fix imported proof checking against resolved source snapshots  _2026-10-17_

## Files changed

```
Makefile                                  |   1 +
 crates/cli/src/consistency.rs             |  31 ++++-----
 crates/validate/src/regime.rs             |  64 ++++++++++++++++--
 crates/validate/tests/dl_proof_vectors.rs | 109 +++++++++++++++++++++++++++++-
 docs/WASM_TESTING.md                      |   5 +-
 5 files changed, 183 insertions(+), 27 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

