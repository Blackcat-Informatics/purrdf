#!/usr/bin/env bash
set -eu
# Preparation only: root admission is required before executing this driver.
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
preparation="$PWD/.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-runtime-refusal-qualification-driver"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1
# Exclusive parent: never adopt existing output or targets from another attempt.
logs=$(mktemp -d /opt/purrdf-308-runtime-refusal.XXXXXXXX)
export CARGO_TARGET_DIR="$logs/controller-target"
export CARGO_BUILD_BUILD_DIR="$logs/controller-build"
mirror="$preparation/actual-$(basename "$logs")"
mkdir "$mirror"
printf '%s\n' "$logs" > "$mirror/owned-parent.txt"
run() {
  local label=$1
  shift
  printf '%q ' "$@" >> "$logs/commands.txt"
  printf '\n' >> "$logs/commands.txt"
  set +e
  "$@" > "$logs/$label.log" 2>&1
  local result=$?
  set -e
  printf '%s\t%s\n' "$label" "$result" >> "$logs/terminals.txt"
  printf '%s exit %s\n' "$label" "$result"
  cp "$logs/$label.log" "$logs/commands.txt" "$logs/terminals.txt" "$mirror/"
  if [ "$result" -ne 0 ]; then exit "$result"; fi
}
cat /proc/self/cgroup > "$logs/cgroup.txt"
cgroup_path=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cgroup_path/memory.max" "/sys/fs/cgroup$cgroup_path/memory.swap.max" > "$logs/resource-limits.txt"
# This script must be launched under the admitted64GiB/no-swap user scope.
[ "$(cat "/sys/fs/cgroup$cgroup_path/memory.max")" = 68719476736 ]
[ "$(cat "/sys/fs/cgroup$cgroup_path/memory.swap.max")" = 0 ]
printf 'RUSTC_WRAPPER=%s\nRUSTC_WORKSPACE_WRAPPER=%s\nTARGET=%s\nBUILD=%s\n' "${RUSTC_WRAPPER-}" "${RUSTC_WORKSPACE_WRAPPER-}" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" > "$logs/environment.txt"
cargo -Vv > "$logs/compiler.txt"
rustc -vV >> "$logs/compiler.txt"
df -h "$logs" > "$logs/disk.txt"
# Existing configuration-absence fixtures require a real isolated Cargo ancestor.
for ancestor in /opt/purrdf-native-profile-tests /opt /; do
  [ ! -e "$ancestor/.cargo/config" ] && [ ! -e "$ancestor/.cargo/config.toml" ]
done
[ -d /opt/purrdf-native-profile-tests ] && [ ! -L /opt/purrdf-native-profile-tests ]
b3sum crates/rdf-capi/tests/support/hosted.rs crates/rdf-capi/tests/support/profile.rs crates/rdf-capi/tests/support/phases.rs crates/rdf-capi/examples/native_ci_profile.rs .github/workflows/ci.yaml CONTRIBUTING.md Cargo.toml Cargo.lock /home/paudley/.cargo/config.toml > "$logs/source-blake3.txt"
git rev-parse HEAD > "$logs/source-ref.txt"
git diff --binary > "$logs/unstaged-source.patch"
git diff --cached --binary > "$logs/staged-source.patch"
run list cargo test --locked --jobs 8 -p purrdf-capi --test native_profile -- --list
run tests cargo test --locked --jobs 8 -p purrdf-capi --test native_profile
run clippy cargo clippy --locked --jobs 8 -p purrdf-capi --all-targets -- -D warnings
run controller-build cargo build --locked --jobs 8 -p purrdf-capi --example native_ci_profile --profile test --message-format=json-render-diagnostics
# Standard Cargo JSON artifact selection; Rust production collectors own protocol validation.
run controller-selection jq -Rn -e --arg manifest "$PWD/crates/rdf-capi/Cargo.toml" '
 [inputs | select(startswith("{")) | fromjson |
  select(.reason == "compiler-artifact" and .target.name == "native_ci_profile" and .target.kind == ["example"] and .executable != null)] |
 if length == 1 and .[0].manifest_path == $manifest and
    .[0].profile.opt_level == "3" and .[0].profile.debug_assertions == true and
    .[0].profile.overflow_checks == true and .[0].profile.test == false
 then .[0] else error("unique owning test-profile controller artifact required") end
