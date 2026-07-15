<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF columnar schema

`purrdf-columnar` represents one RDF 1.2 dataset and one content-addressed blob
store as a closed set of five Parquet files. A `ParquetFiles` value always
contains all five files, including valid zero-row files for empty tables.

Every table is flat. The only Parquet physical types are `INT64` and
`BYTE_ARRAY`; textual byte arrays carry the standard UTF8 annotation. Required
columns contain one value per row. Optional columns use definition level 0 for
null and 1 for present.

## Tables

### `terms.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `id` | INT64 | required | Dense, zero-based columnar term id. |
| `kind` | INT64 | required | `0` IRI, `1` literal, `2` blank node, `3` triple term. |
| `lex` | UTF8 | optional | IRI, literal lexical form, or blank label; null for triple terms. |
| `datatype` | INT64 | optional | Term id of a literal's datatype IRI. |
| `lang` | UTF8 | optional | Lower-cased language tag. |
| `direction` | INT64 | optional | `0` left-to-right, `1` right-to-left. |
| `scope` | INT64 | optional | Blank-node scope ordinal. |
| `triple_s` | INT64 | optional | Triple-term subject id. |
| `triple_p` | INT64 | optional | Triple-term predicate id. |
| `triple_o` | INT64 | optional | Triple-term object id. |
| `named_graph` | INT64 | required | `1` when the term names a declared graph, otherwise `0`. |

The term dictionary is the recursive closure of every term reachable from base
quads, reifier bindings, annotations, and named-graph declarations. Rows are
ordered by a canonical value encoding, not by backend-local ids. This retains
explicitly declared empty named graphs and makes bytes independent of a
`DatasetView` implementation's id allocation.

### `quads.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `s` | INT64 | required | Subject term id. |
| `p` | INT64 | required | Predicate term id. |
| `o` | INT64 | required | Object term id. |
| `g` | INT64 | optional | Named-graph term id; null is the default graph. |

Rows are unique and sorted by `(s, p, o, g)` with the default graph before
named graphs.

### `reifiers.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `reifier` | INT64 | required | Reifier resource term id. |
| `s` | INT64 | required | Reified statement subject id. |
| `p` | INT64 | required | Reified statement predicate id. |
| `o` | INT64 | required | Reified statement object id. |
| `g` | INT64 | optional | Graph of the binding; null is the default graph. |

Rows are unique and sorted by `(reifier, s, p, o, g)`.

### `annotations.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `reifier` | INT64 | required | Annotated reifier term id. |
| `predicate` | INT64 | required | Annotation predicate term id. |
| `value` | INT64 | required | Annotation object term id. |
| `g` | INT64 | optional | Graph of the annotation; null is the default graph. |

Rows are unique and sorted by `(reifier, predicate, value, g)`.

### `blobs.parquet`

| Column | Type | Cardinality | Meaning |
| --- | --- | --- | --- |
| `digest` | UTF8 | required | Lowercase SHA-256 digest, 64 hexadecimal characters. |
| `bytes` | BYTE_ARRAY | required | Content-addressed payload bytes. |

Rows are sorted by raw digest bytes. A reader re-hashes every payload and
rejects a mismatch; it never repairs or re-files corrupt content.

## Parquet profile

Each non-empty table is one row group with one Data Page V2 per column. Values
use PLAIN encoding. Optional-column definition levels use the Parquet RLE
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

## Decode safety budget

Before allocating a decoded column or entering Zstd, the reader cross-checks the
row-group, column-chunk, and page sizes and estimates the table's resident column
slots plus uncompressed value bytes. That fixed-profile working set may not
exceed 256 MiB per table. Metadata-derived vectors initially reserve at most
65,536 rows, and the public reader decodes and releases one physical table at a
time. Inputs beyond the ceiling fail with `ColumnarError::LimitExceeded`; the
writer applies the same budget so it cannot emit files its paired reader rejects.
