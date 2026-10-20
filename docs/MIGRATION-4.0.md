<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Migrating to PurRDF 4.0

The breaking Rust API changes below belong to the 4.0 major release. Upgrade
the PurRDF crates together when adopting them.

## SHACL processor recursion failures

Exhaustive matches on `ProcessorFailure` must handle
`ValidationDepth { shape, depth, limit }`. Nested filter/exists validation that
crosses its recursion bound now returns this typed processor failure instead
of `ShapesError::Invalid`. Its human-readable message is unchanged. It remains
a hard failure through SPARQL node-expression calls, including COALESCE and IF.

## Datalog proofs and shared heads

Ordinary head conjunctions now execute one shared body. Read `Derivation::head`
alongside `Derivation::rule` to identify the producing atom. `ProofArena::by_rule`
selects head zero; use `by_rule_head` to construct a proof for another head.
The independent proof encoding is version 2 and explicitly includes this ordinal.
Regenerate stored version 1 proof bytes from the original inputs and rule program;
the decoder refuses their old tag. Planner version 6 also changes content-addressed
plan identities, causing earlier cached plans to miss and be replanned.

`Guard::not_equal` supplies native term inequality without a callback. Programs
containing native filters use an operation-tagged version 2 clause identity.
Unchanged callback-only programs retain their clause and reasoning-contract
identities. A callback named `!=` keeps its caller-defined meaning and has a
different identity.

Use `DlClause::with_ordered_body` with `clause::BodyLiteral::{Atom, Guard}` when
a guard computes a key consumed by a later positive atom. Guard inputs must be
bound at that point, and outputs must be fresh. The join planner may reorder atoms
within each segment but never across a guard. Negated conjunctions remain attached
with `with_negations` and run after the positive body and its guards.

`with_guards` keeps its existing meaning: all guards follow the full positive
join. Ordered bodies with an early guard use a version 3 clause identity that
includes its position; an equivalent body with all guards at the end keeps its
existing clause identity. Callback guards still require the caller's evaluator
and remain outside the independent proof checker's supported fragment.

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
