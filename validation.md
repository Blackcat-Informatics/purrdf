# Current acceptance verdict

All implementation, native acceptance, strict caller, hook, hosted-check,
current-head independent-review and integration prerequisites pass for a010ace7b.
CI run 40076759527 completed successfully: 42 jobs passed and four were skipped
by existing configuration. All 53 PR checks are terminal with success or
configured skips. The actual CI watcher exited with status 0 and was collected.
No owned artifact writer remains live. All nonempty command logs have frozen,
byte-identical text copies for archive.

Independent source review explicitly covers this head, with low risk and no
correctness/architecture finding. Current API capture includes one issue comment,
one initial-head review submission, zero inline comments and zero review threads.
The final-head judgment is the sticky comment coverage, not a relabelled review
submission. Documentation-metric and bounded-cost dispositions are in review-debt.

The fetched base is unchanged bf85c9174, with no base-only delta. Merge-tree exits
with status 0 at a459bbf4e119572f4e68ee478823802eb64cd317, the qualified source
including proof/test/doc corrections. No additional combined behavior is untested.
This verdict permits normal ghprsq publication; actual landing, durable archive,
intended issue closure and owned cleanup must still be verified. Parent scope
remains unchanged and open.

## Earlier evidence and remediation history

# Validation-request boundary acceptance

Initial implementation is committed at e3c2b93b2 and published as PR 700.
Review remediation is committed and pushed at 6b5abee8b. Final-head hosted
qualification/review and landing remain incomplete.

- Original concrete failure: original-reproduce.log, exit status 101, one selected native
  runtime case. Native/direct forms answer true and both SPARQL forms false for
  the same warning-only inner shape under a violation-only request. The log was
  transferred with only the test from the parked recursion checkout.
- First compiler feedback: request-boundary-native.log, exit status 101 (wrong EvalCtx
  field), then callers-check.log, exit status 101 (namespace/import wiring). Corrected
  actual field user_functions and actual Shapes path; removed obsolete imports.
- callers-check-rewired.log: exit status 0, cargo check -p purrdf-shapes --all-targets.
  This covers the first complete binding/callback API wiring, before the later
  rule-request, shared graph-name and nested-registry refinements.
- native-callers.log: exit status 0, cargo test -p purrdf-shapes --test
  custom_node_expressions --test change_path_alloc --test conformance_memo --test
  shape_catalog. All 65 selected cases ran: 43 custom-expression, 12 original
  allocation/report-golden, 3 memo-sharing and 7 catalog/ownership cases. Includes
  actual reproduction/inverse/default, retained input disposal, independent
  request/fresh graph, prepared/restored complete reports and actual graph override.
  Later rule/native-view/nested-registry changes require affected requalification.
- rule-request-check.log: exit status 101 on an unnecessary test qualification only;
  corrected to the existing entail_dataset import. Production code compiled.
- callback-and-rules-native.log: exit status 101, test constructor accepted &str rather
  than String; corrected to its existing owned new_unchecked constructor.
- callback-and-rules-native-current.log: exit status 0, all 46 native custom-expression
  cases ran. This includes the actual four-way reproduction and inverse/default
  controls, original inputs dropped before retained-registry calls, nested raw
  callback queries under alternating request policies and fresh graphs, complete
  reports/products, explicit graph overrides, rule heads and conditions, selected
  ED/WD/legacy blank-argument laws and hard failures, and original direct-call
  allocation ceilings. The new scoped-cost control proves equal allocation count,
  requested, peak and retained bytes as custom severity population and graph-name
  lengths grow from 1 to 32 to 512. The worker control crosses the actual 1024 cutoff
  at 1025 focuses under a two-worker pool, toggling request policies three times.
- workspace-strict.log: exit status 101 on the newly opaque context missing from manual
  Debug. The corrected formatter reports presence and elides the payload.
- workspace-strict-current.log: exit status 101 on format_collect in a new worker fixture;
  replaced transient formatted strings with writes into its final String.
- workspace-strict-final.log: exit status 0, strict workspace/all-targets (including real
  CLI, C, Python, WASM and validate callers), before the ownership refinement.
- shared-registry-check.log: exit status 0, Shapes all-targets after moving immutable
  bindings behind one shared owner and using a nested-registry/weak-context
  back-reference. Root registry moves and clones retain the actual request.
- shared-registry-workspace-strict.log: exit status 0, strict workspace/all-targets on that
  ownership refinement. Subsequent changes are documentation and native test
  coverage only; no production behavior changed after this pass.
