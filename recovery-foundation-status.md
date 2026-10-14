# Foundation compiler checkpoint

S1 now includes geo main dd79ef77e after an exact named stash preservation and
fast-forward. The pre-geo stash object is retained in recovery-pre-geo-stash-oid.txt;
pre/post layer receipts remain. The only textual conflict was moved geo relation
tests: kept the geographic property test and applied the former three builder
Result adaptations to their actual geo-kernel index test home. The untracked
Datalog dictionary bytes remained identical and the staged layer remains empty.

The first foundation check failed because geo added Hash to StorageError and the
new capacity carriers lacked Hash; derived it on both exact value types. The next
check exposed 54 core errors. Bounded successive compiler-driven corrections
reduced these to 46, 35, 29, 8, then 11 strict/dead-code findings and one final
snapshot caller mismatch. All original failure logs remain with the passing receipt.

The final command `cargo check --locked -p purrdf-lex -p purrdf-core --lib`
completed with terminal 0 in 6.54s. Both libraries now compile under the original
optimized dev profile, assertions/overflow checks and warnings policy. Cargo
created the canonical capacity-artifacts and capacity-build cache roots and their
markers. No runtime or full workspace qualification is inferred.

Corrections preserve storage errors through imports, CDT diagnostics,
canonical/skolem term mapping and freeze, dataset projection/union/snapshot,
pipeline graph/root digest callers, mutable/composite publication, paged term
translation/compaction and segmented construction. Obsolete unchecked import and
arena convenience wrappers were removed. Native protocol events remain dependency
free and now carry structured capacity identity/counts and distinct physical
failures through one core adapter. The actual event open_scope boundary checks
before mutation; its last-slot/repeated-refusal/prior-state regression is written,
not yet run.

Core library test compilation subsequently exposed 122 stale test callers of the
fallible production APIs. Corrections unwrap expected successful fixture values,
keep original semantic assertions, and use the same typed mapper error in the
recursive reference. Success parity now compares successful values, not equal
errors. After bounded retries the full library test binary compiled in 36.56s
(recovery-foundation-tests-reference.log, terminal 0).

Nine exact named runtime controls then passed (recovery-foundation-runtime.log):
the actual event scope counter accepts its last slot and preserves all state on
repeated typed refusal; the shared checked arena range crosses 4 GiB globally
and refuses locally; both arena widths append and resolve real strings through
the shared producer; local/global terminal ID errors and the compact local niche
hold; invalid IRI ingress preserves dictionary state; 300 generated terms retain
recursive-reference rewrite identities and call order. The runner verifies each
exact test exists before execution. These are focused controls, not the complete
boundary matrix or full core runtime suite.

Next: advance to Datalog's real compiler errors under the next bounded build slot.
All remaining full 490/495 behavior, host dictionaries/joins, sparse stack and
acceptance remains outstanding. No timing campaign has been started.

A confirmed reifier rollback defect is now repaired in source, runtime pending:
the implicit rdf:reifies term previously published before a dependent binding
row could refuse admission. The original store-once producer now admits a miss's
row and index buffers, runs its implicit dependency, then publishes the binding.
Hits return before both operations. Resident and governed reifier APIs use the
same producer. The new native_graph_birth_tests regression pre-reserves lexical
capacity, refuses only binding growth twice, checks unchanged original logical
state, then checks successful reuse and a duplicate under a closed growth grant.
The prepared recovery-core-reifier-runtime.sh runs this regression and existing
native graph birth/freeze controls after the numeric owner's lane is released.
This does not yet close segmented accounting or general compound-term rollback.

The prepared reifier checkpoint now passed: recovery-core-reifier-runtime.log
records all four native graph birth/freeze/refusal tests PASS, compile 34.53s and
terminal_status=0. The downstream check first exposed an algebra closure error
type ambiguity, then seven SPARQL caller propagation failures; those are repaired
with original typed storage/IRI errors retained, including fallible public partial
answer reconstruction and its real reasoning caller. The next check advanced
to 82 RDF migration errors (recovery-downstream-check-callers.log). GTS and native
codec ingress now propagate original term/row admission failures in source; GTS
segment-scope refusal retains its stable code/location plus exact typed capacity
data and an actual sink boundary regression. These RDF changes await compilation.

