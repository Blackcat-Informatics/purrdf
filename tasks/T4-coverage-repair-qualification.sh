#!/usr/bin/env bash
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
logs=/opt/purrdf-308-coverage-repair-20261008.G8BEKgFM
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 CI=1
export CARGO_TARGET_DIR=/opt/purrdf-308-task5-full-20261008.3ig79spB/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-308-task5-full-20261008.3ig79spB/build
finish() { result=$?; trap - EXIT; printf '%s\n' "$result" > "$logs/qualification.exit"; exit "$result"; }
trap finish EXIT
run() {
 local label=$1; shift
 set +e
 "$@" > "$logs/$label.log" 2>&1
 local result=$?
 set -e
 printf '%s\n' "$result" > "$logs/$label.exit"
 if test "$result" -ne 0; then tail -n 65 "$logs/$label.log"; return "$result"; fi
 printf '%s exit0\n' "$label"
}
cat /proc/self/cgroup > "$logs/qualification-cgroup.txt"
control_group=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$control_group/memory.max" "/sys/fs/cgroup$control_group/memory.swap.max" > "$logs/qualification-limits.txt"
cargo -Vv > "$logs/compiler.txt"
rustc -vV >> "$logs/compiler.txt"
run native-profile cargo test --locked -p purrdf-capi --test native_profile --jobs 8
run clippy cargo clippy --locked -p purrdf-capi --all-targets --jobs 8 -- -D warnings
run controller cargo build --locked -p purrdf-capi --example native_ci_profile --profile test --jobs 8 --message-format=json-render-diagnostics
jq -er 'select(.reason=="compiler-artifact" and .target.name=="native_ci_profile" and .executable!=null)|.executable' "$logs/controller.log" > "$logs/controller-path.txt"
test "$(wc -l < "$logs/controller-path.txt")" -eq 1
cp -- "$(cat "$logs/controller-path.txt")" "$logs/native_ci_profile.corrected.frozen"
run fmt cargo fmt --all -- --check
