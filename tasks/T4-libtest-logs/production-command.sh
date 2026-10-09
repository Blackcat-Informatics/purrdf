#!/usr/bin/env bash
set -eu
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1
export CARGO_TARGET_DIR=/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90
export CARGO_BUILD_BUILD_DIR=/opt/.cargo/slots/95be0b9034a0e1f3/0/build
export PURRDF_PROFILE_REAL_CARGO=/home/paudley/stage/packages/rustup/active-toolchain/bin/cargo
export PURRDF_PROFILE_TELEMETRY=/home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c/.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-libtest-logs/production-cargo
logs=.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-libtest-logs
controller=/home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c/.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-libtest-logs/production-bin/cargo
cat /proc/self/cgroup > "$logs/production-cgroup.txt"
cgroup_path=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cgroup_path/memory.max" "/sys/fs/cgroup$cgroup_path/memory.swap.max" > "$logs/production-resource-limits.txt"
printf '%q ' "$controller" test --locked --jobs 8 -p purrdf-capi --lib > "$logs/production-command.txt"
printf '
' >> "$logs/production-command.txt"
set +e
"$controller" test --locked --jobs 8 -p purrdf-capi --lib > "$logs/production-shim.log" 2>&1
result=$?
set -e
printf '%s
' "$result" > "$logs/production-exit.txt"
printf 'production-shim exit %s
' "$result"
exit "$result"
