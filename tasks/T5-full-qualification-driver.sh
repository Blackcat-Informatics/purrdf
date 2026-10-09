#!/usr/bin/env bash
set -eu
# Prepared only. Root must admit the sole local lane before launch.
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
logs=/opt/purrdf-308-task5-full-20261008.3ig79spB
controller=/opt/purrdf-308-runtime-refusal.hyKidvRk/native_ci_profile.frozen
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 CI=1
export CARGO_TARGET_DIR="$logs/target"
export CARGO_BUILD_BUILD_DIR="$logs/build"
[ ! -e "$logs/controller.exit" ] && [ ! -e "$logs/make-check.log" ]
[ ! -e "$CARGO_TARGET_DIR" ] && [ ! -e "$CARGO_BUILD_BUILD_DIR" ]
finish() {
 result=$?
 trap - EXIT
 printf '%s\n' "$result" > "$logs/controller.exit"
 exit "$result"
}
trap finish EXIT
[ "$(git rev-parse HEAD)" = 47c9cdfcffe9271facb85c05034d89e466a4d77a ]
git diff --exit-code > "$logs/unstaged-admission.log"
git diff --cached --exit-code > "$logs/staged-admission.log"
git status --porcelain=v1 > "$logs/status-before.txt"
git rev-parse HEAD 'HEAD^{tree}' > "$logs/source-before.txt"
git stash list > "$logs/stashes-before.txt"
b3sum --check /opt/purrdf-308-runtime-refusal.hyKidvRk/source-blake3.txt > "$logs/source-admission.txt"
b3sum --check /opt/purrdf-308-runtime-refusal.hyKidvRk/frozen-controller-blake3.txt > "$logs/controller-admission.txt"
cp -p "$controller" "$logs/native_ci_profile.frozen"
b3sum "$logs/native_ci_profile.frozen" > "$logs/frozen-controller-blake3.txt"
cat /proc/self/cgroup > "$logs/cgroup.txt"
cg=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cg/memory.max" "/sys/fs/cgroup$cg/memory.swap.max" > "$logs/resource-limits.txt"
[ "$(cat "/sys/fs/cgroup$cg/memory.max")" = 68719476736 ]
[ "$(cat "/sys/fs/cgroup$cg/memory.swap.max")" = 0 ]
printf 'CARGO_BUILD_JOBS=%s\nRUST_TEST_THREADS=%s\nCI=%s\nCARGO_TARGET_DIR=%s\nCARGO_BUILD_BUILD_DIR=%s\nRUSTC_WRAPPER=%s\nRUSTC_WORKSPACE_WRAPPER=%s\n' "$CARGO_BUILD_JOBS" "$RUST_TEST_THREADS" "$CI" "$CARGO_TARGET_DIR" "$CARGO_BUILD_BUILD_DIR" "${RUSTC_WRAPPER-}" "${RUSTC_WORKSPACE_WRAPPER-}" > "$logs/environment.txt"
cargo -Vv > "$logs/compiler.txt"
rustc -vV >> "$logs/compiler.txt"
command -v cargo rustc rustup node python3 wasm-opt make cc jq b3sum > "$logs/tool-paths.txt"
node --version > "$logs/node-version.txt"
python3 --version > "$logs/python-version.txt"
wasm-opt --version > "$logs/binaryen-version.txt"
[ "$(wasm-opt --version)" = 'wasm-opt version 130' ]
rustup target list --installed > "$logs/installed-targets.txt"
rg -x wasm32-unknown-unknown "$logs/installed-targets.txt"
df -h "$logs" > "$logs/disk-before.txt"
printf '%s\n' 'CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 CI=1 make check (private raw SDK target/build; unchanged inherited compiler wrappers)' > "$logs/commands.txt"
set +e
make check > "$logs/make-check.log" 2>&1
result=$?
set -e
printf '%s\n' "$result" > "$logs/make-check.exit"
[ "$result" = 0 ] || exit "$result"
b3sum --check /opt/purrdf-308-runtime-refusal.hyKidvRk/source-blake3.txt > "$logs/source-readback.txt"
b3sum --check "$logs/frozen-controller-blake3.txt" > "$logs/controller-readback.txt"
git rev-parse HEAD 'HEAD^{tree}' > "$logs/source-after.txt"
cmp "$logs/source-before.txt" "$logs/source-after.txt"
git diff --exit-code > "$logs/unstaged-readback.log"
git diff --cached --exit-code > "$logs/staged-readback.log"
git stash list > "$logs/stashes-after.txt"
cmp "$logs/stashes-before.txt" "$logs/stashes-after.txt"
git status --porcelain=v1 > "$logs/status-after.txt"
df -h "$logs" > "$logs/disk-after.txt"
