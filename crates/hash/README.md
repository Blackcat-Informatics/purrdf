<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-hash` — Zero-Dependency BLAKE3, MD5, SHA-1, SHA-3, CRC-32 and a Fixed-Key Table Hasher

[![crates.io](https://img.shields.io/crates/v/purrdf-hash.svg)](https://crates.io/crates/purrdf-hash)
[![docs.rs](https://docs.rs/purrdf-hash/badge.svg)](https://docs.rs/purrdf-hash)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-hash` is the PurRDF toolkit's digest leaf: the SPARQL `MD5()`,
`SHA1()` and `SHA3-*()` built-ins, OpenPGP v4 key fingerprints and Datalog
derivation identities all compute through it, and `fixed::FixedHasher` is
the workspace's in-memory table hasher. It has **no runtime dependencies**,
allocates nothing, and builds for `wasm32-unknown-unknown`.

| Module | Algorithm | Specification | Output |
|---|---|---|---|
| `blake3` | Unkeyed BLAKE3-256 | BLAKE3 §2.1–2.5 | 32 bytes |
| `md5` | MD5 | RFC 1321 | 16 bytes |
| `sha1` | SHA-1 | FIPS 180-4 | 20 bytes |
| `sha3` | SHA3-224 / 256 / 384 / 512, Keccak-f[1600] | FIPS 202 | 28 / 32 / 48 / 64 bytes |
| `crc32` | CRC-32/ISO-HDLC | reflected `0xEDB88320`, init and xorout `0xFFFFFFFF` | `u32` |

`frame` is the workspace's length framing: `frame_le` appends a field as its
length in eight little-endian bytes followed by its bytes, `frame_le_into`
streams exactly those bytes into any `Digest`, and `frame_be_labelled` is the
big-endian label-and-value framing three published identities were minted
with. Every preimage and wire encoding in the workspace frames through it, and
`purrdf-hash-conformance` replays its frozen answers on every target.

## SHA-2 is the `sha2` crate

`purrdf-hash` implements no SHA-2. The workspace's one SHA-2 implementation is
the external `sha2` crate, which the crates that need it depend on directly:
the SHA-256 content identities and query provenance, and `purrdf-ed25519`'s
SHA-512. A native SHA-256 (x86 SHA extensions and portable) and SHA-384/512
were measured against `sha2` on the backends this workspace selects and were
not faster in every case — SHA-256 on the SHA extensions was 3.7% faster at
64 B, 1.4% slower at 1 KiB and tied at 1 MiB, and portable SHA-512 was 43–47%
slower at every size — so `sha2` stays, and `purrdf-hash` keeps its zero
runtime dependencies.

## Shared kernels

Beside the digests, the crate is the home of the small specified kernels every
other crate uses, each the workspace's one implementation (`helpers-ledger.toml`
names the job, and `purrdf-hash-conformance` replays their frozen vectors
natively and on wasm32):

