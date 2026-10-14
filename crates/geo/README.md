<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->


English | [简体中文](README.zh-Hans.md)
# purrdf-geo

The GeoSPARQL compatibility facade for PurRDF. It reexports the shared geometry
kernel through the established Rust paths and exposes the evaluator's `functions`
and `relation` modules. Geometry and numerical computation live in
`purrdf-geo-kernel`; SPARQL registration and execution live in `purrdf-sparql-eval`.

## Standard functions and explicit configuration

The native SPARQL engine installs the standard GeoSPARQL scalar registrations.
Standard WKT and GeoJSON use WGS84 longitude/latitude. `GeoProfile` supplies
additional references, axis orders, output units and actual coordinate-operation
chains. Caller-named `GeoVocab` registrations retain their explicit planar
contracts. Missing required configuration is a typed refusal, not a fabricated
coordinate operation or a false geometry result.

## Exact coordinates and specified numerical laws

Decimal coordinates are read as exact rationals. Planar orientation,
intersection, winding and DE-9IM decisions use integer/rational arithmetic.
The planar measure implementation uses integer square roots at a specified
internal scale. Its integer computation is exact for that scaled law; an
irrational physical length is not thereby exact. Each truncated segment is at
most its true length, with error below one scale unit. The total error bound
grows with the number of segments. Output rounding is explicit.

The kernel also provides certified geodesy, ellipsoidal metrics, coordinate
operations, physical buffers, hierarchical cells/covers and spatial indexes.
These operations retain their declared metric/reference laws and budgets.
Argument errors follow SPARQL expression-error behavior. Operational precision,
convergence, work, memory, environment and reference-configuration refusals
remain fatal rather than becoming false FILTER results or unbound BIND values.

## Portable execution

`make geo-determinism` executes the registered planar, geographic, operation,
buffer, cover and index controls on native, portable wasm and SIMD wasm.
Compilation alone is not proof of matching completed answers. See the
[kernel README](../geo-kernel/README.md) and
[exactness design](../../docs/design/purrdf-geo-exactness.md).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
