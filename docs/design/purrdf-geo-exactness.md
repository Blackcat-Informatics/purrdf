<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# PurRDF geometry: exactness, determinism, and the answers this crate refuses to guess

`purrdf-geo-kernel` owns geometry, geographic metrics, coordinate operations and
spatial indexing. `purrdf-sparql-eval` owns their evaluator adapters;
`purrdf-geo` preserves the existing module paths as a compatibility facade.
This document distinguishes the explicitly planar contracts from the standard
geographic contracts. Both retain exact carrier coordinates and deterministic
completed outputs.

Standard CRS84 and GeoJSON use WGS84 longitude/latitude. Other references and
coordinate-operation chains require explicit registration. Standard OGC terms
live in `purrdf_iri::vocab::ogc`; application vocabularies remain caller-supplied.
See [geographic profiles](../GEO-PROFILE.md) for configuration and
[certified geodesic arithmetic](purrdf-geodesic-numerics.md) for numerical laws.

---

## 1. Exact carrier arithmetic and certified geographic arithmetic

Geometry is the part of a data-carrier backbone where floating point normally
destroys reproducibility, and it does so in two distinct ways.

The first is ordinary: `f64` addition is not associative, so the length of a
polyline depends on the order its segments were summed, and a refactor that
reverses a traversal silently changes an answer. The second is worse: the same
source, compiled for two targets, can disagree. A predicate decided by the sign
of a cross product near zero is decided by whichever way the rounding fell, and
`x86_64` and `wasm32-unknown-unknown` need not fall the same way.

Neither is a tolerance problem to be papered over with an epsilon. An epsilon
turns "wrong" into "wrong less often", and a topological predicate that is wrong
less often is still a query that returns the wrong rows with no symptom.

The explicitly planar geometry implementation computes through exact integers.
The geographic engine computes through controlled interval arithmetic in XSD,
retaining the original rational coordinates for refinement. A floating result
never decides an unresolved topological predicate by an epsilon.

* **Coordinates are read as exact rationals.** A WKT or GeoJSON coordinate is a
  decimal lexical form, and decimal lexical forms are exactly representable as
  rationals. `wkt::parse` and `geojson::parse` read the digits into an exact
  numerator and denominator through `Rat::parse_decimal`. `str::parse::<f64>()`
  appears nowhere on the ingest path, so nothing is rounded on the way in and
  `1.5`, `1.50` and `15e-1` produce the identical geometry.
* **Planar geometric decisions are comparisons of exact integers.** Orientation,
  segment intersection, point-in-ring, ring winding, the noding, the scan line
  and the DE-9IM matrix are all sign tests over `Int`, an arbitrary-precision
  signed integer. Rust specifies integer arithmetic completely and identically on
  every target, so two targets cannot disagree.
* **Planar irrational measures use exact integer square roots.** A length is a sum of
  square roots, and a sum of individually-rounded terms depends on the rounding.
  Each segment's length is therefore computed as `floor(sqrt(n·m·10^36))/m` — an
  exact integer square root at a fixed internal scale of `10^-18` — and the terms
  are summed **as integers**, which is associative without qualification. There
  is one truncation, at the end, of a value that was exact until then, and its
  error is at most `k · 10^-18` for `k` segments. That bound is stated on the
  function rather than left for a reader to discover.
* **Numeric literal conversion has one exact home.** GeoSPARQL's numeric
  functions return `xsd:double`, so exactly one conversion happens: `Rat::to_f64`
  computes the correctly rounded nearest double using integer arithmetic and
  assembles it with `f64::from_bits`. It is a rounding, not a computation.

Both the kernel and facade deny unsafe code and floating-point arithmetic.
Generic controlled floating operations, outward intervals and higher-precision
arithmetic live in XSD. Geographic traversal calls those homes; it cannot add a
second floating implementation inside a geometry module.

