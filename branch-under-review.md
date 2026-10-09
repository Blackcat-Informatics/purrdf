# Branch under review — issue #401

Branch `paudley/401-update-transfers-preserve-rdf-1-2` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (2)

- `20e2e637c` Preserve typed graph transfers and fresh LOAD identities  _2026-10-08_
- `b0fc9e889` Preserve physical RDF record roles in mutable datasets  _2026-10-08_

## Files changed

```
crates/rdf-core/src/ir/builder.rs                |   29 +
 crates/rdf-core/src/ir/import.rs                 |   45 +
 crates/rdf-core/src/ir/mod.rs                    |    5 +-
 crates/rdf-core/src/ir/mutable.rs                | 1575 ++++++++++++++++++----
 crates/rdf-core/src/ir/mutable/delta_view.rs     |  237 ++--
 crates/rdf-core/src/ir/validate.rs               |  154 ++-
 crates/rdf-core/src/lib.rs                       |   22 +-
 crates/sparql-eval/Cargo.toml                    |    4 +
 crates/sparql-eval/src/update.rs                 |  282 +++-
 crates/sparql-eval/tests/update_typed_records.rs |  563 ++++++++
 10 files changed, 2454 insertions(+), 462 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