' "$logs/controller-build.log"
cp "$logs/controller-selection.log" "$logs/controller-artifact.json"
controller_source=$(jq -er .executable "$logs/controller-artifact.json")
[ -f "$controller_source" ] && [ ! -L "$controller_source" ]
case "$(realpath "$controller_source")" in
  "$logs/controller-target/"*) ;;
  *) printf '%s\n' 'controller outside private owned target' >&2; exit 1 ;;
esac
run freeze-controller cp -p "$controller_source" "$logs/native_ci_profile.frozen"
b3sum "$logs/native_ci_profile.frozen" > "$logs/frozen-controller-blake3.txt"
run frozen-controller-readback b3sum --check "$logs/frozen-controller-blake3.txt"
run census cargo run --quiet --locked --jobs 8 -p helper-census -- --check
run helper-self-test cargo run --quiet --locked --jobs 8 -p helper-census -- --self-test
run glossary cargo run --quiet --locked --jobs 8 -p helper-census -- --glossary-gate
run shared-helper-self-test python3 scripts/check-shared-helpers.py --self-test
run shared-helper-gate python3 scripts/check-shared-helpers.py
run actionlint actionlint
run shard-self-test python3 scripts/check-test-shards.py --self-test
run shard-inventory python3 scripts/check-test-shards.py
run profile-self-test python3 scripts/check-build-profiles.py --self-test
run profile-gate python3 scripts/check-build-profiles.py
run toolchain python3 scripts/check-toolchain-pin.py
run parity-self-test python3 scripts/check-gate-parity.py --self-test
run parity python3 scripts/check-gate-parity.py
run fmt cargo fmt --all -- --check
run whitespace git diff --check
run staged-whitespace git diff --cached --check
run ratchet cargo run --quiet --locked --jobs 8 -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
run generated bash scripts/check-generated.sh
run source-readback b3sum --check "$logs/source-blake3.txt"
run controller-readback b3sum --check "$logs/frozen-controller-blake3.txt"
mkdir "$logs/integration-2"
jq -n --arg root "$PWD" --arg directory "$logs/integration-2" '
 {root:$root,directory:$directory,cargo:"/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo",
  lane:"integration-2",warmth:"cold",jobs:8,test_threads:8,
  runner_class:"local owning production collector seam; no hosted or performance acceptance",
  dependency_cache:"existing registry/download cache unchanged; no flush",
  compiler_cache:"inherited compiler configuration captured, no override or flush",
  page_cache:"uncontrolled local OS cache; cold means empty owned Cargo target AND build"}
' > "$logs/request-integration-2.json"
jq -n --arg receipt "$logs/integration-2/integration-2-cold-receipt.json" --arg output "$logs/integration-2-self-validation.json" '
 {before:[$receipt],after:[$receipt],change:"none",output:$output}
' > "$logs/integration-2-self-policy.json"
run integration-2 "$logs/native_ci_profile.frozen" run "$logs/request-integration-2.json"
run production-self-validation "$logs/native_ci_profile.frozen" compare "$logs/integration-2-self-policy.json"
# Data inventory only. The Rust run/compare and original Rust Make test own acceptance.
jq '{request:.context.request,phases:[.phases[]|{name,success,error,exit:.command.exit_code}],children:.context.cargo.receipts}' \
 "$logs/integration-2/integration-2-cold-receipt.json" > "$logs/production-parent-summary.json"
while IFS= read -r child; do
 jq -c 'select(.context.original_argv[0] == "run") |
 {argv:.context.original_argv,effective_jobs:.context.effective_cargo_jobs,
  exit:.phases[0].command.exit_code,stdout:.phases[0].command.stdout,stderr:.phases[0].command.stderr,
  executable:.context.run_executable,capi_artifacts:.context.capi_artifacts}' "$child"
done < <(jq -r '.context.cargo.receipts[].path' "$logs/integration-2/integration-2-cold-receipt.json") \
 > "$logs/actual-run-child-inventory.jsonl"
run final-source-readback b3sum --check "$logs/source-blake3.txt"
run final-controller-readback b3sum --check "$logs/frozen-controller-blake3.txt"
cp "$logs"/*.txt "$logs"/*.json "$logs"/*.jsonl "$logs"/*.log "$mirror/"
