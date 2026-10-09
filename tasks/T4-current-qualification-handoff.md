# Current composition: executable focused qualification handoff

Status: first focused phase actually PASS; see T4-current-focused-qualification.md.
The source-based production seam instructions below remain PREPARED/UNRUN; root
must explicitly admit those before controller children or an owned source snapshot.
The settled tree includes the root-authorized one-token clippy correction. Original
preparation did not alter tracked source/index/refs or start a workload.

Applicable repository AGENTS instructions and root `.baseline` / `.goals` read;
these two law files are present in the root checkout, not this worktree.
Current candidate is HEAD `221b1ace8bd30e010dba6b9d732cfc9ff576a0ed` plus the
resolved main538 staged tree `ff72f20806a30bb7d90ea5886713a9994e7100eb`.
Use `T4-main538-resolution-review.md` and `T4-telemetry-implementation.md` for
exact source/patch identity. The validation index's opening historical config
failure description is superseded by its later actual config and second hosted
attempt entries; do not interpret that opening as the current correction state.

## Admitted environment and focused commands

Run sequentially from the assigned worktree. Retain separate stdout/stderr and
each actual terminal exit in `tasks/T4-current-logs/`; do not lose the native exit
through an unchecked `tee`. Read back the source/index identity before and after.
The existing target/build roots were observed present; Cargo owns their existing
containment. Do not create tags, force artifact admission or alter shared config.

```sh
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8
export CARGO_TARGET_DIR=/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90
export CARGO_BUILD_BUILD_DIR=/opt/.cargo/slots/95be0b9034a0e1f3/0/build
rustc -vV
cargo -Vv
cargo-capi --version
node --version
wasm-opt --version
cc --version
cargo test --locked --jobs 8 -p purrdf-capi --test native_profile -- --list
cargo test --locked --jobs 8 -p purrdf-capi --test native_profile
cargo clippy --locked --jobs 8 -p purrdf-capi --all-targets -- -D warnings
cargo build --locked --jobs 8 -p purrdf-capi --example native_ci_profile --profile test --message-format=json-render-diagnostics
cargo run --quiet --locked --jobs 8 -p helper-census -- --check
python3 scripts/check-shared-helpers.py --self-test
python3 scripts/check-shared-helpers.py
actionlint
python3 scripts/check-test-shards.py --self-test
python3 scripts/check-test-shards.py
python3 scripts/check-build-profiles.py --self-test
python3 scripts/check-build-profiles.py
python3 scripts/check-toolchain-pin.py
python3 scripts/check-gate-parity.py --self-test
python3 scripts/check-gate-parity.py
cargo fmt --all -- --check
git diff --cached --check -- crates/rdf-capi/Cargo.toml crates/rdf-capi/tests/c_smoke.rs crates/rdf-capi/tests/support/phases.rs crates/rdf-capi/tests/support/profile.rs Makefile .github/workflows/ci.yaml
```

SDK Cargo is selected deliberately so supported private output contracts are
honored. Compiler/config versions must be captured anew, not copied from the
previous qualification. The pinned cargo-c generator is 0.10.23; generic help
is not its version identity. Node and Binaryen prerequisites are checked by the
actual owning Make routes. Any missing prerequisite is a hard blocker; do not
silently skip or install/build while another lane owns admission.

Source discovery predicts 28 tests in native_profile (old25 plus three telemetry
fixtures); use actual `--list` and terminal counts. The new fixtures are
`phases::tests::mixed_harness_frames_preserve_strict_artifact_selection`,
`profile::tests::mixed_children_bind_libraries_before_cleanup_and_match_actual_c_smoke`,
and `profile::tests::missing_malformed_ambiguous_and_tampered_child_bindings_refuse`.
They execute real filesystem cleanup/bindings and malformed/ambiguous/missing/
tampered inputs, not passing canned runtime answers. All28, including actual
empty/configured/malformed SDK queries, comparison invalidations, counterfactual
and restoration controls, should execute. Final counts remain unknown until run.

Select the rebuilt controller ONLY from actual Cargo JSON: exactly one successful
`compiler-artifact` for the metadata-identified purrdf-capi package, example target
`native_ci_profile`, `target.test = false`, non-null executable, O3 and assertions
on. Require its actual file/executable identity and retain the complete selected
frame plus BLAKE3/byte count. A previously guessed debug/examples path is not an
artifact receipt. Use that selected absolute path as `CONTROLLER` below.

## Real production collector seams

The production `run` schema offers whole existing lanes, not arbitrary commands.
The narrow valid selections are `doc` and `capi`. Do not invent a request field,
replace the actual Make commands, or call `hosted-run` with a pretend GitHub SHA.
The doc route executes workspace doctests, workspace example compilation under
test profile, then the THREE real examples graphql_oracle_fixture,
typescript_oracle_fixture and text_relevance. This creates the actual mixed
Cargo/libtest output that failed the previous collector. It is broader than a
single test but does not execute the full workspace suite.

After admission create a NEW owned parent `/opt/purrdf-308-current-telemetry`
(refuse an existing populated parent; never clean it). Proposed distinct arms:
`doc`, `capi-test`, `capi-dev`. Create parent directories only: controller/Cargo
create each arm's target/build directories. The controller must remain outside
all three arm target/build roots. No warm replay is necessary solely for this
focused bug qualification; mandatory matched cold/warm measurement is hosted.

The exact cold doc Request JSON is:

```json
{
  "root": "/home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c",
  "directory": "/opt/purrdf-308-current-telemetry/doc",
  "cargo": "/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo",
  "lane": "doc",
  "warmth": "cold",
  "jobs": 8,
  "test_threads": 8,
  "runner_class": "local current-source seam qualification; not hosted timing evidence",
  "dependency_cache": "existing local registry/download cache reused; no flush",
  "compiler_cache": "inherited compiler wrappers and effective configuration captured; no new override or flush",
  "page_cache": "uncontrolled local OS page cache; cold means empty owned Cargo target and build only"
}
```

Write that request to Stage `tasks/T4-current-logs/request-doc.json`. Optional
`previous` must be absent for cold, not a fictional predecessor. Run:

```sh
"$CONTROLLER" run "$STAGE/tasks/T4-current-logs/request-doc.json"
"$CONTROLLER" run "$STAGE/tasks/T4-current-logs/request-capi-test.json"
"$CONTROLLER" run "$STAGE/tasks/T4-current-logs/request-capi-dev.json"
```

`STAGE` is the absolute existing assigned Stage directory. The second request
has exactly the same fields/declarations, changing lane to `capi` and directory
to `/opt/purrdf-308-current-telemetry/capi-test`. The third changes lane to
`capi`, directory to `/opt/purrdf-308-current-telemetry/capi-dev` and root to the
owned counterfactual snapshot described below. Save all three exact requests.
Run sequentially, never sharing arm targets/build or overwriting receipts.

The capi route is the actual `make capi-check`: generator self-tests, current
header verification using its original temporary `cargo capi build`, and the
actual c_smoke test. Each successful child must record invocation-bound artifact
identity BEFORE header temporary cleanup. Header scratch may legitimately be
gone at outer collection; its captured frame/path/bytes/digest must still verify.
Different parent/header/nested library records must remain separate, not a
flattened ambiguous stream. Actual C-smoke selects and hashes its runtime library
and emits the real16phase receipt. Its compiled smoke.c now exercises XPath
laws/refusals through current header/linkage and current shared regime vectors.

## Exact owned counterfactual, preserving the live worktree

Current source is uncommitted, so cloning/checking out HEAD alone is INVALID.
For capi-dev, after admission root must authorize an owned source snapshot or
perform it: shared local clone at
`/opt/purrdf-308-current-telemetry/nested-dev-source`, detached at current HEAD,
then overlay an archive of the already-created staged tree
`ff72f20806a30bb7d90ea5886713a9994e7100eb`. Remove only the old tracked paths
deleted in that tree (currently `scripts/check-i18n-glossary.py`) from the snapshot.
This preserves a real Git source inventory/context, introduces no fake commit,
new branch or persistent worktree, and never changes the live index/source.
Bind the staged-tree identity before export; if it changed, reassess first.

Use EXACTLY the existing `hosted::counterfactual` computation: require one
occurrence of `let profile = if cfg!(debug_assertions) {\n        "test"` in the
snapshot c_smoke.rs and replace only its `"test"` with `"dev"`. Save the
byte-exact snapshot delta; compare the entire staged source inventory before
and after so the only logical source change is that admitted nested-profile
branch. No production matcher, header, fixture, manifest, configuration or other
file may differ. Do not replace the profile using an environment override.
The existing fixture tests this exact computation, including duplicate/missing
needle refusal. The preparation does not authorize editing the live c_smoke.

Snapshot commands after root admission can use `git clone --shared --no-checkout`
from the assigned worktree, ordinary detached checkout of 221b, and
`git archive ff72f20806a30bb7d90ea5886713a9994e7100eb` piped to tar inside that
owned clone. Check ALL pipeline exits; root/source/index/refs remain unchanged.
No snapshot is created during preparation. Do not invoke the existing hosted
launcher before publication: it checks out GITHUB_SHA and would omit this patch.

## Actual evidence required before admission is called PASS

Retain `<lane>-cold-receipt.json`, complete `<lane>-cold-cargo/` child receipts,
raw mixed stdout, compiler messages and attributable timing HTML. All mandatory
phases must be successful; failed native/collector terminals and uploads must
remain preserved. Doc must show the real three-command production route and
successful collection. Each capi arm must show header capture despite owned
cleanup, exact runtime artifact binding and ALL16C phases successful, including
actual C compile/link/runtime/projection. Require strict selection/refusal rather
than first/latest library or file-existence fallback.

Capture source/compiler/config/target identities and complete Cargo metadata
before accepting receipts. Current inventories should include CLI observer
examples (test=false), helper glossary probe (test=false), Datalog harness-free
rules_runtime/factor_allocation/negative_guard_admission and CAPI xpath_regex.
Shard gates decide actual complete coverage/counts; old1004unit/42member/71gate
numbers are historical, not assumed-current results. Record actual terminals
and counts, final source/patch readbacks and controller artifact readback.

No unresolved production failure may be deferred into another campaign. If
these actual focused seams fail, inspect the retained computation and fix only
owned errors with root coordination, then requalify affected paths. Normal
hooks/commit/push follow qualification and independent adjudication; they are
root-owned and never bypassed.

After publication a fresh complete hosted campaign must execute all TWELVE
cases: before-monolithic, before-capi, before-downstream, after-lib, after-doc,
after-integration-1..4, after-capi, after-downstream, profile-before-capi; every
case must finish its real cold and unchanged warm arm under current-attempt
compiler admission. Final exact comparison must accept all receipts. The
failed37746157677attempt's four successful pairs cannot be recycled as current
candidate timing proof. Source/config invalidation fixtures qualify refusals;
no measured speed claim exists until the new full matched campaign accepts.

Prepared requirements are executable through existing production paths. Missing
now: lane admission, any needed owned snapshot authorization, actual execution
and terminal/artifact evidence. There is no supported smaller arbitrary-command
Request seam; using doc/capi is necessary to exercise original production callers.