RDF/SPARQL production compilation now passes: recovery-downstream-check-projection-import.log
records cargo check --locked -p purrdf-rdf -p purrdf-sparql-eval --lib, terminal 0
in 6.13s. The original RDF type errors reduced 82 -> 57 -> 29 -> one unused import.
The coherent projection chain now propagates term/row refusal through CSVW,
VoID, DCAT, research-object, LPG and SKOS reconstruction. ProjectionError retains
original StorageError and its exact capacity payload. Native quad-source APIs
and Turtle normalization retain RdfDiagnostic instead of converting it to String;
both flat/folded native source loops check scope width before minting. Fixed
test/bench-only GTS fixtures unwrap their authored successful construction.
RDF library test compilation and actual GTS/projection runtime are next; neither
is inferred from this production compiler pass. Public caller migration outside
these packages, full capacity/rollback census and the full sparse-stack contract
remain incomplete.

RDF library test compilation passed in 42.52s (recovery-rdf-tests-fixtures.log,
terminal 0). Its first checkpoint had six test-only errors, preserved in
recovery-rdf-tests-no-run.log: three incorrectly unwrapped trait forwards, a
stale unwrap on a serializer usize ID, an unhandled union Result, and a private
count accessor used by the new GTS scope control. The scope regression now checks
logical state through the actual frozen dataset. A new projection-error test
asserts original capacity and allocation causes survive direct and RDF diagnostic
adaptation, with their std::error::Error source retained. Inventory captured in
recovery-rdf-test-inventory.txt; 510 affected GTS/codec/projection/native-quad/
normalization/composition cases are queued in recovery-rdf-ingress-runtime.sh.
No runtime success is claimed until that script completes.

RDF affected runtime PASS: recovery-rdf-ingress-runtime.log terminal 0, 511 tests (GTS import 23, codecs 260, projections 192, native quads 9, Turtle 8, composition 19). Original inventory omitted the new GTS boundary test; actual runtime count is authoritative. Five projection freeze adapters subsequently changed to preserve original typed diagnostics through ProjectionError::from, with a direct terminal native-source-scope boundary regression added. That final slice awaits recovery-rdf-typed-freeze.sh; prior 511-case result is not attributed to these later edits.

Final projection freeze slice PASS: recovery-rdf-typed-freeze.log terminal 0, all 192 projection and 10 native-quad cases passed. The source-scope boundary control preserves zero/terminal identities and exact typed next-slot refusal where addressable. The next core slice moves content-ID side-table admission before lexical/term publication; its new actual governed-interner test repeats refusal, checks prior state/index identity, then reuses the same owner successfully. This new core slice awaits its focused runtime checkpoint.

Content-ID admission ordering PASS: recovery-core-content-admission.log terminal 0; 5 native birth tests and 18 content-ID controls (22 unique, new regression selected by both filters), compile 37.17s. Actual original-owner repeated refusal leaves no text/term/index changes and successful retry preserves expected ID. Next coherent source-only correction separates explicit blank scope storage admission from publication; its actual repeated refusal/duplicate/retry test awaits the bounded core lane. Whole compound value/row/merge transactions and segmented counters remain unqualified.

Blank-scope publication correction PASS: recovery-core-blank-admission.log terminal 0, 6 native-birth tests and 46 scope controls (51 unique; new regression overlaps), compile 39.96s. The actual governed blank ingress now publishes a missing scope only after successful term insertion; duplicate identity and owner reuse survive repeated refusal. Existing merge caller publishes its scope explicitly as before; whole-merge rollback is not claimed. Next compiler boundary is recovery-umbrella-production-check.sh (cargo check --locked -p purrdf --lib), prepared but not yet executed.

Umbrella production check FAIL: recovery-umbrella-production-check.log terminal 101, 33 concrete compiler errors (JSON projection 28, MIME projection 1, columnar reader 4). Their actual writer/construction chains now propagate fallible term/row results in source. JSON uses its original typed Storage variant; MIME and columnar now expose the shared typed StorageError with Error::source. Columnar has a direct first-party lex edge (layers row updated) for the shared error/macro, and its freeze adapter preserves capacity while retaining prior malformed-RDF classification otherwise. The lock projection and bounded umbrella retry are prepared in recovery-umbrella-ingress-check.sh but not yet run; no compilation pass claimed.

