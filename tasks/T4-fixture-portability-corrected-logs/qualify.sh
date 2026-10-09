#!/usr/bin/env bash
set -eu
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1
export CARGO_TARGET_DIR="/home/paudley/Active/.purrdf-308-fixture-portability-20261008/target"
export CARGO_BUILD_BUILD_DIR="/home/paudley/Active/.purrdf-308-fixture-portability-20261008/build"
logs=/opt/purrdf-308-fixture-portability.L4bBkXQS/corrected
mirror="$PWD/.stage/reduce-native-ci-test-compilation-and-c/tasks/T4-fixture-portability-corrected-logs"
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
printf 'RUSTC_WRAPPER=%s\nRUSTC_WORKSPACE_WRAPPER=%s\nTARGET=%s\nBUILD=%s\n' "${RUSTC_WRAPPER-}" "${RUSTC_WORKSPACE_WRAPPER-}" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" > "$logs/environment.txt"
cargo -Vv > "$logs/compiler.txt"
rustc -vV >> "$logs/compiler.txt"
b3sum crates/rdf-capi/tests/support/hosted.rs crates/rdf-capi/tests/support/profile.rs crates/rdf-capi/tests/support/phases.rs crates/rdf-capi/examples/native_ci_profile.rs .github/workflows/ci.yaml CONTRIBUTING.md Cargo.toml Cargo.lock /home/paudley/.cargo/config.toml > "$logs/source-blake3.txt"
git rev-parse HEAD > "$logs/source-ref.txt"
git diff --cached --binary > "$logs/staged-source.patch"
run list cargo test --locked --jobs 8 -p purrdf-capi --test native_profile -- --list
run tests cargo test --locked --jobs 8 -p purrdf-capi --test native_profile
run clippy cargo clippy --locked --jobs 8 -p purrdf-capi --all-targets -- -D warnings
run controller-build cargo build --locked --jobs 8 -p purrdf-capi --example native_ci_profile --profile test --message-format=json-render-diagnostics
python3 "$logs/freeze-controller.py" "$logs"
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
cp "$logs"/* "$mirror/"
