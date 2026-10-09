# Task 2 implementation and qualification

Parent3d0f398e1. Root owns Git/forge. Changed only CAPI manifest's explicit
test=false host example, examples/native_ci_profile.rs, tests/native_profile.rs,
tests/support/profile.rs and C smoke's narrow PURRDF_PROFILE_CARGO telemetry
override before existing Cargo selection. No shipping/external dependency,
feature, workflow, target gate, main or sibling source changed. Both controller
and actual C smoke share tests/support/phases.rs within the same crate.

The controller runs existing Make test/test-shard/capi-check commands or the
existing downstream preserve-order command. Target selection remains in Makefile
and the current gate. Requests identify source, captured actual Cargo, task-owned
/opt arm, lane, cold/warm and prior warm receipt, jobs1..8, libtest threads,
runner class, and separate dependency/compiler/page-cache declarations.
Controller builds must remain outside measured arm target/build directories.
Cold refuses nonempty target OR new-layout build directories; nothing is deleted.
Warm requires prior complete successful unchanged source/tools/configuration,
target inventory, lane, physical arm, concurrency and cache declarations.

Cache strings are declarations, not observed statistics or proof of cold
compiler/page caches. Actual wrapper/configuration/environment are retained
separately. Admitted campaign cache observations remain Task4. Likewise physical
available parallelism is separate from explicit jobs/libtest environment
overrides; local8 is never called hosted equivalence.

Preparation is outside actual command cost. Identity binds Cargo-resolved
build/profile/target configuration, lock/toolchain/config file hashes, actual
Rust/Cargo/CC versions, relevant environment, Git audit state, tracked AND
untracked source bytes and full Cargo metadata target inventory. Only selected
codegen configuration is persisted, not unrelated config/stderr notes. Logical
source/arm roots normalize comparisons while raw commands/evidence remain.
Rustup Cargo proxy invocation paths retain the required executable basename.

The controller executable copied as cargo in private PATH delegates exact
arguments to actual captured Cargo, adding only supported timings/JSON/noncolored
telemetry flags for build/test. Existing flags/harness arguments are preserved;
metadata/header/lint/version behavior is unchanged. The C override admits its
actual nested Cargo build through the same launcher. Cargo always runs and is
freshness authority; receipts, artifact existence and fresh=true never skip it.

Children retain monotonic durations, arguments, outputs/statuses, typed compiler
artifact metadata and actual timing HTML. Rust preparation/codegen/link is
combined; no pure link cost is inferred by subtraction. Root found an initial
warm attribution flaw: the collector copied all old timing files. The corrected
launcher snapshots hashes before EACH child, then copies only new/changed files
named in that child's own Timing report saved to output under unique names.
Outer tests cannot double-attribute nested C Cargo timing reports. Old files are
retained untouched. The real file fixture tests unchanged cold HTML, changed
latest HTML, new warm HTML and per-invocation filtering.

Typed comparison policy admits unchanged source, exact debug nested dev→test
replacement, or the complete monolithic versus six-shard partition. It rejects
missing/failed/cancelled phases, malformed/missing artifact inventories,
lost/changed retained bytes, unequal source/tools/config/flags/targets/features,
concurrency/cache/runner definitions or lane inventory. Counterfactual source
must contain only the literal one-branch replacement, with no other changed
file. Partition requires monolithic+dedicated C+downstream against all six
shards+dedicated C+downstream and matching actual combined compiler target/features
coverage and per-runner inputs. C-bearing lanes require all16 C phases successful.
Output labels aggregate commands; hosted six-runner critical path is explicitly
NOT MEASURED. Invalid comparison writes no success output; sum overflow fails.

## Final checks and logs

All Cargo builds/checks/tests used CARGO_BUILD_JOBS=8 and -j8.

- cargo test -p purrdf-capi --test native_profile --locked -j8: PASS13 tests
  (nine controller plus four shared phases). tasks/T2-native-profile-tests.log.
- PURRDF_C_PHASE_RECEIPT=<Stage>/tasks/T2-c-smoke-receipt.json cargo test -p
  purrdf-capi --test native_profile --test c_smoke --locked -j8: C smoke PASS5,
  controller PASS12 before the final timing fixture. C receipt16 successful
  phases. Subsequent edits touched controller only, not C smoke/shared helper.
- cargo clippy -p purrdf-capi --all-targets --locked -j8 -- -D warnings:
  final PASS; tasks/T2-clippy.log.
- Existing python3 scripts/check-test-shards.py --self-test and live gate:
  PASS, six shards cover42 members. New integration target is covered and
  controller test=false accepted. No Python tooling was added.
- cargo build -p purrdf-capi --example native_ci_profile --profile test --locked
  -j8 --message-format=json-render-diagnostics: PASS. Actual selected executable
  and O3/assertions/overflow/test=false in tasks/T2-controller-build.jsonl;
  stderr tasks/T2-controller-build.log.
- Actual controller copied as /opt/purrdf-native-profile-seam.r8vbqBOh/cargo,
  with PURRDF_PROFILE_REAL_CARGO=/home/paudley/stage/root/bin/cargo and
  PURRDF_PROFILE_TELEMETRY naming that directory: --version PASS.
  tasks/T2-executable-shim.log.
- Same executable with explicit CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR
  naming existing qualification caches runs the unchanged example build above:
  PASS. Its supported delegated flags, actual fresh=true artifacts and unique
  hash-bound timing HTML are in
  /opt/purrdf-native-profile-seam.r8vbqBOh/cargo-1325516-1791430064361880162.json.
  tasks/T2-executable-telemetry.jsonl and .log contain forwarded actual output.
  This is a cached build SEAM CHECK, not an admitted performance comparison.
- Executable run nonexistent request: expected exit1 refusal;
  tasks/T2-executable-refusal.log. Initial shim build without explicit target
  admission also refused; final seam explicitly names target AND build paths.
- cargo fmt --all --check and git diff --check: PASS; tasks/T2-fmt.log.

Initial compilation found local Result alias/JSON macro collision, renamed
IoResult. Clippy found two redundant clones and an empty assertion; all fixed
without lint exceptions. Final checks were rerun after corrections.

No controlled cold/warm comparison, source/config invalidation campaign, hosted
campaign, measured improvement, full make check/wasm, PR, commit, push or merge
is claimed. Task2 source is ready for independent review. Tasks3–5 retain all
acceptance; instrumentation alone does not close this delivery.
