<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-geo

GeoSPARQL 1.1 (OGC 22-047r1) for PurRDF: exact, float-free geometry reached from
SPARQL through the evaluator's two existing extension seams.

`purrdf-geo` is a **sibling crate**, not a kernel change. It parses
`geo:wktLiteral` and `geo:geoJSONLiteral` lexical forms into an exact geometry
model, decides the OGC topological relations over that model, computes the
non-topological accessors, measures and constructors, and hands the `geof:`
family to a host as registrations against the scalar-function registry and the
Query Rewrite relations as registrations against the property-function registry.
Nothing in `purrdf-core` or `purrdf-sparql-eval` changes to make that work.

## It mints no vocabulary

GeoSPARQL's IRIs are OGC's, not PurRDF's. Every IRI this crate reads or writes —
the two literal datatypes, the `geof:` function names, the `geo:` spatial
relations rewritten by the relations, and the coordinate reference system a WKT
literal omits — is supplied by the caller through `GeoVocab`, which has no
`Default` and never will. A vocabulary term that is absent makes the feature that
needs it a hard error, never a fabricated fallback. Every fixture in this crate
uses `example.org`.

## Every answer is exact, and identical on every target

Geometry is where floating point normally destroys reproducibility: `f64`
addition is not associative, so a different traversal order gives a different
answer, and a native build and a `wasm32-unknown-unknown` build can disagree
about a predicate that sits near a boundary. This crate closes that channel
rather than mitigating it.

- **Coordinates are read as exact rationals.** A lexical decimal is parsed digit
  by digit into an exact numerator and denominator. `str::parse::<f64>()` appears
  nowhere on the ingest path, so nothing is rounded on the way in.
- **Every geometric decision is integer arithmetic.** Orientation, segment
  intersection, point-in-ring, ring winding and the DE-9IM matrix are comparisons
  of exact rationals over arbitrary-precision integers. Rust specifies integer
  arithmetic completely and identically on every target.
- **Irrational measures are integer square roots.** A length is a sum of `sqrt`
  terms, and a sum of rounded terms depends on the rounding; so each term is an
  exact integer square root at a fixed internal scale and the terms are summed as
  integers — one rounding, at the end, of a value that was exact until then.
- **The single float boundary is the result literal.** An `xsd:double` result is
  the correctly rounded nearest double, computed with integer arithmetic and
  assembled with `f64::from_bits`. The crate root denies
  `clippy::float_arithmetic`, so there is no second float path to find.

## What is here, and what is not

Implemented: the WKT and GeoJSON codecs; every topological relation of the Simple
Features, Egenhofer and RCC8 families over an exact DE-9IM; the accessors; and
the measures and constructors that are exactly computable.

Registered but **hard-erroring by name**: the operations that would require
facilities this crate deliberately does not have — a coordinate-reference-system
database for `geof:transform`, an ellipsoidal geodesic for the `metric*` family.
They are never silently absent and never answer a default. A topological
predicate that returned `false` because it was unimplemented would be
indistinguishable from one that returned `false` because the geometries genuinely
do not relate, and that is the failure this crate exists to keep out.

See [`docs/design/purrdf-geo-exactness.md`](../../docs/design/purrdf-geo-exactness.md)
for the full accounting.

## Licence

MIT OR Apache-2.0.
