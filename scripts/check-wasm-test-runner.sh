#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Observe scripts/wasm-test-runner.sh failing what it must fail before any wasm32
# test result is trusted. A runner that exited 0 whatever the module did would
# make every `make wasm-test` row green and prove nothing, so every refusal here
# sits beside a neighbouring run that must PASS:
#
#   * a panicking case fails the run with 101, reported `FAILED` with its
#     message, the tally and the count of cases not run; the same fixture
#     filtered to a passing case exits 0;
#   * a flag the harness does not implement is refused with 101; a flag it
#     implements runs;
#   * a host clock read (`Date.now`) and a host entropy read (`Math.random`)
#     inside purrdf_testkit::harness::without_host_clock_or_entropy fail by the
#     source's name, and so does a read inside an outer seal after an inner one
#     ends; the same reads outside the seal, and after it, pass;
#   * a module that never reports an exit status is refused; the print
#     fixture, which does report one, passes, and its print_line lines reach
#     the console.
#
# The fixtures are purrdf-testkit's own bins, built for wasm32-unknown-unknown.
# Needs the wasm32 target, the pinned wasm-bindgen CLI and Node, so it is not
# part of `make check`. `make wasm-test` runs it first. Usage:
#
#     bash scripts/check-wasm-test-runner.sh

set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
	printf 'FAIL: %s\n' "$*" >&2
	exit 1
}

command -v node >/dev/null 2>&1 || fail "node not found; it is required to run the wasm module"

if ! rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then
	if [ -n "${CI:-}" ]; then
		fail "wasm32-unknown-unknown target absent in CI"
	fi
	echo "SKIP: wasm32-unknown-unknown target not installed — 'rustup target add wasm32-unknown-unknown' to enable"
	exit 0
fi

if ! command -v wasm-bindgen >/dev/null 2>&1; then
	if [ -n "${CI:-}" ]; then
		fail "the wasm-bindgen CLI is absent in CI"
	fi
	echo "SKIP: the wasm-bindgen CLI is not on PATH — install wasm-bindgen-cli $(sed -n 's/^wasm-bindgen = "=\([0-9][0-9.]*\)"$/\1/p' Cargo.toml) to enable ('make doctor')"
	exit 0
fi

runner="$PWD/scripts/wasm-test-runner.sh"

cargo build --locked --quiet --target wasm32-unknown-unknown -p purrdf-testkit --bins ||
	fail "the testkit fixtures do not build for wasm32-unknown-unknown"
target_dir="$(cargo metadata --locked --format-version 1 --no-deps |
	python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
bins="$target_dir/wasm32-unknown-unknown/debug"
harness_fixture="$bins/testkit-harness-fixture.wasm"
print_fixture="$bins/testkit-harness-print-fixture.wasm"
host_fixture="$bins/testkit-wasm-host-fixture.wasm"
for module in "$harness_fixture" "$print_fixture" "$host_fixture"; do
	[ -f "$module" ] || fail "the build produced no $module"
done

# ---------------------------------------------------------------------------
# One observation: run the runner, then hold its status and console to a claim.
# ---------------------------------------------------------------------------
label=""
out=""
status=0

observe() {
	label="$1"
	shift
	if out="$("$runner" "$@" 2>&1)"; then
		status=0
	else
		status=$?
	fi
}

expect_status() {
	[ "$status" = "$1" ] || fail "$label: exited $status, expected $1. Output:
$out"
}

expect_text() {
	[[ "$out" == *"$1"* ]] || fail "$label: the output lacks \`$1\`. Output:
$out"
}

refute_text() {
	if [[ "$out" == *"$1"* ]]; then
		fail "$label: the output contains \`$1\`, which it must not. Output:
$out"
	fi
}

passed() {
	echo "ok: $label"
}

# ---------------------------------------------------------------------------
# 1. A panicking case, beside a passing one.
# ---------------------------------------------------------------------------
observe "a panicking case fails the run" "$harness_fixture"
expect_status 101
expect_text "test alpha_passes ... ok"
expect_text "test delta_panics ... FAILED"
expect_text "---- delta_panics stdout ----"
expect_text "thread 'delta_panics' panicked at "
expect_text "the planted failure"
expect_text "note: 2 cases not run: a panic ends a wasm32 run at the case that panicked"
expect_text "test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in "
refute_text "gamma_passes ... ok"
passed

observe "the passing neighbour of the panicking case passes" "$harness_fixture" --exact alpha_passes
expect_status 0
expect_text "test alpha_passes ... ok"
expect_text "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in "
passed

observe "the case list is the fixture's, in its order" "$harness_fixture" --list
expect_status 0
expect_text "alpha_passes: test
beta_passes: test
delta_panics: test
epsilon_is_ignored: test
gamma_passes: test

5 tests, 0 benchmarks"
passed

# ---------------------------------------------------------------------------
# 2. A refused flag, beside an accepted one.
# ---------------------------------------------------------------------------
observe "an unimplemented flag is refused" "$harness_fixture" --frobnicate
expect_status 101
expect_text "error: Unrecognized option: 'frobnicate'"
refute_text "running "
passed

observe "an implemented flag is accepted" "$harness_fixture" --test-threads=1 alpha
expect_status 0
expect_text "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in "
passed

# ---------------------------------------------------------------------------
# 3. Sealed host reads, beside the same reads unsealed.
# ---------------------------------------------------------------------------
for sealed in \
	"a_host_clock_read_inside_the_seal_fails Date.now" \
	"a_host_entropy_read_inside_the_seal_fails Math.random" \
	"a_host_clock_read_after_a_nested_seal_fails Date.now"; do
	case_name="${sealed% *}"
	source_name="${sealed#* }"
	observe "$case_name: a sealed $source_name read fails by name" "$host_fixture" --exact "$case_name"
	expect_status 101
	expect_text "test $case_name ... FAILED"
	expect_text "$source_name is sealed"
	expect_text "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in "
	passed
done

for open_case in \
	a_host_clock_read_outside_the_seal_passes \
	a_host_entropy_read_outside_the_seal_passes \
	a_host_clock_read_after_the_seal_passes \
	the_seal_returns_the_computations_value_passes; do
	observe "$open_case: the unsealed neighbour passes" "$host_fixture" --exact "$open_case"
	expect_status 0
	expect_text "test $open_case ... ok"
	expect_text "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in "
	passed
done

# ---------------------------------------------------------------------------
# 4. A module that never reports an exit status, beside one that does.
# ---------------------------------------------------------------------------
scratch_base="$bins/wasm-test-runner"
mkdir -p "$scratch_base"
scratch="$(mktemp -d "$scratch_base/check.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
# The smallest valid module: the magic number and version, nothing else. It
# loads, runs nothing and reports nothing.
printf '\0asm\1\0\0\0' >"$scratch/silent.wasm"
observe "a module that never reports an exit status is refused" "$scratch/silent.wasm"
[ "$status" != 0 ] || fail "$label: exited 0. Output:
$out"
expect_text "never reported an exit status"
passed

observe "the print fixture reports its status and its lines reach the console" "$print_fixture"
expect_status 0
expect_text "print-fixture case=prints_through_print_line line=0"
expect_text "print-fixture case=prints_more_through_print_line line=31"
expect_text "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in "
passed

echo "OK: the wasm32 test runner fails a panicking case, a refused flag, sealed host reads and a silent module, and passes each neighbour"
