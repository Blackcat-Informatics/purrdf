#!/usr/bin/env bash
set -u
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8
export RUST_TEST_THREADS=1
export CARGO_TARGET_DIR=/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90
export CARGO_BUILD_BUILD_DIR=/opt/.cargo/slots/95be0b9034a0e1f3/0/build
logs=.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-serial-corrected-logs
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
run ratchet cargo run --quiet --locked --jobs 8 -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
run generated bash scripts/check-generated.sh
