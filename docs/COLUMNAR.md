<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF columnar schema

English | [简体中文](COLUMNAR.zh-Hans.md)

`purrdf-columnar` represents one RDF 1.2 dataset and one content-addressed blob
store as a closed set of five Parquet files. A `ParquetFiles` value always
contains all five files, including valid zero-row files for empty tables.

Every table is flat. The only Parquet physical types are `INT64` and
`BYTE_ARRAY`; textual byte arrays carry the standard UTF8 annotation. Required
columns contain one value per row. Optional columns use definition level 0 for
null and 1 for present.

## Exact-file source manifest

PURREMB `source_format = 2` binds the actual five-file schema v1 set through
the following canonical binary manifest. All integers are unsigned little-endian.

| Field | Encoding |
| --- | --- |
| Magic | Eight ASCII bytes `PURRCOLM` |
| Manifest version | `u32`, value `1` |
| Table count | `u32`, value `5` |
| Each file record | `u64` filename byte length, UTF-8 filename bytes, `u64` actual file byte length, raw 32-byte SHA-256 of all actual file bytes |

Records occur exactly in `Table::ALL` order: `terms.parquet`, `quads.parquet`,
`reifiers.parquet`, `annotations.parquet`, `blobs.parquet`. Filename framing is
`purrdf_hash::frame::frame_le`; there is no padding or trailing data. Every file
is included, including valid zero-row Parquet files. An absent file is not an
empty table.

`source_manifest` computes these bytes from `ParquetFiles`; it does not validate
Parquet or certify RDF. Certification completes the strict native reader,
including every blob digest, then independently canonicalizes the reconstructed
RDF dataset using the `purrdf-rdfc12` profile. PURREMB stores the manifest byte
length and `SHA256(manifest bytes)`, separately from that certified RDF digest.

`read_with_ordinals` completes the same strict read as `read` while retaining
only requested original zero-based term, quad, reifier and annotation rows.
Evidence uses the reader's explicit source-dictionary-to-kernel map and remains
bound to its reconstructed dataset. Neither kernel IDs nor final iteration order
are source ordinals. An empty request retains no statement-row evidence;
fulfilling a request never skips validation of later rows, tables or blobs.

## Projection failures

Native integrity refusals retain their original `ContentStoreError` in
`ColumnarError::ContentStore` or `BlobContent { row, cause }`. The latter row is
zero-based. Both SHA-256 identities are borrowed as original digest bytes through
the structured-error protocol. `Reconstruction` retains the complete original
`RdfDiagnostic`, including its storage and native-cause attachments. Standard
error sources borrow these causes directly; transport does not format, clone or
allocate a replacement cause. Existing human message prefixes and frozen bytes
are unchanged.

`project` and `write` return `ColumnarWriteError<D::ReadError>`. Its `SourceRead`
variant owns the original `DatasetView` failure; its `Codec` variant owns the
existing `ColumnarError`. Source transport neither formats nor clones the error.
The standard error chain borrows that original cause. When the source implements
the structured-error protocol, inspection reports its original nested fields.
`read` and a prepared projection's `to_parquet` still return `ColumnarError`.

## Tables

### `terms.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `id` | `INT64` | required | Dense, zero-based columnar term id. |
| `kind` | `INT64` | required | `0` IRI, `1` literal, `2` blank node, `3` triple term. |
| `lex` | UTF8 | optional | IRI, literal lexical form, or blank label; null for triple terms. |
| `datatype` | `INT64` | optional | Term id of a literal's datatype IRI. |
| `lang` | UTF8 | optional | Lower-cased language tag. |
| `direction` | `INT64` | optional | `0` left-to-right, `1` right-to-left. |
| `scope` | `INT64` | optional | Blank-node scope ordinal. |
| `triple_s` | `INT64` | optional | Triple-term subject id. |
| `triple_p` | `INT64` | optional | Triple-term predicate id. |
| `triple_o` | `INT64` | optional | Triple-term object id. |
| `named_graph` | `INT64` | required | `1` when the term names a declared graph, otherwise `0`. |

