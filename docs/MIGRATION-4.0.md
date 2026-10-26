<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Migrating to PurRDF 4.0

The breaking Rust API changes below belong to the 4.0 major release. Upgrade
the PurRDF crates together when adopting them.

## Certified direct OWL answers

Call `direct_subsumptions()` on `Certified<ClassHierarchy>` and `direct_types()`
on `Certified<Realization>`, before discarding or splitting the certificate.
The raw hierarchy and realization no longer expose direct candidates, including
in debug output. Their established positive facts remain available.

Both accessors return a `DirectnessRefusal` unless the original certificate is
exactly `DlCompleteness::Decided`. Its `certificate()` borrows the original full
certificate, also exposed through structured inspection. A completely searched
fragment with omitted axioms cannot establish directness for the supplied
ontology: those axioms could add an intermediate class or a more-specific type.

Shared classification and realization calls now refuse incomplete directness
before emitting answer rows. Their error message starts with
`owl-directness-unavailable` and retains the existing complete certificate block.
One-shot, session and proof calls use the same path across language wrappers.
Fully decided answer, certificate and proof bytes remain unchanged. The shared
host methods accept no stop input; native refusals retain native cancellation
evidence without adding a new host protocol claim.

## Canonical reserved-vocabulary refusals

`ReservedVocabulary` now lends its original IRI and position through `iri()` and
`position()`. Cloning shares the immutable payload and its original admission
owner. The refusal remains readable after its source is destroyed.

Sources must provide `DatasetView::reserve_owned_workspace` when an offending
IRI requires ownership. A missing capability returns
`CanonError::OwnedWorkspaceUnavailable`; an unknown live ceiling does not grant
permission. Original admission and source failures remain native causes.
Verified resident carriers explicitly provide unbounded admission.
`check_admissible` and `check_admissible_view` now return `CanonError`, including
physical allocation refusal. Flat boundaries retain native causes through
`ViewCanonError::Refused(CanonError<E>)`.

This admission covers the retained refusal and its publication control. It does
not certify canonicalization scratch or successful canonical result ownership.

## Canonical Function Ontology catalogs

Build `FnoCatalogInput` and pass it to `FnoCatalog::try_new`. The admitted
`FnoCatalog` is immutable; `as_input()` borrows its fields. Admission checks RDF
values, conflicting node descriptions and references before changing the input.
An `FnoAdmissionError` owns the original rejected input and native reason.

Labels, document banners and descriptions now use literal `TermValue` owners,
preserving datatype, language and RDF 1.2 base direction. Use `FnBlank { label,
scope }` for mapping identities instead of a sanitized `bnode_label` string.
The existing term conversion and blank-label codec preserve those native scopes
at RDF egress.

Set-valued vectors sort canonically and indistinguishable duplicates collapse;
compatible roles on one subject share their canonical facts. `expects` keeps
its original list order. Emission preserves the admitted RDF
facts while canonical order and collision-free list identities intentionally
change output bytes. Distinct mapping labels and function IRIs no longer merge
through sanitization. Slice's `list_functions_catalog` and `emit_list_functions`
now return admission results and retain their explicit language retagging.

Read a catalog with `FnoCatalog::from_dataset(&view, ontology_iri)`. The ontology
IRI is explicit; the document is the unique subject whose `dcterms:isPartOf`
names it. The reader uses the same admission path as caller-built catalogs and
returns boxed `FnoReadError` values carrying original field operands, strict list
causes or source-read failures. A source failure discards the entire result.

The strict profile covers the complete default graph. Every statement must be
represented by the admitted model or its RDF lists. Named graphs, reifiers,
annotations, disconnected outputs or child mappings, unknown properties and
extra list-cell facts are refused. Parameters and implementations may stand
alone. Lists have disjoint blank cells; shared `rdf:nil` represents empty lists.
Labels and descriptions preserve literal facets. Implementation formats and
binding names are plain strings; required flags use the exact boolean spellings
`true` and `false`. These boundaries preserve both catalog equality after a
write/read and graph isomorphism after a read/write.

## Caller-owned loss profiles