The denial governs geographic **library** code. Generic exact arithmetic lives
in `purrdf_xsd::{integer, rational}`; geographic `exact::{Int, Rat}` paths reexport
those shared types. The rational tests compare `Rat::to_f64` bit patterns with
Rust's independent literal parser. That host floating-point calculation is an
oracle in the tests; shipping carrier conversion uses the exact integer and
IEEE half-even homes.

Exact integer decisions also avoid unnecessary rational reduction. XSD tests a
half-open difference interval by comparing
`a.num*b.den-b.num*a.den` with integer multiples of `a.den*b.den`. Longitude
lifting uses this predicate to retain an original coordinate already in the
selected chart; the upper antipodal endpoint still takes the wrapped path.
Both paths produce the same exact coordinate. Checked work and workspace
admission precede the products and any retained coordinate copy.

### 1.1 Fixed output laws

`measure::LENGTH_SCALE_DIGITS` is 18 and is part of the crate's contract, not a
tuning parameter. Two hosts that computed a length at two precisions would
compute two different answers to the same query, which is exactly the
per-consumer optionality this repository forbids. The same reasoning that keeps
`purrdf-text`'s series-term count fixed applies here.

That constant belongs to the established planar law. Geographic point distance
instead uses correctly rounded micrometres; direct coordinates use a fixed
15-place default angular grid. Every grid, including that default, must meet its
declared surface-error certificate on the original ellipsoid; a sufficiently
large axis can require a finer explicit grid. General geographic metrics have separate declared
approximation bounds and grids. Increasing proof precision tightens an invocation
enclosure without selecting a different completed output precision.

Geographic metrics use the actual selected closed set. A region can retain
curves or isolated points after its open surface interior disappears. The one
certified native arrangement preserves those original source-parameter domains;
distance, offsets and covers include them. Length adds selected curve strata to
ordinary curves and chooses the greatest dimension's total. Perimeter uses the
selected areal boundary when an areal face exists, otherwise the complete curve
total. Area integrates only areal faces. These choices are part of each metric's
law identity, so a corrected selection rule changes its certificate law field
even when an ordinary polygon's numeric answer stays identical.
Distance, length, perimeter and region-buffer output laws also bind the shared
selected-topology law. Its complete complement sectors remove internal union
walls and nodes from a complemented areal source. The point laws and areal
integral remain unchanged by a correction confined to those lower strata.

Numerical preparation, refinement and output conversion share one admission
context. Comparing independently supplied reference declarations admits every
original parameter limb scan before equality is tested; the physical-surface
comparison shares that admission and ignores only carrier axis order. Cold
reference identities render arbitrary original rationals only
after their checked work and storage bounds are admitted. Higher-precision
preparation includes its outer live workspace before an observer can cancel.
An internal prepared buffer invocation validates both reference bindings and
its frozen direct grid once; subsequent samples check the initialized context
binding and run the same direct solver. A cold binding still takes the original
admitted identity path. Work and simultaneous result/scratch storage are checked
before opening either reservation.
Returned scalar limbs are detached from the reusable arena and remain admitted
through all conversion and certificate checks. A caller retaining those scalars
in a batch uses their documented retained-storage census; a cancelled result
releases its scoped owners and preserves the observer's original failure.

Zero-radius buffers preserve the exact source closure. Their WKT exports use
the source's minimum terminating-decimal scale: a normalized denominator
`2^a*5^b` needs `max(a,b)` fraction places. This is an exact input property,
independent of proof precision. An exact carrier that has no finite decimal
representation refuses export. Positive buffer materialization keeps its
declared 15-place angular grid.

---

## 2. The determinism claim is evidence, not an argument

Section 1 is an argument. An argument is not evidence, and the defect this crate
exists to prevent is the one that produces no symptom — so the claim is made
observable.

`purrdf_geo::determinism::digest` runs a hand-written corpus through **every
consumer-visible output path**: the WKT writer, the GeoJSON writer, all ordered
DE-9IM matrices, the exact decimal measures, the constructors, and the IEEE bit
patterns at the float boundary. It folds the resulting **bytes** into one FNV-1a
`u64`.

