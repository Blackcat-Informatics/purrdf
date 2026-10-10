#!/usr/bin/env bash
set -euo pipefail
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4
export CARGO_TARGET_DIR=/opt/purrdf-schema-499-qualification/target
export CARGO_BUILD_BUILD_DIR=/opt/purrdf-schema-499-qualification/build
export TMPDIR=/opt/purrdf-schema-499-qualification/tmp
cd /home/paudley/Active/purrdf/.worktrees/499-entailment-prepare-schema-invariant
exec systemd-run --user --scope --quiet -p MemoryMax=16G -p MemorySwapMax=0 -- "$@"
