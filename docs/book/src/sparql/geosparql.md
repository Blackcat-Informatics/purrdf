<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# GeoSPARQL

**What it replaces, and where it stops.** This is the surface that lets an RDF
project drop the PostGIS it kept beside its triple store for spatial
predicates and measures: the `geof:` functions and `?a geo:sfWithin ?b` with
its Simple Features, Egenhofer and RCC8 siblings, answered in-process over the
dataset the query already holds, with no GEOS or PROJ, and byte-identical
natively and on wasm32. Distances, lengths, perimeters, areas and buffers on
CRS84 are measured on the WGS84 ellipsoid in metres. It is not a PostGIS: there
is no coordinate-reference-system database, so every reference other than
CRS84 and every coordinate operation is registered explicitly; the concave
hull, the bounding circle, the overlay set operations and the GML/KML/DGGS
encodings hard-error by name. No raster.

The computation lives in `purrdf-geo-kernel`; `purrdf-sparql-eval` owns the
SPARQL adapters, and `purrdf-geo` (`purrdf::geo` from the umbrella crate)
re-exports both. There is no GEOS and no PROJ behind it — the DE-9IM engine,
the ellipsoidal geodesics, WKT and GeoJSON are implemented in pure Rust, which
is what lets it build for `wasm32-unknown-unknown`.

## Built in: the standard functions and CRS84

PurRDF mints no vocabulary of its own; published W3C and OGC standard
vocabularies are built in, and GeoSPARQL 1.1 (OGC 22-047r1) is one of them.
`NativeSparqlEngine::new()` and `Default` install the immutable standard
`geof:` function set under the OGC function namespace. No registry, binding or
vocabulary setup is needed, and a caller registration cannot shadow a standard
IRI.

```rust,ignore
use purrdf::SparqlRequest;
use purrdf::sparql::NativeSparqlEngine;

let result = NativeSparqlEngine::new().query(
    &dataset,
    SparqlRequest {
        query: r#"PREFIX geof: <http://www.opengis.net/def/function/geosparql/>
                  PREFIX geo:  <http://www.opengis.net/ont/geosparql#>
                  SELECT (geof:metricDistance("POINT(0 0)"^^geo:wktLiteral,
                                              "POINT(1 0)"^^geo:wktLiteral) AS ?m)
                  WHERE {}"#,
        base_iri: None,
        substitutions: &[],
    },
)?;
// ?m = "1.11319490793E5"^^xsd:double — metres on the WGS84 ellipsoid.
```

A `geo:wktLiteral` with no `<IRI>` prefix is read as CRS84 — GeoSPARQL 1.1's
own normative default — and so is GeoJSON, which RFC 7946 fixes to it: WGS84
longitude followed by latitude. Every other coordinate reference system,
EPSG:4326 included, is unregistered until the caller declares it on a
`GeoProfile`, and its use is refused by name until then. EPSG:4326 must be
declared with its latitude/longitude axis order; no axis order is inferred
from a name, a numeric range or a dataset. An ellipsoid declaration installs
no datum transformation. CRS84 itself cannot be replaced.

```rust,ignore
use purrdf::geo::{AxisOrder, Crs, GeoProfile, GeographicReference};
use purrdf::iri::vocab::ogc;
use purrdf::sparql::QueryOptions;

let mut profile = GeoProfile::standard();
profile.register_reference(
    Crs::new(ogc::EPSG4326)?,
    GeographicReference::wgs84().with_axes(AxisOrder::LatLon),
)?;
let options = QueryOptions::new().with_geo(&profile);
```

