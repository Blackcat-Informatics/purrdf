<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Migrating to PurRDF 4.0

The breaking Rust API changes below belong to the 4.0 major release. Upgrade
the PurRDF crates together when adopting them.

## Columnar projection errors

`purrdf::columnar::project` and `write` return
`ColumnarWriteError<D::ReadError>`. Update explicit return types and source-error
matches: `ColumnarWriteError::SourceRead` carries the original read error rather
than a formatted `ColumnarError::SourceRead` detail. Codec refusals are retained
as `ColumnarWriteError::Codec(ColumnarError)`. Human message prefixes are
preserved, and inspection borrows the original cause. Reading prepared Parquet
files and encoding an existing projection still use `ColumnarError`.

Match native content integrity failures through `ColumnarError::ContentStore`
and `BlobContent { row, cause }`, and RDF reconstruction failures through
`Reconstruction(RdfDiagnostic)`. These replace formatted `Malformed` details;
the complete original cause, digest bytes and zero-based row remain inspectable.
Storage-bearing reconstruction diagnostics retain their enclosing fields and
attachments. Existing `Clone`/`Eq` implementations and human prefixes remain.

Freeze failures previously returned as `ColumnarError::Storage` now also return
`Reconstruction(RdfDiagnostic)`. Update `Storage` matches for freeze failures to
handle `Reconstruction` and inspect its diagnostic's `storage_error()` instead.

## Explicit capacity limits

Unstated Datalog stored-fact, join-step and term-arena settings now impose no
logical ceiling on every target. Physical storage admission and counter
representation still apply. Select finite limits explicitly; effective settings
remain part of contract identity. Target-specific capacity default constants
have been removed. `EvalOptions::without_stored_fact_limit`,
`without_join_step_limit` and `without_term_arena_limit` remove selected ceilings.

`MaterializeLimits` adds `max_term_arena_bytes: Option<u64>`; use `None` to
apply no logical arena ceiling. `RegimeHost::limit_knobs` and `service_knobs`
return three names, in stored-facts, join-steps, term-arena-bytes order.

Materialization, conclusion-directed services and entailment queries expose
`max_term_arena_bytes` in Python, `maxTermArenaBytes` in JavaScript, and
`--max-term-arena-bytes` in the CLI. JavaScript free functions take the new
optional argument immediately after `maxJoinSteps`; query methods accept it
in their options. An arena refusal preserves the observed and permitted byte
counts and names the calling surface's control.

The corresponding C entry points declare ABI 0.12.0 and add a nullable `const uint64_t *` immediately
after `max_join_steps`. Rebuild C clients against the regenerated `purrdf.h`;
pass `NULL` to retain the default. A refusal leaves result buffers untouched.

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

## Datalog guard values

`GuardCall::inputs` now borrows `DictionaryTerm<Id>` values: original `Id` handles
or newly computed `Surface` spellings. `GuardEvaluator::evaluate` returns
`GuardRows<Id>`, whose cells are `DictionaryValue::Id` or owned
`DictionaryValue::Surface`. A native callback can retain and return original
identities directly, without rendering them or looking up a spelling.

A callback that needs lexical values calls `call.evaluate_lexical(|inputs| ...)`.
The helper pins the original dictionary owners throughout that callback and
preserves typed source failures. Emit an owned string with `.into()` or an
explicit `DictionaryValue::Surface`; the engine reuses an existing identity when
the spelling is already known. Native inequality compares the original bindings.
Guard declarations and clause identities retain their existing meaning.

## Datalog dictionary order and native integers

`TermDictionary` implementations now provide `compare_terms`. This defines the
canonical total order used by partition scans, round publication and tied witness
selection. An RDF dictionary calls `dictionary::compare_lexical_terms` to preserve
canonical lexical order and original dictionary pins. A native dictionary may
compare original IDs directly. `compare_rows` applies that contract in position
order and preserves source errors.

Maintained partition directories retain original term IDs. They establish their
canonical order through `compare_terms` before the dictionary transaction commits;
publication and subsequent traversal need no lexical copies or source comparison.
Removing a partition preserves every surviving partition's original identity.

`dictionary::IntegerDictionary<u32>` and `IntegerDictionary<u64>` provide the
native alternative. Construct one with an explicit `with_symbols` table for the
program's named constants, insert `DictionaryTerm::Id` rows and return
`DictionaryValue::Id` from native guards. No cell value is interned, formatted or
resolved, including during recursive model-only evaluation and witness ties.
Unknown symbols and explicitly requested lexical projection fail with typed
errors; this dictionary supplies no fabricated numeric spellings. Recorded
lexical derivations therefore require a dictionary with actual spellings.

The rule evaluator continues to use triple or quad shapes. Separately,
`RowColumns<I, N, R>` and `RowIndex` store and index caller-selected row widths;
they do not add an arbitrary-arity rule language.

Store append boundaries (`row_slots`, `rows_from`) now use `u64`. Keep these
logical coordinates separate from physical slice lengths. Maintained stores can
retire row metadata and empty partitions without renumbering surviving handles;
partition IDs must be resolved through the store rather than cast to array slots.

`seminaive::Maintained` owns a materialized model and accepts signed
`maintenance::ChangeBatch` transactions over its original dictionary IDs.
`apply` retains a transaction; `resume` completes it or preserves its pending
state under a governor refusal. Read `model_delta` and `applied_watermark` only
after completion. `relation` distinguishes complete, sound lower-bound and
withheld extensions, with the host's original applied and target positions.
Native witnesses borrow their original dictionary; request lexical derivations
explicitly at the presentation boundary.

Use `begin` and `resume_until` to retain transaction preparation and execution
under one cumulative operation allowance. `transaction_work` includes admitted
row, dependency, comparison and task steps, retained table-scan layout bounds,
and original native join steps. A `MaintenanceError::WorkLimit` preserves the
transaction and states its consumed and next required allowance. Original
`EvalOptions` limits still govern facts, terms and native joins.

Use `apply_next` to poll a `ChangeSource` directly. It refuses a busy model before
polling, preserves the host's original failure as `ChangeApplyError::Source`, and
returns `false` only for an unavailable batch. An accepted batch, including an
empty transaction, advances its applied position only after `resume` completes.

For fresh-term streams, construct the store with `store::ReclaimingInterner` and
use `Maintained::new_reclaiming`. `RelationStore::into_reclaiming` transfers an
existing store's arrangements and IDs unchanged. The explicit `ReclaimDictionary`
protocol retires unused dictionary metadata; ordinary `TermInterner` keeps its
permanent-ID contract. Current live facts and proofs retain their terms. The
latest `ModelDelta`, including deleted facts, also pins its spellings until the
next transaction completes. A saved raw ID alone does not pin a term;
`TermExpired` reports an ID whose owner has released it. `IntegerDictionary`
retains declared program symbols and the host's external identity authority.

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
