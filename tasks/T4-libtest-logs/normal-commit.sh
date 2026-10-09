#!/usr/bin/env bash
set -eu
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8
export CARGO_TARGET_DIR=/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90
export CARGO_BUILD_BUILD_DIR=/opt/.cargo/slots/95be0b9034a0e1f3/0/build
logs=.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-libtest-logs
staged_paths=$(git diff --cached --name-only)
if [ "$staged_paths" != crates/rdf-capi/tests/support/profile.rs ]; then printf 'Unexpected staged paths; refusing commit\n' >&2; exit 1; fi
cat /proc/self/cgroup > "$logs/commit-cgroup.txt"
cgroup_path=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cgroup_path/memory.max" "/sys/fs/cgroup$cgroup_path/memory.swap.max" > "$logs/commit-resource-limits.txt"
set +e
git commit -m "Fix native profiling of C API Rust test artifacts" > "$logs/commit.log" 2>&1
result=$?
set -e
printf '%s\n' "$result" > "$logs/commit.exit"
printf 'normal commit exit %s\n' "$result"
exit "$result"
