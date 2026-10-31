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

Each non-empty table contains one or more contiguous row groups, each with one
Data Page V2 per column. Empty tables contain no row groups. Values use PLAIN encoding. Optional-column definition levels use the Parquet RLE
hybrid's run-length form; there are no repetition levels because the schema is
flat. A write chooses one codec for all value bodies: UNCOMPRESSED or ZSTD.
Zstd is a runtime choice backed by the always-present pure-Rust
`structured-zstd` dependency, never a Cargo feature.

Page headers and file metadata use Thrift Compact Protocol. Files carry stable
key/value metadata identifying columnar schema version `1` and their logical
table. The writer emits no clock, RNG, filesystem, or ambient producer data.

The reader is intentionally not a general Parquet engine. It accepts this exact
schema and encoding profile and fails closed on structural drift, unsupported
features, malformed references, invalid RDF positions, decompression size
mismatches, or trailing data.

## Physical bounds and group streaming

The writer consumes canonical rows and takes the longest next prefix whose
per-column PLAIN bytes plus exact RLE definition bytes fit the signed 32-bit
page-size fields, and whose row count fits the page's signed 32-bit count.
A byte-array cell must itself fit the PLAIN signed 32-bit length. Compression
is measured after encoding the candidate: if a compressed page exceeds its
physical field, the candidate is split at its midpoint and each half is encoded
by the same rule, in order. An indivisible oversized row fails explicitly.
These cuts depend on the data and selected codec, never a row-count tuning
constant or the host's available memory. Files whose rows fit one group retain
the same encoding. Offsets and table row counts fit signed 64-bit fields; the
footer fits its unsigned 32-bit length. Schema version remains `1`.

Pending groups hold encoded PLAIN column buffers, packed nullable-column
presence, and one four-byte end offset per present byte-array value. Source
rows are released after their cells are appended. Integer columns need no
per-row offset array. Compression and subdivision borrow ranges of these
buffers; each child rebuilds its definition levels for its own row range.
Buffers are reused across groups, and their capacities are bounded by the
largest admitted group. Compression output and final file buffers are separate
resident owners.

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

The API owns the input or output file buffers, and the RDF dataset and content
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