| Module | What it holds |
|---|---|
| `hex` | Base16 (RFC 4648 §8) in either case: `Lower`/`Upper` (`Display`), `encode`, `encode_into`, `encode_to_slice` and their uppercase siblings; `decode` (the `xsd:hexBinary` lexical space), `decode_canonical`, `decode_32`, `decode_32_canonical`, `nibble`, `parse_u32`; and `Digest32`, the 32-byte digest value every content identity wraps |
| `frame` | Length framing: `frame_le`, `frame_le_into`, `frame_be_labelled` |
| `Domain` (crate root) | The registered hash domain-separation string (see [Hash domains](#hash-domains)) |
| `fnv` | FNV-1a 64-bit: `BASIS`, `PRIME`, `fnv1a64`, and `fold` into a caller's state |
| `mix` | SplitMix64 (`splitmix64_next`, the published counter stream; `splitmix64_step`, the self-composed stream; `splitmix64_finalize`; `GOLDEN_GAMMA`); the signed-unit draws that map a 64-bit draw onto an exact binary64 in `[-1, 1)` (`signed_unit`, `signed_unit_next`, `signed_unit_step` and their `_nonzero` forms); and the 64-bit linear congruential generator with Knuth's MMIX multiplier (`lcg64_next`, `LCG64_MULTIPLIER`, `LCG64_MMIX_INCREMENT`) |
| `dispatch` | The `Backend` trait every family of named execution paths implements (`selected`, `is_available`, `all_available`, `name`) and `PURRDF_REQUIRE_SIMD_PATHS`, the one variable a test run sets to require paths: `1` for every path the host is expected to run, or a `family:path` list |

## Usage

```rust
use purrdf_hash::md5::Md5;
use purrdf_hash::sha1::Sha1;
use purrdf_hash::sha3::Sha3_256;
use purrdf_hash::crc32::Crc32;

// One shot.
let digest: [u8; 32] = Sha3_256::digest(b"abc");
assert_eq!(digest.len(), 32);
let md5: [u8; 16] = Md5::digest(b"abc");
assert_eq!(md5[..4], [0x90, 0x01, 0x50, 0x98]);
assert_eq!(Crc32::checksum(b"123456789"), 0xCBF4_3926);

// Streaming: any split of the input gives the same answer.
let mut sha1 = Sha1::new();
sha1.update(b"ab");
sha1.update(b"c");
assert_eq!(sha1.finalize(), Sha1::digest(b"abc"));
```

Every hasher also implements the object-safe `Digest` trait, so an algorithm
chosen at run time can be driven through `&mut dyn Digest`.

## Execution paths

| Algorithm | Paths | Selected when |
|---|---|---|
| BLAKE3 | SSE2 / SSSE3 / AVX2 / AVX-512, NEON, wasm SIMD128, portable | See the capability table below; streaming and one-shot use the same kernels |
| SHA-1 | x86 SHA extensions (`sha1rnds4`, `sha1nexte`, `sha1msg1/2`) | x86-64 with `sha`, `ssse3`, `sse4.1` |
| | Armv8 SHA1 instructions (`sha1c/p/m`, `sha1h`, `sha1su0/1`) | AArch64 with SHA1 |
| | portable | otherwise |
| CRC-32 | `pclmulqdq` folding, 64 bytes per step, Barrett reduction | x86-64 with `pclmulqdq`, `sse4.1` |
| | Armv8 CRC32 instructions | AArch64 with CRC32 |
| | Armv8 `pmull` folding finished by CRC32 | never (available to tests and the bench; not yet measured on Arm hardware) |
| | slicing-by-16 tables | otherwise |
| MD5 | portable (a strict serial dependency chain) | always |
| SHA-3 | portable Keccak-f[1600] | always |

Paths are chosen by run-time CPU detection, and every path computes the same
bytes: the test suite replays 14,097 frozen answers per algorithm against
each path the host can run, natively and on wasm32. Every constant — MD5's
sine table, SHA-3's round constants, rotation offsets and lane permutation,
the CRC tables and the carry-less folding and Barrett constants — is computed
at compile time from its definition rather than typed.

All `unsafe` code lives in one private module (the processor kernels, their
detection, their vector loads and the table hasher's AES block); the rest of
the crate is
`#![deny(unsafe_code)]`.

MD5 and SHA-1 are here because protocols name them, not as security
primitives: both are broken for collision resistance.

## SHA-3: expose the fixed permutation to the compiler

Keccak-f[1600] has 25 lanes of 64 bits and exactly 24 rounds. The implementation
fuses the rho rotation and pi permutation: each destination takes a fixed
source lane, applies its theta correction, and rotates by a fixed amount.
Expanding those 25 lane expressions makes both indices and rotation counts
compile-time constants, allowing immediate rotations and removing indexed
schedule loads. The constants are still generated from the FIPS 202 equations.

Two rounds are expanded per loop iteration to expose instruction scheduling
across round boundaries. Expanding all 24 rounds increased code size and
performed worse in the measured x86 workloads. This changes instruction
scheduling only: every specified round and bit operation remains present.
All four SHA-3 output widths replay independent frozen differential vectors.

## BLAKE3: mathematics and implementation

This module implements the unkeyed 256-bit mode from the
[BLAKE3 specification, §2](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf).
All paths retain the specified **seven rounds**, 1,024-byte chunks, tree
shape, counters and domain flags. Optimizations change how operations are
scheduled and data is buffered; they never change the digest function.
[PROVENANCE.md](PROVENANCE.md) records the specification and oracle sources.

### Compression and message schedule

A compression state has sixteen 32-bit words. A quarter round takes four
state words `(a, b, c, d)` and two message words `(x, y)` and applies:

```text
a = a + b + x        d = rotr(d XOR a, 16)
c = c + d            b = rotr(b XOR c, 12)
a = a + b + y        d = rotr(d XOR a,  8)
c = c + d            b = rotr(b XOR c,  7)
```

Every addition is modulo `2^32`. Four independent column quarter rounds
precede four diagonal quarter rounds. Between rounds, the sixteen message
words follow the permutation
`[2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]`.
The seven schedules are composed at compile time. Explicit round calls let
the compiler resolve message indices without a dynamic scheduling loop.
The first eight output words are `v[i] XOR v[i+8]`.

The default x86 short-message and stream-finalization paths use scalar ARX.
Four independent scalar quarter-round chains let the processor schedule work
without the vector single-block dependency chain. On x86-64, a small inline
assembly helper pins rotation to the two-operand `ROR` instruction. In the
measured native-CPU build, LLVM's BMI2 `RORX` choice and register allocation
nearly doubled the 1 KiB path's latency; retaining the destination register
removed that regression. The helper declares its flag clobber and has no memory
or stack effects. Native tests compare all four rotations with Rust integer
arithmetic; other architectures and Miri use `rotate_right` directly.


On x86, a single-block kernel places the four independent quarter rounds
in four SIMD lanes. Lane shuffles turn columns into diagonals and back.
SSSE3 implements the 8- and 16-bit rotations with byte shuffles, and the
others with shifts and OR. The AVX-512F/VL path uses native lane rotations
and selects message words with vector permutations. These are distinct
schedules of the same arithmetic, checked against the portable compressor.

### Independent chunks and SIMD tree reduction

Blocks within a chunk form a serial chain. Different chunks are independent:
chunk `j` uses counter `j`, starts with the specified IV, and sets
`CHUNK_START` and `CHUNK_END` at its first and last block. A parent hashes
its two 32-byte child chaining values with counter zero and `PARENT` set.
Only the final compression sets `ROOT`. A root digest is consequently not
interchangeable with an intermediate chaining value.

The AVX-512F batch kernel puts the same state word from sixteen chunks into
one vector. Contiguous block loads are transposed using 32-bit and 64-bit
interleaves followed by 128-bit lane shuffles. This avoids sixteen strided
gathers for every message block. Each vector arithmetic instruction then
advances sixteen independent chunk computations.

Parent reductions remain in vector registers. Even and odd child lanes
become the left and right halves of each parent message. The live lane count
contracts through the tree. For an incomplete batch, completed chunk lanes
are preserved while longer chunks finish; an unmatched child is carried to
the next level. This preserves BLAKE3's required tree: each left subtree is
complete, has a power-of-two number of chunks, and is at least as large as
its right sibling. Final child chaining values are retained so `ROOT` is
applied exactly once, rather than trying to recover a root from a finalized
non-root chaining value.

Full AVX-512 batches additionally collect up to 256 chunk chaining values
before reducing parents. Sixteen parent lanes can then combine children
across neighbouring batches, avoiding repeated reductions with mostly idle
lanes. The bounded CV workspace is 8 KiB; the narrower paths retain their
64-chunk workspace.

A complete four-chunk AVX-512 batch uses a different register basis: each
512-bit state register holds four quarter-round lanes for each of four chunks.
The message rows are `[0,2,4,6]`, `[1,3,5,7]`, `[14,8,10,12]` and
`[15,9,11,13]` within each chunk. Conjugating the specification's permutation
into this basis gives seven vector permutation operations between rounds.
The `b` row finishes last in each quarter round. Leaving it stationary at the
column/diagonal boundary lets the other three row shuffles overlap its final
operations. Counters and block lengths are built once; only the START/END lanes
change between blocks. A pair of parent nodes uses the corresponding 256-bit
layout. A complete-subtree entry keeps this path separate from partial-input
padding, avoiding a 4 KiB stack frame observed in the combined caller's assembly.
These are scheduling and storage changes; all seven specified rounds remain.

The narrower kernels share their ARX round schedule, length flags and
chunk handling. AVX-512VL reuses the four-lane source for partial batches through
4 KiB and for four-parent reductions. SSE2 and NEON use four independent chunk lanes; AVX2 uses
eight; wasm SIMD uses four. Full batches read directly from caller memory,
with uniform block lengths and flags: they avoid both a padded copy and per-lane tail masks. Partial
batches retain padding and masks to preserve exact final-chunk semantics.
Their contiguous loads are transposed with 32-bit and 64-bit interleaves and
lane shuffles. Up to 64 chunk chaining values are reduced together; an
eight-lane parent level switches to four lanes before individual parent
compressions become necessary. NEON and wasm SIMD also
vectorize the four quarter rounds within a single block, so short identities
benefit without needing multiple chunks.

| Target capability | Chunk lanes | Single-block path |
|---|---:|---|
| x86 SSE2 (runtime detected on 32-bit; baseline on x86-64) | 4 | SSE2 shifts, OR and lane shuffles |
| x86-64 SSSE3 | 4 | Byte-shuffle rotations |
| x86-64 AVX2 | 8 | AVX2 message permutations and byte-shuffle rotations |
| x86-64 AVX-512F/VL | 4 or 16 | Native lane rotations and vector permutations |
| Little-endian AArch64 NEON | 4 | NEON quarter rounds |
| wasm32 SIMD128 | 4 | SIMD quarter rounds |
| Other targets, including baseline wasm32 | 1 | Portable 32-bit ARX |

The portable compressor uses 32-bit wrapping addition, XOR and rotation;
it does not require 64-bit multiplication or vector instructions. This is
also the independent arithmetic reference for the hardware kernels.
Runtime feature checks guard the x86 kernels. NEON and wasm SIMD are
compiled only when enabled for those targets. SIMD width and CPU availability
never affect bytes.
Default x86 hashes through 1 KiB use scalar ARX: on the measured processor,
four independent scalar quarter-round chains schedule faster than the single
vector chain of within-block SIMD. Default streams use that same short-input
path. Backend-forcing APIs retain the named single-block kernels for tests
and measurement; the choice for other architectures remains their native path.
The leaf crate creates no threads. `hash_with_join` accepts the caller's
`Join` implementation and minimum input size for scheduling; below that
threshold it runs the ordinary SIMD path. The crate still owns the canonical
split, chunk counters and ROOT flag. GTS implements this seam with its shared
Rayon pool, while a threadless caller can run both closures sequentially.
The scheduler and threshold affect performance only.

### Streaming is a first-class path

```rust
use purrdf_hash::blake3::{hash, Hasher, RecordHasher};

let mut stream = Hasher::new();
stream.update(b"ab");
stream.update(b"");
stream.update(b"c");
assert_eq!(stream.finalize(), hash(b"abc"));

// Finalization is a snapshot: appending remains valid.
stream.update(b"d");
assert_eq!(stream.finalize(), hash(b"abcd"));
stream.reset(); // Reuses storage; this is not secure erasure.
assert_eq!(stream.finalize(), hash(b""));

// The same function with smaller state for short, framed identities.
let mut record = RecordHasher::new();
record.update(b"abc");
assert_eq!(record.finalize(), hash(b"abc"));
```

`Hasher` accumulates small writes into a 16 KiB batch so they can fill sixteen
parallel chunks. Large updates are processed directly from the input slice.
If an update ends at a complete aligned subtree, its final compression inputs
are retained as a pending `Output`; its message bytes need no copy into the
buffer. Finalization adds ROOT, while a later nonempty update converts that
output to a chaining value before appending. This also avoids repeatedly
splitting a 16 KiB write into 8-, 4-, 2- and 1-chunk subtrees.

`RecordHasher` uses the same streaming engine with a 1 KiB buffer. Its lower
initialization and stack cost suit short identities; it does not batch small
writes into sixteen chunks. Both expose `update`, non-consuming `finalize`,
`reset`, cloning, the common `Digest` trait, and `std::io::Write`. Encoders
can write directly into either hasher without a digest-specific writer adapter.
`flush` preserves the message; a digest snapshot does not require flushing. Reset changes counters and
valid lengths without clearing storage, making reuse across records cheap.
Buffer capacity is an execution choice, not a digest or wire-format choice.

The streaming tree stores at most 54 pending subtree chaining values for
inputs shorter than `2^64` bytes. A completed aligned subtree is merged while
its count has trailing zero bits, the same carry operation as binary
addition. Storage is bounded independently of input length: a fixed input
buffer and `54 * 32` bytes for tree chaining values, plus a pending output and counters. Large updates
consume the largest complete subtree aligned to the current chunk counter
directly from caller memory, retaining a partial suffix or final compression inputs for root finalization.
This batches parent reductions without enlarging the streaming buffer. No whole
message buffering or heap allocation is required. Input-length overflow is
rejected before processing the update.

### Evidence and performance boundaries

The frozen oracle contains **18,214** public-API answers from `blake3` 1.8.5:
8,214 patterned inputs cover every length through 8,192 bytes and
power-of-two boundaries through 1 MiB; another 10,000 seeded random inputs
cover irregular trees through 64 KiB, with starting offsets from 0 to 31.
Both streaming buffer sizes replay it with irregular splits and empty
updates on every backend the host can execute. `backend::Blake3Backend`
allows tests and benchmarks to force a backend, so AVX-512 cannot mask an
SSE2 or AVX2 defect. Counter-carry and unaligned-input tests independently
compare the narrow kernels with scalar chunk computations. Separate checks cover snapshots, reset and cloned streams.
The native, baseline wasm and SIMD wasm runners execute the same corpus.

A paired measurement on an AMD Ryzen AI Max+ 395, compiling with
`target-cpu=native`, opt-level 3, thin LTO and one codegen unit, compared fresh
stream states over a 1 MiB message with `blake3` 1.8.5. Twelve samples alternated
execution order. The [raw samples and per-algorithm distributions](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/benchmarks/hash-comparison-2026-09-28/README.md)
include baseline and native builds and their source/compiler receipts.

| Update size | Removed crate median | PurRDF stream median |
|---|---:|---:|
| 8 bytes | 1,793.4 µs | 517.0 µs |
| 64 bytes | 1,250.3 µs | 154.9 µs |
| 1 KiB | 1,243.2 µs | 117.5 µs |
| 16 KiB | 110.5 µs | 113.4 µs |
| 64 KiB | 95.7 µs | 95.8 µs |

These measurements describe this workload and host, not a universal speedup
or an ISA throughput guarantee. In particular, batching benefits tiny writes
while large writes and fresh short messages have different costs.
`benches/digests.rs` measures one-shot hashing and both stream capacities,
with fresh and reused states, across message lengths and update sizes. The
`blake3-backends` group forces each available backend for one-shot and
streaming measurements. For an older x86 instruction baseline, compile with
`RUSTFLAGS="-C target-cpu=x86-64"`; forcing SSE2 in a native-CPU build does
not constrain the surrounding compiler-generated code to SSE2. These runs
still measure the host microarchitecture, not an older processor:

```sh
cargo bench -p purrdf-hash-conformance --bench digests -- blake3
cargo bench -p purrdf-hash-conformance --bench digests -- blake3-backends
cargo test -p purrdf-hash-conformance --test blake3
```

## Fixed-key table hasher

`fixed::FixedHasher` is a `Hasher`, and `fixed::FixedState` is the
`BuildHasher` for `HashMap<K, V, FixedState>`. The keys are compile-time
constants derived from `⌊2^64/φ⌋`, with no run-time seeding, so a build
hashes equal inputs equally on every run.

The RDF term interner uses `FixedHasher::hash_terminal(tag, bytes)` when a
tagged byte slice is the whole key. This avoids streaming state updates for
ordinary IRIs while preserving a separate generic `Hasher` path for compound
terms. Borrowed lookup, stored-term rehash and frozen lookup must all call the
same terminal function. Its result is an in-memory table hash only.

```rust
use core::hash::BuildHasher;
use std::collections::HashMap;
use purrdf_hash::fixed::FixedState;

let mut map: HashMap<&str, u32, FixedState> = HashMap::with_hasher(FixedState::new());
map.insert("http://example.org/p", 1);
assert_eq!(FixedState::new().hash_one(7u32), FixedState::new().hash_one(7u32));
```

| Input | Portable build | AES build |
|---|---|---|
| Integers | Folded multiply into a 64-bit state | One AES round into a 128-bit state |
| 0–16 byte slices | Packed words, two independently seeded folds and a length term | Packed words and length domain, one AES absorption |
| 17–32 byte slices | Two folds plus rotated tail words | Sequential first/last 16-byte absorption |
| Longer slices | Four independent multiply lanes | Four independent AES lanes, then absorption |
| Streaming finalization | One folded multiply | Two AES rounds, then low 64 bits |

For an AES integer update, `S = R(S XOR block, K)`. Finalization is
`low64(R(R(S, F0), F1))`, where `R` is the hardware AES encryption round.
Thus the final integer passes through three rounds before truncation. Two
rounds gave good average avalanche but failed structured high-bit collision
tests. The 17–32-byte path absorbs the first and last blocks sequentially,
mixing the byte length into the state before them. Merging one-round lanes
by addition also failed sparse-key tests and is not used. Longer independent
lanes retain their extra diffusion before merging. The terminal IRI operation
uses the sequential AES path at 17–32 bytes and its separately tested folded
finalizer at other lengths.

For a portable two-word update, let `F(x, k)` XOR the low and high halves
of the 128-bit product `x * k`. The update is
`S = F(S XOR first, K_A) XOR F(second XOR PAIR_X, K_B) XOR length_term`.
The second lane has its own dense offset from the same fixed key schedule.
Leaving that lane unseeded permits a structured cancellation:
`F(0, k) = 0` and `F(2^64 - 1, k) = 2^64 - 1` for every nonzero `k`.
An all-ones high word can then cancel an all-ones word in the next field.
Seeding both lanes removes that zero/all-ones special case while retaining
two independent multiplies; constant metadata can still be folded at compile
time. Successive folds also passed the quality tests, but their dependency
chain was slower for packed triple keys. The regression tests exercise wide
integer fields and byte fields on the portable implementation even when the
host selects AES. The public offsets do not provide resistance to deliberately
constructed collisions.

RDF blank nodes, short literals and embedded triples pack their typed fields
into one or two integer blocks. The metadata includes lengths, scope or full
64-bit datatype identity, language presence and text direction. Packed triple
IDs retain all 64 bits on the global path. `pack_short_bytes` uses fixed-size
loads and shifts; variable-length copies were measurably more expensive.
Borrowed, stored, global and frozen lookups share the same protocol.

AES selection is made at compile time. On 32-bit targets (i686, wasm32), the
portable 128-bit product is built from 32-bit multiplies with identical output.
Both functions are pinned by their own frozen self-vectors. The portable vectors
record the seeded-lane law; the AES vectors also pin its short-terminal fallback. Avalanche,
collision and χ² distribution requirements are the same for both paths.
Hardware width and instruction availability do not establish throughput: compare
the concrete caller protocols and complete workloads using the maintained
comparison scripts described in `docs/design/purrdf-simd.md`.

Because the function depends on the build, a table hash must never be
persisted, sent over the wire or used as a content address. Use a digest
for any of those. The keys are public, so the hasher offers no resistance
to deliberately colliding input.

## Hash domains

A hash domain is the byte string that names one preimage family: it opens the
preimage (or is its named `domain` part), so a digest of one structure can never
be read as a digest of another. Every domain in the workspace is a
`purrdf_hash::Domain` constant registered next to the construction it separates:

```rust
use purrdf_hash::{Domain, blake3};

const RECORD_DOMAIN: Domain = Domain::new(b"purrdf-example/record/v1");

let mut hasher = blake3::Hasher::new();
hasher.update(RECORD_DOMAIN.as_bytes());
hasher.update(b"payload");
let _digest = hasher.finalize();
```

The contract, held by `scripts/check-hash-domains.py` on every `make check`:

* **Unique and prefix-free.** No two registered domains are equal, and none is
  a byte-prefix of another.
* **Spelt through `Domain`.** A domain-shaped literal (one holding a NUL, a
  version token such as `-v1`, `/v2` or `.v1`, or the bytes of a registered
  domain) handed straight to a hasher, or declared as a string domain constant,
  fails the gate.
* **One family per domain.** A registered domain that opens hashers in two
  functions of its file fails the gate, unless a `hash-domain` variant row in
  `helpers-ledger.toml` declares it with a documented reason. A published
  identity digest computed with no domain at all is recorded the same way.
* **Never renamed.** A domain's bytes are part of every digest computed under
  it, and those digests are published: content addresses, cache keys, proof and
  artifact identities, golden files. The spellings below are frozen, whatever
  convention they follow. A changed preimage layout gets a new domain; the old
  spelling is never reused for a different layout.

**Convention for a new domain: `purrdf-<crate>/<purpose>/v<N>`**, for example
`purrdf-text/index/v2`. Today's spellings vary: NUL-terminated dotted paths,
hyphenated, dotted, spaced, slashed and colon-separated versions, unversioned
slashed paths, colon-prefixed tags and bare words. The slashed form
`purrdf-<crate>/…` is the most widely used namespaced spelling: the
`purrdf-sparql-eval`, `purrdf-hnsw`, `purrdf-geo`, `purrdf-text` and
`purrdf-shapes` domains all open with it. It names the owning crate, so two
crates can never coin the same domain; it keeps the purpose, which is itself
hyphenated, unambiguous; and it puts the version last, where a layout change
bumps it. A version is a decimal without leading zeros. The gate refuses a pair
that would prefix each other (`…/v1` beside `…/v10`), so a family that keeps two
versions registered at once never spells one as the other's prefix.

The registry: every registered domain, generated by
`python3 scripts/check-hash-domains.py --write-readme` and checked against the
source on every run.

<!-- hash-domain-registry:begin (generated by scripts/check-hash-domains.py --write-readme) -->

| Domain bytes | Constant | Defined in |
|---|---|---|
| `b"assertion"` | `ASSERTION_KIND` | `crates/gts/src/examples/agent_memory.rs` |
| `b"citation"` | `CITATION_KIND` | `crates/markdown/src/identity.rs` |
| `b"gts-mmr-leaf-v1"` | `LEAF_DOMAIN` | `crates/gts/src/mmr.rs` |
| `b"gts-mmr-parent-v1"` | `PARENT_DOMAIN` | `crates/gts/src/mmr.rs` |
| `b"gts-mmr-root-v1"` | `ROOT_DOMAIN` | `crates/gts/src/mmr.rs` |
| `b"gts-segment-heads-v1"` | `SEGMENT_HEADS_DOMAIN` | `crates/gts/src/replication.rs` |
| `b"merkle-root\x1f"` | `MERKLE_ROOT_DOMAIN` | `crates/slice/src/cache.rs` |
| `b"path-snapshot-edge-set-v1"` | `PATH_SNAPSHOT_DOMAIN_V1` | `crates/sparql-eval/src/path_relation.rs` |
| `b"path-witness-identifier-v1"` | `PATH_ID_DOMAIN_V1` | `crates/sparql-eval/src/path_relation.rs` |
| `b"phase:bundle"` | `BUNDLE_PHASE` | `crates/slice/src/cache.rs` |
| `b"phase:parse"` | `PARSE_PHASE` | `crates/slice/src/cache.rs` |
| `b"phase:reason"` | `REASON_PHASE` | `crates/slice/src/cache.rs` |
| `b"phase:shacl"` | `SHACL_PHASE` | `crates/slice/src/cache.rs` |
| `b"phase:syntax"` | `SYNTAX_PHASE` | `crates/slice/src/cache.rs` |
| `b"purrdf-datalog restricted chase witness v1"` | `WITNESS_DIGEST_TAG` | `crates/datalog/src/chase.rs` |
| `b"purrdf-datalog-contract-v1"` | `CONTRACT_DIGEST_TAG` | `crates/datalog/src/cache.rs` |
| `b"purrdf-datalog-dl-clause-ir-v3"` | `CLAUSE_IR_DIGEST_TAG` | `crates/datalog/src/cache.rs` |
| `b"purrdf-datalog-guarded-dl-clause-ir-v1"` | `GUARDED_CLAUSE_IR_DIGEST_TAG` | `crates/datalog/src/cache.rs` |
| `b"purrdf-datalog-plan-identity-v1"` | `PLAN_IDENTITY_TAG` | `crates/datalog/src/cache.rs` |
| `b"purrdf-datalog-proof-v1"` | `PROOF_ENCODING_TAG` | `crates/datalog/src/proof.rs` |
| `b"purrdf-datalog-scheduled-contract-v1"` | `SCHEDULED_CONTRACT_DIGEST_TAG` | `crates/datalog/src/cache.rs` |
| `b"purrdf-dl-service-proof-v2"` | `SERVICE_ENCODING_TAG` | `crates/entail/src/reasoner/proof.rs` |
| `b"purrdf-geo/index-source/v1"` | `DIGEST_DOMAIN` | `crates/geo/src/relation.rs` |
| `b"purrdf-hnsw/space-generation-v1"` | `SPACE_GENERATION_DOMAIN` | `crates/hnsw/src/relation.rs` |
| `b"purrdf-json-document-v1"` | `DOCUMENT_DOMAIN` | `crates/json/src/profile.rs` |
| `b"purrdf-json-profile-v1"` | `PROFILE_DOMAIN` | `crates/json/src/profile.rs` |
| `b"purrdf-owl-dl-contract-v1"` | `CONTRACT_DIGEST_TAG` | `crates/entail/src/owl_dl/proof.rs` |
| `b"purrdf-owl-dl-proof-v3"` | `PROOF_ENCODING_TAG` | `crates/entail/src/owl_dl/proof.rs` |
| `b"purrdf-shapes/product/class-catalog"` | `CLASS_CATALOG_DOMAIN` | `crates/shapes/src/product/identity.rs` |
| `b"purrdf-shapes/schema-compilation-key/v1"` | `SCHEMA_KEY_SALT` | `crates/shapes/src/json_schema.rs` |
| `b"purrdf-sparql-eval/aggregate-registry"` | `CONTENT_DOMAIN` | `crates/sparql-eval/src/agg_fn.rs` |
| `b"purrdf-sparql-eval/embedding-space-generation/v1"` | `SPACE_GENERATION_DOMAIN` | `crates/sparql-eval/src/knn/mod.rs` |
| `b"purrdf-sparql-eval/embedding-space-vectors-generation/v1"` | `VECTORS_GENERATION_DOMAIN` | `crates/sparql-eval/src/knn/mod.rs` |
| `b"purrdf-sparql-eval/extension-env"` | `CONTENT_DOMAIN` | `crates/sparql-eval/src/extension_env.rs` |
| `b"purrdf-sparql-eval/property-function-registry"` | `CONTENT_DOMAIN` | `crates/sparql-eval/src/property_fn_plan.rs` |
| `b"purrdf-sparql-eval/user-function-registry"` | `CONTENT_DOMAIN` | `crates/sparql-eval/src/user_fn.rs` |
| `b"purrdf-text/index/v2"` | `INDEX_DIGEST_DOMAIN` | `crates/text/src/index.rs` |
| `b"purrdf-text/source/v1"` | `SOURCE_DIGEST_DOMAIN` | `crates/text/src/index.rs` |
| `b"purrdf.pipeline-root.v1"` | `PIPELINE_ROOT_DOMAIN` | `crates/rdf-core/src/ir/pipeline_bundle.rs` |
| `b"purrdf.purremb.v1.artifact\0"` | `D_ARTIFACT` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.chunking\0"` | `D_CHUNKING` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.external-binding\0"` | `D_EXTERNAL` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.external-contract\0"` | `D_EXTERNAL_CONTRACT` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.family\0"` | `D_FAMILY` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.family-contract\0"` | `D_FAMILY_CONTRACT` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.index\0"` | `D_INDEX` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.index-guard\0"` | `D_INDEX_GUARD` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.matrix\0"` | `D_MATRIX` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.matrix-content\0"` | `D_MATRIX_CONTENT` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.projection\0"` | `D_PROJECTION` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.projection-content\0"` | `D_PROJECTION_CONTENT` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.relation-role\0"` | `D_RELATION_ROLE` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.target\0"` | `D_TARGET` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.target-identity\0"` | `D_TARGET_IDENTITY` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.target-set\0"` | `D_TARGET_SET` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf.purremb.v1.vector-space\0"` | `D_SPACE` | `crates/rdf-core/src/ir/embedding/identity.rs` |
| `b"purrdf:evidence:v1"` | `EVIDENCE_ID_DOMAIN` | `crates/retrieval/src/id.rs` |
| `b"purrdf:fusion-profile:v1"` | `FUSION_PROFILE_ID_DOMAIN` | `crates/retrieval/src/id.rs` |
| `b"purrdf:plan:v1"` | `PLAN_ID_DOMAIN` | `crates/retrieval/src/id.rs` |
| `b"section"` | `SECTION_KIND` | `crates/markdown/src/identity.rs` |
| `b"structure"` | `STRUCTURE_KIND` | `crates/markdown/src/identity.rs` |
| `b"toolcall"` | `TOOLCALL_KIND` | `crates/gts/src/examples/agent_memory.rs` |
| `b"unit"` | `UNIT_KIND` | `crates/markdown/src/identity.rs` |

<!-- hash-domain-registry:end -->

## License

`MIT OR Apache-2.0 OR MulanPSL-2.0`.