Three separate corpora pin their own `GOLDEN_DIGEST`: the planar facade in
`crates/geo/tests/determinism.rs`, the native hierarchy in
`crates/geo-kernel/tests/cells_determinism.rs`, and completed geodesy,
operations, metrics, buffers, covers and indexes in
`crates/geo-kernel/tests/geo_determinism.rs`. Each uses the shared test runner
and prints its named digest and corpus count. The complete planar corpus runs
natively under `make check`; the conformance matrix reads its golden directly
from the test source and compares it with the native example's digest.

The Rust `qualify_determinism` example executes every corpus natively, on portable
wasm, and on wasm with `simd128`. It also executes the shared XSD numerical
target on these paths, including bounded scratch, rounding, allocation and
refusal checks. Node runs the wasm modules through
`scripts/wasm-test-runner.sh`. The gate reads each `GOLDEN_DIGEST` from its
target, compares all named records and refuses a missing execution prerequisite.
`make geo-determinism` runs all twelve target executions; CI runs the gate in
the `geo-determinism` job, where the target and Node are present. The Arm64,
i686 and i586 jobs execute the kernel and XSD targets on their actual arithmetic
paths. `make wasm` separately
builds the release crates.

Two design points in that harness are load-bearing:

**The digest is over serialized bytes.** Byte identity of the answer a consumer
sees is the only claim that covers coordinate lexical forms, matrix renderings
and double renderings at once, and it is the artefact a downstream cache, diff or
signature would key on. A digest over internal values would pass while the
renderer diverged.

**Repeated native runs hold the serialized digest fixed.** The planar test
checks the committed golden and repeats the corpus 64 times to detect a moving
result. `report_digest` invokes
`purrdf_testkit::harness::without_host_clock_or_entropy`, which runs the
computation directly on native targets.

**The wasm runner withdraws host clock and entropy sources during the digest.**
The compatibility facade depends on `purrdf-sparql-eval`, whose target-gated
`wasm-bindgen` imports supply SPARQL's `NOW()` and `RAND()`. These corpora touch
neither. The runner replaces `Date.now`, `new Date()`, `performance.now`,
`Math.random`, `crypto.getRandomValues` and `crypto.randomUUID` with functions
that throw for the duration of `without_host_clock_or_entropy`, so an accidental
host-source read fails by its source's name. This host-source seal is a wasm
runner capability, distinct from repeated native byte agreement.

### 2.1 What the guarantee does not cover

Each digest establishes the pinned output of its named corpus on the target
that actually executes it. The planar digest is separate from certified
numerical and geographic replay targets. A matching arithmetic digest does not
establish a full geometry or host qualification. `make wasm` checks release-crate
compilation; neither that build nor a native digest proves cross-target byte
agreement. Actual replay establishes the agreement for the executed targets.

---

## 3. Three departures from the standard's printed tables

Each is implemented deliberately, each is documented at the constant it changes,
and each exists because the printed form produces a **silent wrong answer** — a
`false` from a topological predicate that no query text can distinguish from an
honest `false`.

### 3.1 `sfIntersects` follows Table 2, not Table 6

OGC 22-047r1 prints two different patterns for `sfIntersects`. Table 2 (the
property) gives the four-row union `T********` / `*T*******` / `***T*****` /
`****T****`. Table 6 (the function) gives `FT*******` / `F**T*****` /
`F***T****` — which is character for character the pattern it also gives for
`sfTouches`.

Table 6 is a published defect, and the standard refutes it from the inside: its
own Table 5 states the cross-family equivalence `intersects | ¬ disconnected |
¬ disjoint`, and a relation equal to `sfTouches` is not the negation of
`sfDisjoint`. Implementing Table 6 would make `geof:sfIntersects` answer `false`
for a point strictly inside a polygon.

### 3.2 `equals` is `T*F**FFF*`, not `TFFFTFFFT`

