# Configuration correction focused qualification

Status: affected local source qualification PASS; local build lane FREE.
Native test session53992 actually exited0; final sequential qualification
session2931 actually exited0. No owned command remains. Normal hooks, commit/push,
independent review, fresh complete hosted attempt and measured acceptance remain
root-owned and are not claimed by this qualification.

Candidate: HEAD6641041e457cbfa105432cc62fa676d5275d7a5f plus the three-path
support/profile.rs, support/phases.rs and support/hosted.rs delta captured in
T4-config-logs/source-delta.patch. Applicable AGENTS/root .baseline/.goals,
T4-config-correction.md and current validation were read. No source, Git/index,
forge, dispatch, cache or service mutation was performed by this worker.

## Route and actual commands

Same active raw SDK first PATH entry:
/home/paudley/stage/packages/rustup/active-toolchain/bin. Every Cargo compilation
used --locked --jobs8 and CARGO_BUILD_JOBS=8. Existing registered Cargo-created
target /opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90 and
build /opt/.cargo/slots/95be0b9034a0e1f3/0/build were reused unchanged through
CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR. No private-artifact admission override,
synthetic tag, HOME override, compiler change or full/campaign execution occurred.
Actual compiler/Cargo versions: rustc1.100.0-nightly
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM23.1.1;
Cargo1.100.0-nightly7941be6fb. Exact outputs and route are retained in logs.

All current acceptance commands below actually exited0; logs are under
tasks/T4-config-logs:

* tests.log: cargo test --locked --jobs 8 -p purrdf-capi --test native_profile;
 25passed,0failed/ignored/filtered. Includes all three new configuration controls
 and existing real restoration/identity/shim/failure/comparison/workflow tests.
* clippy.log: cargo clippy --locked --jobs 8 -p purrdf-capi --all-targets --
 -D warnings; warning-free.
* controller-build.jsonl and controller-build.log: cargo build --locked --jobs 8
 -p purrdf-capi --example native_ci_profile --profile test
 --message-format=json-render-diagnostics. Actual JSON uniquely selects this
 package/example's executable, opt_level3/debug_assertionstrue/target.testfalse.
 Selected executable exists and is executable; controller-artifact.json retains
 full identity. SHA2564a73f072ccf98fe037f058a16229270db57c385343052afc74322a16df90f01a
 binds /opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90/debug/examples/native_ci_profile.
* helper-census-corrected.log: cargo run --quiet --locked --jobs 8 -p helper-census
 -- --check; no unsanctioned isomorphic group or forbidden outside-home match.
* shared-helper-self-test.log and shared-helper-gate.log: python3
 scripts/check-shared-helpers.py --self-test and without arguments. Actual live
 census81enforcedjobs/23reasonedvariants/91distinctrows/1878files passes, including
 the shared invalid home. No cross-crate path include or open copy remains.
* actionlint.log: actionlint; all actual workflow files pass.
* shard-self-test.log and shard-inventory.log: python3 scripts/check-test-shards.py
 --self-test and without arguments; six feature-unified shards cover42members.
* profile-self-test.log and profile-gate.log: python3 scripts/check-build-profiles.py
 --self-test and without arguments;1004units across2gateinvocations build O3 with
 debug assertions/overflow checks on across42workspace members.
* toolchain.log: python3 scripts/check-toolchain-pin.py.
* parity-self-test.log and parity.log: python3 scripts/check-gate-parity.py
 --self-test and without arguments; all71local/hosted gates match.
* fmt.log: cargo fmt --all -- --check; whitespace.log: git diff --check.

## Meaningful configuration controls

The actual native_profile harness executed successful empty-config SDK Cargo
query, configured build/profile/target tables with a synthetic unrelated token,
and malformed owned TOML refusal. SDK Cargo is resolved via actual rustc sysroot;
the configuration-only children env_clear and use testkit-owned cwd/CARGO_HOME,
never an overridden HOME. No compile occurs in those child queries. The real
fixture verifies absent/present state, retained build value, exclusion of the
private marker and actionable failure without marker leakage. Projection fixtures
also refuse malformed/non-object/duplicate/wrong-table-shape/invalid-UTF8 data
and prove a changed configuration state refuses comparison. Unknown failures
remain hard errors; no failed query is turned into empty configuration.

Initial live census command omitted its mandatory mode and exited2 with usage,
not a source defect (helper-census.log and terminals.tsv retained). Corrected
production --check invocation actually passed, followed by the full existing
shared-helper wrapper self-test/live gate. No source repair or fallback was used.

## Source binding and limits

source-receipt.sha256 binds15source/caller/config/workflow inputs; final
source-readback.log verifies all15match with actualexit0. Exact HEAD and patch
are retained. controller-readback.log verifies the actual rebuilt executable
unchanged with actualexit0. Cargo JSON selection is readback of actual compiler
messages, not a guessed target path or historical artifact receipt.

The completed hosted37738739777attempt remains FAILURE: all12arms failed before
native measurement; comparison correctly refused incomplete phases. This local
source qualification does not recover missing historical stderr, assert hosted
Cargo1.101 behaves identically to local1.100, establish matched native timings,
or turn failed hosted evidence into PASS. Root must review/commit this qualified
correction and run the accepted fresh complete matched campaign. No new full
gate, measurement campaign or hosted dispatch has been performed here.