The term dictionary is the recursive closure of every term reachable from base
quads, reifier bindings, annotations, and named-graph declarations. Rows are
ordered by a canonical value encoding, not by backend-local ids. This retains
explicitly declared empty named graphs and makes bytes independent of a
`DatasetView` implementation's id allocation.

### `quads.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `s` | `INT64` | required | Subject term id. |
| `p` | `INT64` | required | Predicate term id. |
| `o` | `INT64` | required | Object term id. |
| `g` | `INT64` | optional | Named-graph term id; null is the default graph. |

Rows are unique and sorted by `(s, p, o, g)` with the default graph before
named graphs.

### `reifiers.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `reifier` | `INT64` | required | Reifier resource term id. |
| `s` | `INT64` | required | Reified statement subject id. |
| `p` | `INT64` | required | Reified statement predicate id. |
| `o` | `INT64` | required | Reified statement object id. |
| `g` | `INT64` | optional | Graph of the binding; null is the default graph. |

Rows are unique and sorted by `(reifier, s, p, o, g)`.

### `annotations.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `reifier` | `INT64` | required | Annotated reifier term id. |
| `predicate` | `INT64` | required | Annotation predicate term id. |
| `value` | `INT64` | required | Annotation object term id. |
| `g` | `INT64` | optional | Graph of the annotation; null is the default graph. |

Rows are unique and sorted by `(reifier, predicate, value, g)`.

### `blobs.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `digest` | UTF8 | required | Lowercase SHA-256 digest, 64 hexadecimal characters. |
| `bytes` | BYTE_ARRAY | required | Content-addressed payload bytes. |

Rows are sorted by raw digest bytes. A reader re-hashes every payload and
rejects a mismatch; it never repairs or re-files corrupt content.

## Parquet profile

Each non-empty table contains one or more row groups. Each column chunk has
one or more Data Page V2 pages and, when selected, one leading dictionary page.
Empty tables contain no row groups. The native reader decodes every encoding
the writer emits:

| Physical values | Encodings |
| --- | --- |
| `INT64` | `PLAIN`, `RLE_DICTIONARY`, `DELTA_BINARY_PACKED` |
| `BYTE_ARRAY` | `PLAIN`, `RLE_DICTIONARY`, `DELTA_LENGTH_BYTE_ARRAY`, `DELTA_BYTE_ARRAY` |

Optional-column definition levels use the Parquet RLE hybrid's run-length
form. There are no repetition levels in this flat schema. Compression is a
runtime choice of `Uncompressed`, `Zstd` (level 3), or `ZstdLevel(i32)` through
the always-present native `purrdf-zstd` codec. Numeric levels from -131072
through 22 are accepted, including zero's native default level. Invalid levels
are refused before source preparation or output. `ColumnOptions` can override
compression and encoding policy by table and zero-based column ordinal.
No semantic Cargo feature changes this profile.

`EncodingPolicy::Adaptive` compares actual encoded and compressed page costs.
PLAIN wins ties. Delta candidates apply only to their physical type; a chunk
dictionary is selected only when its page plus all index pages reduces cost.
The physical dictionary is separate from the canonical RDF term dictionary.
The default physical dictionary caps are 4,096 entries and 1 MiB, including its
payload/index capacities, control allowance and replacement-allocation peak.
Setting either cap to zero disables physical dictionaries. Exceeding a candidate's bounds falls back to PLAIN;
an actual allocator refusal remains an error. `Plain`, `Dictionary` and `Delta`
policies restrict the candidates while keeping deterministic PLAIN fallback.

Page headers and file metadata use Thrift Compact Protocol. Files carry stable
key/value metadata identifying columnar schema version `1` and their logical
table. The writer emits no clock, RNG, filesystem, or ambient producer data.

