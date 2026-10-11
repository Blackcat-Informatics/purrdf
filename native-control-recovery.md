# Native row control correction

Current runtime follow-up: recovery-exists-owner-aggregate-focused.log completed with exit101. Strict evaluator checks, both new owner regressions, testkit32+7, LUBM5+4, and original cost/governor corpora pass. W3C73 manifest cases pass; four negation bags remain wrong. A closed empty EXISTS had uniform Neither; NOT EXISTS preserved Neither under correct FOUR negation, and a later MINUS therefore retained matching rows.

The existing empty-existential constructor now uses actual search completeness, closure and membership to construct Opposed only for licensed empty absence, and Neither plus undecided otherwise. Explicit EvidenceSource first applies its existing scope guard. Original frame, reasons, aggregate bounds and owners remain transported. Ordinary empty top-level summaries and nonempty explicit Neither/Both witnesses are unchanged. The native producer regression exercises positive EXISTS, NOT EXISTS and nested MINUS over complete/open/incomplete empty invocations, without a source and with explicit closed/open scopes. Runtime pending.

Ordinary trace shifts arose because the unused harness relation registry enabled evidence-preserving EXISTS globally. One boolean is now computed during the original numbered-plan walk, including nested EXISTS and subqueries. The preparation guard reads that whole-plan absence proof in constant time. Unknown plans, bound UDFs, explicit sources and shape providers remain conservative. Existing plan-walk assertions check the bit; the owner regression verifies unused providers admit no control for a query or fork. No trace golden changed. A temporary assertion-report probe prints every differing full ledger/sweep block into the next focused receipt and must be removed before completion.

Reachability ordering source readback: engine.rs PreparedQuery::from_algebra calls admit_algebra_with (property_fn_plan::plan_query) before storing planned.unwrap_or(query) and an empty PlanCache (lines307–370,684–692). The admitted text/prebinding path parses first (1091), then calls plan_query_with_memory (1151), replaces the stored query if planned, and only then creates PlanCache (1198). Prepared registry rechecks refuse algebra that would require changed feasibility planning instead of mutating an already numbered tree (6415–6453). eval.rs prepare_query_context_over numbers the actual final query through that cache, or freshly, before installing it (4984–4988). Raw eval normalizes its supplied algebra before Tree::build_admitted (4695–4703); its BGP dispatcher only calls eval_bgp (4227), while native invocation dispatch requires the explicit PropertyFunction variant (4337). There is no late BGP-to-call rewrite at evaluation. UPDATE similarly plans its WHERE at update.rs621 before numbering/installing at720–721. The existing classifier includes nested expression EXISTS bodies and subqueries in the same walk. UDF bodies remain conservative and install their own actual prepared plan at execution. Scalar host callbacks return terms, with no row-evidence surface. Attached-row operator guards remain independent of this preparation-only proof.

The tracked row and cell controls are constructed only by native admission. `workspace::NativeSharedWorkspace` now holds their original `Shared<SharedWorkspacePayload<T>>` directly. Both attachment controls and unique extraction use that handle. Public `SharedWorkspace` retains caller `Arc` support and delegates native construction to this same admission implementation.

The first runtime layout assertion disproved the assumption that removing the caller/native tag alone would restore the ordinary row stride: the native control became pointer-sized, but RetainedRow remained 48 bytes. SmallVec supplies one zero niche; the three Resident/Shared/Tracked alternatives required an additional discriminant. RowStorage now groups Shared and Tracked in its single Owned alternative, preserving the original two-alternative outer layout. This changes neither payload allocations nor ownership.

Admission still computes the original payload layout, charges before allocation, returns the original typed failure, and drops value before grant. Clone retains the same allocation; mutable access requires the original unique owner. No allocator, reference count, destruction, row/evidence merge law, or caller Arc conversion changed.

## Qualification

