# Current implementation status

## Final merge readiness

Publication transport recovery: the first ghprsq invocation remained in its
initial read-only git fetch. The owned SSH connection had sent only its 22-byte
client banner and received no server banner; no authentication or repository
transfer had begun. Terminating that stalled SSH child made fetch exit 128.
Remote PR readback remained OPEN and main remained 7734b013d. The second normal
invocation completed fetch but exited 1 on unavailable PR metadata before
integration. Direct stagectl pr/pr-meta now returns the expected open PR and
unchanged base/head. No merge method, hook, compiler, cache or tool implementation
is changed; retry uses the same ghprsq command. The PR body is reconciled with
completed CI and corrects the bot's false stemmer-semantics sentence.

Current published c8f5ebafd qualifies the complete contract. Fresh Stage merge
brief reports pass: 49 successful hosted checks and four expected skipped
optional jobs. CI run 38856956986 passes the full workspace, all test shards,
all seven assembly configurations and aggregate, conformance, portable execution,
installed host packages and architecture checks. No duplicate local full gate is
claimed or required; the cancelled local run remains status 143.

The complete original five demonstrations and additional rule/provider/ownership
requirements are adjudicated in reviews/implementation-completeness.md. All six
inline comments/replies, five review submissions and both resolved review threads
are refreshed in tasks/final-inline-review.json, final-reviews.json and
final-review-threads.json. No new material finding or unresolved thread exists.
The owner contract assessment is not represented as independent approval;
actual hosted review dispositions are preserved in review-debt.md.

Fresh main integration predicts clean tree 84d7d797e891e5dacab4243b610e5d86fd7bf9af.
Main is already included in the branch; all qualified source behavior applies
without an additional combined delta. Required normal hooks/pushes passed.
Finalize squash notes and use ghprsq; no further implementation or discretionary
qualification precedes this ready delivery. Post-merge actual-tree, closure,
archive, mirror and cleanup verification remain before final delivery accounting.

## Final hosted findings repaired

Run 38855880532 accepted the character-range rewrite, then found the current
compiler's unnecessary_box_pin lint in retrieval's synchronous executor. The
three applicable local futures (production executor, synchronous fixture support
and private fusion test poller) now use std::pin::pin! in c8f5ebafd. Their polling
and waker behavior is unchanged; one heap allocation is removed. The other two
Box::pin search hits are literal source fixtures in helper-census, not executable
pin bindings, and remain unchanged. This delta needs current hosted retrieval
and workspace acceptance; no old all-target pass is claimed for it.
tasks/stem-range-workspace-ci.log preserves the real failure. Standard curl
retrieved the same authenticated job-log endpoint after gh's TLS handshake
timeouts; TLS verification was retained and no toolchain/cache override used.

Run 38855325417 accepted the deprecation metadata repairs and then found the
new compiler's manual_range_patterns lint in the stemmer. The equivalent
contiguous character range 'w'..='y' replaces 'w' | 'x' | 'y' in 8c8295da8.
This is a syntax-only predicate rewrite with identical members; no behavioral
test change or lint suppression is used. Its actual CI log is
tasks/final-workspace-ci.log. Final current-head CI remains mandatory.

Hosted run 38850408634 completed all seven assembly configurations: six passed;
Neoverse found only eval.bgp-join-probe v70 versus measured v85. The scoped local
writer also measured v85, but its older September 30 compiler changed four other
cells that the October 15 CI compiler measured unchanged. Only the applicable
generated v85 hunk is retained; the full local generated patch is preserved in
tasks/neoverse-local-generated.patch. Final new-head CI must establish all seven
configurations together. No compiler, cache or target workaround is used.

The hosted workspace lint requires deprecation version metadata for three
unreleased compatibility items. Their first publishing version is unassigned;
item-scoped allowances retain deprecation warnings without inventing a version.
The older installed Clippy also needs item-scoped unknown_lints allowance.
Affected strict library checks pass in
tasks/unreleased-deprecation-compatible-strict.log (exit 0).

