# Fallible evidence acceptance

Implementation is committed at d9f5163bc with the normal hook passing and is
pushed in PR https://github.com/Blackcat-Informatics/purrdf/pull/684. Stagectl
created the PR but failed parsing its response; direct forge readback verified
the correct open PR, head and base, without repeating creation. Independent
automated review covers all 12 files from 217fbf9a5 through d9f5163bc and reports
no actionable comments. Hosted qualification remains in progress; no merge or
issue completion is claimed.

Current published head is bbfd189ca294371deab3a48ec177b6989824c4fd. It changes
only the host refusal assertion described below; production remains the reviewed
d9f5163bc implementation. All 49 revised-head hosted checks now pass, with four
configured skips and no waiver. The independent reviews cover all 13 files,
including the host correction. Complete feedback and the separate documentation
heuristic disposition are in pr-comments.md. The final clean merge candidate is
7ec33ec25551337fbd4340ee9721097ce5bc0d51 against unchanged main 217fbf9a5.

## Development history and qualification

The records below describe their actual original execution states. The current
verdict is above; later explicit passes supersede failed attempts, never relabel
them as successful executions.

- raw/evidence-doors-check.log: session 73937, exit 0. Evaluator all-target
  compilation after sharing the original admitted body and adding the four public
  fallible evidence doors. Later source adds explicit operational sequencing and
  real tests, executed below.
- raw/evidence-fallible-native.log: session 54089, exit 0. All 22 evidence-ingress
  and 17 fallible-query cases execute. The new healthy control reaches all four
  query forms and three answer tiers over real PagedQueryView storage, inspecting
  original claim rows and operational receipts. The refusal control fails the
  second page after one successful page across answer, status, interned and
  prepared witnessed doors. It preserves the exact original provider error,
  requested-page sequence and consumed-page count, with no successful caller value
  or partial answers. The retained prepared execution succeeds against a healthy
  source afterward. Existing resident evidence and fallible semantics also pass.

The original faulting provider fixture moved intact into tests/support/fallible.rs
and is shared by both targets. No copied page fault implementation or fabricated
successful result replaces the original provider behavior.

The retrieval implementation wires search_scoped_fallible through the original
search_bound, execute_bound, DatasetExclusion and fusion bodies. The public
ReadOperation shares the original typed QueryWorkspace; with_fallible_read keeps
the original admission guards through the awaited composition and checks the same
finish_fallible_read boundary before publishing. Unbounded fallible scopes retain
direct non-sticky source errors in an admitted shared slot; bounded scopes keep
their original account. Existing infallible entry points reuse this evaluator
body with their original capability and no new fallible slot.

- raw/retrieval-fallible-check.log: session 94005, exit 0. Both evaluator and
  retrieval production libraries compile. This does not prove runtime behavior.
- raw/fallible-scoped-real-producers.log: session 59942, exit 101. The shared test
  EvidenceSource fixtures needed GraphMatch<I> after becoming generic over the
  actual source IDs. Both signatures were corrected.
- raw/fallible-scoped-real-producers-2.log: session 42674, exit 101. The new
  fixture's concern was an IRI instead of the evidence contract's required triple
  term. The existing resident control refused it before the fallible call. The
  fixture now supplies the same valid quoted concern shape as existing controls.
- raw/retrieval-fallible-format.log: session 37184, exit 0. Formatting after the
  composed read implementation and fixture correction; later small lint fixes
  still need the final format check.
- raw/fallible-evidence-retrieval-strict.log: session 98549, exit 101. Two needless
  double references and the shared query helper's argument count were corrected.
  The second attempt (session 52673) exposed a misplaced borrow edit, now corrected.
  raw/fallible-evidence-retrieval-strict-3.log: session 30091, exit 101, one missing
  assertion semicolon in the new evidence fixture, now fixed. No strict pass claimed.
- raw/fallible-composition-native.log: session 94983, exit 101. All 21 existing
  real-producer cases passed; the new segmented builder fixture used a block size
  below the documented 512-byte minimum. Corrected the fixture to that minimum.
- raw/fallible-scoped-real-producers-3.log: session 9526, exit 0. The actual new
  segmented-stack search case passes: resident rows, scores and evidence identity
  match; semantic unknown survives; a provider refusal after candidate evaluation
  starts returns the original SegmentedError::Provider with host code 71.
- raw/fallible-composition-native-2.log: session 41526, exit 0. All 22 real-producer,
  22 evidence-ingress and 17 fallible-query tests execute and pass on the shared
  read-owner implementation. No test filter or zero-case target.
- raw/fallible-scoped-late-controls.log: session 37949, exit 101. The new deferred
  exclusion/final descriptor tests needed to move their operation into the async
  future while borrowing the original query inputs; the captures were corrected.
  raw/fallible-scoped-late-controls-2.log: session 98560, exit 0. Both actual
  segmented cases pass. Deferred exclusions cannot lose their original read error
  even when a consumer catches the stream stop; descriptor drift after all streams
  succeed is refused at the final checkpoint. The lower scoped
  executor is now public for composing streams within with_fallible_read; this is
  the same function the search pipeline already uses.
- raw/fallible-composition-format-2.log: session 71349, exit 0. Current source
  formatted after late controls and the existing verdict matrix's extension to
  both resident and real paged fallible entry points.
- raw/fallible-evidence-retrieval-strict-4.log: session 60632, exit 0. Both affected
  crates and every target pass clippy with warnings denied.
