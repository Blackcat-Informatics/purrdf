# Branch under review — issue #491

Branch `paudley/491-rdf-1-2-index-reifiers-by-triple-term` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (3)

- `d0f544f49` perf(rdf): index structural reifiers and annotation values  _2026-10-15_
- `078f9c973` Route exact reifier targets through native statement indexes  _2026-10-15_
- `3e6bab72d` Index resident reifier targets with admitted ordinal ranges  _2026-10-15_

## Files changed

```
Makefile                                           |  11 +
 crates/rdf-core/src/dataset_view.rs                | 157 +++++++
 crates/rdf-core/src/ir/builder/triple.rs           |   1 +
 crates/rdf-core/src/ir/dataset.rs                  | 472 ++++++++++++++++++---
 crates/rdf-core/src/ir/frozen_terms.rs             |   8 +
 crates/rdf-core/src/ir/triple.rs                   | 203 ++++++---
 crates/rdf-core/src/ir/triple/read.rs              | 136 +++++-
 crates/rdf-core/src/lib.rs                         |   4 +-
 crates/rdf-core/tests/native_regex_storage.rs      |  93 ++++
 crates/rdf-core/tests/triple_carrier.rs            | 153 +++++++
 crates/sparql-eval/Cargo.toml                      |   5 +
 crates/sparql-eval/src/bgp.rs                      |  98 +++--
 crates/sparql-eval/src/bgp/window.rs               |  67 +--
 crates/sparql-eval/src/engine.rs                   |   2 +-
 crates/sparql-eval/src/governor/ledger.rs          |  31 +-
 crates/sparql-eval/src/governor/mod.rs             |  13 +-
 crates/sparql-eval/src/path_relation.rs            |   1 +
 crates/sparql-eval/src/statement_layer.rs          | 191 ++++++++-
 crates/sparql-eval/tests/governed_query.rs         | 211 ++++++++-
 crates/sparql-eval/tests/statement_indexes.rs      | 116 +++++
 crates/sparql-eval/tests/support/mod.rs            |   1 +
 .../sparql-eval/tests/support/statement_claims.rs  |  81 ++++
 docs/SPARQL-GOVERNOR-PROFILE.md                    |  20 +-
 docs/design/purrdf-backend-contract.md             |  15 +
 helpers-ledger.toml                                | 222 +++++-----
 25 files changed, 1975 insertions(+), 337 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

