#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Prove purrdf-hnsw's central claim by RUNNING it on the wasm32 target, not by
# reasoning about it.
#
# The claim: the round-structured build plus splitmix64 levels make the graph a
# pure function of the input, so a native build and a wasm32-unknown-unknown build
# produce the same canonical byte image. crates/hnsw/tests/determinism.rs proves the
# thread-count half natively (1/2/4/8 workers, one pinned golden) and pins the
# goldens. It is `harness = false` on the shared test runner, so the same named
# cases also run on wasm32-unknown-unknown, in Node, under the cargo runner every
# wasm32 test uses (scripts/wasm-test-runner.sh). This script runs the target three
# times — natively, on the baseline wasm32 build, and on a wasm32 build with
# `-C target-feature=+simd128`, where LLVM packs the exact distance fold's sixteen
# lanes into f64x2 operations — and fails unless every reporting case prints the
# same digest on all three, each equal to the golden the test file pins for it. The
# two wasm modules must also differ, so the SIMD build is proven to be a different
# compilation rather than the same bytes twice.
#
# Each case that computes a digest prints a
# `determinism-digest case=<name> digest=<hex> corpus_len=<n>` line. On wasm32 the
# digests are computed with every host clock and entropy source sealed, so a build
# that reached one fails by that source's name instead of agreeing by accident.
#
# Not part of `make check`: it needs the wasm32 target, the wasm-bindgen CLI and
# Node, and `make check` must stay runnable without them. `make hnsw-determinism`
# runs it, and CI runs it in the wasm job where all three are already present.

set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
	echo "FAIL: $*" >&2
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

# ---------------------------------------------------------------------------
# 1. The goldens and the expected corpus size, read from the test and the source
#    that define them. Reading them out of the tree rather than restating them
#    here means there is exactly one copy and the assertions cannot drift.
# ---------------------------------------------------------------------------
golden_file="crates/hnsw/tests/determinism.rs"
golden="$(sed -n 's/^const GOLDEN_DIGEST: u64 = 0x\([0-9a-f_]*\);.*/\1/p' "$golden_file" | tr -d '_')"
[ -n "$golden" ] || fail "no GOLDEN_DIGEST constant found in $golden_file"
serial_golden="$(sed -n 's/^const GOLDEN_SERIAL_DIGEST: u64 = 0x\([0-9a-f_]*\);.*/\1/p' "$golden_file" | tr -d '_')"
[ -n "$serial_golden" ] || fail "no GOLDEN_SERIAL_DIGEST constant found in $golden_file"

corpus_file="crates/hnsw/src/determinism.rs"
expected_corpus="$(sed -n 's/^pub const CORPUS_ROWS: usize = \([0-9_]*\);.*/\1/p' "$corpus_file" | tr -d '_')"
[ -n "$expected_corpus" ] || fail "no CORPUS_ROWS constant found in $corpus_file"

# The +simd128 build's flags. Cargo ignores target-scoped flags whenever RUSTFLAGS or
# CARGO_ENCODED_RUSTFLAGS is set, so a caller's RUSTFLAGS is folded into the
# target-scoped value and unset for that build, and CARGO_ENCODED_RUSTFLAGS is refused.
# A target-scoped value also replaces build.rustflags, so -D warnings is restated.
[ -z "${CARGO_ENCODED_RUSTFLAGS:-}" ] ||
	fail "CARGO_ENCODED_RUSTFLAGS is set; Cargo would ignore the +simd128 build's target-scoped flags"
simd_flags="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128"
baseline_env=(env)
simd_env=(env -u RUSTFLAGS CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$simd_flags")

# The wasm module cargo reports it built for the determinism target under the
# environment given as arguments. The path is read from cargo's own
# `compiler-artifact` message, never assumed.
wasm_module() {
	local messages module
	messages="$("$@" cargo test --locked --no-run --target wasm32-unknown-unknown \
		-p purrdf-hnsw --test determinism --message-format=json-render-diagnostics)" ||
		fail "the wasm32 determinism target did not build"
	module="$(printf '%s\n' "$messages" | grep '"reason":"compiler-artifact"' |
		sed -n 's/.*"executable":"\([^"]*\/determinism-[0-9a-f]*\.wasm\)".*/\1/p' | tail -n 1)"
	[ -n "$module" ] || fail "cargo reported no determinism test module"
	[ -f "$module" ] || fail "cargo reported the module at $module, which does not exist"
	printf '%s\n' "$module"
}

