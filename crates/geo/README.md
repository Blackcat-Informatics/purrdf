<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-geo

GeoSPARQL 1.1 (OGC 22-047r1) for PurRDF: exact source geometry and certified
ellipsoidal computations reached through the shared Rust engine and evaluator.

`purrdf-geo` is the compatibility facade. The computational implementation lives
in `purrdf-geo-kernel`, including exact geometry codecs, topology, measures and
reusable spatial indexes. `purrdf-sparql-eval::geo` owns scalar-function and
property-function adapters over that shared implementation. Existing
`purrdf_geo` value and module paths remain available through reexports.

## Standard references and explicit configuration

`NativeSparqlEngine::new()` and `Default` install the immutable official
GeoSPARQL scalar-function set. Unprefixed WKT, explicit OGC CRS84 and standard
GeoJSON denote WGS84 longitude/latitude. Official terms have one home in
`purrdf_iri::vocab::ogc`; PurRDF mints no application vocabulary.

`GeoProfile` registers every additional geographic reference explicitly.
EPSG:4326 refuses until registered as WGS84 latitude/longitude; that declared
axis order swaps the source ordinates without changing their stored spelling.
An ellipsoid registration does not install a datum transformation. CRS84 cannot
be replaced, and arbitrary function registrations cannot shadow standard IRIs.
Identical sealed first-party registrations coalesce.

Caller-named vocabularies and their explicit planar units remain available
through `GeoVocabBuilder` and the compatibility adapters.

## Exact sources and certified numerical results

WKT and GeoJSON decimals are retained as exact rationals, including Z and M.
The exact planar topology and fixed-scale planar metric contracts remain
byte-deterministic. Ellipsoidal point distance computes through the shared Rust
kernel using independently generated coefficients and bounded intervals. A
completed distance is the half-even rounding of the true shortest ellipsoidal
distance to one micrometre; an unresolved rounding boundary refuses rather than
returning an uncertified approximation. Host conversion carries its additional
half-ULP bound.

`MetricContext` is thread-bound. Immutable prepared references are shareable;
workers own their scratch and context. The metered native function seam charges
bounded internal work, checks cancellation between numerical chunks and
preserves governed incomplete-outcome evidence. Operational precision, memory,
work and environment refusals remain query-fatal through FILTER and BIND.

Prepared queries bind semantic laws, carrier references and execution admission
as separate identities. Changing an adequate policy leaves the completed
mathematical law unchanged while refusing replay under a mismatched prepared
profile.

The shared [version-1 geographic profile and request codec](../../docs/GEO-PROFILE.md)
serves Rust string sessions, Python, C, wasm and the CLI. Explicit continuous
image records retain their source curves for tight metrics; angular carrier
materialization uses a separate certified export.

See [`docs/design/purrdf-geo-exactness.md`](../../docs/design/purrdf-geo-exactness.md)
for the computational contracts.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