The reader is intentionally not a general Parquet engine. It accepts this exact
schema and encoding profile and fails closed on structural drift, unsupported
features, malformed references, invalid RDF positions, decompression size
mismatches, or trailing data.

## Physical bounds and group streaming

`write_with_options` and `write_to` accept `WriteOptions`. Defaults are:

| Target | Default |
| --- | --- |
| `page_bytes` | 1 MiB per column page |
| `page_rows` | 65,536 rows, including nulls |
| `row_group_bytes` | 128 MiB across all columns |
| `row_group_rows` | 1,048,576 rows |

Byte targets count canonical PLAIN values and page-local RLE definitions,
excluding headers, compression, indexes and footer. Each new optional page
restarts its runs. A target is an operating preference: one larger value can
occupy its own page and group, subject to physical bounds. Page byte/row targets
fit signed 32-bit fields; group targets, offsets and total rows fit signed
64-bit fields. Every byte-array length and page body must fit its signed
32-bit field, and indexed page header plus body must also fit a signed 32-bit
location size. Footer length fits an unsigned 32-bit field. Schema version
remains `1`.

Candidates are measured after compression. A page that expands beyond a
physical bound splits at its row midpoint and each half is encoded in order.
An indivisible oversized row fails explicitly. Cuts depend on input and
configuration. Dictionary charges and candidate reference limits use canonical
widths so a smaller host pointer does not choose different bytes.

Pending groups hold encoded PLAIN column buffers, packed nullable-column
presence, and one host-sized end offset per present byte-array value. Source
cells are borrowed from canonical preparation. Integer columns need no
per-row offset array. Compression and subdivision borrow ranges of these
buffers; each child rebuilds its definition levels for its own row range.
Buffers are reused across groups, and their capacities are bounded by the
largest admitted group. Candidate compression, physical dictionaries, page
statistics and output are separate owners. Auxiliary metadata is emitted after
each group's contiguous column chunks; its locations, rather than its complete
bytes, remain in the growing footer.

`write_to(view, blobs, options, callback)` emits all five files in `Table::ALL`
order. The callback receives `(Table, &[u8])` and must accept the complete chunk
or return its original error. Configuration and checked source preparation
finish before any callback. `write` and `write_with_options` collect these same
chunks into resident file buffers. `ColumnarSinkError<S, E>` preserves the
original source or sink error without requiring formatting or cloning traits.
Its sink refusal records the table, prior fully acknowledged bytes and attempted
chunk size. The failed callback may have written some or all of its chunk,
including the final file marker. No later callback runs and no rollback or
certification occurs after any failure. Callers own temporary-file and rename
transactions.

## Pruning metadata

Every chunk and data page carries min/max and exact null counts. Integer bounds
use signed ordering; byte bounds use unsigned lexicographic ordering under
Parquet's `TypeDefinedOrder`. Text bounds remain exact valid UTF-8. Binary
bounds use a 64-byte soft target: a lower prefix and a strict upper successor.
When no bounded upper successor exists, the exact full maximum is retained.
Exactness flags distinguish actual extrema from conservative bounds. All-null
pages have no extrema; their column-index entries have empty placeholders and
the null-page flag.

Each chunk has a standard column index and offset index. Locations cover actual
data-page headers plus compressed bodies, exclude dictionary pages, and count
first-row ordinals including nulls. Boundary order is conservatively unordered.
Indexes and Bloom filters are outside column/page byte totals. The native reader
checks their paired offsets/lengths and contiguous extents before advancing to
another group or the footer.

Bloom filters are opt-in through `WriteOptions::bloom` or a column override.
The global choice covers integer and text columns; a column override can also
enable binary payloads. `BloomOptions` defaults to 10 bits per present row and
a 64 KiB bitset cap. The cap is a multiple of 32 bytes, from 32 bytes through
128 MiB; the target is from 1 through 64 bits per value. Counts include repeated
values, so they are an upper bound on distinct values. Filters use standard
uncompressed split blocks and seed-zero `XXH64` over physical value bytes,
excluding byte-array length prefixes. Nulls are omitted; empty bytes are a
present value. Filter storage is deterministic and bounded by the configured
cap plus its small header. Saturation can increase false positives; filters
must never reject a value actually present.