- raw/fallible-composition-native-final.log: session 79195, exit 0. All 23
  real-producer, 22 evidence-ingress and 17 fallible-query tests pass. The original
  verdict/summarization matrix now runs through the fallible door over real paged
  storage as well as the resident control, retaining all four query forms, signed
  information, poison-suffix stopping and declared unknown absence.
- raw/fallible-composition-ownership.log: session 91793, exit 0. All 65 original
  workspace ownership regressions execute, including the new direct-error slot
  test that panics if the original error is formatted, and existing first-cause,
  allocation refusal, extraction, clone and final-owner release controls.
- raw/fallible-composition-wasm-scalar.log: session 64809, exit 0. All 23 real
  producer and 22 evidence-ingress cases execute under scalar WASM through the
  shipped runner, including actual segmented and paged failures and late refusal.
  The ordinary fallible_query target uses the standard native libtest harness;
  its 17 native cases are not claimed as portable execution.
- raw/fallible-composition-wasm-simd.log: session 88578, exit 0. All 45 cases pass. Same actual
  portable targets with the Makefile's target-scoped +simd128 and warnings denied;
  no semantic feature or profile weakening.

- raw/non-latching-evidence-before.log: session 37096, exit 101. New actual public
  evidence-door regression over the existing point-fault fixture reproduces the
  lost original cause when an unbounded adapter returns a direct error without
  latching it: Query with rendered native-sparql-source-read instead of Operational.
  Such an adapter must never cause the boundary to publish success or erase its
  returned operational cause. The same regression calls all four evidence doors.
- raw/non-latching-evidence-format.log: session 28043, exit 0. Formatting after
  moving direct-error capture to the shared reserve_fallible_execution ingress.
  Capture also now preserves an already-created slot when admission repeats.
- raw/non-latching-evidence-ownership-after.log: session 95576, exit 0. All 66
  ownership controls pass; the new regression executes all four public evidence
  doors and preserves the original non-latching read cause, with no cause clones.
- raw/non-latching-evidence-strict.log: session 91063, exit 0. Both affected crates
  and every target pass warnings-denied clippy on the corrected source.
- raw/non-latching-evidence-composition-after.log: session 8341, exit 0. All 62
  native composition cases pass after the shared-ingress correction.
- raw/non-latching-evidence-wasm-scalar-after.log: session 78749, exit 0. All 45
  actual portable evidence/producer cases pass on the corrected source.
- raw/non-latching-evidence-wasm-simd-after.log: session 32006, exit 0. All 45
  cases also pass with target-scoped SIMD enabled on the corrected source.
- raw/fallible-evidence-helpers.log: session 11789, exit 0. The repository's
  helper census, single-home and hash-domain gate with its refusal self-tests:
  85 enforced jobs, 120 distinct groups, 145 unique prefix-free hash domains.
- raw/commit.log: session 34221, exit 0. Normal signed commit d9f5163bc, all
  configured hooks run without bypass; 12 explicit source/test paths and no
  Stage process records included.

The current source has 62 native composition cases, 66 ownership cases, strict
affected all-target qualification and 45 cases on each scalar and SIMD WASM.
No full hosted gate, independent review or merge pass is claimed yet.

The unchanged .github/workflows/ci.yaml checks parity with make check, runs full
workspace check/clippy and all hygiene gates, partitions the complete workspace
test surface into its seven governed shards, and builds release crates on WASM.
Use those current-PR hosted gates for broad qualification, alongside normal local
hooks and focused helper checks; they remain required and unverified until they
actually pass. Local focused results are not a full workspace gate claim.

Hosted run 39863553027 failed integration shard 2 at the existing host_identity
certificate test, which explicitly required an unbounded non-latching refusal
to become Query with a rendered diagnostic. That expectation conflicts with the
new common ingress preserving the original typed operational cause. The raw job
log is retained in raw/hosted-integration-2-failure.log. No gate is waived. The
test now requires Operational with the exact original Deleted(low) for both
bounded/unbounded and refusing/sticky certificates, retaining numeric/temporal
values, Arc forwarding, certificate counts and final pin-release assertions.
raw/host-identity-typed-refusal.log: session 68099, exit 0, all six host cases
execute. Production source is unchanged from the independently reviewed commit.
This assertion correction must be committed/pushed with normal hooks and all
required hosted checks refreshed for the resulting head before merge.

- raw/host-identity-format.log: session 87268, exit 0, current formatting check.
- raw/host-identity-strict.log: session 56496, exit 0, affected host target with
  clippy warnings denied. Full affected all-target production checking remains
  valid from the unchanged original implementation; this is a one-test change.
- raw/host-identity-wasm-scalar.log: session 80733, exit 0. All six original host
  cases execute on scalar WASM, including the stronger original-cause matrix.
- raw/host-identity-wasm-simd.log: session 67378, exit 0. The same six cases
  execute on SIMD WASM with the Makefile's target-scoped flags and warnings denied.
- raw/host-identity-commit.log: session 60014, exit 0. Normal signed commit
  bbfd189ca changes only the host test assertion; all hooks run without bypass.
- raw/host-identity-push.log: session 64436, exit 0. Normal push advances PR #684
  to bbfd189ca. No source history rewriting, waiver or gate bypass.

Remaining: ghprsq publication, actual integration/archive readback and supported
cleanup. Hosted CI, complete feedback and the integration assessment are settled.
All commands preserve normal toolchains/profiles and use
eight build jobs with four test and Rayon threads. One owned Cargo command runs
at a time.
