# Branch under review — issue #556

Branch `paudley/556-segmented-stack-reads-an-ordinary-quad` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (1)

- `cef34c8aa` Preserve authored ordinary rows across sealed stack layers  _2026-10-15_

## Files changed

```
crates/rdf-core/src/ir/mutable.rs              | 19 +++---
 crates/rdf-core/src/ir/paged/stack.rs          | 40 ++---------
 crates/rdf-core/src/ir/segmented/query/view.rs | 17 ++---
 crates/rdf-core/tests/paged_stack.rs           | 91 ++++++++++++++++----------
 crates/rdf-core/tests/segmented_stack.rs       | 81 ++++++++++++++++++++++-
 crates/sparql-eval/tests/paged_stack_query.rs  | 45 +++++--------
 docs/design/purrdf-backend-contract.md         | 10 +--
 helpers-ledger.toml                            | 86 ++++++++++++------------
 8 files changed, 222 insertions(+), 167 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

