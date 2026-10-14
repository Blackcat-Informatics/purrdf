# Issue #531: Evaluator delivery follow-up: relocated test artifact and unqualified performance evidence

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

This records the unresolved evidence boundary of the combined evaluator delivery for #515, #516, #511, #518 and #512. It does not reopen its canceled exclusive-timing campaign.

The user explicitly directed:

> You will NEVER have exclusive timing so remove that work from the list. evaluator branch will be pushed/merged after next round regardless of status, file an issue with any followup work.

## Outstanding finding

The retained older candidate's 36-lane comparison rejected 20 lanes, including DISTINCT at +18.09%. That is an observation about the older artifact, not proof that the final source still has those regressions. Subsequent concrete repairs restore the ordinary row to 40 bytes and remove successful-append error construction. Five alternating fixed-work pairs on the final repaired source measure 32,191,575.742 baseline versus 30,833,871.080 current instructions per DISTINCT query (-4.217578%), with enabled and running counter times equal. No final-source throughput result follows from that instruction result. Reviewed numeric-control comparator and timed-loop bodies were unchanged, so the old control regressions have no demonstrated changed-computation cause.

Final-source throughput and physical scaling timing therefore remain unqualified. The exclusive 36-lane comparison and 78 physical timing executions are canceled acceptance work, not a required follow-up checklist and not passing results. The 19 native physical controls and all 110 scalar plus 15 SIMD portable executions did pass.

## Follow-up scope

Use reproducible observations available on the shared host, with their interference and uncertainty stated, to diagnose any remaining attributable operator cost. Preserve the original inputs and distinguish layout/allocation/instruction evidence from throughput. Correct a production hotspot only when the actual computation establishes it. Do not wait for an exclusive timing window or reinstate the canceled 36/78 campaign.

The final verification round is **FAIL**, terminal status 2. `make metadata`, strict workspace checks and the preceding hygiene gates passed. The complete no-fail-fast native/doctest collection ended with exactly one failed target, detailed below. Because `make check` stopped there, its subsequent preserve-order consumer, final core-hygiene step and nested release-crate `make wasm` were not reached in this round. Earlier portable runtime acceptance is distinct and does not turn those unexecuted steps into passes.

The retained raw receipts and source reviews will be archived with the evaluator delivery's selected Stage evidence. Link the delivery PR and archived evidence after publication.

## Final-round finding: retained cache embeds its old temporary-root path

`purrdf-testkit --test temp` reports six passes and one failure in `the_unit_test_root_is_the_directory_cargo_names_for_integration_tests`. The retained test executable derives `/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence/target/native-build/tmp` from its actual executable location, while its compiled `env!("CARGO_TARGET_TMPDIR")` still contains `/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/build/tmp`. The old path is a preserved alias whose `realpath` equals the current canonical path. The exact lexical assertion correctly fails; physical alias equivalence does not make the asserted contract pass.

The final round used canonical Cargo target/build paths, but reused this unaffected pre-relocation test artifact. Follow-up should refresh the affected artifact through normal Cargo mechanisms under the canonical path and establish that the asserted directory identity agrees, then finish the unexecuted local gate steps. Preserve the test and containment contract; do not weaken it to hide stale metadata. This failure does not establish a query/evidence semantic defect. The entire final gate remains FAIL. Raw receipt: `recovery-closed-empty-final-gates-recipe.log`, failed target at lines 25963–25983 and terminal disposition at lines 28579–28582.


## Comments (13)

### paudley — 2026-10-11T22:45:09Z

Delivery PR: https://github.com/Blackcat-Informatics/purrdf/pull/532. The PR records the final local gate failure and unexecuted suffix separately from passing focused/portable evidence and pending hosted checks. The canceled exclusive timing campaign is not reinstated.

### paudley — 2026-10-11T22:47:17Z

PR #532's first hosted run has two additional demonstrated findings on head `5e1f3658a`:

- [Documentation job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017985): `cargo doc` fails on unresolved intra-doc links `check_plan_matches_relations` at `crates/sparql-eval/src/engine.rs:6534` and `NativeSparqlEngine::admit` at line 6539. Correct the stale references to the actual production admission homes; do not suppress broken-link warnings.
- [MSRV job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017929): compiler E0275 at `crates/shapes/src/schema_surface.rs:3462`, calling `purrdf_core::graph::scc_component_index(graph)`. The solver recursively tries `&SharedWorkspace<SharedWorkspace<...>>: IntoIterator` through the generic shared wrapper. Diagnose the actual generic bound/type inference and retain the supported compiler contract; increasing a recursion limit is not an established repair.

These are failed hosted checks, distinct from the final local round's stale temp-root artifact failure. Other hosted checks are still running, so this is not the final hosted verdict. Raw job logs are retained as `hosted-doc.log` and `hosted-msrv.log` in the selected Stage evidence. No additional implementation or retest round was started under the user's publication instruction.


### paudley — 2026-10-11T22:48:39Z

The [hosted workspace job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017965) also fails on head `5e1f3658a`: Clippy `items_after_test_module` at `crates/rdf-core/src/query_scope/counters.rs:22`. The file places `ConstraintCounts`, `ScopeError` and `ScopeCounters` after its test module. Move the test module after production items without suppressing the lint. The hosted compiler is `rustc 1.101.0-nightly (6f5da406b 2026-10-10)`; the passing local strict receipt used the recorded managed toolchain, so it does not overrule this actual hosted failure.

This is the third demonstrated hosted finding, alongside the documented stale rustdoc links and MSRV trait-solver overflow. All are retained in this follow-up under the explicit instruction to merge the evaluator after its completed round regardless of status. No further implementation/retest round has been started. Other hosted work remains in progress; pending or skipped jobs are not passes.


### paudley — 2026-10-11T22:52:16Z

The first hosted assembly configuration results also expose stale generated/manual projection counts, not a demonstrated scanner-threshold failure:

| Configuration | Row | Recorded | Measured |
|---|---|---|---|
| aarch64 | eval.bgp-join-probe | v3·f0·r0 | v20·f0·r0 |
| x86_64 | rdf.projections | v66·f0·r0 | v68·f0·r0 |
| x86_64 | eval.bgp-join-probe | v7·f0·r0 | v42·f0·r0 |
| x86_64-v4 | rdf.projections | v23·f0·r0 | v28·f0·r0 |
| x86_64-v4 | eval.bgp-join-probe | v2·f0·r0 | v14·f0·r0 |

These three `simd-asm-config` jobs FAIL in [run 38181556067](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067). Retained raw logs are `hosted-asm-aarch64.log`, `hosted-asm-x86_64.log` and `hosted-asm-x86_64-v4.log`. Follow-up must regenerate the projection through the existing generator against the actual source/compiler/configuration and verify all affected configuration rows, preserving scanner thresholds and the measured implementation. Scalar WASM assembly, release-crate WASM and shapes conformance have passed in the same hosted run; other pending jobs are not inferred to pass. No extra implementation/build round was started.


### paudley — 2026-10-11T22:59:18Z

PR #532 merged through normal `ghprsq` as `29661e74bdfa5e760fca5595e72b572176d7dd0e`; all five original issues are closed under the explicit revised shipping boundary. The [selected evidence archive](https://github.com/Blackcat-Informatics/purrdf/tree/b6df69fdffd63ffdd8f3f35f9472e2317e4a302d) contains 1,620 files, including the original failures, reviews, revised scope and exact pre-merge findings. The ordinary [Gitee mirror run](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38182392831) passed; direct remote main readback equals the landing commit.

Additional hosted results observed at landing remain follow-up findings:

- x86_64-v3 assembly projection: `rdf.projections` recorded v24·f0·r0, measured v29·f0·r0; `eval.bgp-join-probe` recorded v4·f0·r0, measured v26·f0·r0. [Job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017886).
- aarch64-neoverse-v1 assembly projection: `eval.bgp-join-probe` recorded v72·f0·r0, measured v121·f0·r0. [Job](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595018240). These join the five previously recorded stale cells; preserve the existing thresholds when regenerating and checking projections.
- Both [pytest](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595018272) and [Python conformance](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017943) report 4,216 passes and two failures: `test_compile_emits_the_sparql_each_stratum_runs` and `test_compile_says_what_a_plan_costs_without_running_it`. Both fail the regex-only `producer_call` helper at `bindings/python/tests/test_retrieval.py:261`, which cannot recognize a generated argument list containing an RDF 1.2 triple term `<<( ... )>>`. Diagnose/update the actual contract test against structured generated SPARQL while preserving the original producer-argument assertions; do not simply delete the assertions. Aggregate conformance and assembly jobs fail because their component jobs fail.

Core/SPARQL/SHACL conformance, release-crate WASM, portable execution, both WASM assembly configurations, Arm build, Miri and two RISC-V determinism lanes now pass. Remaining in-progress jobs and CodeRabbit processing are not passing results. Subsequent findings belong here under the original explicit user direction; no extra repair or timing campaign was launched before merge.


### paudley — 2026-10-12T04:40:31Z

Additional hosted finding after the evaluator merge: the `workspace` and `wasm-exec` jobs on PR #535 fail while compiling unchanged `crates/retrieval/tests/real_producers.rs`. Its `open_admitted` implementations still return `Result<Box<dyn PfCursor>, EvalError>`, while the current `PropertyFunction` trait requires `Result<WorkspaceCursor, EvalError>`; three further return-type mismatches follow. This is a production-consumer test compile failure, not evidence about PR #535's packaging change. The job logs identify `E0053` and `E0308` at the test's lines 2273 and 2314. Update the real producer implementations to the current cursor contract and rerun the affected test plus the hosted jobs. `wasm-exec` stopped at this compile error, so its later WASM execution was not reached.

The same PR's `doc` job also reports unresolved Rustdoc links in evaluator paths (`crates/sparql-eval/src/engine.rs` and `crates/sparql-eval/src/property_fn_eval.rs`); inspect and correct the exact links before claiming documentation qualification. No exclusive timing work is requested.


### paudley — 2026-10-12T04:52:40Z

Additional hosted result on PR #535: the `wasm-package` job completed with three failures in existing optimized-package depth-floor tests. `async-concurrency.test.mjs` measured 135 synchronous levels versus its recorded 182; `query.test.mjs` measured 236 LATERAL levels versus 282 and 134 NOT EXISTS levels versus 181. The npm package run completed with 446 passes and 3 failures. These tests and the affected evaluator source are unchanged by PR #535's packaging-script patch.

The #365 work is measuring the current per-recursion frame cost and repairing the shared evaluator/package behavior. The retained 1 MiB stack region, 64 KiB reserve and 284-group host admission did not change; current optimized package measurements show about 4,080 shadow-stack bytes per LATERAL level and 7,104 per NOT EXISTS level. Keep the original floor assertions in force until a production correction passes them. This is a current-source hosted failure, not a reason to revive canceled exclusive timing work.


### paudley — 2026-10-12T05:01:20Z

Final hosted readback on PR #535 adds evaluator/retrieval failures to this follow-up. The `pytest`, `conformance (python)` and aggregate `conformance` lanes fail two existing `bindings/python/tests/test_retrieval.py` cases (`test_compile_emits_the_sparql_each_stratum_runs`, `test_compile_says_what_a_plan_costs_without_running_it`). The compiled SPARQL text contains a property-function call, but `_producer_call` returns `None` for it; the Python conformance shard reports 4,221 passes, 2 failures, 1 skip and 4 xfails. Diagnose whether serialization or the test parser is wrong and run the real compiled query contract; do not weaken its producer-call assertion.