- Root independently inspected both the native control extraction and the final Owned enum factoring: [bounded source review PASS](reviews/native-row-control-root-review.md). This is not whole-delivery acceptance.
- `recovery-native-control-focused.log`: terminal1. Ingress20, prepared17, retrieval21 and strict all-three-package all-target checks PASS. Library1509 PASS, one layout assertion FAIL (48 versus40); this is preserved failed evidence.
- `unit_sequence_has_one_empty_solution` asserts pointer-sized native controls and original 40-byte 64-bit row stride. Existing coverage retains owned-default/rank, coalescing, bag cloning/ordering/truncation, resident re-entry and rowless search.
- `retained_row_clone_keeps_the_original_dictionary` covers tracked clone, original query-owner lifetime, unique resident extraction, owned-default/rank update and accepted-row transport.
- `consuming_iterator_drop_keeps_extracted_spilled_row_admitted` now also attaches rank under the original bounded account, clones/drops the tracked owner, verifies grant release by the final owner and exact typed admission refusal with no spare control budget.
- `clone_has_inline_zero_and_spilled_one_allocation` now additionally measures actual tracked publication: two controls, zero allocations on clone, no release by an intermediate owner, and exact original control/spill bytes released by the last owner at both widths.
- Corrected focused collection: `recovery-native-layout-focused.log`, terminal0. Layout40 and original dictionary/rank extraction PASS; ingress20, prepared17, retrieval21, allocation3 and join allocation1 PASS; evaluator/text/retrieval all-target strict PASS19.47s. The mistaken `workspace::tests` exact filter ran zero tests and proves nothing; `recovery-native-layout-grant.log` executes the actual `workspace::owned_row_regressions` test, one PASS, covering the extended bounded grant/refusal/last-owner controls.
- Fixed-work diagnostic, original36 throughput, physical/portable qualification and full gate remain required. No issue completion or performance acceptance claimed.

## Fixed-work result and remaining row computation

`recovery-native-control-bench-build.log` is terminal0: original benchmark candidate built in 3m08s with matching compiler/profile/flags and the existing current diagnostic was relinked. `recovery-native-layout-diagnostic.log` is terminal0, five alternating fresh-process pairs. All retain exactly 1,000 original Q_H queries, 200,000 result rows, original fixture and result destruction inside each call. Counter enabled and running metadata agree exactly. Median baseline 32,191,574.321 instructions/query; median current 32,760,197.279, +1.766372%. Current physical row is40 bytes; current bag is72 versus baseline24. This is instruction evidence only, not throughput acceptance.

The initial `distinct-native-layout-profile-*` profiles also include fixture/warm-up and therefore cannot localize query-only costs. The following `distinct-native-window-profile-{baseline,current}` samples use the original driver's own acknowledged enable/disable protocol and record only its fixed Q_H query window, with zero lost samples. Both full exact counters and sampled instruction totals agree on the increased work. The aggregate sampling shifts toward native row append and BGP transport; sampling alone does not establish per-function exact deltas.

Readback of actual emitted `AdmittedVec<RetainedRow>::push` found an unconditional call to EvalError drop glue on the existing-capacity success path, from `append_capacity`'s eager `ok_or` error. The current projection/transport has changed outlining. A bounded correction now explicitly returns the same error only on failed checked addition; growth and admission remain at their existing homes. The existing original-refusal test also exercises a real nonempty byte vector plus `usize::MAX`, preserving bytes, grant state and unfailed account before the typed overflow. Focused checks, emitted-cost confirmation and fixed-work refresh of this correction are pending; no removed-cost claim yet.

The append correction's focused native controls now pass in `recovery-row-append-focused.log`: workspace ownership58 (including original row grant and new real-vector overflow), ingress20, prepared17, row allocation3, join allocation1 and all19 physical controls. Evaluator/text/retrieval all-target strict passes27.88s. Root's bounded review was updated to cover source semantics and these receipts. Benchmark compilation and subsequent emitted/fixed-work verification are still running; this does not change the performance verdict.

## Confirmed append computation correction

`recovery-row-append-focused.log` completed terminal0, including query benchmark build3m18s and exact-profile fixed-work driver relink. `row-append-lazy-query-bench.asm` and `row-append-lazy-fixed-work.asm` show both actual artifacts now branch directly on available row capacity with no unconditional EvalError drop call on success. The earlier emitted call is retained in `distinct-native-window-profile-current/row-append.asm`. This confirms removal at the actual production computation, not only a source-level hypothesis.

