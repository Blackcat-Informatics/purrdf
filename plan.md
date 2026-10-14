# Exact-only numeric and temporal delivery: #470

## Contract and baseline

The primary issue removes the bounded public integer/decimal types, makes the
exact tower the only numeric value/API path, changes default division to exact
when terminating and otherwise 18 fractional places rounded half to even, and
makes temporal seconds exact. This reaches Rust, SPARQL, CLI, Python, C, and
sync/async WASM; the governor profile and vectors, CHANGELOG Breaking section,
English book, and zh-Hans translation must agree. Explicit `exact`, `N`, and
`N:ROUNDING` policies remain available.

The companion #443 optimization contract remains required in a separate
delivery. Its small-value parity, subquadratic algorithms, SIMD and assembly,
allocation/GCD, cost and end-to-end performance acceptance are not #470
closure criteria. The unqualified Toom/parser work removed from this checkout
is retained as `tasks/T4-443-source-before-split.patch` with its prior receipts;
its selected surviving checkout is the existing clean
`/home/paudley/Active/purrdf/.worktrees/v4-decimal-defaults` donor on
`paudley/423-v4-decimal-defaults`. The patch and four T4 evidence files were
copied byte-for-byte to its `.stage/443-preserved-kernels/tasks/` donor evidence
area, with originals retained. That area is not yet a stagectl-created #443 run
or implementation qualification. Use that single checkout for #443 rather than
creating another implementation branch. This division
follows the current authorized shipping boundary; it
does not close or descope #443.

`.goals` calls for first-party Rust, maximal performance and wasm portability;
`.baseline` requires no silent incomplete work. The issue allows no new external
dependency without approval. The user removed exclusive timing campaigns from
the delivery list. Instruction-count work may use the available nonexclusive
machine; elapsed-time wins are not asserted from contended measurements.

This new #470 worktree is the sole implementation checkout. The old
`paudley/423-v4-decimal-defaults` commit is unreviewed donor code, not a passing
delivery; transfer useful units only after comparing against current main. The
#422 donor concerns a closed issue and may provide cases but is not copied
wholesale. Delete obsolete donor checkouts after verified transfer. Do not edit
S1's core/lex/Datalog/dictionary homes; coordinate shared evaluator/host files
and generated projections before editing them. `main` stays protected.

## Prior art and design boundary

Current main already has an inline exact `Integer`, three inline `BigInt`
limbs, bounded scratch, Karatsuba, exact costs, and the existing SIMD manifest.
It still defaults to truncating 18-place division, retains bounded numeric APIs
and temporal seconds, and uses the hybrid bounded evaluator path. The donor
implements much of the #470 core but explicitly leaves SPARQL assertions,
bindings, docs, and performance comparisons unfinished; its source predates the
geo and evaluator merges. Reconcile both sides at each production home.

No foundation-only precursor or default-only closure is acceptable. #470 may
ship independently only when its entire exact-value, default-division,
exact-temporal, host, profile, documentation and repository-gate contract
qualifies. #443 keeps its own complete contract and cannot be counted from
this delivery. A passing #470 leaf test, commit or package is not issue closure.

## Task 1: Reconcile exact-only XSD value and division law

Transfer the donor's useful XSD value and division logic into current main,
preserving newer geo/evaluator ownership and costs. Its temporal change is
insufficient: `temporal/seconds.rs` retains a private `Fixed { i128, u8 }` and
18-place ceiling behind exact public accessors. Replace the retained bounded
seconds storage and arithmetic with exact representation and tests on actual
sub-nanosecond and greater-than-18-place values. Remove bounded public
variants and conversions only when every production consumer is updated. Add
tests for terminating/nonterminating signs, ties, large coefficients/scales,
explicit policies, exact temporal arithmetic and canonical lexical identity.
Compile the XSD leaf and its immediately affected callers after coherent API
slices. Governed small and large operations must complete under adequate
declared work/scratch, and one-below work or storage admission must refuse
before output/owner mutation. Review the actual diff, commit normally with
hooks, and push.

## Task 2: Complete all numeric hosts and documentation

Propagate the exact-only API and division default through SPARQL evaluator,
CLI, Python, C, and sync/async WASM. Exercise installed/package entry points,
not only internal helpers. Bump and regenerate the governor profile/vectors
without rewriting older profiles. Update CHANGELOG Breaking, English book and
zh-Hans together with actual API signatures. Qualify explicit policy opt-ins,
error mapping, temporal seconds and deterministic native/WASM results. The
cross-host table includes a terminating quotient longer than 18 places,
`2 / 3`, an explicit half-even tie, and a long fractional temporal second,
with equal lexical values and typed errors. Review, commit with hooks and push.

## Task 3: Complete #470 qualification and integration

Run exact oracle properties, governed real-query admission/refusal controls,
the frozen governor profile replay, native and portable numeric determinism,
actual installed Python and C, CLI and synchronous/asynchronous optimized WASM
package tests. The cross-host table compares the same terminating >18-place
quotient, default repeating quotient, true half-even tie, explicit `exact`,
bare `N` and `N:ROUNDING`, long temporal seconds, and typed errors. Verify
current artifact/profile identity in each host. Regenerate and verify the
conformance document through its normal full matrix; finish metadata,
translated-book and documentation claims on final source. Run the required
repository gate once after focused repairs settle, plus applicable release
WASM/portable checks. No exclusive timing is required. Obtain independent
final-source review against all #470 requirements, reconcile current main
without discarding either side, and record any actual failing gate. A narrow
suite or checkpoint commit is not broad acceptance.

## Task 4: Publish and merge #470

After #470's entire contract passes, create a PR that closes only #470,
resolve review findings, satisfy hosted checks, post final notes and merge only
through `/home/paudley/stage/root/bin/ghprsq` with selected Stage evidence.
Verify the issue closure and ordinary mirror, recheck the already transferred
#443 donor evidence before removing this worktree, then clean up this completed
delivery branch. Do not claim #443
completion or use a hook bypass, unreviewed donor bulk merge, silent scope cut,
or exclusive timing campaign.