Ingress compiler progression: full offline metadata updated Cargo.lock only with the new columnar-to-lex edge. recovery-umbrella-ingress-lock-check.log compiles JSON/MIME/columnar, then fails on 14 entailment errors. recovery-umbrella-entail-check.log reduces entailment to 5; recovery-umbrella-entail-consumers-check.log reports one entailment ignored merge result plus five slice errors; recovery-umbrella-entail-merge-check.log confirms entailment production compiles, with only those five slice errors remaining. No umbrella pass yet. Actual entailment proof creation and graph construction now propagate StorageError, proof visitation retains ProofId rather than narrowing its address, and as_graph is Result<Option> so a storage refusal cannot become an absent query. Slice source is corrected through its existing owned conversion (no representation change), preserving typed storage refusals and existing contextual parse diagnostics. Next runner recovery-umbrella-slice-check.sh is prepared, not run. New downstream library test compilation/runtime has not yet been qualified.

Slice production compilation confirmed by recovery-umbrella-slice-check.log; umbrella still fails on five SHACL materialization/union callers. The SHACL source slice now caches Result<Arc<RdfDataset>, ShapesError> at its original OnceLock, preserves original native Arc fast path, and exposes typed StorageError via ShapesError. Data accessors and FocusGraphSource propagate failure; two query runners now return ShapesError rather than inventing absent focus data. Schema union and SRL carry typed cause; shape-file union returns ShapesError. This slice is not yet compiled: query-wrapper propagation from the old String surface remains the immediate compiler boundary. Bounded retry prepared in recovery-umbrella-shacl-materialization-check.sh, awaiting shared Cargo lane.

SHACL compiler recovery: recovery-umbrella-shacl-materialization-check.log failed with eight errors; recovery-umbrella-shacl-callers-check.log with six. Removing the implicit ShapesError-to-String conversion exposed eleven actual typed-boundary diagnostics in recovery-umbrella-shacl-typed-boundaries-check.log. Query/cache wrappers now retain ShapesError and only textual Invalid diagnostics gain contextual prefixes. Lint propagates Storage as an operational error rather than a malformed-shape verdict. SRL document errors carry ShapesError. DatasetImportError conversion calls its existing StorageError conversion home.

The actual Datalog guard protocol was String-only. GuardError now distinguishes StorageError from the original evaluation message; the sole seminaive adapter preserves Storage in EvalError, and SHACL's actual guard plus final rule refusal preserve it in turn. recovery-datalog-typed-guard-check.log captured eight test-only adaptation errors; after correcting fixtures and the shared conversion macro, recovery-datalog-typed-guard-runtime.log is terminal 0 with 354/354 tests passing (2.99s runtime). The existing stage-order regression now also invokes the actual terminal u64 term mint and asserts original space, attempted count and limit, before the later guard's failure.

recovery-umbrella-shacl-guard-check.log remains terminal 101 with two expression.rs cached-scalar callers returning String. No umbrella pass, commit readiness or complete S1 qualification is claimed. The next source slice propagates ShapesError through the node-expression evaluation chain and preserves target-query contextual errors; it is not yet compiled. Original dirty migration and preservation stashes remain intact.

The first expression checkpoint (recovery-umbrella-shacl-expression-check.log) failed on five callers; recovery-umbrella-shacl-target-callers-check.log confirmed those repaired and exposed five further target/preparation/layer-hook boundaries. The callback enum is now named CallbackError and shared by GuardEvaluator and LayerHooks, with one conversion home preserving StorageError and contextualizing only textual errors. The first callback test compile recorded one missing fixture qualification; recovery-datalog-typed-callback-runtime-fixed.log then passed all 355 library tests (4.62s), including the new actual terminal term-mint refusal through assume/retract/finish and no later callback execution.

recovery-umbrella-shacl-layer-check.log remains terminal 101 with two production errors: the validation wrapper and unique-values plan preparation. The next coherent source slice propagates typed errors through the shared constraint evaluation chain and preserves its contextual SPARQL/expression diagnostics. SomeValue retains its specified delayed textual failure behavior but must immediately return a typed storage refusal, which cannot be erased by a later conforming value. This slice is formatted with diff-check clean but not compiled or runtime-qualified. Prepared retry: recovery-umbrella-shacl-constraints-check.sh. Public/dedicated validation and XPath chunk-error adapters are the next traced boundary; no umbrella pass or signing readiness is claimed.

