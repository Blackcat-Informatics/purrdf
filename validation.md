# Current acceptance

FINAL BINDING STATUS: PASS. Published head ede07f859 has 50 successful hosted
checks and five configured skips. All local components, corrected native/WASM
targets, strict checks and normal hooks/publication pass with the original
full-run failure and scoped reuse recorded below. Complete feedback reads show
no actionable finding or unresolved thread. Completion review:
tasks/completion-review.md. CI test merge 465c34b6e has exactly the predicted
tree for main d839dd72a and head ede07f859. The deficiency ledger is empty below
its marker. Actual merge, closure/mirror verification and cleanup remain.

## Earlier qualification progression

The final status above supersedes pending workflow states retained below.

Published head: ede07f859. Runtime source is unchanged from the qualified repair
f329f8120; the final four-line documentation correction reflects main's additional
portable role case in the clean combined tree. Both commits pass normal hooks
and HTTPS publication. All local gate components are now covered: the original
full run has only the old cast failure, corrected native/WASM targets and strict
checks pass, and the separate JSON consumer, core hygiene and complete release
WASM build pass. The original failed command remains recorded as failed.

Independent source review covers the twelve feature files and all eight repair
files with no actionable findings. The inventory-only correction has an owner
comparison against current main and the predicted integration. See review-debt.md
and tasks/integration-assessment.md. Final hosted CI, feedback refresh, binding
merge gates and actual ghprsq integration remain pending. No closure is claimed.

## Qualification details and original failure

Initial hosted integration-5 on f90f628db failed at the existing an_any_uri_casts_to_string_alone
case: xsd:hexBinary over an anyURI returned 0F instead of unbound. All other
hosted jobs pass or have their expected skips. The failed test aggregator is
the dependent result, not a separate defect. Raw evidence is retained in
tasks/pr579-integration5-failure.log and pr579-check-details.json.

The native AnyUri variant bypassed the old unmodelled StringOnly row and fell
through the generic nonnumeric admission. The working correction restores
string/self-only admission at that existing home, removes the obsolete anyURI
fallback classification, and parses URI values before string conversion so the
native whitespace mapping is retained. Existing operator-table coverage is
extended with base64 and normalized string/self controls; a portable governed
case is added at the existing numeric/cast target. The corrected native command
passes all 23 operator-table tests and all three governed portable-target cases,
including malformed URI source refusal and the unchanged numeric transcript.
Log: tasks/anyuri-cast-native.log. Portable registration/documentation is complete;
strict compilation passes for all targets of sparql-eval, xsd and entail with
warnings denied (tasks/anyuri-cast-strict.log). Actual WASM execution passes all
three governed cases, retaining digest 3704a65139127bbd over the original 2822
numeric transcript records (tasks/anyuri-cast-wasm.log). Normal hooked commit and
HTTPS publication pass at f329f8120. The separate JSON consumer, core hygiene
and release WASM build pass; final hosted CI is running on ede07f859.
Logs: tasks/final-json-consumer.log, final-core-hygiene.log,
final-wasm-build.log, anyuri-cast-commit.log and anyuri-cast-push.log.

Source review also found the reverse admission error: binary, calendar,
duration and list values could reach the new anyURI target through the generic
nonnumeric row. The same gate now admits an anyURI target only from URI or the
existing string row. Native controls cover those reverse refusals and a derived
string neighbor; the portable case checks both directions for six primitives.

Scheduling update: the running gate is executing a prebuilt deep-request test
without a compiler and under 100 MiB resident. One bounded focused build may
overlap that execution under the original capacity limits. The first admission
check found only 31,165,112 KiB available (below 32 GiB), so no second Cargo
invocation was started. Compilation remains pending; this is not a test failure
or a claim that the host is unusable.

Normative source: XPath F&O 3.1 sections 19.1.1, 19.1.6 and 19.1.7,
https://www.w3.org/TR/xpath-functions-31/#casting-from-primitive-to-primitive.
The original make check has terminated at its workspace-test recipe. Every
workspace target, including doc tests, passed except the original operator-table
binary's anyURI-to-hexBinary failure (22 passing, one failing). The benchmark
entry-point release build, C ABI smoke, both 100,000-level parser controls and
all other native targets completed. This failed command is not a pass on the
correction. No launcher/toolchain/cache override was introduced. The independent
JSON consumer test, rdf-core-hygiene and wasm build recipes were not reached;
they remain required. Reuse the unaffected completed coverage and execute the
changed cast targets, strict affected checks and those missing recipes.

After the original native handle reported terminal exit 2, both portable cases
were registered in Makefile and the focused native correction command started
as the sole owned Cargo stream, with the original eight-job/four-thread/16 GiB
caps. Log: tasks/anyuri-cast-native.log. No overlapping build was launched.

The actual TextIndex delta path is implemented and published in PR #579. Final
qualification and the consumer repair remain in progress; no closure is claimed.
Surface operations support
complete-document retraction and
insertion/replacement, reusing normalized text, alignment, span shapes, codes and
existing gram positions. Flat posting runs are remapped/merged in order. The
existing pronunciation union now uses the same sorted join, rather than a
second body. Canonical generation and observable evidence match fresh builds in
the focused cases below. These are owned operations; no implicit snapshot copy
or sublinear total-work claim is made. Canonical hashing still visits content.

