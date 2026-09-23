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
# The wasm module is built twice: on the baseline wasm32 target, and with
# `-C target-feature=+simd128`, where LLVM packs the exact distance fold's sixteen
# lanes into f64x2 operations. Both digests must equal the same golden, so the
# vectorized compilation is held to the scalar one's bits, and the two modules must
# differ, so the SIMD build is proven to be a different compilation rather than the
# same bytes twice.
#
# Three numbers, one constant, no reasoning in between. If native and wasm disagree,
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
# a stale workspace build can never be mistaken for a fresh wasm one, and the module
# folded is the one cargo reports it just wrote.
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

# Build the helper for wasm32 and print the path of the module cargo reports it wrote.
#
# The path is read from cargo's own `compiler-artifact` message, never assumed from
# `--target-dir`: a cargo wrapper is entitled to place final artifacts somewhere else,
# and then the assumed path is either empty or, worse, holds a stale module from an
# earlier build that the digest would silently fold instead. Any arguments are an
# `env` prefix for the build.
build_module() {
	local messages module
	messages="$(cd "$helper_dir" && env "$@" cargo build --quiet --release \
		--target wasm32-unknown-unknown --target-dir "$target_dir" \
		--message-format=json-render-diagnostics)" || fail "the wasm helper did not build"
	module="$(printf '%s\n' "$messages" | grep '"reason":"compiler-artifact"' |
		sed -n 's/.*"\([^"]*\/purrdf_hnsw_determinism\.wasm\)".*/\1/p' | tail -n 1)"
	[ -n "$module" ] || fail "cargo reported no purrdf_hnsw_determinism.wasm artifact"
	[ -f "$module" ] || fail "cargo reported the wasm module at $module, which does not exist"
	printf '%s\n' "$module"
}

# Each module is copied out the moment it is built, into a file this script owns, so
# the second build can never overwrite the first one's module before it is run.
wasm="$target_dir/purrdf_hnsw_determinism.wasm"
cp "$(build_module)" "$wasm"

# The +simd128 module. Cargo ignores target-scoped flags whenever RUSTFLAGS or
# CARGO_ENCODED_RUSTFLAGS is set, so a caller's RUSTFLAGS is folded into the
# target-scoped value and unset for this build, and CARGO_ENCODED_RUSTFLAGS is refused.
# A target-scoped value also replaces build.rustflags, so -D warnings is restated.
[ -z "${CARGO_ENCODED_RUSTFLAGS:-}" ] ||
	fail "CARGO_ENCODED_RUSTFLAGS is set; Cargo would ignore the +simd128 build's target-scoped flags"
simd_flags="${RUSTFLAGS:-} ${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:-} -D warnings -C target-feature=+simd128"
simd_wasm="$target_dir/purrdf_hnsw_determinism.simd128.wasm"
cp "$(build_module -u RUSTFLAGS CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="$simd_flags")" "$simd_wasm"

if cmp -s "$wasm" "$simd_wasm"; then
	fail "the +simd128 module is byte-identical to the baseline one; the target feature did not reach the build"
fi

# One module's digest and corpus length, as `digest corpus_len`.
run_module() {
	local output digest corpus
	output="$(node scripts/hnsw-determinism.mjs "$1")"
	digest="$(printf '%s\n' "$output" | sed -n 's/^digest=//p')"
	corpus="$(printf '%s\n' "$output" | sed -n 's/^corpus_len=//p')"
	[ -n "$digest" ] || fail "the wasm module $1 printed no digest"
	[ -n "$corpus" ] || fail "the wasm module $1 printed no corpus length"
	printf '%s %s\n' "$digest" "$corpus"
}

read -r wasm_digest wasm_corpus <<<"$(run_module "$wasm")"
read -r simd_digest simd_corpus <<<"$(run_module "$simd_wasm")"

# ---------------------------------------------------------------------------
# 3. Compare. All of them, plus a non-vacuity check on the corpus.
# ---------------------------------------------------------------------------
echo "wasm32          digest=$wasm_digest corpus_len=$wasm_corpus"
echo "wasm32+simd128  digest=$simd_digest corpus_len=$simd_corpus"
echo "golden          digest=$golden corpus_len=$expected_corpus"

for corpus in "$wasm_corpus" "$simd_corpus"; do
	[ "$corpus" = "$expected_corpus" ] ||
		fail "a wasm digest folded $corpus rows, but the corpus has $expected_corpus"
done

for pair in "wasm32:$wasm_digest" "wasm32+simd128:$simd_digest"; do
	build="${pair%%:*}"
	digest="${pair#*:}"
	if [ "$digest" != "$golden" ]; then
		fail "$build AND THE NATIVE GOLDEN DISAGREE: $digest vs $golden.
  purrdf-hnsw's determinism claim is that these are equal by construction, so a
  difference is a real defect, not a tolerance to widen. Look for a float
  operation that is not associative across targets or builds, a usize-width
  assumption, or iteration over a hash map reaching an output."
	fi
done

echo "OK: the wasm32 and wasm32+simd128 digests are identical to the natively pinned golden"
