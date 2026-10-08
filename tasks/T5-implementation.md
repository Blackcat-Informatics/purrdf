# Task 5 implementation and qualification (in progress)

Parent source 3ec2dbba8. Original Rust source added: CLI rules_campaign and
rules_alloc examples, Datalog factor_allocation and portable rules_runtime
integrations, and their explicit target registrations. No shipping dependency,
feature, default, or CLI dispatch changed. Task5 also repairs production joins:
initial variants with provably empty required OldOnly suffixes are skipped by
both physical kernels. Exact original pair work changes 27N to 9N and the positive
two-Spool neighbor 41N to 18N, with all authored premises retained. Native
regressions, including later partial deltas, have passed. Source-dependent
consumer/wasm/release checks passed after this fix. Root retains Git/forge ownership.
The actual timed release campaign passed all20 production/observer cases; all9
existing microbenchmarks completed. Current measurements and evidence boundaries
are in T5-campaign-results.md, with conditions in T5-campaign-admission.md.
Independent review, normal commit/push and Task6 full gate remain required.

## Historical collector preparation and pre-repair captures

The evidence below records intermediate checks. Its 6N/27N work, consumer and
wasm checks and release binaries predate the required empty-suffix repair;
they are not qualification of the final production source. Current repair and
source-dependent results are recorded in the final section.

The Linux collector generates the issue namespace/term-length Turtle corpora and
single/pair/notype SRL rules. It invokes the actual production binary with ordinary
rules --srl --explain=... INPUT OUTPUT argv, retaining stdout/stderr/status, exact
argv, binary BLAKE3 identities, source HEAD/diff/new-tool bytes, root manifests and
lock, rustc -Vv/Cargo/host/disk observations and per-child before/after load. Linux
wait4 owns the exact child, retries EINTR, records actual child peak ru_maxrss in
KiB, and refuses failed children. A separate observer executable calls the SAME
public purrdf_cli::run with CountingAllocator/WholeProcessWindow; its allocator
traffic/live retention/peak span receipt is distinct from production elapsed/RSS.
Both executable targets explicitly disable automatic test execution.

All emitted facts, every complete proof block and its authored premises, exact
inference counter and channels are checked. Empty pair output/proof must be truly
empty. The collector self-test plants omitted premises, duplicate proofs, junk
prologues/trailers, extra facts and unexpected stdout. No currently passing
self-test was claimed at this initial preparation point. Production tenfold
input growth must remain below a conservative twentyfold time/RSS envelope; these
are empirical observations, not a statistical claim. A two-spool positive pair
neighbor at 1000 vaults makes the pair proof check nonvacuous without pretending its
fanout has the same work count as the original 100k one-spool acceptance corpus.

Actual factor allocation integration passed at N100/1000/10000. Its input and
compilation are outside the whole-process allocation window; production evaluation
and returned state are inside. Exact work is 3N. Requested bytes are
994911/9762171/101923859; retained bytes 66136/615316/6739660; peak working spans
138450/1342194/13377650. This verifies additive traffic/state growth and is not RSS.
All 333 Datalog library tests also passed. Settled portable native integration passed
both connected/authored-premise and independent/canonical-existential cases.

The connected runtime exact 6N count was traced rather than inferred from a
changed assertion. All seeds are new initially: last-new-atom variants
Delta/Old/Old, Full/Delta/Old and Full/Full/Delta admit N,2N,3N frames respectively;
only the last variant emits heads. Round two result-predicate delta addresses
none of the body predicates, so delta_can_match skips every variant. The initial
3N assertion omitted the aborted old-suffix variants and failed; it was corrected
with this source explanation. This historical 6N behavior was subsequently
repaired to 3N by empty-suffix elimination; no limit changed.

Settled examples clippy and Datalog all-target clippy passed warning-free with
CARGO_BUILD_JOBS=8 and --jobs8. Initial example compile found unreachable public
visibility, and initial clippy found collapsed if/format collection; all fixed
without lint suppression. Logs under tasks/T5-logs preserve failed and settled
runs distinctly. Actual release build is underway. Full Task5 qualification and
independent review remain required before commit/push; Task6 owns the sole full
makecheck gate.

Release preparation finished successfully: normal release CLI, collector and
observer build in 4m10s, with captured Cargo JSON artifact profiles/paths.
Collector adversarial self-test passed. Owned heavy lane released to root;
consumer/wasm/bench preparation and serialized actual CLI campaign remain next.

## Historical collector and consumer preparation before the required repair

Collector v2 strictly parses four unique required allocation metrics as u64/u64/
i64/i64; zero traffic or nonpositive measured peak, malformed/missing/unknown/
duplicate/truncated fields hard-fail. Signed retention remains valid. Its TSV indexes
all four metrics and compares actual requested/peak growth for each original rule.
Ten allocation poison controls and the fact/proof/channel controls passed in the
immutable copied v2 executable. The initial copied v1 self-test correctly refused
its absent Cargo CACHEDIR ancestor; v2 explicitly takes a scratch root through the
existing testkit TempDir constructor. All captured v1 binaries remain preserved.

