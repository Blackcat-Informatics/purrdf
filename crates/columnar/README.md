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

The native codec writes flat `INT64` and `BYTE_ARRAY` columns with PLAIN,
dictionary and delta value encodings, RLE definition levels and Data Page V2.
It emits statistics and page indexes, with optional standard Bloom filters.
Runtime options control page/group targets and per-column Zstandard levels.
`write_to` emits incremental chunks; resident APIs collect the same output.
Source preparation and growing footer metadata remain resident. The crate
builds for `wasm32-unknown-unknown`.

The exact schema and invariants are documented in
[`docs/COLUMNAR.md`](../../docs/COLUMNAR.md). Most applications should access
this crate as `purrdf::columnar` through the umbrella `purrdf` crate.

`make columnar-oracle` writes a production-surface ZSTD fixture and asks the
DuckDB CLI to read every file, verify representative row counts, and confirm
that annotated text and raw payloads surface as `VARCHAR` and `BLOB`.

## License

Licensed under any one of the following, at your option:

- MIT license
- Apache License, Version 2.0
- Mulan Permissive Software License, Version 2 (MulanPSL-2.0)