The standard prints `TFFFTFFFT` for `sfEquals`, `ehEquals` and `rcc8eq` alike.
Position 4 of that pattern is `boundary ∩ boundary`, and it demands `T` — a
*non-empty* boundary intersection. But a `Point` and a `MultiPoint` have an empty
boundary by definition, and so does a closed curve. Read literally,
`geof:sfEquals("POINT(1 1)", "POINT(1 1)")` is `false`.

This was found by a test, not by reading, and independently by two parts of the
implementation at once. The pattern implemented instead is `T*F**FFF*`, which is
precisely `within AND contains` — the conjunction of the standard's own two
patterns, character by character — and which is the definition equality actually
has. It agrees with `TFFFTFFFT` on every pair of geometries that *have*
boundaries, which is why the defect is invisible until a point is involved.

### 3.3 The type-dispatched relations answer the reversed argument order

`sfCrosses` is type-dispatched, and the standard gives patterns for point/curve,
point/area and curve/area but says nothing about the reversed pairs. Reading that
silence as "answer `false`" would make `geof:sfCrosses(?line, ?polygon)` true
while `geof:sfCrosses(?polygon, ?line)` is false for the same crossing — a wrong
answer produced by argument order alone. The reversed pairs are answered with the
transposed pattern.

The exclusions the standard *does* state are honoured: `sfTouches` is false for a
point/point pair, and `sfOverlaps` is false whenever the dimensions differ.

---

## 4. References and operations are explicit

`purrdf-geo` ships no coordinate-reference-system database. That is a
deliberate scope line: a CRS database is megabytes of tabular data with its own
release cadence. The numerical engine instead compiles explicit in-memory
reference and operation profiles and remains usable offline and on wasm.

Two consequences, both refusals rather than guesses:

* **A binary operation on two geometries in different systems is refused** by
  name, naming both systems. Coordinates in two systems are two different numbers
  describing the same place; arithmetic across them is meaningless, and answering
  anyway would be plausible and silently wrong.
* **Explicit planar measurements retain their coordinate units.** The caller
  *declares* the linear unit of each CRS it uses
  (`GeoVocabBuilder::declare_crs_unit`), and a measurement requested in a unit
  that has not been declared for that system is refused by name. The `metric*`
  family additionally requires the caller to have declared which IRI denotes the
  metre. A number in the wrong unit is the worst kind of wrong answer: plausible,
  silent, and off by a factor nobody can see.

Standard geographic measurements use ellipsoidal ground metres and registered
unit factors. WGS84 and CGCS2000 are distinct native ellipsoids and references;
selecting an ellipsoid does not supply a datum transformation. EPSG:4326 refuses
until its WGS84 latitude/longitude axis registration is supplied. Transformations
use actual compiled chains with declared axes, units, projection parameters,
domains and, when required, epochs and height. Continuous geometry images follow
the original complete edges rather than transforming vertices alone.

### How far a refusal travels

"Refused" is two different outcomes, and which one a `geof:` call gets is decided
by `GeoError::is_expression_error` — the single site in `crates/geo-kernel/src/error.rs`
that answers it — rather than at each call site.

A refusal that is a statement about *these arguments* (`GeoError::Literal`, a
lexical form its datatype does not license; `GeoError::Domain`, well-formed
arguments the operation is undefined on, which is where the mixed-system and
undeclared-unit refusals above land) is a **SPARQL expression error**. SPARQL 1.1
§17.2 puts it there — "Functions invoked with an argument of the wrong type will
produce a type error" — and the enclosing operator resolves it: a `FILTER`
eliminates that one solution (§17), a `BIND` or `SELECT` expression leaves the
variable unbound and evaluation continues (§10). Every other row is answered
normally. The alternative was tried and is worse: with no per-solution channel,
one malformed geometry anywhere in a dataset fails every query that scans past it.

