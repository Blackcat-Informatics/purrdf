<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# GeoSPARQL

**What it replaces, and where it stops.** This is the surface that lets an RDF
project drop the PostGIS it kept beside its triple store for spatial
predicates: `?a geo:sfWithin ?b` and its Simple Features, Egenhofer and RCC8
siblings, plus the `geof:` functions, answered in-process over the dataset the
query already holds, exactly, with no GEOS or PROJ, and byte-identical natively
and on wasm32. It is GeoSPARQL 1.1's topological predicates, accessors, and
exactly computable measures and constructors over vector geometry, not a
PostGIS: `geof:transform` hard-errors by name (there is no CRS database), a
`metric*` measure answers only in a CRS the caller declared in metres (there is
no ellipsoidal geodesic), and the buffers, the concave hull (`geof:convexHull`
is implemented), the overlay set operations and the GML/KML/DGGS encodings are
registered and hard-error by name. No raster.

`purrdf-geo` (`purrdf::geo` from the umbrella crate) implements GeoSPARQL 1.1
(OGC 22-047r1) for PurRDF: exact, float-free geometry reached from SPARQL
through the evaluator's two existing extension seams. It parses
`geo:wktLiteral` and `geo:geoJSONLiteral` lexical forms into an exact geometry
model, decides the OGC topological relations over that model, computes the
non-topological accessors, measures and constructors, and hands the `geof:`
family to a host as scalar-function registrations and the spatial relations as
property-function registrations. There is no GEOS and no PROJ behind it — the
DE-9IM engine, WKT and GeoJSON are implemented in-crate, in pure Rust, which is
what lets it build for `wasm32-unknown-unknown`.

## It mints no vocabulary

GeoSPARQL's IRIs are OGC's, not PurRDF's. Every IRI the crate reads or writes —
the two literal datatypes, the `geof:` function names, the `geo:` spatial
relations, the Simple Features geometry classes, and the coordinate reference
system a WKT literal omits — is supplied by the caller through `GeoVocab`,
which has no `Default` and never will. A term that is absent makes the feature
that needs it a hard error, never a fabricated fallback.

```rust,ignore
use purrdf::geo::{Crs, GeoVocabBuilder};

let crs = Crs::new("http://www.opengis.net/def/crs/OGC/1.3/CRS84")?;
let vocab = GeoVocabBuilder::new(
    "http://www.opengis.net/ont/geosparql#",       // geo:
    "http://www.opengis.net/def/function/geosparql/", // geof:
    crs.clone(),                                    // the CRS a bare WKT literal means
    crs.clone(),                                    // the CRS GeoJSON is in
)?
.declare_crs_unit(&crs, "http://www.opengis.net/def/uom/OGC/1.0/metre")?
.declare_metre("http://www.opengis.net/def/uom/OGC/1.0/metre")?
.declare_simple_features_namespace("http://www.opengis.net/ont/sf#")?
.build();
```

## The `geof:` family on the scalar seam

`functions::register` installs every `geof:` function into a
`UserFunctionRegistry` under the vocabulary's function namespace, and the
registry is handed to the engine through `QueryOptions::functions`:

```rust,ignore
use purrdf::geo::functions;
use purrdf::sparql::{NativeSparqlEngine, QueryOptions, UserFunctionRegistry};
use purrdf::SparqlRequest;

let mut functions_registry = UserFunctionRegistry::new();
functions::register(&mut functions_registry, &vocab);

let result = NativeSparqlEngine::new().query_with_options_view(
    &dataset,
    SparqlRequest {
        query: r#"PREFIX geof: <http://www.opengis.net/def/function/geosparql/>
                  PREFIX geo:  <http://www.opengis.net/ont/geosparql#>
                  SELECT ?a ?b WHERE {
                    ?fa geo:hasGeometry/geo:asWKT ?a .
                    ?fb geo:hasGeometry/geo:asWKT ?b .
                    FILTER(geof:sfWithin(?a, ?b))
                  }"#,
        base_iri: None,
        substitutions: &[],
    },
    QueryOptions { functions: &functions_registry, ..QueryOptions::EMPTY },
)?;
```

