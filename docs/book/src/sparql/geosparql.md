<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# GeoSPARQL

PurRDF evaluates spatial predicates, geographic metrics, buffers and explicit
coordinate operations in process, over the dataset a query already holds.
`purrdf-geo` is the Rust facade: it reexports the geometry and geographic types
from `purrdf-geo-kernel` and the native evaluator's scalar and property-function
adapters. The computational kernel depends on no SPARQL evaluator, GEOS, PROJ
or external geographic runtime. The same kernel serves Rust, Python, C,
WebAssembly and the CLI.

## References are explicit

The standard geographic profile assigns unprefixed WKT, OGC `CRS84` and GeoJSON
to WGS 84 longitude followed by latitude. Original decimal coordinates remain
exact rationals. Longitude outside `[-180, 180]` and latitude outside
`[-90, 90]` refuse before conversion; valid inputs are never wrapped or clamped.

EPSG:4326 requires an explicit WGS 84 latitude/longitude registration. Other
references, units and operation chains require caller registrations. An
ellipsoid does not establish a datum transformation, and the engine does not
guess zones, height, epoch, control points or regional transformation
parameters. Standard OGC terms have their published meanings; application
namespaces and custom registrations remain caller supplied.

[`Geographic profiles and requests`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GEO-PROFILE.md)
specifies the versioned JSON records, reference axes, operation parameters,
limits, identity rules and every host entry.

## Scalar functions in the native query engine

The default `NativeSparqlEngine` installs the immutable standard `geof:`
functions. Registering a conflicting standard function is a hard error before
execution. A query selects additional bindings and independent numerical
limits with `QueryOptions::with_geo`:

```rust,ignore
use purrdf::geo::GeoProfile;
use purrdf::sparql::{NativeSparqlEngine, QueryOptions};
use purrdf::SparqlRequest;

let profile = GeoProfile::standard();
let result = NativeSparqlEngine::new().query_with_options_view(
    &dataset,
    SparqlRequest {
        query: r#"PREFIX geof: <http://www.opengis.net/def/function/geosparql/>
                  PREFIX geo: <http://www.opengis.net/ont/geosparql#>
                  SELECT (geof:metricDistance(
                    "POINT(0 0)"^^geo:wktLiteral,
                    "POINT(1 0)"^^geo:wktLiteral) AS ?metres)
                  WHERE {}"#,
        base_iri: None,
        substitutions: &[],
    },
    &QueryOptions::new().with_geo(&profile),
)?;
```

Geographic `metricDistance`, `metricLength`, `metricPerimeter`, `metricArea`
and `metricBuffer`, and their explicit-unit counterparts, use the declared
ellipsoid. Projected metre systems retain their planar law. Length includes
polygon boundaries and preserves the declared mixed-dimension rule; area
subtracts holes and squares unit factors. Z and M are retained without
inventing an elevation.

Malformed arguments and geometric domain errors retain SPARQL expression-error
behavior. Operational precision, work, memory, convergence, cancellation and
floating-environment failures remain fatal through `FILTER` and `BIND`.
Governed evaluation retains its original incomplete-outcome and work evidence.
The direct geographic request API preserves the same typed causes instead of
turning a refused invocation into an empty result.

## Geometry and geographic laws

WKT and GeoJSON retain their original coordinate-linear edges, including a
written long route across longitude. Explicit shortest-geodesic and
azimuth/length arcs retain their selected branches; ambiguous endpoint-only
arcs refuse. Prepared regions represent holes, complements and the whole
surface. Metrics, topology, continuous coordinate-operation images, buffers
and covers use those same prepared curves and regions.

Point distance is the correctly rounded shortest ellipsoidal distance at six
decimal metre places, using half-even rounding. A complete certified enclosure
must select one rounded answer; acceptable approximation error alone does not
authorize a result. Direct coordinates use the declared angular grid.
Geometric distance, length, perimeter, area and outward buffers carry their
own declared certificates and rounding laws. The numerical primitives have
one home in `purrdf-xsd`; no platform `libm` or reassociated reduction selects
an answer.

Explicit coordinate chains cover projections, geographic/geocentric
conversion, the named GCJ/BD equations, and caller-parameter Helmert,
similarity, polynomial and in-memory grid models. Bounded inverses isolate all
admissible roots and refuse ambiguity. These mathematical laws do not claim
surveying accuracy or official provider certification.

## Spatial relations and cells

The property-function adapter projects caller-selected geometry predicates
into a reusable `GeoIndex` and registers the selected Simple Features,
Egenhofer or RCC8 relations. Asserted relation triples retain their RDF
meaning; calculated relations add the entailed matches. The actual query's
selected geographic context and original allocation owners remain live through
the cursor. An empty relation-family registration refuses.

The native cubed-sphere hierarchy assigns profile-qualified cell IDs at levels
0–30. Parent/child operations, big-endian keys and ordered ranges share that
definition. Closed disk, box and prepared-region covers are conservative;
caller limits refuse a complete cover rather than truncate or silently
coarsen it. Reusable point indexes keep original coordinates and refine
candidates with the requested physical or reported-distance law. `h3` v4.0
compatibility is an architecture document, not an implemented family.

## Host ownership and qualification

`GeoSession` and `GeoPointIndex` expose the same versioned requests on every
host. Python query and update entries accept `geo=`. WebAssembly engines
snapshot the selected profile for suspended operations. C provides opaque
create/call/free handles and context-taking query entries. The CLI accepts
`--geo-profile` for geographic requests, queries and updates.

A semantic refusal has a complete response record. An operational refusal
keeps its original failure channel and creates no unadmitted response under
the failed invocation. Prepared data is immutable and shareable; live
numerical contexts stay thread bound. Native storage is admitted before birth
and retained until its actual payload is destroyed.

Frozen numerical, geometry and hierarchy transcripts compare completed native
and WebAssembly answers. The public GeodTest collector checks the complete
500,000-row source by digest, while allocator, backend and host fixtures check
their separate contracts. Mathematical derivations and exactness boundaries
are recorded in
[`Geodesic numerics`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-geodesic-numerics.md)
and
[`Geometry exactness`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-geo-exactness.md).