Constraint review corrected an additional typed-cause loss in PreparedValidator::collect_records: a retained semantic candidate failure no longer replaces actual Shapes(Storage). Added a collector regression using actual terminal Datalog mint refusal. Format/diff checks pass; compiler/runtime remain pending the numeric owner's build-lane release. No umbrella pass or new delivery claimed.

Constraints checkpoint session58702 terminated101: six remaining String boundaries (ShapeReporting twice, chunk warm_unique_values, document validation wrapper, rule conditions and targetWhere). Actual constraint propagation compiles; new collector regression remains unrun. Source repair now carries ShapesError through ShapeReporting and the existing XPath RootError chunk envelope, retaining attached native causes and canonical first-error selection, plus conditions/targetWhere/document lint adapters. Prepared reporting retry, not run; shared lane returned to numeric owner.

Reporting check terminal101 with one remaining validate_results_using return type; immediate report-results retry terminal101 confirms typed reporting compiles and exposes14 ignored ingress Results. Source correction now checks the six data-class interning, source blank reservation, projected quad and SRL EDB ingresses, preserving typed Storage and existing SourceContext classification for textual source errors. Eight report/rule serialization ingresses remain for the next coherent slice. Prepared data-ingress compile checkpoint awaits numeric Node lane handoff; no runtime or umbrella pass.

Data-ingress checkpoint session61224 terminal101: three actual projection consumers remain String-only (rules run_rules, entail_dataset, SRL document). Subsequent source slice checks all seven rule-builder writes and their real SRL materialization/compaction/extension callers, retaining storage causes at public projection/rules adapters and existing textual classification otherwise. Report push_triple serialization is still knowingly unconverted. Prepared rule-ingress compiler retry; no runtime claim.

Rule-ingress checkpoint terminal101 at one IriReadError adapter (3 diagnostics). Added typed storage-preserving conversion at ShapesError. Report serializer now returns fallible graph/text values through its original writer and existing collections::try_build_rdf_list; no partial report publication or freeze panic. First report compile found7 qualification/caller errors, fixed. recovery-umbrella-shacl-report-consumers-check.log session32691 terminal101 proves purrdf-shapes production compiles, then exposes6 purrdf-validate errors. Current source fixes retain Storage in complete payload diagnostics, product messages and expression selector (all its real native builder writes checked). Next bounded validate-ingress compiler check pending. No SHACL runtime pass; collector regression and all changed report goldens remain unrun.

Umbrella production PASS: recovery-umbrella-reasoning-ingress-check.log terminal0, cargo check --locked -p purrdf --lib finished0.22s. Previous validate-ingress check proved shared purrdf-validate compilation and exposed one reasoning graph-builder error. The actual surrogate-filtered graph now propagates original diagnostics through both ordinary/governed callers; it replaces the original graph only after successful freeze. All14 ignored SHACL ingress Results addressed. This is production compilation only: SHACL library test compilation/runtime, new collector regression, report goldens, host and portability checks remain unrun. Next prepared checkpoint recovery-shacl-tests-no-run.sh. S1 remains incomplete, #495 host dictionary/sparse stack still unimplemented; no commit/PR/issue closure claimed.

SHACL libtest compilation session28227 terminal101:83 fixture-only type errors, no runtime executed. Authored success fixtures now unwrap fallible report/core materialization and resident IDs; textual refusal assertions explicitly render ShapesError, three complete-rule adapters retain Shapes, and test helper result types are corrected. Production fallibility unchanged. Prepared recovery-shacl-tests-fixtures.sh awaits bounded lane.

SHACL fixture compilation PASS: recovery-shacl-tests-fixtures.log session28626 terminal0,54.23s. 83→0 test compiler errors. Actual binary inventory970 tests retained in recovery-shacl-test-inventory.txt. Test parity assertions now require successful report serialization rather than comparing Result values (which could match two refusals). Runtime is prepared but not yet run; no acceptance inferred from compilation.

SHACL final runtime PASS: recovery-shacl-runtime-error-category.log session97555 terminal0,970/970 tests pass in12.20s. First runtime retained969/970 with one canonical XPath error-category regression. CompleteValidationError::execution now preserves original Invalid-to-Execution classification while retaining typed Storage; actual execution callers use that adapter. Original canonical focus/worker geometry test and new storage-versus-suppressed-semantic collector test both pass without expectation changes. Umbrella production had already passed. Full S1 host, sparse stack, portable and final qualification remain open.

