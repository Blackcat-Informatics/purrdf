<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Segmented query storage and block sizing

`SegmentedBuildLimits` sets resident construction ceilings. Creating an empty
`SegmentedBuilder` does not reserve those ceilings; native term and row storage
grows as input arrives. Callers do not need to estimate private term expansion
to avoid a reservation for unused capacity. Offered input still obeys the
original explicit term, lexical-byte and row limits.

Dictionary blocks close before the next encoded record would exceed
`block_bytes`, or when they reach `records_per_block`. The record count is an
upper bound. The insertion-order dictionary uses front coding and stores its first
key once; routing reads that decoded record instead of a duplicate fence. The
encoder keeps raw front coding when all offered records fit. Otherwise it tries
native DEFLATE on bounded record prefixes and uses a compressed payload only
when it holds more records than the raw block. Prefix selection does not assume
that compressed size grows monotonically. The payload-length high bit marks
compression; the remaining bits give the encoded length, with zero padding to
the physical block boundary. The count and decoded-key-byte fields stay outside
the compressed payload.

The lexical-order reverse index stores only eight-byte identifiers into this dictionary.
Its blocks fill their physical eight-byte identifier slots independently of the
dictionary record-count ceiling and carry no lexical payload or decoded text
arena. Their populations and block counts may differ from dictionary blocks.
One encoded record, including its framing, must fit a block or sealing
returns `ConstructionLimit`. The encoder does not retry the complete image with
a larger layout. Selective term lookup uses authenticated stored ID ranges;
opening still loads no dictionary directory or term map. Full-population
blocks retain their direct first-probe lookup. Data profile 3 refuses older
profiles; regenerate their images from the original RDF input.

Reverse lookup compares the referenced dictionary keys through the same admitted
reader as forward lookup. A held reverse-index block may leave the payload cache
while its original reservation remains charged until the last internal owner
drops. This allows a one-slot cache to read the dictionary during lookup without
evicting an escaped lexical term guard or hiding its live bytes.

`SegmentedReadLimits` specifies ceilings, not arrays to allocate at opening.
The exact request ledger, payload cache and learned routing bounds grow as they are used. Their
concrete capacities, copied/decoded blocks, pins and query scratch share the
live-byte admission. Buffer replacement admits both the old and new layouts
until the old allocation is destroyed. A refusal reports the original typed
cause; evidence is never truncated to fit a smaller ledger.

After authentication and before inflation, the reader admits the complete framed
decode buffer and `Inflater::max_live_heap_bytes()` workspace in addition to the
physical block and decoded cache allocation. Framed output is bounded by decoded
key bytes plus sixteen bytes per record. The scratch reservation is released
before the decoded block enters the cache. Truncated streams, trailing bytes and
native codec refusals retain their typed `purrdf_deflate::Error` cause.

Before authentication, admission covers the physical page and its control block.
After authentication, the stored count and decoded-byte fields determine the
additional cache allocation. The maximum layout product is a validation bound;
it is never reserved as though every record filled a physical page. Compression
probes likewise allocate their exact front-coded frame instead of padding one
physical block per offered record.

Learned routing bounds are cache hints, not required read state. Their index
grows only from spare capacity, without evicting cached payloads. If growth
exceeds the live-byte ceiling after a payload is admitted, the reader keeps the
payload without adding that hint. The same authenticated ID-range
search remains authoritative. Payload, exact-ledger, source and allocator
failures still report their original typed causes; none is hidden by this policy.

`SegmentedEvidence::bookkeeping_bytes()` reports the exact charged capacity of
the reader's retained request ledger, cache buckets and routing-bound buckets. It is included in
`live_bytes()` and can grow across queries. For query-owner release checks,
exclude this capacity and `unpinned_cache_bytes()` from both observations;
the original control, pinned blocks and query workspace remain in the balance.

