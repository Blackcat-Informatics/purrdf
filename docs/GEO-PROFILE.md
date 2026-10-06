<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Geographic profiles and requests

Rust, SPARQL, Python, C, WebAssembly and the CLI use the same geographic
kernel. The string boundary is `purrdf_validate::geo`: its profile, request
and response records have version `1`, reject duplicate and unknown fields,
and preserve typed errors, exact quantities, certificates and identities.
Decimal quantities are strings. Digest and cell-key strings use lowercase
hexadecimal with their fixed width.

## References and profiles

The standard profile interprets unprefixed WKT, explicit OGC CRS84 and
GeoJSON as WGS84 longitude followed by latitude. EPSG:4326 is unregistered
until the caller explicitly declares WGS84 latitude followed by longitude.
Other references require explicit registration. An ellipsoid declaration
does not install a datum transformation.

```json
{
  "version": 1,
  "references": [
    {
      "crs": "http://www.opengis.net/def/crs/EPSG/0/4326",
      "ellipsoid": "wgs84",
      "axes": "lat-lon"
    }
  ],
  "units": [
    {"iri": "https://example.org/unit/km", "metres_per_unit": "1000"}
  ],
  "limits": {
    "max_output_elements": 131072,
    "max_work_items": 262144,
    "max_workspace_bytes": 67108864,
    "max_iterations": 128,
    "max_subdivision_levels": 64,
    "max_precision_bits": 512,
    "max_scratch_destinations": 768
  }
}
```

The destination count bounds simultaneously live reusable integer buffers.
Their limb capacity follows the admitted precision and original source widths;
their complete storage also consumes `max_workspace_bytes`. Raising the count
requires an explicit policy change. A scratch refusal is fatal and records
`numerical-scratch` with `reason: "capacity"` and required/admitted limb counts,
`reason: "destinations"` and the destination count, `reason: "retained-destinations"` and
held/total destination counts when a live result prevents arena replacement,
or `reason: "size-overflow"`. A leased Taylor workspace refuses with
`numerical-taylor-scratch` and `reason: "in-use"`.
The admitted count changes the policy identity; it does not change metric, grid
or index output laws.

`GeoProfile::standard()` and an omitted host `geo` option select the same
immutable standard interpretation. Standard metre has exact factor `1`.
Caller units require positive exact factors; area conversion squares the
linear factor. CRS84 and standard metre cannot be replaced. Identical
registrations coalesce.

The `operations` member registers named explicit source/target operation
chains. Models declare their references, axes, units and complete parameters:
GCJ applicability, projection meridian and zone, false origins, hemisphere,
rotation convention and law, pivot, epochs and rates, polynomial basis, or
in-memory grid extents and holes. Geographic/geocentric operations use an
explicit ellipsoid. Three-dimensional operations require actual height.
Compilation rejects inconsistent chain endpoints; application quantizes the
final result once. Neither zones nor datum parameters are inferred.

Model `law` tags include `gcj-rational-harmonic-v1`,
`gcj-rational-harmonic-inverse-v1`, `bd09ll-v1`, `bd09ll-inverse-v1`,
`baidu-mercator-analytic-v1`, `ellipsoidal-mercator-v1`, `web-mercator`,
`mercator-to-geographic`, `geographic-to-geocentric`,
`geocentric-to-geographic`, `transverse-mercator`,
`transverse-mercator-inverse`, the two explicit Helmert rotation laws,
`similarity2d-v1`, `polynomial2d` and `polynomial2d-inverse` and `bilinear-grid` and `bilinear-grid-inverse`.
GCJ's mandatory `applicability` contains exact `west`, `east`, `south` and
`north` bounds. For its inverse these bound admissible original source roots;
both cusp charts are isolated, including turns. Multiple distinct roots
refuse even when they round to the same coordinate. Unresolved roots refuse.
Offset inverses certify a forward residual of at most `10⁻¹²` degree;
projection inverses certify at most `10⁻⁶` metre after quantization. These
numerical bounds describe the declared equations, not surveying accuracy or
official provider certification.

Semantic law IDs identify output mathematics; the binding ID identifies
reference, operation and unit registrations; the policy ID identifies
admitted precision, work, memory and output limits. Tightening a proof does
not change an unchanged completed value or law. Resource exhaustion is a
typed refusal, with no partial batch result.
Proof-display lower and upper bounds use exact directed rounding at 36 decimal
places through the shared rational formatter. Their display grid does not
choose a metric or operation's completed output grid.

