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
An encoded key too large for an ordinary block is stored once across authenticated
overflow pages. Its forward-dictionary page contains a single identifier and a
span descriptor; it carries no copy of the key. Overflow pages have a five-byte
kind/length header and canonical zero padding. The physical block size remains
the caller's choice. Full certification checks each span, its reconstructed RDF
value and exact contiguous coverage of the overflow section.

Selective term lookup uses authenticated stored ID ranges;
opening still loads no dictionary directory or term map. Full-population
blocks retain their direct first-probe lookup. Data profile 6 refuses older
profiles; regenerate their images from the original RDF input.

Reverse lookup compares the referenced dictionary keys through the same admitted
reader as forward lookup. A held reverse-index block may leave the payload cache
while its original reservation remains charged until the last internal owner
drops. This allows a one-slot cache to read the dictionary during lookup without
evicting an escaped lexical term guard or hiding its live bytes.

Oversized-key comparisons stream authenticated pages until their ordering is
known, admitting one physical page of scratch. Routing and small reverse lookups
do not allocate the complete oversized key. A forward lexical guard materializes
the key once inside its cached descriptor owner, after admitting the complete
decoded payload and page scratch. That payload remains charged until its last
escaped guard drops, even after the session drops. Failed reads preserve the
original provider request and typed cause; incomplete payloads never enter the
cache.

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

## Ordered subject joins

A read session over one segment can scan every statement stream in the
segment's numeric subject order. Inclusive subject bounds narrow the
authenticated block directory before reading statement payloads. A range
cursor keeps a bookmark and releases its payload pin before returning a row.
Other cursors and term lookups can use the same cache slot between yields.
The order guarantee is local to the segment's dictionary. It does not
transfer to a stack that translates or assigns identifiers across segments.

An unseeded two-pattern basic graph pattern with fixed predicates and
one graph scope can merge using that guarantee when the subject variable
occurs once in each pattern and is their only shared variable.
The operator combines asserted, reifier and annotation streams in subject
order. It bounds the probe between the first driving key and the last
possible driver from the authenticated directory. Each stream can seek
forward using that directory when the opposite key lies past its current
page. This skips interior disjoint ranges without reopening the scan.
Exact predicate and graph counts reject absent patterns before walking
a multi-page stream directory.
Repeated driving keys
reuse a single probe row or reopen a longer probe run. Both preserve
bag multiplicity without buffering the group. The ordinary result and the prepared row
cursor use the same merge. Operations with governed settlement, evidence,
an annotation value range, or an order-sensitive output window use the
indexed evaluator.

## Statistical metadata

Data profile 6 retains four authenticated statistical sections: predicate/scope
records, compressed distinct-ID domains, selected object frequencies and
object-category/datatype counts. Predicate records carry row and distinct counts
and ranges into the other sections. Each predicate/scope record also carries
the graph-aware and graph-free literal-source sums, their exact population and
a physical-record sum that distinguishes asserted, reifier and annotation rows.
These content summaries are computed from validated rows during construction;
all-graph summaries combine the graph partitions without rehashing their rows.
They are distinct from the sizing statistics' cache discriminator. Fixed record widths permit selective binary
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

The header also retains the exact normalized statement-union
and reifier multiset: asserted and annotation occurrences of the same value quad
count once in that union, while a virtual reifier declaration remains a separate
family. Its 128-bit population and 256-bit sum are reconstructed and compared
during full certification, even when a modified image has a rebuilt hash tree.
This content fingerprint is independent of dictionary IDs and classification;
it is neither a signature nor a membership proof. Chronological observations
retain original immutable source owners and use the query's own visibility and
read admission for overlap and removals. A source version's retained grant stays
charged after the creating query drops.

The normalized source directory collapses asserted/annotation copies
of a statement and keeps virtual reifiers in their separate identity family;
its entries retain actual source-local addresses for selective value reads.
Each 37-byte record contains the canonical record digest, the source-local
32-bit ordinal and stream tag. Range equality uses a canonical BLAKE3 Merkle
node over the sorted digests, the binary Patricia trie over their bits. A run
of equal digests is a leaf: the digest itself when it occurs once, otherwise
the keyed hash of the digest and its count. A set of two or more distinct
digests splits at the first bit where its least and greatest digests differ;
its node is the keyed hash of the two child nodes. The keys are derived from
the domains `purrdf-core/source-directory-leaf/v1` and
`purrdf-core/source-directory-branch/v1`. The node depends only on the member
multiset, never on paging, chunking or edit history, so equal nodes mean equal
members. The additive sums kept beside it are not collision resistant: a party
choosing rows can build two different sets with equal sums and populations.
They remain only for content fingerprints and for agreement between two values
the same process built.