`SegmentedFileProvider::from_file` attaches an opened native file to the caller's
certified snapshot identity. Supply the independently trusted receipt when
opening a session or publishing a layer. The adapter does not certify an
untrusted file or derive authority from its checksum. Browser-owned bytes use
`SegmentedBytes` through the same reader and authentication law. Provider Debug
output omits resident image contents.

Query-local representatives grow through the existing amortized admission home.
A bound ID originally imported from the same source retains that source's local
address; a cross-source probe still compares exact values. Bound reifier and
annotation probes use the existing subject-sorted stream. The statement-layer
consumer can stop the traversal before another row is translated, so an
ordinary positive LIMIT need not intern every reifier in the image.

## Statistical metadata

Data profile 3 retains four authenticated statistical sections: predicate/scope
records, compressed distinct-ID domains, selected object frequencies and
object-category/datatype counts. Predicate records carry row and distinct counts
and ranges into the other sections. Fixed record widths permit selective binary
searches for predicate, class and category counts without reading distinct-ID
payloads, RDF rows or dictionary values. An explicit domain visit streams its
requested range through the existing delta codec and read admission owner.

Full certification reconstructs the RDF dataset and checks each statistical
section against its derived metadata, including exact block packing. A matching
hash tree alone cannot authorize false counts or domains. The data profile and
sections participate in representation identity. Earlier data-profile images
require regeneration from their RDF input; this reader refuses unsupported
profiles. The certification receipt's encoding and hash laws remain distinct
from the data profile.

The reader exposes these counts through the common snapshot statistics seam.
Residual bounds retain the tighter existing subject-range estimate where it is
available. Operational metadata failures latch the original read cause and make
guarded query completion fail, even if no RDF row has been emitted.

The shared block cache indexes the complete `(snapshot, slot)` key through the
workspace's fixed-key hash home. Recency links name those same keys, so table
growth cannot invalidate them. Hits update a bounded number of links without
scanning or shifting the retained blocks. Eviction follows oldest-first links,
skipping pinned owners, and unlinks the first eligible block. Hash buckets,
control bytes and replacement overlap are admitted through the shared physical
table allocator; cached payloads and escaped pins keep their original grants.

Authenticated dictionary reads also teach a source-qualified page-range index.
It retains only bounds learned from actual reads, without lexical bytes or payload
pins. Those bounds survive payload eviction, so reloading a known page does not
repeat its fence search. Cold searches use nearby known ranges to narrow their
ordinal interval. Routing metadata shares the original live-byte grant and is
released with the operation; opening never preloads a directory or term map.

## Measured layout controls

The byte and peak figures in this section were recorded on main at
`0d075201f`, before data profile 2 added the authenticated statistics sections.
They are historical layout measurements. The 2,000/200,000-claim query control
separately passed profile 2's answer, touched-value and live-byte limits. Neither
result establishes profile 3's image size, lookup work or live residency; those
require new measurements of the complete representation.

The `segmented_query` integration control builds 200,000 RDF 1.2 claims, each
with a reifier and an annotation. It has 600,260 terms and a 9,291,797-byte
compact pack. Each image is reopened from a native file, queried through the
fallible engine with a 256 MiB live-byte ceiling, and compared with the resident
dataset. COUNT independently returns 400,000 rows; LIMIT 10 touches 21 query
representatives. The smaller 2,000-claim neighbor checks the same behavior.

| Block bytes / dictionary records | Image bytes | Dictionary + reverse-index bytes | Row bytes | Summary bytes | Authentication-tree bytes |
|---|---:|---:|---:|---:|---:|
| 4096 / 32 | 171,057,152 | 153,673,728 | 12,902,400 | 282,624 | 4,194,304 |
| 4096 / 64 | 92,127,232 | 76,840,960 | 12,902,400 | 282,624 | 2,097,152 |
| 8192 / 128 | 90,906,624 | 76,840,960 | 12,861,440 | 147,456 | 1,048,576 |