Standard `geof:buffer` and `geof:metricBuffer` retain the complete original
source and use the same certified outward materializer. Numeric radii undergo
the shared XSD double promotion once; `geof:buffer` then applies the explicitly
registered unit factor exactly. Positive radii use a fixed 15-place angular WKT
grid. Negative radii give the empty geometry; zero retains the exact source
closure and every finite-decimal ordinate, including coordinates wider than
15 places. An exact closure containing a nonterminating rational refuses WKT
serialization rather than rounding the source. Native offset membership remains
available for that exact source.
Point and complete-source materialization laws are separate identities and
both occur in effective query provenance.

## Requests and completed responses

```json
{
  "version": 1,
  "operation": "distance",
  "a": {"longitude": "0", "latitude": "0"},
  "b": {"longitude": "1", "latitude": "0"}
}
```

The completed `result.metres` is `"111319.490793"`. Point distance is the
correct half-even rounding of shortest ellipsoidal distance to a micrometre,
over the original validated coordinates. Its certificate states the fixed
half-micrometre rounding bound and a separate binary64 conversion bound.
`"proof": true` adds the invocation enclosure as a separate proof receipt.
Unresolved rounding returns `PrecisionExhausted`.

Points default to decimal strings. With `"encoding":"ieee64"`, longitude
and latitude contain their exact eight-byte hexadecimal binary64 bits.
Finite dyadics are preserved; nonfinite and original out-of-range coordinates
refuse before numerical conversion.

| Operation | Inputs and behavior |
|---|---|
| `profile` | Inspect effective law, binding and policy identities. |
| `distance`, `distance-batch` | Exact point pair, or a complete `pairs` array; optional `crs`. |
| `inverse`, `inverse-batch` | Canonical shortest branch distance, azimuths, arc, Jacobi scales, reduced length and signed quadrilateral area; optional separate proof. |
| `within` | Public reported-double comparison with a typed `threshold_metres`. |
| `within-physical` | Certified unrounded comparison with exact decimal `threshold_metres`. |
| `direct`, `direct-batch` | `start`, exact `azimuth_degrees` and `length_metres`, or complete `inputs`; frozen angular output grid. |
| `shortest-arc-at` | Original unique shortest-arc endpoints `a`, `b` and exact unit-interval `parameter`. |
| `geometry` | Standard function `function` and RDF `arguments`. |
| `geometry-metric` | `metric`: `length`, `perimeter`, `area` or `geodesic-area-integral`; original carrier, explicit prepared or continuous-image `geometry`. The signed integral takes one selected geodesic edge. |
| `geometry-distance` | Complete original carrier, prepared or continuous-image geometries `a` and `b`. |
| `geometry-relate` | The exact physical DE-9IM `matrix` for complete `a` and `b`, with topology `law`, actual `binding` and both original source identities. Uses the same prepared atlas as standard relation functions. |
| `buffer-points` | Complete point `geometry`, exact `radius_metres` and target `crs`; outward carrier with a certified 0.1-metre radius band. |
| `buffer` | Complete original `geometry`, exact `radius_metres` and target `crs`; outward carrier with the same certified 0.1-metre radius band, including original curves and areal regions. |
| `offset-contains` | Complete source `geometry`, exact `radius_metres`, and original `point` in explicit `crs`; certified physical membership. |
| `operation` | Inspect a named compiled chain. |
| `transform`, `transform-batch` | Named chain and exact operation `point` or `points`. |
| `transform-geometry` | Named actual chain, original carrier and optional observation epoch; certified export. |
| `cell`, `cell-batch` | Explicit native `grid`, `level` and exact point or points. |
| `cell-hierarchy` | Validated `cell`, optional ancestor and descendant storage level. |
| `cell-scale` | Certified physical bounds and optional maximum edge target. |
| `cover-disk`, `cover-reported`, `cover-box` | Complete physical, reported or closed box cover. |
| `cover-region` | Prepared polygon carrier, empty or whole surface region. |
| `point-index` | Prepare stable-key original target-reference points. |
| `point-index-query` | Prepare an `index` and run its `search` with the same adapter. |

