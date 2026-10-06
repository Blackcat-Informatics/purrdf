<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Read sessions and persistent snapshots

`DatasetView` has one term-access contract for resident and operational storage.
Forward lookup returns a typed `Result` containing a GAT term guard. Borrow text
through `TermGuard::term()` while that guard lives. Reverse and visitor-based
batch lookup preserve the same typed failure. Resident `RdfDataset` guards are
borrowed `TermRef` values with `Infallible` errors; local `TermId` and quad rows
remain four and sixteen bytes. Resident inherent conveniences are retained.

Operational row iterators stop after failure and retain the first typed cause.
Use `checked_read` around a complete consumer drain; publish its output only
when the final checkpoint succeeds. A failed read is distinct from an absent
term, malformed RDF, or a rejected SPARQL expression. `FallibleDatasetView`
retains richer operation evidence for engines that need complete-result receipts.

Logical global term IDs, page IDs, byte positions, row hints and global counts
use `u64`. Convert to `usize` only after a bounded local span has been admitted.
`SegmentedHandle` encodes a snapshot identity and exact term ordinal in forty
bytes. Decode and call `SegmentedSession::attach` before using the compact ID;
IDs from another generation must never be reinterpreted.

## Segmented representation v1

`SegmentedBuilder` is a bounded resident constructor, not an external ingestion
engine. It uses the ordinary validated RDF interner. IDs returned by
`intern_batch` remain stable through sealing and reopening. An optional logical
first ordinal exercises dictionaries above 32-bit and JavaScript's exact Number
range without growing local arrays to that ordinal.

The `PURRSEG1` version-one header occupies the start of one fixed-width block.
Seven contiguous streams contain a forward dictionary, a sorted reverse index,
three disjoint RDF row streams, named-graph declarations, and quad-block pruning
summaries. Header ordinals/counts/addresses are little-endian `u64`. Local record
lengths and counts are checked `u32`. Dictionary records use prefix/suffix front
coding and stable global IDs; reverse records retain those same IDs. Rows carry
four `u64` IDs, with zero denoting the default graph. Summary records cover one
quad block exactly and state its row count and bounds on all four axes. Graph
blocks include declaration-only empty graphs.

A provider-backed binary authentication tree follows the data blocks. A leaf
binds its logical block position and complete bytes. Internal nodes bind both
children. Snapshot identity binds the root and layout dimensions; the header,
dictionary mapping, all RDF streams, empty graphs and index claims are thereby
part of one immutable representation identity. This identity is distinct from
canonical RDF identity, and from an eager pack/columnar projection's ID authority.

Full certification checks every block: dense stable IDs, canonical packing,
forward/reverse correspondence and strict ordering, dictionary/reference and RDF
role closure, nesting limits, disjoint unique row partitions, graph declaration
closure, truthful counts/fences/pruning summaries and expanded owned-term costs.
The v1 cost law charges 144 fixed bytes per expanded node plus its payload and
referenced children. That allowance covers the supported 32-bit and 64-bit
owned layouts; it does not depend on the certifier's pointer width.
Only successful certification returns `SegmentedReceipt`. Recomputing a hash
for false metadata does not prove any of those semantic laws. The tests forge
fences, summary bounds, references and footprint claims, recompute the entire
tree, and require certification to refuse them.

`SegmentedImage::certify` is a materializing validation convenience using the
resident IR's local construction capacity. Its allocations belong to the
construction/certification phase, outside the selective reader's live ceiling.

## Reopening and live admission

An immutable `SegmentedProvider` fills exactly one caller-owned bounded range.
The portable kernel performs no filesystem, network, clock, or signing-key I/O.
Opening reads the authenticated header and its tree proof; it does not decode
the whole dictionary or index. Each subsequently admitted block is authenticated
against the same certification root. Subject probes binary-search provider-backed
summary blocks, and reverse lookup binary-searches provider-backed term fences.
The reader allocates neither a whole block directory nor a global translation
array. Metadata and dictionary blocks share a sparse bounded cache.

A receipt's fixed 104-byte encoding can be restored only through an explicit
`SegmentedReceiptAuthority`. The host authenticates the exact certificate from
a trusted successful full-certification step. An independently retained content
pin or signed manifest is suitable; a checksum of unvalidated data is not.
Changing the provider's promised identity/length, replacing authenticated bytes,
or supplying another receipt refuses the operation.

`SegmentedReadLimits` selects live bytes, sparse cache slots, simultaneous pins,
cumulative I/O, and exact request-ledger capacity independently. The ledger never
truncates: capacity exhaustion refuses before another request or buffer growth.
Completed and refused requests retain their exact ranges and ordered digest.
Older evidence retains its exact prefix. Eviction and reloading remain visible.

Reserve before allocating. Charges include encoded blocks, decode arenas and
entries, sparse cache/index capacity, outstanding pins, shared state/evidence,
and admitted workspace. `reserve_workspace` supplies an RAII reservation for
operator scratch, batches and owned/output staging; growing an allocation needs
a successful reservation increase first. Release the allocation before releasing
its reservation. Pins remain charged through eviction and may outlive the session.
Pin-limit refusal happens before term block I/O. Host/provider and caller-owned
source buffers, arbitrary host drains, returned owned values and materializing
conveniences require their own admission; they are not secretly part of the
reader's bounded cache claim. A bounded operation reserves owned staging for its
whole lifetime. The shared engine admits a conservative supported query bound;
unpriced operational query forms return typed refusal.

`export_trig_lines` reserves its sink/traversal staging, writes through the shared
RDF term emitter, and preserves all three RDF streams and empty graph declarations.
A source or drain failure refuses completion; discard any unpublished prefix.

## Migration and qualification tool

The original eager pack v1 reader and bytes remain available.
`SegmentedImage::from_pack_v1` migrates what that validated reader carries,
including declaration-only empty graphs, which pack v1 now writes as zero-row
named partitions. A pack written before that has no such partition; restate its
declarations explicitly through `from_pack_v1_with_graphs` using a trusted sidecar
(restating a declaration the pack already carries is idempotent). Never infer
declarations from unused dictionary IRIs. The new snapshot binds the restated
declarations.

The native example prepares a deterministic fixture outside the reader budget:

```sh
cargo run -p purrdf-core --example segmented_fixture -- prepare /tmp/read-proof.bin 1000000
```

Retain the printed `trusted_receipt_pin` independently. Reopen in a fresh process
with that exact pin and a caller-selected live ceiling:

```sh
cargo run -p purrdf-core --example segmented_fixture -- read /tmp/read-proof.bin TRUSTED_RECEIPT_PIN 268435456
```

The provider uses file range reads, not a resident source image. The fixture has
one million total rows, two thousand subject/object terms, selective predicates,
and an empty graph. Quad pages are packed by block byte width; dictionary pages carry at most
128 records. The prepared one-million-row source occupies about 32 MiB.
Subject-bound query estimates count the admitted range through the existing
bounded iterator; unbound patterns retain certified page-cardinality bounds. The reader checks a full scan, an exact selective answer and
streamed export, reporting request, I/O, eviction, conservative residency and
counting-allocator evidence. It reserves enough exact evidence for the scan and
export, including tree proofs and reloads. Build binaries and prepare artifacts
outside performance measurement windows. Allocator evidence is not process RSS,
cgroup or physical-board qualification; enforce and capture those independently.
