# BLAKE3 modes and extendable output

Complete keyed hashing, two-stage context derivation, extended output and their
streaming forms without adding dependencies or a second compression algorithm.
The existing unkeyed tree, short-input policy and hardware dispatch remain the
production path for every mode. Repository law requires portable deterministic
bytes, one implementation per job, normal hooks and protected integration.

## Task 1: Implement and qualify the complete modes

Thread one key/flags value through existing chunks, parents and all batched
kernels. Extend each existing single-block kernel to emit either eight chaining
words or all sixteen output words, specializing away unused output operations.
Expose keyed_hash, derive_key, hash_xof and keyed/derived XOF functions; streaming
constructors preserve mode across reset. An owned seekable output reader retains
root inputs and fills caller storage without allocation or rehashing input.

Freeze the official full-length vectors with provenance. Replay every input in
all three modes on every available backend, both stream buffer sizes, unrelated
update boundaries, reset/clone and seek/read boundaries. Check the first 32 bytes
against digest APIs and megabyte output against independent block/partition
controls. Retain existing frozen unkeyed and tree-counter controls. Execute native
and installed portable/SIMD WASM paths, and applicable architecture checks. No
elapsed-time or throughput measurements. Run strict affected compilation and
hygiene during development; run the required final gate on settled source.

## Task 2: Publish and integrate

Publish the implemented behavior after focused checks and normal hooks, while
assembly projections finish. This starts hosted qualification promptly; it does
not waive any merge requirement. Reuse the completed same-head seven-lane CI
measurements: the existing document writer regenerates the five stale cells,
and its existing parity/coverage checks validate the resulting projection.
The duplicate local matrix is interrupted, not counted as passed. No kernel
threshold, tool, compiler flag, report verdict or acceptance criterion changes.
Normal hooks and the final hosted matrix must cover the resulting candidate
before integration; reuse the real review for unchanged Rust and its requested
documentation correction.

Review the complete issue against source and recorded results; resolve material
findings. Commit through normal hooks, push and open the closing PR. Resolve
actual hosted review and CI, merge only through ghprsq, verify closure/mirror and
remove the delivered branch and worktree. Update the portfolio tracker.

## Review and prior art

The existing original compression and SIMD batch implementations are the only
algorithm homes; official specification and frozen answers provide the oracle,
not upstream implementation bodies. Brief found no prior attempt, no comments
and no linked PR. Its narrow title search does not establish global recurrence.
No subagents are permitted. Owner analysis is not independent approval; hosted
source review will provide independent review before integration. Generic pool
or external consumer migrations would expand scope and are excluded. Seekable
output is included because it avoids rehashing megabyte masks and fits the same
retained root object. No new dependency, timing gate or fallback kernel is added.
