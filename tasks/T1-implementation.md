<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Task 1 implementation and focused checks

Status: implementation and assigned local checks PASS. Independent task review,
normal commit hooks, signing, push and progress publication remain with the parent.
This is not completion of the whole delivery.

The approved [plan](../plan.md) and [plan review](../reviews/plan-review.md)
govern this task. Worktree base is origin/main6273b6173. Changes are limited to
`crates/datalog/src/{plan,cache,seminaive,schedule}.rs` and these Stage reports.

## Implemented behavior

`atom_priority` makes a shared bound variable in ANY of the four positions the
first selection criterion. Known-position, constant, repeated-variable and authored
position preferences remain the subsequent deterministic tie breakers. Shared
constants do not connect components. The same `binding_aware_order` loop selects
ordinary atoms and hybrid groups. A cycle connects if any member connects; its best
member supplies the static preferences and its earliest authored coordinate breaks
ties. Selecting a cycle binds all its variables. Certification and the internal
variable descent order are unchanged.

Hybrid operators and premise-restoration swaps are lowered AFTER group ordering.
Authored body coordinates and binding slots remain unchanged. Acyclic plans still
have no resident group sidecar, and the zero/one/two-atom certification shortcut
remains in place. No default, limit, runtime dependency or Cargo feature changed.

Planner and calculus versions are both v2 because changed physical candidate work
can change budget observations/refusal boundaries. Only the five affected frozen
contract/plan pins moved. Both authored clause hash pins stayed unchanged. The
calculus documentation now states accurately that the leapfrog/binary differential
test asserts facts and derivations; it does not assert budget equality.

## Acceptance evidence

All of the following ran in the resolved worktree using
`rustc 1.100.0-nightly (4b6d04e70 2026-09-13)`. Cargo test uses the repository's
optimized test profile with debug assertions and overflow checks inherited from
dev; clippy uses the optimized dev profile. Build jobs were capped at8.

| Criterion | Result and evidence |
| --- | --- |
| Connected edge before a disconnected type | PASS: `plan_sips_connects_before_scanning_another_type` asserts order0,2,1, index shapes, authored slots and restoration for both same-class and different-class constants. |
| Predicate/graph connectivity | PASS: `plan_sips_connects_through_predicate_and_graph_variables` gives each connection priority over an independent atom with more constants and checks the bound-position shape. |
| Existing static ties and acyclic layout | PASS: new repetition/authored-tie test plus existing constant-driver, shape, certification and no-group-sidecar tests. |
| Hybrid bridge/group ordering | PASS: `hybrid_sips_connects_bridges_and_any_member_of_a_cycle` asserts order0,2,3,4,5,1 even though the first cycle atom is unconnected, unchanged cycle coordinates/descent/slots, and exact authored restoration. |
| Exact facts and authored proofs | PASS: `connected_hybrid_joins_preserve_the_analytic_facts_and_authored_proofs` checks both kernels against the analytic triangle facts and exact six-premise canonical proofs over8 insertion permutations. The existing synthetic differential corpus now includes this hybrid. |
| Guard stages and caller context | PASS: `connected_joins_keep_guard_stages_on_the_caller_thread` checks ordinary and cyclic plans through both `evaluate_guarded` and `evaluate_scheduled`, two reached rows, stage-major order, unchanged inputs between stages and exact caller thread. |
| Version and authored identities | PASS: settled cache tests include both unchanged authored clause pins and all updated contract/plan pins. |
| Affected library checks | PASS: `cargo test -p purrdf-datalog --lib --locked --jobs 8`, exit0,314passed,0failed,0ignored,0filtered; test execution2.41s. |
| Package warnings | PASS: `cargo clippy -p purrdf-datalog --all-targets --locked --jobs 8 -- -D warnings`, exit0, finished dev profile in2.61s. This compiles all package targets; only library tests were executed. |
| Formatting and patch whitespace | PASS: `cargo fmt --all --check` and `git diff --check`, both exit0. |

## Resolved development failures

The first formatter check exited1 for formatting in the new code; `cargo fmt -p
purrdf-datalog` corrected it. The first library command exited101 during compilation
with E0506 in the new workload constructor; extracting the cloned head before
replacing `workload.rules` corrected the borrow lifetime. No tests ran in that
failed compilation.

The next library run executed314tests:312passed and the two frozen-identity tests
failed on their deliberately changed v1 values. Narrow cache reruns then revealed
the remaining pins in assertion order. All five new values below came from actual
failure output; the unchanged clause pins passed before the later assertions.
The final settled library command passed all314tests.

| Frozen identity | Observed v2 value |
| --- | --- |
| wasm defaults contract | `3d385bc2c0467c392ba6b46117b8b6cd547982d41b325fbaef367536ea981574` |
| native defaults contract | `2a0bb5e502d94881573555fdc23ff32b6a64d8116e2dd8273efb2829cac24fc7` |
| guarded contract | `a108e586708d4e70e9ea916c45005c041036e32b841b4a614b489ac920ab478f` |
| scheduled contract | `71c1eb49c92901ba68c1ad7953630033d91d8829d702d09a8c5659b026b207cb` |
| plan identity | `bb5cee96a06449cb6d1655d96819d26a704f5d4bb375484c96f8644dd1f42a7b` |

## Scope still pending in the approved delivery

This task establishes static connected ordering and the stated focused invariants.
It establishes neither independent-component factorization/frontiers, canonical
assumed confirmations, shared candidate admission, real CLI scaling, cross-host
qualification nor full physical memory safety. Tasks2–6 retain those acceptance
requirements. No full makecheck, wasm build/runtime, benchmark, commit, push or
forge publication was performed by this worker.