An RDF literal record uses `{"kind":"literal","value":"POINT(0 0)",
"datatype":"http://www.opengis.net/ont/geosparql#wktLiteral"}`. IRI
records use `{"kind":"iri","value":"…"}`. Language and direction
members use the shared RDF 1.2 validation rules.

Metric and region inputs also accept `kind: "prepared"`, with an optional
declared `crs`, `points`, ordered `curves` and an explicit `region`.
Coordinates retain exact `x`, `y`, optional `z` and `m` in declared carrier
axes. Each edge declares `law: "source-linear"` with `start` and `end`, or
`law: "azimuth-length"` with `start`, `azimuth_degrees` and `length_metres`.
`law: "shortest-geodesic"` supplies exact `start` and `end` coordinates;
an ambiguous endpoint-only branch refuses.
`law: "operation-image"` supplies exact original `start`/`end`, an explicitly
registered `operation` and optional `epoch_decimal_year`. It retains the complete
continuous source-linear image in the geometry's actual target reference,
including original source axes and Z/M; it never feeds export chords into metrics.
Region kinds are `empty`, `whole`, `geometry`, complete-union `complement`
and `native`. A native region declares `polygons`; each polygon supplies
ordered `rings` of the same explicit edge records and a required `side` of
`left` or `right`. Left selects the intersection of each ring's physical left
side, so clockwise hole rings exclude their interiors. Right complements
that complete intersection. The optional polygon `interior` is `written`
(default) or `complement`. An optional region `complement` boolean selects
the complement of the complete polygon union after those individual choices.
Exact original closure and the Jordan proof are required; rounded endpoint
coincidence cannot close a selected geodesic. Preparation and evaluation
consume the same complete request work and storage admission, including empty
container capacities.

An explicit continuous metric image uses a named registered chain and the
original source literal, for example:

```json
{
  "kind": "image",
  "operation": "https://example.org/operation/inverse-projection",
  "geometry": {
    "kind": "literal",
    "value": "<https://example.org/crs/projected> LINESTRING(0 0,1000 0)",
    "datatype": "http://www.opengis.net/ont/geosparql#wktLiteral"
  }
}
```

The target must be an actual registered geographic reference. Optional
`epoch_decimal_year` supplies the observation epoch. The prepared image retains
exact source interpolation, height, measure, chain and epoch; point images remain
dimension-zero symbolic points. Metrics evaluate the unrounded complete mapping.
The separate 0.1 metre carrier export is never reused as exact metric input.
Declared axes on an identical datum and ellipsoid can share an exact surface
view; original carrier coordinates and source identity remain retained.

General geometry distance carries a total bound of 0.25 mm; length and
perimeter carry the greater of 1 mm and `10⁻¹⁴ × value`; area carries the
greater of 0.1 m² and `10⁻¹⁴ × value`. These include output quantization.
Length includes polygon boundaries and selects the greatest dimension-total
for mixed geometries. Perimeter uses areal rings when present. Selected regions
retain their complete physical closure, including isolated curves and points
when an areal input collapses. Distance, length, perimeter, buffers and region
covers consume those retained strata; topology derives the actual input
dimensions from its complete matrix. Explicit curves keep their original
traversal length. Carrier edges interpolate written source coordinates,
including long longitude paths.

`within` thresholds declare `kind` as `integer`, `decimal` or `double-bits`
and a string `value`. Finite negative reported thresholds return false;
nonfinite thresholds refuse. Physical and reported comparisons have separate
cover and index refinement laws.

## Cells, covers and reusable indexes

Native grids are `wgs84` and `cgcs2000`, with distinct profile identities.
CubeHilbertQ62V1 has levels 0–30, exact integer hierarchy and eight-byte
big-endian key order. A cell record includes its profile, key, face and level;
inconsistent metadata refuses. Descendant ranges include their stride and
storage level.

`cell-scale` computes exact ellipsoidal edge bounds and, when supplied,
selects the smallest level meeting `maximum_edge_metres`. Both phases and
response encoding share the invocation's cumulative work and storage limits.
Their integer arithmetic and governor callbacks do not depend on the
floating environment.

