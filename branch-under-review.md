# Branch under review — issue #457

Branch `paudley/457-paged-tier-an-lsm-style-stack-of-sealed` against `origin/main`. Gathered by `stagectl brief
--mode review`; every hash and path came from this repository.

## Commits (2)

- `b2edf7450` rdf: expose paged stacks and qualify guarded consumers  _2026-10-06_
- `dd9109995` core: layer sealed paged generations with pinned mutable snapshots  _2026-10-06_

## Files changed

```
crates/purrdf/src/lib.rs                           |   32 +
 crates/rdf-core/src/ir/global.rs                   |   73 +-
 crates/rdf-core/src/ir/mod.rs                      |   10 +-
 crates/rdf-core/src/ir/mutable.rs                  |   28 +-
 crates/rdf-core/src/ir/mutable/delta_view.rs       |   27 +-
 crates/rdf-core/src/ir/paged/mod.rs                |    5 +
 crates/rdf-core/src/ir/paged/provider.rs           |   30 +
 crates/rdf-core/src/ir/paged/query.rs              |  264 ++++-
 crates/rdf-core/src/ir/paged/stack.rs              | 1174 ++++++++++++++++++++
 crates/rdf-core/src/lib.rs                         |   20 +-
 crates/rdf-core/tests/paged_stack.rs               | 1003 +++++++++++++++++
 crates/rdf/src/lib.rs                              |   74 +-
 crates/sparql-eval/examples/paged_stack.rs         |  105 ++
 crates/sparql-eval/examples/support/paged_stack.rs |   48 +
 crates/sparql-eval/tests/paged_stack_query.rs      |  790 +++++++++++++
 docs/design/purrdf-backend-contract.md             |  112 ++
 16 files changed, 3681 insertions(+), 114 deletions(-)
```

## ADRs the changed files cite

None of the changed files cite an ADR.

These are the settled decisions this branch touches. The question
for stage2 is whether the change fits them, not merely whether it
compiles.