A refusal that holds for *every* solution alike stays query-fatal, because
answering "no value" would empty a result set and present that as the answer.
Three kinds are in this class: `GeoError::Unsupported` (a function this crate does
not implement — a `false` from an unimplemented predicate is indistinguishable
from an honest `false`), `GeoError::Config` (a declaration the host never made,
which no row can repair and PurRDF fabricates no default for), and
`GeoError::Arity` (a wrong argument count, which is a defect in the query text
that no row can satisfy).

The explicit planar adapter has no operation profile and refuses `transform`.
The standard geographic adapter resolves registered chains. Precision, work,
memory, cancellation, convergence and floating-environment failures remain
query-fatal through `FILTER` and `BIND`; they never become successful empty results.

---

## 5. JSON retains original numeric lexemes

`serde_json` parses a JSON number into an `f64` or an `i64`. Using it would round
every GeoJSON coordinate on ingest and destroy the guarantee of section 1 before
any geometry existed. Its `arbitrary_precision` feature would fix that, but Cargo
features unify across a workspace, so enabling it here would change
`serde_json`'s behaviour for `purrdf-rdf`'s JSON-LD codec as well — a
per-consumer semantic change of exactly the kind this repository forbids.

`purrdf_lex::json` is the one RFC 8259 reader and writer. Its `Number` variant
retains the **source lexeme verbatim**, so geometry reads exact decimal values.
Object members remain ordered pairs; strict profile/request codecs reject
duplicate and unknown fields explicitly. No binding has its own JSON or
coordinate-rounding implementation.

---

## 6. What is implemented, and what hard-errors by name

The operations this crate cannot answer are **registered** and fail loudly. They
are never silently absent, and they never answer a default. A `geof:` call that
returned `false` because it was unimplemented would be indistinguishable from one
that returned `false` because the geometries genuinely do not relate, and that is
the failure this crate exists to keep out.

### Implemented

* Both literal codecs: `geo:wktLiteral` (with the optional CRS prefix, the `Z`/
  `M`/`ZM` tags, `EMPTY` at every level, and both `MULTIPOINT` spellings) and
  `geo:geoJSONLiteral` (RFC 7946 Geometry objects, with `Feature` and
  `FeatureCollection` refused by name as Requirement 25 requires).
* All twenty-four topological relations across the Simple Features, Egenhofer and
  RCC8 families, plus `geof:relate`, over an exact DE-9IM matrix.
* The accessors: `dimension`, `coordinateDimension`, `spatialDimension`,
  `geometryType`, `isEmpty`, `isSimple`, `is3D`, `isMeasured`, `getSRID`,
  `numGeometries`, `geometryN`, `minX`/`maxX`/`minY`/`maxY`/`minZ`/`maxZ`.
* Planar and geographic `area`, `length`, `perimeter`, `distance`, and their
  `metric*` counterparts under the corresponding explicit measurement law.
* The exactly-computable constructors: `envelope`, `boundary`, `convexHull`,
  `centroid`.
* `asWKT` and `asGeoJSON`; geographic GeoJSON output requires an actual certified
  chain to WGS84 CRS84 when its source reference differs.
* Query Rewrite (Clause 13) over the property-function seam, with all four RIF
  branches.

### Registered and hard-erroring

All 68 scalar functions are registered. The explicit planar/carrier adapter
handles 56 and refuses the twelve entries below. The standard geographic adapter
additionally handles `transform`, `buffer` and `metricBuffer`; the nine other
listed construction/serialization functions remain fatal on both adapters.
The table below describes the explicit planar adapter. The standard geographic
adapter resolves registered operation chains for `transform` and materializes
complete physical offsets for `buffer` and `metricBuffer`. Its buffer certificate
proves containment between radii `r` and `r+0.1 m`; it refuses an uncertified
source image or incomplete resource admission.

| Function | Why |
|---|---|
| `transform` | The explicit planar adapter has no compiled operation profile. |
| `buffer`, `metricBuffer` | The explicit planar adapter has no geographic physical-offset profile. |
| `boundingCircle`, `concaveHull` | These construction laws are not implemented; their parameters are implementation-defined. |
| `intersection`, `union`, `difference`, `symDifference` | No general GeoSPARQL set-construction output law is exposed for two input geometries. Internal arrangement and buffer-union helpers do not define a carrier under these function names. |
| `asGML`, `asKML`, `asDGGS` | Those serializations are not implemented. |

