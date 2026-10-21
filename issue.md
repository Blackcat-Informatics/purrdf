# Issue #488: Datalog, chase and entailment: certified partial models and resumable evaluation instead of total refusal

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

When a Datalog, chase or entailment run stops before its fixpoint (a limit, a stop signal, or memory), return what it has **proven**, typed so it cannot be mistaken for a complete model, together with a continuation that resumes the run from where it stopped. Governed SPARQL already does the first half (`Certain` / `AtMost` / `Unknown`); this asks for the same model in the reasoners, and for the ability to resume instead of re-running from zero.

## Current behaviour (`main` at `6273b6173`)

- Datalog: "a run past one returns no model at all ([`EvalError::BudgetExhausted`]), never a truncated one" (`crates/datalog/src/seminaive.rs:236-242`). `BudgetExhausted` carries a resource and a `BudgetReport` of counters (`seminaive.rs:754-759`), no facts. `Stopped` likewise: "there is no partial least model here either, and the rounds already committed are not an answer to anything" (`seminaive.rs:774-787`; `stop.rs` module docs: "There is no third outcome").
- Chase: `ChaseError::BudgetExhausted` is "as total a refusal" (`chase.rs:736-744`).
- Entailment materialization: "A limit refusal is TOTAL … there is no partial closure to hand back with a note attached" (`crates/entail/src/lib.rs:363-376`).
- The OWL DL hypertableau is the exception and the model: `DlCompleteness::BudgetExhausted` (`entail/src/reasoner/certificate.rs:155-170`) reports "every answer that IS reported alongside this is still sound … but the answer set is not complete", and a boolean service answers `Verdict::Unknown`.
- Governed SPARQL returns `PartialAnswers::{Certain, AtMost, Unknown}` with `positional_prefix` (`crates/sparql-eval/src/governed.rs:351-363`, `457-496`). Its resume story is re-running the same query and snapshot under a larger ceiling (`governed.rs:484-491`); there is no continuation token.

## The concern the current design protects, and why a typed partial meets it

The crate docs are right that a truncated closure **presented as complete** is the failure to prevent, and that limits must not make the answer the caller's choice. A certified partial result meets both:

- For a positive (monotone) stratum, every fact derived before stopping is entailed: it has a derivation from the input, and with proof recording on, that derivation is checkable. Facts of every fully completed stratum are exact. What is unknown is only which further facts would have been derived.
- With stratified negation, facts in a stratum that negates an incomplete stratum are not sound and must be withheld; the result says which strata are complete, which are a sound lower bound, and which are unknown.
- The result is a different type from a model (as `PartialAnswers::Certain` is from a `SparqlResult`), so no caller can read it as complete by accident, and its identity (`cache::contract_hash_with`) records that it is partial and at what consumption.

## Why it matters

Katamari evaluates its derived views ("current claims at these coordinates", ADR-0038) as SPARQL 1.2 RL rule sets over every claim's reifier, and its highest-assurance stores hold an AI's memory, where the rule is: never refuse and return nothing while anything verifiable remains; carry what failed as evidence beside what succeeded. The study behind this request found the same lesson in five large systems: Wikidata's Blazegraph lost days of completed work to timeouts and restarts from zero; ClickHouse returns a partial result "as if the source data ran out" (incomplete presented as complete, the failure purrdf rightly refuses); ScyllaDB fixed its version by paging on work done and returning a resumable cursor with evidence. purrdf already has the certified half in SPARQL; it needs to reach the reasoners, and both need resumption.

Without this, a removable limit (#364) is unattractive to use: the only outcomes of a long run are "complete" or "nothing".

## Proposed direction (a sketch)

1. **Partial models.** `EvalError::BudgetExhausted` / `Stopped` (or a new `Ok`-side outcome) carry a `PartialModel`: the derived facts, per-stratum status (`Complete`, `SoundLowerBound`, `Withheld`), the consumption report, and, when proofs are recorded, a derivation for every returned fact. Entailment materialization and the chase follow the same shape; the DL certificate already does.
2. **Continuation.** At a round boundary the semi-naive state (store, current delta, stratum, round) is a value. Return it as an opaque, snapshot-bound `Continuation` that `resume(continuation, options)` accepts, with new limits. A resumed run that completes returns exactly the model an uninterrupted run would (same identity). Serialization of the continuation can be a later step; an in-process value already removes "start again from zero".
3. **SPARQL.** Add the same: a governed evaluation that stops on a deterministic ceiling returns a continuation alongside its `Certain` prefix, so paging is "continue", not "re-run with a larger ceiling and skip what you already have".
4. Snapshot binding is part of the continuation: resuming against a different snapshot is refused with typed evidence of what changed.

## Acceptance criteria

- A transitive-closure program stopped by a join-step limit returns a partial model whose every fact is in the full model, and whose proofs check with the independent proof checker.
- A stratified program with negation stopped mid-stratum withholds the dependent strata and says so.
- `resume` after a stop yields the same model and the same contract hash as an uninterrupted run, with total join steps equal to the uninterrupted count (no recomputation from zero).
- Entailment `materialize_with` past a limit returns a partial closure typed as partial, never as `ReasoningReport`'s complete form.
- A governed SPARQL query stopped by a deterministic ceiling can be continued to completion; concatenated pages equal the complete answer.

## Related

#364 (limits that should be removable), #479, #179 (governed SPARQL budgets), #103 (typed page failures and budgets through paged execution).


## Comments (1)

### paudley — 2026-10-21T01:47:50Z

The factor execution ownership unit is merged in PR #664 through ghprsq. Production retains eligibility, ground signed filters, productive delta relations, projected head products and candidate winners across pauses, using original row coordinates and deterministic witness ordering.

Qualification: 427 native library +44 integration cases, actual supported WASM29, allocator controls, strict static checks and normal hooks pass. Full CI39523976522 and focused native/WASM39523862730 pass. Final-head automated review covered all8 changed files and reported no actionable findings.

This parent remains open. Public source/program/guard/evidence-bound continuations, typed certified partial models, whole round/commit/stratum/delta ownership, shared memory admission/spill and cross-engine acceptance remain mandatory. The next implementation retains whole-rule joins, guards, negative filtering and head ordinals; its new targeted triple/quad controls pass, but it is not yet submitted or fully qualified. Datalog remains the first work stream.