baseline_module="$(wasm_module "${baseline_env[@]}")"
simd_module="$(wasm_module "${simd_env[@]}")"
if cmp -s "$baseline_module" "$simd_module"; then
	fail "the +simd128 module is byte-identical to the baseline one; the target feature did not reach the build"
fi

# The `determinism-digest` records a run printed, sorted by case name. A record may
# follow libtest's `test <name> ... ` on the same console line, so it is matched
# wherever it starts.
digests() {
	grep -o 'determinism-digest case=[A-Za-z0-9_]* digest=[0-9a-f]\{16\} corpus_len=[0-9]*' |
		sed 's/^determinism-digest //' | sort
}

# ---------------------------------------------------------------------------
# 2. The three runs. Each must pass on its own: every case asserts its golden.
# ---------------------------------------------------------------------------
native_output="$(cargo test --locked -p purrdf-hnsw --test determinism)" ||
	fail "the native determinism target failed"
wasm_output="$("${baseline_env[@]}" CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$runner" \
	cargo test --locked --target wasm32-unknown-unknown -p purrdf-hnsw --test determinism)" ||
	fail "the wasm32 determinism target failed"
simd_output="$("${simd_env[@]}" CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$runner" \
	cargo test --locked --target wasm32-unknown-unknown -p purrdf-hnsw --test determinism)" ||
	fail "the wasm32+simd128 determinism target failed"

native="$(printf '%s\n' "$native_output" | digests)"
wasm="$(printf '%s\n' "$wasm_output" | digests)"
simd="$(printf '%s\n' "$simd_output" | digests)"

echo "native:"
printf '%s\n' "$native" | sed 's/^/  /'
echo "wasm32:"
printf '%s\n' "$wasm" | sed 's/^/  /'
echo "wasm32+simd128:"
printf '%s\n' "$simd" | sed 's/^/  /'
echo "golden          digest=$golden serial=$serial_golden corpus_len=$expected_corpus"

# ---------------------------------------------------------------------------
# 3. Compare, case by case, plus a non-vacuity check on the corpus.
# ---------------------------------------------------------------------------
for pair in "native:$native" "wasm32:$wasm" "wasm32+simd128:$simd"; do
	[ -n "${pair#*:}" ] || fail "the ${pair%%:*} run reported no digest"
done

for pair in "wasm32:$wasm" "wasm32+simd128:$simd"; do
	build="${pair%%:*}"
	if [ "${pair#*:}" != "$native" ]; then
		fail "$build AND NATIVE DISAGREE:
$(diff <(printf '%s\n' "$native") <(printf '%s\n' "${pair#*:}") || true)
  purrdf-hnsw's determinism claim is that these are equal by construction, so a
  difference is a real defect, not a tolerance to widen. Look for a float
  operation that is not associative across targets or builds, a usize-width
  assumption, or iteration over a hash map reaching an output."
	fi
done

while read -r case digest corpus; do
	case="${case#case=}"
	digest="${digest#digest=}"
	corpus="${corpus#corpus_len=}"
	[ "$corpus" = "$expected_corpus" ] ||
		fail "$case folded $corpus rows, but the corpus has $expected_corpus"
	if [ "$case" = "a_serial_insert_builds_a_different_graph" ]; then
		expected="$serial_golden"
	else
		expected="$golden"
	fi
	[ "$digest" = "$expected" ] ||
		fail "$case reports $digest on every build, but its golden is $expected"
done <<<"$native"

echo "OK: every named case reports the same digest natively, on wasm32 and on wasm32+simd128, and each matches its natively pinned golden"
