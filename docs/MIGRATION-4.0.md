<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Migrating to PurRDF 4.0

The breaking Rust API changes below belong to the 4.0 major release. Upgrade
the PurRDF crates together when adopting them.

## Datalog proofs and shared heads

Ordinary head conjunctions now execute one shared body. Read `Derivation::head`
alongside `Derivation::rule` to identify the producing atom. `ProofArena::by_rule`
selects head zero; use `by_rule_head` to construct a proof for another head.
The independent proof encoding is version 2 and explicitly includes this ordinal.
Regenerate stored version 1 proof bytes from the original inputs and rule program;
the decoder refuses their old tag. Planner version 4 also changes content-addressed
plan identities, causing earlier cached plans to miss and be replanned.

`Guard::not_equal` supplies native term inequality without a callback. Programs
containing native filters use an operation-tagged version 2 clause identity.
Unchanged callback-only programs retain their clause and reasoning-contract
identities. A callback named `!=` keeps its caller-defined meaning and has a
different identity.

## Query-algebra projections

Parsed `SELECT *` can produce `GraphPattern::ProjectStar`. Its `ProjectionVars`
retains an immutable, ordered snapshot of the visible variables, allowing nested
wildcard projections to share earlier names. `GraphPattern::Project` continues
to hold an explicit `Vec<Variable>`.

`GraphPattern` is an exhaustive public enum, so downstream exhaustive matches
must add the `ProjectStar` arm. To visit a projection's inner pattern, match both
forms when obtaining that child:

```rust,ignore
let projection_inner = match pattern {
    GraphPattern::Project { inner, .. }
    | GraphPattern::ProjectStar { inner, .. } => Some(inner),
    _ => None,
};
```

For code that reads column names from a `&GraphPattern`, use the shared ordered
view instead of assuming every parsed projection holds a vector:

```rust,ignore
if let Some(names) = pattern.projected_names() {
    for variable in names.iter() {
        println!("{}", variable.as_str());
    }
}
```

`projected_names()` returns `None` for other pattern variants. Its iterator
borrows the projection's names and preserves their visible order. Keep explicit
`Project` construction when a rewrite deliberately chooses an owned variable
list. A consumer that needs such a list can clone the shared view's names at
that boundary; expanding every nested wildcard snapshot recreates the repeated
prefix work the compact representation avoids.