`recovery-row-append-diagnostic.log` completed terminal0: five alternating original Q_H1,000-query pairs. Median baseline32,191,577.729 instructions/query; current30,833,837.549, **4.217688% fewer**. Every raw pair has identical enabled/running metadata. Every workload retains200,000 rows and original query/result-drop boundaries; current row40/bag72 is unchanged by this correction. `distinct-lazy-append-driver-identities.sha256` binds the diagnostic artifacts. These results isolate instruction work and do not qualify elapsed throughput under concurrent host load. Original36 and physical78 timings still require the unchanged acceptance and read-only exclusive admission.

Physical benchmark no-run refreshed in the same existing cache/profile and completed terminal0 in1m26s (`recovery-physical-bench-build.log`). Both timed executables are now ready for one admitted measurement each. The final timing wrapper's benchmark and monitor own dedicated setsid groups; INT/TERM and every EXIT stop/reap their descendants before trying every supplied geo thaw. `timing-lifecycle-smoke.log` exercises those exact functions against only new harmless test groups: TERM143, INT130 and a simulated rejected verdict42 are preserved, and no live benchmark/monitor descendants remain. Root's bounded launcher review PASS covers lifecycle,78x20 sampling, unchanged36 assessor and telemetry refusal; no actual timing or geo freeze has occurred.

## Settled-source portable acceptance

`recovery-native-portable.log` is terminal0 from the unchanged original
`make wasm-test`, run in the admitted8-job/4-test-thread/16GiB/no-swap scope.
All23 invocations complete:110 scalar and15 SIMD executions,125 passed and
zero failed/ignored. Actual scalar consumers include all19 physical controls,
20 evidence ingress cases and21 real producer cases. Runner preflight also
verifies panic, flag, host-read and missing-status refusal. This fulfills the
required portable runtime acceptance on the reviewed source; it does not
establish throughput or substitute for the mandatory full gate.

The single terminal timing-admission check observes unrelated live restic
PID631167 at180%CPU (`recovery-portable-terminal-host-admission.txt`). No timing
benchmark was launched and no geo scope was frozen. The build slot was returned
to root for geo gates. Original36 and authored78x20 physical timing, projections,
the full gate and integration remain incomplete.

## Mandatory helper gate repair

`recovery-native-final-gates.log` retains the original mandatory gate failure:
metadata/36 license profiles,154 claims, workspace strict lint and compilation
passed; the helper census refused12 findings before remaining gates ran.
The repairs consolidate the original mutable-control/equality operations,
error conversion macros, ledger error fusing, character encoding, ASK construction
and fixture scope home. Three exact typed-adapter groups and the existing
constructor-shim group are documented without modifying the detector.
`reviews/recovery-helper-source-review.md` independently reviews each disposition.

`recovery-native-helper-focused.log` is terminal0: census81 enforced jobs and96
exact distinct groups, five-package all-target strict lint,2102 native cases
and60 affected WASM cases pass. Native coverage includes45 core evidence,
466 algebra,1510 evaluator,20 ingress,17 prepared,4 allocation,19 physical and
21 retrieval; portable refresh covers ingress20,physical19,retrieval21. The
original remaining50 scalar and15 SIMD selections have no changed selected
behavior; their125-execution original recipe receipt remains attributable above.
This bounded refresh does not replace completion of the unchanged full gate.

At terminal the read-only timing preflight still observed unrelated restic
PID631167 at181%CPU. No timing run/freeze was attempted. Root received the build
slot; the next R2 build must relink original-profile benchmark artifacts before
any throughput or physical timing qualification.

The relink now passes (`recovery-native-helper-bench-build.log`, terminal0):
query2m27s, physical1m04s and the fixed-work driver, all under the original
benchmark profile. `native-helper-bench-identities.sha256` and
`native-helper-source-diff.sha256` retain their identities. The original timing
launcher then refused at admission, exit40, because a new unrelated restic
PID837270 was live. `recovery-native-helper-matrix.log` records that refusal;
no timing sample or owned freeze occurred, and the private scope readback stayed
running. Both completed public geo scopes are now absent; only root's live
private scope is applicable to a future admitted freeze.

The read-only hash-domain gate also found a stale generated README registry.
Its existing `--write-readme` generator now reports93 valid domains and adds
exactly seven existing evidence/accepted/fused-domain rows, with no runtime
source change. Original refusal and successful generator receipts remain in
`recovery-native-hash-domains.log` and `recovery-native-hash-domains-write.log`.
The metadata/mandatory gate retry is active in
`recovery-native-final-gates-helper.log`; no final gate verdict is claimed yet.

