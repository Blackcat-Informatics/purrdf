<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# PurRDF geometry: exactness, determinism, and the answers this crate refuses to guess

`purrdf-geo` implements OGC GeoSPARQL 1.1 (OGC 22-047r1) as an out-of-core
sibling crate. This document records the decisions behind that surface which a
reader would otherwise take for oversights — the missing dependency, the missing
function, the hand-written parser, the constant that is not configurable — and
states the limits of the guarantees it makes.

All example IRIs use `example.org`. PurRDF mints no vocabulary IRIs.

---

## 1. Every computation is integer arithmetic, and that is a correctness requirement

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

So this crate does not use floating point.

* **Coordinates are read as exact rationals.** A WKT or GeoJSON coordinate is a
  decimal lexical form, and decimal lexical forms are exactly representable as
  rationals. `wkt::parse` and `geojson::parse` read the digits into an exact
  numerator and denominator through `Rat::parse_decimal`. `str::parse::<f64>()`
  appears nowhere on the ingest path, so nothing is rounded on the way in and
  `1.5`, `1.50` and `15e-1` produce the identical geometry.
* **Every geometric decision is a comparison of exact integers.** Orientation,
  segment intersection, point-in-ring, ring winding, the noding, the scan line
  and the DE-9IM matrix are all sign tests over `Int`, an arbitrary-precision
  signed integer. Rust specifies integer arithmetic completely and identically on
  every target, so two targets cannot disagree.
* **Irrational measures are exact integer square roots.** A length is a sum of
  square roots, and a sum of individually-rounded terms depends on the rounding.
  Each segment's length is therefore computed as `floor(sqrt(n·m·10^36))/m` — an
  exact integer square root at a fixed internal scale of `10^-18` — and the terms
  are summed **as integers**, which is associative without qualification. There
  is one truncation, at the end, of a value that was exact until then, and its
  error is at most `k · 10^-18` for `k` segments. That bound is stated on the
  function rather than left for a reader to discover.
* **The single float boundary is the result literal.** GeoSPARQL's numeric
  functions return `xsd:double`, so exactly one conversion happens: `Rat::to_f64`
  computes the correctly rounded nearest double using integer arithmetic and
  assembles it with `f64::from_bits`. It is a rounding, not a computation.

The crate root carries `#![deny(clippy::float_arithmetic)]`, so there is no
second float path to find and none can be added without the denial firing.

That guardrail was **verified rather than assumed**: a temporary `fn probe(a: f64,
b: f64) -> f64 { a + b }` was added to `measure.rs`, `cargo clippy -p purrdf-geo`
reported `error: floating-point arithmetic detected` pointing at the crate-root
denial, and the probe was removed. A guardrail nobody has seen fire is a claim,
not a guarantee.

The denial governs **library** code. Test modules may and do use `f64` as an
*oracle* — `exact.rs` checks `Rat::to_f64` against the bit patterns Rust's own
literal parser produces, which is only possible in the arithmetic being checked
against. That is the same arrangement `purrdf-text` uses, and it is sound for the
same reason: the oracle is not the ground truth the crate ships, it is a second
opinion the crate is compared with.

### 1.1 Why `LENGTH_SCALE_DIGITS` is a constant and not a knob

`measure::LENGTH_SCALE_DIGITS` is 18 and is part of the crate's contract, not a
tuning parameter. Two hosts that computed a length at two precisions would
compute two different answers to the same query, which is exactly the
per-consumer optionality this repository forbids. The same reasoning that keeps
`purrdf-text`'s series-term count fixed applies here.

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

* `crates/geo/tests/determinism.rs` pins that number natively, in `GOLDEN_DIGEST`.
* `scripts/check-geo-determinism.sh` builds the same function for
  `wasm32-unknown-unknown` — through `crates/geo/determinism`, a
  workspace-excluded one-function `cdylib` — runs it under Node, reads
  `GOLDEN_DIGEST` out of the test file rather than restating it, and fails unless
  all three agree.
* `make geo-determinism` runs it; CI runs it in the `wasm` job, where the target
  and Node are already present.

Two design points in that harness are load-bearing:

**The digest is over serialized bytes.** Byte identity of the answer a consumer
sees is the only claim that covers coordinate lexical forms, matrix renderings
and double renderings at once, and it is the artefact a downstream cache, diff or
signature would key on. A digest over internal values would pass while the
renderer diverged.