`GeoProfile` also registers additional linear units, named coordinate
operation chains (for `geof:transform`), and its execution limits; the
version-1 JSON form shared by Python, WebAssembly, C and the CLI is specified
in [`docs/GEO-PROFILE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GEO-PROFILE.md).

## Measures in metres

`geof:metricDistance`, `geof:metricLength`, `geof:metricPerimeter`,
`geof:metricArea` and `geof:metricBuffer` answer on the geometry's ellipsoid —
WGS84 for CRS84 — in metres; `geof:distance`, `length`, `perimeter`, `area`
and `buffer` do the same in the OGC metre or a unit registered on the
`GeoProfile`, converted by its exact factor. A completed metric value is
rounded half-even once, at the end: to the micrometre for distances, lengths
and perimeters, and to 0.01 m² for areas. A point-to-point distance is the correctly rounded true
shortest ellipsoidal distance; a value that cannot be resolved to its rounding
boundary is refused rather than approximated. General geometry distance
carries a total bound of 0.25 mm; length and perimeter the greater of 1 mm and
10⁻¹⁴ × value; area the greater of 0.1 m² and 10⁻¹⁴ × value. Buffers return an
outward carrier with a certified 0.1-metre radius band. Results are
`xsd:double`.

`geof:transform` resolves a registered operation chain to its target IRI and
follows each complete source edge, not just the written vertices; with no
registered chain it is refused. `geof:asGeoJSON` needs CRS84 input or a
registered operation to CRS84.

## How a refusal travels

A malformed literal or a domain refusal — mixed CRSs, the measure of an empty
geometry, an out-of-range index — is a per-solution **expression error**: the
row is eliminated under `FILTER`, and the variable is left unbound under
`BIND`/`SELECT`, while the query continues. An unimplemented function, an
unregistered reference or a wrong argument count holds for every solution
alike and stays query-fatal, because answering "no value" there would empty a
result set and present that as the answer. Precision, work, memory,
convergence and cancellation refusals are query-fatal too, through `FILTER`
and `BIND`; the governor charges the numerical work in bounded chunks.

## Spatial relations on the property-function seam

GeoSPARQL's Query Rewrite rules let `?a geo:sfWithin ?b` hold between
*features* whose geometries satisfy the relation, not only where a triple
asserts it. These relations are a Rust registration over a projection of the
dataset. `GeoIndex::from_dataset` projects a dataset's geometry literals once,
and `relation::register` installs one property function per relation of the
families the caller names — Simple Features, Egenhofer, RCC8 — against a
`PropertyFunctionRegistry`. An empty family list is refused: registering
nothing and returning success would surface much later as a query whose
`geo:sfWithin` was parsed as an ordinary triple pattern and matched nothing.

```rust,ignore
use std::sync::Arc;
use purrdf::geo::relation::{self, GeoIndex, GeoIndexConfig, GraphSelector};
use purrdf::geo::{GeoTerm, RelationFamily, standard_vocabulary};
use purrdf::sparql::{ExtensionEnv, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};
use purrdf::{SparqlRequest, TermValue};

let vocab = standard_vocabulary();
let config = GeoIndexConfig::new(
    vec![TermValue::iri(vocab.term(GeoTerm::AsWkt))],
    GraphSelector::Any,
)?;
let index = Arc::new(GeoIndex::from_dataset(&dataset, vocab, &config)?);

let mut relations = PropertyFunctionRegistry::new();
relation::register(&mut relations, vocab, &index, &[RelationFamily::SimpleFeatures])?;

// The environment a query text is read in claims every registered relation IRI
// — `geo:sfWithin` among them — in predicate position.
let env = ExtensionEnv::over_relations(relations)?;

let result = NativeSparqlEngine::new().query_with_options_view(
    &dataset,
    SparqlRequest {
        query: r#"PREFIX geo: <http://www.opengis.net/ont/geosparql#>
                  SELECT ?a ?b WHERE { ?a geo:sfWithin ?b }"#,
        base_iri: None,
        substitutions: &[],
    },
    QueryOptions::new().with_env(&env),
)?;
```

`relation::register` decides relations with the explicitly planar exact DE-9IM
over the written coordinates. For relations on the ellipsoid, prepare a
`GeographicGeoIndex` from the same `GeoIndex` and a `GeoProfile`, and install
it with `relation::register_geographic`; it uses the same prepared atlas as
the scalar `geof:` relation functions.

An asserted `geo:sfWithin` triple matches whether or not the geometries
satisfy it — the rewrite rules are entailments, not definitions — and a
relation the index refutes contributes no row beyond what the data asserts.

## Every answer is exact, and identical on every target

- **Coordinates are read as exact rationals.** A lexical decimal is parsed
  digit by digit into an exact numerator and denominator; nothing is rounded on
  the way in, and the original source is retained.
- **Topology is exact.** Planar orientation, intersection, point-in-ring and
  the DE-9IM matrix are comparisons of exact rationals over arbitrary-precision
  integers. Ellipsoidal decisions are refined with certified bounds until they
  are decisive; no epsilon decides a predicate.
- **Measures round once.** Ellipsoidal values are enclosed with certified
  bounds over the original coordinates and quantized half-even on a fixed grid
  at the end, independent of the numerical precision used to reach it.
- **The single float boundary is the result literal.** An `xsd:double` result
  is the correctly rounded nearest double, computed with integer arithmetic.
  The crate roots deny `clippy::float_arithmetic`.

Native Rust pins the geometry corpus against committed digest bytes, and
`make wasm` builds the crates for the WASM target. The full accounting is
[`docs/design/purrdf-geo-exactness.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-geo-exactness.md).

## What is here, and what is not

Implemented: the WKT and GeoJSON codecs (with CRS and coordinate-dimension
support); every topological relation of the Simple Features, Egenhofer and RCC8
families; the accessors; the convex hull, envelope, boundary and centroid; the
measures and `metric*` functions; `buffer`/`metricBuffer`; and `transform`
over registered chains.

Registered but **hard-erroring by name**: `geof:boundingCircle` and
`geof:concaveHull` (OGC 22-047r1 leaves their parameters
implementation-defined), `geof:intersection`, `geof:union`,
`geof:difference` and `geof:symDifference` (no general set-construction output
law), and `geof:asGML`, `geof:asKML` and `geof:asDGGS` (only WKT and GeoJSON
are implemented). They are never silently absent and never answer a default.

The standard `geof:` functions are installed by every host's default engine.
Geographic profiles, sessions and point indexes are exposed to Python,
WebAssembly, C and the CLI through the shared version-1 records; the Query
Rewrite relation registration is a Rust-host seam.
