#!/usr/bin/env bash
set -eu
# Preparation only. Root must admit the sole build lane before execution.
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
original=/opt/purrdf-308-runtime-refusal.hyKidvRk
controller="$original/native_ci_profile.frozen"
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1
export CARGO_TARGET_DIR="$original/controller-target"
export CARGO_BUILD_BUILD_DIR="$original/controller-build"
logs=$(mktemp -d "$original/continuation.XXXXXXXX")
mirror="$PWD/.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-runtime-refusal-qualification-driver/actual-$(basename "$logs")"
mkdir "$mirror"
printf '%s\n' "$logs" > "$mirror/owned-parent.txt"
finish() {
  result=$?
  trap - EXIT
  printf '%s\n' "$result" > "$logs/controller.exit"
  find "$logs" -maxdepth 1 -type f -exec cp -p -t "$mirror" {} +
  exit "$result"
}
trap finish EXIT
run() {
  label=$1; shift
  printf '%q ' "$@" >> "$logs/commands.txt"
  printf '\n' >> "$logs/commands.txt"
  set +e
  "$@" > "$logs/$label.log" 2>&1
  result=$?
  set -e
  printf '%s\t%s\n' "$label" "$result" >> "$logs/terminals.txt"
  printf '%s exit %s\n' "$label" "$result"
  cp -p "$logs/$label.log" "$logs/commands.txt" "$logs/terminals.txt" "$mirror/"
  [ "$result" -eq 0 ] || exit "$result"
}
cat /proc/self/cgroup > "$logs/cgroup.txt"
cgroup_path=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cgroup_path/memory.max" "/sys/fs/cgroup$cgroup_path/memory.swap.max" > "$logs/resource-limits.txt"
[ "$(cat "/sys/fs/cgroup$cgroup_path/memory.max")" = 68719476736 ]
[ "$(cat "/sys/fs/cgroup$cgroup_path/memory.swap.max")" = 0 ]
printf 'RUSTC_WRAPPER=%s\nRUSTC_WORKSPACE_WRAPPER=%s\nTARGET=%s\nBUILD=%s\n' "${RUSTC_WRAPPER-}" "${RUSTC_WORKSPACE_WRAPPER-}" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" > "$logs/environment.txt"
cargo -Vv > "$logs/compiler.txt"
rustc -vV >> "$logs/compiler.txt"
run continuation-compiler-readback cmp "$original/compiler.txt" "$logs/compiler.txt"
[ "$(git rev-parse HEAD)" = 40de8b0402a29cff51d3c6dd50d96eb49f7c435f ]
run continuation-source-admission b3sum --check "$original/source-blake3.txt"
run continuation-controller-admission b3sum --check "$original/frozen-controller-blake3.txt"
run original-failed-ratchet-preservation b3sum "$original/ratchet.log" "$original/terminals.txt"
[ ! -e "$original/integration-2" ]
[ ! -e "$original/integration-2-self-validation.json" ]
run continuation-ratchet cargo run --quiet --locked --jobs 8 -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
run continuation-generated bash scripts/check-generated.sh
run continuation-source-readback b3sum --check "$original/source-blake3.txt"
run continuation-controller-readback b3sum --check "$original/frozen-controller-blake3.txt"
mkdir "$original/integration-2"
jq -n --arg root "$PWD" --arg directory "$original/integration-2" '
 {root:$root,directory:$directory,cargo:"/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo",
  lane:"integration-2",warmth:"cold",jobs:8,test_threads:8,
  runner_class:"local owning production collector seam; no hosted or performance acceptance",
  dependency_cache:"existing registry/download cache unchanged; no flush",
  compiler_cache:"inherited compiler configuration captured, no override or flush",
  page_cache:"uncontrolled local OS cache; cold means empty owned Cargo target AND build"}
' > "$logs/request-integration-2.json"
jq -n --arg receipt "$original/integration-2/integration-2-cold-receipt.json" --arg output "$logs/integration-2-self-validation.json" '
 {before:[$receipt],after:[$receipt],change:"none",output:$output}
' > "$logs/integration-2-self-policy.json"
run continuation-integration-2 "$controller" run "$logs/request-integration-2.json"
run continuation-production-self-validation "$controller" compare "$logs/integration-2-self-policy.json"
# Inventory only; the original Rust run/compare and Make test own acceptance.
jq '{request:.context.request,phases:[.phases[]|{name,success,error,exit:.command.exit_code}],children:.context.cargo.receipts}' "$original/integration-2/integration-2-cold-receipt.json" > "$logs/production-parent-summary.json"
while IFS= read -r child; do
  jq -c 'select(.context.original_argv[0] == "run") |
   {argv:.context.original_argv,effective_jobs:.context.effective_cargo_jobs,
    exit:.phases[0].command.exit_code,stdout:.phases[0].command.stdout,stderr:.phases[0].command.stderr,
    executable:.context.run_executable,capi_artifacts:.context.capi_artifacts}' "$child"
done < <(jq -r '.context.cargo.receipts[].path' "$original/integration-2/integration-2-cold-receipt.json") > "$logs/actual-run-child-inventory.jsonl"
run continuation-final-source-readback b3sum --check "$original/source-blake3.txt"
run continuation-final-controller-readback b3sum --check "$original/frozen-controller-blake3.txt"
run original-failure-final-readback cmp "$logs/original-failed-ratchet-preservation.log" <(b3sum "$original/ratchet.log" "$original/terminals.txt")