**Every wasm host import is bound to a throwing stub.** `purrdf-geo` depends on
`purrdf-sparql-eval`, which target-gates `js-sys` and `wasm-bindgen` on wasm32 to
give SPARQL's `NOW()` and `RAND()` a browser clock and browser entropy. The
digest touches neither. Binding those imports to no-ops would let a future change
quietly consult a clock and still produce a digest — a digest that agreed on two
targets while one had read a clock is precisely the false green the harness
exists to prevent. A throwing stub turns that into a failure with the import's
name in it.

### 2.1 What the guarantee does not cover

It covers the two targets it runs on. A 32-bit native target is not exercised;
nothing in the crate depends on pointer width, but that is an argument again, not
evidence, and it is recorded here as such rather than claimed.

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

## 4. Nothing is reprojected, and no unit is converted

`purrdf-geo` ships no coordinate-reference-system database. That is a
deliberate scope line: a CRS database is megabytes of tabular data with its own
release cadence, and dragging one into a wasm-clean carrier crate would be the
"heavy dependency" the umbrella issue's constraint resolution exists to avoid.

Two consequences, both refusals rather than guesses:

* **A binary operation on two geometries in different systems is refused** by
  name, naming both systems. Coordinates in two systems are two different numbers
  describing the same place; arithmetic across them is meaningless, and answering
  anyway would be plausible and silently wrong.
* **A measurement is computed in the coordinate system's own unit.** The caller
  *declares* the linear unit of each CRS it uses
  (`GeoVocabBuilder::declare_crs_unit`), and a measurement requested in a unit
  that has not been declared for that system is refused by name. The `metric*`
  family additionally requires the caller to have declared which IRI denotes the
  metre. A number in the wrong unit is the worst kind of wrong answer: plausible,
  silent, and off by a factor nobody can see.

### How far a refusal travels

"Refused" is two different outcomes, and which one a `geof:` call gets is decided
by `GeoError::is_expression_error` — the single site in `crates/geo/src/error.rs`
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

`geof:transform` is registered and hard-errors (`GeoError::Unsupported`, so
query-fatal), naming the missing database.

---

## 5. The JSON reader is hand-written, because `serde_json` would round every coordinate

`serde_json` parses a JSON number into an `f64` or an `i64`. Using it would round
every GeoJSON coordinate on ingest and destroy the guarantee of section 1 before
any geometry existed. Its `arbitrary_precision` feature would fix that, but Cargo
features unify across a workspace, so enabling it here would change
`serde_json`'s behaviour for `purrdf-rdf`'s JSON-LD codec as well — a
per-consumer semantic change of exactly the kind this repository forbids.

`crates/geo/src/json.rs` is therefore a complete RFC 8259 reader whose `Number`
variant retains the **source lexeme verbatim**, so the consumer decides the value
exactly. Object members are kept as an ordered vector of pairs rather than a map,
because RFC 8259 permits duplicate names and a map would silently drop one.

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
* The exactly-computable measures: `area`, `length`, `perimeter`, `distance`, and
  their `metric*` counterparts under a declared metre.
* The exactly-computable constructors: `envelope`, `boundary`, `convexHull`,
  `centroid`.
* `asWKT` and `asGeoJSON`.
* Query Rewrite (Clause 13) over the property-function seam, with all four RIF
  branches.

### Registered and hard-erroring

| Function | Why |
|---|---|
| `transform` | Needs a coordinate-reference-system database; see §4. |
| `buffer`, `metricBuffer`, `boundingCircle`, `concaveHull` | Need curve approximation whose parameters the standard leaves implementation-defined — it says so explicitly for `concaveHull` — so an answer here would be an invented one presented as a computed one. |
| `intersection`, `union`, `difference`, `symDifference` | Need a planar overlay. The exact noder this crate already has is the foundation for one, but an overlay is a separate subsystem and a half-correct one is worse than a loud refusal. |
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

The noder compares every segment pair, so `relate` is quadratic in the combined
segment count, and splitting is linear in events per segment. The scan line is
`bands × segments`. That is correct and exact for every input, and it is fine for
the geometry sizes GeoSPARQL corpora actually carry, but it is not an indexed
implementation and this document does not pretend otherwise. `crates/geo/benches/relate.rs`
is where a change to it would be measured; like every bench in this repository it
is report-only and asserts no timing.