Cover requests declare `mode`: `fixed` with `level`, or `mixed` with
`levels` containing `min` and `max`. Fixed admission counts logical cells;
mixed admission counts final emitted cells. Limits can refuse a complete
cover and do not change successful geometric resolution. `stored_level`
defaults to 30. Closed boxes preserve date-line wrapping, meridians, full
longitude and poles. Region requests declare `region_kind`: `geometry`
with an RDF literal, `complement` for the complement of the complete polygon
union, `empty`, or `whole`.

Reusable point indexes take a `grid`, `crs`, `level` and `points`, each with
a decimal-string stable `key` and exact `point`. Coordinates must already
be in the declared target reference. Optional `conversion` identifies a
registered actual chain whose target matches that reference. It records
conversion provenance; it never relabels or implicitly converts coordinates.
Search accepts physical or reported thresholds and uses cover ranges,
ordered buckets and the corresponding exact public refinement law.

`index::GeographicGeoIndex` prepares a dataset's immutable `GeoIndex`
projection on an actual registered target surface. Its prepared sources retain
original carrier edges and continuous operation images. Query Rewrite adapters
use `GeoRelation::new_geographic` or `relation::register_geographic`; the original
`GeoRelation::new` retains the explicitly planar contract and fingerprint.
Geographic rows use the same prepared atlas as scalar relations, union computed
and asserted triples, and remain sorted and unique. A source borrowed as both
arguments consumes one original source admission; distinct equal-content
prepared objects consume separate admissions.

The geographic index content identity binds original coordinates, RDF 1.2
subjects/configuration, assertions, target and prepared image identities. Its
index identity additionally binds carrier registrations and the topology/index
law. Admission limits stay outside those identities. A declared epoch is
available through `prepare_with_epoch`. `relation_pairs_with_receipt` reports
actual candidate pairs, examined geometry pairs, work and peak storage.
The conservative six-face candidate fallback examines every eligible entry
pair when no tighter complete curve bound is available. Its candidate cost and
typed limit refusals remain observable.

Geographic property opening charges bounded atlas work through
`PropertyFunction::open_metered`. Existing property callbacks keep `open` and
their original cursor contracts. A governor refusal during opening remains an
incomplete query outcome; a native precision or scratch refusal remains fatal.

## Host surfaces and query provenance

Rust uses `GeoProfile`, `GeoSession`, typed `GeoRequest`, reusable
`GeoPointIndex`, and the pure kernel's prepared APIs. Python exposes
`purrdf.geo.GeoProfile`, `GeoSession` and `GeoPointIndex`; query and update
methods accept `geo=`. Configuration parses before the GIL is released.
WebAssembly exposes `GeoSession` and `GeoPointIndex`; `QueryEngine` can
receive a profile or session and snapshots it for suspended operations.
C provides opaque create/call/free contexts and additive context-taking
query variants. A refused C call returns status 13 and complete JSON error
bytes through the caller buffer.

`purrdf geo REQUEST.json --output RESPONSE.json` executes the same records;
`-` selects stdin or stdout. `--geo-profile PROFILE.json` configures `geo`,
`query` and `update`. A refused geographic request prints its complete
response and exits 1.

The actual default SPARQL engine installs the immutable standard functions.
Conflicting standard registrations fail before execution. Operational
precision, work, memory, convergence and cancellation errors propagate
through FILTER and BIND. Governed numerical work charges the shared schedule
in bounded chunks and retains the governor's incomplete-outcome evidence.

Polynomial and bilinear inverses require `source_domain` with exact decimal
`min_x_metres`, `max_x_metres`, `min_y_metres`, and `max_y_metres`. These are
metre coordinates, distinct from the angular applicability rectangle of GCJ.
The inverse seeks every admissible root and refuses multiple roots. Grid
applicability is the closed union of patches whose four nodes are present;
a valid patch boundary remains in that domain beside a hole. No grid operation
extrapolates. Completed inverse coordinates use six decimal metre places and
certify a forward residual of at most one micrometre after quantization.

`transform-geometry` takes a registered operation `name`, an original typed
WKT or GeoJSON `geometry` record, and an optional `epoch_decimal_year`.
The name fixes both actual carrier endpoints. The response carries target WKT,
the operation identity, a 0.1 metre Hausdorff bound, and complete source/output
vertex counts. Every continuous source edge is evaluated; transforming only
written vertices is insufficient. Domain crossings or uncertified image topology
refuse the complete request. Standard `geof:transform` resolves a unique declared
chain to its target IRI. `geof:asGeoJSON` requires CRS84 input or an actual
registered continuous operation to CRS84.