The text zero-headroom fixture now admits the actual opening peak, then holds
the difference between retained and peak bytes through reserve_workspace. The
query therefore still starts with exactly zero available bytes, preserving
all original refusal, no-provider-read and ownership assertions. The focused
execution passes in tasks/integration-text-original.log (exit 0); its filename
predates the fixture correction. No memory acceptance is relaxed.

Main integration is already resolved and published at 91334d30c. The current
four-file delta contains these final CI repairs and documentation reconciliation.
Historical statements below retain their original scope; no old running-state
statement overrides these current results. Full hosted qualification remains
the final gate; no duplicate local full suite or timing acceptance is added.

Current published source is 91334d30c, including the resolved main integration
and bounded document-writer adaptation. Normal commit hooks passed; the log
explicitly records the staged non-Rust ratchet, not a local full-gate pass.
Both earlier remediation commits
were pushed successfully; hosted qualification can proceed during the final
assembly configuration. Commit 33fc8f2d0 fixes metadata readiness/admission,
forwarding and affected pins/proof registrations; aebd40356 pins the three actual
structural trace changes. All 21 completion cases, exact SHACL allocation and
all 712 regenerated query traces pass at their recorded source. Three traces
change candidate charges, with complete answers and peak cells unchanged.

The original assembly writer is terminal. Six configurations completed their
measurements; x86-64 failed on a fresh typenum artifact without matching assembly
provenance. The runner continued the other six before reporting that failure.
This is not a seven-configuration pass and it wrote no documentation. Earlier
progress observations incorrectly counted x86-64 as completed.
The old published-head hosted failures remain historical; corrected-head hosted
qualification and final material dispositions remain open. Main is now merged
into the worktree with both conflicts resolved. The paged control preserves the
landed primary-row behavior and asserts statistics [3, 1, 0]; the segmented
control retains (4, 1, 0). All 37 combined stack controls and fresh native helper
hygiene pass. The forty unchanged proof contracts are rebound to the actual
combined constructor/source closure, without new exemptions.

The existing document writer rejected a selected configuration. Its bounded
adaptation preserves unmeasured columns, permits a scoped write and retains the
seven-report final gate. All existing self-tests and a new preservation control
pass, with no increase in legacy Python lines. The standard scoped Neoverse
writer is running on stable combined source. Hosted full qualification will
cover all seven configurations; no duplicate local full matrix is required.
The earlier merge brief could not fetch checks; unavailable evidence does not
establish a passing gate.

After generation finishes, integrate the landed main row-kind repair, resolve
the combined physical counts and source proof registrations, and qualify the
actual affected integration. Ready publication/integration takes priority over
other unfinished deliveries. No duplicate local whole gate or timing acceptance
is introduced. The entries below are historical source-scoped records, not a
claim that earlier uncommitted changes or old execution sessions are still active.

PR 557 publishes the complete contract through 279a26a0e. All 15 normative
governor-corpus checks pass, including unchanged frozen expectations and the
complete fuel-sweep trace; the sole ignored writer is not acceptance. Profile
identity and all 71 hygiene route parity checks pass. Hosted run 38804211664
failed the documented profile digest, SHACL fixed allocation pin, three query
trace cases, query-completion metadata forwarding and one ARM documentation
cell. Its aggregate failures are not separate production defects. The complete
owner assessment is reviews/implementation-completeness.md; no closure is claimed.

Current uncommitted remediation fixes the paged snapshot-readiness getter and
selects statistical strings only for actual EXPLAIN surveys. Their focused
regressions pass. The reviewer proposal to change unknown-domain replay is an
evidenced false positive: the original relation-local law is retained and its
7-row/140-peak control passes. All 21 query-completion cases pass after forwarding
statistics through the two test adapters, without changing receipt equality.
All 154 documentation claims pass with the corrected profile-20 digest. Current
native helper census passes (tasks/review-helpers.log, exit 0), including the
unchanged forty proof contracts rebound to their actual source. SHACL allocation
and trace regeneration are running in session 53436. Full hosted qualification
on the correction and final material dispositions remain open.

The first assembly writer was stopped before the queued golden writer could
change its tracked source identity (tasks/review-assembly-doc.log, exit 130).
It is not assembly acceptance. Run the existing writer after generated trace
changes are settled, then keep source stable during that execution.