Host arrangement checkpoint: recovery-datalog-host-arrangement-runtime.log compiled with355 native tests passing and one new fixture failing on an invalid row-ID tie-order expectation. Existing secondary permutations sort object/subject, so value order is guaranteed but equal-value rows retain arrangement order. New test now checks monotonic values and exact row multiset; cursor docs clarify that law. recovery-datalog-host-arrangement-runtime-order.log session23027 terminal0,356/356 pass in2.96s. Actual Batch/Tail/Relation/RowCursor/ValueCursor now parameterize TermIdentity with native defaults; tagged high-bit IDs survive batches, consolidation, tail, all four bounds and seek unchanged. RelationStore/dictionary/compiled evaluator integration and sparse stack remain unimplemented.

Host store checkpoint PASS: recovery-datalog-host-store-runtime.log session98217 terminal0,357/357 Datalog library tests in3.08s. RelationStore and addressed partitions now carry the supplied TermDictionary identity; native lexical ingress and direct-ID ingress share row admission/publication. Actual indexed cursor join over host triple/high-bit IDs observes zero lexical reads or substitute minting; lexical partition order, duplicate-at-capacity, and original source refusal with unchanged published rows are verified. These are store/operator controls, not complete compiled-clause or evaluator host integration. Next production blocker is the native-only SlotSolution/SourceRow/constant resolution and error path in seminaive.rs; sparse stack remains unwritten.

Evaluator frame checkpoint: first recovery-datalog-host-frames-runtime.log failed only on two fixture type-inference sites after adding identity parameters. Explicit native fixture types retain the original controls. recovery-datalog-host-frames-runtime-inference.log session36304 terminal0,357/357 pass in2.92s. Existing SourceRow/SlotSolution/SlotValue/PositionValue and planner bound now carry an associated identity, with native defaults; lexical publication and whole evaluator remain native-only. Subsequent binary-kernel source propagation is in progress and uncompiled.

Binary operator source slice prepared (UNCOMPILED): constant_positions and actual extend_slot_solutions/extend_atom use TermDictionary associated IDs, with original typed lookup faults and source validation before/after resolution and scan. Existing binary/hybrid/factor native callers propagate failures through their original EvalError. Added actual authored two-atom plan regression over host IDs and source-loss controls; shared test dictionary moved into existing test_support to avoid a second provider fixture. Prepared recovery-datalog-host-binary-runtime.sh waits for the host package build lane. No host evaluator completion claimed.

Binary checkpoint PASS: recovery-datalog-host-binary-runtime.log session22771 terminal0,358/358 Datalog library tests in5.90s. Authored two-atom operator regression preserves host IDs with4 constant lookups and no row lexical resolutions, and retains exact source loss on empty input/lookup-time loss. Next source slice generalizes existing positive-join delta scheduling and leapfrog path; not compiled yet. Public host evaluation, guards/negative/factor/publication and sparse stack remain open.

Positive join checkpoint PASS: recovery-datalog-host-positive-runtime.log session97264 terminal0,359/359 Datalog tests in5.47s. Actual compile-produced cyclic plan agrees between native binary and leapfrog kernels over caller IDs under full and each single-row delta; source loss is retained during cyclic descent and at empty-delta/zero-governor boundary. Existing native factor callers consume the same fallible operators. Source publication/guards/negative and whole evaluator are still native-only. Next coherent error slice introduces typed EvalError source parameter with native Infallible, keeping physical/logical Storage separate; uncompiled.

Typed source-error checkpoint PASS: recovery-datalog-host-source-error-runtime-inference.log session73260 terminal0,359/359 Datalog runtime tests in6.75s and umbrella purrdf lib check in21.00s. EvalError retains a generic original Source cause distinct from Storage; native dictionary reads use Infallible. Whole host evaluator remains incomplete. Next bounded source slice is negative-atom probing with caller IDs and source checks on skipped/early-return paths.

Negative atom checkpoint session11791 terminal0:360/360 Datalog runtime and umbrella production lib compile PASS. See recovery-datalog-host-negative-runtime.log. Guard model/callback and lexical pin migration is next; no whole host evaluator or sparse stack qualification established.

Guard model/source checkpoint session37648 terminal0:361/361 Datalog runtime and umbrella lib check PASS. Next: SlotSolution lexical resolution must retain the dictionary Lexical owner while borrowed guard inputs are live; currently native surface/bind_surface prevent generalized grouped negation/full rules. Sparse stack remains unwritten. Heavy lane released.

