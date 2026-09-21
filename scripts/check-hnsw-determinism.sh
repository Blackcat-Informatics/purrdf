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
# native golden. This script proves the OTHER half: it builds the one-function
# cdylib in crates/hnsw/determinism (excluded from the workspace) for
# wasm32-unknown-unknown, calls its export from Node's WebAssembly host, and fails
# unless the wasm digest equals the same golden and folds the same corpus.
#
# Two numbers, one constant, no reasoning in between. If native and wasm disagree,
# the portability guarantee has broken and the digest is the least interesting part
# of the problem.
#
# Not part of `make check`: it needs the wasm32 target and Node, and `make check`
# must stay runnable without either. `make hnsw-determinism` runs it, and CI runs
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
# 1. The golden and the expected corpus size, read from the test and the source
#    that define them. Reading them out of the tree rather than restating them
#    here means there is exactly one copy and the assertions cannot drift.
# ---------------------------------------------------------------------------
golden_file="crates/hnsw/tests/determinism.rs"
golden="$(sed -n 's/^const GOLDEN_DIGEST: u64 = 0x\([0-9a-f_]*\);.*/\1/p' "$golden_file" | tr -d '_')"
[ -n "$golden" ] || fail "no GOLDEN_DIGEST constant found in $golden_file"

corpus_file="crates/hnsw/src/determinism.rs"
expected_corpus="$(sed -n 's/^pub const CORPUS_ROWS: usize = \([0-9_]*\);.*/\1/p' "$corpus_file" | tr -d '_')"
[ -n "$expected_corpus" ] || fail "no CORPUS_ROWS constant found in $corpus_file"

# ---------------------------------------------------------------------------
# 2. The wasm digest.
#
# The helper is OUTSIDE the workspace, so it is built from its own directory with
# its own lock file. `--target-dir` keeps its artifacts out of the workspace's, so
# a stale workspace build can never be mistaken for a fresh wasm one.
# ---------------------------------------------------------------------------
helper_dir="$PWD/crates/hnsw/determinism"
# CARGO_TARGET_DIR may be absolute (shared caches usually are), so it cannot be
# pasted after $PWD unconditionally — that would silently build into a nested
# path inside the repo rather than the cache, and the freshness the separate
# target directory buys would be lost.
case "${CARGO_TARGET_DIR:-}" in
/*) target_dir="$CARGO_TARGET_DIR/hnsw-determinism" ;;
"") target_dir="$PWD/target/hnsw-determinism" ;;
*) target_dir="$PWD/$CARGO_TARGET_DIR/hnsw-determinism" ;;
esac
mkdir -p "$target_dir"

(cd "$helper_dir" && cargo build --quiet --release \
	--target wasm32-unknown-unknown --target-dir "$target_dir")

wasm="$target_dir/wasm32-unknown-unknown/release/purrdf_hnsw_determinism.wasm"
[ -f "$wasm" ] || fail "the wasm module was not produced at $wasm"

wasm_output="$(node scripts/hnsw-determinism.mjs "$wasm")"

wasm_digest="$(printf '%s\n' "$wasm_output" | sed -n 's/^digest=//p')"
wasm_corpus="$(printf '%s\n' "$wasm_output" | sed -n 's/^corpus_len=//p')"
[ -n "$wasm_digest" ] || fail "the wasm module printed no digest"
[ -n "$wasm_corpus" ] || fail "the wasm module printed no corpus length"

# ---------------------------------------------------------------------------
# 3. Compare. All of them, plus a non-vacuity check on the corpus.
# ---------------------------------------------------------------------------
echo "wasm32   digest=$wasm_digest corpus_len=$wasm_corpus"
echo "golden   digest=$golden corpus_len=$expected_corpus"

[ "$wasm_corpus" = "$expected_corpus" ] ||
	fail "the wasm digest folded $wasm_corpus rows, but the corpus has $expected_corpus"

if [ "$wasm_digest" != "$golden" ]; then
	fail "WASM AND THE NATIVE GOLDEN DISAGREE: $wasm_digest vs $golden.
  purrdf-hnsw's determinism claim is that these are equal by construction, so a
  difference is a real defect, not a tolerance to widen. Look for a float
  operation that is not associative across targets, a usize-width assumption, or
  iteration over a hash map reaching an output."
fi

echo "OK: the wasm32 digest is identical to the natively pinned golden"
