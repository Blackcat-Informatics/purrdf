# Issue #667: Deliver bound Datalog continuations and certified partial models

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Behavior

Deliver the ordinary stratified Datalog portion of #488 as one public behavior unit: a snapshot-bound in-process continuation and a certified partial model that cannot be mistaken for `Evaluation`. This shares the retained native join/rule implementation from #665 and completes round, sorted publication and stratum/delta ownership for this entry point.

## Acceptance

- `EvaluationSession` retains the original store, immutable program and pure guard evaluator. Resume validates canonical program, snapshot, guard and evidence identities before doing work; recording mode remains fixed. Source invalidation and guard failures retain their original typed errors and prevent further execution.
- `PartialModel` exposes only seeded/proved published facts, optional recorded derivations, successful consumption and explicit `Complete`, `SoundLowerBound` and `Withheld` stratum statuses. Negative dependent execution waits for lower fixpoints. The partial identity is domain-separated and includes canonical input bindings, governors and consumption, never runtime term/row IDs.
- Every recorded partial proof checks with the independent proof checker. Transitive-closure and stratified-negation fixtures cover triples and quads, sequential and parallel scheduling, ModelOnly and Record, every credit/fact cut, repeated one-credit progress and stopped-run resumption.
- Retain pending winners and original guard work across all five governor refusals. Repeating an unchanged refusal adds no callbacks or successful work. Resumption returns the same complete facts, derivations, successful join count and ordinary final calculus contract as uninterrupted execution.
- Native wide integer handles resume without lexical projection or remapping. A complete `Evaluation` can be taken only after successful completion, once; premature requests preserve the continuation.
- Preserve existing complete-only callers through the same execution owners. Run actual native and WASM public controls, relevant compiler/hygiene/documentation gates, hosted qualification, feedback review and clean integration.

## Parent mapping

#488 remains open for the chase, typed entailment materialization partial closures and actual governed-SPARQL continuation/pages, together with their original acceptance. #486 remains open for common measured admission, spill and complete allocation/lifetime accounting. #501 retains its reusable proof-query/revision contract. This child closes only when the public Datalog behavior above is delivered; internal preparation or green compilation alone does not complete it. The portfolio queue retains every parent requirement and the subsequent signed-maintenance/demand/proof/chase order.


## Comments (0)

