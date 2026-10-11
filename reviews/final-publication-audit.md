# Stage 3 preparation: final publication audit

VERDICT: PASS for the explicit revised shipping disposition.
Original technical qualification: FAIL/incomplete.

The bounded source audit initially found no new implementation defect. Subsequent
hosted execution establishes the documentation and MSRV findings below; they
remain unresolved and are carried under the explicit user-authorized follow-up.
Task 1's focused functional/source judgment remains in `../tasks/T1-review.md`.
Revised-scope publication and integration are supported under the user's existing
authority; this is not a full original-contract or green-CI PASS.
The final local round is FAIL, terminal 2, with one stale-artifact temporary-path
test failure. Visible follow-up [#531](https://github.com/Blackcat-Informatics/purrdf/issues/531)
records that failure, unreached suffixes and unqualified performance. The user's
decision authorizes proceeding with normal publication after this round; no
additional repair/retest cycle is required by this audit. PR
[532](https://github.com/Blackcat-Informatics/purrdf/pull/532) is now open at
`5e1f3658a`; the normal hooked commit carries the reviewed 49-file recovery
delta unchanged. Hosted doc/MSRV checks now FAIL, other execution/review remains
pending; their distinct assessment is `../review-debt.md`.

## Inputs and authority

Applied the complete completion-adversary rubric, Stage validation and scope
rules, applicable AGENTS, root `.baseline` and `.goals`. Reviewed the full
five captured issues and accepted addenda, authoritative `../plan.md`, its
`../shape-contract-plan.md` annex, current `../validation.md`, existing complete
implementation/source reviews and subsequent bounded repair reviews. The owner
has additionally refreshed all five complete issue/comment surfaces as
`publication-issue-{515,516,511,518,512}.md` and reports no new criteria.

Repository: `/home/paudley/Active/purrdf`; worktree:
`/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence`;
branch: `paudley/515-sparql-eval-datalog-per-row-evidence`. The caller verified
live `origin/main` at `e89e288ef476ed06c80c6593094c4d583d1f5214` is already an
ancestor of checkpoint `ccfac0a5e`. The inspected recovery diff contains 49
files. This is pre-publication source/artifact applicability judgment; there is
no PR-head/hosted merge-tree result yet. A changed integration base still needs
the normal bounded integration assessment, not an automatic full retest.

Read-only execution boundary: no builds/tests/timing, private data, processes,
source edits, commits, forge posts or merges. Only review artifacts were written.
Existing substantive reviews are reused rather than reopening cleared source.
Both root and worktree emergency ledgers contain only their notice and marker.
Repository policy overrides generic skill instructions that would log an
authorized cut there; no `.deficiencies` entry is appropriate or created.

The user explicitly changed scope:

> You will NEVER have exclusive timing so remove that work from the list. evaluator branch will be pushed/merged after next round regardless of status, file an issue with any followup work.

The completed `recovery-closed-empty-final-gates-recipe.log` is that final round.
The exclusive 36-lane throughput and 78 physical timing executions are canceled,
not passed. Any unresolved result is to be recorded visibly in the authorized
follow-up before merge, not hidden by an original-contract completion claim.
This decision does not authorize bypassing mandatory hooks or actual enforced
merge guards. Root's actual protection lookup returns HTTP 404, "Branch not
protected"; no configured required statuses were established. Generic Stage-only
green-CI/no-open-findings holds are superseded by the explicit user instruction.

## Attributable execution evidence and applicability

All paths below are relative to this Stage directory's parent of `reviews/`.
The existing launchers use the pinned native toolchain, locked Cargo, contained
build/cache/temp directories, eight build jobs and four test threads. Reused
execution is adjudicated here; no new command was run by the reviewer.

| Evidence | Command / actual observation / terminal scope |
|---|---|
| F: `recovery-closed-empty-focused.log` | Evaluator all-target strict check; library 1513, ingress 20, prepared 17 and row-allocation 3 PASS. The later native-profile target is 32 PASS/4 FAIL; entire collection is not PASS. |
| C: `recovery-closed-empty-conformance.sh/.log` | `cargo test --locked --no-fail-fast -p purrdf-sparql-conformance --test cost_planner_corpus --test evaluator_trace --test governor_correctness_corpus --test sparql_conformance`: cost 1, governor correctness 1 and manifests 74 PASS; query trace FAIL preserved for reviewed follow-up. Original W3C negation manifest is 12 PASS, no xfail/unmodeled. |
| G: `recovery-profile-trace-focused.sh/.log` | Actual profile identity check, native_profile, original trace generator and both trace/governor targets; terminal 0, repaired native_profile 36 PASS, traces 2 PASS, governor 15 PASS. Existing explicit regeneration cases remain ignored in normal runs. |
| P: `recovery-closed-empty-portable.sh/.log` | `cargo test --locked -p purrdf-text --test physical_shapes`, then original `make wasm-test`; terminal 0, native shapes 19 PASS, scalar 110 and SIMD 15 PASS. Actual WASM ingress 20, real producers 21 and shapes 19 execute, with runner panic/exit/sealed-host controls also passing. |
| I: `recovery-closed-empty-instructions-summary.txt` | Five alternating fixed-work pairs, 1000 identical DISTINCT queries each, median instructions/query -4.217578%; instruction diagnostic only. No throughput or physical scaling acceptance follows from it. |
| R: `recovery-closed-empty-final-gates-recipe.log` | Terminal 2, FAIL. Metadata, strict workspace and preceding hygiene pass; complete no-fail-fast native/doctest collection has exactly one failed target: `purrdf-testkit --test temp`, 6 PASS/1 FAIL. Subsequent preserve-order consumer, final core-hygiene and nested release-crate WASM build are unreached. Latest governor prose recipe is v15 with the checked digest; original failed metadata receipt remains retained. |

F/C failures are discharged only by the corresponding later G repairs/results;
P follows the final ownership, closed-empty and reachability changes. The
original complete source review is supplemented by native row/control, helper,
ENF, lazy-owner/aggregate, closed-empty/reachability and trace/profile reviews.
Those reviews cover the recovery diff's affected semantic homes. Current
profile/trace readback confirms only the two explained empty property-function
node charges changed; no answer digest, original W3C expectation or benchmark
threshold was weakened. Current native-profile source separates receipt fields
from execution admission and its 36-case G result closes the transitive `/opt`
failure; the earlier source-only review alone did not prove that result.

## Criterion judgment

DEMONSTRATED means the named functional behavior has attributable executed
production-path evidence, supplemented by unit properties where appropriate.
It does not imply throughput, full-gate, hosted or merge completion.

| Required criterion | Actual demonstration / observation | Status |
|---|---|---|
| FOUR and independent completeness/frame | Original core algebra/frame/standpoint tests and actual F/P signed ingress, duplicate coalescing and scope queries; R shows current core tests passing | DEMONSTRATED |
| Standpoint inheritance, unscoped and scenario semantics | F/P `actual_claim_ingress_inherits_sharper_standpoints_without_scoping_unspecified_claims`, core frame/diamond/cycle controls | DEMONSTRATED |
| Complete SPARQL and Datalog wiring | Complete prior source review; F/P actual operator/query-form/UDF/VM ingress, C original manifests; unchanged Datalog production semi-naive/evidence controls pass in completed R target output | DEMONSTRATED functional coverage |
| Incomplete join and operational failure precedence | F/P declared source/operator cases; `source_refusal_never_publishes_partial_or_undetermined_rows`; real-producer native refusal/terminal settlement | DEMONSTRATED |
| FILTER, OPTIONAL, EXISTS and MINUS | F/P actual gap, optional and negation cases, including 36 invoked native-search/source-scope/query combinations; C unchanged nested-negation bags | DEMONSTRATED |
| Found paths and partial aggregate bounds | F/P native paths and `group_bounds_follow_whole_scope_and_declared_sum_domain`; F ownerless-group lifecycle regression | DEMONSTRATED |
| Verdict/summary/row/ledger tiers and early stopping | F/P actual producer counters and all query-form tier cases; original supported/contradiction/closed-absence conditions preserved | DEMONSTRATED |
| Queryable content-identified RDF ledger | F/P `actual_unknown_answer_exposes_queryable_original_rdf_ledger`: actual evaluator GROUP BY/CONSTRUCT over retained RDF 1.2 reifiers, shared reasons | DEMONSTRATED |
| Privacy-safe ledger projection | Same real ledger case plus core lazy projection tests; incompleteness and cause remain separate from withheld concern | DEMONSTRATED |
| Ordered accepted merge including fused scores | F/P acceptance case and P `scoped_fused_acceptance_retains_original_two_stratum_scores_and_order`; retained provider counters and original scores/order | DEMONSTRATED |
| Pre-depth field scope inside every stratum | P `field_membership_precedes_each_real_stratum_depth_in_both_schedules`, actual text/kNN and bound-candidate cases | DEMONSTRATED |
| Missing time/place/precision and typed unsupported scope | P text scope/constraint cases plus original real-producer integration; unknown receipts stay enumerable and original operational failures remain typed | DEMONSTRATED |
| Deterministic output and default behavior | P native/WASM frozen/shared cases, C manifests, F unchanged prepared allocation bounds; G current trace/profile identity | DEMONSTRATED functional/default allocation; throughput separate |
| Original default throughput within noise | Historical 20-lane rejection is not current qualification; I proves instructions only; original 36-lane requalification canceled by user | CANCELED acceptance; not demonstrated |
| Admission, clone/last-owner, rowless search, no chosen cap | F/P owner/refusal/row allocation and real-producer controls, native-row source review; 40-byte ordinary row and native control retained | DEMONSTRATED focused physical law |
| S1 host registration/decline/exact contract | P native+WASM `registered_host_and_decline`, host refusal/governor boundary | DEMONSTRATED |
| S2 all shipped shapes and real forms | P `prepared_and_query_forms`, `scoped_streams_and_update_where`, actual dictionary/window/series selection versus literal | DEMONSTRATED |
| S3 exactness/determinism and approximate exclusion | P dictionary/window/series generated differential cases, registration and refusal controls | DEMONSTRATED |
| S4 original evidence and zero-row search scope | P `original_evidence_and_empty_barriers`, `false_expression_gaps_keep_original_traversal`, bounded owners plus F/P ingress | DEMONSTRATED |
| D1 analyzer/VALUES/full bindings | P dictionary projection/aliases, expanded dictionary/UNION and analyzer identity | DEMONSTRATED |
| D2 automaton compile/binding reuse | P dictionary cache/size, analyzer identity and cache refusal keeping previous owner | DEMONSTRATED functional/counter coverage |
| D3 one source pass, 534 and larger dictionary cost shape | Existing source/wiring review and P dictionary cache/size/work-counter controls; original 78 timing requalification canceled | DEMONSTRATED counters; timing canceled |
| D4 random exact dictionaries/documents and hidden aliases | P dictionary property/projection/expanded controls; reviewed corrected phrase derivation bound | DEMONSTRATED |
| D5 explicit longest-first nonoverlap | P `longest_first_reference`, exact distinct logical selection law and retained original matches | DEMONSTRATED |
| D6 no dictionary cap and truthful refusal | P long phrase/determinism, cache/size, bounded owner and cache refusal; prior source cap/admission review | DEMONSTRATED |
| W1 ordinal window/barrier one sweep | P window and barrier properties, original evidence/gap traversal; measured timing campaign canceled | DEMONSTRATED semantics/counters |
| B1 typed bucket series and all aggregate order | P typed-series property, bucket custom/parallel order, original grouping home | DEMONSTRATED semantics/counters |
| P1 real direct/prepared/update/scoped/governed consumers | P real shape/producer routes and F prepared/owner/error controls; complete existing source caller review reused | DEMONSTRATED |
| P2 actual portable consumer and release build | P 125 actual runtime executions PASS; release-wide WASM build suffix was not reached in R | Runtime DEMONSTRATED; full build UNVERIFIED, tracked in #531 |
| P3 strict, projections, full gate and publication | Strict and metadata pass; R whole gate FAIL; normal hooked commit/push and PR creation completed; hosted doc/MSRV FAIL, other checks/review pending | Original qualification FAIL/incomplete; revised shipping disposition PASS under explicit authority |

## Exact remaining disposition

1. R's terminal disposition is now recorded from its actual receipt. Lines
   25963–25983 show `the_unit_test_root_is_the_directory_cargo_names_for_integration_tests`
   comparing the current executable-derived canonical `target/native-build/tmp`
   with the old qualification `build/tmp` embedded by `env!("CARGO_TARGET_TMPDIR")`.
   The owner confirms both realpaths coincide; the exact lexical assertion still
   correctly fails. The final summary names only `purrdf-testkit --test temp`
   and records `terminal_status=2`. This is stale retained-artifact metadata,
   not an established query/evidence semantic defect, and it remains a failed
   full gate. No source/test weakening or fresh discretionary run was used.
2. The user-requested visible follow-up is filed as
   [#531](https://github.com/Blackcat-Informatics/purrdf/issues/531). Its exact
   body is `publication-followup-draft.md`, read for this update. It preserves
   the older 20-lane rejection's artifact scope, current instruction-only
   evidence, canceled 36/78 campaign, final failed target and all three
   unreached gate steps. It calls for normal canonical-artifact refresh in
   follow-up without weakening the assertion and expressly forbids restoring
   the exclusive campaign. This truthfully records the revised delivery; it
   does not retroactively satisfy original performance or full-gate acceptance.
   Carry that link and disposition into validation, PR and squash notes.
3. Normal hooked commit/push and PR publication now completed at PR 532. The
   initial full captured comment/submission/inline/thread surfaces are empty,
   with complete thread pagination; checks and CodeRabbit remain in progress.
   Absence of current feedback is not completed hosted review. Later actual doc
   and MSRV failures are retained in `publication-hosted-findings.md` for visible
   #531 follow-up. Read later actual comments and disposition their findings as
   documented in `../review-debt.md`. No skill-only wait for green CI is imposed;
   normal hooks and actual enforced merge guards cannot be bypassed.
4. Assess the actual integration candidate at merge time, including profile
   identity reconciliation if geo's independent v15 lands first. Use ghprsq,
   verify five closures and mirror/archive results, then remove the completed
   delivery branch/worktree. None is done or counted here.

No additional broad source review, timing campaign, new test suite or unchanged
retest is requested. Final-round/follow-up adjudication is complete; historical
reviews remain untouched. Normal hooked publication and revised-scope integration
proceed under the user's express decision. Do not describe canceled, failed or
pending acceptance as original technical qualification.

The owner supplies the current clean merge prediction
`96e38b2af751b0980afc6be0202b13ec70e1feac` against unchanged base
`e89e288ef476ed06c80c6593094c4d583d1f5214`. Source review applicability survives
the commit because no source delta followed it; the final integration and
hosted requirements remain separate. No routine rebuild or identity audit was
run for this workflow transition.

The actual hosted doc failure identifies stale links at engine.rs:6534,6539;
the MSRV failure is E0275 at schema_surface.rs:3462 through shared-wrapper
iterator inference. These are concrete new findings, not environmental success
or merely missing evidence. The exact summaries/raw logs are retained and the
publication owner confirmed posting to #531 comment 6114757866. Their authorized
follow-up disposition is what permits revised delivery; this audit does not
declare those production/documentation defects repaired.

Final refreshed review disposition: the one CodeRabbit comment is a processing
notification with no substantive finding; submissions, inline comments and
fully paginated threads remain empty. This is pending external review, not
approval. Workspace Clippy additionally fails `items_after_test_module` at
`rdf-core/src/query_scope/counters.rs:22` under hosted nightly 1.101.0; its
finding is recorded in `publication-workspace-finding.md` and posted to #531
comment 6114769020. All three demonstrated hosted failures have visible follow-up.
Readback of actual branch rules establishes no required-status-check rule,
zero required approvals and no required thread resolution, while retaining
signed/linear history and PR/deletion/non-fast-forward restrictions. Normal
hooks and ghprsq remain binding. The actual integration assessment preserves
the reviewed clean tree and Gitee ancestry without a new source delta.
Latest owner refresh reports WASM, scalar ASM and shapes conformance successful;
remaining pending/skipped jobs are not passes. Revised shipping disposition
PASS does not change original technical FAIL.


Final pre-merge append: three additional hosted assembly configuration jobs
(aarch64, x86_64 and x86_64-v4) fail projection-count comparison. Exact five
recorded/measured rows are retained in `publication-asm-findings.md`: aarch64
`eval.bgp-join-probe` v3→v20; x86_64 `rdf.projections` v66→v68 and
`eval.bgp-join-probe` v7→v42; x86_64-v4 `rdf.projections` v23→v28 and
`eval.bgp-join-probe` v2→v14 (all f0/r0). These are demonstrated stale
projection counts, not an established scanner-threshold defect. The owner
confirmed visible follow-up in #531 comment 6114798613, retaining all three
raw logs and requiring generator-based reconciliation without weakened thresholds.

The latest captured PR comments contain the unchanged bot processing notice
and the owner's truthful final notes; neither introduces a substantive review
finding. Refreshed review submissions and inline comments are empty. No
external approval or completed bot review is claimed. Revised shipping
disposition remains PASS under explicit user authority; original technical
qualification remains FAIL/incomplete. Normal hooks and ghprsq guards remain
binding. No additional CI wait, per-failure cycle, source edit or test is imposed;
later outcomes remain assigned to #531 under the user's blanket follow-up
direction. Review artifact writers stop after this append for final capture.

