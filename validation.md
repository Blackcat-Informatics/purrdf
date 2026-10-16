Final merge readiness: PASS. Hosted run 38876291016 on 225e078da completed successfully: 49 successful checks and four expected skips. Both material review threads are resolved; final-review-threads.json binds their dispositions to this head. Current-head review has no actionable findings. The clean integration tree is 5a3000f31bfaa2063190de341c77e23fb58e9f02. All affected acceptance passes; historical pending statements below are superseded by this result.

Final integration assessment: fresh origin/main remains an ancestor of published 225e078da. Clean merge-tree predicts 5a3000f31bfaa2063190de341c77e23fb58e9f02, with no unqualified combined source delta. Both actual material review threads are resolved, and current-head review reports no actionable findings. The generic docstring quota and generated follow-ons are declined with the documented public-contract rationale posted at https://github.com/Blackcat-Informatics/purrdf/pull/561#issuecomment-6212302455. Current CI still has three test jobs in progress; no merge readiness is claimed until required checks finish.

# Current non-evicting routing correction

Final affected acceptance is PASS. All 18 segmented and 15 stack cases, all 21
segmented units, strict affected compilation and the original full eight-lane
consumer pass. Both scalar and SIMD WASM pressure executions pass through the
repository's standard runner; each executes the existing case with both cache
sizes. All non-evicting-routing native/units/evaluator/strict, wasm-scalar-final,
wasm-simd and helpers exit files are zero. The first scalar attempt omitted the
runner and returned 101 without executing; it is not coverage. Final fresh census
retains all 40 proof contracts with only their constructor-source closure updated.
Formatting/diff checks pass. Normal commit/push pass at 225e078da. Current hosted run 38876291016
is active; final current-head checks/review remain. The earlier running wording below is historical.

New material finding 4292239479 is reproduced on published a94af3ab2. The existing
pressure case now retains its original two-slot case and adds a 128-slot case.
With the same 2,000,000-byte ceiling and 4096 spare bytes, the published reader
grows optional routing bookkeeping from 94,624 to 102,944 by evicting payloads.
non-evicting-routing-repro.log/.exit records the actual failing assertion (101).
No later request-ledger failure is claimed as reproduced by this case.

The bounded correction permits reserve_buffer eviction only for mandatory cache
payload/bucket and request-ledger admission. Optional routing growth returns its
own Residency to the existing caller, which skips the hint and keeps the admitted
payload. Other errors retain their original latch. The same admission home and
authenticated lookup remain authoritative; no new helper/body or directory is
introduced. Documentation makes spare-capacity-only growth explicit.

Affected native/stack, segmented units and the original full eight-lane consumer
are running in the non-evicting-routing logs. Strict checks, actual scalar/SIMD
pressure executions and a fresh source index are also running sequentially.
Source-bound registrations/helper checks and normal publication remain. Current
hosted run 38870539013 covers the prior published source, not this uncommitted
production correction; it cannot establish readiness for the new delta. No merge
is attempted with the material finding open. Earlier results below keep their
recorded source applicability and are reused only where unchanged.

# Earlier combined-source qualification

Combined-source affected acceptance passes. Original full eight-lane evaluator
control: combined-evaluator.log/.exit, zero; unchanged 256 MiB, populations,
LIMIT/key/COUNT assertions. All 18 segmented and 15 stack integrations pass in
combined-native.log/.exit. Strict affected compilation passes in combined-strict.
Twenty of 21 segmented units pass in combined-units; the one failure looked up
the fixture's former short IRI after this branch made its terms long. Its exact
lookup now matches the long authored term, and the unchanged selective metadata,
count, no-domain-payload and first-refusal assertions all pass in
combined-statistics-fixture.log/.exit. This last delta changes only test input;
shipping-code results and the other 20 unit results remain applicable.

