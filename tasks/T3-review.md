# Independent Task 3 review

**PASS: the current-run/current-attempt isolation correction resolves the historical blocking finding.** This disposition qualifies the settled Task 3 source and focused correction evidence, not a hosted campaign, performance result, full suite, PR or merge.

## Independent correction re-review

Reviewed the sole plan, the historical finding below, T3-correction.md, all five
current source files and the actual correction logs. The worktree AGENTS.md is
byte-identical to the already-read protected-main policy; the main .baseline and
.goals remain applicable. No build, campaign, workflow dispatch, source edit, Git
mutation, forge action or agent delegation was performed in this review.

The production workflow now consistently names compiler admission,
all twelve arm uploads and comparison outputs with both github.run_id and
github.run_attempt. Compiler admission downloads select the exact current name;
comparison downloads select only its current-attempt arm prefix, excluding
admission/comparison and every previous attempt. There is no prior-attempt fallback.
The native YAML workflow fixture checks the actual five upload/download identities
and rejects static admission/arm/comparison names and the broad historical pattern.

The actual hosted-restore entry point binds its directory to the current numeric
run/attempt before restoration. In hosted.rs:77–125, restore_at rejects symlink or
non-directory campaign/download/artifact entries, strips only the exact admitted
prefix, validates every case and every destination, sorts the complete twelve-case
inventory and compares it with the admitted inventory before the first rename.
An existing destination (including a dangling alias) refuses instead of overwriting
audit bytes. This is preflight validation of restoration names/types/destinations;
it is not a claim of transactional rollback if an OS rename subsequently fails.
Receipt success, retained-file hashes, compiler/C phases, source/config/tools,
concurrency and compiled coverage remain separately validated by the existing
shared comparison path. A complete directory-name inventory alone never establishes
a successful measured arm.

The real filesystem fixtures cover attempts 1 and 2 with byte-exact old sentinel
preservation, refusal on a second restore, stale or mixed prefixes, an inadmitted
case, missing current arm, existing destination and a downloads alias into prior
evidence. Rejected collections are checked before any case has moved. The current
workflow reports an actionable ALL-jobs rerun when compiler admission is missing;
restoration reports a FULL profiling-workflow rerun for missing/stale/mixed/partial
arm evidence. Failed-jobs-only retries cannot borrow earlier records. Final workflow
status also requires both compiler admission and all actual arm jobs to succeed.

Inspected actual settled evidence:

- T3-correction-tests.log: 22 passed, zero failures/ignored; includes the four
  filesystem restoration controls, actual workflow identity fixture and retained
  shared controller/freshness/coverage/refusal controls.
- T3-correction-clippy.log: strict all-target CAPI clippy completed successfully.
- T3-correction-controller-build.jsonl/.log: actual controller build completed;
  selected executable is an example with test=false, opt-level3, debuginfo0,
  debug assertions and overflow checks enabled. It was rebuilt, not a stale
  existence-based selection.
- T3-correction-operator-refusal.log: the actual selected controller rejected
  attempt2's request to restore the attempt1 path with “restore path is not the
  current campaign.” This is an identity-refusal control, not a hosted execution.
- Current actionlint, changed-surface rustfmt and git diff --check are clean;
  shard self-test/live inventory report all six feature-unified shards and42
  members; toolchain agreement and71-gate workflow parity report success.

No remaining Task 3 blocker was found. Retain the original review's complete-case,
prerequisite, exact-counterfactual, hardware/cache-identity and clock limitations.
In particular, wall intervals remain explicitly uncertified scheduling observations;
no six-runner speedup can be inferred from them. Task4 still must execute matched
actual cold/warm arms and source/config invalidation, show attributable reduction
and preserve complete native/C/downstream coverage. Task5 still owns full
qualification/completion. This source correction does not make telemetry sufficient
to close the issue.

## Historical first disposition (resolved; retained verbatim)

**BLOCKED: current-attempt artifact isolation is incomplete.** Reviewed the sole plan/full contract, settled five-file implementation, Task 3 report, actual focused/static/clippy/operator-refusal logs and earlier controller/shared-helper contracts. No build, campaign, hosted dispatch, source edit, Git mutation or forge action was performed by this review.

## Blocking correction: include run/attempt in every evidence artifact identity

The measured filesystem campaign is `/opt/purrdf-native-profile/$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT`, but workflow artifact names remain static: `native-profile-admission`, `native-profile-${case}` and `native-profile-comparison`. The exact admission download is likewise static, and comparison downloads `native-profile-*`, then restores directories by stripping that static prefix.

On a rerun, these identities do not distinguish new admission/arm/comparison evidence from artifacts retained by the previous attempt. Reusing immutable artifact names can fail uploads; broader downloads can select prior-attempt evidence, whose embedded absolute retained paths belong to another campaign attempt. The current controller's strict readback may refuse that stale evidence, but an implementation that cannot safely retrieve the current attempt is not complete. A retry must neither overwrite prior audit records nor mix their evidence into new inputs.

Use explicit current run-and-attempt artifact names consistently for admission, all twelve cases and comparison. Admission downloads must name only that attempt; arm downloads must match only current-attempt case artifacts; restore parsing must remove that exact campaign prefix and validate the admitted case inventory. Retain prior attempts unchanged. Add focused workflow/restore-selection controls for attempt 1 versus attempt 2 and stale admission/arm names. Re-run changed-source checks before independent re-review. No actual campaign is needed to demonstrate this naming/selection correction.

## Otherwise reviewed hosted behavior

The existing mandatory gates are unchanged in the workflow diff. Profiling is default-false dispatch-only, with twelve complete optional cases: monolithic/C/downstream before, all six shards/C/downstream after and the narrow nested-dev C counterfactual. Target selection still uses existing commands; private copied-source counterfactual changes only the admitted branch. Controller/header-tool builds stay outside cold target/build directories and empty-artifact cold followed by unchanged warm executes through the shared controller.

Actual floating compiler/Cargo admission, hardware/image/concurrency/configuration/tool matching, pinned Node/Binaryen/header prerequisites and actual Python/CC observations are retained. Direct `cargo-capi --version` now requires the pinned real release and captures executable identity; generic help and misleading version strings refuse. Hardware/cache observations are separated from declarations. Heterogeneous runner inputs refuse comparison rather than inventing equivalence.

Failure uploads retain receipts/timing HTML/setup/source/tool/configuration evidence without build caches. Separate artifact downloads are restored to original absolute paths before hash-bound retained-byte validation; this readback design is coherent once names isolate the attempt. Failed, missing or cancelled arms cannot qualify. Actual wall intervals remain scheduling observations with explicitly uncertified cross-runner synchronization; they are not six-runner speedup claims or pure-link subtraction estimates.

Logs show eighteen focused tests, strict all-target CAPI clippy, successful static workflow/shard/profile checks and actual invalid-operator exit-one refusal. These are focused source qualification, not evidence of a completed hosted campaign. Task 4 still owns actual admitted runs, source/config invalidation and attributable reduction; Task 5 still owns full qualification/completion. Telemetry alone cannot close the issue.