## Native read and memory limits

The reader accepts any self-consistent sequence of supported groups; it need
not reproduce the writer's cuts or compressed bytes. It validates column and
page sizes, offsets, row totals, null counts and PLAIN cardinality before large
decoding allocations or decompression. Metadata collection counts are bounded
by the actual enclosing footer bytes, rather than an arbitrary item ceiling.
Row uniqueness and canonical order hold across group boundaries as well as
within groups. A later group refusal aborts the complete conversion.

For ZSTD pages, native frame inspection checks every concatenated and skippable
frame extent before allocating the bitmap or PLAIN buffer. Known frame content
sizes must fit the Parquet declaration; their total must match when every frame
declares its size. Unknown sizes are checked by actual decompression. Frame
checksums are verified during decompression. Header claims and actual decoder
output remain distinct evidence in refusal diagnostics.

There is no chosen table-byte ceiling. One decoded group is consumed and
released before the next. The term dictionary retains flat records across
groups to resolve forward datatype and triple-term references; semantic rows
then enter the dataset builder group by group. Blob payloads move into their
final content store after digest verification. Physical addressability and
checked size arithmetic still apply. A representable group can be large: page
fields permit nearly 2 GiB per column.

Resident APIs own the input or output file buffers, and the RDF dataset and content
store remain resident. Writing also retains canonical dictionary and sorted row
metadata needed for backend-independent ids and ordering; reading retains term
resolution state and the builder. Footer metadata grows with group count.
Native codec workspace and the page body can still be allocated from matching
metadata before malformed compressed content is rejected. These structural
checks do not establish a separate caller resource-admission policy.
Group streaming removes complete-table decoded columns and intermediate
statement tables, not these necessary owners. `project()` remains an explicitly
eager relational representation; direct `write()` consumes borrowed canonical
cells and blob payloads into pending columns without first constructing that
representation.

## Measurements

The native `codec` benchmark uses the deterministic
[benchmark fixtures](../crates/columnar/benches/common/mod.rs): 500 base rows produce 1,500 statements with repeated
predicates, low/high-cardinality text, sorted and irregular identifiers,
nullable columns and small blobs. `Reifiers` adds 500 reifiers and 500 annotations;
`LargeBlobs` adds one repeated and one irregular 1 MiB payload. The same exact
projected cells were independently compared across the PLAIN physical profile,
the adaptive native writer and Arrow 60.1.0. All use Zstandard level 3.

| Workload | PLAIN profile bytes | Adaptive native bytes | Arrow reference bytes |
| --- | ---: | ---: | ---: |
| Mixed | 10,338 | 8,012 | 21,479 |
| Reifiers | 17,522 | 12,634 | 32,687 |
| LargeBlobs | 1,059,167 | 1,057,560 | 1,070,548 |

The native default compares encoded costs with PLAIN fallback and configurable
operating targets. Dictionaries stop growing at their configured caps.
The representative inputs fit one group; page/group controls are additionally
checked through smaller boundary splits and independent page-index reads.
These measurements do not establish universally optimal targets or encodings.

Quick timing medians and 95 percent bootstrap intervals below are milliseconds
per complete five-table operation. The benchmark uses ten samples, 10,000
resamples, release optimization level 3, fat LTO and one codegen unit, with the
default allocator. Allocation counters run in a separate executable. The PLAIN
profile, final adaptive writer and Arrow reference ran sequentially on a shared
host; contention and separate measurement times limit speed comparisons.