Format version 8 (`PURRSEG8`) stores sparse checkpoints in a separate
authenticated fixed-record stream: one 40-byte record per branch of at least
32 members, holding the little-endian ordinal of the branch's first right-hand
member and its node, in ascending ordinal order. A digest-prefix range of at
least 32 members with two distinct digests is exactly one such branch; a reader
finds it by binary search at that boundary. A smaller range hashes its fewer
than 32 records; those digests are counted in `SourceDifferenceWork::members`,
while the original read owner accounts for checkpoint and directory block
reads. Full certification rebuilds and compares every record and checkpoint
from the RDF rows, including their exact packing. Selective reads compare digest
ranges through the same algorithm as resident directories; only differing
records fetch their physical rows. The original read operation owns every
directory and row read. Replacing a sealed descriptor therefore does not itself
require enumerating either complete layer.

Version 7 images (`PURRSEG7`) remain readable. Their checkpoint stream holds one
256-bit cumulative additive sum per complete group of 32 records; full
certification rebuilds and compares those sums as a local consistency check, and
nothing else reads them. A version 7 range is compared by hashing its records.

The directory payload approaches 38 bytes per row, before physical page padding
and the authentication tree, instead of 88. Images before version 7 require
regeneration from their original RDF input.

Native construction reserves source-record storage from the exact physical row
count. Sources without annotations reuse their physical directory for normalized
keys; they retain no duplicate record or checkpoint buffers. Annotated sources
retain a separate normalized directory because statement copies must collapse.
Resident directories keep the additive total from the freeze and derive the
same sparse branch checkpoints and root node on first use, from the sorted
records, into resident memory with typed allocation refusal. A statistics build
that never compares ranges or writes an image does no Merkle hashing, and sealing
and certification hash only the effective directory. Scalar term hashing streams the initial value directly;
only nested triple terms require admitted traversal scratch. These storage choices
preserve source identities. In 4.0, resident and controlled sealing share the
fallible digest sorter. Its permutation of equal-digest source-local addresses
can change persisted directory bytes from earlier releases; regenerate those
images when upgrading.

An observation retains an effective directory of canonical keys. Its immutable Patricia tree uses leaves of at most
32 keys and copies only changed leaves and paths. Every node carries the
canonical Merkle node of its keys, so an edit rehashes one leaf and its path.
Each node and leaf buffer
retains its original read-account grant; keys carry no dataset, provider or row
address. Successors retain the current snapshot and independent key storage,
allowing replaced source owners to release. Initial positive composition reads
certified keys rather than reconstructing RDF values. Removals and publication
changes still use the original visibility engine to determine actual events.
Known sealed prefixes share those keys. Regrouped intermediate prefixes may
instead refer lazily to the current snapshot's certified directories, avoiding
a retained-key copy merely to populate a cache. One admitted range traversal
compares groups of directories; it never constructs a merged row/key array.
A range is skipped when both groups hold the same members carrier by carrier,
with equal nodes in order. Members spread over other carriers are split until
each range holds one carrier per side, so the digest work of such a regrouping
is linear in its records, while only changed rows are resolved. Contributors
with equal canonical roots collapse before group comparison. Baseline reuse,
compaction checks and the no-op test of a captured layer also compare canonical
roots. Source
removals retain chronological visibility checks, including old removals when
comparison uses physical contributor bags rather than effective keys.

Full compaction compares the maintained effective keys with the new normalized
image directory. A retained suffix permits the same comparison against the
replaced prefix. Sealing a head while starting a fresh batch transfers the
previous complete directory into the new sealed prefix. A large head reset
compares only its new operations when those are smaller than the native
difference; unchanged cumulative heads use native directory differences.
Publishing a single resident head uses the same normalized-key comparison.

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
result establishes profile 4's image size, lookup work or live residency; those
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
These totals do not measure profile 4's single dictionary and identifier-only
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
