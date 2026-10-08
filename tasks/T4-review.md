# Independent Task 4 review

VERDICT: PASS after the required negative-guard expansion correction.

Reviewed actual production source against parent `3912955a5`, authoritative plan,
Task 4 implementation report and current validation. Governing repository laws
were already read. No source edits, forge operations, commits or duplicate builds
were performed. Inspected the settled test/clippy log tails as well as tests.

## Required finding, now resolved

`NegationRuntime::match_guard`, in `seminaive.rs`, iterates every row returned by
`call_guard` and assigns its output values to owned `LocalValue::Computed` values
before recursively visiting the next guard. The enclosing row loop does not test
the shared exhaustion latch.

If the first negative guard's multirow answer is fully admitted, a subsequent
guard can exhaust the remaining credit on the first row. After recursion returns
false, the first guard's loop continues binding/cloning all remaining output rows
although the round has already refused. The recursive charge prevents additional
callbacks, but does not prevent these preceding owned local expansions. This is
inside the evaluator and is distinct from the explicitly excluded caller's eager
answer Vec allocation.

The final source now checks `governor.spent()` before EACH row's local output
assignment and returns the internal false/exhausted result immediately. Re-review
of this correction found no further required fix.

The new `tests/negative_guard_admission.rs` exercises the actual public entry point
with the shared CountingAllocator/CurrentThreadWindow. It preconstructs 128 large
caller output rows, then moves them from a Mutex while measurement is active;
the caller fixture does not clone those rows within the measured window. After a
nested refusal, traffic must include the first actual 64-KiB computed local clone
but remain below four such surfaces. Exact, one-below and zero ceilings also
assert typed total refusal/seeded counts or complete success as appropriate.
This detects owned local expansion, not just callback suppression.

Inspected the documented narrow negative-control log: removing only the new
spent check failed at 8,393,905 requested bytes. Restoring it passes the allocation
fixture. The final correction log records 333 library tests plus this integration
test passing; doc test, all-target clippy and formatting/whitespace checks also
pass. Only the existing first-party alloc-probe dev dependency and its resolved
lockfile edge were added. No shipping dependency or second allocator was added.

## Other assessed behavior

- One atomic successful-credit pool is shared across the round. A separate bool
  represents the single refused reservation; u64MAX retains its final successful
  credit without wrapping. Authored indexed parallel reduction restores private
  buffers, and the pool's final observation/latch overrides local counts.
- Fixpoint absorption transfers the latch and checks budget before committing
  any candidate or derivation. Full-width diagnostics state saturation truthfully
  and do not recommend raising an already maximal ceiling.
- Indexed positive emission, cyclic source capture/output, empty-body identity,
  projected factor products and additional conjunctive heads have admission
  before their touched owned candidate expansion. Existing one-head terminal-row
  charging is retained, preserving the analytic pair-fixture 9N feasibility.
- Single negatives charge probes and subsequent broad partitions. Negative
  conjunctions charge initial local frame, borrowed partition traversal, each
  visited row and reached callback. Global/factor-owned negatives use those same
  routines; exhaustion cannot become a successful incomplete least model.
- Callback answer shape validation admits rows before traversing them. Body
  guard stages retain caller-thread and stage-major execution; exact factor
  witness frontiers and canonical confirmation selection remain unchanged.

No additional required correctness finding was identified in the other reviewed
paths. Existing tests cover asymmetric 32-rule success/refusal at workers 1/4/32,
u64MAX latch and no-commit refusal, 128-head admission, broad empty negative
indices, large body/negative answers and exact/one-below traversal boundaries.
Settled original evidence is 333 library tests and one doc test passing,
warning-free Datalog clippy, and 23 SRL/26 rules/six capacity tests passing.

Eager caller Vec/string and SHACL Producer allocations remain unbounded by this
consumer-admission work. CLI measurements, wasm, wider consumer and full gates
are still assigned subsequent tasks and are not inferred from these tests.
