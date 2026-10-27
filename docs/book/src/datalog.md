<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Datalog: the Fixpoint Engine

[`purrdf-datalog`](https://docs.rs/purrdf-datalog) is deterministic, `wasm32`-clean
Datalog evaluation: a columnar relation store, an index-selecting planner, and a
semi-naive fixpoint, carrying no ambient I/O, no wall clock, and no RNG.

It exists so that a rule set is *data* rather than a hand-written loop.
[`purrdf-entail`](entailment.md) declares the RDF, RDFS, OWL 2 RL, and D calculi
as DL-clause programs and evaluates them here, which is what lets a reasoning
report carry a **contract hash** of the exact program that ran instead of a claim
about which rules were meant to run.

`purrdf-datalog` is re-exported by the umbrella `purrdf` crate as the
`purrdf::datalog` module — the entailment surface CARRIES its types (a
[`ReasoningReport`](entailment.md) hands out a `datalog::cache::ContractHash`
and a `datalog::seminaive::BudgetReport`), so a consumer that matches on them
needs no second dependency on `purrdf-datalog`. Depend on the crate directly
(`purrdf-datalog = "…"`) only when you want the fixpoint alone, with no other
`purrdf` surface in the build.

## One rule IR: the DL-clause

Every rule has the shape

```text
U₁ ∧ … ∧ Uₙ  →  ∃ȳ. (C₁ ∨ … ∨ Cₘ)
```

where each disjunct `Cᵢ` is itself a conjunction of head atoms. That single shape
holds all five head forms — atomic (an ordinary Datalog rule), existential,
disjunctive, conjunctive, and empty (`false`) — so an axiom like `A ⊑ ∃r.C`,
which lowers to `∃y. (r(x, y) ∧ C(y))` with one *shared* witness, is one rule
rather than two unrelated ones.

The semi-naive evaluator runs atomic and unquantified conjunctive heads. Several
heads share one body evaluation; each recorded derivation names its authored
clause index and head ordinal. Recursion, factor projection and stratified
negation use the same evaluator. Predicate completion follows every producer,
even when a shared rule emits a higher-stratum head early. Independent proofs
check the selected head ordinal; their version 2 encoding carries it explicitly.

Existential, disjunctive and false heads are refused **by name**. The existential
form is not lost by that refusal: the restricted chase
consumes it, minting frontier-addressed Skolem witnesses, which is how the four
existential RDF/RDFS patterns fire and the rule inventory reads complete. What
never happens is a surrogate leaking into an answer — the entailment layer above
withholds every conclusion that mentions a witness at the materialization
boundary and reports the withholding, rather than inventing an answer the caller
did not ask for.

## Querying a stratified rule program

`datalog::demand::evaluate_demanded` answers one `ClauseAtom<N>` goal over
`DlClause<N>` rules and a consumed `RelationStore<D, N>`, for triples (`N = 3`)
or quads (`N = 4`). Bound subjects, objects and graph names propagate through
recursive rules, so asking what one place or graph contains can avoid deriving
disconnected closures. The transformation emits
ordinary clauses and runs the existing compiler and semi-naive evaluator.

The returned `DemandAnswer<D, N>` owns the original dictionary and complete matching
rows, exposed through `answers()` in subject/predicate/object/graph order (without
the graph position for triples). Its default arity remains three. IDs keep their original meaning;
`dictionary()` provides read-only resolution. Authored inputs can be dropped.
Constants and repeated variables in the original goal remain filters even when
a broader demand relation is shared, including equalities between a graph and
another position. A free graph visits every compatible partition; the default
graph is a bound graph identity with its original empty-string surface, distinct
from a free graph and internal demand terms. Authored rules retain their local or
cross-graph relationships. The internal closure is not exposed as a
complete model of the authored program. Dictionary failures remain errors during
preparation, execution and answer access, including empty answers.

This entry point admits stratified relational rules with fixed IRI predicates and
one range-restricted atomic head. It validates the whole program before execution,
including unreachable rules. Guards, base or model reads, variable predicates
and other head forms are refused. A head graph variable must have a
positive relational binder, like every other head variable. Maintained demand
revisions remain outside this entry point's contract.

Negated atoms and guard-free negated conjunctions retain their native semantics.
Demand discovery uses an isolated positive shadow: it overapproximates possible
bindings without using negative filters. Original input rows seed that shadow
with the same subject, object and graph IDs; exact derived facts never feed it.
Exact authored rules retain their negative conditions, so native stratification
completes the demanded lower relations before absence is decided. This can
request more than the minimal slice, but does not unconditionally materialize
entire negative-support relations. Each conjunction keeps its existential
bindings local.

Preparation and the vector returned by `answers()` use resident allocation.
Execution uses the supplied `EvalOptions` and `StopSignal`; cancellation or an
exceeded budget returns no partial answer owner. `work()` and `budget()` describe
the transformed execution, including shadow and demand derivations. For programs
with negation, `preparation()` separately reports original input rows and terms,
and additional shadow input rows and terms. Stored-fact and arena totals include
that augmented input; the generated-term allowance starts from its transformed
term count, not the original source count. Preparation allocation is not execution
work. Positive-only programs add no shadow input. Internal predicates use a framed native digest of the program, goal
and sorted binding descriptors, with collision checks against the full dictionary
and authored constants before any term is minted.

## Determinism

Use `Guard::not_equal("?left", "?right")` for native term inequality, without a
callback. It compares exact bound term identity without numeric coercion.
A caller-defined `Guard::filter("!=", ...)` remains a callback: operation kind,
not its name, selects native behavior and participates in the clause digest.
The independent proof checker re-evaluates native top-level filters from the
checked premises.

- Per-key rows keep insertion order; the arrangement is sorted; no map iteration
  order reaches an output path.
- Plans are content-addressed by a BLAKE3 digest over the planner version, the
  caller's contract hash, and a canonical digest of the clause program. The cache
  is owned by the caller, never a process global — a global would make an answer
  depend on evaluation history.
- Where work is parallelised it uses indexed `par_iter`/`par_chunks` reduced in
  source order, never `par_sort` or `par_bridge`, and degrades to
  inline-sequential on `wasm32-unknown-unknown`.

Identical input yields byte-identical output, on every target.

## Limits refuse; they never truncate

Three limits bound what every run holds and enumerates. All are caller-selected
through `EvalOptions`; unstated settings impose no logical ceiling:

| Limit | Bounds | Set with | Default on `wasm32` | Default elsewhere |
| --- | --- | --- | --- | --- |
| join steps | candidate solutions enumerated | `EvalOptions::with_max_join_steps` | unlimited | unlimited |
| stored facts | facts seeded or derived | `EvalOptions::with_max_stored_facts` | unlimited | unlimited |
| term arena bytes | interned term surface bytes | `EvalOptions::with_max_term_arena_bytes` | unlimited | unlimited |

`EvalOptions::without_stored_fact_limit`, `without_join_step_limit` and
`without_term_arena_limit` remove a selected logical ceiling.
Actual storage admission still applies. The effective ceiling is available in
the budget report and participates in the contract hash.

Stored-fact admission checks each unique insertion before changing the store.
On refusal, `BudgetReport::stored_facts` reports the admitted count and
`observed_stored_facts` reports the first refused insertion, exactly one higher.
Duplicate candidates do not consume another stored-fact allowance.

The same unstated capacity settings apply on every target. Finite ceilings are
application choices, selected explicitly before evaluation. Evaluation reserves
a candidate's credit before expanding it, using one round-wide pool; an exhausted
allowance stops further candidate expansion. Work and retained storage remain
separate coordinates, so selecting one ceiling does not select the others.
Physical storage admission and counter representation still apply.

Term-generation guards address divergent rules. A SHACL shape rule
minting a new focus node every round is re-executed only for the focus nodes
whose inputs changed, not for every earlier one. Term generation has two
guards: the term-generating round limit and the generated-term budget.
A caller-selected finite join-step limit can refuse a run before those guards.

A limit can only refuse. A run inside its limits returns the program's least
model, the same model under every limit that admits it. Complete-only calls
return a refusal past a limit. A retained `EvaluationSession` exposes certified
facts through a separate `PartialModel`. Every caller passing the same options gets the same answer or
the same refusal, and the effective limits are folded into the result's
contract hash, so a closure computed under one set of limits, or on one target
under its defaults, never claims the identity of another. A join-step count
also depends on the evaluator's plan, so a limit sized tightly against one
release can refuse under the next; a limit with headroom cannot change a
completed answer.

The Datalog evaluator reads no clock. A host can adapt a wall-clock deadline or
cancellation flag to the answer-blind `StopSignal` interface. The signal is polled
at session entry and round boundaries. A stopped session can resume with a fresh
signal; neither signal changes the completed model.

## Model output, proofs and measured work

Ordinary evaluation defaults to `DerivationMode::ModelOnly`. Set
`EvalOptions::with_derivation_mode(DerivationMode::Record)` when exported
derivations are needed. `Evaluation::derivations` returns `None` when recording
was disabled and `Some([])` when recording was enabled but derived nothing.
Proof construction refuses a model-only result with
`ProofError::DerivationsNotRecorded`. Recording does not change the least model,
internal canonical witness preference or candidate accounting.

`Evaluation::work` separates unique input facts and committed fact insertions
from physical candidate admissions. For an ordinary completed stratified run,
the fact counts are independent of the join plan. Candidate counts can change
with the planner; the report names its version, and callers can compare
candidates per committed fact. Scheduled withdrawals can cause a later commit
of the same fact and therefore another commit event.

`EvalOptions::with_round_execution(RoundExecution::Sequential)` selects the
direct execution path for one call, without changing a global thread pool.
Both scheduling policies use the same evaluator, program-order merge and sorted
commit, and preserve the same model and recorded derivations.

Complete-only entry points such as `evaluate_guarded` return no model on refusal.
The error names the limit, the numbers observed and permitted, and the knob that
raises it; entailment reports these through `EntailError::Evaluate` or
`EntailError::Chase`.

## Certified partial models and continuations

`EvaluationSession::new` consumes the input store and borrows the immutable
program and guard evaluator. Its `SessionContext` requires the caller's canonical
snapshot, guard-semantics and evidence-regime identities. The session adds the
canonical authored-program identity. Retain its `binding()` value and present
that binding unchanged to `resume`, together with cumulative limits and an
optional stop signal. Scheduling may change; derivation recording remains fixed.
Changing any binding coordinate refuses before execution. The original dictionary
checks its pinned source at every public read and resume boundary.

`ResumeProgress::Paused` carries the original resource or stop refusal.
`partial_model()` borrows the seeded and proved published facts, actual successful
work, optional derivations and each stratum's `Complete`, `SoundLowerBound` or
`Withheld` status. Execution never enters a stratum before all preceding strata
finish, so negation cannot observe an incomplete lower extension. Later dependent
conclusions are withheld. The partial identity includes the original canonical
bindings, governors and consumption in a separate hash domain. Runtime term and
row handles never participate in it.

Raise a ceiling or supply a fresh stop signal to resume the retained joins,
guards, rule/round work, sorted publication and current delta. Successful work
is counted once; retrying the same refused allowance adds no successful work or
guard calls. `EvaluationProofs::of_partial` builds proofs for recorded published
derivations, checked by the same independent checker used for complete models.
Source and guard failures stay hard failures and prevent further resumption.

Only `ResumeProgress::Complete` enables `take_complete()` to return an
`Evaluation`. An incomplete request leaves the session intact. The completed
model, derivations and successful join count match uninterrupted execution under
the final options; its calculus identity uses the ordinary `contract_hash_with`
recipe. The continuation is an in-process owner and has no serialized token form.