Fresh constructor/builder source registrations and helper hygiene pass after that
fixture correction (combined-fixture-helpers.log/.exit, zero). No proof class,
sink, reason or exemption is changed. Formatting/diff checks pass. Original
native/scalar/SIMD pressure controls retain their recorded source scope; the
combined native case passes among the 18, and the current hosted gate must cover
the final portable artifact. Normal merge commit and push pass at a94af3ab2. Hosted CI run 38870539013
is in progress; final current-head checks/review remain. The older pending wording below is historical.

PR #557 is merged and #484 is closed at bfb5cb367. This delivery now integrates
that actual landing. Production files merge cleanly, preserving profile 2's four
persisted statistics streams and semantic certification alongside variable-size
dictionary blocks and authenticated lazy routing. The only Git conflicts are
capacity proof registrations. Fresh combined census supplies the 40 constructor
bindings and four builder source authorities; proof classes, sinks and reasons
are unchanged. The original eight-lane 2,000/200,000-claim consumer at 256 MiB is
running in tasks/combined-evaluator.log. Fresh combined helper hygiene is running.
No combined-source pass is claimed before terminal results.

The optional routing correction is published at ee13e8327. Follow-up 57a12dbd1
moves the single pressure regression into the existing supported portable target
and refreshes source registrations, without changing shipping behavior. Evidence:

- review-routing-units.log/.exit: 20 native units pass before the test move;
  remaining production and 19 unit bodies are unchanged by that move.
- review-routing-native.log/.exit: 32 native integration cases pass.
- review-routing-evaluator.log/.exit: all original eight consumer lanes pass.
- review-routing-public-native.log/.exit: the public pressure case passes.
- review-routing-public-wasm-scalar.log/.exit and
  review-routing-public-wasm-simd.log/.exit: that same case actually executes and
  passes on both portable configurations, with 17 other cases filtered.
- review-routing-strict.log/.exit, review-local-pin-strict.log/.exit and
  review-routing-helpers-final.log/.exit: affected strict and helper checks pass.

All listed terminal results are zero. The earlier --lib WASM attempt is 101,
unsupported by the existing integration runner and outside the pressure case;
it establishes no coverage. No runner/tooling workaround was introduced. Earlier
17-case portable passes apply only to their unchanged paths. Current combined
profile changes invalidate the physical consumer result and are being checked
before publication. Full current-head hosted qualification/review remains.

The following historical sections retain their recorded source scope; this
current section supersedes their stale publication and pending-result wording.

# Current implementation

## Final retained-routing repair: all affected acceptance passes

## Review pressure regression and bounded correction

The new actual-reader regression fills the learned range table, admits space for
the next decoded payload, and holds the remaining workspace. Published behavior
fails with Residency { requested: 2006315, limit: 2000000 } at term_block, matching
review comment 4291214207. The complete reproduction is
tasks/review-routing-repro.log/.exit (exit 101). Original populations and budgets
are not weakened.

The current uncommitted correction omits only a new optional routing hint when
its own index growth is refused by the live-byte budget. Its payload and exact
request ledger have already been admitted. The original authenticated lower-bound
search remains the one lookup path. All other errors propagate and retain their
original first-cause latch. cache_insert receives the admitted-hint decision and
cannot insert unreserved routing entries. Documentation now makes this policy
explicit. The regression also checks unchanged index capacity, retained healthy
state, peak <= ceiling, exact value readback and healthy subsequent progress.
Affected unit execution is running in tasks/review-routing-units.log; no pass
is claimed until its terminal result. Final workload/portable/strict/helper
qualification and normal publication still remain for this delta.

PR 561's shared compatibility fixes are committed through ee96cc213 after normal
hooks. The failed workspace log was finally retrieved in
tasks/first-workspace-ci.log: it confirms the same two deprecation metadata
errors in the evaluator as PR 557. The three item-scoped allowances and equivalent
stemmer range are carried here without changing runtime bodies or warning policy.
Two reader comments now explicitly describe lazy learned bounds rather than
incorrectly implying that no routing metadata is retained.
A new hosted material finding (4291214207) concerns tight-budget
routing-table growth and is OPEN in review-debt.md. Existing full workload passes
remain valid for their recorded source and budgets; they do not discharge this
additional pressure case. No final readiness or closure is claimed.