Select `purrdf_core::loss::LossProfile::builtin()` and extend it with
`LossDefinition` rows to declare caller-owned projection pairs and loss codes.
Pass that profile to `check_ledger_sound_in` and `LossLedger::render_json_in`.
Registration descriptions define code meanings; runtime entry notes describe
particular losses and need not repeat those descriptions. Conflicting registered
meanings return `LossProfileError` with the original rejected definition.

Soundness now also checks that each entry belongs to the requested conversion
pair. A reused code can no longer hide a mismatched pair. The explicit check
returns a borrowed `LossSoundnessError` naming the first offending entry's code
and pair operands. The builtin `check_ledger_sound` wrapper retains its string
error type, with first-refusal presentation. Existing builtin render methods and
generated loss matrices retain their schema and bytes. Custom profiles have no
ambient registration and do not affect another caller's builtin profile.

## Retrieval canonical UTF-8 errors

`purrdf_retrieval::PlanError::InvalidUtf8` now carries `source: core::str::Utf8Error`
beside `what`. Update explicit constructions and exhaustive field patterns.
Borrowed structured inspection and the standard error source retain the original
valid-prefix offset and invalid-sequence length, independently of the input buffer.
The human message and `invalid-utf8` refusal identity are unchanged.

`FusionError::MalformedProfile` now carries `Box<ProfileRefusal>` instead of a
rendered string. Its closed native reasons retain canonical/IRI decoder causes,
profile fields and original candidate owners. The inspected `reason` field is now
a borrowed cause with its own code and fields, replacing its former text value.
Use `StructuredError` inspection to recover fields and `Display` for presentation.
The outer
`fusion-malformed-profile` inspection identity and human messages are unchanged.

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

`purrdf_shapes::srl::infer_until` and `infer_triples_until` accept a borrowed
`Option<&dyn StopSignal>`. `Inference::usage()` retains the complete original
`BudgetReport`; `RuleLimitExceeded::usage()` exposes it after exhaustion.
`SrlError::LimitExceeded` and `RulesError::LimitExceeded` now carry
`Box<RuleLimitExceeded>`. Update explicit constructions and exhaustive matches;
method access through a borrowed payload remains unchanged. Their standard
error sources still borrow the original `RuleLimitExceeded` directly.
Handle the new `Stopped { cause }` variants separately from resource limits.
Their native cause owns the stopped evaluator's usage report. Cancellation is
cooperative between work boundaries, including within scheduled rule groups;
it never returns an unfinished inference result or changes contract identity.

Unstated Datalog stored-fact, join-step and term-arena settings now impose no
logical ceiling on every target. Physical storage admission and counter
representation still apply. Select finite limits explicitly; effective settings
remain part of contract identity. The calculus version advances to v3 because
chase work-counter overflow now refuses rather than permitting further work.
All contract identities change, including explicitly finite profiles; cached
results must be recomputed. Shared report vectors change their contract-hash
lines while canonical model bytes stay unchanged. Historical fixture capacity
values remain explicit; they do not retain the previous calculus identity.
Target-specific capacity default constants have been removed.
`EvalOptions::without_stored_fact_limit`,
`without_join_step_limit` and `without_term_arena_limit` remove selected ceilings.

## HNSW arithmetic refusals

Shared distance pair and batch APIs now return `Result<f64, NonFinite>` instead
of `Option<f64>`. `Bounded::NonFinite` carries that same native cause. Inspection
retains the original checked `binary64` bits, including a partial sum refused at
an existing checkpoint. Update output buffers and refusal patterns accordingly.

`HnswError::NonFiniteDistance` adds `cause` beside `row`; its standard error
source and structured inspection borrow the original shared distance refusal.
Exact kNN data errors also retain query and candidate rows, the metric and that
cause. Successful values, checkpoint order and outer human messages are unchanged.

`HnswError::ArithmeticOverflow` now carries `ArithmeticRefusal`. Update unit
matches to `HnswError::ArithmeticOverflow(cause)`. Borrowed inspection keeps the
outer error code and exposes the failed operation, original operands, byte/count
units and destination limit through its cause. Integer-conversion failures retain
their original native source. Successful calculations, refusal order and human
messages are unchanged.

## Regime term arena controls

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
