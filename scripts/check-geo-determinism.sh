#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
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
# So this script computes ONE number two ways:
#   * natively, via `cargo run --example geo_digest`;
#   * under wasm32-unknown-unknown, by building a one-function cdylib
#     (crates/geo/determinism, excluded from the workspace) and calling its
#     export from Node's WebAssembly host;
# and fails unless they are equal to each other AND to the golden constant that
# crates/geo/tests/determinism.rs pins natively.
#
# The digest is folded over SERIALIZED BYTES — WKT and GeoJSON renderings, DE-9IM
# matrix strings, exact decimal measures, and the IEEE bit patterns of the
# xsd:double boundary — because byte identity of the answer a consumer sees is
# the only claim that covers the coordinate lexical forms, the matrix renderings
# and the double renderings at once. See crates/geo/src/determinism.rs.
#
# Not part of `make check`: it needs the wasm32 target and Node, and `make check`
# must stay runnable without either. `make geo-determinism` runs it, and CI runs
# it in the wasm job where both are already present.

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

# ---------------------------------------------------------------------------
# 1. The native digest.
# ---------------------------------------------------------------------------
native_output="$(cargo run --quiet --locked -p purrdf-geo --example geo_digest)"
native_digest="$(printf '%s\n' "$native_output" | sed -n 's/^digest=//p')"
native_corpus="$(printf '%s\n' "$native_output" | sed -n 's/^corpus_len=//p')"
[ -n "$native_digest" ] || fail "the native example printed no digest"
[ -n "$native_corpus" ] || fail "the native example printed no corpus length"

# ---------------------------------------------------------------------------
# 2. The wasm digest.
#
# The helper is OUTSIDE the workspace, so it is built from its own directory with
# its own lock file. `--target-dir` keeps its artifacts out of the workspace's,
# so a stale workspace build can never be mistaken for a fresh wasm one.
# ---------------------------------------------------------------------------
helper_dir="$PWD/crates/geo/determinism"
# CARGO_TARGET_DIR may be absolute (shared caches usually are), so it cannot be
# pasted after $PWD unconditionally — that would silently build into a nested
# path inside the repo rather than the cache, and the freshness the separate
# target directory buys would be lost.
case "${CARGO_TARGET_DIR:-}" in
/*) target_dir="$CARGO_TARGET_DIR/geo-determinism" ;;
"") target_dir="$PWD/target/geo-determinism" ;;
*) target_dir="$PWD/$CARGO_TARGET_DIR/geo-determinism" ;;
esac
mkdir -p "$target_dir"

(cd "$helper_dir" && cargo build --quiet --release \
	--target wasm32-unknown-unknown --target-dir "$target_dir")

wasm="$target_dir/wasm32-unknown-unknown/release/purrdf_geo_determinism.wasm"
[ -f "$wasm" ] || fail "the wasm module was not produced at $wasm"

wasm_output="$(node scripts/geo-determinism.mjs "$wasm")"

wasm_digest="$(printf '%s\n' "$wasm_output" | sed -n 's/^digest=//p')"
wasm_corpus="$(printf '%s\n' "$wasm_output" | sed -n 's/^corpus_len=//p')"
[ -n "$wasm_digest" ] || fail "the wasm module printed no digest"

# ---------------------------------------------------------------------------
# 3. The golden, read from the test that pins it natively.
#
# Read out of the test source rather than restated here, so the two cannot
# diverge: there is exactly one copy of the constant in the tree.
# ---------------------------------------------------------------------------
golden_file="crates/geo/tests/determinism.rs"
golden="$(sed -n 's/^const GOLDEN_DIGEST: u64 = 0x\([0-9a-f_]*\);.*/\1/p' "$golden_file" | tr -d '_')"
[ -n "$golden" ] || fail "no GOLDEN_DIGEST constant found in $golden_file"

# ---------------------------------------------------------------------------
# 4. Compare. All three, and a non-vacuity check on the corpus.
# ---------------------------------------------------------------------------
echo "native   digest=$native_digest corpus_len=$native_corpus"
echo "wasm32   digest=$wasm_digest corpus_len=$wasm_corpus"
echo "golden   digest=$golden"

[ "$native_corpus" -ge 20 ] || fail "the corpus is too small to be worth hashing ($native_corpus members)"
[ "$native_corpus" = "$wasm_corpus" ] ||
	fail "the two targets folded different corpus sizes ($native_corpus vs $wasm_corpus)"

if [ "$native_digest" != "$wasm_digest" ]; then
	fail "NATIVE AND WASM DISAGREE: $native_digest vs $wasm_digest.
  purrdf-geo's determinism claim is that these are equal by construction, so a
  difference is a real defect, not a tolerance to widen. Look for floating-point
  arithmetic that escaped the crate root's deny(clippy::float_arithmetic), for a
  usize-width assumption, or for iteration over a hash map reaching an output."
fi

if [ "$native_digest" != "$golden" ]; then
	fail "the digest moved: computed $native_digest, golden $golden.
  Both targets agree with each other, so this is a deliberate behaviour change
  rather than a portability defect. Update GOLDEN_DIGEST in $golden_file and say
  in the pull request WHICH output changed and why."
fi

echo "OK: native and wasm32 digests are identical, and match the pinned golden"