The final reader retains source-qualified routing fences learned only from
authenticated payload reads. These own no lexical bytes or payload pins and
survive payload eviction under the operation's original accounted Memory grant.
Header-only open and historical snapshot construction still allocate no eager
dictionary directory. Adjacent authenticated ranges narrow cold probes using
the original dense-ID and maximum-record-count laws. Reloading an existing range
does not reserve duplicate entries. Exact missing-entry admission follows decode
while the pending payload is charged and precedes cache publication.

Final evidence, all exit 0:

| Criterion | Evidence under tasks/ |
|---|---|
| All 17 segmented and 15 stack native controls | retained-routing-native.log |
| All 19 segmented units, exact pending allocation and cross-source/bucket identity | retained-routing-units.log |
| Original two populations and four layouts, full LIMIT/key/COUNT at 256 MiB | retained-routing-evaluator.log |
| All 17 actual scalar WASM executions | retained-routing-wasm-scalar.log |
| All 17 actual SIMD-enabled WASM executions | retained-routing-wasm-simd.log |
| Affected strict core library and both integration targets | retained-routing-strict.log |
| Fresh helper census with original contracts and rebound source identities | retained-routing-helpers.log |

The original 200,000-claim 4096/128 COUNT now returns 400,000 rows with 996,135
provider requests and 169,204,172 peak bytes. All eight lanes pass without changed
budgets, populations or assertions. The high-ID fixture crosses a 256-ID bucket;
dense streaming reads one header plus exactly 80 payloads. Warm two/three-page
reuse adds no provider requests. Cache-size-one eviction/reload needs exactly one
payload read with no duplicate routing-fence growth. Old failures below are
historical and explicitly superseded by these final scoped results.

No timing acceptance, custom toolchain/directory, runtime dependency or duplicate
implementation is introduced. Normal publication and complete hosted CI/material
review remain before merge; the original full local suite is not repeated.

Current source has an uncommitted resident dictionary-range index replacing the
insufficient two-position proposal. It indexes at most two fixed-width ID buckets
per resident dictionary page, qualified by the certified snapshot. The existing
cache owner admits each physical allocation through its original grant, and
normal eviction removes range entries without retaining a payload pin. Primary
cache/payload admission precedes I/O; exact range-index growth follows authenticated
decoding while the pending payload is charged, before cache publication.
The initial compile found
an ambiguous closure error type and missing test-only evidence identity; both
are corrected. The initial resident-index revision passes all 32 native integration
cases, including the stronger three-page warm-cache control. Eighteen of 19 units
pass; the exact pending-block control found unnecessary two-entry reservation.
Its correction now admits only the authenticated page's actual one or two entries.
The focused admission rerun passes (range-index-exact-admission.log, exit 0).
The initial resident-index full evaluator run fails, with source captured in
tasks/range-index-before-admission.patch. Its exact-hit index did not prevent
cold-search cache churn. The reader now also narrows cold searches using nearby
authenticated resident ranges and the original dense-ID/count-cap laws.
All 32 native integration cases pass on that reader (nearby-range-native.log,
exit 0), including one header plus exactly 80 sequential payload reads and
read-free two/three-page warm reuse. A further high-ID bucket-boundary fixture
check, 19 units, portable execution, refreshed helper index and the original full
evaluator control are running. Final strict source qualification remains before
publication. See tasks/resident-range-review.md. No complete correction pass is claimed.