Stagectl refreshed the full issue and four comments with no new contract changes.
Its PR creation reported an unparseable response; supported branch lookup and
live forge read established PR 557 was created, so creation was not retried.
The body was updated through gh because stagectl has no edit-body operation.
The qualifying hosted route supplies the final full gate without a duplicate
local workspace run. Critical 556 is now published independently as PR #559.

The complete production contract and all five explicit demonstrations are
implemented and published through 279a26a0e. Focused native/public-query, portable,
strict/helper and normative governor-corpus acceptance pass. Hosted full
qualification and material review remain required; no closure is claimed.
Main's delivered indexes are integrated at 1cf1749a6. Plan.md maps the full
contract; owner analysis/review is not independent approval.

The first core slice defines typed exact/upper-bound counts, per-predicate
distinct-domain metadata and the immutable local-ID SnapshotStatistics seam.
DatasetView exposes the provider without fabricating missing statistics; Arc
forwards it. RDF term kinds reuse the existing RdfTermKind home. No runtime
dependency, sort, allocation path or new physical row layout is introduced.
Affected core library compilation passes: tasks/contract-check.log, exit 0.

This first implementation slice is committed and pushed at a84f28cdd through
normal hooks. BGP cardinality/fallback-cost consumers
now consult the complete three-stream provider count, and retained-order keys
include provider identity/revision. A selected actual BGP join-order/answer test
reuses the original HostCost fixture and passes: supplied statistics select the
rare predicate first instead of the flat fallback's first authored predicate,
with all 100 answers preserved. Evidence: tasks/supplied-statistics-query.log,
exit 0. The two related query controls now share one pattern-construction home;
both directly affected controls pass in tasks/shared-fixture-order.log and
tasks/shared-fixture-host-cost.log. Strict core/evaluator library Clippy with
warnings denied and workspace format checks pass. This is still only the first
implementation slice, not issue completion. No PR is opened for the partial contract.
Storage providers, sorted-domain merge, remaining rule/EXPLAIN consumers and all five complete
acceptance demonstrations remain to implement. No dark interface is counted as
delivery and no foundation-only PR is planned. Checks so far are targeted to the
modified core slice; no broad qualification has been launched for this branch.

## Native stored summaries

The next coherent unit is committed at 527f70f64 with normal hooks. It seals
predicate/class frequencies, exact positional distinct domains, selected object
frequencies, kind/datatype counts and reifier target-predicate counts at freeze.
Native and compact read providers expose the same sealed data, with compact
graph refusal on the original first-cause owner. The existing pack delta codec
holds sorted domains, and query-output retention includes the sidecar bytes.
No asserted, reifier or annotation row was widened. Push passed.

Five targeted core/admission/allocation/ownership controls pass; both actual
native/compact statement query and refusal controls pass; the real supplied
provider changes join order with all 100 answers preserved. Strict affected
core/evaluator libraries and tests pass. Helper hygiene passes all 82 enforced
jobs, 118 groups and 2275 paths. Existing capacity proof contracts are unchanged;
their authority/constructor identities and moved sink coordinates were refreshed
from the native census, with no new proof, exemption, duplicate body or checker
change. See tasks/native-summary-review.md and native-summary-*.log.

Paged/segment providers, exact translated domain union,
native rule ordering, EXPLAIN evidence and all five complete
issue demonstrations remain required. This published unit is not a whole-issue
pass and does not open a partial PR.

## Distinct-aware ordering and replay

Published at de36c4840 with normal hooks; push completed successfully. DP,
greedy ordering and replay share the domain/divisor computation. Existing
PatternCost and solution row layouts are unchanged. The optional domain
sidecar uses the original workspace admission and exists only when concrete
metadata is available. Unknown domains retain the original fallback, and
overlapping stream domain sizes are explicitly upper bounds.

All 37 affected BGP module controls pass, including a real positive-answer
query whose plan changes through both DP and greedy ordering while retaining
five answers. Replay predicts five rows and a peak of five. The compact
provider now preserves graph-first refusal across all four subject/predicate
presence combinations. Strict affected core/evaluator libraries and tests and
all 82 helper jobs pass. Only existing constructor closure identities changed;
all forty proof contracts remain intact. See tasks/distinct-model-review.md,
tasks/distinct-model-bgp-controls-fixed.log,
tasks/distinct-model-compact-guard.log,
tasks/distinct-model-strict-final.log and tasks/distinct-model-helpers.log.
Earlier failed attempts remain recorded with their subsequent dispositions.

