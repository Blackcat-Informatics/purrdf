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
upper bound. Both insertion-order and lexical-order streams use the same
front-coding encoder; their block populations and block counts may differ.
One encoded record, including its fence and framing, must fit a block or sealing
returns `ConstructionLimit`. The encoder does not retry the complete image with
a larger layout. Selective term lookup uses authenticated stored ID ranges;
opening still loads no dictionary directory or term map. Existing full-count
blocks retain their encoding and their direct first-probe lookup.

`SegmentedReadLimits` specifies ceilings, not arrays to allocate at opening.
The exact request ledger, payload cache and learned routing bounds grow as they are used. Their
concrete capacities, copied/decoded blocks, pins and query scratch share the
live-byte admission. Buffer replacement admits both the old and new layouts
until the old allocation is destroyed. A refusal reports the original typed
cause; evidence is never truncated to fit a smaller ledger.

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

Each total also includes one header block. The dictionary appears twice: in
original ID order and in lexical reverse-lookup order. Both use fixed-size,
prefix-compressed blocks capped at the selected number of records. At 4096 / 32,
these two indexes occupy about 90% of this image. Sparse dictionary blocks,
rather than the authentication tree, dominate the expansion over the compact
pack. The row streams are ordinary, reifier and annotation rows, each in one
subject-first order; they are not three permutations of the entire dataset.

Increasing the record cap can reduce padding and the number of authenticated
blocks. These recorded measurements used fixed-count grouping: for this corpus,
4096 / 128 refused with `ConstructionLimit`, while 4096 / 64 and 8192 / 128 fit
and retained identical decoded query answers. The byte-aware encoder now closes
an overflowing group before its next record instead of refusing that group;
the old 4096 / 128 refusal is historical evidence, not current behavior or a new
layout measurement. These measurements describe this authored corpus, not a
universal compression ratio or timing claim. Recorded peak admitted COUNT
bytes are 193,304,220 for both 4096-byte layouts and 168,269,468 for 8192 / 128.