The emitted-assembly configuration jobs fail stale or changed `rdf.projections` counts on x86_64, x86_64-v3 and x86_64-v4, and `eval.bgp-join-probe` counts on x86_64, x86_64-v3, x86_64-v4, aarch64 and aarch64-neoverse-v1. For example, scalar x86_64 `eval.bgp-join-probe` reads `v7·f0·r0` but measures `v43·f0·r0`; x86_64-v3 reads `v4·f0·r0` but measures `v26·f0·r0`. Inspect the actual emitted functions and reconcile source-backed documentation before rerunning all affected configurations. The aggregate `simd-asm` job fails because these configurations fail. No exclusive timing run is required.

These are failures on main-derived source and are not passing results for PR #535 or the original evaluator delivery. Its tooling patch does not edit the named production/test paths.


### paudley — 2026-10-12T05:32:18Z

Additional measured evaluator stack finding from #365 Task 2 (normal optimized `make wasm-pkg` on the #365 checkout, no exclusive timing): the unchanged historical `query.test.mjs` depth floors still fail. On the current package, nested LATERAL answers through 248 and first refuses at 249 (required floor >=281); the original nested-walk `FILTER NOT EXISTS` answers through 142 and first refuses at 143 (required floor >=180). Both are typed evaluator shadow-stack refusals, not host-depth refusals or traps. The package's 1 MiB region, 64 KiB reserve and 284-group host admission match the setup recorded with the older floor. Separate six-shape production bisections still show identical synchronous/asynchronous endpoints, codes and statements.

Optimized package high-water measurements on repeatable nested shapes are linear: LATERAL 179,724 bytes at depth 40 and 333,964 at 80, or 3,856 bytes per level; simple `FILTER NOT EXISTS` 296,252 at 40 and 429,372 at 60, or 6,656 bytes per level. The original measured floors describe less than 3.5 KiB per LATERAL level and about 5.4 KiB per NOT EXISTS level. A measured extraction of evidence finishing from `eval_evaluated_with` recovered 224 bytes per LATERAL level and 448 bytes per NOT EXISTS level but did not restore either floor. Further bounded predispatch, dispatcher-boundary and EXISTS evidence-only experiments did not materially restore it and were reverted. The historical assertions remain intact.

Source comparison identifies the main durable owner: the older `SolutionSeq` held `Arc<VarSchema>` and `Vec<Solution>`; the current by-value `SolutionSeq` holds `SharedSchema` and `RowBag`. `RowBag` now embeds both a resident/shared `BagStorage` and a `RowEvidence` with identity, optional query owner, search flags and optional rank. The evaluator and its LATERAL/EXISTS operators retain multiple `Evaluated`/`Truncation` results across recursive calls. This wider result representation landed with the evaluator evidence work after the older depth-floor measurements and accounts for the remaining per-level stack growth; exact attribution of each frame's bytes remains unmeasured because the linked optimized module has no usable function names. The next production correction must restore the original floors without raising the stack budget or weakening the assertions, then rerun the optimized package and hosted WASM checks. These results do not establish native/portable or full issue acceptance.


### paudley — 2026-10-12T19:40:00Z

The three full npm stack-depth assertions remain failing on the current retained
optimized numeric package. No stack budget, refusal floor, assertion or tooling
has been changed.

Using the package's existing `queryGovernedAsync` evidence API and the original
depth-control inputs, all18 queries at depths1,2,4,8,16,32 completed:

| Shape | Stack bytes at depth2 | Stack bytes at depth16 | Stack bytes at depth32 | Added bytes per level |
|---|---:|---:|---:|---:|
| LATERAL |34300|91420|156700|4080|
| Repeated NOT EXISTS |44956|144412|258076|7104|
| Walk NOT EXISTS |47772|147228|260892|7104|

The two EXISTS inputs have the same per-level slope from depth2, so their shared
recursive execution path needs diagnosis. These are live stack-byte observations,
not elapsed-time performance claims or evidence that the original floors pass.
The measured package precedes the subsequently committed SCC type annotation,
lifetime elisions and aggregate lint annotation; it has not been rebuilt for
those source-only corrections yet.