The previous two-position proposal did not resolve the evaluator failure.
Its public two-page warm-cache regression,
all 19 segmented units and affected strict Clippy pass (tasks/positions-native.log,
positions-unit.log and positions-strict.log, exit 0). The full evaluator rerun
still fails (positions-evaluator-layout.log, exit 101). No whole-delivery pass or
PR is claimed. The actual-entry request probe is complete and its temporary
source edits have been removed. The original two populations and four layouts
are restored in the evaluator test. Retained diagnostic evidence is in
tasks/count-read-diagnostic-ready.log (runtime refusal, exit 101) and the replay
patch tasks/count-read-diagnostic.patch. It reports 251,031 forward-dictionary
reads, 12 reverse-dictionary reads, 9,608 row reads and 18,969 summary reads.
The stronger native regression with three resident dictionary pages fails
(tasks/positions-three-pages.log, exit 101): requested bytes grow from 160 to
10,400 despite initially pinned warm payloads. Two remembered ranges are
insufficient. This failure remains visible in the native test; do not increase
the budget, fixed hint count, or acceptance threshold. The next correction must
find resident ranges through bounded, source-qualified, accounted cache state.
Previous complete native/WASM passes are scoped to the published pre-correction
source. Cross-source unit, refreshed native/portable/evaluator/helper coverage
remain before another commit or whole-issue publication.

Issue, comments and prior art are read. The initial fetch timeout was recovered
by the completed Stage brief. The source confirms both production failures and
the fixed-population reader/certifier dependencies. The owner plan review covers
the full contract. The complete issue remains open; byte-aware packing is next.

## Demand-driven construction

The constructor retains its public Result and all explicit bounds, but no longer
reserves the declared ceilings. Two focused native tests pass: maximum accepted
limits allocate nothing at construction, input allocations match small limits,
three-row persisted content and high IDs round-trip identically, and refused
term/row ingress leaves the accepted prefix intact. See tasks/demand-native.log
and tasks/demand-refusal.log (both exit 0).

Affected strict compilation passes (tasks/demand-strict.log, exit 0). The initial
helper check failed because four pinned sink line numbers moved by six lines.
The unchanged sink expressions, interval checks and minting paths were reviewed;
their line locations and actual constructor/source identities are reconciled.
The standard helper check now passes (tasks/demand-helpers-reconciled.log, exit 0).
No proof class, reason, exemption or checker behavior changed. This is owner
assessment, not independent review. Publication and complete-issue acceptance
remain separate. Task 1 is committed and pushed as `66974d6b3`; normal hooks pass.

## Byte-aware packing and consumers

The existing encoder admits complete records at their actual front-coded size,
including the first-key fence and framing. Its caller advances by the consumed
count; the original count is an upper bound. Header admission permits independent
forward/reverse counts within nonempty population bounds. Full certification
still checks dense insertion IDs, exact total cardinality and complete reverse
correspondence. The reader uses authenticated ID ranges without a directory;
the count quotient is only a safe lower-bound first probe. Existing full-count
images keep one-probe lookup. No new wire version or alternate encoder is added.

Initial integration run: all 16 cases pass (tasks/packing-native.log, exit 0),
including long alternating terms, high IDs, complete streamed output, existing
pin/cache/evidence/refusal/source-fault controls and long RDF 1.2 side tables.
Unit run: all 18 segmented tests pass (tasks/packing-unit.log, exit 0), including
exact 433/449/450-byte boundaries, front coding and recomputed-hash semantic
attacks on variable-population images. Later lookup narrowing moves only after
the in-block bounds check; final affected integration coverage is running.

Strict production and both affected core integration targets pass
(tasks/packing-strict.log, exit 0). The final integration target now also asserts
different actual forward/reverse block counts, nonempty capped populations and
typed single-record overflow. All 17 segmented and 15 segmented-stack cases pass
on the final lookup source (tasks/packing-native-qualified.log, exit 0). This
includes complete 80-term forward/reverse round-trips, independent block counts,
one-record pages, capped multi-record pages, sparse access, streamed output,
RDF 1.2 graph/reifier/annotation behavior, original allocation and source faults.
Portable execution, refreshed helper registrations and the actual evaluator
layout consumer were pending. Refreshed helper hygiene now passes on current
source (tasks/packing-helpers.log, exit 0). The source index changed the constructor
closure and segmented builder owner identities; all 40 constructor bindings and
the four unchanged interval proofs now bind those actual identities. Sink
locations/classes/reasons remain unchanged. Portable execution has started using
the repository runner and standard Cargo target directories. The actual evaluator
layout consumer remains pending. Available memory initially fell below the
plan's 32 GiB launch floor, so no further sizeable job was launched. The existing
statistics assembly process and private geo process remained live and preserved;
work resumed when available memory recovered above 32 GiB.

