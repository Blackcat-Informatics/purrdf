# Branch under review — issue #597

Branch `paudley/597-share-owned-or-borrowed-storage-to` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `bec31ed26` Share inline immutable owned-or-borrowed storage  _2026-10-17_

## Files changed

```
crates/entail/src/owl_dl/proof.rs              | 31 ++-------
 crates/entail/src/owl_dl/proof/schema_tests.rs |  2 +-
 crates/rdf-core/src/lib.rs                     |  2 +
 crates/rdf-core/src/ownership.rs               | 77 +++++++++++++++++++++
 crates/retrieval/src/fusion_stream.rs          | 27 ++------
 helpers-ledger.toml                            | 96 +++++++++++++++-----------
 6 files changed, 148 insertions(+), 87 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