Actual shapes checks passed: 6 capacity, 3 rule corpus, 26 rules-engine,
1 serialization and 23 SRL tests. CLI shapes_tools_cli passed all 14 tests over
the built native binary. Settled entail consumer checks passed: 683 library,
23 corpus oracle (one intentionally ignored maintainer generator), 11 frozen OWL
oracle (two intentionally ignored upstream diagnostic generators), 3 pack parity,
47 reasoner and 2 ordering tests. Ignored generators are not runtime coverage.

The first entail run exposed stale v1 contract pins, with 682/683 library tests
passing. Actual current wasm/native regime pins were captured by a temporary
focused probe, then the probe was removed and frozen pins updated. The contract
documentation now includes versioned evaluator semantics, matching its actual
shared Datalog hash recipe. Authored clause programs remain unchanged.

The original authorized Rust oracle generator regenerated all 142 first-party
goldens. A read-only Rust audit against HEAD proves that only contract hashes,
join-step counts and an original-generator-owned transition header changed;
every other byte (facts, witnesses, rule tallies, stored facts, arena bytes) is
unchanged. D charges its 32 premise-free datatype identity rows per graph;
OWL-RL charges 43, adding nine annotation and two class rows. Named graph cases
multiply the charges across two/three actual graphs. Four collection fixtures
save 39/40 candidates under connectivity priority, as a narrow priority-only
control isolated. The temporary planner control and test were restored/removed
before settled qualification. Exact audit/control/generator logs are retained.
No external/frozen GTS vectors were edited. Wasm build/runtime and bench preparation
are currently required; actual release scaling/performance remains unrun.

## Required empty-suffix repair and refreshed qualification

Actual affected wasm builds for Datalog, shapes, entailment and purrdf-wasm
passed, and Node executed both initial shared-kernel rules_runtime cases.
These are actual wasm runtime checks, distinct from public JS package acceptance
and the final whole-workspace gate. The existing nine-case seminaive bench was
prebuilt and captured as a verified immutable task-owned binary; no benchmark
measurements have been run yet. Release CLI/observer v2 copies were captured
from the exact settled artifact JSON paths, byte-compared and made read-only.

The earlier connected 6N observation was truthful but exposed unnecessary work:
every initial decomposition before the last has a provably empty OldOnly
suffix. The original five-atom guarded pair control at N=100 confirmed 27N,
invalidating the previous 9N source estimate. Its positive two-Spool neighbor
charged 41N (39N prefix work plus 2N admitted head emissions), with every
authored proof position checked. The initial incorrect 39N positive expectation
and correction are both retained in logs. No limit was raised.

JoinSnapshot::delta_positions now starts at the last positive atom only when
the delta exactly equals the entire frozen store range. Both indexed binary
and hybrid joins use it; partial deltas retain the complete position range.
It does no index scan, invokes no guard and creates no candidate. The original
pair now charges exactly 9N, the genuine positive neighbor 18N, and the
three-body connected runtime fixture 3N. Independent factor work remains 3N.
The Cartesian limit fixture now charges n+n² rather than 2n+n² because its
impossible first initial decomposition no longer scans n prefixes. Its exact
admission and one-credit-short refusal checks remain intact. A new partial
delta range regression and later-round fixture protect both productive anchors.

The original entail generator regenerated its projections after the narrow
repair. The byte audit still permits only contract hash, actual join credits
and the explicit generator-owned transition comment. All other payload bytes
remain unchanged. Compared with the intermediate v2 capture, the empty-suffix
repair removes 15 D and 1733 OWL-RL credits across the 142 fixtures. Final
source-dependent consumer, wasm, runtime and binary refresh checks are underway.
Actual release scaling/performance remains unrun until root reserves the lane.

The refreshed source-dependent batch now passes: 334 Datalog library tests,
factor allocation, four native portable runtime cases, Datalog all-target
clippy, 59 shapes cases, 14 actual CLI shapes cases, 683 entail library tests
and its 86 consumer integration cases, and entail all-target clippy. The four
affected wasm library crates build successfully, and Node executes all four
exact-selected runtime cases (no ignored or filtered cases). Initial command
target-name mistakes and a missing exact-selection refusal are retained as
failed invocations; corrected settled runs provide the acceptance evidence.
The bench has been rebuilt without running measurements. Final release
artifact capture is underway; timing and actual 100k CLI acceptance are pending.

The settled release build passed in 2m55s. The exact Cargo JSON-selected CLI,
observer, collector and bench binaries are byte-verified read-only v3 copies
under /opt/purrdf-rules-479-3ec2dbba8-qualification. Earlier v1/v2 copies are
preserved. Compiler -Vv, tracked source patch and all four new Rust source byte
receipts accompany these captures. The immutable collector v3 adversarial
self-test passes and the bench lists all nine existing cases. The immutable
current factor allocation test reproduces all exact bytes listed above. The
owned heavy lane is released to root; no timed campaign or benchmark measurement
has run. Final Task5 acceptance awaits root's serialized campaign reservation.
