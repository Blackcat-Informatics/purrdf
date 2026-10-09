#!/usr/bin/env bash
set -eu
cd /home/paudley/Active/purrdf/.worktrees/308-reduce-native-ci-test-compilation-and-c
r=/opt/purrdf-308-collector-publication-20261008.dWBh1wNr
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
cat /proc/self/cgroup > "$r/commit-cgroup.txt"
cg=$(cut -d: -f3 /proc/self/cgroup)
cat "/sys/fs/cgroup$cg/memory.max" "/sys/fs/cgroup$cg/memory.swap.max" > "$r/commit-resource-limits.txt"
[ "$(cat "/sys/fs/cgroup$cg/memory.max")" = 68719476736 ]
[ "$(cat "/sys/fs/cgroup$cg/memory.swap.max")" = 0 ]
cargo -Vv > "$r/commit-compiler.txt"
rustc -vV >> "$r/commit-compiler.txt"
set +e
git commit -S -m 'Preserve compiled Cargo runtime refusals in native CI receipts' > "$r/commit.log" 2>&1
result=$?
set -e
printf '%s\n' "$result" > "$r/commit.exit"
cat "$r/commit.log"
exit "$result"
