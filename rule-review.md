# Independent review

Reviewed the whole-rule continuation changes against HEAD, including the new
`crates/datalog/src/seminaive/rule.rs`, production dispatch and test-only reference
in `seminaive.rs`, shared factor filter visibility/callback changes, and whole-rule
differential tests in `seminaive/ordered.rs`. Also inspected prepared immutable
runtime commit `610f3f425` in `plan.rs`, `cache.rs` and `seminaive.rs`, plus the
existing ordered, guard, factor execution and governor implementations they call.

**Verdict:** no actionable correctness finding in this delivery's inspected scope.

- Production rule evaluation uses the new frame. The previous whole-rule phase
  sequence remains test-only. Positive, guard, signed-filter and head-emission
  barriers preserve the original ordering, including withholding ordinary heads
  until filtering finishes.
- Pending filters, callback batches, solutions and head ordinals remain owned
  across credit refusal. Additional heads charge before emission; a refused head
  retains its ordinal. Completed factor output transfers once. Source authority
  checks and fallible native/evidence calls propagate errors through `?`.
- Prepared runtimes derive from the same immutable rule/plan pairs and are shared
  through `Arc`; evaluation uses those prepared runtimes. No new platform-specific
  dependency, recursion or unsafe code appears in the inspected changes.
- New tests compare original bounded prefixes and resumed complete results at
  every credit cut and one-credit pages, for triples/quads and planned/binary
  execution. They compare callback/probe counts, charged work, confirmations,
  generated-term metadata and canonical signed evidence, including conjunctive
  heads and negation guards. The source-loss test reaches the whole-rule wrapper.

No additional blocking test requirement was identified. A useful supplemental
failure-path test would drive the whole-rule wrapper through a successful first
guard invocation, pause, and then a later callback error, asserting the typed error
and unchanged invocation count. The ordered-frame test already covers this inner
failure path; the whole-rule wrapper's direct propagation was inspected.

Limitations: this review did not execute tests, builds, clippy, wasm checks,
downstream entailment tests or hooks. Their results must be recorded separately.
The frame is still discarded by the ordinary rule wrapper after a bounded call;
public continuations, retained round/session ownership, transactional publication
and the full parent contracts are outside this internal delivery and remain open.
No performance improvement or allocation bound is established by this review.

## Fixture correction review

Fresh native qualification reported failures in both whole-rule controls with
`SourceLost(0)`, also reproduced by a targeted run. Earlier logs are historical
and do not establish acceptance of the current tree.

Inspected the subsequent `ordered.rs` fixture correction: original row identities
0, `u64::MAX`, 10, 11, 20, 21 and 77, plus callback-produced `WIDE`, now have valid
term spellings in the fixture dictionary. Together with its existing predicate
and graph spellings, these cover the IDs present in the fixture rows and returned
by its callbacks. This is consistent with `emit_solution` rendering both the head
fact and source facts when confirming an assumed row. It restores valid fixture
source authority without changing the production path or swallowing its error.
Body-only no-resolve assertions remain in place.

No new correctness finding from this correction. Fresh tests and the remaining
qualification are pending; this review did not execute them.
