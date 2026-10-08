# Prior-art assessment

## Evidence and coverage

This assessment uses the freshly generated `issue.md`, `prior-art.md` and
`brief.json`, together with an independent read-only source and history audit at
main `6273b6173`. The issue capture contains the complete body and one comment.
Applicable `AGENTS.md`, `.baseline` and `.goals` were read. No forge retrieval,
source edits, builds, tests or benchmarks were performed for this assessment.

The generated recurrence count is one `performance` trailer among the last 200
commits searched. It is not an exhaustive count of earlier performance defects,
nor proof that this topology has appeared only once. The linked-item crawl and
title search identify related work; their titles alone do not establish that
those implementations repaired this rule-engine defect.

## Defect confirmed against production source

`crates/datalog/src/plan.rs::sips_order` ranks an atom by known positions,
constants, repeated-variable equality and authored position. Its known-position
count includes constants. After binding the Vault subject, the connecting target
edge and the unconnected Spool type atom tie on known positions, and the Spool
atom wins on constants. This exactly reproduces the issue comment's static
trace. Connectivity must mean sharing an already bound variable, including a
predicate or graph variable; common constants do not connect components.

The cyclic/hybrid `RulePlan` construction bypasses `sips_order` and emits join
groups at their first authored occurrence. A repair limited to the acyclic helper
therefore leaves another physical ordering path. Positive binary and hybrid
execution materialize each stage as `Vec<SlotSolution>` before forming the head.
The Cartesian intermediate is an implementation cost, not a property of the
issue's connected rule or its output.

The substrate is the actual production home: SPARQL 1.2 RL and SHACL rules lower
through `crates/shapes/src/srl`, scheduled execution calls the shared Datalog
round evaluator, and entailment materialization calls `evaluate_guarded`. A new
private evaluator or a frontend-only rule rewrite would not repair all callers.

## Relevant earlier implementations

The inspected history of `sips_order` begins with `15c5fc2a8`, which introduced
the store-independent planner. `7623c4bf8` widened the same ranking from two
positions to all four. The inspected main-line function history contains no
later connectivity-priority repair.

`fb52af337` made stored-fact and join-step limits caller-settable and added
`delta_can_match`, which skips a semi-naive decomposition whose anchor cannot
match new rows. That avoids rescanning a complete recursive relation for an
impossible delta; it does not prevent two unrelated type atoms from being joined
before their connecting edge. `edc22b059` repaired needless re-execution of SHACL
shape rules for unchanged focus nodes. That is a producer scheduling improvement,
not component factorization.

Existing documentation records a Cartesian run allocating about 20 GB under a
raised join limit as the reason for retaining the low default. These are retained
historical measurements, not measurements repeated by this assessment. The
current default must not be raised to make the new regression pass.

## Accounting and allocation debts

The existing `StepGovernor` grants every rule the evaluation's entire remaining
allowance plus one. Its documentation explicitly bounds a round by rule count
times that allowance. This affects sequential execution as well as parallel
execution. Tight global admission must not introduce worker-order-dependent
refusals or reject an asymmetric workload that fits the total budget.

`GuardEvaluator::evaluate` returns a whole row vector before the substrate can
charge it. Body-guard expansion lacks an exhaustion check in its inner row loop.
Negated conjunction scans and guard calls do not receive a governor. The SHACL
producer implementation first obtains all triples from `rules::execute_rule`
and then builds all lexical output rows. Checking a loop after those allocations
does not establish bounded producer execution.

The term arena remains a fixed 16 MiB ceiling, checked against interned surface
bytes; cache contracts embed that ceiling. Caller/effective arena policy, shared
physical memory admission, full producer streaming, spill and resume are still
unfinished portfolio work owned by the evaluation-governor, shared-memory and
certified-resume lanes. They must not be reported as completed by this rules
slice. Physical-only defaults remain a later portfolio acceptance gate.

## Design constraints for the rules slice

Use connected ordering in both acyclic operators and hybrid groups. For truly
disconnected unguarded components, retain factorized results and project
existential components before enumerating heads. The certificate must account
for coupling through negative atoms/conjunctions. Guarded programs retain
solution multiplicity and authored guard-stage barriers; arbitrary callbacks
must not be skipped merely because an earlier witness produced the same head.
Products genuinely required by head variables may be streamed, but their size
must not be disguised as additive output.

Canonical proof selection is not insertion-first or first-success selection.
`Candidate::preferred_over` compares maximum proof height, summed source heights,
sorted source facts, rule index, then authored source facts. Per-head proof
frontiers need height-threshold-sensitive selection to preserve that order when
components are combined. Assumed-row confirmations need the same deterministic
selection. Restore authored premise order in explanations.

Checks before evaluator-owned expansion and a shared round admission mechanism
belong in this slice. Their presence does not establish that the full callback
producer or every physical allocation has become governed. Exhaustion remains a
typed total refusal here; never convert it to an empty producer, false guard,
empty negation result or successful partial closure. Planner/calculus identity
versions must identify the changed execution/accounting law while the canonical
authored clause hash remains stable.

## Required verification and reusable tests

- Run the issue's single, pair and untyped rules at 1,000, 10,000 and 100,000
  vaults under the unchanged default join limit. Observe both answer and
  explanation. The pair fixture with only one spool per vault has no inferred
  pair: also include a two-spool neighbor so correctness is not vacuous.
- Assert the connecting edge is chosen second for the typed A-edge-B topology.
  Cover hybrid groups, predicate/graph variables, repeated variables and authored
  permutations without treating shared constants as variable connectivity.
- For genuinely disconnected components and a head using one component, prove
  additive measured work and absence of an intermediate product. Include an
  empty component, a head requiring both components, and negative coupling.
- Compare canonical conclusions, witnesses and authored premise order across
  insertion permutations, competing proof heights and lexical alternatives.
  Cover assumed confirmations as well as ordinary new heads.
- Exercise zero, tight, exact and ample global limits across multiple rules and
  asymmetric workloads. Verify worker-count-independent results, typed refusal
  and truthful counters. Check guarded row multiplicity and stage barriers,
  including an error after an earlier successful candidate.
- Measure peak allocations and time across the input sizes. Deterministic work
  and allocation assertions establish the growth claim; elapsed timings are
  report evidence, not fragile unit-test thresholds.

Reuse `crates/datalog/benches/seminaive.rs` fanout/frame-width/recursion lanes,
the analytic synthetic corpus, binary-versus-leapfrog and sequential-versus-
parallel comparisons, authored provenance tests, `rule_capacity_limits`,
`rules_divergence`, `srl_closure`, and incremental-versus-full SHACL equivalence
tests. Existing exact candidate-count fixtures must be reconsidered when the
execution law changes; preserve meaningful refusal neighbors instead of freezing
an inefficient intermediate count as the desired behavior.

## Assessment status

The root cause and related implementation debts are source-confirmed. The
assessment is complete as prior-art evidence. Implementation acceptance is
NOT MET: no new source, tests, scaling measurements, final gates or current-head
review were evaluated here. Missing criteria remain unfinished work, not a PASS
with caveats.
