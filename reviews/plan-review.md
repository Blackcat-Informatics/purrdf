# Independent governor foundation plan review

VERDICT: PASS

Focused recheck: the appended `Acceptance clarifications from independent plan review` resolves R1-R3 below. R1 is DISCHARGED by the same-environment A/B, 32-worker improvement/pre-block-envelope criterion, ungoverned control comparison, full grouped values before timing, and explicit <= 1.2 median ratio at 4 and 32 workers with uncertainty. R2 is DISCHARGED by the named existing suites and explicit source-order refusal/error, ghost-mint/resume, stop-reporting, boundary-neighbor and physical-work/peak witnesses, plus unchanged aggregate reduction geometry. R3 is DISCHARGED by the actual shipped CLI result/receipt demonstration and correction of the prose when cell accounting deliberately keeps OPTIONAL sequential. The plan's original alternative remains authorized by the issue; no additional owner decision is needed. This recheck covers those substantive additions only and does not reopen unchanged scope. Historical findings and suggested rewrites are retained below as the review record. Implementation acceptance remains pending.

Reviewed plan: `/home/paudley/Active/purrdf/.worktrees/478-sparql-governor-governed-parallel-row/.stage/sparql-governor-governed-parallel-row/plan.md`.

Inputs: this Stage's `issue.md`, `raw/issue-469.json`, `prior-art.md`, and `validation.md`; the root `.baseline`, `.goals`, supplied repository AGENTS instructions, and focused Stage 1 planning/completion review rules. The root baseline/goals are untracked and therefore absent from the new worktree; their root copies were read. The prior-art report says the issue cites no governing ADR. No forge retrieval, tests, builds, benchmarks, source edits, ref changes, or sibling mutations were performed. This is a review of the plan, not a demand for already-executed implementation evidence.

The combined scope and three behavior units are coherent. The 1.2 metered-overhead threshold, actual CLI OPTIONAL check, ordered trips, scratch bound, normal hooks, and protected integration are retained. The issue expressly permits either implementing OPTIONAL's claimed fork or correcting its inaccurate claims; choosing the latter is an authorized implementation choice, not a scope cut requiring a new question.

## Source findings that inform implementation

- `crates/sparql-eval/src/parallel.rs:1318-1394`: `par_loop_try_map_init` currently selects ordered blocks for any governor state, not only bounded fuel/scratch. `ORDERED_BLOCKS_PER_THREAD = 64`; the current formula creates 8-row blocks for 16,384 rows at 32 workers. Each block initializes and harvests a context. Distinguish an actual caller ceiling from METERED's bookkeeping engagement when deciding whether ordered bounded blocks are necessary.
- `crates/sparql-eval/src/row_checkpoint.rs:398-583`: workers share spend periodically, stop at a shared horizon, and hand ledgers back. Ordered commit at line 670 replays admissions and intra-item charges; arena ghosts and resumption preserve sequential consumption when workers count duplicate mints. Changing block geometry must preserve these mechanisms rather than replacing them with scheduler-order charging.
- `crates/sparql-eval/src/parallel.rs:180-212,1473-1520`: within-group reduction has a deliberately fixed, target-independent chunk plan. Worker-dependent scheduling blocks are a different concern. Changing the aggregate reduction geometry would change combine/rounding or charges on existing aggregates and is not needed to fix row-loop scheduling.
- `crates/validate/src/governors.rs:146-173` and `crates/sparql-eval/src/binop.rs:1717-1731`: CLI flags start from METERED, so `cell_row_ceiling` is Some on the non-empty OPTIONAL output schema; that independently prevents its predicate loop from forking. `may_fork_governed_loop` at `eval.rs:2024` already distinguishes real caller ceilings from bookkeeping for other dimensions. Merely changing that general gate does not remove OPTIONAL's separate cell check.
- `crates/sparql-eval/benches/numeric_eval.rs:159-166,224-256`: grouped governed lanes currently assert only Complete, while the plain lane checks the number of groups. An incorrectly reduced but Complete answer could appear as a metering speedup unless the revised acceptance compares results before timing.

## Required plan revisions

### R1 — Make the performance and answer criteria falsifiable

