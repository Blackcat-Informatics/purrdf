# Task 4: shared round admission

Parent source: 3912955a5. Initial focused checks passed, but independent review
found one required negative-row expansion fix. The correction and actual
allocation-traffic regression are implemented and their focused checks pass.
Independent re-review PASS. Normal hooks/commit/push and publication remain next
steps. No commits, forge writes or broad/performance campaigns were performed by
this implementer. Initial source scope is `seminaive.rs` and its private
`factors.rs`; the required review correction also adds the allocation integration
test and the dev-only manifest/resolved-lock edge described below.

## Production credit ownership

Every frozen round now owns one shared atomic credit pool. Rule-local governors
retain their own counters but borrow this single pool, rather than receiving a
fresh copy of the remaining budget. Guard-free parallel tasks reduce private
buffers in authored order. Guarded rounds remain serial on the caller thread.

Successful credits and a single refused reservation are represented separately.
This preserves the final successful credit when the remaining ceiling is
u64::MAX. The refusal latch flows through RoundBuffer into FixpointState and is
checked before commit even when the public count saturates at u64::MAX. Ordinary
refusals report limit+1 exactly. The shared host capacity renderer explicitly
describes full-width saturation and says this limit cannot be raised; it does not
offer equal observed/permitted integers as evidence of a strict numerical excess.

Indexed positive joins reserve before cloning a binding/source frame. Cyclic
joins reserve before owned source capture and output cloning. The empty-body
identity is admitted before callbacks or head construction. Independent-factor
products reserve before winner-vector/frame/source expansion; Full matching and
borrowed Old/New/Full classification remain distinct, with no duplicate charge
for classification or discarded index/delta rows.

The final positive/guard/identity row covers its first ordinary head. Each further
conjunctive head reserves another credit before owned candidate/proof expansion.
Thus an arbitrary head list cannot multiply unmetered candidates, while the
single-head connected CLI charge law is preserved. Factorized heads retain their
existing per-projected-product admission. The issue pair fixture's FILTER rejects
its equal-spool rows without adding returned guard rows; its analytically derived
9N positive-emission count was the Task4 source-level estimate, not runtime
evidence. Task5 diagnosis subsequently identified omitted unsuccessful variants:
prefix scans can charge before reaching an empty Old-only suffix. The9N budget
feasibility claim is withdrawn pending the actual original-pair regression and
required optimization/qualification; it is not the actual100k CLI campaign.

Single negative probes meter their first probe and every additional broad
partition. Negative conjunctions admit their local frame, each partition, each
traversed row, and reached callbacks. They iterate borrowed partitions instead
of constructing an eager partition Vec. Global factor preconditions and
factor-owned negatives use the same governed production routines.

Callback answer shape validation now admits each reached returned row before
traversing/validating it. Only a fully admitted, valid answer proceeds to owned
binding/computed-surface expansion. On exhaustion the latch suppresses later
callbacks and heads and forces typed total refusal at the round boundary; no
empty internal result is returned as a successful incomplete model. Authored
body guard stages, multiplicity, fresh state and operational-error ordering are
preserved. Reached malformed rows remain hard failures.

The callback's own eager Vec/string allocation and SHACL Producer's eager triples
remain outside consumer admission. This work does not establish full physical
memory budgeting or streaming callback interfaces and does not close 364. The
approved portfolio owns those additional interfaces. Existing join/fact/arena
defaults, canonical authored hashes, planner/calculus v2, runtime dependencies and Cargo
features remain unchanged.

## Verification

Five new production-entry/boundary tests cover:

- 32 rules with asymmetric 1..32 cardinalities through actual public evaluation at
  workers 1/4/32: identical facts, proofs and successful 528-credit reports; refusal
  at 0, 17 and 527 reports exactly ceiling+1 and only seeded facts.
- Full-width private pool seeded near u64::MAX: final credit admitted, next refused
  once, saturated report truthful. A real two-row production round near the
  accumulated limit refuses before absorption can commit a fact or derivation.
- Public scheduled 128-conjunct head: exact 128 credits commit 129 total facts;
  ceilings 0, 1 and 127 refuse with unchanged seeded count.
- Single and conjunctive negative probes over 1001 partitions with empty addressed
  indices: exact 1002/1003 credits succeed;0, 17 and one-below refuse without a head.
- 10000-row body and negative callbacks, zero allowance preventing the initial
  callback, exact/one-below body output boundaries, bounded negative traversal
  and reached rejecting callbacks. A 100-row negative traversal succeeds at
 203 credits and refuses at 202, with no partially committed closure.