No timing acceptance, broad qualification or whole-issue completion is claimed.
Paged/segment providers, exact translated domain union, native rule ordering,
all five complete demonstrations remain required. Reconcile profile/trace claims
after the planner settles.

## Statistical EXPLAIN evidence

The actual survey and retained positive-plan callers now publish admitted
provider source/revision, row counts and subject/object domains with explicit
Exact/UpperBound/unknown input evidence. Joined output estimates remain
heuristic. QueryExplanation retains and renders these records; resident Clone
preserves them. No solution row or ordinary result wrapper grows.

The supplied-provider public EXPLAIN checks pass, as do all 51 affected BGP,
survey and positive-plan controls. Strict affected libraries/tests pass after
two equivalent empty-slice assertion styles were corrected. Native helper
census --check passes with no unsanctioned duplicate/forbidden implementation.
See tasks/explain-statistics-review.md and explain-statistics-*.log/.exit.
No broad qualification, whole-issue delivery or independent approval is claimed.

## Actual million-claim join demonstration

PASS: the public EXPLAIN path evaluates the required reifier/valid-annotation
join over 1,000,000 reifiers with 6,000,000 annotations. Its BGP produces
1,000,000 actual rows and predicts exactly 1,000,000, within the required 2x
bound; candidate accounting is 2,000,000. No time-based acceptance is used.
The same helper checks populations 1, 128 and 4096 before the actual full run.
It reuses the original statement_claims population and the fixture's
caller-selected valid predicate, not a second corpus generator.

Evidence: tasks/statistics-join-small.log and statistics-join-million.log,
both exit 0; the latter standard libtest executable is identified by the
original cargo test build output in the former. The full run was capped at
16 GiB. Strict compilation of the affected integration target passes in
tasks/statistics-join-strict.log. The tests are committed at 076aac6d3 through
normal hooks. This is one complete issue demonstration, not completion of the
remaining paged/segment/rule contract. Source synchronization with the now
merged predecessor is complete at 1cf1749a6 with normal hooks and push.

## Exact paged metadata and query consumers

The current coherent unit seals admitted local statistics into page headers,
translates before sorting/unioning global domains and shares a page-set-bound
bundle on warm reopen. PagePart's public shape and RDF row/term layouts are
unchanged. The original summary digest also certifies joint statistical contents.
The actual BGP consumer keeps statistical rows distinct from physical host work.
Owner review: tasks/paged-statistics-review.md; not independent approval.

| Affected requirement | Evidence and result |
|---|---|
| Exact domains, graph/stream parity, selected frequency policy, reversed dictionaries and full-width globals | paged-statistics-frequency-controls.log, five pass |
| Summary refusal and changed joints with equal old marginals | paged-summary-digest-controls.log, five pass |
| Original grant/refusal rollback | paged-statistics-original-grant.log, one pass |
| Native and paged zero-allocation metadata reads | paged-statistics-zero-alloc.log, one pass; zero allocations/requested/retained bytes in both windows |
| Actual rare-class native/paged execution and small claims | paged-statistics-public-queries-fixed.log, two pass; the million case remains deliberately ignored in this small selection |
| Only reifier counts change plan; actual EXPLAIN source | paged-statistics-public-provenance.log, one pass |
| Independent supplied rows and host physical work | paged-statistics-measured-host.log, one pass, all 100 answers retained |
| Fallible EXPLAIN receipt, cold/warm and faults | paged-statistics-receipt-adapted.log, three pass |
| SERVICE receipt and outcomes | paged-statistics-service-receipt.log, one pass |
| Strict affected libraries/tests | paged-statistics-final-strict.log, exit 0 with -D warnings |
| Native helper census | paged-statistics-publish-helpers.log, exit 0; no new exemption/checker/duplicate |

