#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Prove purrdf-geo's central claim by RUNNING it on both targets, not by
# reasoning about it.
#
# The claim: because every geometric decision in purrdf-geo is integer
# arithmetic, and Rust specifies integer arithmetic completely and identically on
# every target, a native answer and a wasm32-unknown-unknown answer are
# bit-identical. That is an argument, and an argument is not evidence — the
# failure mode the crate exists to prevent is exactly the one that produces no
# symptom.
#
# So this script runs ONE test target, crates/geo/tests/determinism.rs, twice:
#   * natively, under `cargo test`;
#   * on wasm32-unknown-unknown, in Node, under the same cargo runner every
#     wasm32 test uses (scripts/wasm-test-runner.sh).
# The target is `harness = false` on the shared test runner, so both runs execute
# the same named cases. Each case that computes the digest prints a
# `determinism-digest case=<name> digest=<hex> corpus_len=<n>` line, and this
# script fails unless both runs report the same named cases with the same digests,
# every one of them equal to the golden constant the test file pins.
#
# The digest is folded over SERIALIZED BYTES — WKT and GeoJSON renderings, DE-9IM
# matrix strings, exact decimal measures, and the IEEE bit patterns of the
# xsd:double boundary — because byte identity of the answer a consumer sees is
# the only claim that covers the coordinate lexical forms, the matrix renderings
# and the double renderings at once. See crates/geo/src/determinism.rs. On wasm32
# the digest is computed with every host clock and entropy source sealed, so a
# digest that reached one fails by that source's name instead of agreeing by
# accident.
#
# Not part of `make check`: it needs the wasm32 target, the wasm-bindgen CLI and
# Node, and `make check` must stay runnable without them. `make geo-determinism`
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
# 1. The golden, read from the test that pins it.
#
# Read out of the test source rather than restated here, so the two cannot
# diverge: there is exactly one copy of the constant in the tree.
# ---------------------------------------------------------------------------
golden_file="crates/geo/tests/determinism.rs"
golden="$(sed -n 's/^const GOLDEN_DIGEST: u64 = 0x\([0-9a-f_]*\);.*/\1/p' "$golden_file" | tr -d '_')"
[ -n "$golden" ] || fail "no GOLDEN_DIGEST constant found in $golden_file"

# The `determinism-digest` lines a run printed, one per reporting case, sorted by
# case name. A line may follow libtest's `test <name> ... ` on the same console
# line, so the record is matched wherever it starts.
digests() {
	grep -o 'determinism-digest case=[A-Za-z0-9_]* digest=[0-9a-f]\{16\} corpus_len=[0-9]*' |
		sed 's/^determinism-digest //' | sort
}

# ---------------------------------------------------------------------------
# 2. Both runs. Each must pass on its own: every case asserts the golden.
# ---------------------------------------------------------------------------
native_output="$(cargo test --locked -p purrdf-geo --test determinism)" ||
	fail "the native determinism target failed"
wasm_output="$(CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$runner" \
	cargo test --locked --target wasm32-unknown-unknown -p purrdf-geo --test determinism)" ||
	fail "the wasm32 determinism target failed"

native="$(printf '%s\n' "$native_output" | digests)"
wasm="$(printf '%s\n' "$wasm_output" | digests)"

echo "native:"
printf '%s\n' "$native" | sed 's/^/  /'
echo "wasm32:"
printf '%s\n' "$wasm" | sed 's/^/  /'
echo "golden   digest=$golden"

# ---------------------------------------------------------------------------
# 3. Compare, case by case, plus a non-vacuity check on the corpus.
# ---------------------------------------------------------------------------
[ -n "$native" ] || fail "the native run reported no digest"
[ -n "$wasm" ] || fail "the wasm32 run reported no digest"

if [ "$native" != "$wasm" ]; then
	fail "NATIVE AND WASM DISAGREE:
$(diff <(printf '%s\n' "$native") <(printf '%s\n' "$wasm") || true)
  purrdf-geo's determinism claim is that these are equal by construction, so a
  difference is a real defect, not a tolerance to widen. Look for floating-point
  arithmetic that escaped the crate root's deny(clippy::float_arithmetic), for a
  usize-width assumption, or for iteration over a hash map reaching an output."
fi

while read -r case digest corpus; do
	digest="${digest#digest=}"
	corpus="${corpus#corpus_len=}"
	[ "$corpus" -ge 20 ] || fail "$case folded too small a corpus to be worth hashing ($corpus members)"
	if [ "$digest" != "$golden" ]; then
		fail "the digest moved: $case computed $digest, golden $golden.
  Both targets agree with each other, so this is a deliberate behaviour change
  rather than a portability defect. Update GOLDEN_DIGEST in $golden_file and say
  in the pull request WHICH output changed and why."
	fi
done <<<"$native"

echo "OK: every named case reports the same digest natively and on wasm32, and it matches the pinned golden"
