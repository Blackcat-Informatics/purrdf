# Task 3 hosted capture implementation

Parent eb875cb8c. Root owns Git, forge, dispatch and the serialized performance
campaign. Source is settled and focused verification passed in the admitted lane.
Task 3 is ready for independent review; no task commit or whole-issue completion
is claimed.

The existing workflow adds one default-false native_profile dispatch input and
three optional jobs. Every original required job, selection, strict profile,
lint/hygiene gate and cache remains unchanged. Compiler admission retains actual
floating-nightly rustc and Cargo identities; each experiment runner refuses a
different observed identity. There is no global dated toolchain pin.

Twelve cases cover current-source monolithic/C/downstream BEFORE, all six native
shards/C/downstream AFTER, and the additional nested-dev C BEFORE. AFTER C is
reused for the nested-profile comparison. Each case executes actual cold then
unchanged warm through the shared controller. Only the narrow nested-dev case
copies source and changes the exact debug test-to-dev branch; its binary Git diff
is retained, and existing comparison admission refuses every other source delta.
The new hosted-run and hosted-compare operations generate requests and policies;
actual target selection and C checks remain in their established homes.

Controller builds and cargo-c installation remain outside measured target AND
build directories. Unique run/attempt /opt arms refuse RAM-backed filesystems,
never clean shared caches, and receive a 360-minute baseline bound. Cancelled or
missing arms cannot qualify. Disk/setup start/completion observations separate
prerequisite and controller work from actual measured commands. No dependency or
compiler-artifact cache action runs in the experiment. Registry/download and OS
page-cache state are distinct; cold means empty Cargo build artifacts.

Matching binds observed image/OS/architecture, stable actual CPU model, flags,
topology/count and physical memory, plus available parallelism. Raw CPU and
before/after memory/cache observations are retained. Actual Cargo/libtest limits
are min(observed available parallelism,8). Existing identity captures resolved
configuration, actual compiler/CC, wrappers, codegen environment, source/lock and
target inventory. Observed Python/Node/Binaryen/cargo-c identities also enter
runner matching, so supplied runner labels alone do not admit heterogeneous
hardware or tools. Hardware/compiler/configuration differences hard-refuse.

Prerequisite audit inspected Make test/test-shard/capi-check and real native
callers. Python 3 stdlib drives gzip/zlib and pinned XML/SPARQL inventory scripts;
Node drives ECMAScript oracles; Binaryen validates native wasm-link tests; pinned
cargo-c generates the actual C header. Optional arms install the same Node24,
pinned Binaryen and cargo-c 0.10.23, require/record Python 3 and CC, and leave every
actual prerequisite check in place. The installed Python-extension case is
explicitly ignored by the ordinary native harness; no binding gate is fabricated.
The downstream preserve-order consumer has no additional executable prerequisite.
No wasm compilation target is needed for these native selections. XML corpus
presence is captured separately; its existing verified acquisition still runs.

Root found cargo capi --version exits0 while printing generic help. The settled
implementation invokes the actual PATH-selected cargo-capi --version, retains its
executable content identity, and requires one cargo-c 0.10.23+... version line.
Generic help, a wrong release, prefix lookalikes and missing/extra version text
refuse. Actual Python/Node/Binaryen executable identities are also retained;
version observations enter matching. Zero child exit status alone is not useful
version evidence.

Receipts, Cargo timing HTML, source/configuration/tools/concurrency/cache/setup,
counterfactual diff and controller error output upload even on failure. Download
keeps artifacts separate, then restores each case at its original absolute /opt
path before shared hash-bound readback. Build caches and copied launcher bytes are
not uploaded. Path preservation follows the primary
[upload-artifact documentation](https://github.com/actions/upload-artifact/blob/main/README.md)
and [download-artifact documentation](https://github.com/actions/download-artifact).

Actual command wall start/finish timestamps are separate from monotonic costs.
Validated comparisons retain per-lane intervals and first-start/last-finish
windows, explicitly including scheduling and uncertified cross-runner clock
synchronization. They do not infer six-runner speedup from those windows.
Aggregate Rust cost remains combined preparation/codegen/link/test, never a
subtraction-derived pure link cost.

The native YAML reader verifies the real optional workflow's exact complete
matrix, default-false input, required job presence and failure uploads. Meaningful
controls remove coverage/uploads, enable default dispatch, exercise exact
counterfactual refusal, distinguish stable hardware changes from variable
frequency/cache observations, and refuse missing/reversed command clocks.

Checks actually run:

- actionlint .github/workflows/ci.yaml PASS; tasks/T3-actionlint.log.
- Existing shard/profile self-tests PASS. Live shard inventory PASS, six
  feature-unified shards cover42members; tasks/T3-shard-inventory.log.
- Existing live profile gate PASS1004 units across2 invocations,42members,
  O3/assertions/overflow on (metadata/unit graph, no compilation).
- Existing toolchain-pin PASS and gate parity PASS71 unchanged hygiene gates.
- CARGO_BUILD_JOBS=8 cargo test -p purrdf-capi --test native_profile --locked
  --jobs 8 PASS 18 tests, including all earlier controller/shared-phase controls
  plus five hosted controls; tasks/T3-native-profile-tests.log.
- CARGO_BUILD_JOBS=8 cargo clippy -p purrdf-capi --all-targets --locked --jobs 8
  -- -D warnings PASS; tasks/T3-clippy.log.
- CARGO_BUILD_JOBS=8 cargo build -p purrdf-capi --example native_ci_profile
  --profile test --locked --jobs 8 --message-format=json-render-diagnostics PASS;
  tasks/T3-controller-build.jsonl and .log. Selected actual executable remains
  O3/assertions/overflow on,test=false. Build uses the existing disk cache,
  not a measured cold arm.
- Actual selected executable hosted-run after-unknown: expected exit1 refusal,
  tasks/T3-hosted-operator-refusal.log. This proves actual new operator dispatch
  and hard refusal without starting any campaign or source copy.
- Changed-surface rustfmt --check and git diff --check PASS;
  tasks/T3-whitespace.log.

Initial compilation exposed a redundant generic Object::insert .into inference
ambiguity; removed it and reran focused verification successfully. No lint allow,
gate bypass, fallback or dependency was introduced. The C smoke/shared phase
implementation is unchanged; earlier actual16-phase C evidence remains applicable
to that surface. Task4 still executes actual C in all qualifying cold/warm arms.

No campaign, measured improvement, invalidation witness, hosted dispatch, full
qualification, PR, commit, push or merge is claimed. Tasks4–5 retain their full
acceptance; telemetry does not close this delivery.