All task logs above live under tasks/ and retain terminal .exit files. Initial
failed fixture, frequency-policy, receipt-provenance and compilation attempts
remain recorded separately; they are not passes. No timing acceptance or broad
final gate is claimed. The native million-claim demonstration retains its scope:
this unit does not change its rows or returned counts. Whole issue completion
still requires segmented/effective-snapshot providers, rule ordering, structural
component estimates and final profile/portable/conformance qualification.

The paged unit is published at 2bb0691b2 through normal hooks and push.

## Native connected-order consumer

The next coherent unit consumes the shared snapshot provider through
Stratified::plan_with_statistics. One lowering home serves both metadata-free
and statistical planning; connectivity and binding preferences remain prior
to statistical tie-breaking. Caller probe resolution preserves native IDs.
The actual executable runs through the native semi-naive evaluator, changes
order from [0, 1, 2] to [1, 0, 2], retains eight facts and preserves its one
derivation's authored premises. All 31 affected plan-module controls pass.

Evidence: tasks/rule-statistics-native-query-fixed.log and
rule-statistics-plan-controls.log, both exit 0. Strict library/test compilation
and native helper census pass in rule-statistics-strict.log and
rule-statistics-helpers.log. The first test compilation used a nonexistent len
method; its original failed receipt is retained. Owner review:
tasks/rule-statistics-review.md, not independent approval. No blanket suite or
timing measurement was added. Whole-issue segmented/effective-snapshot,
structural component and final qualification requirements remain incomplete.

The native rule consumer is committed and pushed at 5bcff126e through normal
hooks. The current uncommitted slice adds complete stream totals and structural
candidate estimates, keeping residuals explicitly upper-bound and preserving
tighter indexed native ranges. Its initial production compile passes only the
first structural version; subsequent bound-subject/unbound-object refinements
and their extended runtime cases still require current-source qualification.
Focused public-query and metadata/allocation controls are live. This is not a
completed structural unit or whole-issue pass.

## Structural and residual estimate qualification

The current structural unit passes its affected requirements. The new default
stream-total and structural candidate methods account for all three RDF streams;
residual subject/object constraints are explicitly UpperBound. Native component
ranges can tighten the count. Per-probe row statistics travel alongside physical
work instead of being fetched again by cost-base. Backend documentation states
these semantics and the native rule planning entry point.

| Requirement | Evidence under tasks/ |
|---|---|
| Ten actual native/paged query executions across five structural/residual/unbound shapes, actual rows and EXPLAIN exactness | structural-statistics-current-public-query.log, exit 0 |
| Complete three-stream totals, graph scopes, translated domain property and compact refusal | structural-statistics-metadata-controls.log, five pass |
| Native and paged stream totals/structural/domain reads allocate zero | structural-statistics-zero-alloc.log, one pass, both measured windows zero |
| Native BGP model, supplied rows vs host work, structural joins | structural-statistics-bgp-controls.log, 37 pass |
| Survey, positive plans, term compilation/walk and deep-stack consumers | structural-statistics-survey-controls.log, 14 pass |
| Strict affected libraries/tests with warnings denied | structural-statistics-strict.log, exit 0 |
| Native census; original forty constructor proofs refreshed only | structural-statistics-helpers.log, exit 0 |

The earlier structural-statistics-public-query.log failed an exactness assertion;
it is retained and superseded by the current-source execution, not relabeled.
The initial first-check covers only its stated earlier source. Owner review is
tasks/structural-statistics-review.md; no independent approval is claimed.
The original million-claim predicate-bound/slot-object counts and rows are
unchanged, so that scoped demonstration is reused. No timing acceptance or broad
qualification was added. Segmented/effective-snapshot providers and final
profile/generated, portable and conformance qualification remain required.

## Persisted source statistics qualification

Data profile 2 carries authenticated, selectively readable predicate, frequency,
category and compressed distinct-domain records. The actual SegmentedSession
provider reads them through the original source, cache, admission and first-error
owner. Certification reconstructs RDF and rejects rehashed false statistics.
Opening remains header-only. This is a physical source provider; effective
chronological snapshot statistics are still required before issue completion.

Passing evidence under tasks/ (each terminal exit 0):