The six spatial aggregates (`aggBoundingBox`, `aggBoundingCircle`, `aggCentroid`,
`aggConcaveHull`, `aggConvexHull`, `aggUnion`) are SPARQL **aggregates**, not
scalar functions. They are listed by the crate but deliberately **not registered
on the scalar seam**, because registering them there would make
`geof:aggUnion(?g)` look as though it worked while computing something else
entirely. They belong on the aggregate seam.

---

## 7. Query Rewrite: why all four branches, always

Clause 13's RIF rule expands `?so1 <relation> ?so2` into a disjunction of four
bodies, because `?so1` and `?so2` are `geo:SpatialObject`s and a spatial object
may be a `geo:Feature` (dereferenced through `geo:hasDefaultGeometry`) *or* a
`geo:Geometry` (whose serialization is read directly).

Implementing only the feature-to-feature branch is the classic bug in this
extension, and it is a **short bag reported as complete**: the query returns
fewer rows than it should, every row it does return is correct, and nothing
anywhere reports a problem. `GeoIndex` therefore indexes a spatial object's own
serializations *and* the serializations of its default geometries, which collapses
the four branches into one lookup that cannot be half-implemented.

Three further points the rule forces:

* **`geo:hasDefaultGeometry`, not `geo:hasGeometry`.** Every branch that
  dereferences a feature uses the default-geometry property exclusively. The
  GeoSPARQL 1.0 legacy alias `geo:defaultGeometry` is accepted on input and never
  emitted.
* **Asserted triples still match.** RIF's `:-` is an entailment rule, not a
  definition, so an explicitly asserted `ex:a geo:sfWithin ex:b` must continue to
  match. A property function *replaces* the triple pattern, so the index collects
  asserted triples of each relation predicate and the relation emits them
  alongside the computed ones.
* **Rows are deduplicated.** A spatial object may carry several default
  geometries and several serializations, and the four branches overlap. The
  entailed triple either holds or it does not, and BGP matching over a set of
  triples yields one solution — but the evaluator does not deduplicate
  property-function rows, so the relation must. Emitting a pair twice would
  produce a duplicate solution that no query text explains.

---

## 8. Layout was measured, not assumed

An exact `Coord` is four arbitrary-precision rationals and is 384 bytes. The
first version of the model used a small-vector with an inline capacity of four
for position sequences, which made every geometry 1552 bytes: a `POINT` carried a
kilobyte and a half of ring storage it could never use, a `Vec<Geometry>` paid it
per member, and the WKT parser overflowed a 2 MiB stack at its own nesting cap.

`CoordSeq` is a plain `Vec`. The inline storage bought nothing — a sequence with
any positions in it allocates either way — and removing it cut the model by a
factor of four and fixed the overflow. `crates/geo/tests/layout.rs` pins the
numbers the decision was made on, so a change to them is a test failure with a
diff rather than a regression nobody profiles for.

The residue is the `Point` variant, which holds a `Coord` inline and is therefore
the size of one. Boxing it would put an allocation on the commonest geometry in
the commonest corpus in order to relocate bytes the coordinate occupies
regardless; the scoped `#[allow(clippy::large_enum_variant)]` on that enum records
that trade and points at the test.

---

## 9. Complexity, stated rather than hidden

The established planar noder compares every segment pair, so planar `relate` is quadratic in the combined
segment count, and splitting is linear in events per segment. The scan line is
`bands × segments`. That is correct and exact for every input, and it is fine for
the geometry sizes GeoSPARQL corpora actually carry, but it is not an indexed
implementation and this document does not pretend otherwise. `crates/geo/benches/relate.rs`
is where a change to it would be measured; like every bench in this repository it
is report-only and asserts no timing.