Lexical/grouped-negation slice delivered locally: session61369 terminal0,362/362 runtime and umbrella production compile PASS (recovery-datalog-host-lexical-runtime.log). SlotValue retains original lexical owner; ordinary/group guard callers keep pins until callback return; grouped negative traversal carries associated host IDs and typed Source with first-error semantics. Next blocker is native HeadTerm/HeadKey/Candidate/RoundBuffer and source-fact witness publication; full rule/factor/schedule consumers cannot yet accept the generic dictionary. Sparse stack remains unwritten. No issue closure or final review claimed.

Candidate/publication slice delivered locally: session39144 terminal0,363/363 runtime and umbrella lib compile PASS. Receipt recovery-datalog-host-candidate-runtime-constant.log retains exact first fixture correction provenance. HeadTerm/HeadKey/Candidate/RoundBuffer are generic and original lexical publication/tie-break callers are fallible. Next coherent boundary is emit_solution/RoundSnapshot and evidence capture IDs, then actual evaluate_rule/factor scheduling and generic commit ingress. Sparse stack remains unwritten; S1 uncommitted/incomplete. Heavy lane released.

Host emission/evidence checkpoint PASS: recovery-datalog-host-emission-runtime.log
records 363/363 Datalog tests plus umbrella production library check, terminal0.
The actual emission and evidence publication homes retain caller IDs and typed
source errors, including empty commit validation. Full host-backed factor,
rule/round/fixpoint and commit ingress remain open; sparse-stack implementation
and full #490/#495 acceptance remain open. No S1 commit or closure claimed.

Host factor frontier/projection checkpoint PASS: recovery-datalog-host-frontier-runtime.log
records 364/364 library tests and umbrella production library check, terminal0.
Fallible lexical comparison retains caller IDs and checks empty projections.
Next: factor product/rule/fixpoint execution and original-ID commit ingress.

Host factor/rule checkpoint PASS after one explicit closure error-type annotation:
recovery-datalog-host-factor-rule-runtime-inference.log has 364/364 runtime and
umbrella library PASS. Generic execution reaches the actual projected product and
rule emission. Next compiler boundary is whole-round scheduling, then fixpoint
state and original-ID commit. No S1 commit, issue closure or full acceptance.

Whole-round host scheduling PASS: recovery-datalog-host-round-runtime.log has
364/364 Datalog tests and umbrella library check, terminal0. Mixed original-ID
and fresh lexical ingress is now source-ready behind the H build lane; runtime
and compiler acceptance are pending. Full fixpoint/counters and sparse stack
remain open, and no S1 commit or issue completion is claimed.

Mixed ingress PASS: recovery-datalog-host-mixed-ingress-runtime.log records365/365
runtime plus umbrella library check, terminal0. Both original ingress APIs use
one mixed publication path; native interning rollback encloses dependent
validation. Next is logical u64 counters and generic fixpoint/commit propagation.

Generic fixpoint/commit and logical counters PASS: recovery-datalog-host-fixpoint-runtime.log
records365/365 runtime (2.00s) plus umbrella library check (9.47s), terminal0.
Public host execution reaches actual multi-rule fixpoint and original-ID commit,
with exact u64 logical term accounting beyond 2^40 and empty-program Source refusal.
Host evidence/guard/chase acceptance, remaining capacity/physical allocation
boundaries, sparse stack and mandatory gates remain open. No S1 commit or closure.

Public host guarded/evidenced checkpoint PASS: recovery-datalog-host-guarded-evidence-runtime.log
records366/366 Datalog and umbrella library, terminal0. Actual host callback pins,
Supported/Opposed evidence propagation and first typed source cause are verified.
Full capacity/host ordering/SPARQL/sparse-stack/portable/full-gate requirements
remain open. Generic host chase is not independently required by accepted495
plan; native chase capacity continues under490 and its original controls.

Original host SPARQL DatasetView checkpoint is now2/2 PASS plus umbrella check
(recovery-sparql-host-identity-runtime.log, terminal0). Certified numeric/time
ordering is a source-only narrow DatasetView/expr.rs delta pending its bounded
checkpoint. Sparse stack and the remaining490capacity contract remain open.