That retry exited2 at the unchanged ASM self-test: the Stage launcher injected
global `RUSTFLAGS=-D warnings`, and the production preflight refused it before
the self-test's expected target-specific failures. Metadata, strict workspace
compilation, repaired helper/hash checks and intervening gates had passed.
No repository source/test/gate recipe was changed. Cargo's effective
`build.rustflags` with global RUSTFLAGS unset is exactly['-D','warnings'] from
the existing user Cargo config (`recovery-native-build-rustflags.txt`). The
unchanged17-case ASM self-test passes with that environment
(`recovery-native-asm-selftest-environment.log`). The corrected Stage launcher
now unsets global RUSTFLAGS, preserving the existing warning policy and allowing
target-specific flags. The unchanged full `make check` is running in
`recovery-native-final-gates-environment.log`; the valid metadata result is reused.

## Mandatory runtime environment diagnosis (current no-fail-fast retained)

The corrected full-gate environment reached workspace runtime. Native temporary
path cases failed because the precreated qualification target/build roots have
no CACHEDIR.TAG; the Cargo-created wasm32 subroots do have their marker. The
original testkit correctly refuses an executable without a tagged cache ancestor.
The LUBM output-admission control also correctly rejects a canonical scratch path
inside main's sibling .worktrees cache, whose effective ignore provenance is
personal .git/info/exclude instead of a tracked .gitignore.

Root verified the exact Cargo 1.100.0-nightly source (commit
7941be6fb416b4cd9666aef7b858dfea25587a8c), cargo-util/src/paths.rs692–733:
create_dir_all_excluded_from_backups returns immediately for an existing root,
so our precreation explains the missing native marker. No testkit, output law,
production body or gate is changed. The full no-fail-fast process is retained.
After it terminates, move only the owned target/build cache directories under
this evaluator worktree's actual tracked-ignored target directory, preserving
old lexical paths as symlinks and every artifact. Establish accurate standard
cache metadata explicitly and verify canonical Git ignore provenance. Root
confirmed geo shares only cargo-home; immediately recheck live users before move.
Do not modify executing launcher/cache paths or call these failures a pass.

## Complete runtime collection and production corrections

recovery-native-final-gates-environment.log is terminal2 after all workspace
no-fail-fast targets and doc tests. Four conformance targets expose real bugs:
cost_planner_corpus, evaluator_trace, governor_correctness_corpus, and
sparql_conformance. Preserved EXISTS OPTIONAL bodies were sent to the ENF spine
mapper, which erases a wrapper that the evidence-preserving preparation retained,
then asserts leaf-versus-two-child equality. Four W3C negation cases additionally
combine native records from independently minted fallback query dictionaries.
Neither failure is waived as an environment problem.

The ENF correction selects the existing full clone map when evidence preserves
the body, retaining the existing ENF map otherwise and the synthetic witness
root's counts_rows=false. The direct OPTIONAL regression checks retained body,
both copy roots, all seven mappings and row-count arbitration. Independent source
review: reviews/recovery-enf-preserved-source-map.md. Runtime pending.

The native owner correction extracts EvidenceInput's existing shared lazy owner
and original adopt/import methods into EvidenceOwner at the same home. Native
contexts initialize it lazily, share its control before worker/UDF child forks,
and use its original dictionary for empty/negative fallback records. Real native
producer row/search ingress uses the same explicit adoption/import boundary.
Foreign-owner refusal is unchanged. No-registry contexts and virtual positive
witnesses keep no evidence control. Direct regression covers those absence laws,
children created before the first record, native producer import and foreign-query
refusal. Source review and coherent compilation remain pending.

Ten other failed targets are native cache environment cases. The original scope
was inactive/dead/empty before recovery-native-cache-containment.sh ran. Receipt
recovery-native-cache-containment.log proves same binary inode/hash, old lexical
aliases retained, canonical evaluator-worktree target/native-build, tracked
.gitignore **/target provenance, and explicitly restored native cache metadata.
Next compilation must use canonical paths: testkit's exact temporary-directory
contract compares current_exe-derived paths with compiled CARGO_TARGET_TMPDIR.
Old aliases preserve artifacts, not a claim that old path-bound binaries pass.
Root owns the next build slot for geo while these bounded corrections are written.