The evaluator's 4096/128 control previously expected the whole-group refusal.
That expectation is removed so the existing real file-backed LIMIT, bound-key
and full COUNT checks must now succeed at that layout too, for both 2,000 and
200,000 claims. No population or acceptance threshold was reduced. Historical
layout measurements in the design note are labelled at their original scope;
the old refusal is not claimed as current behavior.

Task 2 is committed and pushed as `4b031fbac`; normal hooks pass. The scalar
WASM run executes all 17 segmented cases successfully using the repository runner
and standard Cargo target directories (tasks/packing-wasm-scalar.log, exit 0).
SIMD-enabled WASM also executes all 17 cases successfully
(tasks/packing-wasm-simd.log, exit 0). This is actual execution in that compiler
configuration, not a claim about an explicitly selected deflate vector kernel:
the record encoder calls the shared inline common_prefix_len home.
The actual evaluator layout consumer is running at its original populations.

## Evaluator finding and correction in progress

The complete evaluator control failed (tasks/packing-evaluator-layout.log,
exit 101). All four 2,000-claim layouts pass, and 200,000 claims at 4096/32 and
4096/64 pass. The new 4096/128 image seals to 64,860,160 bytes and its LIMIT
touches 21 terms, but full COUNT exhausts the unchanged 256 MiB budget as the
request ledger grows past 4,194,304 requests. Repeated forward-directory searches
for already cached variable-population terms cause extra authenticated reads and
evictions. The 200,000-claim 8192/128 lane was not reached; the full run is not a
pass. Original populations, budgets and assertions remain unchanged.

The bounded correction keeps two source-qualified last-used dictionary ranges in
the existing shared reader State. They retain no block pins and allocate no map
or directory; sizeof<State>() is already admitted by the operation owner. A hit
still reads through the same authenticated block/cache authority. Full-count
first-probe results do not displace these variable-population positions. The
actual query's alias cache already memoizes imported terms; reconstructing new
quoted values still alternates fresh subjects with reused literal components,
which is why range reuse must survive that alternation.

An actual public-reader regression alternates two warm variable-population terms
and requires unchanged provider-request and eviction counts. A composed-operation
unit uses overlapping IDs with different physical page addresses and snapshots,
so a foreign range position cannot reinterpret an ID. The correction and these
controls are uncommitted and unverified. Available memory fell below the 32 GiB
launch floor again; no new build is started below it. Refresh affected strict,
native, portable, helper and complete evaluator coverage after the correction.

The complete rerun with two positions still fails at the same 4,194,304-request
ledger growth. The 2,000-claim request count improves only from 14,400 to 14,130;
that does not establish the full cause or solve the 200,000-claim failure. The
source-qualified unit and all 19 targeted segmented units pass, so the positions
are safe for their demonstrated behavior but insufficient as the repair. Do not
increase the position count, cache size, memory budget or acceptance threshold
without identifying actual read traffic.

Temporary diagnosis is confined to the actual evaluator test: select its failing
200,000-claim 4096/128 lane at the original 256 MiB budget, classify its recorded
data requests by actual persisted block kind, and count the hottest slots after
the real refusal. The exact temporary patch is tasks/count-read-diagnostic.patch;
execution is tasks/count-read-diagnostic.log. This is a diagnostic run, not a
qualification pass or an approved smaller population. Restore the original two
populations/four layouts and remove the diagnostic buffers/output before any
commit. No executing production/assembly source was changed for this probe.

The live statistics branch adds header streams but retains the same old fixed
dictionary assumptions. Its appended stream descriptors do not move the existing
dictionary/reverse descriptors. Integration must preserve both its statistics
checks and these variable-population bounds; no dependency on its unfinished
qualification is introduced.