Certified host numeric/time order now passes actual SPARQL FILTER execution:
recovery-sparql-host-order-runtime-admission.log terminal0, host_identity3/3
including16 certificate matrix executions, umbrella production compile47.35s.
The new DatasetView method is separate from raw ID/probe order and consumed
only after existing XSD parse and numeric admission. Arc forwards the method.
Unbounded non-latching source errors retain the existing fatal diagnostic; typed
operational ownership and sticky-source cases preserve exact typed causes.
Next confirmed capacity ingress: infallible named-graph declaration Vec push.

Latest source checkpoint: standard-host core named-graph controls22/22 and unminted segmented-ID control1/1 PASS; downstream umbrella+columnar production check PASS. Resident validation/materialization now preserves typed storage failure instead of panic, and segmented seal/certification retains that exact cause. Focused freeze31/31 and umbrella+columnar production PASS in recovery-core-freeze-storage-standard-check.log. Heavy lane released to H. No repository tooling changes; no S1 commit/closure. Next work remains actual capacity boundaries plus downstream test/host/conformance callers and the sparse segmented stack implementation/acceptance.

Capacity integration checkpoint: 27/27 targeted core integration tests PASS in recovery-core-capacity-integration-standard-retry.log. Conformance-kit production now compiles with exact GradeError::Storage through native producers and resident canonical adapters. Latest combined conformance/C/WASM/Python check reduced 31 diagnostics to 18: one conformance path typo and 17 host callers (recovery-capacity-host-production-standard-community-retry.log, terminal101). Corrections are source-ready; runtime/compile acceptance pending. C uses append-only StorageError15 and existing diagnostic JSON; Python storage adapters retain JSON/code; lexical and physical IRI failures remain distinct. Native grading resource failures never become mismatches. Grader/C native capacity controls are written but unrun. No tooling changes or S1 completion claim; sparse stack and complete capacity/host acceptance remain open.

Host/conformance production checkpoint PASS: recovery-capacity-host-production-standard-iri-path-retry.log terminal0, conformance/C/WASM/Python libraries compiled. The preceding mapping retry exposed only six missing direct IRI crate paths, repaired through existing purrdf::iri reexports with no dependency addition. Focused native capacity tests PASS in recovery-capacity-grader-capi-runtime-typed-fixture.log terminal0: C1/1 and grader1/1; both original constructor capacity errors retain native space/count/limit across error adapters. First runtime attempt failed only fixture CapacityError vs StorageError conversions and is retained. Python storage adapter subsequently preserves the existing nullable presentation attributes as well as JSON/code; this small source-only follow-up awaits next host check. Rust lane released to numeric owner. Generated C header and installed Python/WASM tests remain unrun; sparse stack and complete capacity census/portable/full-gate acceptance remain open. No S1 commit, merge or issue closure.

Final Python presentation follow-up compiled PASS in recovery-capacity-python-presentation-check.log (terminal0). Next actual core capacity defect corrected: intern_literal_parts_with_memory no longer publishes invalid_literal before storage admission; original first admitted diagnostic and short-circuit remain. The full native_graph_birth_tests family passes8/8 in recovery-core-literal-marker-admission-runtime-fixtures.log (terminal0), including repeated closed-grant refusal, unchanged empty builder, later healthy freeze and admitted-invalid validation refusal. Initial core compile found two mistaken assertion-level unwraps from the earlier declaration fixture sweep; removed only those unwraps and retained the failure receipt recovery-core-literal-marker-admission-runtime.log. Rust lane released. Outstanding native compound rollback remains: datatype/embedded-blank or owned-quad/merge prefixes may precede later refusal; sparse segmented stack has not been implemented. Host installed packages, generated C header, broad/portable/independent acceptance and whole issue delivery remain open. No tooling changes or S1 commit.

Current checkpoint: owned literal/term/quad/reifier/annotation suffix rollback and exact retained table accounting now run through original Memory. Latest settled runtime is builder43, blank codec27, hash1 and allocator1 passes; core lib strict passes. All original failures retained. See validation.md for diagnostic compacting, typed governed caller audit,120-byte accounting diagnosis and escaped duplicate ownership. Source-only current delta is contiguous blank fastpath plus clone-free validated append, awaiting explicit old-buffer admission at graph_build before its bounded check. S1 remains uncommitted and incomplete; whole merge/document rollback, wide Datalog object permutations, sparse stack and full acceptance remain required.
