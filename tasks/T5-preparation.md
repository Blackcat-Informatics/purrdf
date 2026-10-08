# Task 5 source inspection and measurement preparation

Planning evidence only; no collector, performance campaign or release CLI run.

The real CLI dispatch is reachable through public purrdf_cli::run. The shipped
main is only a call to it. Success returns; failures/governor outcomes exit with
their actual status. A dev-only allocation probe can call this same entry point
with ordinary process argv and install the existing CountingAllocator around
the whole pipeline, then write WholeProcessWindow results after successful return.
This measures actual dispatch rather than reproducing private run_rules logic.
Keep observer runs separate from production release-binary latency/RSS runs.
CLI already has dev alloc-probe/testkit and native libc; no new shipping
dependency, feature or production allocator is needed.

Exact command remains purrdf rules --srl RULESET --explain=PROOF INPUT OUTPUT.
An .nt output supplies the carrier selection. Validate all output facts and every
authored proof premise, not only inference counts or the presence of proof text.
Existing CLI integration tests document proof-text grammar and file capture;
crates/shapes/src/srl/eval.rs::Inference::proof_text is the production grammar:
derived triple, rule identity, then every premise in authored body order.

Use the issue namespace https://example.invalid/k# and original seven facts per
vault, two Store targets plus one Spool. At1k/10k/100k, single and notype inferN;
pair infers0. A two-Spool neighbour has genuine positive pair output. It should
be sized explicitly to fit unchanged defaults: doubling Spool fanout increases
the connected candidate count, so a positive neighbour is not a claim that the
100k original acceptance corpus has the same work count with different data.
Include independent-factor allocation/work measurements through the same
production evaluator and existing fanout/frame/recursion bench axes.

Generate and collect with original Rust tooling. Preserve identities, exact argv,
child statuses, host/load observations, elapsed duration and actual peak RSS.
Allocation requested/retained/peak bytes are distinct from process RSS. Shared
alloc-probe already supplies resident_kib; a child peak needs an actual OS
measurement rather than inferring a peak from an end-of-run sample. Task-owned
disk under/opt has309GiB free atpreparation; refresh admission before campaign.
Coordinate one performance campaign with root, suspend other owned heavy builds
during timed runs, cap inherited Cargo jobs8, preserve sibling processes/caches.

Affected CLI suite is crates/cli/tests/shapes_tools_cli.rs; public shapes suites
and Datalog unit/doc/all-target checks have focused earlier evidence. New source,
actual CLI scaling, affected entailment consumers, wasm build/runtime and one
settled full gate remain required; source inspection establishes none of them.

Task5 correction to the earlier Task4 analytic feasibility claim: the original
five-atom guarded pair at N=100 actually charged 27N before optimization,
not 9N. The initial last-new-atom variants scanned prefixes costing
(1+4+5+8+9)N; only the last could complete, because every row was in delta and
every earlier variant required an empty OldOnly suffix. The focused public
evaluator control and its exact canonical proof neighbor are retained in
T5-logs/original-pair-prefix-accounting-settled.log. A narrow exact snapshot
guard now skips those provably impossible variants in both physical joins.
Settled work is 9N for the original one-Spool pair and 18N for its positive
two-Spool neighbor, including two admitted distinct-assignment head emissions
per vault. Later partial deltas retain all anchors. Limits were not raised.
This repairs the source-level feasibility error; actual 100k CLI acceptance
still requires the separately admitted release campaign.