Source inspection identifies one bounded LATERAL candidate: `eval_application`
holds its post-left-child/right-side machinery in the frame while recursively
evaluating the left child. An out-of-line finisher, analogous to
`finish_left_join`, could separate that overlap while preserving ownership and
truncation. It has not been implemented or measured, and cannot establish a fix
for EXISTS. Individual optimized frames at `linked.filter` → `exists_prepared`
→ `eval_correlated` still need attribution before changing that path.

The complete package acceptance remains failed; this update closes no requirement.


### paudley — 2026-10-12T20:07:26Z

The earlier left-child extraction candidate was rejected after inspecting the actual failing LATERAL control: it nests through the per-row right child. Extracting the entire post-left body would leave that helper live during the same recursion and could add a frame.

The current source extracts only the final schema/output merge after all right-child collection. It retains the original owners and admission order, adds no allocation, and keeps the original depth floors. Affected strict Clippy checks pass after resolving helper lint failures. Native runtime qualification is running; fresh optimized WASM stack-byte and original depth-floor controls are still pending. This is an unqualified candidate, not a fixed stack-floor claim. The two NOT EXISTS failures remain a separate unfixed path.


### paudley — 2026-10-12T20:30:31Z

The bounded final LATERAL merge extraction passes58 native controls: binop22, correlated ownership3, correlated positive regions3, evaluation stack10 and LATERAL20. Affected strict compilation also passes. This is native source qualification; the optimized-package stack floors remain failed/unqualified.

The attempted unchanged `make wasm-pkg` build was stopped with terminal143 after confirming that its existing recipe invokes a private `/tmp` Cargo target. That conflicts with the standard-directory instruction. My earlier description of that build attempt as standard was wrong. It reached compilation only, did not replace the prior package, and its owned abandoned directory was removed. No repository tooling changed and no canceled run is counted as a pass.

Fresh packaging will use the provided Cargo with its default target directory, select the exact compiler-artifact JSON output, then use the provided bindgen/opt/link commands with the unchanged production flags and original acceptance. No target, cache, PATH or compiler workaround will be introduced. The full unchanged `make check` remains required and allowed: its private-capture self-test mocks files and its nested `make wasm` uses ordinary Cargo directories. The two EXISTS stack-floor failures remain separate unresolved production paths. No timing exclusivity is requested.


### paudley — 2026-10-13T08:08:06Z

# Current #531 stack-floor status

The original optimized WASM depth floors remain **failed** with unchanged stack budgets and assertions: synchronous SELECT answers through 135 levels (required at least 181), right-nested LATERAL through 234 (required at least 281), and nested `FILTER NOT EXISTS` through 133 (required at least 180). These are typed stack refusals, not passing package acceptance.

The LATERAL final-merge extraction, EXISTS branch split, and FILTER no-inline candidate were tested and reverted after fresh optimized packages did not restore the floors. Their native semantic checks do not qualify the optimized depth requirement. Earlier comments proposed that wider result owners accounted for the remaining per-level growth; that is a hypothesis, **not** a measured attribution of the shipped linked frames.

A bounded release-profile diagnostic changed only two original low-stack refusal branches to temporary WASM traps, then removed both probes. On the same compiled module, the pre-link bindgen/`-Oz` diagnostic reproduced adjacent LATERAL 234/235 and NOT EXISTS 133/134 boundaries. It yielded the recurring optimized call-index chain. An independent read of that exact module verified 53 function imports and all 11 reported frame prologues and direct call targets. Those selected reservations are not high-water slopes; pre-link instrumentation is not the shipping package, and Rust source names for the unnamed release functions remain provisional. A separate bench-profile diagnostic was rejected because its Cargo artifact reported `profile.test:true` and lacked the required production export.

The current implementation checkout is clean with the probes and failed candidates removed. The next correction needs an actual recurring source owner and retained-state lifetime established from the linked path, followed by native behavior and unchanged optimized floors. No #531 fix, full npm gate pass, or hosted acceptance is claimed. Detailed receipts and independent review remain in the current numeric Stage under `tasks/T3-531-live-release-stack-attribution.md` and `reviews/issue-531-independent-linked-root-cause.md`.