Direct requests and batches accept `angular_decimal_places` to declare a fixed
half-even angular output grid. The default remains 15 places. An admitted
alternative grid binds its own output law and must preserve the one-micrometre
surface certificate. A coarse or overlarge grid returns a typed refusal;
numerical precision never selects the output quantum. Batches borrow original
coordinates into caller buffers and retain the complete buffer allocation under
the same invocation's work and memory policy.

Dataset spatial projection offers `GeoIndex::from_dataset_in_context` and
`from_dataset_metered` alongside the original constructor. Both execute the
same exact projection and preserve its planar source fingerprint. The admitted
path checks original term expansion, geometry parsing, lossless coordinate
keys, sorting, deduplication and identity work before publishing an index.
Cancellation or a work, storage or output refusal returns no partial index.
The context retains the complete admitted preparation allowance on success;
its work and peak counters are evidence of admission, not allocator measurements.

The WKT writer's context-taking variants use the same carrier rendering body
and preserve ordinary output bytes. They reserve output and arithmetic scratch
before writing; the successful returned string remains admitted in the context
until the caller releases `output.capacity()` bytes with `release_workspace`.
A refused writer releases its transient output and work-list storage.

Scalar `transform` and `transform-batch` requests also accept optional
`angular_decimal_places` and `metric_decimal_places`; their defaults remain
15 and 6. Rust exposes `TransformOutputGrid` and corresponding `apply_with_grid`
and caller-buffer batch methods. Only final chain coordinates are quantized.
The physical half-quantum certificate uses the context's declared ellipsoid
for angular coordinates and the Cartesian metre grid for linear coordinates;
coarse or overlarge grids refuse. Alternate active quanta bind a distinct
completed output law while leaving the compiled operation identity unchanged.
Default responses and certificates retain their existing bytes. Inactive unit
quanta coalesce with defaults, and explicit alternatives never select a grid
from solver precision or tolerance.

Scalar transforms retain the complete original source and normalized input
storage through their numerical phase. The worker prepares its reusable integer
destinations from original source and operation parameter widths, then evaluates
through the same bounded arithmetic context used by metrics. The report-only
`transforms` benchmark compares fresh and warmed workers and caller buffers of
1, 4, 16, 256 and 4096 points, with allocator traffic, actual work and admitted
workspace reported separately.

A session compiles its immutable query identity once under a configuration
budget. `GeoSession::try_new(profile, compile_policy)` exposes that budget
separately from the profile's invocation limits. It admits cold reference-ID
rendering, exact unit rendering, the complete reachable reference/operation
parameter graph, cold standard vocabulary, shared ownership headers and the
binding preimage before hashing. Effective registration changes populate the
profile's derived binding cache within the same admitted scope; equal repeat
registrations preserve it, and policy changes keep the binding separate.
Later plan-cache and provenance reads never render original unit integers again.
Session clones and reusable indexes share that
immutable profile. `profile_snapshot` retains its original owner without copying
the graph; Python profile views and WebAssembly operations use that same snapshot
across host suspension. A required owned configuration copy uses the native
`GeoProfile::clone_in_budget` admission before copying any container, text or
integer limbs. `GeoSession::try_from_profile` spans that borrowed-profile copy
and identity compilation with one configuration budget; text constructors use
the same cumulative `from_profile_str` preparation. Profile export uses
`profile_to_string_with_policy` to admit the
original graph, exact formatting and complete JSON storage under a separate
configuration budget; the exported invocation limits stay unchanged.
Worker creation admits exact reference copies alongside the invocation's
retained sources; installing a later preparation receipt preserves that cost.
Requests encode the cached tuple. `from_profile_str` returns a typed
constructor refusal; the compatibility `new` constructor retains one and
refuses subsequent calls. A response from that refused constructor has null
identity, because no identity was compiled. Admitted sessions keep the same
complete identity on successful and refused requests. The identity records both
the local and global point/source buffer laws; the returned materialization
certificate identifies the actual geometric branch. Configuration input,
exact ellipsoid/model validation, canonical
parameter rendering, registration and cached identities share that same budget
through `GeoSession::from_profile_str_with_policy` and
`geo::profile_from_str_with_policy`. Native model constructors and
`CoordinateOperation::compile_in_budget` / `OperationChain::compile_in_budget`
accept the cumulative `PreparationBudget` directly. Large original operands
refuse before their first unadmitted arithmetic or decimal rendering.
Prepared ellipsoid parameters and all derived metric factors share their
immutable long integer limbs after ownership storage is admitted once. Later
reference and metric-bound clones allocate no limb buffers.

