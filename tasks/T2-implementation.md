# Task 2 implementation and focused qualification

Status: IMPLEMENTED; local checks PASS. Independent review and the root-owned
normal commit/push are not yet performed. This does not complete the delivery.

Parent commit: 4ea5a0bc3. Source changes are limited to
`crates/datalog/src/seminaive.rs` and its new private `seminaive/factors.rs`.
No public evaluator, Cargo feature, dependency, caller governor default, authored
rule hash, or additional planner/calculus version was introduced. Task 1 already
published the changed-observation version v2.

## Implemented behavior

- Runtime-static factors use all four positive variable positions. Negative
  atoms and guard-free negative conjunctions union factors through positive
  OUTER slots; conjunction-local variables do not couple unrelated components.
  Negative predicates without positive outer slots are checked once as ground
  round preconditions. No-positive relational identity retains its original path.
- A rule with any body callback or any callback in a negative conjunction retains
  its original stage-major/multiplicity path. Bindings-only reads do not confer
  replayability. A single connected component likewise retains its original path.
- Each certified factor uses existing binary operators or certified hybrid
  groups in Full mode once per frozen round. Source indices remain authored and
  are restored by body index; no partial vector uses a whole-rule swap array.
  A round whose positive partitions cannot address delta skips Full enumeration.
- Borrowed Full/Old/New tables classify a factor witness New if any source is in
  delta. The first-new-factor decomposition uses earlier Old, anchor New and
  later Full; wholly old firings are excluded. Scheduled model-reading Delta::all
  behavior is preserved by leaving callback rules on the original path.
- Each head independently projects all four positions from the shared Full
  relations. Empty existential projections collapse unused Cartesian dimensions.
  Actual multi-factor output is visited by a borrowed heap odometer, without a
  Cartesian body/product buffer or new rule-width recursion bound. Conjunctive
  heads reuse matching relations while retaining separate canonical winners.
- Each projection/mode bucket stores cumulative height-threshold sum-first AND
  pure-lex frontiers. The global threshold correctly admits raw u32MAX when the
  minimum saturated head height is u32MAX. Global saturated sums select the
  pure-lex frontiers; other sums select the sum-first frontiers. Source height
  calculation and lexical preference are shared with ordinary Candidate emission.
  Authored rule and source-order tie breaks remain in that shared preference.
- New product emission checks the existing governor before owned frame/source
  construction. Full matching is charged once, not again by borrowed mode
  classification. Shared round credits and negative-probe admission remain the
  explicitly separate Task 4 implementation, and assumed confirmations remain
  the explicitly separate Task 3 implementation; neither is claimed repaired here.

## Executed checks

All commands ran in this worktree with at most eight Cargo build jobs.

| Command | Actual result |
| --- | --- |
| `cargo check -p purrdf-datalog -j 8` | PASS, exit 0 |
| `cargo test -p purrdf-datalog -j 8 --lib` (initial existing suite) | PASS, 314 tests |
| `cargo test -p purrdf-datalog -j 8 --lib seminaive::factors` | PASS, first 7 then 9 new regressions; subsequent package run includes all 11 |
| `cargo test -p purrdf-datalog -j 8` (settled Task 2 source) | PASS, 325 unit tests plus 1 doc test; zero failures, ignored tests or filtered tests |
| `cargo clippy -p purrdf-datalog --all-targets -j 8 -- -D warnings` (settled Task 2 source) | PASS, exit 0, warning-free |
| `cargo fmt --all --check` | PASS, exit 0 |
| `git diff --check` | PASS, exit 0 |

The new tests cover all 64 subsets of six seed facts, three head projection
shapes and two insertion permutations, comparing facts and exact witnesses with
the existing ForcedBinary path and an independent analytic oracle. Further tests
cover direct conjunctive heads and every delta lower bound, negative outer/local
coupling, ground negative preconditions, opaque callback exclusion, all four head
positions, predicate/graph connectivity, source-multiset ties broken by authored
premise order, height masking and saturated u32 boundaries. Synthetic u64 source
sum tests exercise exact/neighboring saturation and both distinct frontiers
without pretending to allocate the enormous real premise set that u64 saturation
would require. One-factor heads at 10/100/1000 rows assert exactly 3N charged
steps and N outputs. Necessary multi-factor products assert the full analytic
output and canonical witnesses. A retained-state test at the same sizes asserts
2N Full source frames, 2(N+1) Full/New frontier entries, and actual pointer identity
from both prefix winners back to their single shared Full frame. Existing hybrid
oracle/differential tests pass.

Resolved development failures: the first new acyclic-factor tests exposed that
`RulePlan::join_groups` exists only for a certified hybrid plan. Ordinary factors
now use the existing ordered operators, and hybrid factors use existing groups.
Clippy also required explicit imports and an elided implementation lifetime;
both were corrected without lint suppression.

No full make-check, wasm build/runtime, release CLI performance campaign, hosted
CI, commit hooks, signing, push or forge mutation was executed by this task.
Those checks/actions retain their assigned root/remaining-task ownership.