- segment-statistics-metadata-controls: six controls including persisted parity.
- segment-statistics-seal-controls-fixed: eight construction/certification cases.
- segment-statistics-varint-controls and segment-statistics-delta-controls: three
  cases each, including invalid overflow and retained valid edge behavior.
- segment-statistics-physical-merge-property: two actual persisted partitions
  versus a combined image, row totals and RDF-value-translated distinct unions.
- segment-statistics-strict-fixed: affected core library/tests, warnings denied.
- segment-statistics-explain-receipt-fixed: unchanged numeric/order/receipt test.
- segment-statistics-range-controls-final, actual-peak-control and
  original-fault-control: actual public query, allocation and failure behavior.
- segment-statistics-thousand-header-open and shallow-snapshot: original lazy
  open and historical allocation controls.
- segment-statistics-page-certification: all five affected page-summary cases.
- segment-statistics-helpers: native census, existing forty proofs refreshed.
- segment-statistics-wasm-check: standard wasm32 library compilation only.

The zero-case seal filter and original EXPLAIN failure are retained failures or
noncoverage, superseded only by the named corrected executions. The provider now
refines subject-bound marginal counts with its existing exact source range;
receipt assertions were not weakened. The unchanged million-claim demonstration
is reused only for its unchanged native fixture/count paths. No portable runtime,
timing acceptance or broad final gate is claimed for this unit. Owner review:
tasks/segment-statistics-review.md, not independent approval.

The persisted source unit is committed and pushed at 839018278 through normal
hooks. The next unit supplies chronological stack metadata through one common
combiner. Physical counts are upper bounds across overlapping/removal histories;
ordinary and annotation populations must be combined before either stream can
be bounded because declarations can change row classification. Local NDVs are
not summed. Bound values use the existing source translation path. Current
integrated production compilation passes; runtime qualification is in progress.

## Chronological stack evidence

Both real stack adapters now use the same conservative physical-statistics
combiner. All 37 affected stack cases pass in
`tasks/layered-statistics-stack-controls.log` (23 paged, 14 segmented; exit 0).
These include the new removal-only-head regression, source drift on metadata
point reads, exactness/upper-bound checks against actual streams, stable read
revisions and distinct publication prefixes, annotation promotion/demotion,
original header-only thousand-source open and shallow historical allocation.

Strict core library and both affected integration targets pass in
`tasks/layered-statistics-strict-fixed.log` (exit 0). Native census passes in
`tasks/layered-statistics-helpers.log` (exit 0). The existing forty constructor
proofs and one changed consuming source identity are refreshed with unchanged
contracts/sinks; no new exemption or checker is introduced. Initial unreachable
visibility and tuple-array lint failures remain historical failures.

The production SPARQL persisted reifier LIMIT/full-count control is running on
this source. Final portable execution, profile and whole-issue qualification
remain incomplete. No timing acceptance or independent review is claimed.

Chronological integration qualification is now complete for this unit:
`layered-statistics-persisted-query.log` passes the unchanged public SPARQL
2,000/200,000-claim LIMIT/full-count and block-layout control (exit 0),
`layered-statistics-paged-public-query.log` passes all eight actual paged-stack
consumer controls (exit 0), and `layered-statistics-wasm-check.log` passes
standard wasm32 library compilation (exit 0). Portable compilation is not runtime
execution. Owner review: tasks/layered-statistics-review.md. Final profile,
projections, portable runtime and whole-issue acceptance are still required.

The chronological unit is committed and pushed at 1a2e326a7 through normal
hooks. Standard scalar WASM execution now passes three exact original cases in
tasks/statistics-scalar-wasm-runtime.log (exit 0): chronological annotations
with the new statistics assertions, lazy source-scoped IDs, and thousand-source
header-only open. Cargo uses the existing repository WASM runner and standard
target directories, with no compiler override or tooling edit.

Final reconciliation increments the governor profile from 19 to 20: snapshot
statistics can change join order and therefore charged work/cut points even
though the point schedule and complete answers remain unchanged. The profile
identity control is running. New critical #556 is scheduled after this delivery;
its ordinary-row classification contract is separate from these conservative
statistics and is recorded in the root REWORK_LIST.md queue.
