#!/usr/bin/env bash
set -u
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8
export RUST_TEST_THREADS=1
export CARGO_TARGET_DIR=/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90
export CARGO_BUILD_BUILD_DIR=/opt/.cargo/slots/95be0b9034a0e1f3/0/build
logs=.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-serial-logs
run() {
    local label=$1
    shift
    printf '%q ' "$@" >> "$logs/commands.txt"
    printf '\n' >> "$logs/commands.txt"
    "$@" > "$logs/$label.log" 2>&1
    local result=$?
    printf '%s\t%s\n' "$label" "$result" >> "$logs/terminals.txt"
    printf '%s exit %s\n' "$label" "$result"
    if [ "$result" -ne 0 ]; then exit "$result"; fi
}
run list cargo test --locked --jobs 8 -p purrdf-capi --test native_profile -- --list
cat /proc/self/cgroup > "$logs/cgroup.txt"
cgroup_path=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cgroup_path/memory.max" "/sys/fs/cgroup$cgroup_path/memory.swap.max" > "$logs/resource-limits.txt"
cargo -V > "$logs/compiler.txt"
rustc -vV >> "$logs/compiler.txt"
run tests cargo test --locked --jobs 8 -p purrdf-capi --test native_profile
run clippy cargo clippy --locked --jobs 8 -p purrdf-capi --all-targets -- -D warnings
run controller-build cargo build --locked --jobs 8 -p purrdf-capi --example native_ci_profile --profile test --message-format=json-render-diagnostics
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