The shared string adapter admits complete JSON result storage before building
result arrays, certificates, records or the compact response string. It includes
native results that remain live alongside the writer and exact decimal
formatting scratch. Covers account for the complete returned cell/range arrays;
batches account for every returned element. Output, work or memory exhaustion
produces a typed refusal without a partial successful record. Fixed refusal
records remain available to report the failed admission. Detailed error
formatting and its original exact parameters
use the invocation's cumulative parser, preparation and numerical work/peak
receipt. If that complete detailed record cannot be admitted, the response
carries the fixed work or storage refusal instead; it never opens a second
policy allowance to render an error.

Refusal records preserve exact typed causes. Physical-scale failures include
`target_metres` and, when unattainable, `minimum_metres`; a nonterminating value
uses its exact numerator and denominator. Floating-environment failures include
the reason and the original control-register bits or the expected and observed
probe bits. Invalid ellipsoid/policy failures retain their reason, and checked
arithmetic failures identify the operation.

Scalar coordinate-operation lanes use the workspace's shared inline vector;
up to three axes need no heap-backed coordinate array. The transformation
benchmark reports allocator traffic independently from numerical work,
configuration/preparation cost and wall time. Projection Taylor coefficients
and all-root traversal have their own admitted numerical workspace. Taylor
coefficient buffers are reused by sequential worker phases, checked out without
a lock held across arithmetic or callbacks, and reset after success or refusal.
A simultaneous checkout returns the fatal `numerical-taylor-scratch` refusal
with reason `in-use`; no hidden allocation replaces it.

Carrier parsing and WKT/GeoJSON rendering use the same exact decimal arithmetic
and iterative collection traversal. Governed rendering charges each output
allocation and bounded decimal operation before performing it; cancellation or
an exhausted limit discards the complete response. Consecutive source,
reference, metric and rendering phases preserve cumulative work and the
already observed peak separately from currently retained storage. Reusing a
phase receipt does not charge earlier work or scratch twice.

Standard numeric query results admit the original exact rational-to-binary64
rounding before constructing shifted operands. Canonical IEEE formatting uses
fixed stack buffers and appends at most 24 bytes. The lexical string, datatype
string and enclosing result term are admitted before allocation and remain live
alongside source and numerical preparation until the invocation returns.
`IeeeFormatCost` declares logical codec work for the bounded scientific-format
primitive and the shared normalization's byte visits. It does not count CPU
instructions or the standard library's internal arithmetic. Its temporary
allowance covers the fixed formatting buffers; the caller separately admits
the output string, datatype and result term.
`MetricContext::produce_output` and `produce_output_metered` expose the same
bounded pure-output seam; native callers release its output allowance after
dropping or transferring the result. `remaining_workspace` reports available
live storage independently of earlier temporary peaks. The shared
`carrier::term_in_context` moves an already admitted WKT or GeoJSON string into
an RDF literal after admitting its datatype and enclosing term. Standard query
and strict host exports use that same body; refusal drops and releases the
moved string without allocating its datatype.

`GeometryImage` and `BufferMaterialization` expose a separate
`retained_workspace_bytes()` allowance for their complete owned carrier and
result. Result clones share the immutable carrier rather than allocating another
coordinate tree. Successful materialization releases temporary production storage and
keeps this output charged while the shared WKT or GeoJSON writer admits its
text buffer. A polygon union checks its final coordinate count and actual nested
vector capacities and rational limb storage in one metered walk, then transfers
that storage before releasing the producer's temporary reservation. Its typed
`output_receipt()` keeps the allowance in the context's
owned baseline across later numerical calls. Native callers obtain the receipt,
drop every result owner, then call `release_materialized_output(receipt)` on the
original context, or drop the invocation context. Wrong-context, repeated and
premature releases refuse. It is a conservative storage bound;
it is not an allocator or RSS measurement and does not enter output law IDs or
certificates. Export refusals discard their partial text and preserve the
still-live materialized carrier's allowance.