- settled-native-qualification.log: exit status 101; focused native execution of the library
  plus custom_node_expressions, change_path_alloc, conformance_memo, shape_catalog,
  catalog_product, profile_admission, native_xpath, complete_reports,
  product_model_census and host_entry_points. Includes the move/zero-allocation
  clone proof and named-shapes/Core-data isolation control. The 995 library,
  5 catalog-product, 12 allocation/golden, 24 complete-report, 3 memo, 47 custom,
  11 host and 29 native-XPath cases passed. Product census then passed 19 and
  failed 9 because the private Shapes validation_options field type intentionally
  changed; its declared-model preimage moves the stage id. Later targets in that
  command were not executed. This is not a full passing qualification.
- Codec assessment: options remain request-local and are not encoded. Writer and
  reader still cover the full model; admission constructs the new shared request
  from the caller's options, verified by current prepared/restored callback tests.
  Only the exact report-source cache is excluded from the census. Do not broaden
  that exclusion for request storage. Derived stage id is now
  53f55a030413d8fd8d72449985df0d707844d7abd5e887dc41585f0cbcffc9cf.
- product-fixture-regenerate.log: exit status 0, the exact sanctioned ignored Rust
  generator case ran once. It verified fresh/restored report identity before
  writing prepared-shapes-core.product. Printed length 5608 matches the existing
  GOLDEN_LEN; no size assertion changed. The separate frozen format-epoch fixture
  remains untouched. Prepared-product identity bytes change for the declared
  request representation; RDF/report bytes and data grammar do not change.
- product-and-callback-native.log: exit status 0, all 219 affected product/callback cases
  passed (5 catalog product, 24 complete reports, 47 custom functions, 1 corpus
  equivalence, 7 product determinism, 28 model census, 7 RDF1.2 product identity,
  65 product refusals, 13 product reuse, 15 profile admission, 7 shape catalog).
  The supported generator remains intentionally ignored in the ordinary target;
  its one explicit execution above passed. This also executes the
  profile-admission/catalog targets not reached after the prior failure.
  Source-isolation coverage now checks property probes as well as class checks.
  The expanded class/property source-isolation control passes. Earlier library,
  allocation, memo, host and XPath results remain applicable to their unchanged
  execution paths; only the derived stage constant and its generated fixture
  changed production inputs after that run. The final product batch requalifies
  the changed identity/admission/restore/fixture paths. No full green verdict is
  assigned to the earlier command that stopped at the model census failure.
- candidate-workspace-strict.log: exit status 0, final strict workspace/all-targets
  qualification after the stage/fixture update and settled native controls.
- commit-hooks.log: exit status 0, normal signed commit e3c2b93b2 created with all
  configured staged-index hooks. No bypass or signing override. No source
  qualification changed after the final local native/strict checks.

Commands use the normal Cargo wrapper, eight build jobs and at most four native
test/Rayon threads. No hooks, warning policy or resource ceilings are bypassed.
No general semantic WASM execution is added. Existing raw command output remains
in this exact Stage; archive settled outputs byte-for-byte before merge.

Post-publication remediation is active. Initial-head CI 40067917178 caught an
unresolved UserFunctionRegistry doc link after its import was removed; qualified
the path. current-rustdoc.log passed with -D warnings for both changed crates.
The initial geographic job stopped before testing on Binaryen download connection
reset (ci-geo-current.log). Neither failing job is treated as passing acceptance.

Independent initial-head review at issuecomment-6380583352 covered e3c2b93b2 and
raised repeated shapes-graph work as a nonblocking risk. Its suggestion to defer
performance work until after merge is not accepted. The current uncommitted
remediation reuses a matching owned binding in the existing scope, keyed by
original declaration/source identity, request-option owner and actual graph name.
Weak origin anchors avoid retaining raw shape acquisitions. It shares immutable
subclass pairs and lazily built rowless class dictionaries across callbacks and
rule snapshots, while resolving every actual graph's handles independently.

The new public runtime allocation probe initially failed: length 1 to 32 increased
requested bytes 6618 to 6649 with the same71 allocations. Source inspection found
one remaining temporary TermValue::iri in ShaclRead's composite lookup. The
borrowed composite/selected/delta lookup now routes through the existing canonical
lookup homes without constructing that owned spelling. No source ID or value
encoding changes. scoped-bindings-native.log preserves the actual failed probe
(48 passing, 1 failing). No ceiling or constant was relaxed.

shared-class-config-check.log passed Shapes all-target compilation.
shared-class-config-native.log exited 101: 995 library and 5 catalog-product cases
passed, then10 allocation cases passed and two exact allocation pins failed on
lower counts. Subsequent targets were not run. The separate
scoped-class-payload-native.log exited 0 with all 49 custom-expression cases,
including zero-allocation matching scope reuse and constant class-payload costs.

