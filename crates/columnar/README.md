<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# purrdf-columnar

`purrdf-columnar` is PurRDF's first-party, byte-deterministic Parquet bridge for
DataFrame and SQL consumers. It maps an RDF 1.2 `DatasetView` and its
content-addressed blob store onto exactly five flat tables:

- `terms.parquet`
- `quads.parquet`
- `reifiers.parquet`
- `annotations.parquet`
- `blobs.parquet`

The crate implements the deliberately narrow Parquet subset it writes: INT64
and BYTE_ARRAY physical columns, PLAIN values, RLE definition levels, Data Page
V2, and either UNCOMPRESSED or ZSTD page bodies. It does not wrap Arrow,
parquet-rs, or a general Thrift runtime. All APIs are in-memory and build for
`wasm32-unknown-unknown`.

The exact schema and invariants are documented in
[`docs/COLUMNAR.md`](../../docs/COLUMNAR.md). Most applications should access
this crate as `purrdf::columnar` through the umbrella `purrdf` crate.

`make columnar-oracle` writes a production-surface ZSTD fixture and asks the
DuckDB CLI to read every file, verify representative row counts, and confirm
that annotated text and raw payloads surface as `VARCHAR` and `BLOB`.

## License

Licensed under either of

- Apache License, Version 2.0, or
- MIT license

at your option.
