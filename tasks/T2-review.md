# Independent Task 2 review

VERDICT: PASS for the assigned Task 2 scope, including the final retained-state
regression and settled qualification report.

Reviewed the actual uncommitted implementation against parent `4ea5a0bc3`, the
authoritative plan, issue analysis, prior-art assessment and Task 2 design input.
Read the repository laws from the root checkout. No source edits, forge actions,
commits or duplicate builds/tests were performed by this reviewer.

## Production findings

- Certification follows variables in every subject/predicate/object/graph slot.
  Negative outer slots union the touched positive components; conjunction-local
  variables do not. Unowned negative predicates are round-wide preconditions.
  A connected rule and a no-positive rule retain the existing evaluation path.
- Opaque positive or negative callbacks disable this physical path entirely.
  Thus the existing whole-stage order, caller thread, multiplicity, fresh state,
  computed surfaces and reached-error order are preserved rather than assumed
  replayable. Complete callback regressions remain the assigned Task 3 work.
- Factors reuse existing ordered binary operators and certified leapfrog groups.
  Partial source vectors restore authored body indices without applying a
  whole-rule swap program. The existing connected hybrid runtime oracle now
  traverses this factorized path and checks exact authored six-premise proofs.
- Full relations are computed once; mode tables borrow them. A source tuple is
  New iff any premise is in delta. Earlier Old/anchor New/later Full partitions
  eligible products and excludes entirely old products. Empty factors block
  firing, and a delta-addressability check avoids unnecessary Full enumeration.
- Each conjunctive head projects its own four positions over shared factors.
  Empty projections collapse existential dimensions. Necessary head products
  use a heap odometer and emit directly through the ordinary RoundBuffer path.
  The odometer checks exhaustion on every iteration and performs no additional
  head expansion after the existing per-rule allowance is spent.
- Height-threshold frontiers retain both sum-first and pure-lex winners. Global
  height masking uses the raw u32MAX threshold when the saturated proof height
  is u32MAX. Summed minima select pure lex exactly when their global sum
  saturates. Shared source arithmetic and lexical comparison preserve sorted
  premise, authored rule and authored premise tie breaks.
- No alternate public engine, dependency, Cargo feature, governor default or
  authored rule-hash change was introduced. Planner/calculus v2 were already
  assigned to this delivery by Task 1.

No required Task 2 behavior was found missing. The existing assumed-confirmation
selection and round-credit/negative-probe admission gaps are explicitly assigned
Tasks 3 and 4; this review does not claim those gaps repaired or full physical
memory safety established.

## Verification assessed

The settled Task 2 implementation report records the complete Datalog package
run passing 325 unit tests and one doc test, warning-free all-target clippy,
formatting and whitespace checks. The reviewer independently ran only
`git diff --check`, which passed, and assessed the tests' actual assertions.

New tests compare both facts and exact canonical proofs against ForcedBinary and
analytic expectations across all 64 seed subsets, three head projections and
insertion permutations. Additional tests cover all four head positions,
predicate/graph connections, constant/conjunctive heads, every delta lower bound,
negative outer/local coupling, ground preconditions, callback exclusion,
authored lexical ties, masked heights, u32 saturation and synthetic neighboring
u64 saturation laws. Additive one-factor-head work is pinned to 3N steps for
10/100/1000 rows; required multi-factor outputs have analytic cardinalities and
exact provenance assertions. The additional retained-state regression pins 2N
Full rows, 2(N+1) frontier thresholds, and pointer identity of both winners back
to shared Full frames for 10/100/1000 rows; it also pins 2N matching charges.
Existing hybrid, no-positive identity and guard
scheduling tests remain in the passing package suite.

Real release CLI timing/memory at 1k/10k/100k, affected consumer qualification,
wasm runtime, full make-check, hosted CI and final completeness review were not
run for this task and retain their later explicit ownership. Task 2 PASS is not
a completion verdict for the delivery.