A function's refusals travel exactly as far as SPARQL says they should. A
malformed literal or a domain refusal — mixed CRSs, the measure of an empty
geometry, an out-of-range index — is a per-solution **expression error**: the
row is eliminated under `FILTER`, and the variable is left unbound under
`BIND`/`SELECT`, while the query continues. An unimplemented function, an
undeclared vocabulary term or a wrong argument count holds for every solution
alike and stays query-fatal, because answering "no value" there would empty a
result set and present that as the answer. A caller that needs the refusal
itself, with its message and kind intact, calls `functions::compute`.

## Spatial relations on the property-function seam

GeoSPARQL's Query Rewrite rules let `?a geo:sfWithin ?b` hold between
*features* whose geometries satisfy the relation, not only where a triple
asserts it. `GeoIndex::from_dataset` projects a dataset's geometry literals
once, and `relation::register` installs one property function per relation of
the families the caller names — Simple Features, Egenhofer, RCC8 — against a
`PropertyFunctionRegistry`. An empty family list is refused: registering
nothing and returning success would surface much later as a query whose
`geo:sfWithin` was parsed as an ordinary triple pattern and matched nothing.

```rust,ignore
use std::sync::Arc;
use purrdf::geo::relation::{self, GeoIndex, GeoIndexConfig, GraphSelector};
use purrdf::geo::{GeoTerm, RelationFamily};
use purrdf::sparql::{ParserOptions, PropertyFunctionRegistry};
use purrdf::TermValue;

let config = GeoIndexConfig::new(
    vec![TermValue::iri(vocab.term(GeoTerm::AsWkt))],
    GraphSelector::Any,
)?;
let index = Arc::new(GeoIndex::from_dataset(&dataset, &vocab, &config)?);

let mut relations = PropertyFunctionRegistry::new();
relation::register(&mut relations, &vocab, &index, &[RelationFamily::SimpleFeatures])?;

// The parser must claim `geo:sfWithin` in predicate position; the registry's
// own descriptors are exactly the IRIs it should claim.
let parser_options = ParserOptions {
    extension_fn_namespaces: Vec::new(),
    property_fn_namespaces: Vec::new(),
    property_fn_iris: relations.describe()?.into_iter().map(|d| d.iri).collect(),
};
```

An asserted `geo:sfWithin` triple matches whether or not the geometries
satisfy it — the rewrite rules are entailments, not definitions — and a
relation the index refutes contributes no row beyond what the data asserts.

## Every answer is exact, and identical on every target

Geometry is where floating point normally destroys reproducibility: `f64`
addition is not associative, so a different traversal order gives a different
answer, and a native build and a wasm32 build can disagree about a predicate
that sits near a boundary. This crate closes that channel rather than
mitigating it.

- **Coordinates are read as exact rationals.** A lexical decimal is parsed
  digit by digit into an exact numerator and denominator; nothing is rounded on
  the way in.
- **Every geometric decision is integer arithmetic.** Orientation, segment
  intersection, point-in-ring, ring winding and the DE-9IM matrix are
  comparisons of exact rationals over arbitrary-precision integers, which Rust
  specifies completely and identically on every target.
- **Irrational measures are integer square roots** at a fixed internal scale,
  summed as integers — one rounding, at the end, of a value that was exact
  until then.
- **The single float boundary is the result literal.** An `xsd:double` result
  is the correctly rounded nearest double, computed with integer arithmetic and
  assembled with `f64::from_bits`. The crate root denies
  `clippy::float_arithmetic`, so there is no second float path to find.

The cross-target claim is executed, not argued: `make geo-determinism` runs
the same corpus natively and on wasm32 and compares bytes.

## What is here, and what is not

Implemented: the WKT and GeoJSON codecs (with CRS and coordinate-dimension
support); every topological relation of the Simple Features, Egenhofer and RCC8
families over an exact DE-9IM; the accessors; and the measures and constructors
that are exactly computable.

Registered but **hard-erroring by name**: the operations that would require
facilities the crate deliberately does not have — a coordinate-reference-system
database for `geof:transform`, and an ellipsoidal geodesic for the `metric*`
family. They are never silently absent and never answer a default. A
topological predicate that returned `false` because it was unimplemented would
be indistinguishable from one that returned `false` because the geometries
genuinely do not relate, and that is the failure this crate exists to keep out.

Like the full-text index, this is a Rust-host seam: the registrations are host
closures and do not cross the Python, WebAssembly or C boundary. The full
exactness accounting is
[`docs/design/purrdf-geo-exactness.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-geo-exactness.md).