| Requirement/scope | Evidence |
|---|---|
| Actual TextIndex source admission and ordered merge | PASS, five native controls in tasks/text-delta-matrix-fixed.log; missing additions and wrong generations retain typed operands; graph-free/asserted/annotation and cross-layer multiplicity covered |
| Canonical statistics, ranking and whole-index identities | PASS, the same controls include 195 chained updates over lexical, Han and surface-disabled configurations under a weighted ranking profile; tables, streams, partition/field facts, scores and auxiliary evidence compare with fresh construction |
| Fixed work and token allocation | PASS, mixed update observes exactly four Analyzer::projections calls for four rows in changed documents, and the unchanged document retains its token buffer; no timing used |
| Affected complete package | PASS, cargo test --locked -p purrdf-text, 353 tests in 33 suites, tasks/text-package-tests-final.log. The earlier source-inventory failure in text-package-tests.log was corrected from 29 to 31 for the two new modules. The subsequent coverage-only addition to the public delta case needs its focused rerun below |
| Latest strict/public native/WASM/helper checks | PASS, tasks/text-delta-strict-qualified.log, text-delta-public-native.log, text-delta-public-wasm.log and text-delta-helpers.log. Native and actual WASM each execute the public delta/ranking/surface/empty/nonliteral-coverage case. Census: 82 jobs, 118 distinct rows, no new exemptions or cross-crate includes |
| Coherent initial library compilation | PASS, tasks/surface-retraction-check.log and surface-delta-check.log |
| Retraction, empty/no-op/bound generation, source/code allocation retention | PASS, original three cases in tasks/surface-retraction-tests.log |
| Full surface delta native controls | PASS, eight cases, tasks/surface-delta-tests-final.log; mixed insert/replace, repeated sources, Unicode, source evidence, chained updates, one/two/three-scalar gram queries, pronunciation, analyzer mismatch, and existing invalid-analysis refusal |
| Gram posting physical allocation retention and ordered union | PASS in the same native controls; the retraction retains the exact original posting pointer/capacity; union matches the existing fresh-sort builder |
| Shared pronunciation cache allocation on insertion | PASS, tasks/surface-delta-allocation-retention.log; the old canonical-code allocation survives a new earlier-sorting word and an added document sharing that code |
| Strict affected library/all-test compilation | PASS, tasks/surface-delta-strict-fixed.log; the initial type-complexity failure is retained in surface-delta-strict.log and fixed with a named source/map result, no allow |
| Existing surface integration callers | PASS, fourteen cases, tasks/surface-delta-consumers.log; includes independent exhaustive scalar oracle, positional/budget refusal, HTML/multiscalar evidence, emoji endpoints and pronunciation refinement |
| Helper census | PASS, tasks/surface-delta-helpers.log; no new exemptions or ledger/checker changes |
| Actual wasm32 library compilation | PASS, tasks/surface-delta-wasm-check.log; compilation only, not portable runtime acceptance |
| Independent hosted source review | PASS, completed CodeRabbit review in pr-comments.md covers all 12 files at integrated f90f628db and reports no actionable comments. Paginated review/inline/thread surfaces contain no material findings or unresolved threads. This is source judgment, not independent acceptance execution |
| Corrected cast consumer | PASS: 23 native operator-table cases and three governed cases on native and actual WASM, tasks/anyuri-cast-native.log and anyuri-cast-wasm.log; strict affected all-target compilation passes in anyuri-cast-strict.log. Owner source assessment: tasks/anyuri-cast-owner-review.md, not independent approval |
| Full gate/CI/integration | Original full gate terminated with only the known operator-table failure; corrected native/WASM targets, strict affected checks and all missing final recipes now pass. Unaffected coverage is reused with its source scope. Final hosted test-merge checks on ede07f859 remain pending. No merge or closure claimed |
| Delivery | Normal hooks and HTTPS publication pass through ede07f859. PR #579 links #538/#571. Independent feature and repair source reviews have no actionable finding; final feedback/CI and ghprsq integration remain |

No timing/performance benchmark or full baseline gate ran. The completed hosted
source reviews above cover f90f628db and f329f8120; the final inventory-only
correction has an owner comparison against the combined selections.
Statistics are summed from retained document/predicate facts using the existing
partition/field homes; retained documents are not reanalyzed. Metadata walks,
canonical source sorting/hashing and ordered table merging remain linear or
sorting work, and explicit snapshot cloning copies the buffers. No sublinear
total-work, shallow-copy snapshot or timing improvement is claimed.
Stage plan/prior-art artifact gates only verify files exist; they are not review
or implementation approval. No issue closure or merge is claimed.

Owner assessment: tasks/text-delta-owner-review.md, explicitly not independent
review. Its final style finding replaces the trivial From wrapper with the
existing variant_from macro; the generated conversion is identical. Focused
strict/native controls pass in text-delta-conversion-macro-strict.log
and text-delta-conversion-macro-tests.log (all five delta controls). Earlier package/portable/census
behavior evidence remains applicable to that equivalent constructor spelling;
the single full final gate ran on integrated f90f628db: `make check`,
tasks/text-delta-final-gate.log. Its one failure is corrected and qualified by the
focused later results above; the omitted final recipes also pass. The correction
and inventory are committed and published. Six upstream lexical/XSD files
were integrated without conflict; their XML-character admission changes are
included in this final candidate. No pass is presumed while the command runs.

Publication note: stagectl pr-create returned a JSON parsing error after GitHub
created the PR. A normal stagectl branch lookup confirmed #579; there was no
duplicate creation or alternate merge route.
