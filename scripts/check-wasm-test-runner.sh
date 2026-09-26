#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Observe scripts/wasm-test-runner.sh from outside, before `make wasm-test`
# trusts it with the determinism targets.
#
# Every wasm32 test in the workspace reports through that runner, so a runner
# that exited 0 on a failed case, lost a panic's message or ignored the host seal
# would turn every one of those tests into a false green. This script runs the
# testkit's two fixture binaries on wasm32-unknown-unknown through the runner and
# checks, each time against a neighbour that must succeed:
#
#   * a panicking case: exit 101, the case `FAILED` with its panic message, the
#     libtest `FAILED` tally, and a note naming how many cases did not run (a
#     panic aborts a wasm32 module); the same run skipping that case exits 0
#     with the ok tally;
#   * an unknown flag exits 101 with libtest's message; a known flag (a thread
#     count, which wasm32 runs serially) exits 0;
#   * a host clock read inside `without_host_clock_or_entropy` fails the run
#     naming `Date.now`; the same read outside it passes.
#
# Run by `make wasm-test` before its determinism targets, with the same runner
# setting; it needs the wasm32 target, the wasm-bindgen CLI and Node.

set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
	echo "FAIL: $*" >&2
	exit 1
}

export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$PWD/scripts/wasm-test-runner.sh"
scratch_root="${CARGO_TARGET_DIR:-$PWD/target}"
case "$scratch_root" in
/*) ;;
*) scratch_root="$PWD/$scratch_root" ;;
esac
scratch="$scratch_root/wasm-test-runner-check"
mkdir -p "$scratch"

# Build both fixtures once, so each observation below is only a run.
cargo build --quiet --locked --target wasm32-unknown-unknown -p purrdf-testkit \
	--bin testkit-harness-fixture --bin testkit-wasm-host-fixture

# Run fixture $1 on wasm32 with the remaining arguments; leave its exit status in
# $status and its output in $scratch/out and $scratch/err.
run() {
	local bin="$1"
	shift
	status=0
	cargo run --quiet --locked --target wasm32-unknown-unknown -p purrdf-testkit --bin "$bin" -- "$@" \
		>"$scratch/out" 2>"$scratch/err" || status=$?
}

expect() {
	local want="$1" what="$2"
	[ "$status" = "$want" ] || fail "$what: exit status $status, expected $want
--- stdout
$(cat "$scratch/out")
--- stderr
$(cat "$scratch/err")"
}

contains() {
	local file="$1" text="$2" what="$3"
	grep -qF -- "$text" "$scratch/$file" || fail "$what: $file lacks '$text'
--- stdout
$(cat "$scratch/out")
--- stderr
$(cat "$scratch/err")"
}

run testkit-harness-fixture
expect 101 "a panicking case"
contains out "test alpha_passes ... ok" "a panicking case"
contains out "test delta_panics ... FAILED" "a panicking case"
contains out "---- delta_panics stdout ----" "a panicking case"
contains out "thread 'delta_panics' panicked at crates/testkit/src/bin/harness_fixture.rs:" "a panicking case"
contains out "the planted failure" "a panicking case"
contains out "test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in " "a panicking case"
contains err "the 2 selected case(s) after it did not run" "a panicking case"
contains err "wasm-test-runner: the module trapped or threw: RuntimeError: unreachable" "a panicking case"

run testkit-harness-fixture --skip delta_panics
expect 0 "the same run without the panicking case"
contains out "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 1 filtered out; finished in " "the same run without the panicking case"

run testkit-harness-fixture --frobnicate
expect 101 "an unknown flag"
contains err "error: Unrecognized option: 'frobnicate'" "an unknown flag"

run testkit-harness-fixture --test-threads=4 --skip delta_panics
expect 0 "a known flag"
contains out "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 1 filtered out; finished in " "a known flag"

run testkit-wasm-host-fixture --exact a_sealed_host_read_fails_by_name
expect 101 "a sealed host clock read"
contains err "the code under test called Date.now inside without_host_clock_or_entropy" "a sealed host clock read"

run testkit-wasm-host-fixture --exact an_unsealed_host_read_passes
expect 0 "an unsealed host clock read"
contains out "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in " "an unsealed host clock read"

echo "OK: the wasm32 test runner fails a panicking case, a refused flag and a sealed host read, each beside a neighbour that passes"