expansion-allocation-diagnosis.log preserves all four measurements: changed-row
counts 32/64/128/256 cost 78/145/276/535 allocations with a chain and 14/17/20/23
without one. Source inspection attributes the fixed reduction to borrowed trigger
IRI lookup. Two vectors start at capacity four, subtracting two startup
allocations from three logarithmic growth series; one owned shapes-graph link
probe remains. Exact formulas are now 1+2N+3log2(N)-2 and 1+3log2(N)-2, respectively.
The per-row 2 and per-doubling 3 coefficients are unchanged; no ceiling increased.
Diagnostic prints were removed. The report golden remains unchanged.

borrowed-iri-core-native.log exited 0: 73 cases across composite_probe_alloc,
mutable_declared_graph_drain and shared_views. The one pre-existing large
compaction acceptance case remains ignored. New length 1/32/512 native controls
prove zero allocations/requested bytes and canonical handles through native,
delta, composite and selected dictionaries, including excluded/absent terms.

remediated-shapes-native.log exited 101 after passing 12 allocation/golden,
24 complete-report, 3 memo and 28 census cases; 6 catalog cases passed and the
depth-one requested-byte ratio failed. Lower fixed cost exposed a faulty
ratio envelope: 3458*32 is less than 111982, even though spelling copies were
removed at every depth. Replaced expansion count/byte ratio checks with exact
pins at depths 1/8/32: 10/48/172 allocations and 3458/27502/111982 requested bytes.
Preparation/peak/ownership assertions remain. catalog-and-w3c-native.log exited 0
with all 7 catalog and 29 native W3C 1.2 cases. No product stage/fixture changed in
this remediation; all 28 census cases prove the current constant still matches.
remediated-workspace-strict.log exited with status 0: strict workspace/all-target
Clippy, including actual CLI, C, Python, WASM and validate consumers, passes on the
review remediation. remediated-rustdoc.log exited with status 0: rustdoc with
-D warnings passes for core, Shapes and SPARQL eval.
remediation-commit-hooks.log and remediation-push.log exited with status 0.
Normal signing and configured index hooks ran without bypass. The checkout is
clean. Hosted CI run 40073390589 and the three focused workflows are running
on 6b5abee8b. Final-head hosted checks and independent review remain required.
Initial review UI/auth blocks were structurally projected from
the saved comment; all technical text, coverage markers and original metadata
remain in raw/pr-comment-6380583352.json.

Pending: full final-head hosted checks,
current-head independent technical review, actual integration, ghprsq archive,
issue closure and supported owned cleanup. Full recursion/corpus/host contracts
remain open on their parents and parked feature; this child does not reduce them.

## Archive paths

Every nonempty top-level command log cited above has a byte-identical copy at
`acceptance-evidence/<log stem>.txt`. The archive excludes `.log` files, so these
text copies preserve full positive and negative command output without filtering.
The original path names remain in this index for local replay. Raw forge evidence
is retained under `raw/`; only mechanically identified authentication/UI blocks
are projected, with technical review text and original metadata preserved.

## Current hosted wait

At 07:20 UTC, CI run 40073390589 is confirmed in progress on 6b5abee8b.
The final-head rustdoc job passed. No failed job is reported. The actual
CodeRabbit commit status is pending with description "Review in progress";
the earlier review is not counted as covering the remediation.
`gh run watch 40073390589 --exit-status --interval 30` remains live in owned
session 75198 and writes `current-ci-watch.log`. Collect that exact terminal
handle rather than restarting the run. This live log has not been frozen for
archive. All 35 settled nonempty command logs have frozen text copies.

The complete review API capture currently includes one issue comment, one review
submission, zero inline comments and zero review threads. All endpoints were
paginated; nested thread pagination was checked. These are interim observations,
not final review closure. Recollect when the final-head review finishes.

## Capacity-proof remediation

CI run 40073390589 failed its workspace job at the shared-helper gate.
`current-workspace-failure.log` preserves the actual job output. The borrowed
core changes invalidate the conservative constructor-source identity carried
by 40 existing proof rows, plus the two changed files' authority identities.
This is not treated as a passing gate or an unrelated failure.

The native helper's index was captured for clean main bf85c9174 and head
6b5abee8b. Both have exactly the same 173 constructor-source members. The base
discharges all its existing sinks; every one of the head's 40 newly stale
proof addresses still matches its original file, symbol, line and literal body
fingerprint exactly once. Only composite.rs and mutable/delta_view.rs changed
inside the core authority domain. Source review confirms that their edits add
borrowed lookups and visibility/documentation, not a constructor, narrowing
conversion, mutable write or changed capacity condition. Canonical mappings and
selection boundaries retain their original checked constructors.