The previous ground-negative precondition fixture now expects its three actual
group/partition/row admissions, rather than treating that work as free. It still
blocks before evaluating either 1000-row positive factor.

Settled commands (nested and outer Cargo both capped at 8):

- `CARGO_BUILD_JOBS=8 cargo test -p purrdf-datalog --lib --locked --jobs 8`:
  PASS 333 tests, 0 failures. Final log `/opt/purrdf-t4-final-tests.log`.
- `CARGO_BUILD_JOBS=8 cargo test -p purrdf-datalog --doc --locked --jobs 8`:
  PASS 1 doc test. Log `/opt/purrdf-t4-doc.log`.
- `CARGO_BUILD_JOBS=8 cargo clippy -p purrdf-datalog --all-targets --locked --jobs 8 -- -D warnings`:
  PASS warning-free. Final log `/opt/purrdf-t4-final-clippy.log`.
- `CARGO_BUILD_JOBS=8 cargo test -p purrdf-shapes --test srl_language --test rules_engine --test rule_capacity_limits --locked --jobs 8`:
  PASS 23 SRL + 26 rules + 6 capacity tests. Log `/opt/purrdf-t4-shapes.log`. This preceded
  only the final wording/test addition for the full-width diagnostic.
- `cargo fmt --all --check` and `git diff --check`: PASS.

Development compilation failures (changed private call-site types, the floating
nightly atomic API rename, unnecessary qualifications) and two new-test clippy
findings were repaired. Initial ground-negative count assertions were corrected
against the actual governed computation. A combined `cargo test --lib --doc`
invocation was rejected before running tests; the two valid commands above ran
separately. None of these unsuccessful runs is counted as a pass.

`rustc --print cfg --target wasm32-unknown-unknown` confirms this toolchain exposes
64-bit atomic primitives, but it is not a wasm build/runtime check. No full
makecheck, whole-workspace gate, wasm build, CLI 100k run, allocation benchmark or
hosted CI was run. The one Arc pool allocation per round and contention cost need
the already assigned Task 5 measurement/consumer qualification.

## Required review correction

The reviewer identified that the outer negative guard could continue cloning its
remaining computed local rows after a nested guard latched exhaustion. Callbacks
were already suppressed, so callback-count assertions would miss these owned
expansions. The negative guard now checks the latch before every returned row's
local assignment and returns immediately on refusal.

`tests/negative_guard_admission.rs` exercises the actual public evaluator under
the workspace's shared CountingAllocator/CurrentThreadWindow. All 128 caller
rows, each with a 64 KiB surface, are built before measurement and moved out of a
Mutex without cloning. A nested refusal should clone one reached local, not all
128; requested allocator traffic must lie between one and four surfaces. The
same test checks zero, exact and one-below credit boundaries and unchanged seeded
counts on total refusal. It adds only the existing first-party alloc-probe as a
dev-dependency, with the corresponding resolved Cargo.lock edge; no shipping
dependency or alternate allocator is introduced.

Correction validation and independent re-review PASS (T4-review.md). Root preserved
the eight cited check/control logs under tasks/T4-logs/ as durable Stage evidence.

- `CARGO_BUILD_JOBS=8 cargo test -p purrdf-datalog --test negative_guard_admission --lib --locked --jobs 8`:
  PASS 333 library tests plus the allocation integration regression. Log
  `/opt/purrdf-t4-correction-tests.log`.
- `CARGO_BUILD_JOBS=8 cargo test -p purrdf-datalog --doc --locked --jobs 8`:
  PASS 1 doc test. Log `/opt/purrdf-t4-correction-doc.log`.
- `CARGO_BUILD_JOBS=8 cargo clippy -p purrdf-datalog --all-targets --locked --jobs 8 -- -D warnings`:
  PASS warning-free, including the new integration target. Log
  `/opt/purrdf-t4-correction-clippy.log`.
- `cargo fmt --all --check` and `git diff --check`: PASS.

A narrow pre-fix control temporarily removed only the new negative-row spent
check. The allocation test failed with 8,393,905 requested bytes, despite callbacks
already being suppressed. After restoring the check, the same fixture passed
its measured range of at least 65,536 and fewer than 262,144 requested bytes. The
control log is `/opt/purrdf-t4-correction-pre-fix-control.log`; that expected
negative control is not counted as a successful gate. The correction was restored
before all settled commands above. This is allocation traffic, not RSS or a
timing/performance campaign. No further source experiments remain.