| Workload | PLAIN RDF write | Adaptive RDF write | PLAIN projected encode | Adaptive projected encode | Arrow projected encode |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mixed | 1.954 [1.918, 2.008] | 5.687 [4.657, 6.003] | 0.830 [0.784, 0.850] | 3.080 [3.047, 3.112] | 0.328 [0.311, 0.357] |
| Reifiers | 3.564 [3.434, 3.714] | 10.70 [9.775, 12.51] | 1.854 [1.581, 3.076] | 5.948 [5.556, 6.178] | 1.124 [0.997, 1.739] |
| LargeBlobs | 7.571 [7.503, 7.655] | 15.14 [14.32, 15.67] | 5.823 [4.654, 9.669] | 11.01 [10.24, 12.91] | 6.566 [5.731, 8.429] |

The adaptive writer is slower in these observations. It measures actual candidate
encodings and compression to reduce file size; `EncodingPolicy::Plain` limits
candidate work when that tradeoff is unsuitable. Projected encoding starts from
prepared cells in each implementation. Arrow carrier import is not RDF
reconstruction and cannot substitute for the native read timing. Final native
RDF read medians were 1.857, 8.487 and 5.201 milliseconds for the three workloads;
the corresponding PLAIN-profile medians were 1.742, 6.461 and 5.610. These samples
establish neither a universal speed improvement nor a throughput guarantee.

Rust allocation windows distinguish requested layout traffic, retained owners
and peak live working storage. On the Mixed input, whole-write requested bytes
were 4,279,456 for the PLAIN profile and 6,312,632 for adaptive output; peaks
were 1,025,903 and 1,013,897 bytes. On LargeBlobs the corresponding requested
bytes were 14,836,496 and 33,298,418, with peaks of 5,726,297 and 6,833,776.
Candidate measurement trades allocation work for smaller files and pruning.
The sink retained zero output bytes after completion in all three cases, but
its peak includes canonical preparation and encoder scratch.

A separate 1 MiB blob probe measured source-preparation peak 7,357 bytes,
retained dictionary/sorted-row storage 5,596 bytes, and encoding peak 5,245,000
bytes including output and footer scratch. The largest emitted chunk was
1,048,746 bytes out of 1,054,785 total output bytes. The input dataset and blob
store were already resident outside those windows. Output-chunk size and
encoder peak overlap and must not be added or subtracted as independent peaks.

Whole-process peak RSS for the three-workload measurement/export process was
16,868 KiB for the PLAIN profile, 21,208 KiB for adaptive native output and
20,380 KiB for Arrow. These totals include input preparation, validation,
reporting and exports. They are not isolated encoder footprints. Arrow's Rust
allocation windows exclude its C Zstandard allocations; whole-process RSS
includes that workspace. No total-memory ratio follows from Rust counters.

Bloom size was measured separately on a 1,025-row projected statement table,
five groups, with an equality predicate returning one exact match. All five
groups were eligible by statistics. Instrumented filtered reads include the
filter header/body requests and may repeat bytes through buffering.

| Bits per value | File bytes | Filter bitset bytes | Absent probes positive / 1,280 | Selected groups | Filtered read bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4 | 32,358 | 2,144 | 94 | 2 | 890,494 |
| 10 | 35,215 | 4,992 | 7 | 1 | 447,284 |
| 16 | 37,850 | 7,616 | 0 | 1 | 448,054 |

All variants accepted every present value. The 10-bit default balances this
observed size/filter benefit; filters remain opt-in. Zero observed false
positives at 16 bits is a sample result. Independent offset-index selection
also returned all 11 exact matches from 34 candidate rows and left 11 excluded
pages wholly unread inside a selected group. These are actual file-read
requests, including buffering and cache effects, rather than disk traffic.

Run `cargo bench --locked -p purrdf-columnar --bench codec -- --quick columnar_codec`
to reproduce the native workload family. The standard harness reports median,
dispersion and bootstrap intervals. Comparisons use identical projected inputs;
RDF preparation/reconstruction and Arrow carrier-only encoding/import are
separate operations. Shared-host quick measurements require their intervals
and do not establish a universal throughput winner.