Updated only the 40 constructor identities and six references to the two changed
authority identities. No proof, sink, class, authority member, reason or exemption
was added or removed. `raw/capacity-proof-refresh.json` records the comparison;
the two capacity-only native index projections preserve their authoritative
fields and omit unrelated symbol/group records. The tool exposes the identities
but has no refresh command, so the reviewed ledger values were edited directly.

`refreshed-helpers-gate.log` exited with status 0: the complete native
`cargo run -q --locked -p helper-census -- --check` passes. Existing runtime and
strict results remain applicable because this final repair changes only proof
metadata. Normal hooks are running for the proof-only commit; fresh head CI and
review still must complete before landing.

## Integration preview

Fetched the actual base on October 24. Base bf85c9174 has no base-only delta
against the implementation head 6b5abee8b. The merge-tree command exited with
status 0 and produced tree 3a2d9bb1867b3c0ba27354c67384033b2ef37c3f.
The supported sibling squash-notes file is drafted, explicitly pending final
checks/review. Refresh the preview after the proof-only commit and again before
ghprsq; this preview does not claim publication or an exact-input lock.

## Final proof head

The proof-only commit ce5b5f8a2 was signed, committed and pushed normally.
`proof-refresh-commit-hooks.log` and `proof-refresh-push.log` both exited with
status 0. There are no source changes after the passing native/strict checks.
The full helper check passes for the refreshed ledger. The checkout is clean.
CI run 40073390589 was automatically cancelled after the push; its owned watcher
session 75198 exited with status 1 and has been collected. Its workspace failure
is preserved, rather than relabelled as a pass. Three focused workflows on the
unchanged runtime head 6b5abee8b passed; their scoped results remain applicable.

Final-head CI run 40074482046 is live on ce5b5f8a2. The owned watcher session is
51127 and writes `final-ci-watch.log`; collect that exact handle. This live log
is excluded from the otherwise frozen command outputs. Final-head independent
review remains pending. The proof-head merge-tree preview is clean at
6f206d698088eb55c85006cb00fb5e48996d0eed against unchanged base bf85c9174;
refresh actual inputs and all review/check evidence before landing.

## Governed expression allocation reduction

CI run 40074482046 exposed one additional downward pin change in integration
shard 4. `final-integration4-failure.log` retains the actual failure. All other
seven cases in sparql_path_alloc passed; the governed expression case measured
131877, 263720 and 527403 allocations at 1280, 2560 and 5120 focus nodes. These
are exactly `37 + 103 N + 3 log2(N / 1280)`, reduced from entry 41 and slope 104.
The other three governed surfaces retain their existing exact forms.

Source attribution: the expression fixture has a path-values argument and no
registered declarations or subclass supplement. Its path resolves the focus
through path::eval_planned -> data::resolve_id -> ShaclRead::term_id_by_iri.
The delta-view borrowed lookup removes that one temporary per focus, while
four setup spelling copies disappear from the bounded change expansion. The
opaque SPARQL cases skip that expansion and retain their earlier costs.

Updated only the exact two pins and their measured prose, including the governed
design table. `governed-expression-allocation-native.log` exited with status 0:
all eight native allocation, healthy-neighbour, violation-report, measurement-lock
and registered-prose cases pass. `governed-allocation-strict.log` exited with
status 0 for strict Clippy on this target. Existing production/workspace/runtime
evidence remains applicable; no production code changed in this correction.
Normal signed commit a010ace7b passed its hooks. Fresh hosted checks and review
remain required before landing; no old failing run is labelled green.

## Latest published head and wait

The allocation-pin correction is committed and pushed as a010ace7b. Its normal
hooks and push both exited with status 0. The source checkout is clean. The
previous watcher session 51127 was collected with status 1 after automatic
supersession of CI run 40074482046; the original integration failure is preserved.
The replacement CI run is 40076759527. Its owned watcher session 73681 writes
`governed-pin-ci-watch.log`; collect that exact terminal handle without restarting.
This is the only live output excluded from the current frozen command evidence.
The independent review status on a010ace7b is pending, described as "Review in
progress". No earlier review is substituted for this final-head requirement.

The clean latest-head merge preview is a459bbf4e119572f4e68ee478823802eb64cd317
against unchanged base bf85c9174. Refresh base/head/check/review evidence before
publication. No merge, archive, issue closure or cleanup has yet occurred.