Each historical total also includes one header block. That dictionary appears twice: in
original ID order and in lexical reverse-lookup order. Both use fixed-size,
prefix-compressed blocks capped at the selected number of records. At 4096 / 32,
these two indexes occupy about 90% of this image. Sparse dictionary blocks,
rather than the authentication tree, dominate the expansion over the compact
pack. The row streams are ordinary, reifier and annotation rows, each in one
subject-first order; they are not three permutations of the entire dataset.
These totals do not measure profile 3's single dictionary and identifier-only
reverse index. New byte, reader-work and live-residency measurements are required
before attributing a compression or memory improvement to that representation.

Increasing the record cap can reduce padding and the number of authenticated
blocks. These recorded measurements used fixed-count grouping: for this corpus,
4096 / 128 refused with `ConstructionLimit`, while 4096 / 64 and 8192 / 128 fit
and retained identical decoded query answers. The byte-aware encoder now closes
an overflowing group before its next record instead of refusing that group;
the old 4096 / 128 refusal is historical evidence, not current behavior or a new
layout measurement. These measurements describe this authored corpus, not a
universal compression ratio or timing claim. Recorded peak admitted COUNT
bytes are 193,304,220 for both 4096-byte layouts and 168,269,468 for 8192 / 128.

## Profile 3 text controls

`ir_layout`'s `ir_segmented_text` groups compare 64 standalone literals at
4096 bytes / 64 records, including the datatype term automatically interned by
the builder. The repeated case uses authored prose with distinct leading record
numbers; the lower-redundancy neighbor uses deterministic seeded letters.
These are dictionary controls, separate from the historical claim/query corpus.
One native optimized development-profile quick run measured:

| Text | Image bytes | Build peak heap | Certification peak heap | Cold read peak heap | Admitted read peak | Cold forward/reverse provider requests |
|---|---:|---:|---:|---:|---:|---:|
| Short | 16,640 | 75,751 | 16,912 | 16,752 | 41,412 | 12 |
| Repeated prose | 16,640 | 885,691 | 359,824 | 272,132 | 1,223,700 | 12 |
| Lower redundancy | 100,352 | 1,067,803 | 360,656 | 381,459 | 1,222,728 | 144 |

Heap peaks are incremental allocations within each measurement window, not RSS.
The certification input copy is prepared outside its window; fixture values and
immutable provider bytes are prepared outside the read window. Warm reverse
lookup adds no provider requests. Median build times in this quick run were
131 microseconds, 801 microseconds and 76 milliseconds respectively; ten samples
on a shared host establish neither stable release timing nor a universal ratio.
The lower-redundancy build requested 61,511,493 total allocation bytes across
candidate encoding attempts despite its much smaller peak. Candidate encoding
therefore has a measured construction cost; this reader/storage improvement does
not establish the bounded, caller-owned writer required by the separate writer
work. The benchmark prints all allocation totals and times build, cold forward
lookup and warm reverse lookup independently.

The `compressed_archival_literals_preserve_rdf12_values_and_admitted_workspace`
integration control additionally carries scoped blanks, quoted triples, reifiers,
annotations, named and explicit empty graphs, and sparse identifiers above 2^54.
Its 116,288 literal text bytes fit the same 71,680-byte image as its short-text
neighbor. It checks exact forward/reverse values, certification, sticky typed
residency refusal, guards for escaped values and a warm full-dictionary read with no extra
provider requests. This authored high-redundancy result does not promise that
arbitrary long text has the same size as short text.

The maximum-layout regression uses 256 seeded 5120-byte literals with 1 MiB
pages. Exact-frame compression probes reduce incremental sealing heap peak from
276,341,234 bytes to 19,960,034 bytes, with a 4,194,560-byte image. Fixture values
and the mutable builder are prepared outside this window. The sealed image's
selected forward/reverse values also resolve under a 32,000,000-byte read ceiling; the
reader admits authenticated decoded sizes instead of reserving the 256 MiB
layout maximum. These controls do not establish bounded construction or RSS.