Ground: UNFALSIFIABLE CRITERION. Task 1 currently says, "Acceptance: ... governed_eval/row_loops_16384 A/B including ungoverned controls." A/B execution alone can pass while retaining the regression. Task 2 correctly retains "metered overhead at most 1.2 times ungoverned," but does not say that the timed lanes do the same work and return the same values.

REWRITE TO: "At 4 and 32 workers, run the existing prepared-query `governed_eval/row_loops_16384` filter/bind stop-only and fuel-plus-stop lanes against unchanged main, with the same compiler/profile/dataset and ungoverned controls. Record before/after medians and uncertainty. The revised 32-worker affected lanes must improve against unchanged main, while 4-worker lanes and ungoverned controls show no material regression. Before timing `numeric_eval_group`, compare complete solution bags and numeric values for ungoverned, fuel, scratch and METERED lanes. At both 4 and 32 workers the METERED/ungoverned median ratio must be <= 1.2; report the ratio and uncertainty rather than substituting a faster incomplete query. Record results and conditions in docs/BENCHMARKS.md."

This does not add an arbitrary new overhead threshold; it retains the supplied 1.2 contract and makes the requested regression repair observable.

### R2 — Name the checks behind ordered trips and the bounded-work claim

Ground: UNFALSIFIABLE CRITERION. "Preserve ... the bound of roughly one ceiling plus in-flight rows" and "targeted governor tests, thread-count comparisons, boundary trip and scratch tests" do not identify the witness or observation. The relevant executable checks already exist and need not be replaced with a new testing framework.

REWRITE TO: "Use `row_checkpoint_gate` and `numeric_parallel_determinism` to compare the direct sequential reference with production FILTER, BIND, GROUP BY and fuel-only OPTIONAL at 1, 2, 8 and 32 workers. Compare the trip dimension/limit/consumed value, all consumed dimensions, certified answer/prefix, and expression-error counts. Cover row-admission refusal, refusal inside an expression, duplicate worker mints requiring ordered resumption, exact answering/refusing neighbors, and chained row loops. Retain once-only stop-work reporting and pending-work drain checks. Extend the existing `a_forked_loop_does_about_one_headroom_of_work_on_sixteen_workers` witness to the required 32-worker geometry; record the actual additional work allowed by sharing intervals and in-flight rows. Exercise growing large-value rows under small and larger scratch ceilings and measure peak allocation with the existing allocator probe, distinguishing fixture/retained output from worker workspace. Document the bound and measure that block-size tuning does not restore worker-count-multiplied overshoot. Scheduling-block tuning must leave within-group aggregate reduction chunks unchanged."

Exact headroom zero, very small headroom, and oversized single-row work should be included where existing boundary tests do not cover the changed branch. This is executable coverage selection, not a demand for hypothetical proof before implementation.

### R3 — Make the CLI OPTIONAL disposition concrete

Ground: UNMAPPED REQUIREMENT within the stated "actual CLI fuel OPTIONAL witness": the plan leaves the two authorized alternatives open, without stating how the chosen one passes.

REWRITE TO: "Choose and record one authorized OPTIONAL disposition. If preserving the present cell-bounded sequential production path, correct CHANGELOG and governor-profile descriptions to state exactly which governed loops fork and that CLI --fuel uses METERED cell accounting; demonstrate the representative 200,000-row CLI OPTIONAL query under --fuel and ungoverned control and preserve its answer/cell observations. If implementing the fork, make the CLI's actual METERED fuel path reach it while explicit intermediate-cell ceilings retain deterministic pre-allocation refusal; prove answer, trip, consumption and cell identity against the sequential path. A fuel-only internal test cannot establish the CLI fork claim."

## Disposition and scope

No pre-emptive descope or planned-by-refusal finding. No additional product decision is required: the owner may integrate these rewrites using the existing authorization. Once R1-R3 are addressed, recheck only their dispositions. All implementation and performance acceptance remains pending; the plan review itself needs no code execution. Preserve snapshots, old receipts and sibling worktrees as specified by the plan. Retain source cleanliness separately from ignored Stage evidence, normal hook verification, and ghprsq-only final integration.
